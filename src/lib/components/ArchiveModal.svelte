<script lang="ts">
  import { ui } from '$lib/ui.svelte';
  import { submitArchive } from '$lib/actions';

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showArchiveModal}
  <dialog class="modal-content archive-modal" use:setupDialog onclose={() => ui.showArchiveModal = false} oncancel={(e) => ui.isArchiving && e.preventDefault()} onclick={(e) => e.target === e.currentTarget && !ui.isArchiving && (e.currentTarget as HTMLDialogElement).close()}>
      
      {#if ui.isArchiving}
        <div class="loading-state">
          <div class="spinner"></div>
          <p>{ui.archiveMode === 'compress' ? 'Compressing files...' : 'Extracting files...'}</p>
          <span class="subtext">This might take a moment depending on the size.</span>
        </div>
      {:else}
        <h2>{ui.archiveMode === 'compress' ? 'Compress to Archive' : 'Extract to Folder'}</h2>
        
        <div class="input-group">
          <label>{ui.archiveMode === 'compress' ? 'Archive Name:' : 'Destination Folder Name:'}</label>
          <input 
            type="text" 
            bind:value={ui.archiveInputName} 
            onkeydown={(e) => e.key === "Enter" && submitArchive()} 
            autofocus
          />
        </div>

        {#if ui.archiveMode === 'compress'}
          <div class="input-group">
            <label>Compression Method:</label>
            <select bind:value={ui.archiveMethod}>
              <option value="deflated">Deflate (Standard)</option>
              <option value="stored">Store (No Compression)</option>
            </select>
          </div>

          {#if ui.archiveMethod === 'deflated'}
            <div class="input-group">
              <label>Compression Level: {ui.archiveLevel}</label>
              <input 
                type="range" 
                min="1" 
                max="9" 
                bind:value={ui.archiveLevel} 
              />
              <div class="range-labels">
                <span>Fastest</span>
                <span>Smallest</span>
              </div>
            </div>
          {/if}
        {/if}

        <div class="modal-actions">
          <button class="cancel-btn" onclick={() => ui.showArchiveModal = false}>Cancel</button>
          <button class="confirm-btn" onclick={submitArchive}>
            {ui.archiveMode === 'compress' ? 'Compress' : 'Extract'}
          </button>
        </div>
      {/if}
  </dialog>
{/if}

<style>
.modal-content.archive-modal {
    padding: 20px;
    border-radius: 8px; 
    min-width: 320px;
    display: flex;
    flex-direction: column;
    gap: 15px;
  }
  
  .input-group select option {
    background-color: var(--bg-menu); 
    color: var(--text-main); 
  }
  .input-group input[type="text"]:focus, .input-group select:focus {
    border-color: var(--accent);
  }
  .range-labels {
    display: flex; justify-content: space-between;
    font-size: 0.75rem; color: var(--text-muted);
  }
  .modal-actions {
    display: flex; justify-content: flex-end; gap: 8px; margin-top: 5px;
  }
  .modal-actions button {
    padding: 8px 15px; border-radius: 4px; cursor: pointer; border: none; font-family: inherit;
  }
  .cancel-btn { background: var(--bg-hover); color: var(--text-main); border: 1px solid var(--border-color) !important; }
  .cancel-btn:hover { background: var(--border-color); }
  .confirm-btn { background: var(--accent); color: white; }
  .confirm-btn:hover { filter: brightness(1.1); }

  /* State Styles */
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 20px 0;
    text-align: center;
  }
  .loading-state p {
    margin: 15px 0 5px 0;
    font-weight: 500;
    font-size: 1.05rem;
  }
  .loading-state .subtext {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid var(--border-color);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s cubic-bezier(0.4, 0, 0.2, 1) infinite;
  }
  @keyframes spin { 
    to { transform: rotate(360deg); } 
  }
</style>