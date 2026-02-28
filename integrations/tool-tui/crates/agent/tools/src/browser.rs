//! Browser automation tool backed by dx-agent-browser.

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use dx_agent_browser::{BrowserConfig, BrowserController};

use crate::definition::*;

pub struct BrowserTool {
    controller: Arc<Mutex<BrowserController>>,
}

impl BrowserTool {
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            controller: Arc::new(Mutex::new(BrowserController::new(config))),
        }
    }
}

impl Default for BrowserTool {
    fn default() -> Self {
        Self::new(BrowserConfig::default())
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser".into(),
            description:
                "Control browser automation (launch, navigate, click, type, screenshot, cookies)"
                    .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Browser action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "launch".into(),
                        "navigate".into(),
                        "screenshot".into(),
                        "execute_js".into(),
                        "click".into(),
                        "type_text".into(),
                        "get_text".into(),
                        "get_html".into(),
                        "set_cookie".into(),
                        "get_cookies".into(),
                        "clear_cookies".into(),
                        "close_page".into(),
                        "close".into(),
                    ]),
                },
                ToolParameter {
                    name: "args".into(),
                    description: "Action arguments object".into(),
                    param_type: ParameterType::Object,
                    required: false,
                    default: Some(serde_json::json!({})),
                    enum_values: None,
                },
            ],
            category: "browser".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call
            .arguments
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action'"))?;

        let args = call.arguments.get("args").cloned().unwrap_or_else(|| serde_json::json!({}));

        match action {
            "launch" => {
                let mut controller = self.controller.lock().await;
                controller.launch().await?;
                Ok(ToolResult::success(call.id, "browser launched".into()))
            }
            "navigate" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.url is required"))?;
                let controller = self.controller.lock().await;
                let page = controller.navigate(url).await?;
                Ok(ToolResult::success(call.id, format!("navigated: {}", page.url))
                    .with_data(serde_json::to_value(page)?))
            }
            "screenshot" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let controller = self.controller.lock().await;
                let bytes = controller.screenshot(page_index).await?;
                let encoded =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
                Ok(ToolResult::success(
                    call.id,
                    format!("screenshot captured ({} bytes)", bytes.len()),
                )
                .with_data(serde_json::json!({ "base64_png": encoded })))
            }
            "execute_js" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let script = args
                    .get("script")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.script is required"))?;
                let controller = self.controller.lock().await;
                let result = controller.execute_js(page_index, script).await?;
                Ok(ToolResult::success(call.id, "javascript executed".into()).with_data(result))
            }
            "click" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.selector is required"))?;
                let controller = self.controller.lock().await;
                controller.click(page_index, selector).await?;
                Ok(ToolResult::success(call.id, "click completed".into()))
            }
            "type_text" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.selector is required"))?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.text is required"))?;
                let controller = self.controller.lock().await;
                controller.type_text(page_index, selector, text).await?;
                Ok(ToolResult::success(call.id, "typing completed".into()))
            }
            "get_text" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.selector is required"))?;
                let controller = self.controller.lock().await;
                let text = controller.get_text(page_index, selector).await?;
                Ok(ToolResult::success(call.id, text))
            }
            "get_html" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let controller = self.controller.lock().await;
                let html = controller.get_html(page_index).await?;
                Ok(ToolResult::success(call.id, html))
            }
            "set_cookie" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.name is required"))?;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("args.value is required"))?;
                let path = args.get("path").and_then(|v| v.as_str());
                let controller = self.controller.lock().await;
                controller.set_cookie(page_index, name, value, path).await?;
                Ok(ToolResult::success(call.id, "cookie set".into()))
            }
            "get_cookies" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let controller = self.controller.lock().await;
                let cookies = controller.get_cookies(page_index).await?;
                Ok(ToolResult::success(call.id, format!("{} cookies", cookies.len()))
                    .with_data(serde_json::to_value(cookies)?))
            }
            "clear_cookies" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let controller = self.controller.lock().await;
                controller.clear_cookies(page_index).await?;
                Ok(ToolResult::success(call.id, "cookies cleared".into()))
            }
            "close_page" => {
                let page_index =
                    args.get("page_index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let controller = self.controller.lock().await;
                controller.close_page(page_index).await?;
                Ok(ToolResult::success(call.id, "page closed".into()))
            }
            "close" => {
                let mut controller = self.controller.lock().await;
                controller.close().await?;
                Ok(ToolResult::success(call.id, "browser closed".into()))
            }
            _ => Ok(ToolResult::error(call.id, format!("Unknown browser action: {}", action))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_definition() {
        let tool = BrowserTool::default();
        let def = tool.definition();
        assert_eq!(def.name, "browser");
        assert_eq!(def.category, "browser");
    }

    #[tokio::test]
    async fn unknown_action_returns_error() {
        let tool = BrowserTool::default();
        let call = ToolCall {
            id: "t1".into(),
            name: "browser".into(),
            arguments: serde_json::json!({"action":"unknown"}),
        };

        let result = tool.execute(call).await.unwrap();
        assert!(!result.success);
    }
}
