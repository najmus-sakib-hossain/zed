//! Security Report Generation
//!
//! Generates comprehensive security reports in dx-serializer format with three representations:
//! - HUMAN format: Color-coded severity for terminal display
//! - LLM format: Token-efficient format for disk storage
//! - MACHINE format: Binary format for CI/CD integration
//!
//! **Requirements: 3.4, 3.6**

use crate::diagnostics::{Diagnostic, DiagnosticSeverity};
use crate::scoring_impl::Severity;
use std::collections::HashMap;
use std::path::Path;

/// Output format for security reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityReportFormat {
    /// Human-readable format with color-coded severity (for terminal)
    Human,
    /// LLM-optimized format (for disk storage)
    Llm,
    /// Binary format (for CI/CD)
    Machine,
}

/// Security report containing all vulnerability findings
#[derive(Debug, Clone)]
pub struct SecurityReport {
    /// Total number of vulnerabilities found
    pub total_count: usize,
    /// Vulnerabilities grouped by severity
    pub by_severity: HashMap<Severity, Vec<SecurityFinding>>,
    /// Vulnerabilities grouped by category
    pub by_category: HashMap<String, Vec<SecurityFinding>>,
    /// Overall security score (0-100)
    pub security_score: u8,
}

/// A single security finding
#[derive(Debug, Clone)]
pub struct SecurityFinding {
    /// Severity level
    pub severity: Severity,
    /// Vulnerability ID
    pub id: String,
    /// Vulnerability name
    pub name: String,
    /// File path
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Vulnerability message
    pub message: String,
    /// Remediation guidance
    pub remediation: String,
    /// CWE ID (if applicable)
    pub cwe_id: Option<String>,
    /// OWASP category (if applicable)
    pub owasp_category: Option<String>,
}

impl SecurityReport {
    /// Create a new security report from diagnostics
    #[must_use]
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut by_severity: HashMap<Severity, Vec<SecurityFinding>> = HashMap::new();
        let mut by_category: HashMap<String, Vec<SecurityFinding>> = HashMap::new();

        for diag in diagnostics {
            let severity = Self::diagnostic_severity_to_severity(diag.severity);
            let finding = Self::diagnostic_to_finding(diag);

            by_severity.entry(severity).or_default().push(finding.clone());

            if let Some(ref owasp) = finding.owasp_category {
                by_category.entry(owasp.clone()).or_default().push(finding.clone());
            }
        }

        let total_count = diagnostics.len();
        let security_score = Self::calculate_security_score(&by_severity);

        Self {
            total_count,
            by_severity,
            by_category,
            security_score,
        }
    }

    /// Convert diagnostic severity to scoring severity
    fn diagnostic_severity_to_severity(severity: DiagnosticSeverity) -> Severity {
        match severity {
            DiagnosticSeverity::Error => Severity::Critical,
            DiagnosticSeverity::Warning => Severity::Medium,
            DiagnosticSeverity::Info => Severity::Low,
            DiagnosticSeverity::Hint => Severity::Low,
        }
    }

    /// Convert diagnostic to security finding
    fn diagnostic_to_finding(diag: &Diagnostic) -> SecurityFinding {
        // Parse CWE and OWASP from message if present
        let (message, cwe_id, owasp_category, remediation) =
            Self::parse_diagnostic_message(&diag.message);

        SecurityFinding {
            severity: Self::diagnostic_severity_to_severity(diag.severity),
            id: diag.rule_id.clone(),
            name: diag.rule_id.clone(),
            file: diag.file.display().to_string(),
            line: diag.span.start,
            column: diag.span.end,
            message,
            remediation: remediation.or_else(|| diag.suggestion.clone()).unwrap_or_default(),
            cwe_id,
            owasp_category,
        }
    }

    /// Parse diagnostic message to extract CWE, OWASP, and remediation
    fn parse_diagnostic_message(
        msg: &str,
    ) -> (String, Option<String>, Option<String>, Option<String>) {
        let mut message = msg.to_string();
        let mut cwe_id = None;
        let mut owasp_category = None;
        let mut remediation = None;

        // Split by newlines to extract structured info
        let parts: Vec<&str> = msg.split('\n').collect();
        if !parts.is_empty() {
            message = parts[0].to_string();

            for part in &parts[1..] {
                let trimmed = part.trim();
                if trimmed.starts_with("Remediation:") {
                    remediation =
                        Some(trimmed.trim_start_matches("Remediation:").trim().to_string());
                } else if trimmed.starts_with("CWE:") {
                    let cwe = trimmed.trim_start_matches("CWE:").trim();
                    if cwe != "N/A" {
                        cwe_id = Some(cwe.to_string());
                    }
                } else if trimmed.starts_with("OWASP:") {
                    let owasp = trimmed.trim_start_matches("OWASP:").trim();
                    if owasp != "N/A" {
                        owasp_category = Some(owasp.to_string());
                    }
                }
            }
        }

        (message, cwe_id, owasp_category, remediation)
    }

    /// Calculate security score (0-100) based on vulnerabilities
    fn calculate_security_score(by_severity: &HashMap<Severity, Vec<SecurityFinding>>) -> u8 {
        let mut score: u8 = 100;

        // Deduct points based on severity
        if let Some(critical) = by_severity.get(&Severity::Critical) {
            score = score.saturating_sub((critical.len() as u8) * 10);
        }
        if let Some(high) = by_severity.get(&Severity::High) {
            score = score.saturating_sub((high.len() as u8) * 5);
        }
        if let Some(medium) = by_severity.get(&Severity::Medium) {
            score = score.saturating_sub((medium.len() as u8) * 2);
        }
        if let Some(low) = by_severity.get(&Severity::Low) {
            score = score.saturating_sub(low.len() as u8);
        }

        score
    }

    /// Generate report in the specified format
    #[must_use]
    pub fn generate(&self, format: SecurityReportFormat) -> String {
        match format {
            SecurityReportFormat::Human => self.generate_human(),
            SecurityReportFormat::Llm => self.generate_llm(),
            SecurityReportFormat::Machine => self.generate_machine(),
        }
    }

    /// Generate human-readable report with color-coded severity
    fn generate_human(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!("\n{}\n", "=".repeat(80)));
        output.push_str(&format!("{:^80}\n", "SECURITY SCAN REPORT"));
        output.push_str(&format!("{}\n\n", "=".repeat(80)));

        // Summary
        output.push_str(&format!("Total Vulnerabilities: {}\n", self.total_count));
        output.push_str(&format!("Security Score: {}/100\n\n", self.security_score));

        // Severity breakdown
        output.push_str("Severity Breakdown:\n");
        output.push_str(&format!(
            "  {} Critical: {}\n",
            Self::severity_symbol(Severity::Critical),
            self.by_severity.get(&Severity::Critical).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "  {} High:     {}\n",
            Self::severity_symbol(Severity::High),
            self.by_severity.get(&Severity::High).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "  {} Medium:   {}\n",
            Self::severity_symbol(Severity::Medium),
            self.by_severity.get(&Severity::Medium).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "  {} Low:      {}\n\n",
            Self::severity_symbol(Severity::Low),
            self.by_severity.get(&Severity::Low).map_or(0, std::vec::Vec::len)
        ));

        // Detailed findings
        if self.total_count > 0 {
            output.push_str(&format!("{}\n", "-".repeat(80)));
            output.push_str("DETAILED FINDINGS\n");
            output.push_str(&format!("{}\n\n", "-".repeat(80)));

            for severity in &[
                Severity::Critical,
                Severity::High,
                Severity::Medium,
                Severity::Low,
            ] {
                if let Some(findings) = self.by_severity.get(severity)
                    && !findings.is_empty()
                {
                    output.push_str(&format!(
                        "\n{} {} Issues:\n\n",
                        Self::severity_symbol(*severity),
                        Self::severity_name(*severity)
                    ));

                    for (i, finding) in findings.iter().enumerate() {
                        output.push_str(&format!("{}. [{}] {}\n", i + 1, finding.id, finding.name));
                        output.push_str(&format!(
                            "   Location: {}:{}:{}\n",
                            finding.file, finding.line, finding.column
                        ));
                        output.push_str(&format!("   Message: {}\n", finding.message));

                        if !finding.remediation.is_empty() {
                            output.push_str(&format!("   Remediation: {}\n", finding.remediation));
                        }

                        if let Some(ref cwe) = finding.cwe_id {
                            output.push_str(&format!("   CWE: {cwe}\n"));
                        }

                        if let Some(ref owasp) = finding.owasp_category {
                            output.push_str(&format!("   OWASP: {owasp}\n"));
                        }

                        output.push('\n');
                    }
                }
            }
        }

        output.push_str(&format!("{}\n", "=".repeat(80)));
        output
    }

    /// Generate LLM-optimized format for disk storage
    fn generate_llm(&self) -> String {
        let mut output = String::new();

        // Compact header
        output.push_str("security_report\n");
        output.push_str(&format!("total={}\n", self.total_count));
        output.push_str(&format!("score={}\n", self.security_score));

        // Severity counts
        output.push_str(&format!(
            "critical={}\n",
            self.by_severity.get(&Severity::Critical).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "high={}\n",
            self.by_severity.get(&Severity::High).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "medium={}\n",
            self.by_severity.get(&Severity::Medium).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "low={}\n",
            self.by_severity.get(&Severity::Low).map_or(0, std::vec::Vec::len)
        ));

        // Findings in compact format
        if self.total_count > 0 {
            output.push_str(&format!("findings:{}[\n", self.total_count));

            for severity in &[
                Severity::Critical,
                Severity::High,
                Severity::Medium,
                Severity::Low,
            ] {
                if let Some(findings) = self.by_severity.get(severity) {
                    for finding in findings {
                        output.push_str(&format!(
                            "  {{id={} severity={} file={} line={} col={} msg=\"{}\"",
                            finding.id,
                            Self::severity_name(*severity).to_lowercase(),
                            finding.file,
                            finding.line,
                            finding.column,
                            finding.message.replace('"', "'")
                        ));

                        if !finding.remediation.is_empty() {
                            output.push_str(&format!(
                                " remediation=\"{}\"",
                                finding.remediation.replace('"', "'")
                            ));
                        }

                        if let Some(ref cwe) = finding.cwe_id {
                            output.push_str(&format!(" cwe={cwe}"));
                        }

                        if let Some(ref owasp) = finding.owasp_category {
                            output.push_str(&format!(" owasp=\"{}\"", owasp.replace('"', "'")));
                        }

                        output.push_str("};\n");
                    }
                }
            }

            output.push_str("]\n");
        }

        output
    }

    /// Generate machine-readable format for CI/CD
    fn generate_machine(&self) -> String {
        // JSON format for easy parsing by CI/CD tools
        let mut output = String::new();

        output.push_str("{\n");
        output.push_str(&format!("  \"total_count\": {},\n", self.total_count));
        output.push_str(&format!("  \"security_score\": {},\n", self.security_score));

        // Severity counts
        output.push_str("  \"severity_counts\": {\n");
        output.push_str(&format!(
            "    \"critical\": {},\n",
            self.by_severity.get(&Severity::Critical).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "    \"high\": {},\n",
            self.by_severity.get(&Severity::High).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "    \"medium\": {},\n",
            self.by_severity.get(&Severity::Medium).map_or(0, std::vec::Vec::len)
        ));
        output.push_str(&format!(
            "    \"low\": {}\n",
            self.by_severity.get(&Severity::Low).map_or(0, std::vec::Vec::len)
        ));
        output.push_str("  },\n");

        // Findings array
        output.push_str("  \"findings\": [\n");

        let mut first = true;
        for severity in &[
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
        ] {
            if let Some(findings) = self.by_severity.get(severity) {
                for finding in findings {
                    if !first {
                        output.push_str(",\n");
                    }
                    first = false;

                    output.push_str("    {\n");
                    output.push_str(&format!(
                        "      \"id\": \"{}\",\n",
                        Self::escape_json(&finding.id)
                    ));
                    output.push_str(&format!(
                        "      \"severity\": \"{}\",\n",
                        Self::severity_name(*severity).to_lowercase()
                    ));
                    output.push_str(&format!(
                        "      \"file\": \"{}\",\n",
                        Self::escape_json(&finding.file)
                    ));
                    output.push_str(&format!("      \"line\": {},\n", finding.line));
                    output.push_str(&format!("      \"column\": {},\n", finding.column));
                    output.push_str(&format!(
                        "      \"message\": \"{}\"",
                        Self::escape_json(&finding.message)
                    ));

                    if !finding.remediation.is_empty() {
                        output.push_str(&format!(
                            ",\n      \"remediation\": \"{}\"",
                            Self::escape_json(&finding.remediation)
                        ));
                    }

                    if let Some(ref cwe) = finding.cwe_id {
                        output.push_str(&format!(
                            ",\n      \"cwe_id\": \"{}\"",
                            Self::escape_json(cwe)
                        ));
                    }

                    if let Some(ref owasp) = finding.owasp_category {
                        output.push_str(&format!(
                            ",\n      \"owasp_category\": \"{}\"",
                            Self::escape_json(owasp)
                        ));
                    }

                    output.push_str("\n    }");
                }
            }
        }

        output.push_str("\n  ]\n");
        output.push_str("}\n");

        output
    }

    /// Get severity symbol for display
    fn severity_symbol(severity: Severity) -> &'static str {
        match severity {
            Severity::Critical => "🔴",
            Severity::High => "🟠",
            Severity::Medium => "🟡",
            Severity::Low => "🟢",
        }
    }

    /// Get severity name
    fn severity_name(severity: Severity) -> &'static str {
        match severity {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
        }
    }

    /// Escape JSON strings
    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Save report to file
    pub fn save_to_file(&self, path: &Path, format: SecurityReportFormat) -> std::io::Result<()> {
        let content = self.generate(format);
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::Span;
    use std::path::PathBuf;

    fn create_test_diagnostic(
        severity: DiagnosticSeverity,
        rule_id: &str,
        message: &str,
    ) -> Diagnostic {
        Diagnostic {
            file: PathBuf::from("test.js"),
            span: Span::new(10, 20),
            severity,
            rule_id: rule_id.to_string(),
            message: message.to_string(),
            suggestion: Some("Fix this issue".to_string()),
            related: Vec::new(),
            fix: None,
        }
    }

    #[test]
    fn test_empty_report() {
        let report = SecurityReport::from_diagnostics(&[]);
        assert_eq!(report.total_count, 0);
        assert_eq!(report.security_score, 100);
    }

    #[test]
    fn test_report_with_critical_vulnerability() {
        let diagnostics = vec![create_test_diagnostic(
            DiagnosticSeverity::Error,
            "sql-injection",
            "SQL injection detected\nRemediation: Use parameterized queries\nCWE: CWE-89\nOWASP: A03:2021-Injection",
        )];

        let report = SecurityReport::from_diagnostics(&diagnostics);
        assert_eq!(report.total_count, 1);
        assert_eq!(report.security_score, 90); // 100 - 10 for critical
        assert_eq!(report.by_severity.get(&Severity::Critical).unwrap().len(), 1);
    }

    #[test]
    fn test_report_with_multiple_severities() {
        let diagnostics = vec![
            create_test_diagnostic(DiagnosticSeverity::Error, "critical-1", "Critical issue"),
            create_test_diagnostic(DiagnosticSeverity::Error, "critical-2", "Another critical"),
            create_test_diagnostic(DiagnosticSeverity::Warning, "medium-1", "Medium issue"),
            create_test_diagnostic(DiagnosticSeverity::Info, "low-1", "Low issue"),
        ];

        let report = SecurityReport::from_diagnostics(&diagnostics);
        assert_eq!(report.total_count, 4);
        // 100 - (2*10 for critical) - (1*2 for medium) - (1*1 for low) = 77
        assert_eq!(report.security_score, 77);
    }

    #[test]
    fn test_human_format_generation() {
        let diagnostics = vec![create_test_diagnostic(
            DiagnosticSeverity::Error,
            "test-rule",
            "Test vulnerability",
        )];

        let report = SecurityReport::from_diagnostics(&diagnostics);
        let output = report.generate(SecurityReportFormat::Human);

        assert!(output.contains("SECURITY SCAN REPORT"));
        assert!(output.contains("Total Vulnerabilities: 1"));
        assert!(output.contains("Security Score: 90/100"));
        assert!(output.contains("test-rule"));
    }

    #[test]
    fn test_llm_format_generation() {
        let diagnostics = vec![create_test_diagnostic(
            DiagnosticSeverity::Error,
            "test-rule",
            "Test vulnerability",
        )];

        let report = SecurityReport::from_diagnostics(&diagnostics);
        let output = report.generate(SecurityReportFormat::Llm);

        assert!(output.contains("security_report"));
        assert!(output.contains("total=1"));
        assert!(output.contains("score=90"));
        assert!(output.contains("findings:1["));
    }

    #[test]
    fn test_machine_format_generation() {
        let diagnostics = vec![create_test_diagnostic(
            DiagnosticSeverity::Error,
            "test-rule",
            "Test vulnerability",
        )];

        let report = SecurityReport::from_diagnostics(&diagnostics);
        let output = report.generate(SecurityReportFormat::Machine);

        assert!(output.contains("\"total_count\": 1"));
        assert!(output.contains("\"security_score\": 90"));
        assert!(output.contains("\"findings\": ["));
        assert!(output.contains("\"id\": \"test-rule\""));
    }

    #[test]
    fn test_parse_diagnostic_message_with_metadata() {
        let msg = "SQL injection detected\nRemediation: Use parameterized queries\nCWE: CWE-89\nOWASP: A03:2021-Injection";
        let (message, cwe, owasp, remediation) = SecurityReport::parse_diagnostic_message(msg);

        assert_eq!(message, "SQL injection detected");
        assert_eq!(cwe, Some("CWE-89".to_string()));
        assert_eq!(owasp, Some("A03:2021-Injection".to_string()));
        assert_eq!(remediation, Some("Use parameterized queries".to_string()));
    }

    #[test]
    fn test_parse_diagnostic_message_without_metadata() {
        let msg = "Simple error message";
        let (message, cwe, owasp, remediation) = SecurityReport::parse_diagnostic_message(msg);

        assert_eq!(message, "Simple error message");
        assert_eq!(cwe, None);
        assert_eq!(owasp, None);
        assert_eq!(remediation, None);
    }

    #[test]
    fn test_security_score_calculation() {
        let mut by_severity = HashMap::new();

        // Test with no vulnerabilities
        assert_eq!(SecurityReport::calculate_security_score(&by_severity), 100);

        // Test with critical vulnerabilities
        by_severity.insert(
            Severity::Critical,
            vec![SecurityFinding {
                severity: Severity::Critical,
                id: "test".to_string(),
                name: "test".to_string(),
                file: "test.js".to_string(),
                line: 1,
                column: 1,
                message: "test".to_string(),
                remediation: String::new(),
                cwe_id: None,
                owasp_category: None,
            }],
        );
        assert_eq!(SecurityReport::calculate_security_score(&by_severity), 90);
    }

    #[test]
    fn test_json_escaping() {
        assert_eq!(SecurityReport::escape_json("test"), "test");
        assert_eq!(SecurityReport::escape_json("test\"quote"), "test\\\"quote");
        assert_eq!(SecurityReport::escape_json("test\nline"), "test\\nline");
        assert_eq!(SecurityReport::escape_json("test\\slash"), "test\\\\slash");
    }
}
