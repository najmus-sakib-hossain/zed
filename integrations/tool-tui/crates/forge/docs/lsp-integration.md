
# DX-Forge LSP Integration Guide

## Overview

This guide explains how to integrate the DX-Forge LSP (Language Server Protocol) server with your development environment, specifically VSCode.

## Architecture

@tree[]

## LSP Server Capabilities

The `forge-lsp` server provides: -Text Synchronization: `textDocument/didOpen`, `didChange`, `didClose` -Code Completion: `textDocument/completion` for DX components -Hover Information: `textDocument/hover` for symbol information -Semantic Analysis: AST-based symbol resolution using tree-sitter

## Installation

### 1. Build the LSP Server

```bash
cd /path/to/dx-forge


# Debug build (recommended for development)


cargo build --bin forge-lsp


# Release build (if linker works on your system)


cargo build --release --bin forge-lsp ```
Output: Binary at `target/debug/forge-lsp` or `target/release/forge-lsp`


### 2. Verify Installation


```bash
./target/debug/forge-lsp --version ```

## VSCode Integration

### Method 1: Using Language Client (Recommended)

#### Install Dependencies

```bash
cd vscode-forge npm install vscode-languageclient ```


#### Create Language Client


File: `src/languageClient.ts`
```typescript
import * as path from 'path';
import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';
let client: LanguageClient;
export function activateLanguageClient(context: vscode.ExtensionContext) { // Path to LSP server binary const serverPath = findLSPBinary();
if (!serverPath) { vscode.window.showErrorMessage('forge-lsp binary not found');
return;
}
// Server options const serverOptions: ServerOptions = { run: { command: serverPath, transport: TransportKind.stdio }, debug: { command: serverPath, transport: TransportKind.stdio }
};
// Client options const clientOptions: LanguageClientOptions = { documentSelector: [ { scheme: 'file', language: 'rust' }, { scheme: 'file', language: 'typescript' }, { scheme: 'file', language: 'typescriptreact' }
], synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{rs,ts,tsx}')
}
};
// Create and start client client = new LanguageClient( 'forgeLSP', 'DX-Forge LSP Server', serverOptions, clientOptions );
client.start();
context.subscriptions.push({ dispose: () => client.stop()
});
}
function findLSPBinary(): string | null { const possiblePaths = [ path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, 'target', 'debug', 'forge-lsp'),
path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, 'target', 'release', 'forge-lsp'), 'forge-lsp' // In PATH ];
for (const binPath of possiblePaths) { if (require('fs').existsSync(binPath)) { return binPath;
}
}
return null;
}
```


#### Update Extension Activation


File: `src/extension.ts`
```typescript
import { activateLanguageClient } from './languageClient';
export function activate(context: vscode.ExtensionContext) { // ... existing code ...
// Activate LSP client activateLanguageClient(context);
}
```


#### Update package.json


```json
{ "activationEvents": [ "onLanguage:rust", "onLanguage:typescript", "onLanguage:typescriptreact"
], "contributes": { "configuration": { "type": "object", "title": "DX-Forge LSP", "properties": { "forge.lsp.enabled": { "type": "boolean", "default": true, "description": "Enable DX-Forge LSP server"
}, "forge.lsp.serverPath": { "type": "string", "default": "", "description": "Custom path to forge-lsp binary"
}
}
}
}
}
```


###Method 2: Manual Stdio Communication (Current Implementation) The current `ForgeWatcher` in `extension.ts` uses manual stdio communication. This works but is less robust than using the official Language Client.



## LSP Protocol Messages



### Initialize


Request:
```json
{ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "processId": 12345, "rootUri": "file:///path/to/workspace", "capabilities": {}
}
}
```
Response:
```json
{ "jsonrpc": "2.0", "id": 1, "result": { "capabilities": { "textDocumentSync": 2, "completionProvider": {}, "hoverProvider": true }
}
}
```


### Text Document Sync


didOpen:
```json
{ "jsonrpc": "2.0", "method": "textDocument/didOpen", "params": { "textDocument": { "uri": "file:///path/to/file.rs", "languageId": "rust", "version": 1, "text": "fn main() {}"
}
}
}
```


### Completion


Request:
```json
{ "jsonrpc": "2.0", "id": 2, "method": "textDocument/completion", "params": { "textDocument": { "uri": "file:///path/to/file.tsx" }, "position": { "line": 10, "character": 5 }
}
}
```
Response:
```json
{ "jsonrpc": "2.0", "id": 2, "result": [ { "label": "dxButton", "kind": 7, "detail": "DX Button Component", "documentation": "A customizable button component"
}
]
}
```


## Configuration



### Environment Variables


```bash

# Logging level

export RUST_LOG=info

# LSP server port (if using TCP)

export FORGE_LSP_PORT=7878 ```

### VSCode Settings

`.vscode/settings.json`:
```json
{ "forge.lsp.enabled": true, "forge.lsp.serverPath": "${workspaceFolder}/target/debug/forge-lsp", "forge.lsp.trace.server": "verbose"
}
```

## Debugging

### LSP Server Logs

Add logging to `src/bin/lsp.rs`:
```rust
use tracing_subscriber;
fn main() { tracing_subscriber::fmt()
.with_max_level(tracing::Level::DEBUG)
.init();
// ... rest of code }
```
Run and check logs:
```bash
./target/debug/forge-lsp 2>&1 | tee lsp.log
```

### VSCode Client Logs

- Open VSCode Output panel
- Select "DX-Forge LSP" from dropdown
- View client-server communication

### Test LSP Manually

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":"file:///tmp"}}' | ./target/debug/forge-lsp
```

## Extending LSP Capabilities

### Add New Capability

- Define in `src/server/lsp.rs`:
```rust
pub fn handle_goto_definition(&self, uri: &str, line: usize, col: usize) -> Option<Location> { // Implementation }
```
- Register in `src/bin/lsp.rs`:
```rust
"textDocument/definition" => { let result = server.handle_goto_definition(&uri, line, col);
// Send response }
```
- Update capabilities in initialize response:
```rust
"definitionProvider": true ```


## Performance Optimization



### Incremental Parsing


Currently, the semantic analyzer re-parses the entire file on each change. Optimize with:
```rust
pub fn update_file_incremental( &mut self, file_path: &Path, old_tree: &Tree, source: &str, changes: &[TextEdit]
) -> Result<Tree> { self.parser.parse(source, Some(old_tree))
}
```


### Caching


Add caching for frequently accessed symbols:
```rust
use lru::LruCache;
pub struct SemanticAnalyzer { parser: Parser, symbol_table: HashMap<String, Vec<Symbol>>, cache: LruCache<String, Vec<Symbol>>, // NEW }
```


## Troubleshooting


See `docs/troubleshooting.md` for common issues.


## Resources


- LSP Specification
- tree-sitter Documentation
- vscode-languageclient
