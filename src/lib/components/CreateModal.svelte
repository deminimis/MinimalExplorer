<script lang="ts">
  import { ui } from '$lib/ui.svelte';
  import { submitCreate } from '$lib/actions';

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showCreateModal}
  <dialog class="create-modal" style="margin: 0; bottom: auto; right: auto;" style:top="{ui.createModalY}px" style:left="{ui.createModalX}px" use:setupDialog onclose={() => ui.showCreateModal = false} onclick={(e) => e.target === e.currentTarget && (e.currentTarget as HTMLDialogElement).close()}>
    <p>Create {ui.createModalType === 'folder' ? 'Folder' : 'File'}</p>
    <input 
      type="text" 
      bind:value={ui.createInputName} 
      onkeydown={(e) => e.key === "Enter" && submitCreate()} 
    />
    <div class="modal-actions">
      <button onclick={() => ui.showCreateModal = false}>Cancel</button>
      <button class="active" onclick={submitCreate}>Create</button>
    </div>
  </dialog>
{/if}


<style>
  .create-modal {
    position: absolute;
    background: var(--bg-surface);
    border: 1px solid var(--border-color);
    border-radius: 8px;
    padding: 15px;
    box-shadow: 0 4px 16px rgba(0,0,0,0.5);
    z-index: 1500;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 240px;
  }
  .create-modal p {
    margin: 0;
    font-size: 0.95rem;
    font-weight: bold;
    color: var(--text-main);
  }
  .create-modal input {
    padding: 8px;
    background: var(--bg-base);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    width: 100%;
    box-sizing: border-box;
  }
  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>