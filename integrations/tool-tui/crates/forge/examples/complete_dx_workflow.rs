//! Complete DX Forge Production Example
//!
//! Demonstrates all production features:
//! - Tool version management
//! - Pattern detection
//! - Component injection
//! - Traffic branch safety
//! - Error handling with retry
//! - Lifecycle hooks
//!
//! Run with: cargo run --example complete_dx_workflow

use anyhow::Result;
use dx_forge::{
    DualWatcher, DxTool, DxToolType, ExecutionContext, InjectionManager, Orchestrator,
    OrchestratorConfig, PatternDetector, ToolOutput, ToolRegistry, ToolSource, Version,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::{Duration, sleep};

/// Example DX-UI tool with full production features
struct ProductionDxUiTool {
    version: Version,
    processed_files: Vec<PathBuf>,
}

impl ProductionDxUiTool {
    fn new() -> Self {
        Self {
            version: Version::new(2, 1, 0),
            processed_files: Vec::new(),
        }
    }
}

impl DxTool for ProductionDxUiTool {
    fn name(&self) -> &str {
        "dx-ui"
    }

    fn version(&self) -> &str {
        "2.1.0"
    }

    fn priority(&self) -> u32 {
        80
    }

    fn dependencies(&self) -> Vec<String> {
        vec!["dx-style".to_string()]
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        // Check if any JS/TS files changed
        ctx.changed_files.iter().any(|f| {
            f.extension()
                .and_then(|e| e.to_str())
                .map(|e| matches!(e, "js" | "jsx" | "ts" | "tsx"))
                .unwrap_or(false)
        })
    }

    fn before_execute(&mut self, ctx: &ExecutionContext) -> Result<()> {
        println!("🔧 {} preparing execution...", self.name());
        println!("   📂 Changed files: {}", ctx.changed_files.len());
        Ok(())
    }

    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("🎨 Processing UI components...");

        let mut output = ToolOutput::success();

        // Simulate pattern detection and injection
        for file in &ctx.changed_files {
            if let Some(ext) = file.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "tsx" | "jsx") {
                    println!("   📄 Analyzing: {}", file.display());
                    self.processed_files.push(file.clone());
                    output.files_modified.push(file.clone());
                }
            }
        }

        output.message = format!("Processed {} UI files", self.processed_files.len());
        Ok(output)
    }

    fn after_execute(&mut self, _ctx: &ExecutionContext, output: &ToolOutput) -> Result<()> {
        if output.success {
            println!("✨ {} completed successfully", self.name());
            println!("   Modified: {} files", output.files_modified.len());
        }
        Ok(())
    }

    fn on_error(&mut self, _ctx: &ExecutionContext, error: &anyhow::Error) -> Result<()> {
        println!("❌ {} encountered error: {}", self.name(), error);
        println!("   Rolling back changes...");
        self.processed_files.clear();
        Ok(())
    }

    fn timeout_seconds(&self) -> u64 {
        30
    }
}

/// Example DX-Style tool
struct ProductionDxStyleTool;

impl DxTool for ProductionDxStyleTool {
    fn name(&self) -> &str {
        "dx-style"
    }

    fn version(&self) -> &str {
        "3.0.0"
    }

    fn priority(&self) -> u32 {
        100 // Higher priority - runs first
    }

    fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("🎨 Processing styles...");
        sleep(Duration::from_millis(100));

        Ok(ToolOutput {
            success: true,
            files_modified: vec![],
            files_created: vec![],
            files_deleted: vec![],
            message: "Styles processed".to_string(),
            duration_ms: 100,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 DX Forge - Complete Production Workflow Demo\n");
    println!("{}\n", "=".repeat(60));

    // Step 1: Initialize Tool Registry
    println!("📦 Step 1: Tool Version Management");
    println!("{}", "-".repeat(60));

    let forge_dir = std::env::current_dir()?.join(".dx/forge");
    std::fs::create_dir_all(&forge_dir)?;

    let mut registry = ToolRegistry::new(&forge_dir)?;

    // Register tools
    registry.register(
        "dx-ui".to_string(),
        Version::new(2, 1, 0),
        ToolSource::Crate {
            version: "2.1.0".to_string(),
        },
        HashMap::new(),
    )?;

    registry.register(
        "dx-style".to_string(),
        Version::new(3, 0, 0),
        ToolSource::Crate {
            version: "3.0.0".to_string(),
        },
        HashMap::new(),
    )?;

    println!("✅ Registered tools:");
    for tool in registry.list() {
        println!("   • {} v{}", tool.name, tool.version);
    }
    println!();

    // Step 2: Pattern Detection
    println!("🔍 Step 2: Pattern Detection");
    println!("{}", "-".repeat(60));

    let detector = PatternDetector::new()?;
    let sample_code = r#"
        import React from 'react';
        
        export function MyComponent() {
            return (
                <div>
                    <dxButton variant="primary">Click Me</dxButton>
                    <dxiHome size={24} />
                    <dxfRoboto>Hello World</dxfRoboto>
                </div>
            );
        }
    "#;

    let matches = detector.detect_in_file(&PathBuf::from("sample.tsx"), sample_code)?;
    println!("✅ Detected {} dx patterns:", matches.len());
    for m in &matches {
        println!("   • {} ({}): {}", m.pattern, m.tool.tool_name(), m.component_name);
    }
    println!();

    // Step 3: Component Injection
    println!("💉 Step 3: Component Injection");
    println!("{}", "-".repeat(60));

    let mut injection_mgr = InjectionManager::new(&forge_dir)?;

    println!("📥 Fetching components...");
    for m in &matches {
        if m.tool == DxToolType::Ui {
            let component = injection_mgr.fetch_component(&m.tool, &m.component_name, None).await?;

            println!("   ✅ Cached {} ({} bytes)", m.component_name, component.len());
        }
    }

    let stats = injection_mgr.cache_stats();
    println!("\n📊 Cache Statistics:");
    println!("   Total components: {}", stats.total_components);
    println!("   Total size: {} bytes", stats.total_size_bytes);
    println!();

    // Step 4: Orchestrator Setup
    println!("⚙️  Step 4: Orchestrator Configuration");
    println!("{}", "-".repeat(60));

    let config = OrchestratorConfig {
        parallel: false,
        fail_fast: true,
        max_concurrent: 4,
        traffic_branch_enabled: true,
    };

    let mut orchestrator = Orchestrator::with_config(".", config)?;

    // Add changed files to context
    orchestrator
        .context_mut()
        .changed_files
        .push(PathBuf::from("src/components/Button.tsx"));
    orchestrator
        .context_mut()
        .changed_files
        .push(PathBuf::from("src/styles/main.css"));

    println!("✅ Configuration:");
    println!("   • Fail fast: enabled");
    println!("   • Traffic branch: enabled");
    println!("   • Max concurrent: 4");
    println!();

    // Step 5: Tool Registration
    println!("📋 Step 5: Tool Registration");
    println!("{}", "-".repeat(60));

    orchestrator.register_tool(Box::new(ProductionDxStyleTool))?;
    orchestrator.register_tool(Box::new(ProductionDxUiTool::new()))?;

    println!();

    // Step 6: Execute All Tools
    println!("🚀 Step 6: Tool Execution");
    println!("{}", "-".repeat(60));

    let outputs = orchestrator.execute_all()?;

    println!("\n✨ Execution Complete!");
    println!("{}", "=".repeat(60));
    println!("\n📈 Summary:");

    let total_duration: u64 = outputs.iter().map(|o| o.duration_ms).sum();
    let successful = outputs.iter().filter(|o| o.success).count();

    println!("   • Total tools: {}", outputs.len());
    println!("   • Successful: {}", successful);
    println!("   • Total duration: {}ms", total_duration);
    println!();

    for output in &outputs {
        let status = if output.success { "✅" } else { "❌" };
        println!("   {} {}: {}", status, output.message, output.duration_ms);
    }

    println!("\n🎉 Complete workflow finished successfully!");

    Ok(())
}
