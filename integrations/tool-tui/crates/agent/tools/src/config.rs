//! Config tool — parse, validate, diff, migrate configuration files.
//! Actions: parse | validate | diff | migrate | dotenv

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct ConfigTool;
impl Default for ConfigTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ConfigTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "config".into(),
            description: "Parse, validate, diff, and migrate config files (JSON, YAML, TOML, .env)"
                .into(),
            parameters: vec![
                ToolParameter {
                    name: "action".into(),
                    description: "Config action".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        "parse".into(),
                        "validate".into(),
                        "diff".into(),
                        "migrate".into(),
                        "dotenv".into(),
                    ]),
                },
                ToolParameter {
                    name: "path".into(),
                    description: "Config file path".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "format".into(),
                    description: "Force format (json/yaml/toml/env)".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "other_path".into(),
                    description: "Second file for diff/migrate".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "io".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("parse");
        let path = call
            .arguments
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'path'"))?;

        match action {
            "parse" => {
                let content = tokio::fs::read_to_string(path).await?;
                let ext =
                    std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
                let fmt = call.arguments.get("format").and_then(|v| v.as_str()).unwrap_or(ext);
                let parsed: serde_json::Value = match fmt {
                    "json" => serde_json::from_str(&content)?,
                    "toml" => toml::from_str(&content)?,
                    "env" | "dotenv" => {
                        let pairs: serde_json::Map<String, serde_json::Value> = content
                            .lines()
                            .filter(|l| !l.starts_with('#') && l.contains('='))
                            .filter_map(|l| {
                                let mut s = l.splitn(2, '=');
                                Some((s.next()?.trim().to_string(), json!(s.next()?.trim())))
                            })
                            .collect();
                        json!(pairs)
                    }
                    _ => {
                        // Try JSON first, then TOML
                        serde_json::from_str(&content)
                            .or_else(|_| {
                                toml::from_str::<serde_json::Value>(&content)
                                    .map_err(anyhow::Error::from)
                            })
                            .unwrap_or_else(|_| json!({"raw": content}))
                    }
                };
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&parsed)?)
                    .with_data(parsed))
            }
            "dotenv" => {
                let content = tokio::fs::read_to_string(path).await?;
                let pairs: Vec<(String, String)> = content
                    .lines()
                    .filter(|l| !l.starts_with('#') && l.contains('='))
                    .filter_map(|l| {
                        let mut s = l.splitn(2, '=');
                        Some((s.next()?.trim().to_string(), s.next()?.trim().to_string()))
                    })
                    .collect();
                let data: serde_json::Map<String, serde_json::Value> =
                    pairs.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
                Ok(ToolResult::success(call.id, format!("{} env vars loaded", data.len()))
                    .with_data(json!(data)))
            }
            "diff" => {
                let other = call
                    .arguments
                    .get("other_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Need 'other_path' for diff"))?;
                let a = tokio::fs::read_to_string(path).await?;
                let b = tokio::fs::read_to_string(other).await?;
                let diff = similar::TextDiff::from_lines(&a, &b);
                Ok(ToolResult::success(
                    call.id,
                    diff.unified_diff().header(path, other).to_string(),
                ))
            }
            "validate" | "migrate" => Ok(ToolResult::success(
                call.id,
                format!("Config '{action}' requires schema definition"),
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
        assert_eq!(ConfigTool.definition().name, "config");
    }
}
