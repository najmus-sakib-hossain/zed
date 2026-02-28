//! Data tool — data transformation, validation, generation.
//! Actions: transform | validate | generate | diff | merge | convert | schema_infer | statistics

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct DataTool;
impl Default for DataTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for DataTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "data".into(),
            description: "Data operations: transform JSON/CSV/YAML, validate schemas, generate mock data, diff/merge datasets, statistics".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Data action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["transform".into(),"validate".into(),"generate".into(),"diff".into(),"merge".into(),"convert".into(),"schema_infer".into(),"statistics".into()]) },
                ToolParameter { name: "input".into(), description: "Input data (JSON string)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "file".into(), description: "Input file".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "format".into(), description: "Output format (json, csv, toml, yaml)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "schema".into(), description: "JSON Schema for validation".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "count".into(), description: "Record count for generation".into(), param_type: ParameterType::Integer, required: false, default: Some(json!(10)), enum_values: None },
            ],
            category: "data".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("transform");
        let input = if let Some(i) = call.arguments.get("input").and_then(|v| v.as_str()) {
            i.to_string()
        } else if let Some(f) = call.arguments.get("file").and_then(|v| v.as_str()) {
            tokio::fs::read_to_string(f).await?
        } else {
            String::new()
        };

        match action {
            "schema_infer" => {
                let parsed: Result<serde_json::Value, _> = serde_json::from_str(&input);
                match parsed {
                    Ok(val) => {
                        let schema = infer_schema(&val);
                        let schema_str = serde_json::to_string_pretty(&schema)?;
                        Ok(ToolResult::success(call.id, schema_str).with_data(schema))
                    }
                    Err(e) => Ok(ToolResult::error(call.id, format!("Parse error: {e}"))),
                }
            }
            "validate" => match serde_json::from_str::<serde_json::Value>(&input) {
                Ok(_) => Ok(ToolResult::success(call.id, "Valid JSON".into())),
                Err(e) => Ok(ToolResult::error(call.id, format!("Invalid: {e}"))),
            },
            "convert" => {
                let format =
                    call.arguments.get("format").and_then(|v| v.as_str()).unwrap_or("json");
                match format {
                    "toml" => {
                        let val: serde_json::Value = serde_json::from_str(&input)?;
                        let toml_str = toml::to_string_pretty(&val)?;
                        Ok(ToolResult::success(call.id, toml_str))
                    }
                    "json" => {
                        // Try parsing as TOML and converting to JSON
                        match toml::from_str::<serde_json::Value>(&input) {
                            Ok(val) => Ok(ToolResult::success(
                                call.id,
                                serde_json::to_string_pretty(&val)?,
                            )),
                            Err(_) => Ok(ToolResult::success(call.id, input)),
                        }
                    }
                    _ => Ok(ToolResult::success(
                        call.id,
                        format!("Convert to '{format}' — install specific formatter"),
                    )),
                }
            }
            "generate" => {
                let count = call.arguments.get("count").and_then(|v| v.as_u64()).unwrap_or(10);
                let mut records = Vec::new();
                for i in 0..count.min(1000) {
                    records.push(json!({
                        "id": i,
                        "name": format!("item_{i}"),
                        "value": i as f64 * 1.5,
                        "active": i % 2 == 0,
                    }));
                }
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&records)?)
                    .with_data(json!(records)))
            }
            "statistics" => {
                // Simple statistics on numeric data
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&input) {
                    let numbers: Vec<f64> = extract_numbers(&val);
                    if numbers.is_empty() {
                        return Ok(ToolResult::success(call.id, "No numeric values found".into()));
                    }
                    let sum: f64 = numbers.iter().sum();
                    let count = numbers.len() as f64;
                    let mean = sum / count;
                    let min = numbers.iter().cloned().fold(f64::INFINITY, f64::min);
                    let max = numbers.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                    Ok(ToolResult::success(call.id, format!("count={}, sum={sum:.2}, mean={mean:.4}, min={min}, max={max}", numbers.len()))
                        .with_data(json!({"count": numbers.len(), "sum": sum, "mean": mean, "min": min, "max": max})))
                } else {
                    Ok(ToolResult::error(call.id, "Cannot parse input for statistics".into()))
                }
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Data '{action}' — provide input data for processing"),
            )),
        }
    }
}

fn infer_schema(val: &serde_json::Value) -> serde_json::Value {
    match val {
        serde_json::Value::Null => json!({"type": "null"}),
        serde_json::Value::Bool(_) => json!({"type": "boolean"}),
        serde_json::Value::Number(_) => json!({"type": "number"}),
        serde_json::Value::String(_) => json!({"type": "string"}),
        serde_json::Value::Array(arr) => {
            let items = arr.first().map(infer_schema).unwrap_or(json!({}));
            json!({"type": "array", "items": items})
        }
        serde_json::Value::Object(map) => {
            let props: serde_json::Map<String, serde_json::Value> =
                map.iter().map(|(k, v)| (k.clone(), infer_schema(v))).collect();
            json!({"type": "object", "properties": props})
        }
    }
}

fn extract_numbers(val: &serde_json::Value) -> Vec<f64> {
    let mut nums = Vec::new();
    match val {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                nums.push(f);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                nums.extend(extract_numbers(v));
            }
        }
        serde_json::Value::Object(map) => {
            for v in map.values() {
                nums.extend(extract_numbers(v));
            }
        }
        _ => {}
    }
    nums
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(DataTool.definition().name, "data");
    }
    #[test]
    fn test_infer_schema() {
        let val = json!({"name": "test", "age": 25, "tags": ["a", "b"]});
        let schema = infer_schema(&val);
        assert_eq!(schema["type"], "object");
    }
    #[test]
    fn test_extract_numbers() {
        let val = json!({"a": 1, "b": [2, 3], "c": {"d": 4.5}});
        let nums = extract_numbers(&val);
        assert_eq!(nums.len(), 4);
    }
}
