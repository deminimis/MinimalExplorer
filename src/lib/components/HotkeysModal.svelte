<script lang="ts">
  import { settings } from '$lib/settings.svelte';
  import { ui } from '$lib/ui.svelte';

  let listeningKey: string | null = $state(null);

  function handleKeydown(e: KeyboardEvent, actionId: string) {
    if (!listeningKey) return;
    e.preventDefault();
    e.stopPropagation();

    if (e.key === 'Escape') {
      listeningKey = null;
      return;
    }

    // Build the hotkey string
    const parts = [];
    if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
    if (e.altKey) parts.push('Alt');
    if (e.shiftKey) parts.push('Shift');
    
    // Ignore modifier-only presses until a real key is struck
    if (['Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) return;

    const keyName = e.key.length === 1 ? e.key.toUpperCase() : e.key;
    parts.push(keyName);

    // Update settings dynamically
    settings.hotkeys[actionId] = parts.join('+');
    settings.saveSettings();
    listeningKey = null;
  }

  const hotkeyLabels: Record<string, string> = {
    copy: "Copy Selected Items", cut: "Cut Selected Items", paste: "Paste to Active Pane",
    rename: "Rename Item", delete: "Delete Selected", permDelete: "Permanently Delete",
    newFolder: "Create New Folder", newFile: "Create New File", 
    copyPath: "Copy File Path", cutPath: "Cut File Path",
    properties: "Open Properties", calcSize: "Calculate Folder Size",
    
    selectAll: "Select All", invertSelection: "Invert Selection", selectSameType: "Select Same Type",
    
    goBack: "Go Back", goForward: "Go Forward", goUp: "Go Up One Directory",
    newTab: "New Tab", closeTab: "Close Tab", nextTab: "Next Tab", prevTab: "Previous Tab", 
    switchPane: "Switch Active Pane (Tiled View)", refresh: "Refresh Tab",
    
    focusAddress: "Focus Address Bar", focusFilter: "Focus Filter Bar", 
    openSearch: "Open Deep Search", contentSearch: "Search File Contents",
    
    toggleView: "Toggle List/Grid View", sortType: "Sort by Type", togglePreview: "Toggle File Preview",
    toggleSidebar: "Toggle Sidebar", toggleTerminal: "Toggle Terminal", 
    fullScreen: "Toggle Full Screen", zoomIn: "Zoom In", zoomOut: "Zoom Out",
    
    compress: "Compress to .zip", extractHere: "Extract Here", extractFolder: "Extract to Folder",
    undo: "Undo Last Action"
  };

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showHotkeysModal}
  <dialog class="modal-content" use:setupDialog onclose={() => ui.showHotkeysModal = false} onclick={(e) => e.target === e.currentTarget && (e.currentTarget as HTMLDialogElement).close()}>
      <h2>Keyboard Shortcuts</h2>
      <p class="subtitle">
        Click a shortcut below, then press your new combination. Press Escape to cancel.<br/>
        <strong style="color: var(--text-main);">Tip:</strong> Hold <strong>Shift + Right-Click</strong> on files for advanced options like Permanent Delete and Open Terminal (Admin).
      </p>
      
      <div class="hotkeys-list">
        <div class="hotkey-row">
          <span>Drill In / Out of Folders</span>
          <button class="hotkey-btn" style="cursor: default;" disabled>
            Ctrl + Scroll Wheel
          </button>
        </div>

        {#each Object.keys(settings.hotkeys) as actionId}
          <div class="hotkey-row">
            <span>{hotkeyLabels[actionId]}</span>
            <button 
              class="hotkey-btn {listeningKey === actionId ? 'listening' : ''}"
              onclick={() => listeningKey = actionId}
              onkeydown={(e) => handleKeydown(e, actionId)}
            >
              {#if listeningKey === actionId}
                Press keys...
              {:else}
                {#each settings.hotkeys[actionId as keyof typeof settings.hotkeys].split('+') as key, i}
                  {key}
                  {#if i < settings.hotkeys[actionId as keyof typeof settings.hotkeys].split('+').length - 1}
                    <span class="plus-sign"> + </span>
                  {/if}
                {/each}
              {/if}
            </button>
          </div>
        {/each}
      </div>

      <button class="close-btn" onclick={() => { ui.showHotkeysModal = false;
ui.showPrefsModal = true; }}>Back to Preferences</button>
  </dialog>
{/if}

<style>
  .modal-content {
    padding: 2rem;
    min-width: 440px;
  }
  .subtitle {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin-top: -10px;
    margin-bottom: 20px;
  }
  .hotkeys-list {
    display: flex; flex-direction: column; gap: 8px;
    margin-bottom: 20px; max-height: 400px; overflow-y: auto;
  }
  .hotkey-row {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 0; border-bottom: 1px solid var(--border-color);
  }
  .hotkey-btn {
    background: var(--bg-base); color: var(--text-main);
    border: 1px solid var(--border-color); padding: 6px 12px;
    border-radius: 4px; min-width: 130px; font-family: monospace;
    cursor: pointer; transition: all 0.2s;
  }
  .hotkey-btn:hover {
    background: var(--bg-hover); border-color: var(--accent);
  }
  .hotkey-btn.listening {
    background: var(--accent); color: white; border-color: var(--accent);
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0% { opacity: 1; }
    50% { opacity: 0.7; }
    100% { opacity: 1; }
  }
  .close-btn {
    width: 100%; background: var(--bg-hover); color: var(--text-main);
    border: 1px solid var(--border-color); padding: 10px; border-radius: 6px;
    cursor: pointer; font-weight: bold; transition: background 0.2s;
  }
  .close-btn:hover { background: var(--border-color); }
  .plus-sign {
    color: var(--accent);
    font-weight: 900;
    opacity: 0.7;
    margin: 0 4px;
    font-family: sans-serif;
  }
</style>