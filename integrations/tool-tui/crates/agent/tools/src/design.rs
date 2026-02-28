//! Design tool — UI/UX design system, component generation, assets.
//! Actions: component | palette | typography | spacing | icons | tokens | storybook

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DesignTool;
impl Default for DesignTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DesignTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "design".into(),
            description: "Design system: generate UI components, color palettes, typography scales, spacing systems, design tokens".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Design action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["component".into(),"palette".into(),"typography".into(),"spacing".into(),"icons".into(),"tokens".into(),"storybook".into()]) },
                ToolParameter { name: "name".into(), description: "Component/palette name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "framework".into(), description: "UI framework (react, vue, svelte, html)".into(), param_type: ParameterType::String, required: false, default: Some(json!("react")), enum_values: None },
                ToolParameter { name: "base_color".into(), description: "Base color for palette".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "project".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("tokens");

        match action {
            "palette" => {
                let base =
                    call.arguments.get("base_color").and_then(|v| v.as_str()).unwrap_or("#3b82f6");
                let palette = json!({
                    "50": lighten(base, 0.9), "100": lighten(base, 0.8), "200": lighten(base, 0.6),
                    "300": lighten(base, 0.4), "400": lighten(base, 0.2), "500": base,
                    "600": darken(base, 0.2), "700": darken(base, 0.4), "800": darken(base, 0.6),
                    "900": darken(base, 0.8), "950": darken(base, 0.9),
                });
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&palette)?)
                    .with_data(palette))
            }
            "typography" => {
                let scale = json!({
                    "xs": {"size": "0.75rem", "line_height": "1rem"},
                    "sm": {"size": "0.875rem", "line_height": "1.25rem"},
                    "base": {"size": "1rem", "line_height": "1.5rem"},
                    "lg": {"size": "1.125rem", "line_height": "1.75rem"},
                    "xl": {"size": "1.25rem", "line_height": "1.75rem"},
                    "2xl": {"size": "1.5rem", "line_height": "2rem"},
                    "3xl": {"size": "1.875rem", "line_height": "2.25rem"},
                    "4xl": {"size": "2.25rem", "line_height": "2.5rem"},
                });
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&scale)?)
                    .with_data(scale))
            }
            "spacing" => {
                let spacing = json!({"0": "0", "1": "0.25rem", "2": "0.5rem", "3": "0.75rem", "4": "1rem",
                    "5": "1.25rem", "6": "1.5rem", "8": "2rem", "10": "2.5rem", "12": "3rem",
                    "16": "4rem", "20": "5rem", "24": "6rem", "32": "8rem", "40": "10rem"});
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&spacing)?)
                    .with_data(spacing))
            }
            "tokens" => {
                let tokens = json!({
                    "colors": {"primary": "#3b82f6", "secondary": "#6366f1", "success": "#22c55e", "warning": "#f59e0b", "error": "#ef4444"},
                    "radius": {"sm": "0.125rem", "md": "0.375rem", "lg": "0.5rem", "xl": "0.75rem", "full": "9999px"},
                    "shadow": {"sm": "0 1px 2px rgba(0,0,0,0.05)", "md": "0 4px 6px rgba(0,0,0,0.1)", "lg": "0 10px 15px rgba(0,0,0,0.1)"},
                });
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&tokens)?)
                    .with_data(tokens))
            }
            "component" => {
                let name = call.arguments.get("name").and_then(|v| v.as_str()).unwrap_or("Button");
                let fw =
                    call.arguments.get("framework").and_then(|v| v.as_str()).unwrap_or("react");
                let component = match fw {
                    "react" => format!(
                        r#"interface {name}Props {{ children: React.ReactNode; variant?: 'primary' | 'secondary'; onClick?: () => void; }}

export function {name}({{ children, variant = 'primary', onClick }}: {name}Props) {{
  return <button className={{`btn btn-${{variant}}`}} onClick={{onClick}}>{{children}}</button>;
}}"#
                    ),
                    _ => format!("<!-- {name} component for {fw} -->"),
                };
                Ok(ToolResult::success(call.id, component))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Design '{}' — connect design system for full generation", action),
            )),
        }
    }
}

fn lighten(hex: &str, amount: f64) -> String {
    // Simple hex color lightening
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return format!("#{hex}");
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    let r = (r as f64 + (255.0 - r as f64) * amount) as u8;
    let g = (g as f64 + (255.0 - g as f64) * amount) as u8;
    let b = (b as f64 + (255.0 - b as f64) * amount) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn darken(hex: &str, amount: f64) -> String {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return format!("#{hex}");
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    let r = (r as f64 * (1.0 - amount)) as u8;
    let g = (g as f64 * (1.0 - amount)) as u8;
    let b = (b as f64 * (1.0 - amount)) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DesignTool.definition().name, "design");
    }
    #[test]
    fn test_lighten() {
        let l = lighten("#3b82f6", 0.5);
        assert!(l.starts_with('#'));
    }
    #[test]
    fn test_darken() {
        let d = darken("#3b82f6", 0.5);
        assert!(d.starts_with('#'));
    }
}
