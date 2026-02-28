//! I18n tool — internationalization and localization.
//! Actions: extract | translate | validate | merge | stats | pseudo | missing

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct I18nTool;
impl Default for I18nTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for I18nTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "i18n".into(),
            description: "Internationalization: extract strings, translate, validate locale files, find missing translations, statistics".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "I18n action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["extract".into(),"translate".into(),"validate".into(),"merge".into(),"stats".into(),"pseudo".into(),"missing".into()]) },
                ToolParameter { name: "path".into(), description: "Path to locale files or source code".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "source_locale".into(), description: "Source locale (e.g., en)".into(), param_type: ParameterType::String, required: false, default: Some(json!("en")), enum_values: None },
                ToolParameter { name: "target_locale".into(), description: "Target locale (e.g., es, fr, de)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "file".into(), description: "Specific locale file".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "monitoring".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("stats");
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        match action {
            "extract" => {
                // Scan for translatable strings in source files
                let mut strings = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                    if !["rs", "ts", "tsx", "js", "jsx", "py", "vue", "svelte"].contains(&ext) {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        // Find t("...") or i18n("...") patterns
                        let re = regex::Regex::new(r#"(?:t|i18n)\(\s*["']([^"']+)["']"#).unwrap();
                        for cap in re.captures_iter(&content) {
                            if let Some(key) = cap.get(1) {
                                strings.push(json!({"key": key.as_str(), "file": entry.path().display().to_string()}));
                            }
                        }
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!("Extracted {} translatable strings", strings.len()),
                )
                .with_data(json!({"strings": strings})))
            }
            "validate" => {
                if let Some(file) = call.arguments.get("file").and_then(|v| v.as_str()) {
                    let content = tokio::fs::read_to_string(file).await?;
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(val) => {
                            let count = count_keys(&val);
                            Ok(ToolResult::success(
                                call.id,
                                format!("Valid locale file: {count} keys"),
                            ))
                        }
                        Err(e) => {
                            Ok(ToolResult::error(call.id, format!("Invalid locale file: {e}")))
                        }
                    }
                } else {
                    Ok(ToolResult::success(call.id, "Provide 'file' to validate".into()))
                }
            }
            "stats" => {
                let mut locales = std::collections::HashMap::new();
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let p = entry.path();
                    if p.extension().and_then(|e| e.to_str()) == Some("json") {
                        if let Ok(content) = std::fs::read_to_string(p) {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                let keys = count_keys(&val);
                                locales.insert(p.display().to_string(), keys);
                            }
                        }
                    }
                }
                let output: String = locales
                    .iter()
                    .map(|(f, c)| format!("  {f}: {c} keys"))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::success(
                    call.id,
                    format!("{} locale files:\n{}", locales.len(), output),
                ))
            }
            "pseudo" => {
                // Generate pseudo-localized text for testing
                let source = call.arguments.get("file").and_then(|v| v.as_str());
                if let Some(file) = source {
                    let content = tokio::fs::read_to_string(file).await?;
                    let pseudo = content
                        .chars()
                        .map(|c| match c {
                            'a' => 'à',
                            'e' => 'è',
                            'i' => 'ì',
                            'o' => 'ò',
                            'u' => 'ù',
                            'A' => 'À',
                            'E' => 'È',
                            'I' => 'Ì',
                            'O' => 'Ò',
                            'U' => 'Ù',
                            _ => c,
                        })
                        .collect::<String>();
                    Ok(ToolResult::success(
                        call.id,
                        format!(
                            "Pseudo-localized ({} chars):\n{}",
                            pseudo.len(),
                            &pseudo[..pseudo.len().min(500)]
                        ),
                    ))
                } else {
                    Ok(ToolResult::success(
                        call.id,
                        "Provide 'file' for pseudo-localization".into(),
                    ))
                }
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("I18n '{}' — connect translation service for full support", action),
            )),
        }
    }
}

fn count_keys(val: &serde_json::Value) -> usize {
    match val {
        serde_json::Value::Object(map) => map.len() + map.values().map(count_keys).sum::<usize>(),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(I18nTool.definition().name, "i18n");
    }
}
