<script lang="ts">
  import { settings } from '$lib/settings.svelte';
  import { ui } from '$lib/ui.svelte';

  function handleThemeChange() {
    settings.applyTheme();
    settings.saveSettings();
  }

  let sidebarDropdown = $state(
    settings.showSidebar ? 'pinned' : 
    (settings.sidebarHoverReveal ? 'hover' : 'off')
  );

  function handleSidebarChange() {
    if (sidebarDropdown === 'pinned') {
      settings.showSidebar = true;
      settings.sidebarHoverReveal = false;
    } else if (sidebarDropdown === 'hover') {
      settings.showSidebar = false;
      settings.sidebarHoverReveal = true;
    } else {
      settings.showSidebar = false;
      settings.sidebarHoverReveal = false;
    }
    settings.saveSettings();
  }

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showPrefsModal}
  <dialog class="modal-content" use:setupDialog onclose={() => ui.showPrefsModal = false} onclick={(e) => e.target === e.currentTarget && (e.currentTarget as HTMLDialogElement).close()}>
      <h2>Preferences</h2>
      
      <div class="setting-group">
        <label for="theme-select">Theme Appearance:</label>
        <select id="theme-select" bind:value={settings.theme} onchange={handleThemeChange}>
          <option value="dark">Dark Mode</option>
          <option value="light">Light Mode</option>
          <option value="auto">System Auto</option>
        </select>
      </div>

      <div class="setting-group">
        <label for="split-select">Tiled View Layout:</label>
        <select id="split-select" bind:value={settings.splitDirection} onchange={() => settings.saveSettings()}>
          <option value="vertical">Left / Right (Vertical Split)</option>
          <option value="horizontal">Top / Bottom (Horizontal Split)</option>
        </select>
      </div>

      <div class="setting-group">
        <label for="sidebar-select">Quick Access Sidebar:</label>
        <select id="sidebar-select" bind:value={sidebarDropdown} onchange={handleSidebarChange}>
          <option value="pinned">Pinned</option>
          <option value="hover">Hover to show</option>
          <option value="off">Off</option>
        </select>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.showHiddenFiles} onchange={() => settings.saveSettings()} />
          Show Hidden Files
        </label>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.showGitBadges} onchange={() => settings.saveSettings()} />
          Show Git Repository Badges
        </label>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.showListGridLines} onchange={() => settings.saveSettings()} />
          Show Grid Lines in List View
        </label>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.enableThumbnails} onchange={() => settings.saveSettings()} />
          Show Image Thumbnails (Instead of Icons)
        </label>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.terminalAutoCd} onchange={() => settings.saveSettings()} />
          Terminal Auto-CD to Current Folder
        </label>
      </div>

      <div class="setting-group checkbox-group">
        <label>
          <input type="checkbox" bind:checked={settings.autoCheckUpdates} onchange={() => settings.saveSettings()} />
          Check for Updates on Startup
        </label>
      </div>

      <div class="setting-group">
        <label for="integrated-term-select">Integrated Terminal Shell:</label>
        <div style="display: flex; gap: 8px; align-items: stretch;">
          <select id="integrated-term-select" bind:value={settings.integratedTerminalType} onchange={() => settings.saveSettings()} style="flex: {settings.integratedTerminalType === 'custom' ? '0 0 40%' : '1'};">
            <option value="powershell">PowerShell 7 / PowerShell</option>
            <option value="cmd">Command Prompt (cmd)</option>
            <option value="custom">Custom...</option>
          </select>
          {#if settings.integratedTerminalType === 'custom'}
            <input type="text" placeholder="e.g. wsl, bash" bind:value={settings.customIntegratedTerminal} onchange={() => settings.saveSettings()} style="flex: 1; padding: 8px; background: var(--bg-surface); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 4px; outline: none;" />
          {/if}
        </div>
      </div>

      <div class="setting-group">
        <label for="external-term-select">External Terminal ("Open in Terminal"):</label>
        <div style="display: flex; gap: 8px; align-items: stretch;">
          <select id="external-term-select" bind:value={settings.externalTerminalType} onchange={() => settings.saveSettings()} style="flex: {settings.externalTerminalType === 'custom' ? '0 0 40%' : '1'};">
            <option value="cmd">Command Prompt</option>
            <option value="powershell">PowerShell 7 / PowerShell</option>
            <option value="custom">Custom...</option>
          </select>
          {#if settings.externalTerminalType === 'custom'}
            <input type="text" placeholder="e.g. wt.exe, alacritty" bind:value={settings.customExternalTerminal} onchange={() => settings.saveSettings()} style="flex: 1; padding: 8px; background: var(--bg-surface); color: var(--text-main); border: 1px solid var(--border-color); border-radius: 4px; outline: none;" />
          {/if}
        </div>
      </div>

      <div class="setting-group">
        <button class="menu-nav-btn" onclick={() => { ui.showPrefsModal = false; ui.showHotkeysModal = true; }}>
          Keyboard Shortcuts <span>›</span>
        </button>
      </div>

      <button class="close-btn" onclick={() => ui.showPrefsModal = false}>Done</button>
  </dialog>
{/if}


<style>
  .modal-content {
    padding: 2rem;
    min-width: 320px;
  }
  .setting-group {
    margin: 1.5rem 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .checkbox-group {
    flex-direction: row;
    align-items: center;
  }
  select {
    padding: 8px;
    background: var(--bg-surface);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    outline: none;
  }

  select option {
    background: var(--bg-solid);
    color: var(--text-main);
  }
  .menu-nav-btn {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 10px;
    background: var(--bg-hover);
    border: 1px solid var(--border-color);
    color: var(--text-main);
    border-radius: 6px;
    font-size: 0.95rem;
    cursor: pointer;
  }
  .menu-nav-btn:hover {
    background: var(--accent);
  }
  .close-btn {
    width: 100%;
    background: var(--bg-hover);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    padding: 12px;
    border-radius: 6px;
    margin-top: 15px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.2s ease, border-color 0.2s ease;
  }
  .close-btn:hover {
    background: var(--bg-surface);
    border-color: var(--text-muted);
  }
</style>