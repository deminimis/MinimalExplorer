import { invoke } from "@tauri-apps/api/core";
import { explorer, type FileItem } from './explorer.svelte';
import { isImage } from './utils';
import { settings } from './settings.svelte';
import { ui } from './ui.svelte';

export function decodeFileItems(buffer: Uint8Array | ArrayBuffer | number[]): FileItem[] {
  if (!buffer) return [];
  const u8 = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer as any);
  if (u8.byteLength < 4) return [];
  
  const dataView = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
  let offset = 0;
  const textDecoder = new TextDecoder('utf-8');

  const totalItems = dataView.getUint32(offset, true);
  offset += 4;

  const items: FileItem[] = [];
  for (let i = 0; i < totalItems; i++) {
    if (offset + 2 > u8.byteLength) break;
    
    const nameLen = dataView.getUint16(offset, true);
    offset += 2;
    const name = textDecoder.decode(new Uint8Array(u8.buffer, u8.byteOffset + offset, nameLen));
    offset += nameLen;

    const pathLen = dataView.getUint16(offset, true);
    offset += 2;
    const path = textDecoder.decode(new Uint8Array(u8.buffer, u8.byteOffset + offset, pathLen));
    offset += pathLen;

    const size = Number(dataView.getBigUint64(offset, true));
    offset += 8;

    const modified = Number(dataView.getBigUint64(offset, true));
    offset += 8;

    const flags = dataView.getUint8(offset);
    offset += 1;

    const is_dir = (flags & 1) !== 0;
    const is_hidden = (flags & 2) !== 0;
    const has_snippet = (flags & 4) !== 0;

    let snippet = null;
    if (has_snippet) {
      const snippetLen = dataView.getUint16(offset, true);
      offset += 2;
      snippet = textDecoder.decode(new Uint8Array(u8.buffer, u8.byteOffset + offset, snippetLen));
      offset += snippetLen;
    }

    items.push({ name, is_dir, path, size, modified, is_hidden, snippet });
  }
  return items;
}

const fileCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

export function sortFiles(files: FileItem[]): FileItem[] {
  return files.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1; 
    
    let comparison = 0;
    if (settings.sortBy === 'name') {
      comparison = fileCollator.compare(a.name, b.name);
    } else if (settings.sortBy === 'date') {
      comparison = a.modified - b.modified;
    } else if (settings.sortBy === 'size') {
      comparison = a.size - b.size;
    } else if (settings.sortBy === 'type') {
      const extA = a.name.includes('.') ? a.name.split('.').pop()!.toLowerCase() : '';
      const extB = b.name.includes('.') ? b.name.split('.').pop()!.toLowerCase() : '';
      comparison = fileCollator.compare(extA, extB) || fileCollator.compare(a.name, b.name);
    }
    return settings.sortAscending ? comparison : -comparison;
  });
}

export async function cancelSearch() {
  explorer.isSearching = false;
  await invoke("cancel_search");
}

let searchToken = 0;

export async function startSearch(path: string, query: string, content: boolean) {
  const currentToken = ++searchToken;
  await cancelSearch();
  
  if (currentToken !== searchToken) return; // Abort if another search started while we were cancelling

  // Strictly kill previous background searches
  explorer.searchResults = [];
  explorer.isSearching = true;
  ui.searchQuery = query;
  ui.showSearchModal = true;

  try {
    let searchPromiseDone = false;
    
    const searchPromise = invoke(content ? "search_contents" : "search_directory", { path, query })
      .catch(error => console.error("Search failed:", error))
      .finally(() => searchPromiseDone = true);

    // Poll for chunks
    while (explorer.isSearching && ui.searchQuery === query && currentToken === searchToken) {
      const buffer = await invoke<Uint8Array | ArrayBuffer | number[]>("poll_search_chunk");
      
      if (currentToken !== searchToken) break; 
      
      const u8 = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer as any);

      if (u8.byteLength > 0) {
        const batch = decodeFileItems(u8);
        explorer.searchResults = [...explorer.searchResults, ...batch];
      }

      // Exit if the backend search process has finished 
      if (searchPromiseDone && u8.byteLength === 0) {
        explorer.isSearching = false;
        break;
      }

      await new Promise(r => setTimeout(r, 16));
    }
  } catch (error) {
    console.error("Search loop failed:", error);
    if (currentToken === searchToken) explorer.isSearching = false;
  }
}

export async function setSortMode(mode: 'name' | 'date' | 'size' | 'type') {
  if (settings.sortBy === mode) {
    settings.sortAscending = !settings.sortAscending;
  } else {
    settings.sortBy = mode;
    settings.sortAscending = true;
  }
  
  settings.saveSettings();

  // Capture current selection paths before the reload wipes them
  const primaryPaths = new Set(explorer.selectedFiles.map(f => f.path));
  const secondaryPaths = new Set(explorer.secondarySelectedFiles.map(f => f.path));

  // Trigger a rapid zero-copy reload and await it so we can restore afterward
  await loadDirectory(explorer.currentPath, false, false, false);
  
  if (explorer.isTiledView) {
    const activeTab = explorer.tabs[explorer.activeTabIndex];
    if (activeTab?.secondaryPath) {
      await loadDirectory(activeTab.secondaryPath, false, false, true);
    }
  }

  // Restore selection using the newly loaded file references
  explorer.selectedFiles = explorer.files.filter(f => primaryPaths.has(f.path));
  explorer.secondarySelectedFiles = explorer.secondaryFiles.filter(f => secondaryPaths.has(f.path));
}


let primaryLoadToken = 0;
let secondaryLoadToken = 0;

export async function loadDirectory(path: string, addToHistory: boolean = true, isInit: boolean = false, isSecondary: boolean = false) {
  explorer.errorMessage = "";
  if (isSecondary) explorer.secondaryFilterQuery = "";
  else explorer.filterQuery = "";
  
  const currentToken = isSecondary ? ++secondaryLoadToken : ++primaryLoadToken;
  
  try {
    if (isSecondary) explorer.secondaryFiles = [];
    else explorer.files = [];

    const paneId = isSecondary ? "secondary" : "primary";
    // Stop any in-progress directory loads
    await invoke("cancel_load", { paneId });

    let loadPromiseDone = false;
    let allFiles: FileItem[] = [];
    // Trigger async read and sort in the background
    const loadPromise = invoke("get_files", { 
      paneId,
      path, 
      sortBy: settings.sortBy, 
      sortAscending: settings.sortAscending,
      showGitBadges: settings.showGitBadges
    }).catch(error => {
      console.error("Failed to read directory:", error);
      explorer.errorMessage = String(error);
    }).finally(() => loadPromiseDone = true);

    // Poll for chunks
    while (true) {
      const buffer = await invoke<Uint8Array | ArrayBuffer | number[]>("poll_load_chunk", { paneId });
      const u8 = buffer instanceof Uint8Array ? buffer : new Uint8Array(buffer as any);

      if (u8.byteLength > 0) {
        const batch = decodeFileItems(u8);
        allFiles = [...allFiles, ...batch];
        
        if (isSecondary) explorer.secondaryFiles = allFiles;
        else explorer.files = allFiles;

        if (settings.sessionViewOverrides.has(path)) {
          const override = settings.sessionViewOverrides.get(path)!;
          if (isSecondary) settings.secondaryViewMode = override;
          else settings.primaryViewMode = override;
        } else if (settings.autoViewMode && allFiles.length > 0) {
          const justFiles = allFiles.filter(f => !f.is_dir);
          if (justFiles.length > 0) {
            const imageCount = justFiles.filter(f => isImage(f.name)).length;
            const isGrid = (imageCount / justFiles.length) > 0.5;
            if (isSecondary) settings.secondaryViewMode = isGrid ? 'grid' : 'list';
            else settings.primaryViewMode = isGrid ? 'grid' : 'list';
          }
        }
      }

      if (loadPromiseDone && u8.byteLength === 0) break;
      
      const activeTokenAfter = isSecondary ? secondaryLoadToken : primaryLoadToken;
      if (currentToken !== activeTokenAfter) return; // Stop if user navigated away

      await new Promise(r => setTimeout(r, 16)); 
    }

    if (isSecondary) {
      if (explorer.tabs[explorer.activeTabIndex]) explorer.tabs[explorer.activeTabIndex].secondaryPath = path;
      if (addToHistory && !isInit) {
        if (explorer.tabs[explorer.activeTabIndex]) explorer.tabs[explorer.activeTabIndex].secondaryHistoryStack = [...(explorer.tabs[explorer.activeTabIndex].secondaryHistoryStack || []), path];
        if (explorer.tabs[explorer.activeTabIndex]) explorer.tabs[explorer.activeTabIndex].secondaryForwardStack = [];
      } else if (isInit) {
        if (explorer.tabs[explorer.activeTabIndex]) explorer.tabs[explorer.activeTabIndex].secondaryHistoryStack = [path];
      }
    } else {
      explorer.currentPath = path;
      if (addToHistory && !isInit) {
        explorer.historyStack = [...explorer.historyStack, path];
        explorer.forwardStack = [];
      } else if (isInit) {
        explorer.historyStack = [path];
      }
    }
    
    await invoke("watch_directory", { path });
    const activePaths = explorer.tabs.flatMap(t => t.isTiledView && t.secondaryPath ? [t.path, t.secondaryPath] : [t.path]);
    await invoke("unwatch_all_except", { activePaths });
    
    if (!isSecondary) {
      settings.recentPaths = [path, ...settings.recentPaths.filter(p => p !== path)].slice(0, 10);
    }
    settings.saveSettings();

    // Debounce thumbnail precaching
    if (settings.enableThumbnails) {
      clearTimeout((globalThis as any).thumbnailDebounce);
      (globalThis as any).thumbnailDebounce = setTimeout(() => {
        const schedule = window.requestIdleCallback || window.setTimeout;
        schedule(() => {
          const pathsToCache = allFiles
            .filter(f => !f.is_dir && isImage(f.name))
            .map(f => f.path)
            .slice(0, 50);
          if (pathsToCache.length > 0) {
            invoke("precache_thumbnails", { paths: pathsToCache });
          }
        });
      }, 150); 
    }
  } catch (error) {
    console.error("Failed to read directory:", error);
    explorer.errorMessage = String(error);
  }
}