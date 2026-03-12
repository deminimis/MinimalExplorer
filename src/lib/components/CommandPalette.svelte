<script lang="ts">
  import { ui } from '$lib/ui.svelte';
  import { explorer } from '$lib/explorer.svelte';
  import { settings } from '$lib/settings.svelte';
  import { getAvailableCommands, type Command, startSearch, loadDirectory } from '$lib/actions';

  let commands = $derived(getAvailableCommands());
  
  //  what to display in the list
  let displayList = $derived.by(() => {
    const input = ui.paletteInput.trim();
    
    if (input === "") {
      // Show Recents if empty
      return settings.recentPaths.map(path => ({
        label: path.split(/[/\\]/).filter(Boolean).pop() || path,
        description: path,
        category: 'Recent',
        action: () => loadDirectory(path, true)
      }));
    } else if (input.startsWith('>')) {
      const filter = input.slice(1).toLowerCase().trim();
      // If just ">", show all commands; otherwise filter them
      if (filter === "") return commands;
      
      return commands.filter(c => 
        c.label.toLowerCase().includes(filter) || 
        c.category.toLowerCase().includes(filter)
      );
    }
    return []; 
  });

  function executeItem(item: any) {
    item.action();
    close();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      ui.paletteSelectedIndex = (ui.paletteSelectedIndex + 1) % (displayList.length || 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      ui.paletteSelectedIndex = (ui.paletteSelectedIndex - 1 + (displayList.length || 1)) % (displayList.length || 1);
    } else if (e.key === 'Enter') {
      if (displayList[ui.paletteSelectedIndex]) {
        executeItem(displayList[ui.paletteSelectedIndex]);
      } else if (!ui.paletteInput.startsWith('>') && ui.paletteInput.length > 0) {
        startSearch(explorer.currentPath, ui.paletteInput, false);
        close();
      }
    } else if (e.key === 'Escape') {
      close();
    }
  }

  // Reset selection when input changes
  $effect(() => {
    ui.paletteInput;
    ui.paletteSelectedIndex = 0;
  });

  function close() {
    ui.showCommandPalette = false;
    ui.paletteInput = "";
    ui.paletteSelectedIndex = 0;
  }

  function focusInput(node: HTMLInputElement) {
    setTimeout(() => node.focus(), 10);
  }

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showCommandPalette}
  <dialog class="palette-modal" use:setupDialog onclose={close} onclick={(e) => e.target === e.currentTarget && (e.currentTarget as HTMLDialogElement).close()}>
      <div class="input-container">
        <input 
          bind:value={ui.paletteInput} 
          onkeydown={handleKeyDown}
          use:focusInput
          placeholder="Type '>' for commands or search files..." 
        />
      </div>

      <div class="results">
        {#if displayList.length > 0}
          {#each displayList as item, i}
            <button 
              class="result-item {i === ui.paletteSelectedIndex ? 'active' : ''}" 
              onclick={() => executeItem(item)}
            >
              <span class="category">{item.category}</span>
              <span class="label">{item.label}</span>
              <span class="desc">{item.description}</span>
            </button>
          {/each}
        {:else if ui.paletteInput.length > 0 && !ui.paletteInput.startsWith('>')}
          <div class="search-prompt">
            Press Enter to search files for "<strong>{ui.paletteInput}</strong>"
          </div>
        {:else if ui.paletteInput.startsWith('>')}
           <div class="search-prompt">No commands found</div>
        {/if}
      </div>
  </dialog>
{/if}

<style>
  .palette-modal {
    background: var(--bg-menu);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    width: 600px;
    max-height: 450px;
    box-shadow: 0 20px 50px rgba(0,0,0,0.7);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: pop-in 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards; 
  }
  .input-container { padding: 15px; border-bottom: 1px solid var(--border-color); }
  input {
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-main);
    font-size: 1.1rem;
    outline: none;
  }
  .results { overflow-y: auto; padding: 5px 0; }
  .result-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 15px;
    background: transparent;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-main);
  }
  .result-item.active { background: var(--accent); color: white; }
  .category { font-size: 0.7rem; opacity: 0.6; text-transform: uppercase; width: 70px; }
  .label { flex-grow: 1; font-weight: 500; }
  .desc { font-size: 0.8rem; opacity: 0.7; }
  .search-prompt { padding: 20px; text-align: center; color: var(--text-muted); }
</style>