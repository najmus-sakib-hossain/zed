//! Output Formatting Module
//!
//! Provides various output formats for check results including:
//! - DX Serializer formats (binary, LLM, human)
//! - JSON (legacy)
//! - GitHub Actions annotations
//! - `JUnit` XML
//! - SARIF (Static Analysis Results Interchange Format)

pub mod dx_format;

pub use dx_format::{
    DxCheckReport, DxCoverage, DxDiagnostic, DxFileCoverage, DxOutputFormat, DxScoreBreakdown,
    DxStreamWriter, DxTestResult, DxTestResults,
};

use crate::diagnostics::Diagnostic;
use crate::scoring_impl::ProjectScore;

/// All supported output formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// DX Serializer binary format
    DxBinary,
    /// DX Serializer LLM format (token-efficient)
    DxLlm,
    /// DX Serializer human format (readable)
    DxHuman,
    /// JSON (legacy, compatible)
    Json,
    /// JSON compact (single line)
    JsonCompact,
    /// GitHub Actions annotations
    Github,
    /// `JUnit` XML
    Junit,
    /// SARIF (Static Analysis Results Interchange Format)
    Sarif,
    /// Plain text (simple list)
    Text,
    /// Pretty terminal output with colors
    Pretty,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::DxBinary => write!(f, "dx-binary"),
            OutputFormat::DxLlm => write!(f, "dx-llm"),
            OutputFormat::DxHuman => write!(f, "dx-human"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::JsonCompact => write!(f, "json-compact"),
            OutputFormat::Github => write!(f, "github"),
            OutputFormat::Junit => write!(f, "junit"),
            OutputFormat::Sarif => write!(f, "sarif"),
            OutputFormat::Text => write!(f, "text"),
            OutputFormat::Pretty => write!(f, "pretty"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "dx-binary" | "binary" | "bin" => Ok(OutputFormat::DxBinary),
            "dx-llm" | "llm" => Ok(OutputFormat::DxLlm),
            "dx-human" | "human" | "readable" => Ok(OutputFormat::DxHuman),
            "json" => Ok(OutputFormat::Json),
            "json-compact" | "jsonc" => Ok(OutputFormat::JsonCompact),
            "github" | "gh" | "actions" => Ok(OutputFormat::Github),
            "junit" | "xml" => Ok(OutputFormat::Junit),
            "sarif" => Ok(OutputFormat::Sarif),
            "text" | "txt" | "plain" => Ok(OutputFormat::Text),
            "pretty" | "terminal" | "colored" => Ok(OutputFormat::Pretty),
            _ => Err(format!("Unknown output format: {s}")),
        }
    }
}

/// Format diagnostics according to the specified format
#[must_use]
pub fn format_output(
    diagnostics: &[Diagnostic],
    score: Option<&ProjectScore>,
    format: OutputFormat,
) -> String {
    match format {
        OutputFormat::DxBinary | OutputFormat::DxLlm | OutputFormat::DxHuman => {
            let report = if let Some(s) = score {
                let mut r = DxCheckReport::new(s);
                r.add_diagnostics(diagnostics);
                r
            } else {
                let mut r = DxCheckReport {
                    version: 1,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    score: 500,
                    breakdown: DxScoreBreakdown {
                        formatting: 100,
                        linting: 100,
                        security: 100,
                        patterns: 100,
                        structure: 100,
                    },
                    diagnostics: vec![],
                    test_results: None,
                    coverage: None,
                    files_analyzed: 0,
                    total_issues: 0,
                };
                r.add_diagnostics(diagnostics);
                r
            };

            match format {
                OutputFormat::DxBinary => {
                    // Base64 encode for text output
                    let bytes = report.to_dx_binary();
                    base64_encode_simple(&bytes)
                }
                OutputFormat::DxLlm => report.to_dx_llm(),
                OutputFormat::DxHuman => report.to_dx_human(),
                _ => unreachable!(),
            }
        }
        OutputFormat::Json => format_json(diagnostics, score),
        OutputFormat::JsonCompact => format_json_compact(diagnostics, score),
        OutputFormat::Github => format_github(diagnostics),
        OutputFormat::Junit => format_junit(diagnostics),
        OutputFormat::Sarif => format_sarif(diagnostics),
        OutputFormat::Text => format_text(diagnostics),
        OutputFormat::Pretty => format_pretty(diagnostics, score),
    }
}

/// Format as JSON
fn format_json(diagnostics: &[Diagnostic], score: Option<&ProjectScore>) -> String {
    #[derive(serde::Serialize)]
    struct JsonOutput<'a> {
        diagnostics: Vec<JsonDiagnostic<'a>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        score: Option<JsonScore>,
    }

    #[derive(serde::Serialize)]
    struct JsonDiagnostic<'a> {
        file: String,
        line: u32,
        column: u32,
        severity: &'a str,
        rule: &'a str,
        message: &'a str,
    }

    #[derive(serde::Serialize)]
    struct JsonScore {
        total: u16,
        formatting: u16,
        linting: u16,
        security: u16,
        patterns: u16,
        structure: u16,
    }

    let output = JsonOutput {
        diagnostics: diagnostics
            .iter()
            .map(|d| JsonDiagnostic {
                file: d.file.to_string_lossy().to_string(),
                line: d.span.start,
                column: 1,
                severity: d.severity.as_str(),
                rule: &d.rule_id,
                message: &d.message,
            })
            .collect(),
        score: score.map(|s| JsonScore {
            total: s.total_score,
            formatting: s.get_category_score(crate::scoring_impl::Category::Formatting),
            linting: s.get_category_score(crate::scoring_impl::Category::Linting),
            security: s.get_category_score(crate::scoring_impl::Category::Security),
            patterns: s.get_category_score(crate::scoring_impl::Category::DesignPatterns),
            structure: s.get_category_score(crate::scoring_impl::Category::StructureAndDocs),
        }),
    };

    serde_json::to_string_pretty(&output).unwrap_or_default()
}

/// Format as compact JSON
fn format_json_compact(diagnostics: &[Diagnostic], score: Option<&ProjectScore>) -> String {
    // Same as format_json but use to_string instead of to_string_pretty
    let json = format_json(diagnostics, score);
    // Re-parse and re-serialize compact
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
        serde_json::to_string(&v).unwrap_or(json)
    } else {
        json
    }
}

/// Format as GitHub Actions annotations
fn format_github(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();

    for d in diagnostics {
        let level = match d.severity {
            crate::diagnostics::DiagnosticSeverity::Error => "error",
            crate::diagnostics::DiagnosticSeverity::Warning => "warning",
            _ => "notice",
        };

        output.push_str(&format!(
            "::{} file={},line={},title={}::{}\n",
            level,
            d.file.to_string_lossy(),
            d.span.start,
            d.rule_id,
            d.message.replace('\n', "%0A"),
        ));
    }

    output
}

/// Format as `JUnit` XML
fn format_junit(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();

    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<testsuites>\n");
    output.push_str("  <testsuite name=\"dx-check\" tests=\"");
    output.push_str(&diagnostics.len().to_string());
    output.push_str("\" failures=\"");
    output.push_str(
        &diagnostics
            .iter()
            .filter(|d| d.severity == crate::diagnostics::DiagnosticSeverity::Error)
            .count()
            .to_string(),
    );
    output.push_str("\">\n");

    for d in diagnostics {
        let escaped_msg = escape_xml(&d.message);
        let escaped_rule = escape_xml(&d.rule_id);

        output.push_str(&format!(
            "    <testcase name=\"{}\" classname=\"{}\">\n",
            escaped_rule,
            d.file.to_string_lossy(),
        ));

        if d.severity == crate::diagnostics::DiagnosticSeverity::Error {
            output.push_str(&format!(
                "      <failure message=\"{}\" type=\"{}\">Line {}: {}</failure>\n",
                escaped_msg, escaped_rule, d.span.start, escaped_msg,
            ));
        }

        output.push_str("    </testcase>\n");
    }

    output.push_str("  </testsuite>\n");
    output.push_str("</testsuites>\n");

    output
}

/// Format as SARIF
fn format_sarif(diagnostics: &[Diagnostic]) -> String {
    let mut results = Vec::new();

    for d in diagnostics {
        results.push(serde_json::json!({
            "ruleId": d.rule_id,
            "level": match d.severity {
                crate::diagnostics::DiagnosticSeverity::Error => "error",
                crate::diagnostics::DiagnosticSeverity::Warning => "warning",
                crate::diagnostics::DiagnosticSeverity::Info => "note",
                crate::diagnostics::DiagnosticSeverity::Hint => "none",
            },
            "message": {
                "text": d.message,
            },
            "locations": [{
                "physicalLocation": {
                    "artifactLocation": {
                        "uri": d.file.to_string_lossy(),
                    },
                    "region": {
                        "startLine": d.span.start,
                        "endLine": d.span.end,
                    },
                },
            }],
        }));
    }

    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "dx-check",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://dx.dev/check",
                },
            },
            "results": results,
        }],
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_default()
}

/// Format as plain text
fn format_text(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();

    for d in diagnostics {
        let severity = match d.severity {
            crate::diagnostics::DiagnosticSeverity::Error => "error",
            crate::diagnostics::DiagnosticSeverity::Warning => "warning",
            crate::diagnostics::DiagnosticSeverity::Info => "info",
            crate::diagnostics::DiagnosticSeverity::Hint => "hint",
        };

        output.push_str(&format!(
            "{}:{}:{}: {} [{}] {}\n",
            d.file.to_string_lossy(),
            d.span.start,
            1,
            severity,
            d.rule_id,
            d.message,
        ));
    }

    output
}

/// Format as pretty terminal output
fn format_pretty(diagnostics: &[Diagnostic], score: Option<&ProjectScore>) -> String {
    use colored::Colorize;

    let mut output = String::new();

    // Group by file
    let mut by_file: std::collections::HashMap<_, Vec<_>> = std::collections::HashMap::new();
    for d in diagnostics {
        by_file.entry(d.file.clone()).or_default().push(d);
    }

    for (file, diags) in by_file {
        output.push_str(&format!("\n{}\n", file.to_string_lossy().bold()));

        for d in diags {
            let (icon, _color) = match d.severity {
                crate::diagnostics::DiagnosticSeverity::Error => ("✖", "red"),
                crate::diagnostics::DiagnosticSeverity::Warning => ("⚠", "yellow"),
                crate::diagnostics::DiagnosticSeverity::Info => ("ℹ", "blue"),
                crate::diagnostics::DiagnosticSeverity::Hint => ("💡", "cyan"),
            };

            output.push_str(&format!(
                "  {} {}:{} {} {}\n",
                icon,
                d.span.start.to_string().dimmed(),
                1,
                d.message,
                format!("[{}]", d.rule_id).dimmed(),
            ));
        }
    }

    // Score summary
    if let Some(s) = score {
        output.push_str(&format!("\n{}\n", "Score Summary".bold()));
        output.push_str(&format!("  Total: {}/500 ({})\n", s.total_score, s.grade()));
    }

    output
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Simple base64 encoding
fn base64_encode_simple(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        buf[..chunk.len()].copy_from_slice(chunk);

        let n = (u32::from(buf[0]) << 16) | (u32::from(buf[1]) << 8) | u32::from(buf[2]);

        result.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        result.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[(n & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
        assert_eq!("github".parse::<OutputFormat>().unwrap(), OutputFormat::Github);
        assert_eq!("dx-llm".parse::<OutputFormat>().unwrap(), OutputFormat::DxLlm);
    }

    #[test]
    fn test_github_format() {
        let diag = Diagnostic {
            file: std::path::PathBuf::from("test.rs"),
            span: crate::diagnostics::Span::new(10, 10),
            severity: crate::diagnostics::DiagnosticSeverity::Error,
            rule_id: "test/rule".to_string(),
            message: "Test error".to_string(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        };

        let output = format_github(&[diag]);
        assert!(output.contains("::error"));
        assert!(output.contains("file=test.rs"));
        assert!(output.contains("line=10"));
    }
}
