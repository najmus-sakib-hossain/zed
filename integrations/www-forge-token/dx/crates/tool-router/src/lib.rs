//! # tool-router
//!
//! Dynamically selects relevant tools per turn instead of sending all 50+.
//! Fewer tools = smaller prefix = better cache hits.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - 50 tools, send only 5 relevant = 90% schema token savings
//! - Fewer tools improve prefix cache hit rates too
//! - **Honest savings: 50-90% on schema tokens**
//! - Caveat: keyword-based routing is fragile; future: use classifier
//!
//! STAGE: PromptAssembly (priority 15)

use dx_core::*;
use std::collections::HashSet;
use std::sync::Mutex;

/// Configuration for tool routing.
#[derive(Debug, Clone)]
pub struct ToolRouterConfig {
    /// Tools that are ALWAYS included regardless of context
    pub core_tools: Vec<String>,
    /// Maximum number of tools to include per turn
    pub max_tools: usize,
    /// Keyword → tool name mappings for context-based selection
    pub keyword_routes: Vec<(Vec<String>, Vec<String>)>,
    /// Whether to include tools mentioned by name in conversation
    pub include_mentioned: bool,
}

impl Default for ToolRouterConfig {
    fn default() -> Self {
        Self {
            core_tools: vec![
                "read_file".into(),
                "write_file".into(),
                "run_in_terminal".into(),
                "grep_search".into(),
                "list_dir".into(),
            ],
            max_tools: 15,
            keyword_routes: vec![
                (
                    vec!["test".into(), "spec".into(), "assert".into()],
                    vec!["run_tests".into(), "test_failure".into()],
                ),
                (
                    vec!["git".into(), "commit".into(), "branch".into(), "merge".into()],
                    vec!["git_diff".into(), "git_log".into(), "git_status".into()],
                ),
                (
                    vec!["database".into(), "sql".into(), "query".into(), "migration".into()],
                    vec!["run_sql".into(), "db_schema".into()],
                ),
                (
                    vec!["docker".into(), "container".into(), "image".into()],
                    vec!["docker_build".into(), "docker_run".into()],
                ),
                (
                    vec!["search".into(), "find".into(), "where".into()],
                    vec!["semantic_search".into(), "file_search".into()],
                ),
                (
                    vec!["browser".into(), "web".into(), "url".into(), "http".into()],
                    vec!["open_browser".into(), "fetch_url".into()],
                ),
            ],
            include_mentioned: true,
        }
    }
}

pub struct ToolRouterSaver {
    config: ToolRouterConfig,
    report: Mutex<TokenSavingsReport>,
}

impl ToolRouterSaver {
    pub fn new() -> Self {
        Self::with_config(ToolRouterConfig::default())
    }

    pub fn with_config(config: ToolRouterConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Figure out which tools are relevant for this turn.
    fn select_tools<'a>(&self, messages: &[Message], available: &'a [ToolSchema]) -> Vec<&'a ToolSchema> {
        let mut selected_names: HashSet<String> = HashSet::new();

        // Always include core tools
        for name in &self.config.core_tools {
            selected_names.insert(name.clone());
        }

        // Analyze recent messages for context
        let recent_text: String = messages.iter().rev().take(4)
            .map(|m| m.content.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");

        // Keyword-based routing
        for (keywords, tools) in &self.config.keyword_routes {
            if keywords.iter().any(|kw| recent_text.contains(kw.as_str())) {
                for tool in tools {
                    selected_names.insert(tool.clone());
                }
            }
        }

        // Include tools mentioned by name in conversation
        if self.config.include_mentioned {
            for tool in available {
                if recent_text.contains(&tool.name.to_lowercase()) {
                    selected_names.insert(tool.name.clone());
                }
            }
        }

        // Collect matching tools and cap at max
        let mut result: Vec<&ToolSchema> = available.iter()
            .filter(|t| selected_names.contains(&t.name))
            .collect();

        // If we're under max, add more tools by token cost (cheapest first)
        if result.len() < self.config.max_tools {
            let mut remaining: Vec<&ToolSchema> = available.iter()
                .filter(|t| !selected_names.contains(&t.name))
                .collect();
            remaining.sort_by_key(|t| t.token_count);
            for tool in remaining {
                if result.len() >= self.config.max_tools {
                    break;
                }
                result.push(tool);
            }
        }

        result.truncate(self.config.max_tools);
        result
    }
}

#[async_trait::async_trait]
impl TokenSaver for ToolRouterSaver {
    fn name(&self) -> &str { "tool-router" }
    fn stage(&self) -> SaverStage { SaverStage::PromptAssembly }
    fn priority(&self) -> u32 { 15 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.tools.iter().map(|t| t.token_count).sum();
        let total_tools = input.tools.len();

        let selected = self.select_tools(&input.messages, &input.tools);
        let selected_tools: Vec<ToolSchema> = selected.into_iter().cloned().collect();
        let tokens_after: usize = selected_tools.iter().map(|t| t.token_count).sum();
        let tools_kept = selected_tools.len();

        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "tool-router".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Routed {}/{} tools: {} → {} schema tokens ({:.1}% saved). \
                 Fewer tools = smaller prefix = better cache hits.",
                tools_kept, total_tools, tokens_before, tokens_after, pct
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: input.messages,
            tools: selected_tools,
            images: input.images,
            skipped: false,
            cached_response: None,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, tokens: usize) -> ToolSchema {
        ToolSchema { name: name.into(), description: format!("{} desc", name), parameters: serde_json::json!({}), token_count: tokens }
    }
    fn user_msg(content: &str) -> Message {
        Message { role: "user".into(), content: content.into(), images: vec![], tool_call_id: None, token_count: content.len() / 4 }
    }

    #[tokio::test]
    async fn test_routes_relevant_tools() {
        let config = ToolRouterConfig {
            max_tools: 5,
            ..Default::default()
        };
        let saver = ToolRouterSaver::with_config(config);
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![user_msg("run the test suite and check for failures")],
            tools: (0..50).map(|i| tool(&format!("tool_{}", i), 100)).collect(),
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(out.tools.len() <= 5);
        assert!(saver.last_savings().tokens_saved > 0);
    }
}
