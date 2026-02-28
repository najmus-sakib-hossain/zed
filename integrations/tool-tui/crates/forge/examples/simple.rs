//! Simple example showing how to use forge as a library
//!
//! This demonstrates the orchestrator for DX tool execution

use anyhow::Result;
use dx_forge::{DxTool, ExecutionContext, Orchestrator, ToolOutput};

// Simple example tool
struct ExampleTool;

impl DxTool for ExampleTool {
    fn name(&self) -> &str {
        "example-tool"
    }
    fn version(&self) -> &str {
        "1.0.0"
    }
    fn priority(&self) -> u32 {
        50
    }

    fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
        println!("✓ Example tool executed in: {:?}", ctx.repo_root);
        Ok(ToolOutput::success())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Forge Orchestrator - Simple Example");
    println!("Running a simple DX tool...\n");

    // Create orchestrator
    let mut orchestrator = Orchestrator::new(".")?;

    // Register and execute tool
    orchestrator.register_tool(Box::new(ExampleTool))?;
    let results = orchestrator.execute_all()?;

    println!("\n✓ Executed {} tools successfully!", results.len());

    Ok(())
}
