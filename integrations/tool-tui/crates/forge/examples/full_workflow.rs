//! Complete DX Tools workflow example
//!
//! Demonstrates the full Forge system with multiple tools, version control,
//! file watching, and generated code tracking.

use anyhow::Result;
use dx_forge::{
    DxTool, ExecutionContext, Forge, ForgeConfig, Orchestrator, OrchestratorConfig,
    SnapshotManager, ToolOutput, ToolState, Version,
};
use std::collections::HashMap;
use std::path::PathBuf;

// Include example tools
mod example_tools;
use example_tools::{DxCodegenTool, DxOptimizerTool, DxStyleTool, DxUiTool};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("debug").init();

    println!("🚀 Starting Complete DX Tools Workflow\n");

    // Step 1: Initialize Forge
    println!("📦 Step 1: Initializing Forge...");
    let project_root = std::env::current_dir()?;
    let config = ForgeConfig::new(&project_root)
        .without_auto_watch() // Manual control for this example
        .with_forge_dir(project_root.join(".dx/forge"));

    let mut forge = Forge::with_config(config)?;
    println!("✅ Forge initialized at: {:?}\n", forge.project_root());

    // Step 2: Register DX Tools
    println!("📦 Step 2: Registering DX Tools...");
    let mut orch = Orchestrator::new(&project_root)?;

    // Register tools in any order (orchestrator will sort by priority)
    let components_dir = project_root.join("src/components");
    let generated_dir = project_root.join("src/generated");
    let styles_dir = project_root.join("src/styles");

    orch.register_tool(Box::new(DxOptimizerTool))?;
    orch.register_tool(Box::new(DxUiTool::new(components_dir.clone())))?;
    orch.register_tool(Box::new(DxStyleTool::new(styles_dir.clone())))?;
    orch.register_tool(Box::new(DxCodegenTool::new(
        project_root.join("schemas"),
        generated_dir.clone(),
    )))?;

    println!("✅ Registered 4 DX tools\n");

    // Step 3: Execute All Tools
    println!("📦 Step 3: Executing tools (orchestrated)...");
    let outputs = orch.execute_all()?;

    let success_count = outputs.iter().filter(|o| o.success).count();
    println!("✅ Execution complete: {}/{} tools succeeded\n", success_count, outputs.len());

    // Step 4: Track Generated Files
    println!("📦 Step 4: Tracking generated files...");
    for output in &outputs {
        for file in &output.files_created {
            let mut metadata = HashMap::new();
            metadata.insert("created_at".to_string(), chrono::Utc::now().to_string());

            // Extract tool name from output (in real impl, tools would have names)
            forge.track_generated_file(file.clone(), "dx-tool", metadata)?;
        }
    }

    let all_generated = forge.get_generated_files("dx-tool");
    println!("✅ Tracking {} generated files\n", all_generated.len());

    // Step 5: Create Version Control Snapshot
    println!("📦 Step 5: Creating version control snapshot...");
    let forge_dir = forge.forge_dir().to_path_buf();
    let mut snapshot_mgr = SnapshotManager::new(&forge_dir)?;

    let mut tool_states = HashMap::new();
    tool_states.insert(
        "dx-codegen".to_string(),
        ToolState {
            tool_name: "dx-codegen".to_string(),
            version: Version::new(1, 0, 0),
            config: HashMap::new(),
            output_files: vec![generated_dir.join("generated_types.ts")],
        },
    );
    tool_states.insert(
        "dx-ui".to_string(),
        ToolState {
            tool_name: "dx-ui".to_string(),
            version: Version::new(1, 0, 0),
            config: HashMap::new(),
            output_files: vec![components_dir.join("Button.tsx")],
        },
    );

    let snapshot_id = snapshot_mgr.create_snapshot(
        "Initial DX tools generation",
        tool_states,
        all_generated.clone(),
    )?;

    println!("✅ Created snapshot: {}\n", snapshot_id);

    // Step 6: Demonstrate Branching
    println!("📦 Step 6: Creating feature branch...");
    snapshot_mgr.create_branch("feature/new-components")?;
    snapshot_mgr.checkout_branch("feature/new-components")?;

    println!("✅ On branch: {}\n", snapshot_mgr.current_branch());

    // Step 7: View History
    println!("📦 Step 7: Version history...");
    let history = snapshot_mgr.history(5)?;
    println!("📜 Commit History:");
    for (i, snap) in history.iter().enumerate() {
        println!("  {}. {} - {}", i + 1, snap.id, snap.message);
        println!("     Author: {}", snap.author);
        println!("     Files: {}", snap.files.len());
    }
    println!();

    // Step 8: Demonstrate Parallel Execution
    println!("📦 Step 8: Testing parallel execution...");
    let parallel_config = OrchestratorConfig {
        parallel: true,
        fail_fast: false,
        max_concurrent: 4,
        traffic_branch_enabled: true,
    };

    let mut parallel_orch = Orchestrator::with_config(&project_root, parallel_config)?;
    parallel_orch.register_tool(Box::new(DxCodegenTool::new(
        project_root.join("schemas"),
        generated_dir.clone(),
    )))?;
    parallel_orch.register_tool(Box::new(DxUiTool::new(components_dir.clone())))?;

    let parallel_outputs = parallel_orch.execute_all()?;
    println!("✅ Parallel execution: {} tools completed\n", parallel_outputs.len());

    // Step 9: Traffic Branch Analysis
    println!("📦 Step 9: Traffic branch analysis...");
    use dx_forge::orchestrator::DefaultTrafficAnalyzer;
    use dx_forge::{TrafficAnalyzer, TrafficBranch};

    let analyzer = DefaultTrafficAnalyzer;
    let test_files = vec![
        "src/components/Button.tsx",
        "src/api/types.ts",
        "README.md",
        "package.json",
    ];

    for file in test_files {
        let result = analyzer.analyze(std::path::Path::new(file))?;
        let status = match result {
            TrafficBranch::Green => "🟢 Green (Auto-merge safe)",
            TrafficBranch::Yellow { .. } => "🟡 Yellow (Review required)",
            TrafficBranch::Red { .. } => "🔴 Red (Manual resolution)",
        };
        println!("  {} - {}", file, status);
    }
    println!();

    // Step 10: Cleanup Demo
    println!("📦 Step 10: Cleanup capabilities...");
    println!("Generated files can be cleaned up with:");
    println!("  forge.cleanup_generated(\"tool-name\")");
    println!("This would remove all files generated by that tool.\n");

    // Summary
    println!("🎉 Complete DX Tools Workflow Demonstration");
    println!("{}", "=".repeat(50));
    println!("✅ Forge initialized");
    println!("✅ {} tools registered and executed", outputs.len());
    println!("✅ {} files generated and tracked", all_generated.len());
    println!("✅ Version control with snapshots");
    println!("✅ Branching and history");
    println!("✅ Parallel execution support");
    println!("✅ Traffic branch analysis");
    println!("\n📚 Check docs/ for more information!");

    Ok(())
}
