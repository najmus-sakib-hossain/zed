
# DX Forge Integration Guide for DX Tools

Complete API reference for integrating `dx-forge` into your DX tool (dx-style, dx-ui, dx-icons, dx-fonts, dx-i18n, dx-check, dx-auth).

## Installation

```toml
[dependencies]
dx-forge = "0.0.2"
tokio = { version = "1.48", features = ["full"] }
anyhow = "1.0"
```

## Table of Contents

- Tool Orchestration
- Version Management
- Pattern Detection
- Component Injection
- File Watching
- Traffic Branch Safety
- Error Handling
- Storage & CRDT

## 1. Tool Orchestration

### Creating a DX Tool

Implement the `DxTool` trait to integrate with Forge orchestration:
```rust
use dx_forge::{DxTool, ExecutionContext, ToolOutput};
use anyhow::Result;
struct MyDxTool;
impl DxTool for MyDxTool { fn name(&self) -> &str { "dx-mytool" // Tool identifier }
fn version(&self) -> &str { "1.0.0" // Current tool version }
fn priority(&self) -> u32 { 50 // Lower number = runs earlier (0-100 recommended)
}
fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> { // Your tool logic here let mut output = ToolOutput::success();
output.message = "Task completed".to_string();
Ok(output)
}
// Optional: Pre-execution check fn should_run(&self, ctx: &ExecutionContext) -> bool { // Return true if tool should run based on context
!ctx.changed_files.is_empty()
}
// Optional: Declare dependencies fn dependencies(&self) -> Vec<String> { vec!["dx-style".to_string()] // Runs after dx-style }
// Optional: Lifecycle hooks fn before_execute(&mut self, ctx: &ExecutionContext) -> Result<()> { println!("Setting up...");
Ok(())
}
fn after_execute(&mut self, ctx: &ExecutionContext, output: &ToolOutput) -> Result<()> { println!("Cleaning up...");
Ok(())
}
fn on_error(&mut self, ctx: &ExecutionContext, error: &anyhow::Error) -> Result<()> { eprintln!("Rollback needed: {}", error);
Ok(())
}
fn timeout_seconds(&self) -> u64 { 60 // Execution timeout (0 = no timeout)
}
}
```

### Using the Orchestrator

```rust
use dx_forge::{Orchestrator, OrchestratorConfig};


#[tokio::main]


async fn main() -> Result<()> { // Create orchestrator let mut orchestrator = Orchestrator::new(".")?;
// Or with custom config let config = OrchestratorConfig { parallel: false, // Run tools sequentially fail_fast: true, // Stop on first error max_concurrent: 4, // Max parallel tools traffic_branch_enabled: true, };
let mut orchestrator = Orchestrator::with_config(".", config)?;
// Register tools orchestrator.register_tool(Box::new(MyDxTool))?;
// Execute all tools let outputs = orchestrator.execute_all()?;
// Process results for output in outputs { if output.success { println!("✅ {}", output.message);
} else { eprintln!("❌ {}", output.message);
}
}
Ok(())
}
```

### ExecutionContext API

```rust
// Access context information let ctx: &ExecutionContext = orchestrator.context();
// Repository paths let repo_root = &ctx.repo_root; // PathBuf let forge_path = &ctx.forge_path; // .dx/forge path // Changed files let files = &ctx.changed_files; // Vec<PathBuf> // Git information let branch = &ctx.current_branch; // Option<String> // Shared state between tools ctx.set("my_key", "my_value")?;
let value: Option<String> = ctx.get("my_key")?;
// Traffic branch analysis let analyzer = &ctx.traffic_analyzer;
let branch = analyzer.analyze(file)?;
// Component state manager (if available)
if let Some(mgr) = &ctx.component_manager { let mgr = mgr.read();
let is_managed = mgr.is_managed(&path);
}
```

### ToolOutput Structure

```rust
let mut output = ToolOutput::success(); // or ToolOutput::failure("reason")
output.success = true; // bool output.files_modified = vec![path1, path2]; // Vec<PathBuf> output.files_created = vec![new_file]; // Vec<PathBuf> output.files_deleted = vec![old_file]; // Vec<PathBuf> output.message = "Operation completed".into(); // String output.duration_ms = 150; // u64 ```


## 2. Version Management



### Version Parsing and Comparison


```rust
use dx_forge::{Version, VersionReq};
use std::str::FromStr;
// Parse versions let v1 = Version::from_str("1.2.3")?;
let v2 = Version::from_str("v2.0.0-beta.1")?;
let v3 = Version::new(1, 5, 0);
// Compare versions assert!(v1 < v3);
assert!(v3 < v2);
// Check compatibility (same major version)
assert!(v1.is_compatible_with(&v3));
assert!(!v1.is_compatible_with(&v2));
// Check stability assert!(v1.is_stable()); // >= 1.0.0, no pre-release assert!(!v2.is_stable()); // has pre-release // Version requirements let req = VersionReq::from_str("^1.2.0")?;
assert!(v1.satisfies(&req));
assert!(v3.satisfies(&req));
// Other requirement types let exact = VersionReq::from_str("=1.2.3")?;
let gte = VersionReq::from_str(">=1.0.0")?;
let lt = VersionReq::from_str("<2.0.0")?;
let any = VersionReq::from_str("*")?;
```


### Tool Registry


```rust
use dx_forge::{ToolRegistry, ToolSource, Version};
use std::collections::HashMap;
// Create registry let forge_dir = std::path::Path::new(".dx/forge");
let mut registry = ToolRegistry::new(forge_dir)?;
// Register a tool registry.register( "dx-ui".to_string(), Version::new(2, 1, 0), ToolSource::Crate { version: "2.1.0".to_string(), }, HashMap::new(), // dependencies )?;
// Check registration if registry.is_registered("dx-ui") { let info = registry.get("dx-ui").unwrap();
println!("Tool: {} v{}", info.name, info.version);
}
// Get version let version = registry.version("dx-ui").unwrap();
// Check dependencies let missing = registry.check_dependencies("dx-ui")?;
if !missing.is_empty() { eprintln!("Missing dependencies: {:?}", missing);
}
// Check for updates let latest = Version::new(2, 2, 0);
if registry.needs_update("dx-ui", &latest) { println!("Update available!");
}
// List all tools for tool in registry.list() { println!("{} v{}", tool.name, tool.version);
}
// Unregister registry.unregister("old-tool")?;
```


### ToolSource Types


```rust
use dx_forge::ToolSource;
use std::path::PathBuf;
// From crate let source = ToolSource::Crate { version: "1.0.0".to_string(), };
// From local path let source = ToolSource::Local(PathBuf::from("/path/to/tool"));
// From git let source = ToolSource::Git { url: "https://github.com/user/repo".to_string(), rev: "abc123".to_string(), };
// From R2 storage let source = ToolSource::R2 { bucket: "dx-tools".to_string(), key: "tools/dx-ui/2.1.0".to_string(), };
```


## 3. Pattern Detection



### PatternDetector API


```rust
use dx_forge::{PatternDetector, DxToolType};
use std::path::Path;
// Create detector let detector = PatternDetector::new()?;
// Detect patterns in a file let content = r#"
<dxButton>Click</dxButton> <dxiHome size={24} /> <dxfRoboto>Text</dxfRoboto> "#;
let matches = detector.detect_in_file(Path::new("app.tsx"), content)?;
// Process matches for m in &matches { println!( "Found {} at {}:{} ({})", m.pattern, m.line, m.column, m.tool.tool_name()
);
}
// Check if patterns exist if detector.has_patterns(content) { println!("DX patterns detected!");
}
// Group by tool type let grouped = detector.group_by_tool(matches.clone());
for (tool, tool_matches) in grouped { println!("{}: {} matches", tool.tool_name(), tool_matches.len());
}
// Extract unique components let components = detector.extract_components(&matches);
println!("Components: {:?}", components);
// Detect in multiple files let files = vec![ (PathBuf::from("app.tsx"), content1.to_string()), (PathBuf::from("page.tsx"), content2.to_string()), ];
let all_matches = detector.detect_in_files(&files)?;
```


### DxToolType


```rust
use dx_forge::DxToolType;
// Tool types let ui = DxToolType::Ui; // dx-ui: dxButton, dxInput let icons = DxToolType::Icons; // dx-icons: dxiHome, dxiUser let fonts = DxToolType::Fonts; // dx-fonts: dxfRoboto, dxfInter let style = DxToolType::Style; // dx-style: dxsContainer let i18n = DxToolType::I18n; // dx-i18n: dxtText let auth = DxToolType::Auth; // dx-auth: dxaGoogleLogin let check = DxToolType::Check; // dx-check // Get properties let prefix = ui.prefix(); // "dx"
let tool_name = ui.tool_name(); // "dx-ui"
// Create from prefix let tool = DxToolType::from_prefix("dxi"); // Icons ```

### PatternMatch Structure

```rust
// PatternMatch fields let m = matches[0];
let file = m.file; // PathBuf let line = m.line; // usize (1-indexed)
let column = m.column; // usize (1-indexed)
let pattern = m.pattern; // String (e.g., "dxButton")
let tool = m.tool; // DxToolType let component = m.component_name; // String (e.g., "Button")
```

## 4. Component Injection

### InjectionManager API

```rust
use dx_forge::{InjectionManager, DxToolType, PatternMatch};
use std::path::Path;
// Create injection manager let forge_dir = Path::new(".dx/forge");
let mut mgr = InjectionManager::new(forge_dir)?;
// Check if component is cached if mgr.is_cached(&DxToolType::Ui, "Button") { println!("Component already cached");
}
// Fetch component (caches automatically)
let component = mgr.fetch_component( &DxToolType::Ui, "Button", Some("2.1.0") // version (None = latest)
).await?;
println!("Component code: {}", component);
// Get from cache if let Some(cached) = mgr.get_cached(&DxToolType::Ui, "Button").await? { println!("From cache: {}", cached);
}
// Inject into file with auto-imports let matches = vec![/* PatternMatch instances */];
mgr.inject_into_file(Path::new("app.tsx"), &matches).await?;
// Cache statistics let stats = mgr.cache_stats();
println!("Total components: {}", stats.total_components);
println!("Total size: {} bytes", stats.total_size_bytes);
for (tool, count) in stats.components_by_tool { println!(" {}: {} components", tool, count);
}
// Cleanup old cache (LRU eviction)
let removed = mgr.cleanup_cache(30).await?; // 30 days println!("Removed {} old components", removed);
```

### ComponentMetadata

```rust
// Metadata structure let metadata = ComponentMetadata { name: "Button".to_string(), version: "2.1.0".to_string(), tool: "dx-ui".to_string(), hash: "abc123...".to_string(), size: 1024, dependencies: vec!["React".to_string()], exports: vec!["Button".to_string(), "ButtonProps".to_string()], };
```

## 5. File Watching

### DualWatcher API

```rust
use dx_forge::{DualWatcher, FileChange, ChangeKind, ChangeSource};
// Create dual watcher let mut watcher = DualWatcher::new()?;
// Start watching watcher.start("./src").await?;
// Get receiver for changes let mut rx = watcher.receiver();
// Listen for changes tokio::spawn(async move { while let Ok(change) = rx.recv().await { println!( "{:?}: {} (via {:?})", change.kind, change.path.display(), change.source );
// Check if patterns detected if let Some(patterns) = change.patterns { println!(" Detected {} patterns", patterns.len());
}
}
});
// Or use next_change loop { let change = watcher.next_change().await?;
// Analyze for patterns if not already done let change = watcher.analyze_patterns(change).await?;
// Process change handle_change(change)?;
}
// Stop watching watcher.stop().await?;
```

### FileChange Structure

```rust
let change: FileChange = /* from watcher */;
let path = change.path; // PathBuf let kind = change.kind; // ChangeKind let source = change.source; // ChangeSource let timestamp = change.timestamp; // SystemTime let content = change.content; // Option<String> let patterns = change.patterns; // Option<Vec<PatternMatch>> // ChangeKind variants match change.kind { ChangeKind::Created => { /* file created */ }
ChangeKind::Modified => { /* file modified */ }
ChangeKind::Deleted => { /* file deleted */ }
ChangeKind::Renamed => { /* file renamed */ }
}
// ChangeSource variants match change.source { ChangeSource::Lsp => { /* from LSP */ }
ChangeSource::FileSystem => { /* from fs watcher */ }
}
```

## 6. Traffic Branch Safety

### ComponentStateManager API

```rust
use dx_forge::{ComponentStateManager, TrafficBranch};
use std::path::Path;
// Create state manager let forge_dir = Path::new(".dx/forge");
let mut mgr = ComponentStateManager::new(forge_dir)?;
// Register a component let content = std::fs::read_to_string("Button.tsx")?;
mgr.register_component( Path::new("src/components/Button.tsx"), "dx-ui", // source tool "Button", // component name "2.1.0", // version &content, // initial content )?;
// Check if managed if mgr.is_managed(Path::new("Button.tsx")) { println!("Component is tracked");
}
// Analyze update strategy let remote_content = fetch_latest_button()?;
let branch = mgr.analyze_update( Path::new("Button.tsx"), &remote_content )?;
match branch { TrafficBranch::Green => { println!("🟢 Safe to auto-update");
// Apply update directly }
TrafficBranch::Yellow { conflicts } => { println!("🟡 Merge required");
println!("Conflicts: {:?}", conflicts);
// Attempt 3-way merge }
TrafficBranch::Red { conflicts } => { println!("🔴 Manual resolution required");
println!("Conflicts: {:?}", conflicts);
// User intervention needed }
}
// Update component after merge mgr.update_component( Path::new("Button.tsx"), "2.2.0", // new version &merged_content, )?;
// Unregister component mgr.unregister_component(Path::new("Button.tsx"))?;
// List all managed components for state in mgr.list_components() { println!("{} v{} ({})", state.name, state.version, state.source);
}
```

### Apply Update with Traffic Branch

```rust
use dx_forge::context::apply_update;
let result = apply_update( Path::new("Button.tsx"), &remote_content, "2.2.0", &mut mgr, ).await?;
match result { UpdateResult::AutoUpdated => { println!("✅ Auto-updated successfully");
}
UpdateResult::Merged => { println!("✅ Merged with local changes");
}
UpdateResult::Conflict { conflicts } => { println!("❌ Conflicts detected");
for conflict in conflicts { println!(" - {}", conflict);
}
}
}
```

## 7. Error Handling

### Enhanced Error Handling

```rust
use dx_forge::{EnhancedError, ErrorCategory, categorize_error};
// Create enhanced error let error: anyhow::Error = /* some error */;
let enhanced = EnhancedError::new(error);
println!("{}", enhanced.display());
// Access error properties let category = enhanced.category; // ErrorCategory let context = enhanced.context; // Vec<String> let suggestions = enhanced.suggestions; // Vec<String> // Categorize errors let category = categorize_error(&error);
match category { ErrorCategory::Network => { /* retry */ }
ErrorCategory::FileSystem => { /* check permissions */ }
ErrorCategory::Configuration => { /* fix config */ }
ErrorCategory::Dependency => { /* resolve deps */ }
ErrorCategory::Timeout => { /* increase timeout */ }
ErrorCategory::Validation => { /* fix input */ }
ErrorCategory::Unknown => { /* log and report */ }
}
// Check if retryable if category.is_retryable() { // Retry the operation }
```

### Retry Logic

```rust
use dx_forge::{with_retry, RetryPolicy};
use tokio::time::Duration;
// Default retry policy (3 attempts, exponential backoff)
let policy = RetryPolicy::default();
// Custom retry policy let policy = RetryPolicy { max_attempts: 5, initial_delay: Duration::from_millis(100), backoff_multiplier: 2.0, max_delay: Duration::from_secs(5), };
// No retry let policy = RetryPolicy::no_retry();
// Aggressive retry let policy = RetryPolicy::aggressive();
// Use with retry let result = with_retry(&policy, || { // Your operation that might fail fetch_component_from_r2()
}).await?;
```

### Convert to Enhanced Result

```rust
use dx_forge::{ToEnhanced, EnhancedResult};
fn my_function() -> EnhancedResult<String> { let result = std::fs::read_to_string("file.txt")
.enhance()?; // Convert to EnhancedError Ok(result)
}
```

## 8. Storage & CRDT

### Database API

```rust
use dx_forge::{Database, Operation};
// Open database let db = Database::open(".dx/forge")?;
// Or create new let db = Database::new(Path::new(".dx/forge"))?;
db.initialize()?;
// Store operation let op = Operation { /* ... */ };
db.store_operation(&op)?;
// Get operations let ops = db.get_operations( Some(Path::new("file.txt")), // file filter 100 // limit )?;
// Store anchor let anchor = Anchor::new( "file.txt".to_string(), position, Some("Important line".to_string())
);
db.store_anchor(&anchor)?;
// Get anchors let anchors = db.get_anchors(Path::new("file.txt"))?;
```

### CRDT Operations

```rust
use dx_forge::{Operation, OperationType, Position};
use chrono::Utc;
use uuid::Uuid;
// Create position let pos = Position::new( 10, // line 5, // column 0, // offset "actor_id", // actor 1, // counter );
// Create operation let op = Operation { id: Uuid::new_v4(), file_path: "src/main.rs".to_string(), op_type: OperationType::Insert { position: pos.clone(), content: "let x = 42;".to_string(), length: 11, }, actor_id: "actor_id".to_string(), timestamp: Utc::now(), dependencies: vec![], };
// Operation types let insert = OperationType::Insert { position, content, length };
let delete = OperationType::Delete { position, length };
let replace = OperationType::Replace { position, old_content: "old".to_string(), new_content: "new".to_string(), };
let create = OperationType::FileCreate { path: "file.txt".to_string() };
let delete_file = OperationType::FileDelete;
let rename = OperationType::FileRename { old_path: "old.txt".to_string(), new_path: "new.txt".to_string(), };
```

## Complete Integration Example

```rust
use dx_forge::{ Orchestrator, OrchestratorConfig, DxTool, ExecutionContext, ToolOutput, PatternDetector, InjectionManager, DualWatcher, ToolRegistry, Version, ToolSource, DxToolType, };
use anyhow::Result;
use std::collections::HashMap;
struct DxUiTool { registry: ToolRegistry, injection_mgr: InjectionManager, }
impl DxUiTool { fn new() -> Result<Self> { let forge_dir = std::path::Path::new(".dx/forge");
Ok(Self { registry: ToolRegistry::new(forge_dir)?, injection_mgr: InjectionManager::new(forge_dir)?, })
}
}
impl DxTool for DxUiTool { fn name(&self) -> &str { "dx-ui" }
fn version(&self) -> &str { "2.1.0" }
fn priority(&self) -> u32 { 80 }
fn dependencies(&self) -> Vec<String> { vec!["dx-style".to_string()]
}
fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> { let detector = PatternDetector::new()?;
let mut output = ToolOutput::success();
for file in &ctx.changed_files { if let Ok(content) = std::fs::read_to_string(file) { let matches = detector.detect_in_file(file, &content)?;
// Fetch and inject components for m in &matches { if m.tool == DxToolType::Ui { self.injection_mgr .fetch_component(&m.tool, &m.component_name, None)
.await?;
}
}
// Inject into file self.injection_mgr.inject_into_file(file, &matches).await?;
output.files_modified.push(file.clone());
}
}
output.message = format!("Processed {} files", output.files_modified.len());
Ok(output)
}
}


#[tokio::main]


async fn main() -> Result<()> { // Setup orchestrator let mut orchestrator = Orchestrator::new(".")?;
orchestrator.register_tool(Box::new(DxUiTool::new()?))?;
// Watch for changes let mut watcher = DualWatcher::new()?;
watcher.start("./src").await?;
let mut rx = watcher.receiver();
while let Ok(change) = rx.recv().await { // Update context with changed files orchestrator.context_mut().changed_files = vec![change.path];
// Execute tools let outputs = orchestrator.execute_all()?;
for output in outputs { println!("{}", output.message);
}
}
Ok(())
}
```

## Best Practices

### 1. Tool Priority Guidelines

- 100+: Preprocessing (dx-style, dx-fonts)
- 80-99: Core logic (dx-ui, dx-icons)
- 50-79: Secondary features (dx-i18n, dx-auth)
- 10-49: Validation (dx-check)
- 0-9: Cleanup and reporting

### 2. Error Handling

- Always use `Result<T>` return types
- Implement proper error hooks for cleanup
- Use retry logic for network operations
- Provide helpful error messages

### 3. Performance

- Cache frequently accessed data
- Use pattern detection efficiently
- Batch file operations
- Implement proper cleanup (LRU eviction)

### 4. Versioning

- Follow semantic versioning
- Declare dependencies explicitly
- Check version compatibility
- Handle breaking changes gracefully

### 5. Testing

```rust


#[cfg(test)]


mod tests { use super::*;
use tempfile::TempDir;


#[tokio::test]


async fn test_my_tool() { let temp = TempDir::new().unwrap();
// Test your tool }
}
```

## Support

- Documentation: //docs.rs/dx-forge
- Repository: //github.com/najmus-sakib-hossain/forge
- Issues: //github.com/najmus-sakib-hossain/forge/issues Version: 0.0.2 Last Updated: November 13, 2025
