//! YAML/JSON conversion utilities.
//!
//! Convert between YAML and JSON formats.

use crate::error::{DxError, Result};
use crate::tools::ToolOutput;
use std::path::Path;
use std::process::Command;

/// Convert JSON to YAML.
///
/// # Example
/// ```no_run
/// use dx_media::tools::utility::yaml_convert;
///
/// yaml_convert::json_to_yaml("data.json", "data.yaml").unwrap();
/// ```
pub fn json_to_yaml<P: AsRef<Path>>(input: P, output: P) -> Result<ToolOutput> {
    let input_path = input.as_ref();
    let output_path = output.as_ref();

    let content = std::fs::read_to_string(input_path).map_err(|e| DxError::FileIo {
        path: input_path.to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    // Try yq
    if let Ok(result) = convert_with_yq(input_path, output_path, "yaml") {
        return Ok(result);
    }

    // Simple conversion
    let yaml = json_to_yaml_simple(&content)?;

    std::fs::write(output_path, &yaml).map_err(|e| DxError::FileIo {
        path: output_path.to_path_buf(),
        message: format!("Failed to write file: {}", e),
        source: None,
    })?;

    Ok(ToolOutput::success_with_path("Converted JSON to YAML", output_path))
}

/// Convert YAML to JSON.
///
/// # Example
/// ```no_run
/// use dx_media::tools::utility::yaml_convert;
///
/// yaml_convert::yaml_to_json("data.yaml", "data.json").unwrap();
/// ```
pub fn yaml_to_json<P: AsRef<Path>>(input: P, output: P) -> Result<ToolOutput> {
    let input_path = input.as_ref();
    let output_path = output.as_ref();

    let content = std::fs::read_to_string(input_path).map_err(|e| DxError::FileIo {
        path: input_path.to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    // Try yq
    if let Ok(result) = convert_with_yq(input_path, output_path, "json") {
        return Ok(result);
    }

    // Simple conversion
    let json = yaml_to_json_simple(&content)?;

    std::fs::write(output_path, &json).map_err(|e| DxError::FileIo {
        path: output_path.to_path_buf(),
        message: format!("Failed to write file: {}", e),
        source: None,
    })?;

    Ok(ToolOutput::success_with_path("Converted YAML to JSON", output_path))
}

/// Convert using yq.
fn convert_with_yq(input: &Path, output: &Path, format: &str) -> Result<ToolOutput> {
    let mut cmd = Command::new("yq");

    if format == "json" {
        cmd.arg("-o").arg("json");
    } else {
        cmd.arg("-o").arg("yaml");
    }

    cmd.arg(input);

    let result = cmd.output().map_err(|e| DxError::Config {
        message: format!("Failed to run yq: {}", e),
        source: None,
    })?;

    if !result.status.success() {
        return Err(DxError::Config {
            message: "yq conversion failed".to_string(),
            source: None,
        });
    }

    std::fs::write(output, &result.stdout).map_err(|e| DxError::FileIo {
        path: output.to_path_buf(),
        message: format!("Failed to write file: {}", e),
        source: None,
    })?;

    Ok(ToolOutput::success_with_path(
        format!("Converted to {} using yq", format),
        output,
    ))
}

/// Simple JSON to YAML conversion.
fn json_to_yaml_simple(json: &str) -> Result<String> {
    let mut result = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;
    let mut in_key = false;
    let mut after_colon = false;
    let mut line_start = true;

    for c in json.chars() {
        if escape_next {
            result.push(c);
            escape_next = false;
            continue;
        }

        if c == '\\' && in_string {
            result.push(c);
            escape_next = true;
            continue;
        }

        if c == '"' {
            if !in_string && !after_colon {
                in_key = true;
            } else if in_key {
                in_key = false;
            }
            in_string = !in_string;

            // Don't output quotes in YAML keys
            if in_key || (!in_string && !after_colon) {
                continue;
            }
            result.push(c);
            continue;
        }

        if in_string {
            result.push(c);
            continue;
        }

        match c {
            '{' => {
                if !line_start {
                    result.push('\n');
                }
                depth += 1;
                line_start = true;
                after_colon = false;
            }
            '}' => {
                depth -= 1;
                after_colon = false;
            }
            '[' => {
                if !line_start {
                    result.push('\n');
                }
                depth += 1;
                line_start = true;
                after_colon = false;
            }
            ']' => {
                depth -= 1;
                after_colon = false;
            }
            ':' => {
                result.push(':');
                result.push(' ');
                after_colon = true;
            }
            ',' => {
                result.push('\n');
                for _ in 0..depth {
                    result.push_str("  ");
                }
                line_start = true;
                after_colon = false;
            }
            ' ' | '\t' | '\n' | '\r' => {
                // Skip whitespace
            }
            _ => {
                if line_start {
                    for _ in 0..depth {
                        result.push_str("  ");
                    }
                    line_start = false;
                }
                result.push(c);
            }
        }
    }

    Ok(result)
}

/// Simple YAML to JSON conversion.
#[allow(unused_assignments)]
fn yaml_to_json_simple(yaml: &str) -> Result<String> {
    // This is a very basic implementation
    // For proper YAML parsing, use the serde_yaml crate

    let mut result = String::new();
    let mut depth = 0;
    let mut in_object = false;
    let mut first_item = true;

    for line in yaml.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Count leading spaces
        let indent = line.len() - line.trim_start().len();
        let new_depth = indent / 2;

        // Adjust depth
        while depth > new_depth {
            result.push('}');
            depth -= 1;
        }

        if trimmed.starts_with('-') {
            // Array item
            if !first_item {
                result.push(',');
            }
            let value = trimmed.trim_start_matches('-').trim();
            if value.is_empty() {
                result.push('[');
            } else {
                result.push_str(&format!("\"{}\"", value));
            }
            first_item = false;
        } else if let Some(colon_pos) = trimmed.find(':') {
            // Key-value pair
            if !first_item && in_object {
                result.push(',');
            }

            if !in_object {
                result.push('{');
                in_object = true;
            }

            let key = trimmed[..colon_pos].trim();
            let value = trimmed[colon_pos + 1..].trim();

            result.push_str(&format!("\"{}\":", key));

            if value.is_empty() {
                // Nested object/array
                depth += 1;
                first_item = true;
            } else {
                // Inline value
                if value.starts_with('"')
                    || value.parse::<f64>().is_ok()
                    || value == "true"
                    || value == "false"
                    || value == "null"
                {
                    result.push_str(value);
                } else {
                    result.push_str(&format!("\"{}\"", value));
                }
            }
            first_item = false;
        }
    }

    // Close remaining objects
    while depth > 0 {
        result.push('}');
        depth -= 1;
    }

    if in_object {
        result.push('}');
    }

    Ok(result)
}

/// Convert JSON string to YAML string.
pub fn json_string_to_yaml(json: &str) -> Result<ToolOutput> {
    let yaml = json_to_yaml_simple(json)?;
    Ok(ToolOutput::success(yaml.clone()).with_metadata("format", "yaml".to_string()))
}

/// Convert YAML string to JSON string.
pub fn yaml_string_to_json(yaml: &str) -> Result<ToolOutput> {
    let json = yaml_to_json_simple(yaml)?;
    Ok(ToolOutput::success(json.clone()).with_metadata("format", "json".to_string()))
}

/// Validate YAML file.
pub fn validate_yaml<P: AsRef<Path>>(input: P) -> Result<ToolOutput> {
    let content = std::fs::read_to_string(input.as_ref()).map_err(|e| DxError::FileIo {
        path: input.as_ref().to_path_buf(),
        message: format!("Failed to read file: {}", e),
        source: None,
    })?;

    // Try yq validation
    let mut cmd = Command::new("yq");
    cmd.arg("--exit-status").arg(".").arg(input.as_ref());

    if let Ok(result) = cmd.output() {
        if result.status.success() {
            return Ok(ToolOutput::success("Valid YAML").with_metadata("valid", "true".to_string()));
        }
    }

    // Basic validation
    let valid = !content.contains('\t') && yaml_to_json_simple(&content).is_ok();

    Ok(ToolOutput::success(
        if valid {
            "YAML appears valid"
        } else {
            "YAML validation uncertain"
        }
        .to_string(),
    )
    .with_metadata("valid", valid.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_yaml_simple() {
        let json = r#"{"name": "test", "value": 123}"#;
        let yaml = json_to_yaml_simple(json).unwrap();
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("value:"));
    }
}
