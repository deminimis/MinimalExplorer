<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { settings } from '$lib/settings.svelte';
  import { loadDirectory, handleSidebarContextMenu } from '$lib/actions';
  import { explorer, type FileItem } from '$lib/explorer.svelte';
  import { getSystemIconSrc, formatSize } from '$lib/utils';

  interface QuickAccessItem { name: string; path: string; is_drive: boolean; total_space?: number; free_space?: number; }
  
  let quickAccessItems = $state<QuickAccessItem[]>([]);
  let wslItems = $state<QuickAccessItem[]>([]);
  let isResizing = $state(false);
  let isHovered = $state(false);
// --- Tree State ---
  let expandedFolders = $state<Record<string, boolean>>({});
  let folderChildren = $state<Record<string, FileItem[]>>({});

  onMount(() => {
    const loadDrives = () => {
      // Fire all requests independently.
      invoke<QuickAccessItem[]>('get_quick_access')
        .then(res => quickAccessItems = res)
        .catch(e => console.error(e));

      invoke<QuickAccessItem[]>('get_wsl_distros')
        .then(res => wslItems = res)
        .catch(e => console.error(e));
    };
    
    loadDrives(); 
const interval = setInterval(loadDrives, 30000);
    
    return () => clearInterval(interval);
  });

  async function toggleFolder(e: Event, path: string) {
    e.stopPropagation();
    // Stop click from loading the directory
    if (expandedFolders[path]) {
      expandedFolders[path] = false;
    } else {
      expandedFolders[path] = true;
      if (!folderChildren[path]) {
        try {
          let fetchPath = path;
          if (path.startsWith('\\\\') && !path.endsWith('\\')) {
            fetchPath += '\\';
          }
          folderChildren[path] = await invoke('get_directories', { path: fetchPath, showGitBadges: settings.showGitBadges });
        } catch (err) {
          console.error("Failed to load directories:", err);
        }
      }
    }
  }

  function startResize(e: MouseEvent) {
    isResizing = true;
    document.addEventListener('mousemove', handleResize);
    document.addEventListener('mouseup', stopResize);
  }

  function handleResize(e: MouseEvent) {
    if (!isResizing) return;
    const newWidth = Math.max(150, Math.min(e.clientX, 400));
    settings.sidebarWidth = newWidth;
  }

  function stopResize() {
    isResizing = false;
    settings.saveSettings();
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
  }

  onDestroy(() => {
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
  });


  // --- Custom Quick Access Drag State ---
  let qaDragIndex = $state<number | null>(null);
  let qaHoverIndex = $state<number | null>(null);
  let qaDragStartY = $state(0);
  let qaDragCurrentX = $state(0);
  let qaDragCurrentY = $state(0);
  let isDraggingQA = $derived(qaDragIndex !== null && Math.abs(qaDragCurrentY - qaDragStartY) > 5);

  function handleQAMouseDown(e: MouseEvent, index: number) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('.chevron')) return; // Allow clicking the expand arrow without dragging
    
    qaDragIndex = index;
    qaDragStartY = e.clientY;
    qaDragCurrentY = e.clientY;
    qaDragCurrentX = e.clientX;
  }

  function handleQAWindowMouseMove(e: MouseEvent) {
    if (qaDragIndex === null) return;
    qaDragCurrentX = e.clientX;
    qaDragCurrentY = e.clientY;

    if (isDraggingQA) {
      // Find what sidebar item is sitting behind the cursor
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest('.qa-item') as HTMLElement;
      if (el && el.dataset.index) {
        const idx = parseInt(el.dataset.index);
        qaHoverIndex = (idx !== qaDragIndex) ? idx : null;
      } else {
        qaHoverIndex = null;
      }
    }
  }

  function handleQAWindowMouseUp(e: MouseEvent) {
    if (qaDragIndex !== null) {
      if (isDraggingQA && qaHoverIndex !== null) {
        const newPinned = [...settings.pinnedFolders];
        const [moved] = newPinned.splice(qaDragIndex, 1);
        newPinned.splice(qaHoverIndex, 0, moved);
        
        settings.pinnedFolders = newPinned;
        settings.saveSettings();
      }
      qaDragIndex = null;
      qaHoverIndex = null;
    }
  }
  
  let isVisible = $derived(settings.showSidebar || (settings.sidebarHoverReveal && isHovered));
</script>

<svelte:window onmousemove={handleQAWindowMouseMove} onmouseup={handleQAWindowMouseUp} />

{#if isDraggingQA && qaDragIndex !== null}
  <div class="drag-ghost" style="top: {qaDragCurrentY}px; left: {qaDragCurrentX + 15}px;">
    <span class="icon" style="font-size: 1.2rem;">📌</span>
    <span class="tree-name">{settings.pinnedFolders[qaDragIndex].name}</span>
  </div>
{/if}

{#if settings.sidebarHoverReveal && !settings.showSidebar}
  <div class="sidebar-edge-detector" role="presentation" onmouseenter={() => isHovered = true}>
    <span class="detector-arrow">›</span>
  </div>
{/if}

<div 
  class="sidebar {settings.sidebarHoverReveal && !settings.showSidebar ? 'hover-mode' : ''} {isVisible ? 'visible' : ''}" 
  class:is-resizing={isResizing}
  style:width="{settings.sidebarWidth}px"
  role="presentation"
  onmouseleave={() => isHovered = false}
>
  <div class="sidebar-content">
    <div class="sidebar-section">
      <h3>Quick Access</h3>
      <div class="nav-list" ondragover={(e) => e.preventDefault()}>
        {#each settings.pinnedFolders as item, i (item.path)}
          {@const isDir = item.is_dir !== false}
          {@const isExpanded = expandedFolders[item.path]}
          {@const children = folderChildren[item.path] || []}
          {@const displayChildren = children.filter(f => settings.showHiddenFiles || !f.is_hidden)}

          <div
            class="nav-item qa-item {explorer.currentPath === item.path ? 'selected' : ''} {qaHoverIndex === i ? 'qa-drop-target' : ''} {isDraggingQA && qaDragIndex === i ? 'qa-dragging-source' : ''}"
            data-index={i}
            role="button" 
            tabindex="0"
            onmousedown={(e) => handleQAMouseDown(e, i)}
            onclick={() => isDir ? loadDirectory(item.path, true) : invoke('open_file', { path: item.path })} 
            onkeydown={(e) => e.key === 'Enter' && (isDir ? loadDirectory(item.path, true) : invoke('open_file', { path: item.path }))}
            oncontextmenu={(e) => handleSidebarContextMenu(e, item.path)}
          >
            {#if isDir}
              <button class="chevron {isExpanded ? 'expanded' : ''}" onclick={(e) => toggleFolder(e, item.path)} aria-label="Toggle Folder">›</button>
            {:else}
              <span class="chevron placeholder"></span>
            {/if}
            <span class="icon" style="width: 16px; height: 16px; display: flex;"><img src={getSystemIconSrc(item.path, isDir)} alt="" style="width: 100%; height: 100%; object-fit: contain;" /></span>
            <span class="tree-name" title={item.name}>{item.name}</span>
          </div>

          {#if isDir && isExpanded}
            {#each displayChildren as child (child.path)}
              {@render treeNode(child, 1, false)}
            {/each}
          {/if}
        {/each}
      </div>
    </div>

    {#snippet treeNode(item: {name: string, path: string, total_space?: number, free_space?: number}, depth: number, isDrive: boolean = false)}
      {@const isExpanded = expandedFolders[item.path]}
      {@const children = folderChildren[item.path] || []}
      {@const displayChildren = children.filter(f => settings.showHiddenFiles || !f.is_hidden)}
      
      <div 
        class="nav-item {explorer.currentPath === item.path ? 'selected' : ''}"
        role="button"
        tabindex="0"
        style="padding-left: {4 + depth * 16}px"
        onclick={() => loadDirectory(item.path, true)}
        onkeydown={(e) => e.key === 'Enter' && loadDirectory(item.path, true)}
        oncontextmenu={(e) => handleSidebarContextMenu(e, item.path)}
      >
        <button class="chevron {isExpanded ? 'expanded' : ''}" onclick={(e) => toggleFolder(e, item.path)} aria-label="Toggle Folder">
          ›
        </button>
        <span class="icon" style="width: 16px; height: 16px; display: flex;">
          <img src={getSystemIconSrc(item.path, true)} alt="" style="width: 100%; height: 100%; object-fit: contain;" />
        </span>
        <div class="tree-name-wrapper">
          <span class="tree-name" title={item.name}>{item.name}</span>
          {#if isDrive && item.total_space != null && item.free_space != null}
            {@const used = item.total_space - item.free_space}
            {@const percent = (used / item.total_space) * 100}
            <div class="drive-capacity">
              <div class="capacity-bar"><div class="capacity-fill {percent > 90 ? 'danger' : ''}" style="width: {percent}%"></div></div>
              <span class="capacity-text">{formatSize(item.free_space, false)} free of {formatSize(item.total_space, false)}</span>
            </div>
          {/if}
        </div>
      </div>

      {#if isExpanded}
        {#each displayChildren as child (child.path)}
          {@render treeNode(child, depth + 1, false)}
        {/each}
      {/if}
    {/snippet}

    <div class="sidebar-section">
      <div 
        class="nav-item {explorer.currentPath === 'This PC' ? 'selected' : ''}"
        role="button"
        tabindex="0"
        onclick={() => loadDirectory('This PC', true)}
        onkeydown={(e) => e.key === 'Enter' && loadDirectory('This PC', true)}
        style="margin: 15px 8px 4px 8px; padding-left: 6px;"
      >
        <span class="icon" style="width: 16px; height: 16px; display: flex;">
          <img src={getSystemIconSrc("C:\\", true)} alt="" style="width: 100%; height: 100%; object-fit: contain;" />
        </span>
        <span class="tree-name" style="font-size: 0.85rem; font-weight: 600; letter-spacing: 0.02em;">This PC</span>
      </div>
      
      <div style="height: 1px; background: var(--border-color); margin: 0 16px 8px 16px; opacity: 0.5;"></div>

      <div class="nav-list">
        {#each quickAccessItems.filter(i => i.is_drive) as item}
          {@render treeNode(item, 0, true)}
        {/each}
      </div>
    </div>

    {#if wslItems.length > 0}
      <div class="sidebar-section">
        <h3>Linux (WSL)</h3>
        <div class="nav-list">
          {#each wslItems as item}
            {@render treeNode(item, 0, false)}
          {/each}
        </div>
      </div>
    {/if}
  </div>
  <div class="resizer" role="separator" tabindex="-1" onmousedown={startResize}></div>
</div>

<style>
  .sidebar-edge-detector {
    position: fixed;
    left: 0;
    top: 45px; /* Sit just below the titlebar */
    bottom: 0;
    width: 40px; /* Span from the absolute screen edge right up to the files */
    z-index: 90;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: flex-end; /* Keep the chevron arrow near the content */
    padding-right: 6px;
    cursor: pointer;
    transition: background 0.2s ease;
  }
  .sidebar-edge-detector:hover {
    background: rgba(128, 128, 128, 0.1);
  }
  .detector-arrow {
    color: var(--text-main);
    font-size: 1.5rem;
    user-select: none;
  }
  .sidebar {
  background-color: rgba(0, 0, 0, 0.03);
  border-right: 1px solid rgba(128, 128, 128, 0.3);
  display: flex;
  flex-shrink: 0;
  transition: transform 0.2s cubic-bezier(0.2, 0, 0, 1), background-color 0.2s ease;
  overflow: hidden;
  transform: translateZ(0);
}

.sidebar.is-resizing {
  will-change: transform;
}

:global([data-theme="dark"]) .sidebar {
  background-color: rgba(0, 0, 0, 0.15);
}

.sidebar:hover, .sidebar.hover-mode {
  background-color: var(--bg-solid) !important;
}

.sidebar.hover-mode {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    z-index: 100;
    transform: translateX(-100%);
    box-shadow: 1px 0 8px rgba(0,0,0,0.2);
  }
  .sidebar.hover-mode.visible {
    transform: translateX(0);
  }
  .sidebar-content {
    flex-grow: 1;
    overflow-y: auto;
    padding: 10px;
  }
  .resizer {
    width: 4px;
    cursor: col-resize;
    background: transparent;
  }
  .sidebar-section h3 {
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin: 15px 0 8px 10px;
  }
  .nav-list {
    display: flex;
    flex-direction: column;
    margin: 0 8px 15px 8px; 
  }
  .nav-item {
    padding: 6px 12px 6px 6px; 
    margin-bottom: 2px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--text-main);
    display: flex;
    align-items: center;
    gap: 8px;
    user-select: none;
  }
  .nav-item:hover {
    background: var(--bg-hover);
  }
  .nav-item.selected {
    background: rgba(128, 128, 128, 0.2);
    color: var(--text-main);
  }
  .chevron {
    background: transparent;
    border: none;
    color: var(--text-muted);
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    cursor: pointer;
    border-radius: 4px;
    transition: transform 0.1s;
    padding: 0;
    margin: 0;
  }
  .chevron:hover {
    background: rgba(128,128,128,0.2);
    color: var(--text-main);
  }
  .chevron.expanded {
    transform: rotate(90deg);
  }
  .chevron.placeholder {
    pointer-events: none;
    opacity: 0;
  }
  .tree-name-wrapper {
    display: flex;
    flex-direction: column;
    flex-grow: 1;
    overflow: hidden;
  }
  .tree-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .drive-capacity {
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin-top: 4px;
    margin-bottom: 2px;
  }
  .capacity-bar {
    height: 4px;
    background: var(--border-color);
    border-radius: 2px;
    overflow: hidden;
    width: 90%;
  }
  .capacity-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
  }
  .capacity-fill.danger {
    background: #e81123;
  }
  .capacity-text {
    font-size: 0.7rem;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .nav-item.drag-over-target {
    border-top: 2px solid var(--accent);
    border-radius: 0 0 6px 6px;
  }
  .qa-drop-target {
    border-top: 2px solid var(--accent);
    border-radius: 0 0 6px 6px;
  }
  .qa-dragging-source {
    opacity: 0.4;
  }
  .drag-ghost {
    position: fixed;
    pointer-events: none; /* lets the mouse see through to the elements beneath */
    background: var(--bg-menu);
    border: 1px solid var(--accent);
    border-radius: 6px;
    padding: 6px 12px;
    display: flex;
    align-items: center;
    gap: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
    z-index: 3000;
    font-size: 0.9rem;
    color: var(--text-main);
    backdrop-filter: blur(8px);
    transform: translate(0, -50%);
  }
</style>