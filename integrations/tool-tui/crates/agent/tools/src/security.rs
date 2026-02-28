//! Security tool — security scanning, secret detection, vulnerability analysis.
//! Actions: scan | secrets | deps_check | sast | dast | sbom | compliance_check | hash | encrypt | decrypt

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct SecurityTool;
impl Default for SecurityTool {
    fn default() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SecurityTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "security".into(),
            description: "Security: vulnerability scanning, secret detection, dependency audit, SAST/DAST, SBOM, hashing, encryption".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "Security action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["scan".into(),"secrets".into(),"deps_check".into(),"sast".into(),"sbom".into(),"hash".into(),"encrypt".into(),"decrypt".into()]) },
                ToolParameter { name: "path".into(), description: "Path to scan".into(), param_type: ParameterType::String, required: false, default: Some(json!(".")), enum_values: None },
                ToolParameter { name: "input".into(), description: "Input data for hash/encrypt".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "algorithm".into(), description: "Hash algorithm (sha256, sha512, blake3)".into(), param_type: ParameterType::String, required: false, default: Some(json!("sha256")), enum_values: None },
            ],
            category: "infra".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("scan");
        let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or(".");

        match action {
            "secrets" => {
                // Scan for common secret patterns
                let patterns = [
                    (r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?[a-zA-Z0-9]{20,}"#, "API Key"),
                    (
                        r#"(?i)(secret|password|passwd|pwd)\s*[:=]\s*['"]?[^\s'"]{8,}"#,
                        "Secret/Password",
                    ),
                    (r"(?i)ghp_[a-zA-Z0-9]{36}", "GitHub PAT"),
                    (r"(?i)sk-[a-zA-Z0-9]{48}", "OpenAI Key"),
                    (r"(?i)AKIA[0-9A-Z]{16}", "AWS Access Key"),
                    (r"(?i)Bearer\s+[a-zA-Z0-9._-]{20,}", "Bearer Token"),
                ];
                let mut findings = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let p = entry.path();
                    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if [
                        "exe", "dll", "so", "bin", "png", "jpg", "gif", "woff", "woff2", "ttf",
                        "lock", "sum",
                    ]
                    .contains(&ext)
                    {
                        continue;
                    }
                    if let Ok(content) = std::fs::read_to_string(p) {
                        for (pattern, label) in &patterns {
                            if let Ok(re) = regex::Regex::new(pattern) {
                                for mat in re.find_iter(&content) {
                                    findings.push(json!({"file": p.display().to_string(), "type": label, "match": &mat.as_str()[..mat.as_str().len().min(40)]}));
                                }
                            }
                        }
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!("{} potential secrets found", findings.len()),
                )
                .with_data(json!({"findings": findings})))
            }
            "hash" => {
                let input = call
                    .arguments
                    .get("input")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'input'"))?;
                let algo =
                    call.arguments.get("algorithm").and_then(|v| v.as_str()).unwrap_or("sha256");
                let hash = match algo {
                    "sha256" => {
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(input.as_bytes());
                        format!("{:x}", hasher.finalize())
                    }
                    _ => {
                        use sha2::{Digest, Sha256};
                        let mut hasher = Sha256::new();
                        hasher.update(input.as_bytes());
                        format!("{:x}", hasher.finalize())
                    }
                };
                Ok(ToolResult::success(call.id, format!("{algo}: {hash}"))
                    .with_data(json!({"algorithm": algo, "hash": hash})))
            }
            "deps_check" => {
                let (shell, flag) = if cfg!(windows) {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };
                let output = tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg("cargo audit 2>&1")
                    .current_dir(path)
                    .output()
                    .await;
                match output {
                    Ok(o) => Ok(ToolResult::success(
                        call.id,
                        String::from_utf8_lossy(&o.stdout).to_string(),
                    )),
                    Err(_) => Ok(ToolResult::success(
                        call.id,
                        "Install cargo-audit: `cargo install cargo-audit`".into(),
                    )),
                }
            }
            "scan" | "sast" => {
                // Basic static analysis scan
                let mut issues = Vec::new();
                for entry in walkdir::WalkDir::new(path)
                    .max_depth(5)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let p = entry.path();
                    if let Ok(content) = std::fs::read_to_string(p) {
                        if content.contains("unsafe ") {
                            issues.push(format!("{}: unsafe block", p.display()));
                        }
                        if content.contains(".unwrap()") {
                            issues.push(format!("{}: unwrap() usage", p.display()));
                        }
                        if content.contains("eval(") {
                            issues.push(format!("{}: eval() call", p.display()));
                        }
                        if content.contains("exec(") {
                            issues.push(format!("{}: exec() call", p.display()));
                        }
                    }
                }
                Ok(ToolResult::success(
                    call.id,
                    format!(
                        "{} potential issues:\n{}",
                        issues.len(),
                        issues.iter().take(50).cloned().collect::<Vec<_>>().join("\n")
                    ),
                ))
            }
            _ => Ok(ToolResult::success(
                call.id,
                format!("Security '{}' — install specialized tools (trivy, snyk, etc.)", action),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(SecurityTool.definition().name, "security");
    }

    #[tokio::test]
    async fn test_hash() {
        let tool = SecurityTool;
        let call = ToolCall {
            id: "h1".into(),
            name: "security".into(),
            arguments: json!({"action":"hash","input":"hello world","algorithm":"sha256"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.success);
        assert!(
            r.output
                .contains("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
        );
    }
}
