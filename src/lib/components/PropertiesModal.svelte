<script lang="ts">
  import { ui } from '$lib/ui.svelte';
  import { explorer } from '$lib/explorer.svelte';
  import { openWindowsProperties } from '$lib/actions';
  import { formatSize, formatDate, getSystemIconSrc } from '$lib/utils';

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showPropertiesModal && ui.propertiesData}
  <dialog class="modal-content" use:setupDialog onclose={() => ui.showPropertiesModal = false} onclick={(e) => e.target === e.currentTarget && (e.currentTarget as HTMLDialogElement).close()}>
      
      <div class="header">
        <span class="icon" style="width: 42px; height: 42px; display: flex;"><img src={getSystemIconSrc(ui.propertiesData.name, ui.propertiesData.is_dir)} alt="" style="width: 100%; height: 100%; object-fit: contain;" /></span>
        <h2 title={ui.propertiesData.name}>{ui.propertiesData.name}</h2>
      </div>

      <div class="prop-grid">
        <div class="prop-label">Path:</div>
        <div class="prop-value path" title={ui.propertiesData.path}>{ui.propertiesData.path}</div>

        <div class="prop-label">Type:</div>
        <div class="prop-value">{ui.propertiesData.is_dir ? 'File Folder' : 'File'}</div>

        <div class="prop-label">Size:</div>
        <div class="prop-value">{ui.propertiesData.is_dir ? 'Calculating...' : formatSize(ui.propertiesData.size, false)} 
          <span class="bytes">({ui.propertiesData.size.toLocaleString()} bytes)</span>
        </div>

        {#if !ui.propertiesData.is_dir}
          <div class="prop-label">Checksum:</div>
          <div class="prop-value">
            {#if explorer.checksums[ui.propertiesData.path]}
              <span style="font-family: monospace; font-size: 0.8rem; word-break: break-all;">
                {explorer.checksums[ui.propertiesData.path]}
              </span>
            {:else}
              <button 
                class="advanced-btn" 
                style="padding: 2px 8px; font-size: 0.75rem;"
                onclick={() => explorer.calculateChecksum(ui.propertiesData.path)}
              >
                Analyze File
              </button>
            {/if}
          </div>
        {/if}

        <hr class="grid-span" />

        <div class="prop-label">Created:</div>
        <div class="prop-value">{formatDate(ui.propertiesData.created)}</div>

        <div class="prop-label">Modified:</div>
        <div class="prop-value">{formatDate(ui.propertiesData.modified)}</div>

        <hr class="grid-span" />

        <div class="prop-label">Attributes:</div>
        <div class="prop-value">
          <label><input type="checkbox" checked={ui.propertiesData.readonly} disabled /> Read-only</label>
        </div>
      </div>

      <div class="modal-actions">
        <button class="advanced-btn" onclick={openWindowsProperties}>Advanced...</button>
        <button class="close-btn" onclick={() => ui.showPropertiesModal = false}>OK</button>
      </div>
  </dialog>
{/if}

<style>
  .modal-content {
    padding: 1.5rem;
    width: 400px;
    display: flex;
    flex-direction: column;
    gap: 15px;
  }
  .header {
    display: flex; align-items: center; gap: 12px; border-bottom: 1px solid var(--border-color); padding-bottom: 15px;
  }
  .header .icon { font-size: 2.5rem; }
  .header h2 { margin: 0; font-size: 1.2rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .prop-grid {
    display: grid; grid-template-columns: 80px 1fr; gap: 10px; font-size: 0.9rem; align-items: center;
  }
  .prop-label { color: var(--text-muted); }
  .prop-value { color: var(--text-main); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .prop-value.path { font-family: monospace; background: var(--bg-base); padding: 4px 6px; border-radius: 4px; border: 1px solid var(--border-color); }
  .bytes { color: var(--text-muted); font-size: 0.8rem; margin-left: 5px; }
  .grid-span { grid-column: 1 / -1; border: 0; border-top: 1px solid var(--border-color); width: 100%; margin: 5px 0; }
  .modal-actions { display: flex; justify-content: space-between; margin-top: 10px; }
  .advanced-btn { background: var(--bg-hover); color: var(--text-main); border: 1px solid var(--border-color); padding: 8px 15px; border-radius: 6px; cursor: pointer; transition: 0.2s; }
  .advanced-btn:hover { background: var(--border-color); }
  .close-btn { background: var(--accent); color: white; border: none; padding: 8px 25px; border-radius: 6px; cursor: pointer; font-weight: bold; }
</style>