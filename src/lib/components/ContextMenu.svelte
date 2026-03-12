<script lang="ts">
  import { explorer } from '$lib/explorer.svelte';
  import { settings } from '$lib/settings.svelte';
  import { ui } from '$lib/ui.svelte';
  import { handleCopy, handleCut, renameSelected, deleteSelected, launchTerminal, handlePaste, openCreateModal, handlePreview, handleOpenWith, openInNewTab, openInOtherPane, togglePinItem, openProperties, handleCompress, handleExtractHere, handleExtractToFolder, handleUndo } from '$lib/actions';
</script>

{#if ui.contextMenuVisible}
  <div class="context-menu" style="top: {ui.contextMenuY}px; left: {ui.contextMenuX}px;" role="presentation" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
    {#if ui.menuType === 'item'}
      <button onclick={handleCopy}>Copy</button>
      <button onclick={handleCut}>Cut</button>
      <button onclick={() => { navigator.clipboard.writeText(explorer.selectedFiles.map(f => f.path).join('\n')); ui.closeContextMenu(); }}>Copy Path</button>
      <button onclick={() => { navigator.clipboard.writeText(explorer.selectedFiles.map(f => f.path).join('\n')); handleCut(); }}>Cut Path</button>
      <hr />
      
      <button onclick={() => deleteSelected(false)}>Delete</button>
      {#if ui.shiftKeyPressed}
        <button onclick={() => deleteSelected(true)}>Permanent Delete</button>
      {/if}
      <hr />

      {#if explorer.selectedFiles.length === 1}
        <button onclick={renameSelected}>Rename</button>
        <button onclick={handleCompress}>Compress to .zip</button>
        {#if explorer.selectedFiles[0].name.toLowerCase().endsWith('.zip')}
          <button onclick={handleExtractHere}>Extract Here</button>
          <button onclick={handleExtractToFolder}>Extract to Folder...</button>
        {/if}
        {#if !explorer.selectedFiles[0].is_dir}
          <button onclick={handlePreview}>Preview File</button>
          <button onclick={handleOpenWith}>Open With...</button>
        {/if}
        <button onclick={() => togglePinItem(explorer.selectedFiles[0].name, explorer.selectedFiles[0].path, explorer.selectedFiles[0].is_dir)}>
          {settings.pinnedFolders.some(p => p.path === explorer.selectedFiles[0].path) ? 'Unpin from Quick Access' : 'Pin to Quick Access'}
        </button>
      {/if}
      
      <button onclick={() => launchTerminal(false)}>Open in Terminal</button>
      {#if ui.shiftKeyPressed}
        <button onclick={() => launchTerminal(true)}>Open in Terminal (Admin)</button>
      {/if}
    
      {#if explorer.selectedFiles.length === 1 && explorer.selectedFiles[0].is_dir}
        <hr />
        <button onclick={() => openInNewTab(explorer.selectedFiles[0].path)}>Open in New Tab</button>
        <button onclick={() => openInOtherPane(explorer.selectedFiles[0].path)}>Open in Other Pane</button>
      {/if}
      <hr />
      <button onclick={openProperties}>Properties</button>
     {:else if ui.menuType === 'sidebar'}
      <button onclick={() => togglePinItem("", ui.contextMenuSidebarPath!, true)}>Unpin from Quick Access</button>
      <button onclick={() => openInOtherPane(ui.contextMenuSidebarPath!)}>Open in Second Pane</button>
      <hr />
      <button onclick={openProperties}>Properties</button>
    {:else if ui.menuType === 'drop'}
      <button onclick={() => { import('$lib/actions').then(m => m.executeDropAction('copy')); ui.closeContextMenu(); }}>Copy here</button>
      <button onclick={() => { import('$lib/actions').then(m => m.executeDropAction('move')); ui.closeContextMenu(); }}>Move here</button>
      <hr />
      <button onclick={() => ui.closeContextMenu()}>Cancel</button>
    {:else}
      <button onclick={() => { 
        const activeTab = explorer.tabs[explorer.activeTabIndex] || {};
        const targetPath = ui.contextMenuPane === 'secondary' ? (activeTab.secondaryPath || "C:\\") : explorer.currentPath;
        handlePaste(targetPath);
      }} disabled={!explorer.clipboardAction}>Paste</button>
      
      {#if explorer.undoStack.length > 0}
        {@const lastAction = explorer.undoStack[explorer.undoStack.length - 1]}
        <button onclick={() => { handleUndo(); ui.closeContextMenu(); }}>
          Undo {lastAction.type === 'move' ? 'Move' : 'Rename'}
        </button>
      {/if}
      <hr />
      
      <div class="submenu-container">
        <button class="submenu-trigger">View Mode ▸</button>
        <div class="submenu">
          <button onclick={() => { 
            const activeTab = explorer.tabs[explorer.activeTabIndex] || {};
            const targetPath = ui.contextMenuPane === 'secondary' ? (activeTab.secondaryPath || "C:\\") : explorer.currentPath;
            
            if (ui.contextMenuPane === 'secondary') settings.secondaryViewMode = 'grid';
            else settings.primaryViewMode = 'grid';
            
            settings.sessionViewOverrides.set(targetPath, 'grid');
            settings.saveSettings(); ui.closeContextMenu(); 
          }}>⊞ Grid</button>
          
          <button onclick={() => { 
            const activeTab = explorer.tabs[explorer.activeTabIndex] || {};
            const targetPath = ui.contextMenuPane === 'secondary' ? (activeTab.secondaryPath || "C:\\") : explorer.currentPath;

            if (ui.contextMenuPane === 'secondary') settings.secondaryViewMode = 'list';
            else settings.primaryViewMode = 'list';

            settings.sessionViewOverrides.set(targetPath, 'list');
            settings.saveSettings(); ui.closeContextMenu(); 
          }}>𝄓 List</button>
        </div>
                </div>
                <button onclick={() => { 
        explorer.isTiledView = !explorer.isTiledView;
        if (!explorer.isTiledView) {
          explorer.secondaryFiles = [];
          explorer.secondarySelectedFiles = [];
        }
        ui.closeContextMenu(); 
      }}>
        {explorer.isTiledView ? '◧ Disable Tiled View' : '◨ Enable Tiled View'}
      </button>
      <hr />
      <button onclick={(e) => openCreateModal(e, 'folder')}>New Folder</button>
      <button onclick={(e) => openCreateModal(e, 'file')}>New File</button>
      <hr />
      <button onclick={() => launchTerminal(false)}>Open in Terminal</button>
      <button onclick={() => launchTerminal(true)}>Open in Terminal (Admin)</button>
      <hr />
      <button onclick={openProperties}>Properties</button>
      <hr />
      <button onclick={() => { 
        if (ui.contextMenuPane === 'secondary') settings.showSecondaryTerminal = !settings.showSecondaryTerminal; 
        else settings.showPrimaryTerminal = !settings.showPrimaryTerminal; 
        settings.saveSettings(); 
        ui.closeContextMenu(); 
      }}>
        {(ui.contextMenuPane === 'secondary' ? settings.showSecondaryTerminal : settings.showPrimaryTerminal) ?
'Hide Integrated Terminal' : 'Show Integrated Terminal'}
      </button>
      <button onclick={() => { ui.showPrefsModal = true;
ui.closeContextMenu(); }}>⚙ Preferences</button>
    {/if}
  </div>
{/if}

<style>
  .context-menu {
    position: absolute;
    background: var(--bg-menu);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    box-shadow: 2px 4px 12px rgba(0, 0, 0, 0.5);
    display: flex;
    flex-direction: column;
    z-index: 1000;
    min-width: 150px;
  }

  .context-menu button {
    background: transparent;
    border: none;
    color: var(--text-main);
    padding: 10px 15px;
    text-align: left;
    border-radius: 0;
    font-size: 0.9rem;
  }

  .context-menu button:hover {
    background: #0060df;
  }

  .context-menu hr {
    border: 0;
    height: 1px;
    background: var(--border-color);
    margin: 4px 0;
  }

  .submenu-container {
    position: relative;
  }

  .submenu-trigger {
    display: flex;
    justify-content: space-between;
    width: 100%;
  }

  .submenu {
    display: none;
    position: absolute;
    left: 100%;
    top: 0;
    background: var(--bg-menu);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    box-shadow: 2px 4px 12px rgba(0, 0, 0, 0.5);
    min-width: 150px;
    flex-direction: column;
    z-index: 1001;
  }

  .submenu-container:hover .submenu {
    display: flex;
  }
</style>