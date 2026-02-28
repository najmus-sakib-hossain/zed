
# dx-check VS Code Extension Guide

This guide covers the VS Code extension integration for dx-check.

## Installation

The dx-check LSP is integrated into the `vscode-dx` extension.

### Prerequisites

- Build dx-check with LSP support:
```bash
cargo build -p dx-check --release --features lsp ```
- Ensure the binary is in your PATH or configure the path in settings.


### Extension Setup


- Install the vscode-dx extension
- The extension will automatically detect and start the dx-check LSP server


## Configuration



### VS Code Settings


Add to your `settings.json`:
```json
{"dx.check.enable":true,"dx.check.executablePath":"","dx.check.lintOnSave":true,"dx.check.lintOnType":true,"dx.check.autoFix":false,"dx.check.configPath":""}
```


### Settings Reference


+-------------------+---------+---------+----------------+
| Setting           | Type    | Default | Description    |
+===================+=========+=========+================+
| `dx.check.enable` | boolean | `true`  | Enable/disable |
+-------------------+---------+---------+----------------+


## Features



### Real-time Diagnostics


Diagnostics appear in the Problems panel and as inline squiggles: -Errors: Red squiggles -Warnings: Yellow squiggles


### Quick Fixes


For fixable rules, click the lightbulb icon or press `Ctrl+.` to see available fixes: -Fix: Apply the fix for this diagnostic -Fix all: Apply all fixes of this type in the file


### Hover Documentation


Hover over a diagnostic to see: -Rule name and description -Link to documentation -Whether the rule is fixable


### Commands


+---------+-------------+
| Command | Description |
+=========+=============+
| `DX:    | Lint        |
+---------+-------------+


### Status Bar


The status bar shows: -dx-check status (running/stopped) -Diagnostic counts for the current file Click the status bar item to see more options.


## Supported Languages


The extension activates for these file types: -JavaScript (`.js`, `.mjs`, `.cjs`) -TypeScript (`.ts`, `.mts`, `.cts`) -JSX (`.jsx`) -TSX (`.tsx`) -JSON (`.json`) -Markdown (`.md`)


## Troubleshooting



### LSP Server Not Starting


- Check that dx-check is built with LSP support:
```bash
dx-check lsp --help ```
- Verify the executable path in settings
- Check the Output panel (`View > Output > DX Check`)

### Diagnostics Not Appearing

- Ensure `dx.check.enable` is `true`
- Check that the file type is supported
- Verify your `dx.toml` configuration

### Performance Issues

If linting is slow: -Disable `dx.check.lintOnType` and use `dx.check.lintOnSave` only -Add exclusion patterns in `dx.toml`:```toml [paths] exclude = ["node_modules/", "dist/", "*.min.js"]
```


### Server Crashes


If the server crashes repeatedly:
- Check the Output panel for error messages
- Try restarting: `DX: Restart Server`
- Report issues with reproduction steps


## Configuration Hot-Reload


The extension watches for changes to `dx.toml` and automatically reloads configuration. No restart required.


## Workspace Trust


The extension respects VS Code's Workspace Trust feature. In untrusted workspaces:
- LSP server will not start automatically
- Manual commands are disabled


## Multi-root Workspaces


In multi-root workspaces, each workspace folder can have its own `dx.toml`. The extension will use the appropriate configuration for each file.


## Integration with Other Extensions



### Prettier


dx-check formatting can coexist with Prettier. Configure your default formatter per language:
```
{ "[javascript]": { "editor.defaultFormatter": "dx.vscode-dx" }, "[json]": { "editor.defaultFormatter": "esbenp.prettier-vscode" } }
```


### ESLint


If migrating from ESLint, you can run both temporarily:
```
{ "eslint.enable": true, "dx.check.enable": true }
```
Once satisfied with dx-check, disable ESLint:
```
{ "eslint.enable": false }
```


## Keyboard Shortcuts


Default shortcuts (customizable in Keyboard Shortcuts):
+--------+----------+
| Action | Shortcut |
+========+==========+
| Quick  | Fix      |
+--------+----------+


## Logs and Debugging


Enable verbose logging:
```
{ "dx.check.trace.server": "verbose" }
```
View logs in Output panel: `View > Output > DX Check`
```
