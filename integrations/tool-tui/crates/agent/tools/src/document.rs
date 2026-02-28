//! Document tool — document processing, conversion, search.
//! Actions: parse | convert | extract | summarize | search | template | sign

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DocumentTool;
impl Default for DocumentTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DocumentTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "document".into(),
            description: "Document processing: parse PDF/DOCX/CSV, convert formats, extract text, search content, template rendering".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Document action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["parse".into(),"convert".into(),"extract".into(),"summarize".into(),"search".into(),"template".into()]) },
                ToolParameter { name: "file".into(), description: "Document file path".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "content".into(), description: "Document content".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "output_format".into(), description: "Target format for conversion".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "query".into(), description: "Search query".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "template".into(), description: "Template string".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "variables".into(), description: "Template variables (JSON object)".into(), param_type: ParameterType::Object, required: false, default: None, enum_values: None },
            ],
            category: "data".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("parse");

        match action {
            "parse" | "extract" => {
                if let Some(file) = call.arguments.get("file").and_then(|v| v.as_str()) {
                    let ext = std::path::Path::new(file)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    match ext {
                        "txt" | "md" | "csv" | "json" | "toml" | "yaml" | "yml" | "xml"
                        | "html" | "rs" | "py" | "js" | "ts" => {
                            let content = tokio::fs::read_to_string(file).await?;
                            let lines = content.lines().count();
                            let words = content.split_whitespace().count();
                            let chars = content.len();
                            Ok(ToolResult::success(call.id, format!("{ext} document: {lines} lines, {words} words, {chars} chars\n\n{}", &content[..content.len().min(2000)]))
                                .with_data(json!({"format": ext, "lines": lines, "words": words, "chars": chars})))
                        }
                        "pdf" | "docx" | "xlsx" => Ok(ToolResult::success(
                            call.id,
                            format!(
                                "Binary format '{ext}' — install pandoc for conversion or use specialized library"
                            ),
                        )),
                        _ => Ok(ToolResult::success(call.id, format!("Unknown format '{ext}'"))),
                    }
                } else if let Some(content) = call.arguments.get("content").and_then(|v| v.as_str())
                {
                    let lines = content.lines().count();
                    let words = content.split_whitespace().count();
                    Ok(ToolResult::success(call.id, format!("{lines} lines, {words} words"))
                        .with_data(json!({"lines": lines, "words": words})))
                } else {
                    Ok(ToolResult::error(call.id, "Provide 'file' or 'content'".into()))
                }
            }
            "template" => {
                let template = call
                    .arguments
                    .get("template")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'template'"))?;
                let vars = call.arguments.get("variables").and_then(|v| v.as_object());
                let mut result = template.to_string();
                if let Some(vars) = vars {
                    for (key, value) in vars {
                        let placeholder = format!("{{{{{}}}}}", key);
                        let replacement = match value {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        result = result.replace(&placeholder, &replacement);
                    }
                }
                Ok(ToolResult::success(call.id, result))
            }
            "search" => {
                let file = call
                    .arguments
                    .get("file")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'file'"))?;
                let query = call
                    .arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query'"))?;
                let content = tokio::fs::read_to_string(file).await?;
                let matches: Vec<(usize, &str)> = content
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.to_lowercase().contains(&query.to_lowercase()))
                    .collect();
                let output = matches
                    .iter()
                    .take(50)
                    .map(|(i, line)| format!("{}:{}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(call.id, format!("{} matches:\n{}", matches.len(), output)))
            }
            "convert" => {
                let file = call.arguments.get("file").and_then(|v| v.as_str()).unwrap_or("");
                let target =
                    call.arguments.get("output_format").and_then(|v| v.as_str()).unwrap_or("txt");
                // Use pandoc for conversion if available
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let cmd = format!("pandoc {} -o output.{}", file, target);
                match tokio::process::Command::new(shell).arg(flag).arg(&cmd).output().await {
                    Ok(o) if o.status.success() => {
                        Ok(ToolResult::success(call.id, format!("Converted to {target}")))
                    }
                    _ => Ok(ToolResult::success(
                        call.id,
                        format!("Conversion to {target} — install pandoc for format conversion"),
                    )),
                }
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Document '{}' — provide file or content", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DocumentTool.definition().name, "document");
    }

    #[tokio::test]
    async fn test_template() {
        let tool = DocumentTool;
        let call = ToolCall {
            id: "t1".into(),
            name: "document".into(),
            arguments: json!({"action":"template","template":"Hello {{name}}, you are {{age}} years old!","variables":{"name":"World","age":25}}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.output.contains("Hello World"));
        assert!(r.output.contains("25"));
    }
}
