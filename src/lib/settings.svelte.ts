import { load, type Store } from "@tauri-apps/plugin-store";
import { explorer } from "./explorer.svelte";

export class SettingsState {
  store: Store | null = null;

  // Global Settings
  showHiddenFiles = $state(false);
  showGitBadges = $state(true);
  enableThumbnails = $state(false);
  sessionViewOverrides = new Map<string, 'grid' | 'list'>();
  splitDirection = $state<'vertical' | 'horizontal'>('vertical');
  enablePreviewPane = $state(true);
  theme = $state<'dark' | 'light' | 'auto'>('dark');
  showPrimaryTerminal = $state(false);
  showSecondaryTerminal = $state(false);
  terminalAutoCd = $state(true);
  integratedTerminalType = $state<'powershell' | 'cmd' | 'custom'>('cmd');
  recentPaths = $state<string[]>([]);
  externalTerminalType = $state<'powershell' | 'cmd' | 'custom'>('cmd');
  customIntegratedTerminal = $state('wsl');
  customExternalTerminal = $state('wt.exe');
  
  autoViewMode = $state(true);
  autoCheckUpdates = $state(true);
  
  // Sidebar Settings
  showSidebar = $state(true);
  sidebarHoverReveal = $state(false);
  sidebarWidth = $state(200);
  
  // Sorting State
  sortBy = $state<'name' | 'date' | 'size' | 'type'>('name');
  sortAscending = $state(true);

  // Active view state per panel
  primaryViewMode = $state<'grid' | 'list'>('grid');
  secondaryViewMode = $state<'grid' | 'list'>('grid');
  currentFolderIconSize = $state(120);
  listColumnWidths = $state({ name: 400, date: 180, size: 120 });

  // Pinned Folders
  pinnedFolders = $state<{name: string, path: string, is_dir?: boolean}[]>([]);

  // Configurable Hotkeys
  hotkeys = $state({
    // File Operations
    copy: "Ctrl+C", cut: "Ctrl+X", paste: "Ctrl+V", 
    rename: "F2", delete: "Delete", permDelete: "Shift+Delete", 
    newFolder: "Ctrl+Shift+N", newFile: "Ctrl+N", 
    copyPath: "Ctrl+Shift+C", cutPath: "Ctrl+Shift+X",
    properties: "Alt+Enter", calcSize: "Ctrl+Shift+D",

    // Selection
    selectAll: "Ctrl+A", invertSelection: "Ctrl+I", selectSameType: "Ctrl+Shift+S",

    // Navigation & Tabs
    goBack: "Alt+ArrowLeft", goForward: "Alt+ArrowRight", goUp: "Alt+ArrowUp",
    newTab: "Ctrl+T", closeTab: "Ctrl+W", nextTab: "Ctrl+Tab", prevTab: "Ctrl+Shift+Tab", 
    switchPane: "Tab", refresh: "F5",

    // Search & Focus
    focusAddress: "Alt+D", focusFilter: "Ctrl+F", 
    openSearch: "Ctrl+Shift+F", contentSearch: "Ctrl+Alt+F",

    // View & UI Toggles
    toggleView: "Ctrl+M", sortType: "Alt+S", togglePreview: "Ctrl+U",
    toggleSidebar: "Ctrl+B", toggleTerminal: "Ctrl+`", 
    fullScreen: "F11", zoomIn: "Ctrl+=", zoomOut: "Ctrl+-",

    // Archives
    compress: "Ctrl+Shift+Z", extractHere: "Ctrl+E", extractFolder: "Ctrl+Shift+E",

    // Misc
    undo: "Ctrl+Z"
  });

  async init() {
    this.store = await load("settings.json", { autoSave: false } as any);

    const savedPath = await this.store.get<string>("currentPath");
    if (savedPath) explorer.currentPath = savedPath;

    const savedShowHidden = await this.store.get<boolean>("showHiddenFiles");
    if (savedShowHidden !== undefined && savedShowHidden !== null) this.showHiddenFiles = savedShowHidden;

    const savedShowGitBadges = await this.store.get<boolean>("showGitBadges");
    if (savedShowGitBadges !== undefined && savedShowGitBadges !== null) this.showGitBadges = savedShowGitBadges;

    const savedThumbnails = await this.store.get<boolean>("enableThumbnails");
    if (savedThumbnails !== undefined && savedThumbnails !== null) this.enableThumbnails = savedThumbnails;

    const savedSplit = await this.store.get<string>("splitDirection");
    if (savedSplit) this.splitDirection = savedSplit as 'vertical' | 'horizontal';

    const savedShowPrimTerm = await this.store.get<boolean>("showPrimaryTerminal");
    if (savedShowPrimTerm !== null && savedShowPrimTerm !== undefined) this.showPrimaryTerminal = savedShowPrimTerm;

    const savedShowSecTerm = await this.store.get<boolean>("showSecondaryTerminal");
    if (savedShowSecTerm !== null && savedShowSecTerm !== undefined) this.showSecondaryTerminal = savedShowSecTerm;

    const savedAutoCd = await this.store.get<boolean>("terminalAutoCd");
    if (savedAutoCd !== null && savedAutoCd !== undefined) this.terminalAutoCd = savedAutoCd;

    const savedIntTerm = await this.store.get<string>("integratedTerminalType");
    if (savedIntTerm) this.integratedTerminalType = savedIntTerm as 'powershell' | 'cmd';

    const savedExtTerm = await this.store.get<string>("externalTerminalType");
    if (savedExtTerm) this.externalTerminalType = savedExtTerm as 'powershell' | 'cmd' | 'custom';

    const savedCustomInt = await this.store.get<string>("customIntegratedTerminal");
    if (savedCustomInt) this.customIntegratedTerminal = savedCustomInt;

    const savedCustomExt = await this.store.get<string>("customExternalTerminal");
    if (savedCustomExt) this.customExternalTerminal = savedCustomExt;

    const savedAutoView = await this.store.get<boolean>("autoViewMode");
    if (savedAutoView !== undefined && savedAutoView !== null) this.autoViewMode = savedAutoView;

    const savedAutoUpdate = await this.store.get<boolean>("autoCheckUpdates");
    if (savedAutoUpdate !== undefined && savedAutoUpdate !== null) this.autoCheckUpdates = savedAutoUpdate;

    const savedPrimaryView = await this.store.get<string>("primaryViewMode");
    if (savedPrimaryView) this.primaryViewMode = savedPrimaryView as 'grid' | 'list';

    const savedSecondaryView = await this.store.get<string>("secondaryViewMode");
    if (savedSecondaryView) this.secondaryViewMode = savedSecondaryView as 'grid' | 'list';

    const savedIconSize = await this.store.get<number>("currentFolderIconSize");
    if (savedIconSize) this.currentFolderIconSize = savedIconSize;

    const savedColWidths = await this.store.get<any>("listColumnWidths");
    if (savedColWidths) this.listColumnWidths = { ...this.listColumnWidths, ...savedColWidths };

    const savedPinned = await this.store.get<{name: string, path: string}[]>("pinnedFolders");
    if (savedPinned) this.pinnedFolders = savedPinned;

    const savedUndoStack = await this.store.get<any[]>("undoStack");
    if (savedUndoStack) explorer.undoStack = savedUndoStack;

    const savedHotkeys = await this.store.get<any>("hotkeys");
    if (savedHotkeys) this.hotkeys = { ...this.hotkeys, ...savedHotkeys };
    
    const savedRecent = await this.store.get<string[]>("recentPaths");
    if (savedRecent) this.recentPaths = savedRecent;

    const savedTheme = await this.store.get<string>("theme") || 'dark';
    this.theme = savedTheme as 'dark' | 'light' | 'auto';

    
    const savedShowSidebar = await this.store.get<boolean>("showSidebar");
    if (savedShowSidebar !== undefined && savedShowSidebar !== null) this.showSidebar = savedShowSidebar;

    const savedSidebarHoverReveal = await this.store.get<boolean>("sidebarHoverReveal");
    if (savedSidebarHoverReveal !== undefined && savedSidebarHoverReveal !== null) this.sidebarHoverReveal = savedSidebarHoverReveal;

    const savedSidebarWidth = await this.store.get<number>("sidebarWidth");
    if (savedSidebarWidth) this.sidebarWidth = savedSidebarWidth;

    this.applyTheme();

    // Listen for OS theme changes while running
    window.matchMedia('(prefers-color-scheme: light)').addEventListener('change', () => {
      if (this.theme === 'auto') this.applyTheme();
    });
  }

  applyTheme() {
    import('@tauri-apps/api/core').then(({ invoke }) => {
      if (this.theme === 'auto') {
        const prefersLight = window.matchMedia('(prefers-color-scheme: light)').matches;
        document.documentElement.setAttribute('data-theme', prefersLight ? 'light' : 'dark');
        invoke('set_theme', { theme: 'auto' });
      } else {
        document.documentElement.setAttribute('data-theme', this.theme);
        invoke('set_theme', { theme: this.theme });
      }
    });
  }

  async saveSettings() {
    if (!this.store) return;
    await this.store.set("currentPath", explorer.currentPath);
    
    await this.store.set("autoViewMode", this.autoViewMode);
    await this.store.set("autoCheckUpdates", this.autoCheckUpdates);
    await this.store.set("sortBy", this.sortBy);
    await this.store.set("sortAscending", this.sortAscending);
    await this.store.set("primaryViewMode", this.primaryViewMode);
    await this.store.set("secondaryViewMode", this.secondaryViewMode);
    await this.store.set("currentFolderIconSize", this.currentFolderIconSize);
    await this.store.set("listColumnWidths", $state.snapshot(this.listColumnWidths));

    await this.store.set("showHiddenFiles", this.showHiddenFiles);
    await this.store.set("showGitBadges", this.showGitBadges);
    await this.store.set("enableThumbnails", this.enableThumbnails);
    await this.store.set("splitDirection", this.splitDirection);
    await this.store.set("enablePreviewPane", this.enablePreviewPane);
    await this.store.set("showPrimaryTerminal", this.showPrimaryTerminal);
    await this.store.set("showSecondaryTerminal", this.showSecondaryTerminal);
    await this.store.set("terminalAutoCd", this.terminalAutoCd);
    await this.store.set("integratedTerminalType", this.integratedTerminalType);
    await this.store.set("externalTerminalType", this.externalTerminalType);
    await this.store.set("customIntegratedTerminal", this.customIntegratedTerminal);
    await this.store.set("customExternalTerminal", this.customExternalTerminal);
    await this.store.set("theme", this.theme);
    
    await this.store.set("showSidebar", this.showSidebar);
    await this.store.set("sidebarHoverReveal", this.sidebarHoverReveal);
    await this.store.set("sidebarWidth", this.sidebarWidth);
    await this.store.set("pinnedFolders", $state.snapshot(this.pinnedFolders));
    await this.store.set("undoStack", $state.snapshot(explorer.undoStack));
    await this.store.set("hotkeys", $state.snapshot(this.hotkeys));
    await this.store.set("recentPaths", $state.snapshot(this.recentPaths));

    await this.store.save();
  }
}

export const settings = new SettingsState();