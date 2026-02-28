//! Dummy Tool Implementations
//!
//! Placeholder tools that simulate real tool behavior for architecture validation.

use crate::dx_cache::DxToolId;
use crate::dx_executor::{DxToolExecutable, ExecutionContext, ToolResult};
use anyhow::Result;
use std::time::Instant;

/// A dummy tool that simulates real tool behavior
pub struct DummyTool {
    /// Tool identifier
    tool_id: DxToolId,
    /// Tool name for display
    display_name: String,
    /// Simulated execution delay in milliseconds
    delay_ms: u64,
    /// Dependencies
    deps: Vec<DxToolId>,
}

impl DummyTool {
    /// Create a new dummy tool
    pub fn new(id: DxToolId, name: &str, delay_ms: u64, deps: Vec<DxToolId>) -> Self {
        Self {
            tool_id: id,
            display_name: name.to_string(),
            delay_ms,
            deps,
        }
    }

    /// Check if this is a dummy implementation
    pub fn is_dummy(&self) -> bool {
        true
    }

    /// Get display name
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

impl DxToolExecutable for DummyTool {
    fn id(&self) -> DxToolId {
        self.tool_id
    }

    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult> {
        let start = Instant::now();
        let warm = ctx.has_warm_cache(self.tool_id);

        // Log execution
        tracing::info!(
            tool = %self.display_name,
            project = %ctx.project_root.display(),
            "[DUMMY] Executing tool"
        );

        // Simulate work with delay
        std::thread::sleep(std::time::Duration::from_millis(self.delay_ms));

        Ok(ToolResult {
            tool: self.display_name.clone(),
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            warm_start: warm,
            cache_hits: if warm { 1 } else { 0 },
            cache_misses: if warm { 0 } else { 1 },
            output_files: vec![],
            errors: vec![],
        })
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        ctx.config(self.tool_id).enabled
    }

    fn dependencies(&self) -> &[DxToolId] {
        &self.deps
    }

    fn build_cache(&self, _ctx: &ExecutionContext, _result: &ToolResult) -> Result<()> {
        // Dummy tools don't build real caches
        Ok(())
    }
}

/// Create all dummy tool instances
pub fn create_dummy_tools() -> Vec<Box<dyn DxToolExecutable>> {
    vec![
        Box::new(DummyTool::new(
            DxToolId::Bundler,
            "dx-bundler",
            100,
            vec![DxToolId::NodeModules],
        )),
        Box::new(DummyTool::new(DxToolId::Style, "dx-style", 50, vec![])),
        Box::new(DummyTool::new(
            DxToolId::Test,
            "dx-test-runner",
            200,
            vec![DxToolId::NodeModules, DxToolId::Bundler],
        )),
        Box::new(DummyTool::new(DxToolId::NodeModules, "dx-package-manager", 150, vec![])),
        Box::new(DummyTool::new(DxToolId::Serializer, "dx-serializer", 30, vec![])),
        Box::new(DummyTool::new(DxToolId::Www, "dx-www", 80, vec![])),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dx_cache::DxToolCacheManager;
    use std::path::PathBuf;
    use std::sync::Arc;

    #[test]
    fn test_dummy_tool_creation() {
        let tool = DummyTool::new(DxToolId::Bundler, "test-tool", 10, vec![]);
        assert_eq!(tool.display_name(), "test-tool");
        assert!(tool.is_dummy());
    }

    #[test]
    fn test_create_dummy_tools() {
        let tools = create_dummy_tools();
        assert_eq!(tools.len(), 6);
    }
}
