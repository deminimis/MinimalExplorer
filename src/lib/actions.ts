import { invoke } from "@tauri-apps/api/core";
import { explorer, type FileItem } from './explorer.svelte';
import { isImage } from './utils';
import { settings } from './settings.svelte';
import { ui } from './ui.svelte';
import { loadDirectory } from './core';

export * from './core';


export function handleItemContextMenu(event: MouseEvent, file: FileItem, isSecondary: boolean = false) {
  event.preventDefault();
  event.stopPropagation();
  
  const selection = isSecondary ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  
  if (!selection.find(f => f.path === file.path)) {
    if (isSecondary) {
      explorer.secondarySelectedFiles = [file];
    } else {
      explorer.selectedFiles = [file];
    }
  }
  
  ui.shiftKeyPressed = event.shiftKey;
  ui.menuType = 'item';
  ui.contextMenuPane = isSecondary ? 'secondary' : 'primary';
  ui.contextMenuX = Math.min(event.clientX, window.innerWidth - 160);
  ui.contextMenuY = Math.min(event.clientY, window.innerHeight - 420);
  ui.contextMenuVisible = true;
}

export function togglePinItem(name: string, path: string, is_dir: boolean = true) {
  const exists = settings.pinnedFolders.find(p => p.path === path);
  if (exists) {
    settings.pinnedFolders = settings.pinnedFolders.filter(p => p.path !== path);
  } else {
    settings.pinnedFolders = [...settings.pinnedFolders, { name, path, is_dir }];
  }
  settings.saveSettings();
  ui.closeContextMenu();
}

export function handleSidebarContextMenu(event: MouseEvent, path: string) {
  event.preventDefault();
  event.stopPropagation();
  ui.shiftKeyPressed = event.shiftKey;
  ui.menuType = 'sidebar';
  ui.contextMenuSidebarPath = path;
  ui.contextMenuX = Math.min(event.clientX, window.innerWidth - 160);
  ui.contextMenuY = Math.min(event.clientY, window.innerHeight - 140);
  ui.contextMenuVisible = true;
}

export function handleBackgroundContextMenu(event: MouseEvent, isSecondary: boolean = false) {
  event.preventDefault();
  
  if (isSecondary) {
    explorer.secondarySelectedFiles = [];
  } else {
    explorer.selectedFiles = [];
  }
  
  ui.shiftKeyPressed = event.shiftKey;
  ui.menuType = 'background';
  ui.contextMenuPane = isSecondary ? 'secondary' : 'primary';
  ui.contextMenuX = Math.min(event.clientX, window.innerWidth - 160);
  ui.contextMenuY = Math.min(event.clientY, window.innerHeight - 490);
  ui.contextMenuVisible = true;
}

export function handleCopy() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length > 0) {
    explorer.clipboardFiles = [...selection];
    explorer.clipboardAction = 'copy';
    ui.closeContextMenu();
  }
}

export function handleCut() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length > 0) {
    explorer.clipboardFiles = [...selection];
    explorer.clipboardAction = 'cut';
    ui.closeContextMenu();
  }
}

let isPasting = false;

export async function handlePaste(targetPath: string = explorer.currentPath) {
  if (isPasting || explorer.clipboardFiles.length === 0 || !explorer.clipboardAction) return;
  
  isPasting = true;
  try {
    const successfulCuts = new Set<string>();

    for (const file of explorer.clipboardFiles) {
      const separator = targetPath.endsWith('\\') || targetPath.endsWith('/') ? '' : '\\';
      const dest = `${targetPath}${separator}${file.name}`;
      
      // Prevent identical path overwrite 
      if (file.path.toLowerCase().replace(/\//g, '\\') === dest.toLowerCase().replace(/\//g, '\\')) {
         explorer.errorMessage = `Cannot paste ${file.name} into the same directory.`;
         continue;
      }

      try {
        if (explorer.clipboardAction === 'copy') {
          await invoke("copy_item", { src: file.path, dest });
        } else if (explorer.clipboardAction === 'cut') {
          await invoke("move_item", { src: file.path, dest });
          explorer.undoStack.push({ type: 'move', src: file.path, dest });
          settings.saveSettings();
          successfulCuts.add(file.path);
        }
      } catch (error) {
        explorer.errorMessage = `Paste failed for ${file.name}: ${error}`;
      }
    }
    
    if (explorer.clipboardAction === 'cut') {
      // Only remove items that were successfully moved to prevent clipboard loss
      explorer.clipboardFiles = explorer.clipboardFiles.filter(f => !successfulCuts.has(f.path));
      if (explorer.clipboardFiles.length === 0) explorer.clipboardAction = null;
    }
    
    // Refresh both panes to prevent ghost files in dual-pane view
    loadDirectory(explorer.currentPath, false, false, false);
    if (explorer.isTiledView) {
      const activeTab = explorer.tabs[explorer.activeTabIndex];
      if (activeTab?.secondaryPath) loadDirectory(activeTab.secondaryPath, false, false, true);
    }
    ui.closeContextMenu();
  } finally {
    isPasting = false;
  }
}

export function openCreateModal(event: MouseEvent, type: 'file' | 'folder') {
  event.stopPropagation();
  ui.createModalType = type;
  ui.createInputName = type === 'folder' ? "New Folder" : "New Text Document.txt";
  ui.createModalX = Math.min(ui.contextMenuX, window.innerWidth - 260 - 20);
  ui.createModalY = Math.min(ui.contextMenuY, window.innerHeight - 120 - 20);
  ui.showCreateModal = true;
  ui.closeContextMenu();
}

export async function submitCreate() {
  if (!ui.createInputName.trim()) return;
  try {
    const separator = explorer.currentPath.endsWith('\\') || explorer.currentPath.endsWith('/') ? '' : '\\';
    const targetPath = `${explorer.currentPath}${separator}${ui.createInputName}`;
    await invoke("create_item", { path: targetPath, isDir: ui.createModalType === 'folder' });
    loadDirectory(explorer.currentPath, false);
  } catch (error) {
    explorer.errorMessage = `Failed to create ${ui.createModalType}: ${error}`;
  }
  ui.showCreateModal = false;
}

export async function launchTerminal(admin: boolean) {
  const targetFile = explorer.selectedFiles.length === 1 ? explorer.selectedFiles[0] : null;
  const targetPath = (targetFile && targetFile.is_dir) ? targetFile.path : explorer.currentPath;
  try {
    const shellCommand = settings.externalTerminalType === 'custom' ? settings.customExternalTerminal : settings.externalTerminalType;
    await invoke("open_in_terminal", { path: targetPath, admin, shell: shellCommand });
  } catch (error) {
    explorer.errorMessage = `Failed to open terminal: ${error}`;
  }
  ui.closeContextMenu();
}

export async function deleteSelected(permanent: boolean = false) {
  const selection = explorer.focusedPane === 'secondary' ?
    explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length === 0) return;
  
  const actionText = permanent ? 'permanently delete' : 'move to the Recycle Bin';
  const itemText = selection.length === 1 ? `"${selection[0].name}"` : `${selection.length} items`;
  if (!confirm(`Are you sure you want to ${actionText} ${itemText}?`)) {
    ui.closeContextMenu();
    return;
  }

  // Extract just the paths for the batch operation
  const paths = selection.map(f => f.path);

  try {
    await invoke("delete_items", { paths, permanent });
  } catch (error) {
    explorer.errorMessage = `Delete failed: ${error}`;
  }
  
  settings.saveSettings();
  if (explorer.focusedPane === 'secondary') explorer.secondarySelectedFiles = [];
  else explorer.selectedFiles = [];
  
  const targetPath = explorer.focusedPane === 'secondary' ? (explorer.tabs[explorer.activeTabIndex]?.secondaryPath || "C:\\") : explorer.currentPath;
  loadDirectory(targetPath, false, false, explorer.focusedPane === 'secondary');
  
  ui.closeContextMenu();
}

export async function renameSelected() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length !== 1) return;
  
  ui.inlineRenamePath = selection[0].path;
  ui.inlineRenameValue = selection[0].name;
  ui.closeContextMenu();
}

export async function submitInlineRename() {
  if (!ui.inlineRenamePath || !ui.inlineRenameValue.trim()) {
    ui.inlineRenamePath = null;
    return;
  }
  const oldPath = ui.inlineRenamePath;
  const newName = ui.inlineRenameValue.trim();
  ui.inlineRenamePath = null;
  
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  const file = selection.find(f => f.path === oldPath);
  if (!file || file.name === newName) return;

  const pathParts = oldPath.split(/[/\\]/);
  pathParts.pop();
  let parentPath = pathParts.join("\\");
  if (parentPath.length === 2 && parentPath.endsWith(':')) parentPath += "\\";
  const newPath = parentPath.endsWith("\\") ? `${parentPath}${newName}` : `${parentPath}\\${newName}`;

  try {
    await invoke("rename_item", { oldPath, newPath });
    explorer.undoStack.push({ type: 'rename', src: oldPath, dest: newPath });
    settings.saveSettings();
    const targetPath = explorer.focusedPane === 'secondary' ? (explorer.tabs[explorer.activeTabIndex]?.secondaryPath || "C:\\") : explorer.currentPath;
    loadDirectory(targetPath, false, false, explorer.focusedPane === 'secondary');
  } catch (error) {
    explorer.errorMessage = `Rename failed: ${error}`;
  }
}

export async function handleOpenWith() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length === 1 && !selection[0].is_dir) {
    try {
      await invoke("open_with", { path: selection[0].path });
    } catch (error) {
      explorer.errorMessage = `Failed to Open With: ${error}`;
    }
  }
  ui.closeContextMenu();
}

export async function handlePreview() {
  if (explorer.selectedFiles.length === 1 && !explorer.selectedFiles[0].is_dir) {
    const file = explorer.selectedFiles[0];
    const lowerName = file.name.toLowerCase();

    if (file.size > 20 * 1024 * 1024) { // 20MB limit
      explorer.errorMessage = "File too large to preview.";
      ui.closeContextMenu();
      return;
    }
    
    // Clear previous states
    explorer.previewImagePath = null;
    explorer.previewTextContent = null;
    explorer.previewPdfPath = null;
    explorer.isEditingPreview = false;

    if (lowerName.endsWith('.pdf')) {
      explorer.previewPdfPath = file.path;
      explorer.previewFilePath = file.path;
    } else if (isImage(file.name)) {
      explorer.previewImagePath = file.path;
      explorer.previewFilePath = file.path;
    } else {
      try {
        // Initialize state for incremental loading
        explorer.previewOffset = 0;
        const CHUNK_SIZE = 1024 * 1024; // 1MB
        
        const text = await invoke<string>("read_text_file", { 
          path: file.path, 
          offset: 0, 
          length: CHUNK_SIZE
        });
        
        if (explorer.selectedFiles.length !== 1 || explorer.selectedFiles[0].path !== file.path) return;

        explorer.previewTextContent = text;
        explorer.previewFilePath = file.path;
        explorer.previewOffset = CHUNK_SIZE;
      } catch (error) {
        explorer.errorMessage = `Cannot preview this file type: ${error}`;
      }
    }
  }
  ui.closeContextMenu();
}

export function openInNewTab(path: string) {
  explorer.addNewTab(path, true);
  ui.closeContextMenu();
}

export function openInOtherPane(path: string) {
  if (!settings.splitDirection) settings.splitDirection = 'vertical';
  explorer.isTiledView = true;
  const activeTab = explorer.tabs[explorer.activeTabIndex];
  if (activeTab) activeTab.secondaryPath = path;
  loadDirectory(path, true, false, true);
  ui.closeContextMenu();
}

export async function openProperties() {
  let targetPath = explorer.currentPath;
  
  if (ui.menuType === 'item') {
    const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
    if (selection.length > 0) targetPath = selection[0].path; 
  } else if (ui.menuType === 'sidebar') {
    targetPath = ui.contextMenuSidebarPath || explorer.currentPath;
  } else if (explorer.focusedPane === 'secondary') {
    const activeTab = explorer.tabs[explorer.activeTabIndex];
    targetPath = activeTab?.secondaryPath || "C:\\";
  }

  try {
    const props = await invoke("get_file_properties", { path: targetPath });
    ui.propertiesData = props;
    ui.showPropertiesModal = true;
  } catch (error) {
    explorer.errorMessage = `Failed to get properties: ${error}`;
  }
  ui.closeContextMenu();
}

export async function openWindowsProperties() {
  if (ui.propertiesData) {
    try {
      await invoke("show_windows_properties", { path: ui.propertiesData.path });
      ui.showPropertiesModal = false;
    } catch (error) {
      explorer.errorMessage = `Failed to open Windows properties: ${error}`;
    }
  }
}

export function handleCompress() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length === 0) return;
  
  ui.archiveMode = 'compress';
  ui.archiveInputName = `${selection[0].name}.zip`;
  ui.archiveMethod = 'deflated';
  ui.archiveLevel = 6;
  ui.showArchiveModal = true;
  ui.closeContextMenu();
}

export async function handleExtractHere() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length === 0) return;
  const file = selection[0];
  const dest = explorer.currentPath;
  
  ui.closeContextMenu();
  ui.archiveMode = 'extract';
  ui.isArchiving = true;
  ui.showArchiveModal = true;

  try {
    await invoke("extract_item", { src: file.path, dest });
    loadDirectory(explorer.currentPath, false);
  } catch(e) { 
    explorer.errorMessage = String(e); 
  }
  
  ui.isArchiving = false;
  ui.showArchiveModal = false;
}

export function handleExtractToFolder() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length === 0) return;
  
  ui.archiveMode = 'extract';
  ui.archiveInputName = selection[0].name.replace(/\.[^/.]+$/, "");
  ui.showArchiveModal = true;
  ui.closeContextMenu();
}

export async function submitArchive() {
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  if (selection.length === 0 || !ui.archiveInputName.trim()) return;
  
  const file = selection[0];
  const separator = explorer.currentPath.endsWith('\\') || explorer.currentPath.endsWith('/') ? '' : '\\';
  const dest = `${explorer.currentPath}${separator}${ui.archiveInputName.trim()}`;

  ui.isArchiving = true;

  try {
    if (ui.archiveMode === 'compress') {
      await invoke("compress_item", { 
        src: file.path, 
        dest, 
        method: ui.archiveMethod, 
        level: parseInt(ui.archiveLevel as any) 
      });
    } else {
      await invoke("extract_item", { src: file.path, dest });
    }
    loadDirectory(explorer.currentPath, false);
  } catch(e) { 
    explorer.errorMessage = String(e); 
  }
  
  ui.isArchiving = false;
  ui.showArchiveModal = false;
}

export async function handleUndo() {
  const action = explorer.undoStack.pop();
  if (!action) return;
  settings.saveSettings();

  try {
    if (action.type === 'move') {
      await invoke("move_item", { src: action.dest, dest: action.src });
    } else if (action.type === 'rename') {
      await invoke("rename_item", { oldPath: action.dest, newPath: action.src });
    }
    
    // Refresh panes to reflect the undo
    loadDirectory(explorer.currentPath, false, false, false);
    if (explorer.isTiledView) {
      const activeTab = explorer.tabs[explorer.activeTabIndex];
      if (activeTab?.secondaryPath) loadDirectory(activeTab.secondaryPath, false, false, true);
    }
  } catch (error) {
    explorer.errorMessage = `Undo failed: ${error}`;
  }
}

export interface Command {
  label: string;
  description: string;
  action: () => void;
  category: 'File' | 'View' | 'Navigation';
  condition?: () => boolean;
}

export function getAvailableCommands(): Command[] {
  const hasSelection = explorer.selectedFiles.length > 0;
  const isZip = hasSelection && explorer.selectedFiles[0].name.toLowerCase().endsWith('.zip');

  const commands: Command[] = [
    { label: 'New Folder', description: 'Create a new directory', category: 'File', action: () => openCreateModal(new MouseEvent('click'), 'folder') },
    { label: 'New File', description: 'Create a new text document', category: 'File', action: () => openCreateModal(new MouseEvent('click'), 'file') },
    { label: 'Toggle Sidebar', description: 'Show or hide the sidebar', category: 'View', action: () => { settings.showSidebar = !settings.showSidebar; settings.saveSettings(); } },
    { label: 'Toggle Dual Pane', description: 'Enable or disable tiled view', category: 'View', action: () => { explorer.isTiledView = !explorer.isTiledView; } },
    { label: 'Preferences', description: 'Open application settings', category: 'View', action: () => { ui.showPrefsModal = true; } },
    // Navigation (Jump to Folder)
    ...settings.pinnedFolders.map(folder => ({
      label: `Jump to: ${folder.name}`,
      description: folder.path,
      category: 'Navigation' as const,
      action: () => loadDirectory(folder.path, true)
    }))
  ];

  // Contextual commands
  if (hasSelection) {
    commands.push(
      { label: 'Copy', description: 'Copy selection to clipboard', category: 'File', action: handleCopy },
      { label: 'Delete', description: 'Move selection to trash', category: 'File', action: () => deleteSelected(false) },
      { label: 'Properties', description: 'Show item details', category: 'File', action: openProperties }
    );
  }

  if (isZip) {
    commands.push(
      { label: 'Extract Here', description: 'Unzip files in current folder', category: 'File', action: handleExtractHere }
    );
  }

  return commands;
}

export async function loadMorePreview() {
  if (!explorer.previewFilePath || explorer.isEditingPreview) return;

  try {
    const CHUNK_SIZE = 1024 * 1024; // 1MB
    const originalPath = explorer.previewFilePath;
    const nextChunk = await invoke<string>("read_text_file", { 
      path: explorer.previewFilePath, 
      offset: explorer.previewOffset, 
      length: CHUNK_SIZE
    });
    
    // Prevent race condition if user closed or switched files during read
    if (explorer.previewFilePath !== originalPath) return;

    if (nextChunk && nextChunk.length > 0) {
      explorer.previewTextContent = (explorer.previewTextContent || "") + nextChunk;
      explorer.previewOffset += CHUNK_SIZE;
    }
  } catch (error) {
    console.error("Failed to load more preview:", error);
  }
}

export function handleDropContextMenu(event: MouseEvent, destPath: string, isSecondary: boolean = false) {
  event.preventDefault();
  event.stopPropagation();
  ui.menuType = 'drop';
  ui.dropDestPath = destPath;
  ui.contextMenuPane = isSecondary ? 'secondary' : 'primary';
  ui.contextMenuX = Math.min(event.clientX, window.innerWidth - 160);
  ui.contextMenuY = Math.min(event.clientY, window.innerHeight - 140);
  ui.contextMenuVisible = true;
}

export async function executeDropAction(action: 'copy' | 'move') {
  if (!ui.dropDestPath) return;
  const targetPath = ui.dropDestPath;
  const isCopy = action === 'copy';
  
  const selection = explorer.focusedPane === 'secondary' ? explorer.secondarySelectedFiles : explorer.selectedFiles;
  for (const file of selection) {
    const separator = targetPath.endsWith('\\') || targetPath.endsWith('/') ? '' : '\\';
    const dest = `${targetPath}${separator}${file.name}`;
    
    const normalizedDest = dest.toLowerCase().replace(/\//g, '\\');
    const normalizedSrc = file.path.toLowerCase().replace(/\//g, '\\');

    if (normalizedDest.startsWith(normalizedSrc) || normalizedSrc === normalizedDest) {
      explorer.errorMessage = `Cannot move ${file.name} into itself.`;
      continue;
    }
    try {
      if (isCopy) {
        await invoke("copy_item", { src: file.path, dest });
      } else {
        await invoke("move_item", { src: file.path, dest });
        explorer.undoStack.push({ type: 'move', src: file.path, dest });
        settings.saveSettings();
      }
    } catch(err) {
      explorer.errorMessage = `Move failed: ${err}`;
    }
  }
  
  loadDirectory(explorer.currentPath, false, false, false);
  if (explorer.isTiledView) {
    const activeTab = explorer.tabs[explorer.activeTabIndex];
    if (activeTab?.secondaryPath) loadDirectory(activeTab.secondaryPath, false, false, true);
  }
}