<script lang="ts">
  import { explorer } from '$lib/explorer.svelte';
  import { settings } from '$lib/settings.svelte';
  import { ui } from '$lib/ui.svelte';
  import { loadDirectory } from '$lib/actions';
  import { invoke } from '@tauri-apps/api/core';
  import type { FileItem } from '$lib/explorer.svelte';
  
  let { isSecondary = false } = $props();
  let activeTab = $derived(explorer.tabs[explorer.activeTabIndex] || {});
  let currentPath = $derived(isSecondary ? (activeTab.secondaryPath || "C:\\") : explorer.currentPath);
  let historyStack = $derived(isSecondary ? (activeTab.secondaryHistoryStack || []) : explorer.historyStack);
  let forwardStack = $derived(isSecondary ? (activeTab.secondaryForwardStack || []) : explorer.forwardStack);
  let rawSegments = $derived(currentPath.split(/[/\\]/).filter(Boolean));
  let pathSegments = $derived.by(() => {
    const maxSegments = 5; 
    if (rawSegments.length <= maxSegments) return rawSegments.map((s, i) => ({ name: s, index: i, isTruncated: false }));
    
    const start = rawSegments.slice(0, 2);
    const end = rawSegments.slice(-2);
    return [
      ...start.map((s, i) => ({ name: s, index: i, isTruncated: false })),
      { name: '...', index: -1, isTruncated: true },
      ...end.map((s, i) => ({ name: s, index: rawSegments.length - 2 + i, isTruncated: false }))
    ];
  });
  
  let isEditingPath = $state(false);
  let editPathValue = $state("");
  let debounceTimer: ReturnType<typeof setTimeout>;
  let autocompleteItems = $state<FileItem[]>([]);
  let autocompleteToken = 0;

  async function handlePathAutocomplete() {
    if (!isEditingPath || !editPathValue) {
      autocompleteItems = [];
      return;
    }
    
    const normalized = editPathValue.replace(/\//g, '\\');
    const lastSlash = normalized.lastIndexOf('\\');
    
    const parentPath = lastSlash >= 0 ? normalized.substring(0, lastSlash + 1) : "";
    const search = lastSlash >= 0 ? normalized.substring(lastSlash + 1).toLowerCase() : normalized.toLowerCase();
    
    if (!parentPath) return;

    const currentToken = ++autocompleteToken;
    try {
      const dirs = await invoke<FileItem[]>('get_directories', { path: parentPath, showGitBadges: settings.showGitBadges });
      if (currentToken === autocompleteToken) {
        autocompleteItems = dirs.filter(d => d.name.toLowerCase().startsWith(search)).slice(0, 8);
      }
    } catch {
      if (currentToken === autocompleteToken) autocompleteItems = [];
    }
  }

  let activeDropdownIndex = $state<number | null>(null);
  let dropdownItems = $state<FileItem[]>([]);
  let dropdownX = $state(0);
  let dropdownY = $state(0);
  let dropdownToken = 0;

  async function openDropdown(e: MouseEvent, index: number) {
    e.stopPropagation();
    if (activeDropdownIndex === index) {
      activeDropdownIndex = null;
      return;
    }

    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const startX = rect.left;
    const startY = rect.bottom + 2;

    const targetPath = getPathForSegment(index);
    const currentToken = ++dropdownToken;
    
    try {
      const items = await invoke<FileItem[]>('get_directories', { path: targetPath, showGitBadges: settings.showGitBadges });
      if (currentToken === dropdownToken) {
        dropdownItems = items;
        dropdownX = startX;
        dropdownY = startY;
        activeDropdownIndex = index;
      }
    } catch (error) {
      console.error(error);
    }
  }
  function closeDropdown() {
    activeDropdownIndex = null;
  }

  function focusInput(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function getPathForSegment(index: number) {
    let path = rawSegments.slice(0, index + 1).join("\\");
    if (currentPath.startsWith("\\\\") || currentPath.startsWith("//")) {
      path = "\\\\" + path;
    }
    if (path.length === 2 && path.endsWith(":")) path += "\\";
    return path;
  }

  function handleGoBack() {
    if (historyStack.length > 1) {
      const activeTab = explorer.tabs[explorer.activeTabIndex];
      const current = historyStack[historyStack.length - 1];
      const prev = historyStack[historyStack.length - 2];

      if (isSecondary) {
        activeTab.secondaryForwardStack = [current, ...(activeTab.secondaryForwardStack || [])];
        activeTab.secondaryHistoryStack = activeTab.secondaryHistoryStack!.slice(0, -1);
      } else {
        activeTab.forwardStack = [current, ...activeTab.forwardStack];
        activeTab.historyStack = activeTab.historyStack.slice(0, -1);
      }
      
      loadDirectory(prev, false, false, isSecondary);
    }
  }

  function handleGoForward() {
    if (forwardStack.length > 0) {
      const activeTab = explorer.tabs[explorer.activeTabIndex];
      const next = forwardStack[0];

      if (isSecondary) {
        activeTab.secondaryHistoryStack = [...(activeTab.secondaryHistoryStack || []), next];
        activeTab.secondaryForwardStack = activeTab.secondaryForwardStack!.slice(1);
      } else {
        activeTab.historyStack = [...activeTab.historyStack, next];
        activeTab.forwardStack = activeTab.forwardStack.slice(1);
      }
      
      loadDirectory(next, false, false, isSecondary);
    }
  }

  function handleGoUp() {
    if (currentPath === "This PC") return;
    
    const segments = currentPath.split(/[/\\]/).filter(Boolean);
    // going up from drive root takes us to "This PC"
    if (segments.length === 1 && segments[0].endsWith(':')) {
      loadDirectory("This PC", true, false, isSecondary);
      return;
    }
    
    if (segments.length > 0) {
      segments.pop();
      let parentPath = segments.join("\\");
      
      if (currentPath.startsWith("\\\\") || currentPath.startsWith("//")) {
        if (segments.length > 0) {
          parentPath = "\\\\" + parentPath;
        } else {
          parentPath = "This PC"; 
        }
      }
      
      if (parentPath.length === 2 && parentPath.endsWith(":")) parentPath += "\\";
      if (!parentPath) parentPath = "This PC";
      loadDirectory(parentPath, true, false, isSecondary);
    }
  }

  function handleInputEnter() {
    loadDirectory(currentPath, true, false, isSecondary);
  }

  $effect(() => {
    if (ui.triggerAddressFocus > 0 && ((isSecondary && explorer.focusedPane === 'secondary') || (!isSecondary && explorer.focusedPane === 'primary'))) {
      isEditingPath = true;
      editPathValue = currentPath;
    }
    if (ui.triggerFilterFocus > 0 && ((isSecondary && explorer.focusedPane === 'secondary') || (!isSecondary && explorer.focusedPane === 'primary'))) {
      document.querySelector<HTMLInputElement>(isSecondary ? '.secondary-pane .filter-input' : '.primary-pane .filter-input')?.focus();
    }
  });
</script>

<svelte:window onclick={closeDropdown} />

{#if activeDropdownIndex !== null}
      <div 
        class="breadcrumb-dropdown" 
        style="top: {dropdownY}px; left: {dropdownX}px;"
        role="presentation" 
        onclick={(e) => e.stopPropagation()}
      >
    {#each dropdownItems as item}
      <button onclick={() => {
        loadDirectory(item.path, true, false, isSecondary);
        activeDropdownIndex = null;
      }}>
        <span class="icon">📁</span> {item.name}
      </button>
    {/each}
    {#if dropdownItems.length === 0}
      <div style="padding: 10px; color: var(--text-muted); font-size: 0.85rem;">Empty</div>
    {/if}
  </div>
{/if}

<div class="address-bar">
  <button class="icon-btn" onclick={handleGoBack} disabled={historyStack.length <= 1} title="Back">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="15 18 9 12 15 6"></polyline></svg>
  </button>
  <button class="icon-btn" onclick={handleGoForward} disabled={forwardStack.length === 0} title="Forward">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6"></polyline></svg>
  </button>
  <button class="icon-btn" onclick={handleGoUp} title="Up">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="18 15 12 9 6 15"></polyline></svg>
  </button>

  <div 
    class="breadcrumbs {isEditingPath ? 'editing' : ''}" 
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget) {
        isEditingPath = true;
        editPathValue = currentPath;
        autocompleteItems = [];
      }
    }}
  >
    {#if isEditingPath}
      <div class="input-wrapper">
        <input 
          type="text" 
          class="path-input"
          bind:value={editPathValue} 
          use:focusInput
          oninput={handlePathAutocomplete}
          onkeydown={(e) => { 
            if (e.key === 'Enter') { 
              loadDirectory(editPathValue, true, false, isSecondary);
              isEditingPath = false; 
              autocompleteItems = [];
            }
            if (e.key === 'Escape') {
              isEditingPath = false;
              autocompleteItems = [];
            }
          }}
          onblur={() => setTimeout(() => { isEditingPath = false; autocompleteItems = []; }, 150)}
        />
        {#if autocompleteItems.length > 0}
          <div class="autocomplete-dropdown" role="presentation" onclick={(e) => e.stopPropagation()}>
            {#each autocompleteItems as item}
              <button onclick={() => { 
                editPathValue = item.path; 
                loadDirectory(item.path, true, false, isSecondary); 
                isEditingPath = false; 
                autocompleteItems = []; 
              }}>
                <span class="icon">📁</span> {item.name}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {:else}
      {#each pathSegments as seg}
        {#if seg.isTruncated}
          <span class="breadcrumb-separator">...</span>
        {:else}
          <button 
            class="breadcrumb-btn" 
            onclick={() => loadDirectory(getPathForSegment(seg.index), true, false, isSecondary)}
          >
            {seg.name}
          </button>
          <button 
          class="breadcrumb-separator chevron-btn" 
          onclick={(e) => {
            if (seg.index !== -1) openDropdown(e, seg.index);
          }}
          title="See subfolders"
        >
          ›
        </button>
        {/if}
      {/each}
    {/if}
  </div>
  
  <div class="slider-container">
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"></circle><line x1="21" y1="21" x2="16.65" y2="16.65"></line><line x1="11" y1="8" x2="11" y2="14"></line><line x1="8" y1="11" x2="14" y2="11"></line></svg>
    <input 
      type="range" 
      min="80" 
      max="300" 
      bind:value={settings.currentFolderIconSize} 
      onchange={() => settings.saveSettings()} 
      title="Adjust zoom level"
    />
  </div>

  <div class="filter-container">
    <input 
      type="text" 
      class="filter-input" 
      placeholder="Filter items..." 
      value={isSecondary ? explorer.secondaryFilterQuery : explorer.filterQuery}
      oninput={(e) => {
        clearTimeout(debounceTimer);
        const val = e.currentTarget.value;
        debounceTimer = setTimeout(() => {
          if (isSecondary) explorer.secondaryFilterQuery = val;
          else explorer.filterQuery = val;
        }, 150);
      }}
    />
    {#if (isSecondary ? explorer.secondaryFilterQuery : explorer.filterQuery).trim() !== ""}
      <div class="hints-container">
        <button class="deep-search-hint" onclick={() => {
            const q = isSecondary ? explorer.secondaryFilterQuery : explorer.filterQuery;
            import('$lib/actions').then(m => m.startSearch(currentPath, q, false));
        }}>
          ↳ Search subdirectories
        </button>
        <button class="deep-search-hint content-search" onclick={() => {
            const q = isSecondary ? explorer.secondaryFilterQuery : explorer.filterQuery;
            import('$lib/actions').then(m => m.startSearch(currentPath, q, true));
        }}>
          ↳ Search contents
        </button>
      </div>
    {/if}
  </div>
</div>

<style>
  .address-bar {
    display: flex;
    gap: 10px;
    margin-bottom: 1rem;
    align-items: center;
  }
  .slider-container {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.85rem;
    margin-left: 1rem;
  }
  .breadcrumbs {
    flex-grow: 1;
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 4px 8px;
    min-height: 20px;
    overflow-x: auto;
    white-space: nowrap;
    cursor: text;
  }
  .breadcrumbs.editing {
    overflow: visible; 
  }
  .input-wrapper {
    position: relative;
    width: 100%;
    display: flex;
  }
  .autocomplete-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    z-index: 2000;
    background: var(--bg-menu);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    box-shadow: 2px 4px 16px rgba(0,0,0,0.4);
    max-height: 250px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    margin-top: 4px;
    padding: 4px 0;
  }
  .autocomplete-dropdown button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-main);
    padding: 8px 16px;
    text-align: left;
    font-size: 0.9rem;
    cursor: pointer;
  }
  .autocomplete-dropdown button:hover {
    background: var(--accent);
    color: white;
  }
  .breadcrumbs::-webkit-scrollbar {
    display: none;
  }
  .breadcrumb-btn {
    background: transparent;
    border: none;
    color: var(--text-main);
    padding: 4px 6px;
    border-radius: 4px;
    font-size: 0.9rem;
  }
  .breadcrumb-btn:hover {
    background: var(--bg-hover);
  }
  .breadcrumb-separator {
    color: var(--text-muted);
    font-size: 1.1rem;
    margin: 0 2px;
    user-select: none;
    background: transparent;
    border: none;
    padding: 0 4px;
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    align-items: center;
  }
  .breadcrumb-separator:hover {
    background: var(--bg-hover);
    color: var(--text-main);
  }
  .breadcrumb-dropdown {
    position: fixed;
    max-height: 400px;
    overflow-y: auto;
    z-index: 2000;
    min-width: 220px;
    background: var(--bg-menu);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    box-shadow: 2px 4px 16px rgba(0, 0, 0, 0.4);
    padding: 4px 0;
    transform-origin: top left;
    animation: drop-slide 0.15s cubic-bezier(0.2, 0, 0, 1) forwards;
    will-change: transform, opacity;
    transform: translateZ(0);
    backface-visibility: hidden;
  }

  @keyframes drop-slide {
    0% { opacity: 0; transform: translateY(-8px) scale(0.98); }
    100% { opacity: 1; transform: translateY(0) scale(1); }
  }

  .breadcrumb-dropdown button {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text-main);
    padding: 8px 16px;
    text-align: left;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background-color 0.1s ease;
  }
  
  .breadcrumb-dropdown button:hover {
    background: var(--accent);
    color: white;
  }
  .filter-container {
    position: relative;
    display: flex;
    flex-direction: column;
  }
  .hints-container {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    z-index: 100;
  }
  .deep-search-hint {
    background: var(--bg-menu);
    border: 1px solid var(--accent);
    color: var(--text-main);
    padding: 6px 12px;
    border-radius: 6px;
    font-size: 0.8rem;
    white-space: nowrap;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0,0,0,0.4);
    animation: pop-in 0.2s ease forwards;
  }
  .deep-search-hint.content-search {
    border-color: #bb86fc;
  }
  .deep-search-hint.content-search:hover {
    background: #bb86fc;
  }
  .deep-search-hint:hover { background: var(--accent); color: white; }

  .filter-input {
    background: var(--bg-surface);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    color: var(--text-main);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    padding: 4px 8px;
    width: 140px;
    font-size: 0.9rem;
    outline: none;
    font-family: inherit;
  }
  .filter-input:focus {
    border-color: var(--accent);
  }
  .path-input {
    width: 100%;
    background: var(--bg-base);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    color: var(--text-main);
    border: 1px solid var(--accent);
    padding: 2px 6px;
    border-radius: 4px;
    outline: none;
    font-family: inherit;
  }
  button.icon-btn {
    background: transparent;
    border: none;
    padding: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted); 
    border-radius: 6px;
    transition: background-color 0.2s ease, color 0.2s ease;
  }
  button.icon-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-main);
  }

  /* Range Slider */
  .slider-container input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 90px;
    height: 4px;
    background: var(--border-color);
    border-radius: 2px;
    outline: none;
  }
  .slider-container input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 6px;
    height: 16px;
    border-radius: 3px;
    background: var(--text-muted);
    cursor: pointer;
    transition: background 0.15s ease;
  }
  .slider-container input[type="range"]::-webkit-slider-thumb:hover {
    background: var(--text-main);
  }
</style>