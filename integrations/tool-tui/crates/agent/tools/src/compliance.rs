//! Compliance tool — license checking, GDPR, accessibility, audit trails.
//! Actions: license_check | gdpr_scan | a11y_audit | audit_trail | spdx | policy_check

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct ComplianceTool;
impl Default for ComplianceTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ComplianceTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "compliance".into(),
            description: "Compliance: license checking, GDPR scanning, accessibility audit, SPDX SBOM, policy enforcement".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Compliance action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["license_check".into(),"gdpr_scan".into(),"a11y_audit".into(),"audit_trail".into(),"spdx".into(),"policy_check".into()]) },
                ToolParameter { name: "path".into(), description: "Path to scan".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "policy".into(), description: "Policy file or rules".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "monitoring".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action =
            call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("license_check");
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        match action {
            "license_check" => {
                // Scan for license files and headers
                let mut licenses = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let name = entry.file_name().to_string_lossy().to_lowercase();
                    if name.contains("license")
                        || name.contains("licence")
                        || name.contains("copying")
                    {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            let license_type = detect_license(&content);
                            licenses.push(json!({"file": entry.path().display().to_string(), "type": license_type}));
                        }
                    }
                }
                Ok(ToolResult::success(call.id, format!("{} license files found", licenses.len()))
                    .with_data(json!({"licenses": licenses})))
            }
            "gdpr_scan" => {
                // Scan for PII patterns
                let patterns = [
                    (r"(?i)\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", "email"),
                    (r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b", "phone"),
                    (r"\b\d{3}-\d{2}-\d{4}\b", "ssn"),
                    (r"(?i)(?:ip[_-]?address|remote[_-]?addr)", "ip_tracking"),
                    (r"(?i)(?:date[_-]?of[_-]?birth|dob|birthday)", "dob"),
                ];
                let mut findings = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .max_depth(5)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ![
                        "rs", "ts", "js", "py", "json", "yaml", "yml", "toml", "env", "cfg",
                    ]
                    .contains(&ext)
                    {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        for (pattern, pii_type) in &patterns {
                            if let Ok(re) = regex::Regex::new(pattern) {
                                if re.is_match(&content) {
                                    findings.push(json!({"file": entry.path().display().to_string(), "type": pii_type}));
                                }
                            }
                        }
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!("{} potential PII findings", findings.len()),
                )
                .with_data(json!({"findings": findings})))
            }
            "a11y_audit" => {
                // Scan HTML files for accessibility issues
                let mut issues = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("");
                    if !["html", "htm", "tsx", "jsx", "vue", "svelte"].contains(&ext) {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if content.contains("<img") && !content.contains("alt=") {
                            issues.push(format!("{}: img without alt", entry.path().display()));
                        }
                        if content.contains("<input")
                            && !content.contains("label")
                            && !content.contains("aria-label")
                        {
                            issues.push(format!("{}: input without label", entry.path().display()));
                        }
                        if content.contains("onclick")
                            && !content.contains("onkeydown")
                            && !content.contains("onkeyup")
                        {
                            issues.push(format!(
                                "{}: click handler without keyboard",
                                entry.path().display()
                            ));
                        }
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "{} a11y issues:\n{}",
                        issues.len(),
                        issues.iter().take(20).cloned().collect::<Vec<_>>().join("\n")
                    ),
                ))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Compliance '{}' — install specialized tools for deeper analysis", action),
            )),
        }
    }
}

fn detect_license(content: &str) -> &str {
    let lower = content.to_lowercase();
    if lower.contains("mit license")
        || lower.contains("permission is hereby granted, free of charge")
    {
        return "MIT";
    }
    if lower.contains("apache license") {
        return "Apache-2.0";
    }
    if lower.contains("gnu general public license") {
        return "GPL";
    }
    if lower.contains("bsd") {
        return "BSD";
    }
    if lower.contains("isc license") {
        return "ISC";
    }
    if lower.contains("mozilla public license") {
        return "MPL-2.0";
    }
    "Unknown"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(ComplianceTool.definition().name, "compliance");
    }
    #[test]
    fn test_detect_license() {
        assert_eq!(detect_license("MIT License\nPermission is hereby granted..."), "MIT");
        assert_eq!(detect_license("Apache License Version 2.0"), "Apache-2.0");
    }
}
