<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { Terminal } from 'xterm';
  import type { FitAddon } from '@xterm/addon-fit';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { settings } from '$lib/settings.svelte';

  let { id, cwd } = $props<{ id: string, cwd: string }>();

  let terminalContainer: HTMLElement;
  let term: Terminal;
  let fitAddon: FitAddon;
  let unlistenPromise: Promise<() => void>;
let lastCwd = "";

onMount(() => {
  let resizeObserver: ResizeObserver | undefined;
  let isDestroyed = false;

  (async () => {
    const { Terminal: XTerm } = await import('xterm');
    const { FitAddon: XFitAddon } = await import('@xterm/addon-fit');
    // @ts-ignore 
    await import('xterm/css/xterm.css');

    // Don't initialize if the user closed the terminal during the network request
    if (!terminalContainer || isDestroyed) return;

    term = new XTerm({ fontFamily: 'monospace' });
    fitAddon = new XFitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalContainer);
    fitAddon.fit();

    const shellCommand = settings.integratedTerminalType === 'custom' ? settings.customIntegratedTerminal : settings.integratedTerminalType;
    invoke('spawn_pty', { id, rows: term.rows, cols: term.cols, cwd, shell: shellCommand }).then(() => {
      // If unmounted while spawning, immediately kill and abort listener setup
      if (isDestroyed) {
        invoke('kill_pty', { id });
        return;
      }
      unlistenPromise = listen(`pty-out-${id}`, (e: any) => {
        if (term && term.element) term.write(e.payload);
      });
      unlistenPromise.then(unlisten => {
        if (isDestroyed) unlisten();
      });
    });

    term.onData((data) => {
      invoke('write_pty', { id, data });
    });

    resizeObserver = new ResizeObserver(() => {
      if (fitAddon && term) {
        fitAddon.fit();
        invoke('resize_pty', { id, rows: term.rows, cols: term.cols });
      }
    });

    resizeObserver.observe(terminalContainer);
  })();

  return () => {
    isDestroyed = true;
    if (resizeObserver) resizeObserver.disconnect();
    if (term) term.dispose();
    invoke('kill_pty', { id });
    if (unlistenPromise) unlistenPromise.then(unlisten => unlisten());
  };
});

  $effect(() => {
    // Dynamically inject colors into the terminal canvas based on the current theme
    if (term) {
      const isLight = settings.theme === 'light' || (settings.theme === 'auto' && window.matchMedia('(prefers-color-scheme: light)').matches);
      term.options.theme = {
        background: isLight ? '#e9ecef' : '#1e1e1e',
        foreground: isLight ? '#333333' : '#ffffff',
        cursor: isLight ? '#333333' : '#ffffff',
        selectionBackground: isLight ? 'rgba(0, 0, 0, 0.2)' : 'rgba(255, 255, 255, 0.3)',
        black: '#000000',
        red: '#cd3131',
        green: isLight ? '#00bc00' : '#0dbc79',
        yellow: isLight ? '#949800' : '#e5e510',
        blue: '#2472c8',
        magenta: isLight ? '#bc05bc' : '#bc3fbc',
        cyan: '#11a8cd',
        white: isLight ? '#555555' : '#e5e5e5',
        brightBlack: '#666666',
        brightRed: isLight ? '#cd3131' : '#f14c4c',
        brightGreen: isLight ? '#14ce14' : '#23d18b',
        brightYellow: isLight ? '#b5ba00' : '#f5f543',
        brightBlue: '#3b8eea',
        brightMagenta: isLight ? '#bc05bc' : '#d670d6',
        brightCyan: '#29b8db',
        brightWhite: isLight ? '#a5a5a5' : '#ffffff'
      };
    }
  });

  let terminalHeight = $state(250);
  let isResizing = $state(false);

  function startResize(e: MouseEvent) {
    isResizing = true;
    document.addEventListener('mousemove', handleResize);
    document.addEventListener('mouseup', stopResize);
  }

  function handleResize(e: MouseEvent) {
    if (!isResizing) return;
    // Calculate new height relative to bottom of the viewport
    const newHeight = window.innerHeight - e.clientY - 16;
    terminalHeight = Math.max(100, Math.min(newHeight, window.innerHeight * 0.8));
  }

  function stopResize() {
    isResizing = false;
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
    
    if (fitAddon && term) {
      fitAddon.fit();
      invoke('resize_pty', { id, rows: term.rows, cols: term.cols });
    }
  }

  onDestroy(() => {
    document.removeEventListener('mousemove', handleResize);
    document.removeEventListener('mouseup', stopResize);
  });

  $effect(() => {
    // Auto CD by sending the command directly to the terminal input
    if (lastCwd === "") {
      lastCwd = cwd; 
    } else if (settings.terminalAutoCd && cwd !== lastCwd) {
      lastCwd = cwd;
      invoke('write_pty', { id, data: `cd /d "${cwd}"\r` });
    }
  });
</script>

<div class="terminal-wrapper" style="height: {terminalHeight}px;">
    <div class="terminal-resizer" onmousedown={startResize} role="separator" tabindex="-1"></div>
  <div bind:this={terminalContainer} class="terminal-container"></div>
</div>

<style>
  .terminal-wrapper {
    position: relative;
    width: 100%;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
  }
  .terminal-resizer {
    height: 6px;
    cursor: ns-resize;
    position: absolute;
    top: -3px;
    left: 0;
    right: 0;
    z-index: 10;
    background: transparent;
  }
  .terminal-container {
    flex-grow: 1;
    min-height: 0;
    width: 100%;
    border-top: 1px solid var(--border-color);
    background: var(--bg-solid);
    overflow: hidden;
    padding: 4px;
    box-sizing: border-box;
  }
</style>