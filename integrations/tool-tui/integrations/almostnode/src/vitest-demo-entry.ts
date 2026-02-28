/**
 * Vitest Testing Demo — Entry Point
 * Uses xterm.js for terminal emulation with native ANSI color support.
 * Supports vitest watch mode via streaming container.run() API.
 */

import { createContainer } from './index';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

// DOM elements
const editor = document.getElementById('editor') as HTMLTextAreaElement;
const terminalEl = document.getElementById('terminal') as HTMLDivElement;
const statusEl = document.getElementById('status') as HTMLSpanElement;
const statusDot = document.getElementById('statusDot') as HTMLDivElement;
const fileTabs = document.getElementById('fileTabs') as HTMLDivElement;
const watchModeCheckbox = document.getElementById('watchMode') as HTMLInputElement;
const saveBtn = document.getElementById('saveBtn') as HTMLButtonElement;

// File contents
const files: Record<string, string> = {
  'utils.js': `function capitalize(str) {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

function sum(a, b) {
  return a + b;
}

function contains(str, word) {
  return str.includes(word);
}

module.exports = { capitalize, sum, contains };
`,
  'utils.test.js': `const { describe, it, expect } = require('vitest');
const { capitalize, sum, contains } = require('./utils');

describe('capitalize', () => {
  it('capitalizes the first letter', () => {
    expect(capitalize('hello')).toBe('Hello');
  });

  it('handles empty string', () => {
    expect(capitalize('')).toBe('');
  });
});

describe('sum', () => {
  it('adds two numbers', () => {
    expect(sum(1, 2)).toBe(3);
  });

  it('handles negative numbers', () => {
    expect(sum(-1, 1)).toBe(0);
  });
});

describe('contains', () => {
  it('finds a word in a string', () => {
    expect(contains('hello world', 'hello')).toBe(true);
  });

  it('returns false when word is missing', () => {
    expect(contains('hello world', 'goodbye')).toBe(false);
  });
});
`,
  'package.json': JSON.stringify({
    name: 'my-app',
    version: '1.0.0',
    scripts: {
      test: 'vitest run',
    },
  }, null, 2),
};

let activeFile = 'utils.js';
let isRunning = false;

// Command history
const commandHistory: string[] = [];
let historyIndex = -1;
let currentLine = '';

// Watch mode state
let watchAbortController: AbortController | null = null;

// Create the container
const container = createContainer();

// Set up xterm.js terminal
const term = new Terminal({
  theme: {
    background: '#0c0c0c',
    foreground: '#c0c0c0',
    cursor: '#00ff88',
    cursorAccent: '#0c0c0c',
    selectionBackground: 'rgba(0, 255, 136, 0.15)',
    black: '#0c0c0c',
    red: '#e74c3c',
    green: '#00ff88',
    yellow: '#e8c872',
    blue: '#3498db',
    magenta: '#9b59b6',
    cyan: '#1abc9c',
    white: '#c0c0c0',
    brightBlack: '#666666',
    brightRed: '#ff6b6b',
    brightGreen: '#00ffaa',
    brightYellow: '#f0e68c',
    brightBlue: '#5dade2',
    brightMagenta: '#bb8fce',
    brightCyan: '#48c9b0',
    brightWhite: '#f0f0f0',
  },
  fontFamily: "'IBM Plex Mono', 'Menlo', monospace",
  fontSize: 13,
  cursorBlink: true,
  convertEol: false,
});

const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(terminalEl);

// Fit terminal after a frame to ensure container has dimensions
requestAnimationFrame(() => fitAddon.fit());
new ResizeObserver(() => fitAddon.fit()).observe(terminalEl);

// Expose for E2E tests
(window as any).__term = term;
(window as any).__container = container;
(window as any).__files = files;

function writeToTerminal(text: string) {
  term.write(text.replace(/\n/g, '\r\n'));
}

function showPrompt() {
  term.write('\x1b[32m$ \x1b[0m');
}

function setStatus(text: string, running: boolean = false) {
  statusEl.textContent = text;
  if (running) {
    statusDot.classList.add('running');
  } else {
    statusDot.classList.remove('running');
  }
}

function syncFilesToVFS() {
  files[activeFile] = editor.value;
  for (const [name, content] of Object.entries(files)) {
    container.vfs.writeFileSync(`/${name}`, content);
  }
}

function switchTab(fileName: string) {
  files[activeFile] = editor.value;
  activeFile = fileName;
  editor.value = files[fileName] || '';

  const tabs = fileTabs.querySelectorAll('.file-tab');
  tabs.forEach(tab => {
    if ((tab as HTMLElement).dataset.file === fileName) {
      tab.classList.add('active');
    } else {
      tab.classList.remove('active');
    }
  });
}

function saveFile() {
  files[activeFile] = editor.value;
  container.vfs.writeFileSync(`/${activeFile}`, editor.value);
  // In watch mode, restart vitest to pick up file changes.
  // Vitest caches modules internally (Vite's module graph), so we need a full
  // restart to re-read updated files from VFS.
  if (watchAbortController) {
    restartWatchMode();
  }
}

// Flag to indicate watch mode should restart after stopping
let pendingRestart = false;

async function startWatchMode() {
  if (isRunning) return;

  isRunning = true;
  setStatus('Watch mode', true);

  syncFilesToVFS();

  watchAbortController = new AbortController();

  writeToTerminal('\x1b[34m[watch] starting vitest in watch mode...\x1b[0m\n');

  try {
    await container.run('vitest --watch', {
      onStdout: (data: string) => writeToTerminal(data),
      onStderr: (data: string) => writeToTerminal(data),
      signal: watchAbortController.signal,
    });
  } catch {
    // Abort throws — expected
  }

  isRunning = false;
  watchAbortController = null;

  // If a restart is pending (file was saved), immediately start again
  if (pendingRestart) {
    pendingRestart = false;
    startWatchMode();
  } else {
    setStatus('Ready');
    showPrompt();
  }
}

function stopWatchMode() {
  pendingRestart = false;
  if (watchAbortController) {
    watchAbortController.abort();
  }
}

function restartWatchMode() {
  if (watchAbortController) {
    pendingRestart = true;
    watchAbortController.abort();
  }
}

async function executeCommand(command: string) {
  if (!command.trim()) return;
  if (isRunning) return;

  isRunning = true;
  setStatus('Running...', true);

  commandHistory.push(command);
  historyIndex = -1;

  syncFilesToVFS();

  try {
    const result = await container.run(command);
    if (result.stdout) writeToTerminal(result.stdout);
    if (result.stderr) writeToTerminal(result.stderr);
    if (result.exitCode !== 0 && !result.stdout.includes('Duration')) {
      writeToTerminal(`\x1b[2mexit code: ${result.exitCode}\x1b[0m\n`);
    }
  } catch (error) {
    writeToTerminal(`\x1b[31mError: ${error}\x1b[0m\n`);
  }

  isRunning = false;
  setStatus('Ready');
  showPrompt();
}

// xterm.js input handling
term.onData((data) => {
  if (isRunning) return;

  switch (data) {
    case '\r': { // Enter
      term.write('\r\n');
      const command = currentLine.trim();
      currentLine = '';
      historyIndex = -1;
      if (command) {
        executeCommand(command);
      } else {
        showPrompt();
      }
      break;
    }

    case '\x7f': // Backspace
      if (currentLine.length > 0) {
        currentLine = currentLine.slice(0, -1);
        term.write('\b \b');
      }
      break;

    case '\x1b[A': // Up arrow
      if (commandHistory.length > 0) {
        if (historyIndex === -1) historyIndex = commandHistory.length;
        if (historyIndex > 0) {
          historyIndex--;
          term.write('\r\x1b[32m$ \x1b[0m\x1b[K');
          currentLine = commandHistory[historyIndex];
          term.write(currentLine);
        }
      }
      break;

    case '\x1b[B': // Down arrow
      if (historyIndex !== -1) {
        if (historyIndex < commandHistory.length - 1) {
          historyIndex++;
          term.write('\r\x1b[32m$ \x1b[0m\x1b[K');
          currentLine = commandHistory[historyIndex];
          term.write(currentLine);
        } else {
          historyIndex = -1;
          term.write('\r\x1b[32m$ \x1b[0m\x1b[K');
          currentLine = '';
        }
      }
      break;

    default:
      // Printable characters
      if (data >= ' ' && data.length === 1) {
        currentLine += data;
        term.write(data);
      }
      break;
  }
});

// Tab click handler
fileTabs.addEventListener('click', (e) => {
  const tab = (e.target as HTMLElement).closest('.file-tab') as HTMLElement;
  if (tab && tab.dataset.file) {
    switchTab(tab.dataset.file);
  }
});

// Watch mode checkbox handler
watchModeCheckbox.addEventListener('change', () => {
  if (watchModeCheckbox.checked) {
    startWatchMode();
  } else {
    stopWatchMode();
  }
});

// Save button
saveBtn.addEventListener('click', () => saveFile());

// Cmd+S / Ctrl+S saves the current file
document.addEventListener('keydown', (e) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 's') {
    e.preventDefault();
    saveFile();
  }
});

// Initialize
async function init() {
  for (const [name, content] of Object.entries(files)) {
    container.vfs.writeFileSync(`/${name}`, content);
  }

  editor.value = files[activeFile];

  writeToTerminal('\x1b[34malmostnode vitest testing demo\x1b[0m\n');
  writeToTerminal('\x1b[2mInstalling vitest...\x1b[0m\n');
  setStatus('Installing vitest...', true);

  try {
    await container.npm.install('vitest', {
      onProgress: (msg: string) => {
        writeToTerminal(`\x1b[2m${msg}\x1b[0m\n`);
      },
    });

    writeToTerminal('\x1b[34mvitest installed successfully!\x1b[0m\n\n');
    writeToTerminal('\x1b[2mType npm run test to run tests, or enable watch mode.\x1b[0m\n');
    setStatus('Ready');
  } catch (error) {
    writeToTerminal(`\x1b[31mFailed to install vitest: ${error}\x1b[0m\n`);
    setStatus('Install failed');
  }

  showPrompt();
  term.focus();
}

init();
