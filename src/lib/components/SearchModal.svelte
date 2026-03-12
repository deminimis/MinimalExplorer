<script lang="ts">
  import { ui } from '$lib/ui.svelte';
  import { explorer } from '$lib/explorer.svelte';
  import { startSearch, cancelSearch, loadDirectory } from '$lib/actions';
  import { getSystemIconSrc } from '$lib/utils';

  let searchedQuery = $state("");
  let isContentSearch = $state(false);

  // --- Advanced Form State ---
  let advExt = $state("");
  let advSizeOp = $state(">");
  let advSizeVal = $state<string | number>("");
  let advSizeUnit = $state("mb");
  let advDateOp = $state("<");
  let advDateVal = $state<string | number>("");
  let advDateUnit = $state("d");
  let advType = $state("any");
  let advHidden = $state(false);

  // Dynamically compile the form fields
  function getFullQuery() {
    let parts = [ui.searchQuery.trim()];
    
    let ext = advExt.trim().replace(/^\./, '');
    if (ext) parts.push(`ext:${ext}`);
    
    if (advSizeVal !== null && advSizeVal !== "") parts.push(`size:${advSizeOp}${advSizeVal}${advSizeUnit}`);
    if (advDateVal !== null && advDateVal !== "") parts.push(`date:${advDateOp}${advDateVal}${advDateUnit}`);
    
    if (advType === "dir") parts.push("type:dir");
    if (advType === "file") parts.push("type:file");
    if (advHidden) parts.push("is:hidden");
    
    return parts.filter(Boolean).join(" ");
  }

  let fullSearchQuery = $derived(getFullQuery());

  // Extract just the name portion for highlighting snippets 
  let nameQueryOnly = $derived(
    fullSearchQuery.split(/\s+/)
      .filter(p => !p.toLowerCase().startsWith('ext:') && 
                   !p.toLowerCase().startsWith('size:') && 
                   !p.toLowerCase().startsWith('type:') && 
                   !p.toLowerCase().startsWith('date:') && 
                   !p.toLowerCase().startsWith('is:'))
      .join(' ')
  );

  function escapeHtml(unsafe: string) {
    return unsafe
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
  }

  function focusInput(node: HTMLInputElement) {
    setTimeout(() => node.focus(), 10);
  }

  function executeSearch(content: boolean) {
    const finalQuery = fullSearchQuery;
    if (finalQuery.trim() === '') {
        explorer.searchResults = [];
        explorer.isSearching = false;
        searchedQuery = "";
        return;
    }
    
    searchedQuery = finalQuery;
    isContentSearch = content;
    const activeTab = explorer.tabs[explorer.activeTabIndex] || {};
    const targetPath = explorer.focusedPane === 'secondary' ? (activeTab.secondaryPath || "C:\\") : explorer.currentPath;

    // Clear advanced fields to prevent compounding since startSearch updates ui.searchQuery
    advExt = "";
    advSizeVal = "";
    advDateVal = "";
    advType = "any";
    advHidden = false;

    startSearch(targetPath, finalQuery, content);
  }

  function handleResultClick(file: any) {
    ui.showSearchModal = false;
    explorer.isSearching = false;
    
    if (file.is_dir) {
        loadDirectory(file.path, true, false, explorer.focusedPane === 'secondary');
    } else {
        const parts = file.path.split(/[/\\]/);
        parts.pop();
        const parentPath = parts.join('\\') + (parts.length === 1 && parts[0].endsWith(':') ? '\\' : '');
        
        loadDirectory(parentPath, true, false, explorer.focusedPane === 'secondary').then(() => {
            if (explorer.focusedPane === 'secondary') explorer.secondarySelectedFiles = [file];
            else explorer.selectedFiles = [file];
        });
    }
  }

  function setupDialog(node: HTMLDialogElement) { if (!node.open) node.showModal(); }
</script>

{#if ui.showSearchModal}
  <dialog class="search-modal" use:setupDialog onclose={() => { ui.showSearchModal = false; cancelSearch(); }} onclick={(e) => e.target === e.currentTarget && (e.currentTarget as HTMLDialogElement).close()}>
      
      <div class="search-header">
        <svg class="search-icon" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line></svg>
        <input 
          type="text" 
          bind:value={ui.searchQuery} 
          onkeydown={(e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                executeSearch(e.shiftKey);
            }
          }}
          placeholder="Search files and folders..." 
          use:focusInput
        />
        {#if explorer.isSearching}
          <div class="spinner"></div>
        {/if}
      </div>

      <div class="search-hints">
        <span><strong>Enter</strong> = Search Names</span>
        <span><strong>Shift + Enter</strong> = Search Contents <em>(may take a while if searching many files)</em></span>
      </div>

      <details class="advanced-filters">
        <summary>Advanced Search Filters ▸</summary>
        <div class="form-builder">
          
          <div class="form-group">
            <label>Extension</label>
            <div class="input-row">
              <span class="prefix">ext:</span>
              <input type="text" placeholder="pdf, png, txt" bind:value={advExt} onkeydown={(e) => e.key === 'Enter' && executeSearch(e.shiftKey)} />
            </div>
          </div>
          
          <div class="form-group">
            <label>File Size</label>
            <div class="input-row">
              <select bind:value={advSizeOp}>
                <option value=">">&gt; (Larger)</option>
                <option value="<">&lt; (Smaller)</option>
              </select>
              <input type="number" min="0" placeholder="e.g. 50" bind:value={advSizeVal} onkeydown={(e) => e.key === 'Enter' && executeSearch(e.shiftKey)} />
              <select bind:value={advSizeUnit}>
                <option value="mb">MB</option>
                <option value="gb">GB</option>
                <option value="kb">KB</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label>Date Modified</label>
            <div class="input-row">
              <select bind:value={advDateOp}>
                <option value="<">Newer than</option>
                <option value=">">Older than</option>
              </select>
              <input type="number" min="0" placeholder="e.g. 7" bind:value={advDateVal} onkeydown={(e) => e.key === 'Enter' && executeSearch(e.shiftKey)} />
              <select bind:value={advDateUnit}>
                <option value="d">Days</option>
                <option value="w">Weeks</option>
                <option value="h">Hours</option>
              </select>
            </div>
          </div>

          <div class="form-group">
            <label>Item Type</label>
            <select bind:value={advType} class="full-width-select">
              <option value="any">Any (Files & Folders)</option>
              <option value="file">Files Only</option>
              <option value="dir">Folders Only</option>
            </select>
          </div>

          <div class="form-group checkbox-group">
            <label>
              <input type="checkbox" bind:checked={advHidden} />
              Include Hidden Items
            </label>
          </div>

        </div>
      </details>

      {#if fullSearchQuery.trim() !== '' || searchedQuery.trim() !== ''}
        <div class="search-results">
          {#if searchedQuery && fullSearchQuery === searchedQuery}
            <div class="search-status">Showing results for "{searchedQuery}"</div>
          {:else if fullSearchQuery.trim() !== ''}
            <div class="search-status dimmed">Press Enter to search for "{fullSearchQuery}"</div>
          {/if}

          {#each explorer.searchResults as file}
            <div class="result-item" role="button" tabindex="0" onclick={() => handleResultClick(file)} onkeydown={(e) => e.key === 'Enter' && handleResultClick(file)}>
              <span class="icon" style="width: 24px; height: 24px; display: flex;"><img src={getSystemIconSrc(file.path, file.is_dir)} alt="" style="width: 100%; height: 100%; object-fit: contain;" /></span>
              <div class="details">
                <span class="name">{file.name}</span>
                {#if file.snippet}
                  <span class="snippet">
                    {#if nameQueryOnly.trim() !== ''}
                      {@html escapeHtml(file.snippet).replace(new RegExp(escapeHtml(nameQueryOnly).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi'), match => `<mark class="search-highlight">${match}</mark>`)}
                    {:else}
                      {escapeHtml(file.snippet)}
                    {/if}
                  </span>
                {/if}
                <span class="path" title={file.path}>{file.path}</span>
              </div>
            </div>
          {/each}
          
          {#if !explorer.isSearching && explorer.searchResults.length === 0 && searchedQuery === fullSearchQuery && searchedQuery !== ''}
            <div class="no-results">No results found for "{searchedQuery}"</div>
          {/if}
        </div>
      {/if}
  </dialog>
{/if}

<style>
  dialog.search-modal {
    margin-top: 12vh;
    margin-bottom: auto;
  }
  dialog.search-modal::backdrop {
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
  }
  .search-modal {
    background: var(--bg-surface); border: 1px solid var(--border-color);
    border-radius: 12px; width: 600px; max-width: 90%;
    box-shadow: 0 15px 40px rgba(0,0,0,0.6); display: flex; flex-direction: column;
    overflow: hidden; backdrop-filter: blur(16px);
    animation: pop-in 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275) forwards;
  }
  .search-header {
    display: flex; align-items: center;
    padding: 15px 20px;
    border-bottom: 1px solid var(--border-color); background: var(--bg-base);
  }
  .search-icon { color: var(--text-muted); margin-right: 12px; }
  .search-header input {
    flex-grow: 1; background: transparent; border: none; color: var(--text-main);
    font-size: 1.2rem; outline: none;
    font-family: inherit;
  }
  .search-results {
    max-height: 50vh; overflow-y: auto; padding: 10px 0;
  }
  .result-item {
    display: flex; align-items: center; padding: 10px 20px; cursor: pointer;
    gap: 15px;
    border-bottom: 1px solid rgba(128,128,128,0.1);
  }
  .result-item:hover, .result-item:focus { background: var(--bg-hover); outline: none; }
  .result-item .icon { font-size: 1.5rem; }
  .details { display: flex; flex-direction: column; overflow: hidden; }
  .details .name { color: var(--text-main); font-weight: 500;
    font-size: 0.95rem; }
  .details .snippet { color: #bbbbbb; font-family: monospace; font-size: 0.8rem; margin: 2px 0; overflow: hidden; text-overflow: ellipsis;
    white-space: nowrap; }
  .details .path { color: var(--text-muted); font-size: 0.75rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .no-results { padding: 20px; text-align: center; color: var(--text-muted); }
  
  .spinner {
    width: 18px; height: 18px; border: 2px solid var(--border-color);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .search-status { padding: 10px 20px 5px; color: var(--text-main); font-size: 0.85rem; font-weight: 500; }
  .search-status.dimmed { color: var(--text-muted); font-style: italic; opacity: 0.7; }
  .search-hints { display: flex; flex-direction: column; gap: 4px;
    padding: 10px 20px 0; font-size: 0.8rem; color: var(--text-muted); }
  .search-hints strong { color: var(--text-main); }

  .advanced-filters {
    padding: 0 20px 10px;
    border-bottom: 1px solid rgba(128,128,128,0.1);
  }
  .advanced-filters summary {
    font-size: 0.8rem;
    color: var(--accent);
    cursor: pointer;
    user-select: none;
    margin-bottom: 8px;
    font-weight: 500;
  }
  
  /* --- New Form Builder Styles --- */
  .form-builder {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 15px;
    padding-top: 10px;
    padding-bottom: 5px;
  }
  .form-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .form-group label {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .input-row {
    display: flex;
    gap: 4px;
    align-items: stretch;
  }
  .input-row select, .full-width-select {
    background: var(--bg-hover);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 6px;
    font-size: 0.85rem;
    outline: none;
    cursor: pointer;
    transition: border-color 0.2s;
  }
  .input-row input {
    background: var(--bg-base);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 6px 8px;
    font-size: 0.85rem;
    width: 100%;
    min-width: 0;
    outline: none;
    transition: border-color 0.2s;
  }
  .input-row input:focus, .input-row select:focus, .full-width-select:focus {
    border-color: var(--accent);
  }
  .prefix {
    display: flex;
    align-items: center;
    font-size: 0.85rem;
    color: var(--text-muted);
    background: var(--bg-hover);
    padding: 0 8px;
    border: 1px solid var(--border-color);
    border-radius: 4px;
    user-select: none;
  }
  .checkbox-group {
    justify-content: flex-end; 
    padding-bottom: 5px;
  }
  .checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    color: var(--text-main);
    cursor: pointer;
    user-select: none;
  }
</style>