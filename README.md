# Minimal Explorer

Minimal Explorer is a fast, highly performant, and minimalist Windows Explorer replacement built for Windows 11. Engineered with a lightweight Rust backend, it prioritizes speed, keyboard-centric workflows, and a sleek aesthetic. 

Because Windows Explorer is so integrated into Windows 10/11, it is not practically possible to completely replace it with this file explorer. But with a little manual labor, this file explorer could easily be converted for Linux (mostly just removed Windows API). 

## Download Here:
____

## Features

- **Fast Architecture:** BRust backend with native-like performance and minimal RAM usage.
- **Dual-Pane & Tabbed Interface:** Manage files side-by-side with horizontal/vertical split views, or open multiple directories in background tabs.
- **Integrated Terminal:** Built-in drop-down terminal (PowerShell, CMD, or custom/WSL) that automatically syncs with your current working directory.
- **Advanced Deep Search:** Quickly search for files by name, or use advanced filters to search by file contents, extension, size, and modification date.
- **Native Archive Management:** Compress to `.zip` or extract archives directly within the UI without third-party software.
- **Rich File Previews:** Instantly preview images, PDFs, raw text, and Markdown files right inside the explorer.
- **Quick Access Sidebar:** Pin favorite folders for quick access, alongside automatic detection and mounting of WSL (Linux) distributions.
- **Command Palette & Custom Hotkeys:** A VS Code-style command palette (`Ctrl+Shift+P` or `>`) and fully customizable keyboard shortcuts for power users.
- **Modern UI:** Sleek, custom-drawn titlebars, window controls, and right-click context menus with beautiful Acrylic-style blur effects.


## Configuration

Access the Preferences menu to adjust the application to your liking:

- Theme: Force Dark Mode, Light Mode, or respect System Auto.
- Layout: Switch between vertical and horizontal split panes.
- Terminal Shell: Choose between PowerShell, Command Prompt, or specify a custom path (e.g., wsl).
- Thumbnails: Toggle image thumbnails on or off for better performance.
- Hotkeys: almost all are customizable 





## To build

### Prerequisites

Before you begin, ensure you have the following installed on your Windows 11 machine:
- [Node.js](https://nodejs.org/) (v18 or higher)
- [Rust](https://www.rust-lang.org/tools/install) (Ensure C++ Build Tools are installed via Visual Studio Installer)
- [Git](https://git-scm.com/)

### Installation

1. **Clone the repository:**
2. **Install Dependencies**: npm install
3. **Run in dev mode:** npm run tauri dev
