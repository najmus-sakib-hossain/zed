//! # DX Forge - Production-Ready VCS and Orchestration Engine
//!
//! Forge is the orchestration backbone for the DX tools ecosystem, providing:
//! - Content-addressable storage with SHA-256 blob hashing
//! - Git-compatible versioning with traffic branch safety system
//! - Dual-watcher architecture (LSP + File System monitoring)
//! - Tool orchestration with priority-based execution and dependency resolution
//! - Component injection for zero-bloat dependency management
//! - Semantic versioning with dependency resolution
//! - Pattern detection for dx-tools (dxButton, dxiIcon, dxfRoboto, etc.)
//! - R2 component caching and injection
//! - Production error handling with retry logic
//!
//! ## Architecture Overview
//!
//! Forge eliminates node_modules bloat by detecting code patterns via LSP,
//! injecting only needed components directly into user files, and coordinating
//! DX tool execution with traffic branch safety logic.
//!
//! ### Core Components
//!
//! - **Orchestrator**: Coordinates tool execution with lifecycle hooks, circular dependency detection
//! - **Dual-Watcher**: Monitors LSP + file system changes with pattern detection
//! - **Traffic Branch System**: Green (auto), Yellow (merge), Red (manual) for safe updates
//! - **Storage Layer**: Content-addressable blobs with R2 cloud sync
//! - **Version Manager**: Semantic versioning with compatibility checking
//! - **Pattern Detector**: Identifies dx-tool patterns in source code
//! - **Injection Manager**: Fetches and caches components from R2 storage
//!
//! ## Quick Start - Tool Development
//!
//! ```rust,no_run
//! use dx_forge::{DxTool, ExecutionContext, ToolOutput, Orchestrator};
//! use anyhow::Result;
//!
//! struct MyDxTool;
//!
//! impl DxTool for MyDxTool {
//!     fn name(&self) -> &str { "dx-mytool" }
//!     fn version(&self) -> &str { "1.0.0" }
//!     fn priority(&self) -> u32 { 50 }
//!
//!     fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
//!         // Your tool logic here
//!         Ok(ToolOutput::success())
//!     }
//! }
//!
//! fn main() -> Result<()> {
//!     let mut orchestrator = Orchestrator::new(".")?;
//!     orchestrator.register_tool(Box::new(MyDxTool))?;
//!     let _outputs = orchestrator.execute_all()?;
//!     Ok(())
//! }
//! ```
//!
//! ## Quick Start - Change Detection
//!
//! ```rust,no_run
//! use dx_forge::{DualWatcher, FileChange};
//! use anyhow::Result;
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut watcher = DualWatcher::new()?;
//!     let project_root = PathBuf::from(".");
//!
//!     // Start watching for changes
//!     watcher.start(&project_root).await?;
//!
//!     // Subscribe to the unified change stream
//!     let mut rx = watcher.receiver();
//!
//!     while let Ok(change) = rx.recv().await {
//!         println!("Change detected: {:?} ({:?})", change.path, change.source);
//!     }
//!
//!     Ok(())
//! }
//! ```

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================
//
// Modules are organized into logical groups based on their functionality.
// Each group serves a specific purpose in the Forge architecture.
// ============================================================================

// ----------------------------------------------------------------------------
// Core Foundation Modules
// ----------------------------------------------------------------------------
// These modules provide the fundamental building blocks of Forge, including
// the unified API, execution context, and storage layer.

/// Core Forge API - unified entry point for all Forge operations
pub mod core;

/// Execution context management for tool operations
pub mod context;

/// Content-addressable storage with SQLite backend
pub mod storage;

/// Synchronization primitives and utilities
pub mod sync;

// ----------------------------------------------------------------------------
// Orchestration & Tool Execution Modules
// ----------------------------------------------------------------------------
// These modules handle tool registration, execution ordering, and lifecycle
// management with the traffic branch safety system.

/// Tool orchestration with priority-based execution and dependency resolution
pub mod orchestrator;

/// File system and LSP change detection with dual-watcher architecture
pub mod watcher;

/// Sovereign orchestration engine for advanced tool management
pub mod sovereign;

// ----------------------------------------------------------------------------
// API Functions Module
// ----------------------------------------------------------------------------
// The 132 Eternal API Functions providing comprehensive Forge functionality.

/// Complete API function library (132 functions)
pub mod api;

// ----------------------------------------------------------------------------
// DX Tools Support Modules
// ----------------------------------------------------------------------------
// Modules supporting DX tool development, including pattern detection,
// component injection, and version management.

/// DX tool definitions and registry
pub mod tools;

/// Pattern detection for dx-tool usage in source code
pub mod patterns;

/// Component injection from R2 storage
pub mod injection;

/// Semantic versioning and dependency resolution
pub mod version;

/// DX tool cache management
pub mod dx_cache;

/// DX tool execution engine
pub mod dx_executor;

// ----------------------------------------------------------------------------
// Community & Publishing Modules
// ----------------------------------------------------------------------------
// Contribution tracking and plugin publishing workflow.

/// Contributor credits and attribution
pub mod credits;

/// Plugin publishing pipeline
pub mod publish;

// ----------------------------------------------------------------------------
// Platform & Infrastructure Modules
// ----------------------------------------------------------------------------
// Cross-platform I/O, configuration, and resource management.

/// Platform-native I/O abstraction (io_uring, kqueue, IOCP)
pub mod platform_io;

/// Configuration validation and management
pub mod config;

/// DX directory structure management (.dx/)
pub mod dx_dir;

/// Resource handle management with RAII cleanup
pub mod resource_manager;

/// Metrics collection and reporting
pub mod metrics;

/// Graceful shutdown handling
pub mod shutdown;

// ----------------------------------------------------------------------------
// Error Handling Module
// ----------------------------------------------------------------------------
// Production-grade error handling with categorization and retry logic.

/// Enhanced error handling with categories and retry policies
pub mod error;

// ----------------------------------------------------------------------------
// Daemon & Server Modules
// ----------------------------------------------------------------------------
// Background daemon and server functionality for IDE integration.

/// Forge daemon for background operations
pub mod daemon;

/// HTTP/WebSocket server for external integrations
pub mod server;

// ----------------------------------------------------------------------------
// CRDT & Collaboration Module
// ----------------------------------------------------------------------------
// Conflict-free replicated data types for collaborative editing.

/// CRDT document support for collaborative editing
pub mod crdt;

// ----------------------------------------------------------------------------
// Internal Implementation Modules
// ----------------------------------------------------------------------------
// These modules contain internal implementation details. They are exposed
// for advanced use cases but are not part of the stable public API.

/// Auto-update functionality (internal)
pub mod auto_update;

/// General caching utilities (internal)
pub mod cache;

/// Performance profiling (internal)
pub mod profiler;

/// Tool serialization utilities (internal)
pub mod serializer_tool;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================
//
// This section re-exports types from internal modules to provide a clean,
// organized public API. Types are grouped by functionality with clear
// documentation of naming conventions where multiple similar types exist.
//
// TYPE NAMING CONVENTIONS:
// ------------------------
// This crate has multiple modules with similar type names. To avoid confusion,
// we use the following naming conventions for re-exports:
//
// ToolStatus variants:
//   - `ToolStatus` (default) - Core lifecycle status (Stopped, Starting, Running, etc.)
//   - `SovereignToolStatus` - Sovereign orchestrator status (includes Healthy, Degraded)
//   - `DxToolStatus` - DX tool registry status (Ready, Running, Disabled, Error)
//   - `DaemonToolStatus` - Daemon state status (Idle, Running, Success, Failed, Disabled)
//
// ToolRegistry variants:
//   - `ToolRegistry` (default) - Version management registry (manages versions/dependencies)
//   - `DxToolRegistry` - DX tool registration and tracking
//
// ToolInfo variants:
//   - `ToolInfo` - Version registry tool info (name, version, source, dependencies)
//   - `DxToolInfo` - DX tool registry info (status, run counts, etc.)
// ============================================================================

// ----------------------------------------------------------------------------
// Core Types
// ----------------------------------------------------------------------------
// Primary types for initializing and configuring Forge instances.

pub use core::{
    EditorInfo, EditorType, Forge, ForgeConfig, GeneratedFileInfo, LifecycleEvent, OutputStrategy,
    ToolId, ToolStatus as LifecycleToolStatus,
};

/// Primary tool lifecycle status (from core module)
/// Use this for tool lifecycle management in the Forge API
pub type ToolStatus = LifecycleToolStatus;

// ----------------------------------------------------------------------------
// Orchestration Types
// ----------------------------------------------------------------------------
// Types for tool orchestration, execution context, and traffic branch safety.

pub use orchestrator::{
    Conflict, DxTool, ExecutionContext, Orchestrator, OrchestratorConfig, ToolOutput,
    TrafficAnalyzer, TrafficBranch,
};

// ----------------------------------------------------------------------------
// File Watching Types
// ----------------------------------------------------------------------------
// Types for monitoring file system and LSP changes.

pub use watcher::{ChangeKind, ChangeSource, DualWatcher, FileChange, FileWatcher, LspWatcher};

// ----------------------------------------------------------------------------
// Storage Types
// ----------------------------------------------------------------------------
// Types for content-addressable storage and database operations.

pub use context::{ComponentStateManager, UpdateResult};
#[cfg(feature = "crdt")]
pub use crdt::CrdtDocument;
pub use crdt::{Operation, OperationType, Position};
pub use storage::{Database, DatabasePool, DatabasePoolConfig, OperationLog, PooledConnection};

// ----------------------------------------------------------------------------
// Error Handling Types
// ----------------------------------------------------------------------------
// Types for production-grade error handling with categorization and retry.

pub use error::{
    EnhancedError, EnhancedResult, ErrorCategory, RetryPolicy, ToEnhanced, categorize_error,
    with_retry,
};

// ----------------------------------------------------------------------------
// DX Tools Support Types
// ----------------------------------------------------------------------------
// Types for pattern detection, component injection, and version management.

pub use injection::{CacheStats, ComponentMetadata, InjectionManager};
pub use patterns::{DxToolType, PatternDetector, PatternMatch};
pub use version::{
    Branch, FileSnapshot, Snapshot, SnapshotDiff, SnapshotId, SnapshotManager, ToolInfo,
    ToolRegistry as VersionToolRegistry, ToolSource, ToolState, Version, VersionReq,
};

/// Tool registry for version management (from version module)
/// Manages installed tool versions, dependencies, and compatibility
pub type ToolRegistry = VersionToolRegistry;

// ----------------------------------------------------------------------------
// Sovereign Orchestration Types
// ----------------------------------------------------------------------------
// Types for the Binary Dawn sovereign orchestration engine.

pub use sovereign::{
    BackgroundTask, BackgroundWorker, DxForge, DxToolDefinition,
    Orchestrator as SovereignOrchestrator, ToolStatus as SovereignToolStatus, TrafficLight,
    TrafficManager,
};

// Backward compatibility alias for SovereignToolStatus
#[deprecated(
    since = "0.2.0",
    note = "Use `SovereignToolStatus` for sovereign orchestrator status. This alias will be removed in a future release."
)]
/// Deprecated alias for SovereignToolStatus
pub type SovereignOrchestratorToolStatus = SovereignToolStatus;

// ----------------------------------------------------------------------------
// DX Tool Cache Types
// ----------------------------------------------------------------------------
// Types for DX tool caching and warm-start optimization.

pub use dx_cache::{
    CacheEntry, CacheStats as DxCacheStats, DxToolCacheManager, DxToolId, SyncResult,
    WarmStartResult,
};

// ----------------------------------------------------------------------------
// DX Tool Executor Types
// ----------------------------------------------------------------------------
// Types for executing DX tools with various configurations.

pub use dx_executor::{
    BundlerTool, DxToolExecutable, DxToolExecutor, ExecutionContext as DxExecutionContext,
    PackageManagerTool, StyleTool, TestRunnerTool, ToolConfig, ToolResult,
};

// ----------------------------------------------------------------------------
// Daemon Types
// ----------------------------------------------------------------------------
// Types for the Forge daemon and background operations.

pub use daemon::{
    DaemonConfig, DaemonEvent, DaemonServer, DaemonState, DaemonStateManager, ForgeDaemon,
    IpcCommand, IpcResponse, LspBridge, LspMessage, LspNotification, ProjectState, TaskPriority,
    ToolState as DaemonToolState, ToolStatus as DaemonToolStatus, WorkerPool, WorkerTask,
};

// LSP Server types require the "daemon" feature (axum dependency)
#[cfg(feature = "daemon")]
pub use daemon::{LspRequest, LspResponse as LspServerResponse, LspServer, LspServerState};

// ----------------------------------------------------------------------------
// DX Tools Registry Types
// ----------------------------------------------------------------------------
// Types for DX tool registration and tracking.

pub use tools::{
    DummyTool, ToolInfo as DxToolInfo, ToolRegistry as DxToolRegistry, ToolStatus as DxToolStatus,
    create_dummy_tools,
};

// Backward compatibility aliases
#[deprecated(
    since = "0.2.0",
    note = "Use `DxToolInfo` instead. This alias will be removed in a future release."
)]
/// Deprecated alias for DxToolInfo
pub type RegistryToolInfo = DxToolInfo;

#[deprecated(
    since = "0.2.0",
    note = "Use `DxToolStatus` instead. This alias will be removed in a future release."
)]
/// Deprecated alias for DxToolStatus
pub type RegistryToolStatus = DxToolStatus;

// ----------------------------------------------------------------------------
// Platform I/O Types
// ----------------------------------------------------------------------------
// Types for platform-native I/O operations with automatic fallback.

pub use platform_io::{
    EventStream, FallbackBackend, FileEvent, FileEventKind, IoBackend, Platform, PlatformIO,
    PlatformInfo, WriteOp, create_platform_io, create_platform_io_with_fallback_tracking,
};

// Platform-specific backend exports (conditionally compiled)
#[cfg(target_os = "linux")]
pub use platform_io::IoUringBackend;

#[cfg(target_os = "macos")]
pub use platform_io::KqueueBackend;

#[cfg(target_os = "windows")]
pub use platform_io::IocpBackend;

// ----------------------------------------------------------------------------
// Resource Management Types
// ----------------------------------------------------------------------------
// Types for RAII-based resource handle management.

pub use resource_manager::{HandleGuard, ResourceManager};

// ----------------------------------------------------------------------------
// Configuration Types
// ----------------------------------------------------------------------------
// Types for configuration validation and management.

pub use config::{ConfigValidator, ValidationError, ValidationResult};

// ----------------------------------------------------------------------------
// DX Directory Types
// ----------------------------------------------------------------------------
// Types for .dx/ directory structure management.

pub use dx_dir::{DX_SUBDIRS, DxPaths, current_project};

// ----------------------------------------------------------------------------
// Metrics Types
// ----------------------------------------------------------------------------
// Types for metrics collection and reporting.

pub use metrics::MetricsCollector;

// ----------------------------------------------------------------------------
// Shutdown Types
// ----------------------------------------------------------------------------
// Types for graceful shutdown handling.

pub use shutdown::{ExitCode, ShutdownConfig, ShutdownHandler, ShutdownState};

// ----------------------------------------------------------------------------
// Legacy Exports (Deprecated)
// ----------------------------------------------------------------------------
// These exports are deprecated and will be removed in future versions.

#[deprecated(
    since = "0.2.0",
    note = "Use `DualWatcher` directly or the new `Forge` API instead. This alias will be removed in a future release."
)]
pub use watcher::DualWatcher as ForgeWatcher;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ============================================================================
// API FUNCTION RE-EXPORTS
// ============================================================================
//
// The 132 Eternal API Functions organized by functional domain.
// Each group provides a cohesive set of operations for a specific aspect
// of Forge functionality.
// ============================================================================

// ----------------------------------------------------------------------------
// Core Lifecycle & System Orchestration (4 functions)
// ----------------------------------------------------------------------------
// Functions for initializing, configuring, and shutting down Forge.
// Note: These are deprecated but re-exported for backward compatibility.

#[allow(deprecated)]
pub use api::lifecycle::{get_tool_context, initialize_forge, register_tool, shutdown_forge};

// ----------------------------------------------------------------------------
// Version Governance & Package Identity (6 functions)
// ----------------------------------------------------------------------------
// Functions for managing tool versions and package variants.

pub use api::version::{
    activate_package_variant, current_forge_version, declare_tool_version, enforce_exact_version,
    query_active_package_variant, require_forge_minimum,
};

// ----------------------------------------------------------------------------
// Pipeline Execution & Orchestration (7 functions)
// ----------------------------------------------------------------------------
// Functions for executing and managing tool pipelines.

pub use api::pipeline::{
    execute_pipeline, execute_tool_immediately, get_resolved_execution_order,
    restart_current_pipeline, resume_pipeline_execution, suspend_pipeline_execution,
    temporarily_override_pipeline_order,
};

// ----------------------------------------------------------------------------
// Triple-Path Reactivity Engine (21 functions)
// ----------------------------------------------------------------------------
// Functions for real-time, debounced, and idle event handling.
// Includes batch operations, debounce control, and idle detection.

pub use api::reactivity::{
    begin_batch_operation, cancel_debounce, cancel_scheduled_idle_task, configure_debounce_delay,
    configure_idle_threshold, end_batch_operation, get_debounce_delay, get_idle_threshold,
    has_pending_debounce, is_idle, is_task_scheduled, record_activity, schedule_task_for_idle_time,
    scheduled_idle_task_count, time_since_last_activity, time_since_last_event,
    trigger_debounced_event, trigger_debounced_event_with_delay, trigger_idle_event,
    trigger_idle_event_with_threshold, trigger_realtime_event,
};

// ----------------------------------------------------------------------------
// Safe File Application & Branching Decision Engine (15 functions)
// ----------------------------------------------------------------------------
// Functions for safely applying file changes with traffic branch safety.
// Note: FileChange is already exported from watcher module.

pub use api::branching::{
    ApplicationRecord, BranchColor, BranchingVote, apply_changes, apply_changes_force_unchecked,
    apply_changes_with_preapproved_votes, automatically_accept_green_conflicts,
    automatically_reject_red_conflicts, is_change_guaranteed_safe, issue_immediate_veto,
    preview_proposed_changes, prompt_review_for_yellow_conflicts, query_predicted_branch_color,
    register_permanent_branching_voter, reset_branching_engine_state,
    revert_most_recent_application, submit_branching_vote,
};

// ----------------------------------------------------------------------------
// Global Event Bus & Observability (11 functions)
// ----------------------------------------------------------------------------
// Functions for publishing and subscribing to Forge events.

pub use api::events::{
    ForgeEvent, emit_magical_config_injection, emit_package_installation_begin,
    emit_package_installation_success, emit_pipeline_completed_event, emit_pipeline_started_event,
    emit_security_violation_detected, emit_tool_completed_event, emit_tool_started_event,
    publish_event, subscribe_to_event_stream,
};

// ----------------------------------------------------------------------------
// Configuration System (16 functions)
// ----------------------------------------------------------------------------
// Functions for configuration management, injection, and validation.

pub use api::config::{
    auto_format_config_file, enable_live_config_watching, expand_config_placeholder,
    get_active_config_file_path, inject_authentication_config, inject_font_system_config,
    inject_full_config_section_at_cursor, inject_icon_system_config, inject_media_pipeline_config,
    inject_package_specific_config, inject_style_tooling_config, inject_ui_framework_config,
    jump_to_config_section, perform_config_schema_migration, provide_config_completion_suggestions,
    reload_configuration_manifest, validate_config_in_realtime,
};

// ----------------------------------------------------------------------------
// CI/CD & Workspace Orchestration (8 functions)
// ----------------------------------------------------------------------------
// Functions for CI/CD integration and monorepo workspace management.

pub use api::cicd::{
    abort_running_ci_job, broadcast_change_to_workspace, detect_workspace_root,
    list_all_workspace_members, query_current_ci_status, register_ci_stage,
    synchronize_monorepo_workspace, trigger_ci_cd_pipeline,
};

// ----------------------------------------------------------------------------
// .dx/ Directory Management (10 functions)
// ----------------------------------------------------------------------------
// Functions for managing the .dx/ directory structure and state.

pub use api::dx_directory::{
    cache_tool_offline_binary, checkout_dx_state, commit_current_dx_state,
    get_dx_binary_storage_path, get_dx_directory_path, list_dx_history, load_tool_offline_binary,
    pull_dx_state_from_remote, push_dx_state_to_remote, show_dx_state_diff,
};

// ----------------------------------------------------------------------------
// Offline-First Architecture (5 functions)
// ----------------------------------------------------------------------------
// Functions for offline operation and binary management.

pub use api::offline::{
    detect_offline_mode, download_missing_tool_binaries, force_offline_operation,
    update_tool_binary_atomically, verify_binary_integrity_and_signature,
};

// ----------------------------------------------------------------------------
// Cart System (8 functions)
// ----------------------------------------------------------------------------
// Functions for staging and committing changes in batches.

pub use api::cart::{
    CartItem, clear_cart_completely, commit_cart_immediately, commit_entire_cart,
    export_cart_as_shareable_json, get_current_cart_contents, import_cart_from_json,
    remove_specific_cart_item, stage_item_in_cart,
};

// ----------------------------------------------------------------------------
// Package Management (8 functions)
// ----------------------------------------------------------------------------
// Functions for installing, updating, and managing packages.

pub use api::packages::{
    PackageInfo, fork_existing_variant, install_package_with_variant, list_all_installed_packages,
    pin_package_to_exact_version, publish_your_variant, search_dx_package_registry,
    uninstall_package_safely, update_package_intelligently,
};

// ----------------------------------------------------------------------------
// Generated Code Governance (5 functions)
// ----------------------------------------------------------------------------
// Functions for managing DX-generated code regions.

pub use api::codegen::{
    allow_safe_manual_edit_of_generated_code, claim_full_ownership_of_file, is_region_dx_generated,
    mark_code_region_as_dx_generated, release_ownership_of_file,
};

// ----------------------------------------------------------------------------
// Developer Experience & Editor Integration (19 functions)
// ----------------------------------------------------------------------------
// Functions for IDE integration, AI suggestions, and developer tooling.

pub use api::dx_experience::{
    apply_ai_generated_completion, apply_user_accepted_suggestion, await_editor_idle_state,
    create_watcher_ignored_scratch_file, display_dx_command_palette,
    display_inline_code_suggestion, dx_global_cache_directory, execute_full_security_audit,
    generate_comprehensive_project_report, log_structured_tool_action, open_dx_explorer_sidebar,
    open_embedded_dx_terminal, open_file_and_reveal_location, path_to_forge_manifest,
    project_root_directory, request_user_attention_flash, show_onboarding_welcome_tour,
    trigger_ai_powered_suggestion, update_dx_status_bar_indicator,
};

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Initialize a new dx project.
///
/// This creates the project scaffolding using the specified template.
///
/// # Arguments
/// * `name` - The project name (will be used as the directory name)
/// * `template` - The template to use (e.g., "default", "minimal", "full")
///
/// # Errors
/// Returns an error if project creation fails.
pub fn init(name: &str, template: &str) -> anyhow::Result<()> {
    use std::fs;
    use std::path::Path;

    let project_path = Path::new(name);

    // Create project directory
    fs::create_dir_all(project_path)?;

    // Create src directory
    fs::create_dir_all(project_path.join("src"))?;
    fs::create_dir_all(project_path.join("src/pages"))?;
    fs::create_dir_all(project_path.join("src/components"))?;

    // Create dx.toml config
    let config_content = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
template = "{template}"

[build]
output = "dist"
"#
    );
    fs::write(project_path.join("dx.toml"), config_content)?;

    // Create main entry point based on template
    let main_content = match template {
        "minimal" => r#"// Minimal dx project
export default function App() {
    return <h1>Hello, dx!</h1>;
}
"#
        .to_string(),
        "full" => r#"// Full dx project with routing
import { Router, Route } from "dx/router";

export default function App() {
    return (
        <Router>
            <Route path="/" component={Home} />
            <Route path="/about" component={About} />
        </Router>
    );
}

function Home() {
    return <h1>Home</h1>;
}

function About() {
    return <h1>About</h1>;
}
"#
        .to_string(),
        _ => r#"// Default dx project
export default function App() {
    return (
        <main>
            <h1>Welcome to dx</h1>
            <p>Edit src/pages/index.dx to get started.</p>
        </main>
    );
}
"#
        .to_string(),
    };

    fs::write(project_path.join("src/pages/index.dx"), main_content)?;

    println!("✨ Created new dx project: {}", name);
    println!("📁 Template: {}", template);
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  dx dev");

    Ok(())
}
