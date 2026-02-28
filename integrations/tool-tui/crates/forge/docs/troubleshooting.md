
# DX-Forge Troubleshooting Guide

## Common Issues & Solutions

## Build & Compilation Issues

### Issue: Windows Linker Error (forge-lsp Release Build)

Error Message:
```
error: linking with `link.exe` failed: exit code: 1120 LINK : fatal error LNK1120: 1 unresolved externals ```
Cause: Missing or misconfigured Visual Studio Build Tools Solutions: -Use Debug Builds (Recommended for development):
```bash
cargo build --bin forge-lsp ```
- Install Visual Studio Build Tools:
- Download: //visualstudio.microsoft.com/downloads/
- Install "Desktop development with C++"
- Ensure "MSVC v143" and "Windows SDK" are selected
- Check Rust Toolchain:
```bash
rustc --version rustup show rustup default stable-x86_64-pc-windows-msvc ```


### Issue: tree-sitter Compilation Errors


Error Message:
```
error[E0308]: mismatched types expected `&Language`, found `&LanguageFn`
```
Cause: Incompatible tree-sitter versions Solution: Check `Cargo.toml` versions match:
```toml
tree-sitter = "0.24.7"
tree-sitter-rust = "0.24.0"
```
Update code to use `.into()`:
```rust
let language = tree_sitter_rust::LANGUAGE.into();
parser.set_language(&language)?;
```


## VSCode Extension Issues



### Issue: better-sqlite3 Installation Fails


Error Message:
```
npm error gyp ERR! build error npm error gyp ERR! stack Error: `MSBuild.exe` failed ```
Cause: Missing C++ build tools Solutions: -Install Visual Studio Build Tools (see above) -Use WSL:
```bash
wsl cd /mnt/f/Code/forge/vscode-forge npm install ```
- Alternative Package (pure JS fallback):
```bash
npm uninstall better-sqlite3 npm install better-sqlite3-multiple-ciphers ```
Update `src/database.ts`:
```typescript
import Database from 'better-sqlite3-multiple-ciphers';
```

### Issue: Extension Not Activating

Symptoms: Commands not registered, no output in console Debug Steps: -Check Extension Host Log: -`Ctrl+Shift+P` → "Developer: Show Logs" → "Extension Host" -Verify package.json activationEvents:
```json
"activationEvents": [ "onCommand:forge.start", "workspaceContains:**/Cargo.toml"
]
```
- Check for Compilation Errors:
```bash
cd vscode-forge npm run compile ```
- Install in Development Mode:
- Press `F5` in VSCode (opens Extension Development Host)
- Check Debug Console for errors


### Issue: forge.showTraffic Command Not Found


Error Message:
```
command 'forge.showTraffic' not found ```
Solutions: -Verify Command Registration in `extension.ts`:
```typescript
context.subscriptions.push( vscode.commands.registerCommand('forge.showTraffic', () => { TrafficBranchPanel.createOrShow(context.extensionUri, forgeDatabase);
})
);
```
- Check package.json:
```json
"contributes": { "commands": [{ "command": "forge.showTraffic", "title": "Show Traffic Branch Status"
}]
}
```
- Reload Extension:
- `Ctrl+Shift+P` → "Developer: Reload Window"

## LSP Server Issues

### Issue: LSP Server Not Starting

Symptoms: No completions, no hover information Debug Steps: -Check Binary Exists:
```bash
ls target/debug/forge-lsp # Should exist ```
- Test Manual Start:
```bash
./target/debug/forge-lsp

# Should wait for JSON-RPC input

```
- Check Logs: Enable logging in `extension.ts`:
```typescript
outputChannel.appendLine(`Starting LSP: ${forgeBinary}`);
```
- Verify Permissions:
```bash
chmod +x target/debug/forge-lsp # Unix/Linux ```

### Issue: Completions Not Showing

Possible Causes: -Wrong File Type: -Completions only work in `.tsx`, `.ts`, `.rust` files -Check `documentSelector` in language client config -Semantic Analyzer Not Initialized:
```rust
// In src/server/lsp.rs pub struct LspServer { semantic_analyzer: SemanticAnalyzer, // Must be initialized // ...
}
```
- No DX Patterns Detected: Currently uses Rust parser. For TSX:
- Add `tree-sitter-tsx` to `Cargo.toml`
- Update `detect_dx_patterns()` to use TSX parser for `.tsx` files

## Database Issues

### Issue: forge.db Not Found

Error Message:
```
Error: ENOENT: no such file or directory, open 'forge.db' ```
Solutions: -Create Database (if doesn't exist): -Run Forge CLI to initialize: `forge init` -Or create schema manually (see `docs/database-schema.md`) -Check Database Path in `database.ts`:
```typescript
constructor(workspaceRoot: string) { this.dbPath = path.join(workspaceRoot, 'forge.db');
// Verify path is correct }
```
- Verify File Permissions:
```bash
ls -l forge.db # Should be readable ```

### Issue: CRDT Operations Not Loading

Symptoms: Empty operation list, no data in panels Debug Steps: -Check Database Schema:
```sql
sqlite3 forge.db .schema operations .schema traffic_status ```
- Verify Data Exists:
```sql
SELECT COUNT(*) FROM operations;
SELECT * FROM traffic_status LIMIT 5;
```
- Check Database Connection:
```typescript
if (!this.db) { console.error('Database not initialized');
return [];
}
```
- Test Query Directly:
```typescript
const stmt = this.db.prepare('SELECT * FROM operations LIMIT 1');
console.log(stmt.get());
```


## WebSocket Issues



### Issue: WebSocket Connection Failed


Error Message:
```
$(alert) WebSocket connection failed: ECONNREFUSED ```
Solutions: -Start WebSocket Server:
```bash


# If you have a standalone WebSocket server


cargo run --bin forge-ws-server ```
- Check Port:
```typescript
// Default is 3456 webSocketClient = new WebSocketClient('ws://localhost:3456');
```
- Verify Server Running:
```bash
netstat -ano | findstr :3456 # Windows
lsof -i :3456 # Unix/Linux ```
- Configure Firewall:
- Allow port 3456 in Windows Firewall
- Or use different port

### Issue: Auto-Reconnect Loop

Symptoms: Constant connect/disconnect messages Cause: No server running, client keeps retrying Solution: Disable auto-reconnect temporarily:
```typescript
webSocketClient.connect().catch(err => { logError(`WebSocket connection failed: ${err}`);
// Don't auto-reconnect if no server });
```
Or increase backoff:
```typescript
private maxRetries = 5; // Limit retries private reconnectDelay = Math.min(this.reconnectAttempts * 2000, 30000);
```

## R2 Storage Issues

### Issue: R2 Authentication Failed

Error Message:
```
Error: 403 Forbidden - Invalid signature ```
Solutions: -Check Environment Variables:
```bash
echo JavaScript/TypeScript2_ACCOUNT_ID echo JavaScript/TypeScript2_ACCESS_KEY_ID

# Should not be empty

```
- Verify Credentials:
- Login to Cloudflare Dashboard
- Check R2 Access Keys
- Generate new key if needed
- Create `.env` File:
```env
R2_ACCOUNT_ID=your-account-id R2_BUCKET_NAME=forge-blobs R2_ACCESS_KEY_ID=your-access-key R2_SECRET_ACCESS_KEY=your-secret-key ```
- Load Environment:
```rust
dotenvy::dotenv().ok(); // Loads .env file ```


### Issue: Component Upload Fails


Error Message:
```
Error: Hash mismatch after download ```
Cause: Network corruption or incorrect hash calculation Solutions: -Retry Upload: -Retry logic is built-in (3 attempts) -Check network stability -Verify Hash Calculation:
```rust
let computed_hash = compute_sha256_hex(&data);
assert_eq!(computed_hash, expected_hash);
```
- Check Blob Size Limits:
- R2 max object size: 5 TB
- Ensure component size is reasonable

## Performance Issues

### Issue: Slow Semantic Analysis

Symptoms: LSP slow to respond, high CPU usage Solutions: -Enable Incremental Parsing:
```rust
pub fn update_file(&mut self, old_tree: &Tree, edits: &[Edit]) { self.parser.parse_with(old_tree, edits);
}
```
- Add Symbol Caching:
```rust
use lru::LruCache;
cache: LruCache::new(100), // Cache 100 files ```
- Limit File Size:
```rust
if source.len() > 1_000_000 { return Err(anyhow!("File too large for analysis"));
}
```


### Issue: High Memory Usage


Symptoms: Extension host crashes, system slowdown Solutions: -Clear Symbol Table Periodically:
```rust
if self.symbol_table.len() > 1000 { self.symbol_table.clear();
}
```
- Limit Watcher Scope:
```typescript
const watcher = vscode.workspace.createFileSystemWatcher( '**/*.{rs,ts,tsx}', false, false, false );
// Exclude node_modules, target, etc.
```
- Disable on Large Projects:
```json
"forge.lsp.maxProjectSize": 100000 // files ```

## Testing Issues

### Issue: Tests Fail to Compile

Error Message:
```
error[E0599]: no method named `calculate_sync_actions`
```
Cause: Method is private Solution: Make method public for tests:
```rust


#[cfg_attr(test, allow(dead_code))]


pub(crate) fn calculate_sync_actions(...) -> ... { ```


### Issue: Test Panics


Error Message:
```
thread 'test_sync_calculation' panicked at 'called `Result::unwrap()` on an `Err` value' ```
Debug: -Use Better Error Messages:
```rust
let storage = R2Storage::new(config)
.expect("Failed to create R2Storage");
```
- Print Errors:
```rust
match R2Storage::new(config) { Ok(s) => s, Err(e) => { eprintln!("Error: {:?}", e);
panic!("Test failed");
}
}
```
- Run Single Test:
```bash
cargo test test_sync_calculation -- --nocapture ```


## Getting Help


If issues persist: -Check Logs: -Rust: `RUST_LOG=debug cargo run` -VSCode: Output → Extension Host -LSP: Output → DX-Forge LSP -Create Minimal Reproduction: -Isolate the issue -Share code snippet -System Information:
```bash
rustc --version node --version code --version cat /etc/os-release # Linux ```
- GitHub Issues:
- Include error messages
- Attach logs
- Describe expected vs actual behavior

## Quick Reference

### Useful Commands

```bash


# Rebuild everything


cargo clean && cargo build


# Run specific test


cargo test test_name -- --nocapture


# Check without building


cargo check


# Format code


cargo fmt


# Lint


cargo clippy


# VSCode extension


cd vscode-forge npm run compile npx vsce package ```


### Log Locations


- Rust: stderr
- VSCode Extension: `Ctrl+Shift+P` → "Developer: Show Logs"
- LSP Server: Check extension host log
- Database: In `forge.db` (SQLite)


### Default Ports


+---------+--------+
| Service | Port   |
+=========+========+
| LSP     | Server |
+---------+--------+
