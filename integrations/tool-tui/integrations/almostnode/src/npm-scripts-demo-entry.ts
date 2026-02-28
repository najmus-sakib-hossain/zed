/**
 * npm Scripts Demo — Entry Point
 * Interactive terminal for running npm scripts and bash commands
 */

import { createContainer } from './index';

// DOM elements
const pkgEditor = document.getElementById('pkgEditor') as HTMLTextAreaElement;
const terminalOutput = document.getElementById('terminalOutput') as HTMLDivElement;
const terminalInput = document.getElementById('terminalInput') as HTMLInputElement;
const statusEl = document.getElementById('status') as HTMLSpanElement;

// State
const commandHistory: string[] = [];
let historyIndex = -1;
let isRunning = false;

// Create the container
const container = createContainer();

// Default server.js for "npm start"
container.vfs.writeFileSync('/server.js', `console.log('Server starting on port 3000...');
console.log('Ready to accept connections');
`);

// Write initial package.json
syncPackageJson();

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

function appendToTerminal(text: string, className: string = 'stdout') {
  const span = document.createElement('span');
  span.className = className;
  span.innerHTML = escapeHtml(text);
  if (!text.endsWith('\n')) span.innerHTML += '\n';
  terminalOutput.appendChild(span);
  terminalOutput.scrollTop = terminalOutput.scrollHeight;
}

function syncPackageJson() {
  try {
    // Validate JSON before writing
    JSON.parse(pkgEditor.value);
    container.vfs.writeFileSync('/package.json', pkgEditor.value);
  } catch {
    // Invalid JSON — skip sync, will error on npm run
  }
}

async function executeCommand(command: string) {
  if (!command.trim()) return;
  if (isRunning) return;

  isRunning = true;
  terminalInput.disabled = true;
  statusEl.textContent = 'Running...';

  // Add to history
  commandHistory.push(command);
  historyIndex = commandHistory.length;

  // Show the command
  appendToTerminal(`$ ${command}`, 'cmd');

  // Sync package.json from editor to VFS
  syncPackageJson();

  try {
    const result = await container.run(command);
    if (result.stdout) appendToTerminal(result.stdout, 'stdout');
    if (result.stderr) appendToTerminal(result.stderr, 'stderr');
    if (result.exitCode !== 0) {
      appendToTerminal(`exit code: ${result.exitCode}`, 'dim');
    }
  } catch (error) {
    appendToTerminal(`Error: ${error}`, 'stderr');
  }

  isRunning = false;
  terminalInput.disabled = false;
  terminalInput.focus();
  statusEl.textContent = 'Ready';
}

// Terminal input handling
terminalInput.addEventListener('keydown', (e) => {
  if (e.key === 'Enter') {
    const command = terminalInput.value.trim();
    terminalInput.value = '';
    executeCommand(command);
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    if (historyIndex > 0) {
      historyIndex--;
      terminalInput.value = commandHistory[historyIndex];
    }
  } else if (e.key === 'ArrowDown') {
    e.preventDefault();
    if (historyIndex < commandHistory.length - 1) {
      historyIndex++;
      terminalInput.value = commandHistory[historyIndex];
    } else {
      historyIndex = commandHistory.length;
      terminalInput.value = '';
    }
  }
});

// Show welcome message
appendToTerminal('almostnode npm scripts demo', 'info');
appendToTerminal('Type a command below, e.g. npm run build\n', 'dim');

// Focus the input
terminalInput.focus();
