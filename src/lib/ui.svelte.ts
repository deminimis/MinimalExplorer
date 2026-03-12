export class UIState {
  // Modals
  showCreateModal = $state(false);
  createModalType = $state<'file' | 'folder'>('folder');
  createInputName = $state("");
  createModalX = $state(0);
  createModalY = $state(0);
  
  // Archive Modal State
  showArchiveModal = $state(false);
  archiveMode = $state<'compress' | 'extract'>('compress');
  archiveInputName = $state("");
  archiveMethod = $state<'deflated' | 'stored'>('deflated');
  archiveLevel = $state(6);
  isArchiving = $state(false);
  shiftKeyPressed = $state(false);

  showPrefsModal = $state(false);
  showHotkeysModal = $state(false);
  showPropertiesModal = $state(false);
  propertiesData = $state<any>(null);

  // Focus Triggers
  triggerAddressFocus = $state(0);
  triggerFilterFocus = $state(0);
  
  // Search Modal
  showSearchModal = $state(false);
  showCommandPalette = $state(false);
  paletteInput = $state("");
  paletteSelectedIndex = $state(0); 
  searchQuery = $state("");

  // Context Menu
  contextMenuVisible = $state(false);
  contextMenuX = $state(0);
  contextMenuY = $state(0);
  menuType = $state<'item' | 'background' | 'sidebar' | 'drop'>('background');
  contextMenuPane = $state<'primary' | 'secondary'>('primary');
  contextMenuSidebarPath = $state<string | null>(null);
  dropDestPath = $state<string | null>(null);

  // Inline Rename State
  inlineRenamePath = $state<string | null>(null);
  inlineRenameValue = $state("");

  // Updater State
  updateAvailable = $state(false);
  updateVersion = $state("");
  updateBody = $state("");
  isUpdating = $state(false);
  updateProgress = $state(0);
  updateTotal = $state(0);

  closeContextMenu() {
    this.contextMenuVisible = false;
  }
}

export const ui = new UIState();