
# DX Forge API Quick Reference

A concise reference for the 132 eternal API functions in forge v0.1.0.

## Quick Import

```rust
use dx_forge::*;
```

## Core Lifecycle

```rust
initialize_forge()?; // Call once at startup let tool_id = register_tool(tool)?; // Register your tool let ctx = get_tool_context()?; // Get current context shutdown_forge()?; // Clean shutdown ```


## Version Management


```rust
declare_tool_version("my-tool", "1.0.0")?;
enforce_exact_version("tool", "1.0.0", actual); // Panics on mismatch let version = current_forge_version();
let variant = query_active_package_variant()?;
activate_package_variant("shadcn-pro", false)?;
```


## Pipeline Control


```rust
execute_pipeline("default")?;
execute_tool_immediately("urgent-tool")?;
let order = get_resolved_execution_order()?;
suspend_pipeline_execution()?;
resume_pipeline_execution()?;
restart_current_pipeline()?;
```


## Reactivity


```rust
// Instant path trigger_realtime_event(file, content)?;
// 300ms debounce trigger_debounced_event(file, content).await?;
// ≥2s idle trigger_idle_event(file).await?;
// Batching begin_batch_operation()?;
// ... multiple operations ...
end_batch_operation()?;
```


## File Changes & Branching


```rust
// Apply with safety let files = apply_changes(changes)?;
// Fast path (pre-approved)
let files = apply_changes_with_preapproved_votes(changes)?;
// Force (unsafe)
let files = apply_changes_force_unchecked(changes)?;
// Preview let preview = preview_proposed_changes(changes)?;
// Branching votes let vote = BranchingVote { voter_id: "security".to_string(), color: BranchColor::Green, reason: "Safe change".to_string(), confidence: 0.95, };
submit_branching_vote(&file, vote)?;
// Query outcome let color = query_predicted_branch_color(&file)?;
let safe = is_change_guaranteed_safe(&file)?;
// Hard veto issue_immediate_veto(&file, "security", "Breaking change")?;
```


## Events


```rust
// Subscribe let mut rx = subscribe_to_event_stream();
// Emit events emit_tool_started_event("my-tool")?;
emit_tool_completed_event("my-tool", 150)?;
emit_pipeline_started_event("default")?;
emit_magical_config_injection("style")?;
// Custom events publish_event(ForgeEvent::Custom { event_type: "custom".into(), data: json!({}), timestamp: Utc::now().timestamp()
})?;
```


## Configuration


```rust
// Auto-detect config let config_path = get_active_config_file_path()?;
// Magic injection (the killer feature!)
let config = inject_full_config_section_at_cursor("style")?;
// Specific configs let style = inject_style_tooling_config()?;
let auth = inject_authentication_config()?;
let ui = inject_ui_framework_config()?;
// Expansion let expanded = expand_config_placeholder("style:")?;
// Validation & completion validate_config_in_realtime()?;
let suggestions = provide_config_completion_suggestions("st")?;
```


## CI/CD & Workspace


```rust
// CI/CD trigger_ci_cd_pipeline("deploy")?;
register_ci_stage("test", "npm test")?;
let status = query_current_ci_status()?;
// Workspace let root = detect_workspace_root()?;
let members = list_all_workspace_members()?;
synchronize_monorepo_workspace()?;
```


##.dx/ Directory


```rust
// Paths let dx_dir = get_dx_directory_path()?;
let bin_dir = get_dx_binary_storage_path()?;
// Binary caching cache_tool_offline_binary("style", binary_data)?;
let binary = load_tool_offline_binary("style")?;
// State management let commit_id = commit_current_dx_state("message")?;
checkout_dx_state(&commit_id)?;
let history = list_dx_history()?;
// Cloud sync push_dx_state_to_remote("https://dx.cloud")?;
pull_dx_state_from_remote("https://dx.cloud")?;
```


## Offline Mode


```rust
let offline = detect_offline_mode()?;
force_offline_operation()?;
download_missing_tool_binaries(vec!["tool1".into()])?;
verify_binary_integrity_and_signature("tool")?;
update_tool_binary_atomically("tool", new_binary)?;
```


## Cart System


```rust
// Stage items let item = CartItem { id: "item-1".into(), package_id: "shadcn".into(), variant: Some("default".into()), files: vec![], config: json!({}), };
stage_item_in_cart(item)?;
// Manage cart let contents = get_current_cart_contents()?;
remove_specific_cart_item("item-1")?;
clear_cart_completely()?;
// Commit let installed = commit_entire_cart()?;
// Import/export let json = export_cart_as_shareable_json()?;
import_cart_from_json(&json)?;
```


## Package Management


```rust
// Install let files = install_package_with_variant("shadcn", "pro")?;
// Manage uninstall_package_safely("old-package")?;
update_package_intelligently("shadcn")?;
pin_package_to_exact_version("shadcn", "2.0.0")?;
// Discover let results = search_dx_package_registry("button")?;
let installed = list_all_installed_packages()?;
// Variants let new_variant = fork_existing_variant("shadcn", "pro", "my-pro")?;
publish_your_variant("shadcn", "my-pro")?;
```


## Code Governance


```rust
// Mark generated code mark_code_region_as_dx_generated(file, 10, 50, "codegen")?;
let is_gen = is_region_dx_generated(file, 25)?;
// Allow editing allow_safe_manual_edit_of_generated_code(file, 25)?;
// File ownership claim_full_ownership_of_file(file, "my-tool")?;
release_ownership_of_file(file)?;
```


## DX Experience


```rust
// Paths let root = project_root_directory()?;
let manifest = path_to_forge_manifest()?;
let cache = dx_global_cache_directory()?;
// Editor integration open_file_and_reveal_location(file, 10, 5)?;
let suggestion_id = display_inline_code_suggestion(file, 10, "Fix this")?;
apply_user_accepted_suggestion(&suggestion_id)?;
// UI display_dx_command_palette()?;
open_embedded_dx_terminal()?;
open_dx_explorer_sidebar()?;
update_dx_status_bar_indicator("Ready", "green")?;
// AI let suggestion = trigger_ai_powered_suggestion("context")?;
apply_ai_generated_completion(&suggestion)?;
// Utilities log_structured_tool_action("tool", "action", json!({}))?;
let report = generate_comprehensive_project_report()?;
execute_full_security_audit()?;
```


## Complete Tool Example


```rust
use dx_forge::*;
use anyhow::Result;
struct MyDxTool { enabled: bool, }
impl DxTool for MyDxTool { fn name(&self) -> &str { "my-dx-tool" }
fn version(&self) -> &str { "1.0.0" }
fn priority(&self) -> u32 { 50 }
fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> { // Your tool logic here emit_tool_started_event(self.name())?;
let start = std::time::Instant::now();
// Do work...
let duration = start.elapsed().as_millis() as u64;
emit_tool_completed_event(self.name(), duration)?;
Ok(ToolOutput::success())
}
fn should_run(&self, ctx: &ExecutionContext) -> bool { self.enabled && !ctx.changed_files.is_empty()
}
fn dependencies(&self) -> Vec<String> { vec!["dx-codegen".into()]
}
}
fn main() -> Result<()> { // Initialize initialize_forge()?;
// Register tool let tool = Box::new(MyDxTool { enabled: true });
register_tool(tool)?;
// Execute pipeline execute_pipeline("default")?;
// Clean up shutdown_forge()?;
Ok(())
}
```


## Type Definitions


```rust
// Branching colors enum BranchColor { Green, // Auto-approve Yellow, // Review recommended Red, // Manual resolution NoOpinion, // Abstain }
// Events enum ForgeEvent { ToolStarted { tool_id: String, timestamp: i64 }, ToolCompleted { tool_id: String, duration_ms: u64, timestamp: i64 }, PipelineStarted { pipeline_id: String, timestamp: i64 }, PipelineCompleted { pipeline_id: String, duration_ms: u64, timestamp: i64 }, PackageInstallationBegin { package_id: String, timestamp: i64 }, PackageInstallationSuccess { package_id: String, timestamp: i64 }, SecurityViolationDetected { description: String, severity: String, timestamp: i64 }, MagicalConfigInjection { config_section: String, timestamp: i64 }, Custom { event_type: String, data: serde_json::Value, timestamp: i64 }, }
// Cart items struct CartItem { pub id: String, pub package_id: String, pub variant: Option<String>, pub files: Vec<PathBuf>, pub config: serde_json::Value, }
```
Total: 132 functions across 14 categories Version: forge v0.1.0 Status: Production ready ✅
