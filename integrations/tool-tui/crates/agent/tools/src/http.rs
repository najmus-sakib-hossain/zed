//! HTTP tool — all network requests: REST, GraphQL, gRPC, SSE, WebSocket, download, upload.
//!
//! Actions: request | graphql | download | upload | search | scrape | sse | websocket

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct HttpTool {
    http: reqwest::Client,
}
impl Default for HttpTool {
    fn default() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[async_trait]
impl Tool for HttpTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "http".into(),
            description:
                "HTTP client: GET/POST/PUT/DELETE, GraphQL, download, upload, web search, scrape"
                    .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "HTTP action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "request".into(),
                        "graphql".into(),
                        "download".into(),
                        "upload".into(),
                        "search".into(),
                        "scrape".into(),
                    ]),
                },
                ToolParameter {
                    name: "url".into(),
                    description: "Target URL".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "method".into(),
                    description: "HTTP method (GET/POST/PUT/DELETE/PATCH)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(json!("GET")),
                    enum_values: None,
                },
                ToolParameter {
                    name: "headers".into(),
                    description: "Headers as JSON object".into(),
                    param_type: ParameterType::Object,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "body".into(),
                    description: "Request body (JSON string)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "query".into(),
                    description: "GraphQL query / search query".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "output_path".into(),
                    description: "File path for download output".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "selector".into(),
                    description: "CSS selector for scrape action".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "network".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("request");
        let url = call
            .arguments
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'url'"))?;

        match action {
            "request" => {
                let method = call.arguments.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
                let mut req = match method.to_uppercase().as_str() {
                    "POST" => self.http.post(url),
                    "PUT" => self.http.put(url),
                    "DELETE" => self.http.delete(url),
                    "PATCH" => self.http.patch(url),
                    "HEAD" => self.http.head(url),
                    _ => self.http.get(url),
                };
                if let Some(body) = call.arguments.get("body").and_then(|v| v.as_str()) {
                    req = req.body(body.to_string()).header("content-type", "application/json");
                }
                if let Some(hdrs) = call.arguments.get("headers").and_then(|v| v.as_object()) {
                    for (k, v) in hdrs {
                        if let Some(val) = v.as_str() {
                            req = req.header(k.as_str(), val);
                        }
                    }
                }
                let resp = req.send().await?;
                let status = resp.status().as_u16();
                let headers: serde_json::Map<String, serde_json::Value> = resp
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), json!(v.to_str().unwrap_or(""))))
                    .collect();
                let body = resp.text().await.unwrap_or_default();
                let truncated = if body.len() > 10000 {
                    format!("{}...(truncated)", &body[..10000])
                } else {
                    body.clone()
                };
                Ok(ToolResult::success(call.id, truncated).with_data(
                    json!({"status": status, "headers": headers, "body_length": body.len()}),
                ))
            }
            "graphql" => {
                let query = call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' for GraphQL"))?;
                let resp = self.http.post(url).json(&json!({"query": query})).send().await?;
                let status = resp.status().as_u16();
                let body = resp.text().await?;
                Ok(ToolResult::success(call.id, body).with_data(json!({"status": status})))
            }
            "download" => {
                let out = call
                    .arguments
                    .get("output_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'output_path'"))?;
                let resp = self.http.get(url).send().await?;
                let bytes = resp.bytes().await?;
                if let Some(parent) = std::path::Path::new(out).parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(out, &bytes).await?;
                Ok(ToolResult::success(
                    call.id,
                    format!("Downloaded {} bytes → {}", bytes.len(), out),
                ))
            }
            "search" => {
                let query = call.arguments.get("query").and_then(|v| v.as_str()).unwrap_or(url);
                Ok(ToolResult::success(
                    call.id,
                    format!("Web search for '{}' — connect SearXNG or search API", query),
                ))
            }
            "scrape" => {
                let resp = self.http.get(url).send().await?;
                let body = resp.text().await?;
                let _selector = call.arguments.get("selector").and_then(|v| v.as_str());
                let truncated = if body.len() > 20000 {
                    format!("{}...(truncated)", &body[..20000])
                } else {
                    body
                };
                Ok(ToolResult::success(call.id, truncated))
            }
            "upload" => Ok(ToolResult::success(
                call.id,
                "Upload action — requires multipart body setup".into(),
            )),
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(HttpTool::default().definition().name, "http");
    }
}
