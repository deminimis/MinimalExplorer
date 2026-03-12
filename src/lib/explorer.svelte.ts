export interface UndoAction {
  type: 'move' | 'rename';
  src: string;
  dest: string;
}

export interface FileItem {
  name: string;
  is_dir: boolean;
  path: string;
  size: number;
  modified: number;
  is_hidden: boolean;
  snippet?: string | null;
}

export interface Tab {
  id: string;
  path: string;
  historyStack: string[];
  forwardStack: string[];
  isTiledView: boolean;
  previewImagePath: string | null;
  secondaryPath?: string;
  secondaryHistoryStack?: string[];
  secondaryForwardStack?: string[];
}

export class ExplorerState {
  // Core State
  tabs = $state<Tab[]>([{
    id: crypto.randomUUID(),
    path: "C:\\",
    secondaryPath: "C:\\",
    historyStack: [],
    forwardStack: [],
    secondaryHistoryStack: [],
    secondaryForwardStack: [],
    isTiledView: false,
    previewImagePath: null
  }]);
  activeTabIndex = $state(0);
  files = $state.raw<FileItem[]>([]);
  secondaryFiles = $state.raw<FileItem[]>([]);
  filterQuery = $state("");
  secondaryFilterQuery = $state("");
  errorMessage = $state("");
  checksums = $state<Record<string, string>>({});
  #worker: Worker | null = null;
  #workerPromise: Promise<Worker> | null = null;

  async calculateChecksum(path: string) {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const checksum = await invoke<string>("calculate_file_checksum", { path });
      this.checksums[path] = checksum;
    } catch (error) {
      console.error("Checksum failed:", error);
    }
  }

  // Selection & Clipboard State
  selectedFiles = $state<FileItem[]>([]);
  secondarySelectedFiles = $state<FileItem[]>([]);
  lastSelectedIndex = $state(-1);
  clipboardFiles = $state<FileItem[]>([]);
  clipboardAction = $state<'copy' | 'cut' | null>(null);
  undoStack = $state<UndoAction[]>([]);
  previewTextContent = $state<string | null>(null);
  previewOffset = 0;
  previewFilePath = $state<string | null>(null);
  previewPdfPath = $state<string | null>(null);
  isEditingPreview = $state(false);
  focusedPane = $state<'primary' | 'secondary'>('primary');
  
  // Search State
  searchResults = $state.raw<FileItem[]>([]);
  isSearching = $state(false);
  
  get currentPath() {
    return this.tabs[this.activeTabIndex]?.path ?? "C:\\";
  }

  get historyStack() {
    return this.tabs[this.activeTabIndex]?.historyStack ?? [];
  }
  
  set historyStack(value: string[]) {
    if (this.tabs[this.activeTabIndex]) {
      this.tabs[this.activeTabIndex].historyStack = value;
    }
  }

  get forwardStack() {
    return this.tabs[this.activeTabIndex]?.forwardStack ?? [];
  }
  
  set forwardStack(value: string[]) {
    if (this.tabs[this.activeTabIndex]) {
      this.tabs[this.activeTabIndex].forwardStack = value;
    }
  }
  
  set currentPath(value: string) {
    if (this.tabs[this.activeTabIndex]) {
      this.tabs[this.activeTabIndex].path = value;
    }
  }

  get isTiledView() {
    return this.tabs[this.activeTabIndex]?.isTiledView ?? false;
  }

  set isTiledView(value: boolean) {
    if (this.tabs[this.activeTabIndex]) {
      this.tabs[this.activeTabIndex].isTiledView = value;
    }
  }

  get previewImagePath() {
    return this.tabs[this.activeTabIndex]?.previewImagePath ?? null;
  }

  set previewImagePath(value: string | null) {
    if (this.tabs[this.activeTabIndex]) {
      this.tabs[this.activeTabIndex].previewImagePath = value;
    }
  }

  // Extracted Methods
  switchTab(index: number) {
    if (this.activeTabIndex === index) return;
    this.activeTabIndex = index;
  }

  addNewTab(targetPath?: string, background: boolean = false) {
    const currentTab = this.tabs[this.activeTabIndex];
    const newPath = targetPath || (currentTab ? currentTab.path : "C:\\");

    this.tabs.push({
      id: crypto.randomUUID(),
      path: newPath,
      secondaryPath: "C:\\", 
      historyStack: [newPath],
      forwardStack: [],
      secondaryHistoryStack: ["C:\\"],
      secondaryForwardStack: [],
      isTiledView: false, 
      previewImagePath: null
    });
    
    if (!background) {
      this.activeTabIndex = this.tabs.length - 1;
    }
  }

  closeTab(index: number) {
    if (this.tabs.length === 1) return; 
    this.tabs = this.tabs.filter((_, i) => i !== index);
    
    if (this.activeTabIndex >= this.tabs.length) {
      this.activeTabIndex = this.tabs.length - 1;
    } else if (this.activeTabIndex > index) {
      this.activeTabIndex--;
    }
  }
}


export const explorer = new ExplorerState();