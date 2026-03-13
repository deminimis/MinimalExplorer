<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { explorer, type FileItem } from '$lib/explorer.svelte';
  import { isImage, getSystemIconSrc } from '$lib/utils';
  import { settings } from '$lib/settings.svelte';
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { loadDirectory, handleBackgroundContextMenu, handlePreview } from '$lib/actions';
  import FileItemComponent from './FileItem.svelte';

  let { isSecondary = false } = $props();
  
  let rawFiles = $derived(isSecondary ? explorer.secondaryFiles : explorer.files);
  let filterQuery = $derived(isSecondary ? explorer.secondaryFilterQuery : explorer.filterQuery);
  let filterQueryLower = $derived(filterQuery.trim().toLowerCase());
  let files = $derived(
    filterQueryLower === "" 
      ? rawFiles 
      : rawFiles.filter(f => f.name.toLowerCase().includes(filterQueryLower))
  );
  let selectedFiles = $derived(isSecondary ? explorer.secondarySelectedFiles : explorer.selectedFiles);

  // --- Virtualization State ---
  let scrollTop = $state(0);
  let containerHeight = $state(600);
  let _containerWidth = $state(800);
  let containerWidth = $derived(_containerWidth ?? 800); 
  let resizeTimer: ReturnType<typeof setTimeout>;

  $effect(() => {
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(() => {
    }, 100);
    return () => clearTimeout(resizeTimer);
  });

  let scrollFrameId: number | null = null;
  let wheelCooldown = false;
  let containerRef: HTMLElement;
  let observer: IntersectionObserver;
  let pendingObserverNodes: HTMLElement[] = [];

  onMount(() => {
    observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        const wrapper = entry.target as HTMLElement;
        
        if (entry.isIntersecting) {
          if (wrapper.dataset.src) {
            const img = (wrapper.tagName === 'IMG' ? wrapper : wrapper.querySelector('img')) as HTMLImageElement;
            if (img) img.src = wrapper.dataset.src;
  
            observer.unobserve(wrapper);
          }
        }
      }
    }, { root: containerRef, rootMargin: '200px' });

    for (const node of pendingObserverNodes) {
      if (node.dataset.src) observer.observe(node);
    }
    pendingObserverNodes = [];

    containerRef.addEventListener('wheel', handleWheel, { passive: false });
    return () => {
      if (observer) observer.disconnect();
      if (containerRef) containerRef.removeEventListener('wheel', handleWheel);
      if (scrollFrameId !== null) cancelAnimationFrame(scrollFrameId);
      clearTimeout(typeTimeout);
    };
  });

  function handleWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();

      if (wheelCooldown) return;
      wheelCooldown = true;
      setTimeout(() => wheelCooldown = false, 150); 

      const activeTab = explorer.tabs[explorer.activeTabIndex];
      const currentPath = isSecondary ? (activeTab?.secondaryPath || "C:\\") : explorer.currentPath;

      if (e.deltaY > 0) {
        const segments = currentPath.split(/[/\\]/).filter(Boolean);
        if (segments.length > 1) {
          segments.pop();
          let parentPath = segments.join("\\");
          if (parentPath.length === 2 && parentPath.endsWith(":")) parentPath += "\\";
          loadDirectory(parentPath, true, false, isSecondary);
        }
      } else if (e.deltaY < 0) {
        const hoveredEl = (e.target as HTMLElement).closest('.tile, .list-row') as HTMLElement;
        let targetPath = null;

        if (hoveredEl && hoveredEl.dataset.isdir === 'true') {
          targetPath = hoveredEl.dataset.path;
        }

        const targetFolder = targetPath 
          ? { path: targetPath } 
          : selectedFiles.find(f => f.is_dir) || displayFiles.find(f => f.is_dir);
          
        if (targetFolder) {
          loadDirectory(targetFolder.path, true, false, isSecondary);
        }
      }
    }
  }

  function handleScroll(e: Event) {
    const target = e.currentTarget as HTMLElement;
    if (scrollFrameId === null) {
      scrollFrameId = requestAnimationFrame(() => {
        scrollTop = target.scrollTop;
        scrollFrameId = null;
      });
    }
  }

  function lazyLoad(node: HTMLElement, src: string | null) {
    if (src) {
      node.dataset.src = src;
      if (observer) observer.observe(node);
      else pendingObserverNodes.push(node);
    }
    return {
      update(newSrc: string | null) {
        if (!newSrc) {
          if (observer) observer.unobserve(node);
          delete node.dataset.src;
          return;
        }
        if (node.dataset.src === newSrc) return;
        node.dataset.src = newSrc;
        const img = (node.tagName === 'IMG' ? node : node.querySelector('img')) as HTMLImageElement;
        if (img) {
          img.removeAttribute('src');
          img.classList.remove('loaded');
        }
        if (observer) observer.observe(node);
      },
      destroy() { 
        if (observer) observer.unobserve(node);
      }
    };
  }

  let displayFiles = $derived(files.filter(f => settings.showHiddenFiles || !f.is_hidden));
  let isListView = $derived((isSecondary ? settings.secondaryViewMode : settings.primaryViewMode) === 'list');
  let zoomScale = $derived(settings.currentFolderIconSize / 120);
  let itemHeight = $derived(isListView ? Math.max(24, Math.floor(24 * zoomScale)) : settings.currentFolderIconSize + 55);
  let columns = $derived(isListView ? 1 : Math.max(1, Math.floor((containerWidth + 15) / (settings.currentFolderIconSize + 15))));
  let startRow = $derived(Math.max(0, Math.floor(scrollTop / itemHeight) - 20)); 
  let endRow = $derived(Math.min(Math.ceil(displayFiles.length / columns), Math.ceil((scrollTop + containerHeight) / itemHeight) + 25));
  let startIndex = $derived(startRow * columns);
  let endIndex = $derived(Math.min(displayFiles.length, endRow * columns));

  let paddingTop = $derived(startRow * itemHeight);
  let paddingBottom = $derived(Math.max(0, (Math.ceil(displayFiles.length / columns) - endRow) * itemHeight));

  let visibleFiles = $derived(displayFiles.slice(startIndex, endIndex));

  function updateSelection(newSelection: FileItem[]) {
    if (isSecondary) explorer.secondarySelectedFiles = newSelection;
    else explorer.selectedFiles = newSelection;
  }

  // --- Selection & Drag/Drop State ---
  let dragMode = $state<'none' | 'lasso' | 'files'>('none');
  let dragButton = $state(0);
  let startX = $state(0), startY = $state(0), currentX = $state(0), currentY = $state(0);
  let selectionBox = $state<{ left: number, top: number, width: number, height: number } | null>(null);
  let dragBaseSelection = $state<FileItem[]>([]);
  let canvasRef = $state<HTMLCanvasElement | null>(null);
  let isDragThresholdMet = $derived(Math.abs(currentX - startX) > 5 || Math.abs(currentY - startY) > 5);

  $effect(() => {
    const mode = dragMode;
    const box = selectionBox;
    const scroll = scrollTop;

    untrack(() => {
      if (canvasRef && containerWidth && containerHeight) {
        const ctx = canvasRef.getContext('2d');
        if (ctx) {
          ctx.clearRect(0, 0, containerWidth, containerHeight);
          if (mode === 'lasso' && box) {
            ctx.fillStyle = 'rgba(128, 128, 128, 0.2)';
            ctx.strokeStyle = 'rgba(128, 128, 128, 0.5)';
            ctx.lineWidth = 1;
      
            const drawY = box.top - scroll;
            ctx.fillRect(box.left, drawY, box.width, box.height);
            ctx.strokeRect(box.left, drawY, box.width, box.height);
          }
        }
      }
    });
  });

  

  // --- Type-to-Select State ---
  let typeBuffer = $state("");
  let typeTimeout: ReturnType<typeof setTimeout> | undefined;

  // --- Column Resize State ---
  let resizingCol = $state<'name' | 'date' | 'size' | null>(null);
  let colStartX = $state(0);
  let colStartWidth = $state(0);

  function startColResize(e: MouseEvent, col: 'name' | 'date' | 'size') {
    e.preventDefault();
    e.stopPropagation();
    resizingCol = col;
    colStartX = e.clientX;
    colStartWidth = settings.listColumnWidths[col];
  }

  function autoSizeCol(e: MouseEvent, col: 'name' | 'date' | 'size') {
    e.preventDefault();
    e.stopPropagation();
    if (col === 'date') settings.listColumnWidths.date = 160;
    else if (col === 'size') settings.listColumnWidths.size = 120;
    else settings.listColumnWidths.name = 400;
    settings.saveSettings();
  }

  function handleSingleClick(file: FileItem, event: MouseEvent, index: number) {
    const target = event.target as HTMLElement;
    const isAlreadySelected = selectedFiles.some(s => s.path === file.path);
    
    // Ignore clicks on empty padding to preserve lasso
    if (!isAlreadySelected && !target.closest('.icon-wrapper, .tile-name, .list-icon, .file-name-text, .git-badge')) return;
    
    explorer.focusedPane = isSecondary ? 'secondary' : 'primary';
    if (event.ctrlKey || event.metaKey) {
      const exists = selectedFiles.find(f => f.path === file.path);
      updateSelection(exists 
        ? selectedFiles.filter(f => f.path !== file.path) 
        : [...selectedFiles, file]);
    } else if (event.shiftKey && explorer.lastSelectedIndex !== -1) {
      const start = Math.min(explorer.lastSelectedIndex, index);
      const end = Math.max(explorer.lastSelectedIndex, index);
      updateSelection(displayFiles.slice(start, end + 1));
    } else {
      updateSelection([file]);
    }
    explorer.lastSelectedIndex = index;

    if (explorer.previewImagePath !== null || explorer.previewTextContent !== null) {
      handlePreview();
    }
  }

  async function handleDoubleClick(file: FileItem) {
    if (file.is_dir) {
      loadDirectory(file.path, true, false, isSecondary);
    } else {
      try {
        await invoke("open_file", { path: file.path });
      } catch (error) {
        console.error("Failed to open file:", error);
        explorer.errorMessage = `Could not open file: ${error}`;
      }
    }
  }

  function onMouseDown(e: MouseEvent) {
    if (e.button !== 0 && e.button !== 2) return;
    dragButton = e.button;
    explorer.focusedPane = isSecondary ? 'secondary' : 'primary';
    
    const target = e.target as HTMLElement;
    const itemEl = target.closest('.tile, .list-row') as HTMLElement;
    
    const path = itemEl?.dataset.path;
    const isAlreadySelected = path ? selectedFiles.some(f => f.path === path) : false;
    
    // Allow drag if clicking content OR if clicking the padding of an already selected item
    const isClickingItem = target.closest('.icon-wrapper, .tile-name, .list-icon, .file-name-text, .git-badge') || isAlreadySelected;
    
    const rect = containerRef.getBoundingClientRect();
    startX = e.clientX - rect.left + containerRef.scrollLeft;
    startY = e.clientY - rect.top + containerRef.scrollTop;
    currentX = startX;
    currentY = startY;

    if (isClickingItem && itemEl) {
      const path = itemEl.dataset.path;
      if (path && !selectedFiles.some(f => f.path === path) && !e.ctrlKey && !e.shiftKey) {
        const file = displayFiles.find(f => f.path === path);
        if (file) updateSelection([file]);
      }
      dragMode = 'files';
    } else {
      if (e.button === 2) return; // Prevent lasso on right-click
      dragMode = 'lasso';
      if (!e.ctrlKey && !e.metaKey && !e.shiftKey) {
        updateSelection([]);
        dragBaseSelection = [];
      } else {
        dragBaseSelection = [...selectedFiles];
      }
      updateSelectionBox();
    }
  }

  function onMouseMove(e: MouseEvent) {
    if (resizingCol) {
      settings.listColumnWidths[resizingCol] = Math.max(50, colStartWidth + (e.clientX - colStartX));
      return;
    }
    if (dragMode === 'none') return;
    
    const rect = containerRef.getBoundingClientRect();
    currentX = e.clientX - rect.left + containerRef.scrollLeft;
    currentY = e.clientY - rect.top + containerRef.scrollTop;

    if (dragMode === 'lasso') {
      updateSelectionBox();
      calculateLassoSelection();
    } else if (dragMode === 'files' && isDragThresholdMet) {
      document.querySelectorAll('.drag-over').forEach(el => el.classList.remove('drag-over'));
      const hoverEl = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement;
      const dropTarget = hoverEl?.closest('.tile, .list-row') as HTMLElement;
      if (dropTarget && dropTarget.dataset.isdir === 'true') {
        if (!selectedFiles.some(f => f.path === dropTarget.dataset.path)) {
          dropTarget.classList.add('drag-over');
        }
      }
    }
  }

  async function onMouseUp(e: MouseEvent) {
    if (resizingCol) {
      resizingCol = null;
      settings.saveSettings();
      return;
    }
    
    if (dragMode === 'files' && isDragThresholdMet && selectedFiles.length > 0) {
      document.querySelectorAll('.drag-over').forEach(el => el.classList.remove('drag-over'));
      const hoverEl = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement;
      const dropTarget = hoverEl?.closest('.tile, .list-row') as HTMLElement;
      if (dropTarget && dropTarget.dataset.isdir === 'true') {
        const targetPath = dropTarget.dataset.path!;
        if (!selectedFiles.some(f => f.path === targetPath)) {
          if (dragButton === 2) {
            import('$lib/actions').then(m => m.handleDropContextMenu(e, targetPath, isSecondary));
          } else {
            await performDrop(targetPath, e.ctrlKey);
          }
        }
      }
    }
    
    if (dragButton === 2 && dragMode === 'files' && isDragThresholdMet) {
      setTimeout(() => {
        dragMode = 'none';
        selectionBox = null;
      }, 50);
    } else {
      dragMode = 'none';
      selectionBox = null;
    }
  }

  async function performDrop(targetPath: string, isCopy: boolean) {
    for (const file of selectedFiles) {
      const separator = targetPath.endsWith('\\') || targetPath.endsWith('/') ? '' : '\\';
      const dest = `${targetPath}${separator}${file.name}`;
      
      if (dest.startsWith(file.path) || file.path === dest) {
        explorer.errorMessage = `Cannot move ${file.name} into itself.`;
        continue;
      }
      try {
        if (isCopy) {
          await invoke("copy_item", { src: file.path, dest });
        } else {
          await invoke("move_item", { src: file.path, dest });
          explorer.undoStack.push({ type: 'move', src: file.path, dest });
          settings.saveSettings();
        }
      } catch(err) {
        explorer.errorMessage = `Move failed: ${err}`;
      }
    }
    
    import('$lib/actions').then(m => m.loadDirectory(explorer.currentPath, false, false, false));
    if (explorer.isTiledView) {
      const activeTab = explorer.tabs[explorer.activeTabIndex];
      if (activeTab?.secondaryPath) import('$lib/actions').then(m => m.loadDirectory(activeTab.secondaryPath || "C:\\", false, false, true));
    }
  }

  function updateSelectionBox() {
    selectionBox = {
      left: Math.min(startX, currentX),
      top: Math.min(startY, currentY),
      width: Math.abs(currentX - startX),
      height: Math.abs(currentY - startY)
    };
  }

  function calculateLassoSelection() {
    if (!selectionBox) return;
    const currentLassoSelection = new Set<string>();

    const boxLeft = selectionBox.left;
    const boxTop = selectionBox.top;
    const boxRight = selectionBox.left + selectionBox.width;
    const boxBottom = selectionBox.top + selectionBox.height;

    let headerHeight = 0;
    if (isListView) {
      const thead = containerRef.querySelector('thead');
      if (thead) headerHeight = thead.getBoundingClientRect().height;
    }

    for (let i = 0; i < displayFiles.length; i++) {
      let itemTop, itemBottom, itemLeft, itemRight;

      if (isListView) {
        itemTop = headerHeight + (i * itemHeight);
        itemBottom = itemTop + itemHeight;
        itemLeft = 0;
        itemRight = containerWidth;
      } else {
        const row = Math.floor(i / columns);
        const col = i % columns;
        const tileW = settings.currentFolderIconSize;
        const tileH = settings.currentFolderIconSize + 40;
        const gap = 15;
        itemTop = row * itemHeight;
        itemBottom = itemTop + tileH;
        itemLeft = col * (tileW + gap);
        itemRight = itemLeft + tileW;
      }

      const isIntersecting = !(
        itemLeft > boxRight ||
        itemRight < boxLeft ||
        itemTop > boxBottom ||
        itemBottom < boxTop
      );

      if (isIntersecting) currentLassoSelection.add(displayFiles[i].path);
    }

    const baseSet = new Set(dragBaseSelection.map(f => f.path));
    updateSelection(displayFiles.filter(f => baseSet.has(f.path) || currentLassoSelection.has(f.path)));
  }
</script>

<svelte:window 
  onmouseup={onMouseUp} 
  onmousemove={onMouseMove} 
  onkeydown={(e) => {
    if ((isSecondary && explorer.focusedPane !== 'secondary') || (!isSecondary && explorer.focusedPane === 'secondary')) return;
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

    if (['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.key)) {
      e.preventDefault();
      let idx = explorer.lastSelectedIndex !== -1 ? Math.max(0, explorer.lastSelectedIndex) : 0;
      
      if (e.key === 'ArrowRight') idx++;
      else if (e.key === 'ArrowLeft') idx--;
      else if (e.key === 'ArrowDown') idx += columns;
      else if (e.key === 'ArrowUp') idx -= columns;

      idx = Math.max(0, Math.min(idx, displayFiles.length - 1));
      const file = displayFiles[idx];

      if (file) {
        updateSelection(e.shiftKey && explorer.lastSelectedIndex !== -1 
            ? displayFiles.slice(Math.min(explorer.lastSelectedIndex, idx), Math.max(explorer.lastSelectedIndex, idx) + 1)
            : [file]);
        if (!e.shiftKey) explorer.lastSelectedIndex = idx;
        
        const row = Math.floor(idx / columns);
        const pos = row * itemHeight;
        if (pos < containerRef.scrollTop) containerRef.scrollTop = pos;
        if (pos > containerRef.scrollTop + containerHeight - itemHeight - 40) containerRef.scrollTop = pos - containerHeight + itemHeight + 40;
      }
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
      const char = e.key.toLowerCase();
      const currentSelectedName = explorer.lastSelectedIndex !== -1 && displayFiles[explorer.lastSelectedIndex] ? displayFiles[explorer.lastSelectedIndex].name.toLowerCase() : "";
      
      if (typeBuffer === char || (typeBuffer === "" && currentSelectedName.startsWith(char))) {
        typeBuffer = char;
        clearTimeout(typeTimeout);
        typeTimeout = setTimeout(() => { typeBuffer = ""; }, 750);
        
        let matchIndex = displayFiles.findIndex((f, i) => i > explorer.lastSelectedIndex && f.name.toLowerCase().startsWith(char));
        if (matchIndex === -1) matchIndex = displayFiles.findIndex(f => f.name.toLowerCase().startsWith(char)); 
        
        if (matchIndex !== -1) {
          updateSelection([displayFiles[matchIndex]]);
          explorer.lastSelectedIndex = matchIndex;
          const row = Math.floor(matchIndex / columns);
          const pos = row * itemHeight;
          if (pos < containerRef.scrollTop) containerRef.scrollTop = pos;
          if (pos > containerRef.scrollTop + containerHeight - itemHeight - 40) containerRef.scrollTop = pos - containerHeight + itemHeight + 40;
        }
      } else {
        typeBuffer += char;
        clearTimeout(typeTimeout);
        typeTimeout = setTimeout(() => { typeBuffer = ""; }, 750);
        
        const matchIndex = displayFiles.findIndex(f => f.name.toLowerCase().startsWith(typeBuffer));
        if (matchIndex !== -1) {
          updateSelection([displayFiles[matchIndex]]);
          explorer.lastSelectedIndex = matchIndex;
          const row = Math.floor(matchIndex / columns);
          const pos = row * itemHeight;
          if (pos < containerRef.scrollTop) containerRef.scrollTop = pos;
          if (pos > containerRef.scrollTop + containerHeight - itemHeight - 40) containerRef.scrollTop = pos - containerHeight + itemHeight + 40;
        }
      }
    }
  }}
/>

<div class="file-container" class:is-interacting={resizingCol !== null || dragMode !== 'none'} style:--zoom-scale={zoomScale} bind:this={containerRef} 
  bind:clientWidth={_containerWidth} 
  bind:clientHeight={containerHeight}
  onscroll={handleScroll} role="presentation" onmousedown={onMouseDown} ondragstart={(e) => e.preventDefault()}
  onclick={(e: MouseEvent) => {
    const target = e.target as HTMLElement;
    const itemEl = target.closest('.tile, .list-row') as HTMLElement;
    if (itemEl && itemEl.dataset.path) {
      const index = parseInt(itemEl.dataset.index || '-1');
      const file = displayFiles[index];
      if (file && file.path === itemEl.dataset.path) {
        handleSingleClick(file, e, index);
      }
    }
  }}
  ondblclick={(e: MouseEvent) => {
    const target = e.target as HTMLElement;
    const itemEl = target.closest('.tile, .list-row') as HTMLElement;
    if (itemEl && itemEl.dataset.path) {
      if (!target.closest('.icon-wrapper, .tile-name, .list-icon, .file-name-text, .git-badge')) return;
      const index = parseInt(itemEl.dataset.index || '-1');
      const file = displayFiles[index];
      if (file && file.path === itemEl.dataset.path) {
        handleDoubleClick(file);
      }
    }
  }}
  onkeydown={(e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      const target = e.target as HTMLElement;
      const itemEl = target.closest('.tile, .list-row') as HTMLElement;
      if (itemEl && itemEl.dataset.path) {
        e.preventDefault(); e.stopPropagation();
        const index = parseInt(itemEl.dataset.index || '-1');
        const file = displayFiles[index];
        if (file && file.path === itemEl.dataset.path) {
          if (file.is_dir) {
            if (e.ctrlKey || e.metaKey) import('$lib/actions').then(m => m.openInNewTab(file.path));
            else if (e.shiftKey) import('$lib/actions').then(m => m.openInOtherPane(file.path));
            else handleDoubleClick(file);
          } else handleDoubleClick(file);
        }
      }
    }
  }}
  oncontextmenu={(e: MouseEvent) => {
    e.preventDefault(); e.stopPropagation();
    if (dragMode !== 'none' && isDragThresholdMet) return;

    const target = e.target as HTMLElement;
    const itemEl = target.closest('.tile, .list-row') as HTMLElement;

    if (itemEl && itemEl.dataset.path) {
      const index = parseInt(itemEl.dataset.index || '-1');
      const file = displayFiles[index];
      if (file && file.path === itemEl.dataset.path) {
        const isClickingItemContent = target.closest('.icon-wrapper, .tile-name, .list-icon, .file-name-text, .git-badge');
        const isAlreadySelected = selectedFiles.some(s => s.path === file.path);

        if (isAlreadySelected) {
          import('$lib/actions').then(m => m.handleItemContextMenu(e, file, isSecondary));
        } else if (isClickingItemContent) {
          updateSelection([file]);
          import('$lib/actions').then(m => m.handleItemContextMenu(e, file, isSecondary));
        } else {
          updateSelection([]);
          import('$lib/actions').then(m => m.handleBackgroundContextMenu(e, isSecondary));
        }
        return;
      }
    }
    
    updateSelection([]);
    import('$lib/actions').then(m => m.handleBackgroundContextMenu(e, isSecondary));
  }}
>
  
  <canvas 
    bind:this={canvasRef} 
    width={containerWidth} 
    height={containerHeight} 
    style="pointer-events: none; position: absolute; top: {scrollTop}px; left: 0; z-index: 100;"
  ></canvas>

  {#if dragMode === 'files' && isDragThresholdMet && selectedFiles.length > 0}
    {@const firstFile = selectedFiles[0]}
    {@const showThumb = settings.enableThumbnails && !firstFile.is_dir && isImage(firstFile.name)}
    <div class="drag-ghost" style="top: {currentY}px; left: {currentX + 15}px;">
      <div class="ghost-icon-wrapper">
        {#if showThumb}
          <img src={convertFileSrc(firstFile.path, 'thumbnail')} alt="thumb" class="ghost-img" />
        {:else}
          <img src={getSystemIconSrc(firstFile.path, firstFile.is_dir)} alt="icon" class="ghost-img" />
        {/if}
      </div>
      <span class="ghost-count">{selectedFiles.length} item{selectedFiles.length !== 1 ? 's' : ''}</span>
    </div>
  {/if}
  
  {#if displayFiles.length === 0}
    {#if isSecondary ? explorer.isLoadingSecondary : explorer.isLoadingPrimary}
      <div class="empty-state">
        <div class="spinner"></div>
        <p>Loading...</p>
      </div>
    {:else}
      <div class="empty-state">
        <div class="empty-icon">
          <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path></svg>
        </div>
        <p>{filterQuery ? "No items match your filter" : "This folder is empty"}</p>
      </div>
    {/if}
  {:else if (isSecondary ? settings.secondaryViewMode : settings.primaryViewMode) === 'list'}
    <table class="list-view-table {scrollTop > 0 ? 'is-scrolled' : ''}">
      <thead>
        <tr>
          <th style="width: {settings.listColumnWidths.name}px" role="button" tabindex="0" onclick={(e) => { if ((e.target as HTMLElement).classList.contains('col-resizer')) return; import('$lib/actions').then(m => m.setSortMode('name')) }} >
            Name {settings.sortBy === 'name' ? (settings.sortAscending ? '▲' : '▼') : ''}
            <div class="col-resizer" onmousedown={(e) => startColResize(e, 'name')} ondblclick={(e) => autoSizeCol(e, 'name')} role="separator" tabindex="-1"></div>
          </th>
          <th style="width: {settings.listColumnWidths.date}px" role="button" tabindex="0" onclick={(e) => { if ((e.target as HTMLElement).classList.contains('col-resizer')) return; import('$lib/actions').then(m => m.setSortMode('date')) }} >
            Date Modified {settings.sortBy === 'date' ? (settings.sortAscending ? '▲' : '▼') : ''}
            <div class="col-resizer" onmousedown={(e) => startColResize(e, 'date')} ondblclick={(e) => autoSizeCol(e, 'date')} role="separator" tabindex="-1"></div>
          </th>
          <th style="width: {settings.listColumnWidths.size}px" role="button" tabindex="0" onclick={(e) => { if ((e.target as HTMLElement).classList.contains('col-resizer')) return; import('$lib/actions').then(m => m.setSortMode('size')) }} >
            Size {settings.sortBy === 'size' ? (settings.sortAscending ? '▲' : '▼') : ''}
            <div class="col-resizer" onmousedown={(e) => startColResize(e, 'size')} ondblclick={(e) => autoSizeCol(e, 'size')} role="separator" tabindex="-1"></div>
          </th>
        </tr>
      </thead>
      <tbody>
        {#if paddingTop > 0}<tr style="height: {paddingTop}px; pointer-events: none;"><td colspan="3" style="padding: 0; border: none;"></td></tr>{/if}
        
        {#each visibleFiles as file, index}
          <FileItemComponent 
            {file}
            index={startIndex + index}
            isListView={true}
            isSelected={explorer.selectedFiles.some(s => s.path === file.path)}
            isCut={explorer.clipboardAction === 'cut' && explorer.clipboardFiles.some(c => c.path === file.path)}
            isSingleSelected={false}
            {filterQuery}
            lazyLoadAction={lazyLoad}
          />
        {/each}

        {#if paddingBottom > 0}<tr style="height: {paddingBottom}px; pointer-events: none;"><td colspan="3" style="padding: 0; border: none;"></td></tr>{/if}
      </tbody>
    </table>
  {:else}
    <div class="grid-layout" style:--tile-size="{settings.currentFolderIconSize}px" style:--columns={columns} style:padding-top="{paddingTop}px" style:padding-bottom="calc({paddingBottom}px + 2rem)">
      {#each visibleFiles as file, index}
        <FileItemComponent 
          {file}
          index={startIndex + index}
          isListView={false}
          isSelected={selectedFiles.some(s => s.path === file.path)}
          isCut={explorer.clipboardAction === 'cut' && explorer.clipboardFiles.some(c => c.path === file.path)}
          isSingleSelected={selectedFiles.length === 1 && selectedFiles[0].path === file.path}
          {filterQuery}
          lazyLoadAction={lazyLoad}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .file-container { flex-grow: 1;
overflow-y: auto; background: transparent; position: relative; user-select: none; }

.file-container.is-interacting {
  will-change: transform;
}
  .file-container img {
    -webkit-user-drag: none;
  }
  .grid-layout {
    display: grid;
    grid-template-columns: repeat(var(--columns), minmax(0, 1fr));
    gap: 15px;
    padding-bottom: 2rem;
  }
  .list-view-table {
    width: 100%;
    border-collapse: collapse; 
    text-align: left;
    font-size: calc(0.9rem * var(--zoom-scale));
    table-layout: fixed;
  }
  .list-view-table th {
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-color);
    color: var(--text-muted);
    position: sticky;
    top: 0;
    background: var(--bg-solid); 
    z-index: 10;
    font-weight: normal;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: box-shadow 0.2s ease;
  }

  .list-view-table.is-scrolled th {
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }
  .list-view-table td {
    padding: calc(2px * var(--zoom-scale)) 8px;
    user-select: none;
    font-size: calc(0.85rem * var(--zoom-scale));
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .list-row {
    content-visibility: auto;
    contain-intrinsic-size: calc(24px * var(--zoom-scale)); 
  }
  .col-resizer {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
    background: transparent;
    z-index: 15;
  }
  .col-resizer:hover, .col-resizer:active {
    background: var(--accent);
    opacity: 0.5;
  }
  .drag-ghost {
    position: absolute;
    pointer-events: none;
    background: var(--bg-menu);
    border: 1px solid var(--accent);
    border-radius: 6px;
    padding: 6px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    z-index: 2000;
    font-size: 0.9rem;
    color: var(--text-main);
    backdrop-filter: blur(8px);
    transform: translate(5px, 5px); /* Position it right at the tip of the cursor */
  }
  :global(.drag-over) {
    background: rgba(0, 96, 223, 0.3) !important;
    border: 1px solid var(--accent) !important;
    transform: scale(1.02);
  }
  .empty-state {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    text-align: center;
    user-select: none;
    animation: pop-in 0.2s cubic-bezier(0.2, 0, 0, 1) forwards;
  }
  .empty-icon {
    color: var(--border-color);
    margin-bottom: 12px;
    opacity: 0.6;
  }
  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid var(--border-color);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s cubic-bezier(0.4, 0, 0.2, 1) infinite;
    margin-bottom: 12px;
  }
  @keyframes spin { 
    to { transform: rotate(360deg); } 
  }
  .empty-state p {
    margin: 0;
    font-size: 0.95rem;
    letter-spacing: 0.02em;
  }
  .ghost-icon-wrapper {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .ghost-img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 4px;
  }
</style>