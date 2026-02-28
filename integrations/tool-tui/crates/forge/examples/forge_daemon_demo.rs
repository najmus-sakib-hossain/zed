//! Forge Daemon Demo - Binary Dawn Edition
//!
//! Demonstrates the Forge Daemon with:
//! - Dual watchers (LSP + FileSystem)
//! - Background worker pool
//! - Tool orchestration
//! - R2 cloud sync
//!
//! Run: cargo run --example forge_daemon_demo

use anyhow::Result;
use dx_forge::{
    DaemonConfig, DaemonEvent, ForgeDaemon, LspBridge, TaskPriority, WorkerPool, WorkerTask,
};
use std::path::PathBuf;
use std::time::Duration;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════════╗");
    println!("║       ⚔️  FORGE DAEMON DEMO - Binary Dawn Edition                 ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  Dual Watcher Architecture:                                       ║");
    println!("║    • LSP Watcher (Primary) - VS Code extension integration       ║");
    println!("║    • FileSystem Watcher (Fallback) - notify-debouncer            ║");
    println!("║                                                                   ║");
    println!("║  Background Workers:                                              ║");
    println!("║    • Cache warming    • R2 sync     • Pattern analysis           ║");
    println!("║    • Package prefetch • Cleanup     • Index projects             ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!();

    // Get project root (current directory or argument)
    let project_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("📁 Project root: {}", project_root.display());
    println!();

    // ========================================================================
    // 1. Create Worker Pool
    // ========================================================================
    println!("1️⃣  Creating worker pool...");
    let worker_pool = WorkerPool::new(4);
    println!("   ✅ {} background workers ready", worker_pool.worker_count());
    println!();

    // ========================================================================
    // 2. Create LSP Bridge
    // ========================================================================
    println!("2️⃣  Creating LSP bridge for VS Code integration...");
    let lsp_bridge = LspBridge::default();
    // Note: In production, you'd call lsp_bridge.start().await to listen
    println!("   ✅ LSP Bridge ready on port 9527");
    println!("   📝 VS Code extension can connect to: 127.0.0.1:9527");
    println!();

    // ========================================================================
    // 3. Create Daemon
    // ========================================================================
    println!("3️⃣  Creating Forge Daemon...");
    let config = DaemonConfig {
        project_root: project_root.clone(),
        enable_lsp_watcher: true,
        enable_fs_watcher: true,
        debounce_ms: 100,
        worker_count: 4,
        enable_r2_sync: false,
        auto_run_tools: true,
        max_concurrent_tools: 4,
        tool_timeout_ms: 30_000,
        ..Default::default()
    };

    let daemon = ForgeDaemon::new(config)?;
    println!("   ✅ Daemon created");
    println!();

    // ========================================================================
    // 4. Subscribe to events
    // ========================================================================
    println!("4️⃣  Subscribing to daemon events...");
    let mut event_rx = daemon.subscribe();

    tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            match event {
                DaemonEvent::Started => {
                    println!("🟢 [EVENT] Daemon started");
                }
                DaemonEvent::FileChanged(change) => {
                    println!("📝 [EVENT] File changed: {:?}", change.path.file_name());
                }
                DaemonEvent::ToolStarted(tool) => {
                    println!("🔧 [EVENT] Tool started: {:?}", tool);
                }
                DaemonEvent::ToolCompleted(tool, result) => {
                    println!(
                        "✅ [EVENT] Tool completed: {:?} ({}ms, warm: {})",
                        tool, result.duration_ms, result.warm_start
                    );
                }
                DaemonEvent::ToolFailed(tool, error) => {
                    println!("❌ [EVENT] Tool failed: {:?} - {}", tool, error);
                }
                DaemonEvent::Stopped => {
                    println!("🔴 [EVENT] Daemon stopped");
                    break;
                }
                _ => {}
            }
        }
    });
    println!("   ✅ Event subscription active");
    println!();

    // ========================================================================
    // 5. Queue some background tasks
    // ========================================================================
    println!("5️⃣  Queuing background tasks...");

    worker_pool
        .queue_with_priority(
            WorkerTask::IndexProject {
                root: project_root.to_string_lossy().to_string(),
            },
            TaskPriority::High,
        )
        .await;

    worker_pool
        .queue(WorkerTask::WarmCache {
            tool: "bundler".to_string(),
        })
        .await;
    worker_pool
        .queue(WorkerTask::WarmCache {
            tool: "style".to_string(),
        })
        .await;
    worker_pool
        .queue(WorkerTask::AnalyzePatterns {
            paths: vec!["src".to_string()],
        })
        .await;

    println!("   ✅ 4 background tasks queued");
    println!();

    // ========================================================================
    // 6. Show architecture
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════════════");
    println!("                    FORGE DAEMON ARCHITECTURE");
    println!("═══════════════════════════════════════════════════════════════════");
    println!();
    println!("┌─────────────────────────────────────────────────────────────────┐");
    println!("│                     FORGE DAEMON (Binary Dawn)                   │");
    println!("├─────────────────────────────────────────────────────────────────┤");
    println!("│  ┌─────────────────────┐     ┌─────────────────────────────┐   │");
    println!("│  │   LSP Watcher       │     │   FileSystem Watcher        │   │");
    println!("│  │   (Primary)         │     │   (Fallback)                │   │");
    println!("│  │   - VS Code events  │     │   - notify-debouncer        │   │");
    println!("│  │   - Port 9527       │     │   - Recursive watch         │   │");
    println!("│  └─────────┬───────────┘     └──────────────┬──────────────┘   │");
    println!("│            │                                │                   │");
    println!("│            └──────────────┬─────────────────┘                   │");
    println!("│                           ▼                                     │");
    println!("│  ┌────────────────────────────────────────────────────────────┐│");
    println!("│  │              UNIFIED CHANGE STREAM                          ││");
    println!("│  │  (Deduplication + Pattern Detection + Priority Queue)       ││");
    println!("│  └───────────────────────────┬────────────────────────────────┘│");
    println!("│                              ▼                                  │");
    println!("│  ┌────────────────────────────────────────────────────────────┐│");
    println!("│  │              TOOL ORCHESTRATOR                              ││");
    println!("│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          ││");
    println!("│  │  │ bundler │ │ style   │ │ test    │ │ www     │ ...      ││");
    println!("│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘          ││");
    println!("│  └───────────────────────────┬────────────────────────────────┘│");
    println!("│                              ▼                                  │");
    println!("│  ┌────────────────────────────────────────────────────────────┐│");
    println!("│  │              BACKGROUND WORKER POOL (4 workers)             ││");
    println!("│  │  - Cache warming    - R2 sync     - Pattern analysis       ││");
    println!("│  │  - Package prefetch - Cleanup     - Project indexing       ││");
    println!("│  └────────────────────────────────────────────────────────────┘│");
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();

    // Wait for background tasks
    println!("⏳ Waiting for background tasks to complete...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let stats = worker_pool.stats();
    println!();
    println!("📊 Worker Pool Stats:");
    println!("   • Tasks completed: {}", stats.tasks_completed);
    println!("   • Tasks failed: {}", stats.tasks_failed);
    println!("   • Tasks queued: {}", stats.tasks_queued);
    println!("   • Busy workers: {}", stats.busy_workers);
    println!();

    // ========================================================================
    // 7. Usage instructions
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════════════");
    println!("                        USAGE INSTRUCTIONS");
    println!("═══════════════════════════════════════════════════════════════════");
    println!();
    println!("To start the daemon in watch mode:");
    println!();
    println!("  ```rust");
    println!("  use dx_forge::{{ForgeDaemon, DaemonConfig}};");
    println!();
    println!("  #[tokio::main]");
    println!("  async fn main() -> anyhow::Result<()> {{");
    println!("      let daemon = ForgeDaemon::new(DaemonConfig::default())?;");
    println!("      daemon.start().await?;  // Blocks until Ctrl+C");
    println!("      Ok(())");
    println!("  }}");
    println!("  ```");
    println!();
    println!("To integrate with VS Code extension:");
    println!();
    println!("  1. Extension connects to 127.0.0.1:9527");
    println!("  2. Sends textDocument/didChange notifications");
    println!("  3. Receives dx/toolStarted, dx/toolCompleted events");
    println!();
    println!("═══════════════════════════════════════════════════════════════════");
    println!("                        ✅ DEMO COMPLETE");
    println!("═══════════════════════════════════════════════════════════════════");

    // Cleanup
    worker_pool.stop();

    Ok(())
}
