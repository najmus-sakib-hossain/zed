//! Diagram tool — generate diagrams (Mermaid, PlantUML, ASCII).
//! Actions: generate | mermaid | plantuml | ascii | sequence | class_diagram | er_diagram | flowchart

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DiagramTool;
impl Default for DiagramTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DiagramTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "diagram".into(),
            description: "Generate diagrams: Mermaid, PlantUML, ASCII art. Sequence, class, ER, flowchart diagrams from code or descriptions".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Diagram action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["mermaid".into(),"plantuml".into(),"ascii".into(),"sequence".into(),"class_diagram".into(),"er_diagram".into(),"flowchart".into()]) },
                ToolParameter { name: "description".into(), description: "Diagram description".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "file".into(), description: "Source file to diagram".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "output".into(), description: "Output file path".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "project".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("mermaid");
        let description = call.arguments.get("description").and_then(|v| v.as_str());

        match action {
            "class_diagram" => {
                if let Some(file) = call.arguments.get("file").and_then(|v| v.as_str()) {
                    let content = tokio::fs::read_to_string(file).await?;
                    let mut mermaid = String::from("classDiagram\n");
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                            let name = trimmed
                                .split_whitespace()
                                .nth(if trimmed.starts_with("pub") { 2 } else { 1 })
                                .unwrap_or("Unknown");
                            let name = name.trim_end_matches(|c: char| !c.is_alphanumeric());
                            mermaid.push_str(&format!("    class {name}\n"));
                        }
                    }
                    let diagram = format!("```mermaid\n{mermaid}```");
                    if let Some(out) = call.arguments.get("output").and_then(|v| v.as_str()) {
                        tokio::fs::write(out, &diagram).await?;
                    }
                    Ok(ToolResult::success(call.id, diagram))
                } else {
                    Ok(ToolResult::success(
                        call.id,
                        "Provide 'file' for class diagram extraction".into(),
                    ))
                }
            }
            "flowchart" => {
                let desc = description.unwrap_or("Start --> Process --> End");
                let mermaid = format!("```mermaid\nflowchart TD\n    {}\n```", desc);
                Ok(ToolResult::success(call.id, mermaid))
            }
            "sequence" => {
                let desc =
                    description.unwrap_or("Client->>Server: Request\nServer->>Client: Response");
                let mermaid = format!("```mermaid\nsequenceDiagram\n    {}\n```", desc);
                Ok(ToolResult::success(call.id, mermaid))
            }
            "er_diagram" => {
                let desc = description
                    .unwrap_or("USER ||--o{ ORDER : places\nORDER ||--|{ LINE_ITEM : contains");
                let mermaid = format!("```mermaid\nerDiagram\n    {}\n```", desc);
                Ok(ToolResult::success(call.id, mermaid))
            }
            "mermaid" | "plantuml" | "ascii" => {
                let desc = description.unwrap_or("Provide 'description' for diagram generation");
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "{} diagram from description: {desc} — connect LLM for intelligent generation",
                        action
                    ),
                ))
            }
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DiagramTool.definition().name, "diagram");
    }

    #[tokio::test]
    async fn test_flowchart() {
        let tool = DiagramTool;
        let call = ToolCall {
            id: "f1".into(),
            name: "diagram".into(),
            arguments: json!({"action":"flowchart","description":"A --> B --> C"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.output.contains("mermaid"));
        assert!(r.output.contains("A --> B --> C"));
    }
}
