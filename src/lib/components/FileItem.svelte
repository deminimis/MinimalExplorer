<script lang="ts">
  import { ui } from '$lib/ui.svelte';
  import { settings } from '$lib/settings.svelte';
  import { formatSize, formatDate, getSystemIconSrc, isImage } from '$lib/utils';
  import { submitInlineRename } from '$lib/actions';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import type { FileItem } from '$lib/explorer.svelte';

  let {
    file,
    index,
    isListView,
    isSelected,
    isCut,
    isSingleSelected,
    filterQuery,
    lazyLoadAction
  }: {
    file: FileItem;
    index: number;
    isListView: boolean;
    isSelected: boolean;
    isCut: boolean;
    isSingleSelected: boolean;
    filterQuery: string;
    lazyLoadAction: any;
  } = $props();

  function focusAndSelect(node: HTMLInputElement) {
    node.focus();
    const lastDot = node.value.lastIndexOf('.');
    if (lastDot > 0) node.setSelectionRange(0, lastDot);
    else node.select();
  }
</script>

{#snippet highlightedText(text: string, query: string)}
  {#if query.trim() === ""}
    {text}
  {:else}
    {@const index = text.toLowerCase().indexOf(query.toLowerCase())}
    {#if index !== -1}
      {text.slice(0, index)}<mark class="search-highlight">{text.slice(index, index + query.length)}</mark>{text.slice(index + query.length)}
    {:else}
      {text}
    {/if}
  {/if}
{/snippet}

{#if isListView}
  <tr 
    class="list-row {isSelected ? 'selected-item' : ''} {isCut ? 'cut-item' : ''}" 
    data-path={file.path}
    data-isdir={file.is_dir}
    data-index={index}
    role="button"
    tabindex="0"
  >
    <td class="list-icon-cell">
      <span class="list-icon">
        <img src={getSystemIconSrc(file.path, file.is_dir)} alt="icon" style="width: 100%; height: 100%; object-fit: contain;" />
      </span>
      <span class="file-name-text">
        {#if ui.inlineRenamePath === file.path}
          <input type="text" class="inline-rename-input" bind:value={ui.inlineRenameValue} onblur={submitInlineRename} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); submitInlineRename(); } else if (e.key === 'Escape') { e.stopPropagation(); ui.inlineRenamePath = null; } }} use:focusAndSelect onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()} />
        {:else}
          {@render highlightedText(file.name, filterQuery)}
        {/if}
        {#if file.is_dir && file.snippet}
          <span class="git-badge">⎇ {file.snippet}</span>
        {/if}
      </span>
    </td>
    <td class="meta-text">{formatDate(file.modified)}</td>
    <td class="meta-text">{formatSize(file.size, file.is_dir)}</td>
  </tr>
{:else}
  <div 
    class="tile {isSelected ? 'selected-item' : ''} {isSingleSelected ? 'single-selected' : ''} {isCut ? 'cut-item' : ''}"
    use:lazyLoadAction={settings.enableThumbnails && !file.is_dir && isImage(file.name) ? convertFileSrc(file.path, 'thumbnail') : null}
    data-path={file.path}
    data-isdir={file.is_dir}
    data-index={index}
    role="button"
    tabindex="0"
    onmousemove={(e: MouseEvent) => {
      const target = e.currentTarget as HTMLElement;
      const rect = target.getBoundingClientRect();
      target.style.setProperty('--mouse-x', `${e.clientX - rect.left}px`);
      target.style.setProperty('--mouse-y', `${e.clientY - rect.top}px`);
    }}
  >
    <div class="icon-wrapper">
      {#if settings.enableThumbnails && !file.is_dir && isImage(file.name)}
        <img 
          alt={file.name}
          class="gallery-img" 
          decoding="async"
          onload={(e: Event) => (e.currentTarget as HTMLImageElement).classList.add('loaded')}
          onerror={(e: Event) => {
            const target = e.currentTarget as HTMLImageElement;
            if (target.src.includes('thumbnail')) {
              target.src = convertFileSrc(file.path);
            } else if (!target.src.includes('icon.localhost')) {
              target.src = getSystemIconSrc(file.path, file.is_dir);
            } else {
              target.style.display = 'none';
              target.classList.add('loaded');
            }
          }}
        />
      {:else}
        <div class="tile-icon" style="width: 60%; height: 60%;">
          <img src={getSystemIconSrc(file.path, file.is_dir)} alt="icon" style="width: 100%; height: 100%; object-fit: contain;" />
        </div>
      {/if}
    </div>
    <div class="tile-name" title={file.name}>
      {#if ui.inlineRenamePath === file.path}
        <input type="text" class="inline-rename-input grid-rename" bind:value={ui.inlineRenameValue} onblur={submitInlineRename} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); submitInlineRename(); } else if (e.key === 'Escape') { e.stopPropagation(); ui.inlineRenamePath = null; } }} use:focusAndSelect onclick={(e) => e.stopPropagation()} ondblclick={(e) => e.stopPropagation()} />
      {:else}
        {@render highlightedText(file.name, filterQuery)}
      {/if}
    </div>
    {#if file.is_dir && file.snippet}
      <div class="git-badge grid-badge">⎇ {file.snippet}</div>
    {/if}
  </div>
{/if}

<style>
  .tile {
    width: 100%;
    box-sizing: border-box;
    content-visibility: auto;
    contain-intrinsic-size: calc(var(--tile-size) + 40px);
    background: transparent;
    border: 1px solid transparent;
    padding: 8px;
    border-radius: 8px;
    text-align: center;
    transition: all 0.2s ease;
    user-select: none;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    height: calc(var(--tile-size) + 40px);
    position: relative;
    overflow: visible; 
  }
  .tile::before {
    content: '';
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    border-radius: inherit;
    background: radial-gradient(
      120px circle at var(--mouse-x, 0) var(--mouse-y, 0),
      rgba(255, 255, 255, 0.08),
      transparent 100%
    );
    opacity: 0;
    transition: opacity 0.2s ease;
    pointer-events: none;
    z-index: 0;
  }
  :global([data-theme="light"]) .tile::before {
    background: radial-gradient(
      120px circle at var(--mouse-x, 0) var(--mouse-y, 0),
      rgba(0, 0, 0, 0.05),
      transparent 100%
    );
  }
  .tile:hover::before {
    opacity: 1;
  }
  .tile:hover {
    background: var(--bg-hover);
    border-color: var(--border-color);
  }
  .icon-wrapper {
    width: var(--tile-size);
    height: var(--tile-size);
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    border-radius: 4px;
    overflow: hidden;
  }
  .icon-wrapper:has(.gallery-img:not(:global(.loaded)))::before {
    content: "";
    position: absolute;
    top: 0; left: 0; width: 100%; height: 100%;
    background: linear-gradient(90deg, transparent, rgba(128, 128, 128, 0.1), transparent);
    transform: translateX(-100%);
    animation: skeleton-sweep 1.2s infinite linear;
    z-index: 0;
    pointer-events: none;
  }
  @keyframes skeleton-sweep {
    100% { transform: translateX(100%); }
  }
  .tile-icon {
    font-size: calc(var(--tile-size) * 0.5);
  }
  .gallery-img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    background-color: transparent;
    z-index: 1;
    border-radius: 6px;
    box-sizing: border-box;
    border: 1px solid rgba(255, 255, 255, 0.15); 
    opacity: 0;
    transition: opacity 0.1s ease;
  }
  
  .gallery-img.loaded {
    opacity: 1;
  }
  :global([data-theme="light"]) .gallery-img {
    border: 1px solid rgba(0, 0, 0, 0.15);
  }
  .tile-name {
    font-size: 0.85rem;
    font-weight: 500;
    letter-spacing: 0.01em;
    padding-top: 8px;
    width: 100%;
    word-wrap: break-word;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    white-space: normal;
  }
  .single-selected .tile-name {
    overflow: visible;
    -webkit-line-clamp: unset;
    line-clamp: unset;
    display: block;
    position: absolute;
    top: calc(var(--tile-size) + 8px);
    background: var(--bg-surface);
    color: var(--text-main);
    z-index: 100;
    padding: 4px;
    border-radius: 4px;
    border: 1px solid var(--accent);
    box-shadow: 0 4px 12px rgba(0,0,0,0.6);
    width: max-content;
    max-width: 250px;
    left: 50%;
    transform: translateX(-50%);
  }
  .list-icon-cell {
    display: flex;
    align-items: center;
    overflow: hidden;
  }
  .list-icon {
    margin-right: 8px;
    font-size: calc(1.2rem * var(--zoom-scale));
    display: flex;
    align-items: center;
    justify-content: center;
    width: calc(20px * var(--zoom-scale));
    height: calc(20px * var(--zoom-scale));
    flex-shrink: 0;
  }
  .selected-item {
    background: rgba(128, 128, 128, 0.2) !important;
    border-color: rgba(128, 128, 128, 0.5) !important;
  }
  .cut-item {
    opacity: 0.5;
    transition: opacity 0.2s ease;
  }
  .file-name-text {
    font-weight: 500;
    letter-spacing: 0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta-text {
    color: var(--text-muted);
    font-size: 0.8rem;
    letter-spacing: 0.02em;
  }
  .inline-rename-input {
    background: var(--bg-solid);
    color: var(--text-main);
    border: 1px solid var(--accent);
    border-radius: 3px;
    padding: 2px 4px;
    font-family: inherit;
    font-size: 0.85rem;
    width: 100%;
    box-sizing: border-box;
    outline: none;
  }
  .grid-rename {
    text-align: center;
    margin-top: 2px;
  }
  .search-highlight {
    background-color: rgba(255, 165, 0, 0.6);
    color: var(--text-main);
    border-radius: 2px;
    padding: 0 1px;
  }
  .git-badge {
    background: rgba(46, 160, 67, 0.15);
    color: #3fb950;
    border: 1px solid rgba(46, 160, 67, 0.4);
    font-size: 0.7rem;
    padding: 1px 6px;
    border-radius: 10px;
    margin-left: 8px;
    vertical-align: middle;
    white-space: nowrap;
  }
  .grid-badge {
    margin-left: 0;
    margin-top: 4px;
    display: inline-block;
  }
</style>