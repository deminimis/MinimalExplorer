<script lang="ts">
  import { explorer } from '$lib/explorer.svelte';
  import { settings } from '$lib/settings.svelte';
  import { ui } from '$lib/ui.svelte';
  import { marked } from 'marked';
  import { loadDirectory, handleCopy, handleCut, handlePaste, handleCompress, handleExtractHere, handleExtractToFolder, cancelSearch } from '$lib/actions';
  
  import FileContainer from '$lib/components/FileContainer.svelte';
  import TabBar from '$lib/components/TabBar.svelte';
  import AddressBar from '$lib/components/AddressBar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import TerminalPane from '$lib/components/TerminalPane.svelte';
  import PreferencesModal from '$lib/components/PreferencesModal.svelte';
  import CreateModal from '$lib/components/CreateModal.svelte';
  import ContextMenu from '$lib/components/ContextMenu.svelte';
  import HotkeysModal from '$lib/components/HotkeysModal.svelte';
  import SearchModal from '$lib/components/SearchModal.svelte';
  import ArchiveModal from '$lib/components/ArchiveModal.svelte';
  import { listen } from "@tauri-apps/api/event";
  import { invoke, convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { sortFiles } from '$lib/actions';
  import DOMPurify from 'isomorphic-dompurify';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
  import PropertiesModal from '$lib/components/PropertiesModal.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import type { FileItem } from '$lib/explorer.svelte';





  type WatchEvent = { watched_path: string, item: FileItem | null, deleted_path: string | null };

  let paneSplitRatio = $state(50);
  let isResizingPane = $state(false);

  function startPaneResize(e: MouseEvent) {
    isResizingPane = true;
    document.addEventListener('mousemove', handlePaneResize);
    document.addEventListener('mouseup', stopPaneResize);
  }

  function handlePaneResize(e: MouseEvent) {
    if (!isResizingPane) return;
    const wrapper = document.querySelector('.panes-wrapper') as HTMLElement;
    if (!wrapper) return;
    const rect = wrapper.getBoundingClientRect();
    if (settings.splitDirection === 'vertical') {
      let newRatio = ((e.clientX - rect.left) / rect.width) * 100;
      paneSplitRatio = Math.max(10, Math.min(newRatio, 90));
    } else {
      let newRatio = ((e.clientY - rect.top) / rect.height) * 100;
      paneSplitRatio = Math.max(10, Math.min(newRatio, 90));
    }
  }

  function stopPaneResize() {
    isResizingPane = false;
    document.removeEventListener('mousemove', handlePaneResize);
    document.removeEventListener('mouseup', stopPaneResize);
  }

  import { onDestroy } from 'svelte';
  onDestroy(() => {
    document.removeEventListener('mousemove', handlePaneResize);
    document.removeEventListener('mouseup', stopPaneResize);
  });

  function checkHotkey(e: KeyboardEvent, hotkeyStr: string) {
    const keys = hotkeyStr.toLowerCase().split('+');
    const needsCtrl = keys.includes('ctrl');
    const needsShift = keys.includes('shift');
    const needsAlt = keys.includes('alt');
    const mainKey = keys.filter(k => !['ctrl', 'shift', 'alt', 'meta'].includes(k))[0];

    if ((e.ctrlKey || e.metaKey) !== needsCtrl) return false;
    if (e.shiftKey !== needsShift) return false;
    if (e.altKey !== needsAlt) return false;
    if (e.key.toLowerCase() !== mainKey) return false;
    return true;
  }

  function handleWindowKeydown(e: KeyboardEvent) {
    const hk = settings.hotkeys;

    // Command Palette Trigger (Ctrl+Shift+P or typing '>')
    if ((e.key.toLowerCase() === 'p' && e.ctrlKey && e.shiftKey) || (e.key === '>' && !(e.target instanceof HTMLInputElement))) {
      e.preventDefault();
      ui.paletteInput = ">";
      ui.showCommandPalette = true;
      return;
    }

    // Force intercept Filter/Search globally before inputs or the browser can grab it
    if (checkHotkey(e, hk.focusFilter)) { 
      e.preventDefault(); 
      e.stopPropagation(); 
      ui.triggerFilterFocus++; 
      return; 
    }

    // only block hotkeys that interfere with text editing
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      if (e.key === 'Escape') {
        ui.closeContextMenu();
        if (explorer.isSearching) cancelSearch();
        e.target.blur();
        return;
      }
      
      // Allow specific global UI hotkeys to pass through
      const isGlobalHotkey = checkHotkey(e, hk.openSearch) || checkHotkey(e, hk.toggleSidebar) || 
                             checkHotkey(e, hk.toggleTerminal) || checkHotkey(e, hk.newTab) || 
                             checkHotkey(e, hk.closeTab) || checkHotkey(e, hk.nextTab) || 
                             checkHotkey(e, hk.prevTab) || checkHotkey(e, hk.switchPane) || 
                             checkHotkey(e, hk.fullScreen) || checkHotkey(e, hk.focusAddress) || 
                             (e.key.toLowerCase() === 'f' && (e.ctrlKey || e.metaKey)); // Command Palette

      if (!isGlobalHotkey) {
        return; // Stop execution to let typing, Ctrl+A, Ctrl+C, Delete, and arrow keys function normally inside the input
      }
    }

    if (e.key === 'Escape') {
      ui.closeContextMenu();
      explorer.isEditingPreview = false;
      if (explorer.isSearching) cancelSearch();
      return;
    }

    const activeTab = explorer.tabs[explorer.activeTabIndex] || {};
    const isSecondary = explorer.focusedPane === 'secondary';
    const targetPath = isSecondary ? (activeTab.secondaryPath || "C:\\") : explorer.currentPath;

    // Helper for active files
    const files = isSecondary ? explorer.secondaryFiles : explorer.files;
    const query = isSecondary ? explorer.secondaryFilterQuery : explorer.filterQuery;
    const displayFiles = files.filter(f => (settings.showHiddenFiles || !f.is_hidden) && f.name.toLowerCase().includes(query.toLowerCase()));
    
    const currentSel = isSecondary ? explorer.secondarySelectedFiles : explorer.selectedFiles;
    const updateSel = (newSel: FileItem[]) => isSecondary ? (explorer.secondarySelectedFiles = newSel) : (explorer.selectedFiles = newSel);

    // Selections
    if (checkHotkey(e, hk.selectAll)) { e.preventDefault(); updateSel([...displayFiles]); return; }
    if (checkHotkey(e, hk.invertSelection)) { e.preventDefault(); updateSel(displayFiles.filter(f => !currentSel.some(s => s.path === f.path))); return; }
    if (checkHotkey(e, hk.selectSameType)) {
      e.preventDefault();
      if (currentSel.length > 0) {
        const exts = new Set(currentSel.map(f => f.name.includes('.') ? f.name.split('.').pop()!.toLowerCase() : ''));
        updateSel(displayFiles.filter(f => exts.has(f.name.includes('.') ? f.name.split('.').pop()!.toLowerCase() : '')));
      }
      return;
    }

    // Navigation
    if (checkHotkey(e, hk.goBack)) {
      e.preventDefault();
      const history = isSecondary ? (activeTab.secondaryHistoryStack || []) : explorer.historyStack;
      if (history.length > 1) loadDirectory(history[history.length - 2], false, false, isSecondary);
      return;
    }
    if (checkHotkey(e, hk.goForward)) {
      e.preventDefault();
      const forward = isSecondary ? (activeTab.secondaryForwardStack || []) : explorer.forwardStack;
      if (forward.length > 0) loadDirectory(forward[0], false, false, isSecondary);
      return;
    }
    if (checkHotkey(e, hk.goUp)) {
      e.preventDefault();
      const segments = targetPath.split(/[/\\]/).filter(Boolean);
      if (segments.length > 1) {
        segments.pop();
        let parentPath = segments.join("\\");
        if (parentPath.length === 2 && parentPath.endsWith(":")) parentPath += "\\";
        loadDirectory(parentPath, true, false, isSecondary);
      }
      return;
    }
    if (checkHotkey(e, hk.refresh)) { e.preventDefault(); loadDirectory(targetPath, false, false, isSecondary); return; }

    // Tabs & UI Toggles
    if (checkHotkey(e, hk.newTab)) { e.preventDefault(); explorer.addNewTab(); loadDirectory(explorer.tabs[explorer.activeTabIndex].path, true, true, false); return; }
    if (checkHotkey(e, hk.closeTab)) { e.preventDefault(); if(explorer.tabs.length > 1) { explorer.closeTab(explorer.activeTabIndex); loadDirectory(explorer.tabs[explorer.activeTabIndex].path, false, false, false); } return; }
    if (checkHotkey(e, hk.nextTab)) { e.preventDefault(); explorer.switchTab((explorer.activeTabIndex + 1) % explorer.tabs.length); loadDirectory(explorer.tabs[explorer.activeTabIndex].path, false, false, false); return; }
    if (checkHotkey(e, hk.prevTab)) { e.preventDefault(); explorer.switchTab((explorer.activeTabIndex - 1 + explorer.tabs.length) % explorer.tabs.length); loadDirectory(explorer.tabs[explorer.activeTabIndex].path, false, false, false); return; }
    if (checkHotkey(e, hk.switchPane) && explorer.isTiledView) { e.preventDefault(); explorer.focusedPane = isSecondary ? 'primary' : 'secondary'; return; }
    if (checkHotkey(e, hk.toggleSidebar)) { e.preventDefault(); settings.showSidebar = !settings.showSidebar; settings.saveSettings(); return; }
    if (checkHotkey(e, hk.toggleTerminal)) { 
      e.preventDefault(); 
      if (explorer.focusedPane === 'secondary') settings.showSecondaryTerminal = !settings.showSecondaryTerminal;
      else settings.showPrimaryTerminal = !settings.showPrimaryTerminal;
      settings.saveSettings(); 
      return; 
    }
    if (checkHotkey(e, hk.fullScreen)) { e.preventDefault(); invoke('window_toggle_fullscreen'); return; }

    // Focus & Views
    if (checkHotkey(e, hk.focusAddress)) { e.preventDefault(); ui.triggerAddressFocus++; return; }
    if (checkHotkey(e, hk.openSearch)) { e.preventDefault(); ui.showSearchModal = true; return; }
    if (checkHotkey(e, hk.contentSearch)) { e.preventDefault(); ui.showSearchModal = true; return; }
    if (checkHotkey(e, hk.toggleView)) { 
      e.preventDefault(); 
      if (isSecondary) settings.secondaryViewMode = settings.secondaryViewMode === 'grid' ? 'list' : 'grid';
      else settings.primaryViewMode = settings.primaryViewMode === 'grid' ? 'list' : 'grid';
      settings.saveSettings(); return; 
    }
    if (checkHotkey(e, hk.sortType)) { e.preventDefault(); import('$lib/actions').then(m => m.setSortMode('type')); return; }

    // File Operations
    if (checkHotkey(e, hk.compress)) { e.preventDefault(); handleCompress(); return; }
    if (checkHotkey(e, hk.extractHere)) { e.preventDefault(); handleExtractHere(); return; }
    if (checkHotkey(e, hk.extractFolder)) { e.preventDefault(); handleExtractToFolder(); return; }
    if (checkHotkey(e, hk.copy)) { e.preventDefault(); handleCopy(); return; }
    if (checkHotkey(e, hk.cut)) { e.preventDefault(); handleCut(); return; }
    if (checkHotkey(e, hk.paste)) { e.preventDefault(); handlePaste(targetPath); return; }
    if (checkHotkey(e, hk.undo)) { e.preventDefault(); import('$lib/actions').then(m => m.handleUndo()); return; }
    if (checkHotkey(e, hk.rename)) { e.preventDefault(); import('$lib/actions').then(m => m.renameSelected()); return; }
    if (checkHotkey(e, hk.permDelete)) { e.preventDefault(); import('$lib/actions').then(m => m.deleteSelected(true)); return; }
    if (checkHotkey(e, hk.delete)) { e.preventDefault(); import('$lib/actions').then(m => m.deleteSelected(false)); return; }
    if (checkHotkey(e, hk.togglePreview)) { 
      e.preventDefault(); 
      if (explorer.previewImagePath !== null || explorer.previewTextContent !== null || explorer.previewPdfPath !== null) {
        explorer.previewImagePath = null; explorer.previewTextContent = null; 
        explorer.previewPdfPath = null; explorer.previewFilePath = null; explorer.isEditingPreview = false;
      } else {
        import('$lib/actions').then(m => m.handlePreview());
      }
      return; 
    }
    if (checkHotkey(e, hk.newFolder)) { e.preventDefault(); ui.createModalType = 'folder'; ui.createInputName = "New Folder"; ui.createModalX = window.innerWidth / 2 - 120; ui.createModalY = window.innerHeight / 2 - 60; ui.showCreateModal = true; return; }
    if (checkHotkey(e, hk.newFile)) { e.preventDefault(); ui.createModalType = 'file'; ui.createInputName = "New Text Document.txt"; ui.createModalX = window.innerWidth / 2 - 120; ui.createModalY = window.innerHeight / 2 - 60; ui.showCreateModal = true; return; }
    if (checkHotkey(e, hk.copyPath)) { e.preventDefault(); navigator.clipboard.writeText(currentSel.map(f => f.path).join('\n')); return; }
    if (checkHotkey(e, hk.cutPath)) { e.preventDefault(); navigator.clipboard.writeText(currentSel.map(f => f.path).join('\n')); handleCut(); return; }
    if (checkHotkey(e, hk.properties) || checkHotkey(e, hk.calcSize)) { e.preventDefault(); import('$lib/actions').then(m => m.openProperties()); return; }
    if (checkHotkey(e, hk.zoomIn) || (e.ctrlKey && (e.key === '=' || e.key === '+'))) { e.preventDefault(); settings.currentFolderIconSize = Math.min(300, settings.currentFolderIconSize + 20); settings.saveSettings(); return; }
    if (checkHotkey(e, hk.zoomOut) || (e.ctrlKey && e.key === '-')) { e.preventDefault(); settings.currentFolderIconSize = Math.max(80, settings.currentFolderIconSize - 20); settings.saveSettings(); return; }
    // Command Palette hotkey
    if (e.key.toLowerCase() === 'f' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      ui.showSearchModal = true;
      return;
    }
  }


  onMount(() => {
    // Initialize settings and perform initial load 
    settings.init().then(async () => {
      const launchPath = await invoke<string | null>('get_launch_path');
      
      const startPath = launchPath || explorer.currentPath;
      
      await loadDirectory(startPath, true, true, false);
      const activeTab = explorer.tabs[explorer.activeTabIndex] || {};
      
      if (explorer.isTiledView) {
        await loadDirectory(activeTab.secondaryPath || "C:\\", true, true, true);
      }

      // Startup Update Check
      if (settings.autoCheckUpdates) {
        try {
          const update = await check();
          if (update && update.available) {
            ui.updateVersion = update.version;
            ui.updateBody = update.body || "A new version is available.";
            ui.updateAvailable = true;
            (window as any).pendingUpdate = update;
          }
        } catch (err) {
          console.error("Failed to check for updates:", err);
        }
      }
    });

    const unlistenDrop = listen('tauri://drop', async (event: any) => {
      const filePaths = event.payload.paths || event.payload; 
      if (!Array.isArray(filePaths)) return;
      
      for (const src of filePaths) {
        const fileName = src.split(/[/\\]/).pop(); 
        const separator = explorer.currentPath.endsWith('\\') || explorer.currentPath.endsWith('/') ? '' : '\\'; 
        const dest = `${explorer.currentPath}${separator}${fileName}`; 
        try {
          await invoke("copy_item", { src, dest }); 
        } catch (error) {
          explorer.errorMessage = `External drop failed: ${error}`;
        }
      }
      loadDirectory(explorer.currentPath, false); 
    });

    // File System Watcher
    const unlistenChanged = listen('items_changed', async (event: { payload: WatchEvent[] }) => {
      let primaryChanged = false;
      let secondaryChanged = false;

      let primaryMap = new Map(explorer.files.map(f => [f.path, f]));
      let secondaryMap = new Map(explorer.secondaryFiles.map(f => [f.path, f]));
      const activeTab = explorer.tabs[explorer.activeTabIndex];

      for (const { watched_path, item, deleted_path } of event.payload) {
        // Primary Pane Update
        if (watched_path === explorer.currentPath) {
          if (item && !primaryMap.has(item.path)) {
            primaryMap.set(item.path, item);
            primaryChanged = true;
          } else if (deleted_path && primaryMap.has(deleted_path)) {
            primaryMap.delete(deleted_path);
            primaryChanged = true;
          }
        }
        
        // Secondary Pane Update
        if (explorer.isTiledView && watched_path === activeTab?.secondaryPath) {
          if (item && !secondaryMap.has(item.path)) {
            secondaryMap.set(item.path, item);
            secondaryChanged = true;
          } else if (deleted_path && secondaryMap.has(deleted_path)) {
            secondaryMap.delete(deleted_path);
            secondaryChanged = true;
          }
        }
      }

      // Re-sort locally and trigger reactivity in bulk
      if (primaryChanged) explorer.files = sortFiles(Array.from(primaryMap.values()));
      if (secondaryChanged) explorer.secondaryFiles = sortFiles(Array.from(secondaryMap.values()));
    });

    // 4. Cleanup all listeners on unmount 
    return () => {
      unlistenDrop.then(fn => fn());
      unlistenChanged.then(fn => fn());
    };
  });
  </script>

<svelte:window 
  onkeydowncapture={handleWindowKeydown}
  onscrollcapture={() => ui.closeContextMenu()}
  onclick={() => {
    ui.closeContextMenu();
    ui.showCreateModal = false;
  }}
  ondragover={(e) => e.preventDefault()}
  ondrop={(e) => e.preventDefault()}
  oncontextmenu={(e) => {
    const target = e.target;
    // Block native right-click everywhere EXCEPT inputs, textareas, and the terminal
    if (target instanceof Element && !target.closest('.terminal-container') && target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
      e.preventDefault();
    }
  }}
/>

<main class="app-container">
  <TabBar />

  <div class="layout-wrapper">
    {#if settings.showSidebar || settings.sidebarHoverReveal}
      <Sidebar />
    {/if}

    <div class="panes-wrapper {settings.splitDirection}">
          <div class="pane primary-pane {explorer.focusedPane === 'primary' ? 'focused-pane' : ''}" role="presentation" onmousedown={() => explorer.focusedPane = 'primary'} style="flex: {paneSplitRatio}">
      <AddressBar />

      {#if explorer.errorMessage}
        <p class="error">Error: {explorer.errorMessage}</p>
      {/if}

      <FileContainer />
      <div style:display={settings.showPrimaryTerminal ? 'flex' : 'none'} style:flex-direction="column">
          {#key settings.integratedTerminalType + settings.customIntegratedTerminal}
            <TerminalPane id="primary" cwd={explorer.currentPath} />
          {/key}
        </div>
    </div>

    {#if explorer.previewImagePath !== null || explorer.previewTextContent !== null || explorer.previewPdfPath !== null}
            <div class="pane-resizer" role="separator" tabindex="-1" onmousedown={startPaneResize}></div>
            <div class="pane preview-pane" style="flex: {100 - paneSplitRatio}">
        <div class="preview-header">
          <span>{explorer.previewFilePath ? explorer.previewFilePath.split(/[/\\]/).pop() : 'Preview'}</span>
          <div class="preview-actions">
            {#if explorer.previewTextContent !== null}
              <button class="header-btn" onclick={() => explorer.isEditingPreview = !explorer.isEditingPreview}>
                {explorer.isEditingPreview ? 'Preview' : 'Edit'}
              </button>
              {#if explorer.isEditingPreview}
                <button class="header-btn" onclick={async () => {
                  if (explorer.previewFilePath && explorer.previewTextContent !== null) {
                    try {
                      await invoke("write_text_file", { path: explorer.previewFilePath, contents: explorer.previewTextContent });
                    } catch (error) {
                      explorer.errorMessage = `Save failed: ${error}`;
                    }
                  }
                }}>Save</button>
              {/if}
            {/if}
            <button class="close-preview-btn" onclick={() => { 
              explorer.previewImagePath = null;
              explorer.previewTextContent = null; 
              explorer.previewPdfPath = null;
              explorer.previewFilePath = null;
            }}>✕</button>
          </div>
        </div>
        <div class="preview-content" onscroll={(e) => {
            const target = e.currentTarget;
            // Trigger load when scrolled within 100px of the bottom
            if (target.scrollHeight - target.scrollTop - target.clientHeight < 100) {
                import('$lib/actions').then(m => m.loadMorePreview());
            }
        }}>
          {#if explorer.previewPdfPath !== null}
            <object data={convertFileSrc(explorer.previewPdfPath)} type="application/pdf" class="inline-pdf-preview" title="PDF Preview"></object>
          {:else if explorer.previewImagePath !== null}
            <img src={convertFileSrc(explorer.previewImagePath || '')} alt="Preview" class="inline-preview-img" />
          {:else if explorer.previewTextContent !== null}
            {#if explorer.isEditingPreview}
              <textarea class="inline-text-editor" bind:value={explorer.previewTextContent}></textarea>
            {:else if explorer.previewFilePath?.toLowerCase().endsWith('.md')}
              <div class="markdown-preview">
                {@html DOMPurify.sanitize(marked.parse(explorer.previewTextContent) as string)}
              </div>
            {:else}
              <pre class="inline-text-preview">{explorer.previewTextContent}</pre>
            {/if}
          {/if}
        </div>
      </div>
    {:else if explorer.isTiledView}
            <div class="pane-resizer" role="separator" tabindex="-1" onmousedown={startPaneResize}></div>
            <div class="pane secondary-pane {explorer.focusedPane === 'secondary' ? 'focused-pane' : ''}" role="presentation" onmousedown={() => explorer.focusedPane = 'secondary'} style="flex: {100 - paneSplitRatio}">
        <AddressBar isSecondary={true} />
        {#if explorer.errorMessage}
          <p class="error">Error: {explorer.errorMessage}</p>
        {/if}
        <FileContainer isSecondary={true} />
        <div style:display={settings.showSecondaryTerminal ? 'flex' : 'none'} style:flex-direction="column">
          {#key settings.integratedTerminalType + settings.customIntegratedTerminal}
            <TerminalPane id="secondary" cwd={explorer.tabs[explorer.activeTabIndex]?.secondaryPath || "C:\\"} />
          {/key}
        </div>
      </div>
    {/if}
  </div>
  </div> <ContextMenu />
</main>

{#if ui.updateAvailable}
  <div class="update-toast">
    <div class="update-header">
      <span><span class="icon">🚀</span> Update Available ({ui.updateVersion})</span>
      <button class="close-btn" onclick={() => ui.updateAvailable = false}>✕</button>
    </div>
    {#if ui.isUpdating}
      <div class="progress-bar-container">
        <div class="progress-bar" style="width: {ui.updateTotal ? (ui.updateProgress / ui.updateTotal) * 100 : 0}%"></div>
      </div>
      <p class="progress-text">Downloading... {ui.updateTotal ? Math.round((ui.updateProgress / ui.updateTotal) * 100) : 0}%</p>
    {:else}
      <p class="update-body">{ui.updateBody}</p>
      <button class="update-btn" onclick={async () => {
        const update = (window as any).pendingUpdate;
        if (!update) return;
        ui.isUpdating = true;
        try {
          await update.downloadAndInstall((event: any) => {
            if (event.event === 'Started') {
              ui.updateTotal = event.data.contentLength || 0;
            } else if (event.event === 'Progress') {
              ui.updateProgress += event.data.chunkLength;
            }
          });
          await relaunch();
        } catch (err) {
          console.error("Update failed", err);
          ui.isUpdating = false;
          ui.updateAvailable = false;
        }
      }}>Download & Restart</button>
    {/if}
  </div>
{/if}

<PreferencesModal />
<CreateModal />
<HotkeysModal />
<PropertiesModal />
<SearchModal />
<ArchiveModal />
<CommandPalette />

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 0 1rem 1rem 1rem;
    box-sizing: border-box;
    background-color: transparent;
  }
  .error {
    color: #ff5555;
    background: #441111;
    padding: 10px;
    border-radius: 4px;
  }
  .panes-wrapper {
    display: flex;
    flex-grow: 1;
    overflow: hidden;
    gap: 12px;
  }
  .panes-wrapper.vertical {
    flex-direction: row;
  }
  .panes-wrapper.horizontal { flex-direction: column; }
  .pane { flex: 1; display: flex; flex-direction: column; min-width: 0; min-height: 0; border-radius: 8px;
overflow: hidden; border: 1px solid transparent; transition: border-color 0.2s; contain: strict; }
  
  .pane-resizer {
    flex: 0 0 12px;
    margin: 0 -12px;
    z-index: 10;
    cursor: col-resize;
    background: transparent;
  }
  .panes-wrapper.horizontal .pane-resizer {
    cursor: row-resize;
    margin: -12px 0;
  }

  /* .focused-pane placeholder */

  .secondary-pane {
    background: rgba(0, 0, 0, 0.15);
    border: 1px solid var(--border-color);
  }
  /* Preview Pane Styling */
  .preview-pane {
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
  }
  .preview-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 15px;
    background: var(--bg-hover);
    border-bottom: 1px solid var(--border-color);
    font-weight: bold;
  }
  .close-preview-btn {
    background: transparent;
    border: none;
    color: var(--text-main);
    font-size: 1.1rem;
  }
  .close-preview-btn:hover {
    color: #ff5555;
  }
  .preview-content {
    flex-grow: 1;
    overflow: auto;
    padding: 15px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .inline-preview-img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .inline-text-preview, .inline-text-editor, .markdown-preview {
    width: 100%;
    height: 100%;
    margin: 0;
    overflow: auto;
    color: var(--text-main);
    align-self: flex-start;
  }
  .inline-text-preview {
    white-space: pre-wrap;
    word-wrap: break-word;
    font-family: monospace;
    font-size: 0.9rem;
  }
  .inline-text-editor {
    background: var(--bg-solid);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    padding: 12px;
    font-family: 'Consolas', 'Courier New', monospace;
    font-size: 0.9rem;
    resize: none;
    outline: none;
    border-radius: 6px;
    transition: border-color 0.2s ease;
  }
  
  .inline-text-editor:focus {
    border-color: var(--accent);
  }
  .markdown-preview {
    padding: 0 10px;
    line-height: 1.5;
  }
  .preview-actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .header-btn {
    background: var(--bg-base);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .header-btn:hover {
    background: var(--bg-hover);
  }
  .inline-pdf-preview {
    width: 100%;
    height: 100%;
    border: none;
    border-radius: 4px;
  }
  .layout-wrapper {
    display: flex;
    flex-grow: 1;
    overflow: hidden;
    gap: 12px;
    position: relative;
  }

  /* Toast Styling */
  .update-toast {
    position: fixed;
    bottom: 20px;
    right: 20px;
    background: var(--bg-surface);
    border: 1px solid var(--accent);
    border-radius: 8px;
    padding: 15px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.6);
    z-index: 5000;
    width: 300px;
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    animation: pop-in 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards;
  }
  .update-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-weight: bold;
    color: var(--text-main);
    margin-bottom: 8px;
  }
  .update-header .icon { margin-right: 6px; }
  .update-header .close-btn {
    background: transparent; border: none; color: var(--text-muted); cursor: pointer;
  }
  .update-header .close-btn:hover { color: var(--text-main); }
  .update-body {
    font-size: 0.85rem; color: var(--text-muted); margin-bottom: 12px;
    display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden;
  }
  .update-btn {
    width: 100%; background: var(--accent); color: white; border: none;
    padding: 8px; border-radius: 4px; cursor: pointer; font-weight: 500;
  }
  .update-btn:hover { filter: brightness(1.1); }
  .progress-bar-container {
    width: 100%; height: 6px; background: var(--bg-base); border-radius: 3px; overflow: hidden; margin: 10px 0;
  }
  .progress-bar {
    height: 100%; background: var(--accent); transition: width 0.1s linear;
  }
  .progress-text { font-size: 0.8rem; color: var(--text-muted); text-align: center; margin: 0; }
</style>

