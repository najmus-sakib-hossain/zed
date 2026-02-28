//! Tool registry for managing available tools.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

use crate::definition::{Tool, ToolCall, ToolDefinition, ToolResult};

/// Registry that holds all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a new tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name.clone();
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// List all tool definitions (for LLM function calling)
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.is_available())
            .map(|t| t.definition())
            .collect()
    }

    /// Execute a tool call
    pub async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let tool = self
            .tools
            .get(&call.name)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", call.name))?;

        if !tool.is_available() {
            return Ok(ToolResult::error(
                call.id.clone(),
                format!("Tool '{}' is not available", call.name),
            ));
        }

        tool.execute(call).await
    }

    /// Number of registered tools
    pub fn count(&self) -> usize {
        self.tools.len()
    }

    /// Get tools by category
    pub fn by_category(&self, category: &str) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|t| t.definition().category == category)
            .map(|t| t.definition())
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        let mut r = Self::new();
        r.register_all();
        r
    }
}

impl ToolRegistry {
    /// Register all 50 built-in tools.
    pub fn register_all(&mut self) {
        use std::sync::Arc;

        // I/O & System (8)
        self.register(Arc::new(crate::file::FileTool::default()));
        self.register(Arc::new(crate::search::SearchTool::new(".")));
        self.register(Arc::new(crate::shell::ShellTool::default()));
        self.register(Arc::new(crate::sandbox::SandboxTool::default()));
        self.register(Arc::new(crate::system::SystemTool::default()));
        self.register(Arc::new(crate::http::HttpTool::default()));
        self.register(Arc::new(crate::network::NetworkTool::default()));
        self.register(Arc::new(crate::config::ConfigTool::default()));

        // Browser & Desktop (2)
        self.register(Arc::new(crate::browser::BrowserTool::default()));
        self.register(Arc::new(crate::desktop::DesktopTool::default()));

        // Version Control (2)
        self.register(Arc::new(crate::git::GitTool::default()));
        self.register(Arc::new(crate::github::GithubTool::default()));

        // Code Intelligence (7)
        self.register(Arc::new(crate::lsp::LspTool::default()));
        self.register(Arc::new(crate::ast::AstTool::default()));
        self.register(Arc::new(crate::analyze::AnalyzeTool::default()));
        self.register(Arc::new(crate::refactor::RefactorTool::default()));
        self.register(Arc::new(crate::format::FormatTool::default()));
        self.register(Arc::new(crate::lint::LintTool::default()));
        self.register(Arc::new(crate::review::ReviewTool::default()));

        // Execution & Quality (4)
        self.register(Arc::new(crate::testing::TestTool::default()));
        self.register(Arc::new(crate::debug::DebugTool::default()));
        self.register(Arc::new(crate::profile::ProfileTool::default()));
        self.register(Arc::new(crate::experiment::ExperimentTool::default()));

        // Data & Storage (3)
        self.register(Arc::new(crate::database::DatabaseTool::default()));
        self.register(Arc::new(crate::data::DataTool::default()));
        self.register(Arc::new(crate::document::DocumentTool::default()));

        // AI & Memory (5)
        self.register(Arc::new(crate::memory::MemoryTool::default()));
        self.register(Arc::new(crate::context::ContextTool::default()));
        self.register(Arc::new(crate::llm::LlmTool::default()));
        self.register(Arc::new(crate::agent::AgentTool::default()));
        self.register(Arc::new(crate::spawn::SpawnTool::default()));

        // Infrastructure (5)
        self.register(Arc::new(crate::docker::DockerTool::default()));
        self.register(Arc::new(crate::kubernetes::KubernetesTool::default()));
        self.register(Arc::new(crate::infra::InfraTool::default()));
        self.register(Arc::new(crate::package::PackageTool::default()));
        self.register(Arc::new(crate::security::SecurityTool::default()));

        // Project & Docs (4)
        self.register(Arc::new(crate::project::ProjectTool::default()));
        self.register(Arc::new(crate::docs::DocsTool::default()));
        self.register(Arc::new(crate::diagram::DiagramTool::default()));
        self.register(Arc::new(crate::design::DesignTool::default()));

        // Communication (2)
        self.register(Arc::new(crate::notify::NotifyTool::default()));
        self.register(Arc::new(crate::tracker::TrackerTool::default()));

        // Workflow & Deployment (2)
        self.register(Arc::new(crate::workflow::WorkflowTool::default()));
        self.register(Arc::new(crate::deploy::DeployTool::default()));

        // Monitoring & Specialized (5)
        self.register(Arc::new(crate::monitor::MonitorTool::default()));
        self.register(Arc::new(crate::media::MediaTool::default()));
        self.register(Arc::new(crate::i18n::I18nTool::default()));
        self.register(Arc::new(crate::compliance::ComplianceTool::default()));
        self.register(Arc::new(crate::migrate::MigrateTool::default()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::*;
    use async_trait::async_trait;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition {
                name: "echo".into(),
                description: "Echo input back".into(),
                parameters: vec![ToolParameter {
                    name: "text".into(),
                    description: "Text to echo".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                }],
                category: "test".into(),
                requires_confirmation: false,
            }
        }

        async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
            let text = call.arguments.get("text").and_then(|v| v.as_str()).unwrap_or("(empty)");
            Ok(ToolResult::success(call.id, text.to_string()))
        }
    }

    #[tokio::test]
    async fn test_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(EchoTool));

        assert_eq!(registry.count(), 1);
        assert_eq!(registry.definitions().len(), 1);

        let call = ToolCall {
            id: "1".into(),
            name: "echo".into(),
            arguments: serde_json::json!({"text": "hello"}),
        };

        let result = registry.execute(call).await.unwrap();
        assert!(result.success);
        assert_eq!(result.output, "hello");
    }
}
