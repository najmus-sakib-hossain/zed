//! Example: DX Tools Orchestration Engine
//!
//! This example demonstrates how to build and coordinate multiple DX tools
//! using Forge's orchestration engine with priority-based execution,
//! dependency resolution, and traffic branch safety logic.
//!
//! Run with: cargo run --example orchestration

use anyhow::{Context, Result};
use dx_forge::{DualWatcher, DxTool, ExecutionContext, FileChange, Orchestrator, ToolOutput};
use dx_forge::{TrafficAnalyzer, TrafficBranch};
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Example DX-Style tool: manages CSS/styling
struct DxStyleTool;

impl DxTool for DxStyleTool {
    fn name(&self) -> &str {
        "dx-style"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn priority(&self) -> u32 {
        100 // High priority - styles needed first
    }

    fn dependencies(&self) -> Vec<String> {
        vec![] // No dependencies
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        // Run if any CSS or style-related files changed
        ctx.changed_files.iter().any(|f| {
            f.to_str().map(|s| s.ends_with(".css") || s.contains("style")).unwrap_or(false)
        })
    }

    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("🎨 [dx-style] Processing styles...");
        thread::sleep(Duration::from_millis(50)); // Simulate work

        for file in &ctx.changed_files {
            println!("Processed: {}", file.display());
        }

        let mut output = ToolOutput::success();
        output.message = "Styles processed successfully".to_string();
        Ok(output)
    }
}

/// Example DX-UI tool: manages UI components
struct DxUiTool;

impl DxTool for DxUiTool {
    fn name(&self) -> &str {
        "dx-ui"
    }

    fn version(&self) -> &str {
        "2.1.0"
    }

    fn priority(&self) -> u32 {
        80 // Medium-high priority
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["dx-style".to_string()] // Needs styles first
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        // Run if any component files changed or if styles changed
        ctx.changed_files.iter().any(|f| {
            f.to_str()
                .map(|s| s.contains("component") || s.ends_with(".tsx") || s.ends_with(".jsx"))
                .unwrap_or(false)
        })
    }

    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("🧩 [dx-ui] Injecting UI components...");
        thread::sleep(Duration::from_millis(100)); // Simulate work

        for file in &ctx.changed_files {
            println!("   Analyzing component file: {}", file.display());
        }

        let mut output = ToolOutput::success();
        output.message = "UI components injected successfully".to_string();
        Ok(output)
    }
}

/// Example DX-Icons tool: manages icon assets
struct DxIconsTool;

impl DxTool for DxIconsTool {
    fn name(&self) -> &str {
        "dx-icons"
    }

    fn version(&self) -> &str {
        "1.5.0"
    }

    fn priority(&self) -> u32 {
        60 // Medium priority
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["dx-ui".to_string()] // Needs UI components first
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        // Run if icon references detected in changes
        ctx.changed_files
            .iter()
            .any(|f| f.to_str().map(|s| s.contains("icon") || s.contains("dxi")).unwrap_or(false))
    }

    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("🎭 [dx-icons] Processing icon references...");
        thread::sleep(Duration::from_millis(30)); // Simulate work

        for file in &ctx.changed_files {
            println!("   Scanning for dxi* patterns in {}", file.display());
        }

        let mut output = ToolOutput::success();
        output.message = "Icons injected: dxiArrowRight, dxiCheck".to_string();
        Ok(output)
    }
}

/// Example DX-Check tool: runs validation and linting
struct DxCheckTool;

impl DxTool for DxCheckTool {
    fn name(&self) -> &str {
        "dx-check"
    }

    fn version(&self) -> &str {
        "3.0.0"
    }

    fn priority(&self) -> u32 {
        10 // Low priority - runs last
    }

    fn dependencies(&self) -> Vec<String> {
        vec![
            "dx-style".to_string(),
            "dx-ui".to_string(),
            "dx-icons".to_string(),
        ]
    }

    fn should_run(&self, _ctx: &ExecutionContext) -> bool {
        true // Always run validation
    }

    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("✅ [dx-check] Running validation...");
        thread::sleep(Duration::from_millis(80)); // Simulate work

        let file_count = ctx.changed_files.len();
        println!("   Validated {} files - all checks passed", file_count);

        let mut output = ToolOutput::success();
        output.message = format!("Validated {} files - all checks passed", file_count);
        Ok(output)
    }
}

/// Monitor file changes with the dual-watcher
async fn monitor_changes(repo_path: &str) -> Result<()> {
    println!("\n📡 Starting Dual-Watcher (LSP + File System)...");

    let mut watcher = DualWatcher::new()?;
    let mut rx = watcher.receiver();

    // Start watcher (runs LSP + file system watchers internally)
    watcher.start(repo_path).await?;

    println!("   Watching for changes (Ctrl+C to stop)...\n");

    // Process changes for 5 seconds (demo)
    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    loop {
        tokio::select! {
            Ok(change) = rx.recv() => {
                println!("   📝 Change detected: {:?}", change.path);
                println!("      Kind: {:?}, Source: {:?}", change.kind, change.source);
            }
            _ = &mut timeout => {
                println!("   (Demo timeout reached)");
                break;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 DX Forge - Orchestration Engine Demo\n");
    println!("   This example demonstrates tool coordination with:");
    println!("   - Priority-based execution order");
    println!("   - Dependency resolution");
    println!("   - Traffic branch safety logic");
    println!("   - Dual-watcher change detection\n");

    // Get repository path
    let repo_path = std::env::current_dir()
        .context("Failed to get current directory")?
        .to_str()
        .context("Invalid UTF-8 in path")?
        .to_string();

    // Create orchestrator
    println!("⚙️  Initializing Orchestrator...");
    let mut orchestrator = Orchestrator::new(&repo_path)?;

    // Register tools
    println!("   Registering DX tools:");
    orchestrator.register_tool(Box::new(DxStyleTool));
    println!("   ✓ dx-style (priority: 100)");

    orchestrator.register_tool(Box::new(DxUiTool));
    println!("   ✓ dx-ui (priority: 80, depends: dx-style)");

    orchestrator.register_tool(Box::new(DxIconsTool));
    println!("   ✓ dx-icons (priority: 60, depends: dx-ui)");

    orchestrator.register_tool(Box::new(DxCheckTool));
    println!("   ✓ dx-check (priority: 10, depends: dx-style, dx-ui, dx-icons)");

    // Execute all tools
    println!("\n🔄 Executing tools in dependency order...\n");
    match orchestrator.execute_all() {
        Ok(outputs) => {
            println!("\n✨ Execution complete! Results:\n");
            for output in outputs {
                let status = if output.success { "✅" } else { "❌" };
                println!("   {} {}", status, output.message);
                println!("      Modified: {}", output.files_modified.len());
                println!("      Created: {}", output.files_created.len());
                println!("      Deleted: {}", output.files_deleted.len());
                println!("      ⏱️  Execution time: {}ms", output.duration_ms);
                println!();
            }
        }
        Err(e) => {
            eprintln!("❌ Orchestration failed: {}", e);
            return Err(e);
        }
    }

    // Demonstrate traffic branch analysis
    println!("🚦 Traffic Branch Analysis:");
    let analyzer = dx_forge::orchestrator::DefaultTrafficAnalyzer;
    let branch = analyzer.analyze(Path::new("src/components/Button.tsx"))?;

    match branch {
        TrafficBranch::Green => println!("   🟢 Green: Safe to auto-update"),
        TrafficBranch::Yellow { conflicts } => {
            println!("   🟡 Yellow: Merge required");
            for conflict in conflicts {
                println!("      - {}", conflict.reason);
            }
        }
        TrafficBranch::Red { conflicts } => {
            println!("   🔴 Red: Manual resolution required");
            for conflict in conflicts {
                println!("      - {}", conflict.reason);
            }
        }
    }

    // Start file watcher (for 5 seconds)
    if let Err(e) = monitor_changes(&repo_path).await {
        eprintln!("Watcher demo failed: {}", e);
    }

    println!("\n🎉 Demo complete!\n");
    println!("Next steps:");
    println!("  - Create tool manifests in tools/ directory");
    println!("  - Implement DxTool trait for your tools");
    println!("  - Configure execution order in orchestration.toml");
    println!("  - Run: forge orchestrate --watch\n");

    Ok(())
}
