<script lang="ts">
  import { explorer } from '$lib/explorer.svelte';
  import { loadDirectory } from '$lib/actions';
  import { invoke } from '@tauri-apps/api/core';

  function handleTabSwitch(index: number): void {
    if (explorer.activeTabIndex === index) return;
    explorer.switchTab(index);
    
    const tab = explorer.tabs[index];
    
    // Force the app to load the primary files for the tab we just switched to
    loadDirectory(tab.path, false, false, false);
    
    if (tab.isTiledView && tab.secondaryPath) {
      // Load secondary pane if it's dual-pane
      loadDirectory(tab.secondaryPath, false, false, true);
    } else {
      // Wipe secondary files memory if this is a single pane
      explorer.secondaryFiles = [];
    }
  }

  function handleTabClose(e: Event, index: number) {
    e.stopPropagation();
    if (explorer.tabs.length === 1) return; 
    
    explorer.closeTab(index);
    
    // Force UI to sync with the new active tab and trigger OS watcher cleanup
    const activeTab = explorer.tabs[explorer.activeTabIndex];
    loadDirectory(activeTab.path, false, false, false);
    
    if (activeTab.isTiledView && activeTab.secondaryPath) {
      loadDirectory(activeTab.secondaryPath, false, false, true);
    } else {
      // Wipe secondary files memory if this is a single pane
      explorer.secondaryFiles = [];
    }
  }
</script>

<div class="titlebar" role="presentation" data-tauri-drag-region>
  <div class="tabs-bar" data-tauri-drag-region>
    {#each explorer.tabs as tab, i}
      <div 
        class="tab {explorer.activeTabIndex === i ? 'active' : ''}" 
        role="button"
        tabindex="0"
        onclick={() => handleTabSwitch(i)}
        onkeydown={(e) => e.key === 'Enter' && handleTabSwitch(i)}
        data-tauri-drag-region
      >
        {tab.path.split(/[/\\]/).filter(Boolean).pop() || tab.path}
        {#if explorer.tabs.length > 1}
          <span 
            class="close-tab" 
            role="button"
            tabindex="0"
            onclick={(e) => handleTabClose(e, i)}
            onkeydown={(e) => { e.key === 'Enter' && handleTabClose(e, i); }}
          >✕</span>
        {/if}
      </div>
    {/each}
    <button class="add-tab-btn" onclick={() => { 
      explorer.addNewTab();
      loadDirectory(explorer.tabs[explorer.activeTabIndex].path, true, true, false); 
    }} title="New tab" data-tauri-drag-region>+</button>
  </div>

  <div class="window-controls">
    <button class="window-btn" onclick={() => invoke('window_minimize')} title="Minimize">─</button>
    <button class="window-btn" onclick={() => invoke('window_toggle_maximize')} title="Maximize">□</button>
    <button class="window-btn close-window" onclick={() => invoke('window_close')} title="Close">✕</button>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    background: transparent;
    user-select: none;
    padding-top: 8px; 
  }
  .tabs-bar {
    display: flex;
    gap: 5px;
    flex-grow: 1;
    padding-left: 10px;
    margin-bottom: 8px;
  }
  .window-controls {
    display: flex;
    align-items: flex-start;
    height: 100%;
    align-self: flex-start;
    margin-top: -8px; 
  }
  .window-btn {
    background: transparent;
    border: none;
    color: var(--text-main);
    width: 46px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1rem;
    transition: background 0.1s;
  }
  .window-btn:hover {
    background: var(--bg-hover);
  }
  .window-btn.close-window:hover {
    background: #e81123;
    color: white;
  }
  .tab {
    background: rgba(0, 0, 0, 0.4); 
    padding: 6px 16px;
    border-radius: 0;
    font-size: 0.85rem;
    border: 1px solid var(--border-color);
    border-bottom: none;
    display: flex;
    align-items: center;
    gap: 8px;
    color: #bbbbbb; 
    transition: background 0.2s, box-shadow 0.2s, color 0.2s;
  }

  /* Inactive tabs in light theme */
  :global([data-theme="light"]) .tab {
    background: rgba(255, 255, 255, 0.4); 
    color: #555555; 
  }

  /* Active tab in dark theme */
  .tab.active {
    background: rgba(25, 25, 25, 0.95);
    border-bottom-color: transparent;
    color: #ffffff;
    box-shadow: 0 -8px 25px rgba(100, 150, 255, 0.15); 
    z-index: 1;
  }
  
  /* Active tab in light theme */
  :global([data-theme="light"]) .tab.active {
    background: rgba(255, 255, 255, 0.95);
    color: #000000;
    box-shadow: 0 -8px 25px rgba(255, 165, 0, 0.25); 
  }

  .close-tab {
    margin-left: 4px;
    border-radius: 50%;
    width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    opacity: 0.5;
  }
  .close-tab:hover {
    opacity: 1;
    background: var(--bg-hover);
    color: var(--text-main);
  }
  .add-tab-btn {
    background: transparent;
    border: none;
    padding: 5px 10px;
    color: var(--text-muted);
  }
</style>