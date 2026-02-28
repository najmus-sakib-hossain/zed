//! Binary Report Format (.sr) and Markdown Report Generation
//!
//! Integration with dx-serializer for zero-copy binary report serialization
//! and dx-markdown for human-readable report generation.
//! _Requirements: 10.2, 10.3_

use crate::error::{Result, SecurityError};
use crate::signer::{BinaryReport, ReportSigner, SignedReport};
use crate::stream::{Finding, FindingType, Severity};
use ed25519_dalek::{SigningKey, VerifyingKey};
use markdown::{
    CellValue, ColumnDef, DxmDocument, DxmMeta, DxmNode, HeaderNode, InlineNode, ListItem,
    ListNode, SemanticBlockNode, SemanticBlockType, TableNode, markdown::to_markdown,
};
use std::path::Path;

/// Magic bytes for .sr files
pub const DXS_MAGIC: [u8; 4] = *b"SR\0";

/// Current .sr format version
pub const DXS_VERSION: u8 = 1;

/// Binary finding entry in the report
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BinaryFinding {
    /// Finding type (0=Vulnerability, 1=Secret, 2=RuleViolation)
    pub finding_type: u8,
    /// Severity level (0-4)
    pub severity: u8,
    /// Hash of the file path (for compact storage)
    pub file_hash: u64,
    /// Line number where finding was detected
    pub line_number: u32,
    /// Column number
    pub column: u16,
    /// Length of the message
    pub message_len: u16,
    /// Message bytes (variable length, follows struct)
    pub message: String,
}

impl BinaryFinding {
    /// Create a new binary finding
    pub fn new(
        finding_type: FindingType,
        severity: Severity,
        file_hash: u64,
        line_number: u32,
        column: u16,
        message: String,
    ) -> Self {
        Self {
            finding_type: finding_type as u8,
            severity: severity as u8,
            file_hash,
            line_number,
            column,
            message_len: message.len() as u16,
            message,
        }
    }

    /// Serialize finding to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(18 + self.message.len());
        bytes.push(self.finding_type);
        bytes.push(self.severity);
        bytes.extend_from_slice(&self.file_hash.to_le_bytes());
        bytes.extend_from_slice(&self.line_number.to_le_bytes());
        bytes.extend_from_slice(&self.column.to_le_bytes());
        bytes.extend_from_slice(&self.message_len.to_le_bytes());
        bytes.extend_from_slice(self.message.as_bytes());
        bytes
    }

    /// Parse finding from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<(Self, usize)> {
        if bytes.len() < 18 {
            return None;
        }

        let finding_type = bytes[0];
        let severity = bytes[1];
        let file_hash = u64::from_le_bytes(bytes[2..10].try_into().ok()?);
        let line_number = u32::from_le_bytes(bytes[10..14].try_into().ok()?);
        let column = u16::from_le_bytes(bytes[14..16].try_into().ok()?);
        let message_len = u16::from_le_bytes(bytes[16..18].try_into().ok()?) as usize;

        if bytes.len() < 18 + message_len {
            return None;
        }

        let message = String::from_utf8_lossy(&bytes[18..18 + message_len]).to_string();
        let total_len = 18 + message_len;

        Some((
            Self {
                finding_type,
                severity,
                file_hash,
                line_number,
                column,
                message_len: message_len as u16,
                message,
            },
            total_len,
        ))
    }
}

/// Full security report with findings
#[derive(Debug, Clone)]
pub struct SecurityReport {
    /// Report header
    pub header: BinaryReport,
    /// List of findings
    pub findings: Vec<BinaryFinding>,
}

impl SecurityReport {
    /// Create a new security report
    pub fn new(score: u8, git_hash: [u8; 20]) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            header: BinaryReport::new(score, timestamp, git_hash, 0),
            findings: Vec::new(),
        }
    }

    /// Add a finding to the report
    pub fn add_finding(&mut self, finding: BinaryFinding) {
        self.findings.push(finding);
        self.header.findings_count = self.findings.len() as u32;
    }

    /// Add a finding from a stream Finding
    pub fn add_stream_finding(&mut self, finding: &Finding, file_hash: u64) {
        let finding_type = match finding.finding_type {
            FindingType::Vulnerability => FindingType::Vulnerability,
            FindingType::Secret => FindingType::Secret,
            FindingType::RuleViolation => FindingType::RuleViolation,
            _ => FindingType::Vulnerability,
        };

        let severity = Severity::from_u8(finding.severity).unwrap_or(Severity::Medium);

        self.add_finding(BinaryFinding::new(
            finding_type,
            severity,
            file_hash,
            finding.line_number,
            finding.column,
            finding.message.clone(),
        ));
    }

    /// Serialize report to bytes (without signature)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();

        // Add findings
        for finding in &self.findings {
            bytes.extend_from_slice(&finding.to_bytes());
        }

        bytes
    }

    /// Parse report from bytes (without signature)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 38 {
            return Err(SecurityError::InvalidFormat("Report too small".to_string()));
        }

        // Parse header
        if &bytes[0..4] != DXS_MAGIC {
            return Err(SecurityError::InvalidFormat("Invalid magic".to_string()));
        }

        let version = bytes[4];
        if version != DXS_VERSION {
            return Err(SecurityError::InvalidFormat(format!("Unsupported version: {}", version)));
        }

        let score = bytes[5];
        let timestamp = u64::from_le_bytes(bytes[6..14].try_into().unwrap());
        let mut git_hash = [0u8; 20];
        git_hash.copy_from_slice(&bytes[14..34]);
        let findings_count = u32::from_le_bytes(bytes[34..38].try_into().unwrap());

        let header = BinaryReport {
            magic: DXS_MAGIC,
            version,
            score,
            timestamp,
            git_hash,
            findings_count,
        };

        // Parse findings
        let mut findings = Vec::with_capacity(findings_count as usize);
        let mut offset = 38;

        for _ in 0..findings_count {
            if offset >= bytes.len() {
                break;
            }

            if let Some((finding, len)) = BinaryFinding::from_bytes(&bytes[offset..]) {
                findings.push(finding);
                offset += len;
            } else {
                break;
            }
        }

        Ok(Self { header, findings })
    }
}

/// Report exporter for .sr files
///
/// Handles export and import of signed security reports.
/// _Requirements: 8.1, 10.2_
pub struct ReportExporter;

impl ReportExporter {
    /// Export a signed report to .sr file
    pub fn export(report: &SecurityReport, key: &SigningKey, path: &Path) -> Result<()> {
        // Sign the report
        let signed = ReportSigner::sign(&report.header, key);

        // Build the full file
        let mut bytes = report.to_bytes();
        bytes.extend_from_slice(&signed.signature);
        bytes.extend_from_slice(&signed.signer_public_key);

        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Export an unsigned report to .sr file
    pub fn export_unsigned(report: &SecurityReport, path: &Path) -> Result<()> {
        let bytes = report.to_bytes();
        std::fs::write(path, bytes)?;
        Ok(())
    }

    /// Import a signed report from .sr file
    pub fn import(path: &Path) -> Result<(SecurityReport, Option<SignedReport>)> {
        let bytes = std::fs::read(path)?;

        if bytes.len() < 38 {
            return Err(SecurityError::InvalidFormat("File too small".to_string()));
        }

        // Check if file has signature (needs at least 96 more bytes: 64 sig + 32 pubkey)
        let has_signature = bytes.len() >= 38 + 96;

        // Parse the report
        let report_end = if has_signature {
            bytes.len() - 96
        } else {
            bytes.len()
        };

        let report = SecurityReport::from_bytes(&bytes[..report_end])?;

        // Parse signature if present
        let signed = if has_signature {
            let sig_start = bytes.len() - 96;
            let mut signature = [0u8; 64];
            signature.copy_from_slice(&bytes[sig_start..sig_start + 64]);

            let mut signer_public_key = [0u8; 32];
            signer_public_key.copy_from_slice(&bytes[sig_start + 64..]);

            Some(SignedReport {
                report: report.header.clone(),
                signature,
                signer_public_key,
            })
        } else {
            None
        };

        Ok((report, signed))
    }

    /// Import and verify a signed report
    pub fn import_verified(path: &Path, key: &VerifyingKey) -> Result<SecurityReport> {
        let (report, signed) = Self::import(path)?;

        if let Some(signed_report) = signed {
            if !ReportSigner::verify(&signed_report, key) {
                return Err(SecurityError::SignatureError);
            }
        } else {
            return Err(SecurityError::InvalidFormat("Report is not signed".to_string()));
        }

        Ok(report)
    }
}

/// Hash a file path for compact storage
pub fn hash_file_path(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    hasher.finish()
}

// ============================================================================
// Markdown Report Generator
// ============================================================================

/// Markdown report generator using dx-markdown
///
/// Generates human-readable security reports with:
/// - Score summary with visual indicator
/// - Findings table with severity, location, description
/// - Remediation suggestions
/// _Requirements: 10.3_
pub struct MarkdownReportGenerator;

impl MarkdownReportGenerator {
    /// Generate a markdown report from a SecurityReport
    pub fn generate(report: &SecurityReport) -> String {
        let doc = Self::build_document(report);
        to_markdown(&doc)
    }

    /// Generate a markdown report with file path mapping
    pub fn generate_with_paths(
        report: &SecurityReport,
        path_map: &std::collections::HashMap<u64, String>,
    ) -> String {
        let doc = Self::build_document_with_paths(report, path_map);
        to_markdown(&doc)
    }

    /// Build a DxmDocument from a SecurityReport
    fn build_document(report: &SecurityReport) -> DxmDocument {
        Self::build_document_with_paths(report, &std::collections::HashMap::new())
    }

    /// Build a DxmDocument with file path mapping
    fn build_document_with_paths(
        report: &SecurityReport,
        path_map: &std::collections::HashMap<u64, String>,
    ) -> DxmDocument {
        let mut doc = DxmDocument {
            meta: DxmMeta {
                version: "1.0".to_string(),
                ..Default::default()
            },
            ..Default::default()
        };

        // Title
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Security Scan Report".to_string())],
            priority: None,
        }));

        // Score summary section
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 2,
            content: vec![InlineNode::Text("Score Summary".to_string())],
            priority: None,
        }));

        // Score with visual indicator
        let score = report.header.score;
        let score_indicator = Self::score_indicator(score);
        let score_text = format!("Security Score: {} {}", score, score_indicator);
        doc.nodes
            .push(DxmNode::Paragraph(vec![InlineNode::Bold(vec![InlineNode::Text(score_text)])]));

        // Score interpretation
        let interpretation = Self::score_interpretation(score);
        doc.nodes.push(DxmNode::SemanticBlock(SemanticBlockNode {
            block_type: if score >= 80 {
                SemanticBlockType::Info
            } else if score >= 50 {
                SemanticBlockType::Warning
            } else {
                SemanticBlockType::Warning
            },
            content: vec![InlineNode::Text(interpretation)],
            priority: None,
        }));

        // Timestamp
        let timestamp = Self::format_timestamp(report.header.timestamp);
        doc.nodes
            .push(DxmNode::Paragraph(vec![InlineNode::Text(format!("Generated: {}", timestamp))]));

        // Git hash
        let git_hash = Self::format_git_hash(&report.header.git_hash);
        doc.nodes
            .push(DxmNode::Paragraph(vec![InlineNode::Text(format!("Git Commit: {}", git_hash))]));

        // Findings section
        if !report.findings.is_empty() {
            doc.nodes.push(DxmNode::Header(HeaderNode {
                level: 2,
                content: vec![InlineNode::Text("Findings".to_string())],
                priority: None,
            }));

            // Summary counts
            let (critical, high, medium, low) = Self::count_by_severity(&report.findings);
            doc.nodes.push(DxmNode::Paragraph(vec![InlineNode::Text(format!(
                "Total: {} findings ({} critical, {} high, {} medium, {} low)",
                report.findings.len(),
                critical,
                high,
                medium,
                low
            ))]));

            // Findings table
            let table = Self::build_findings_table(&report.findings, path_map);
            doc.nodes.push(DxmNode::Table(table));

            // Remediation section
            doc.nodes.push(DxmNode::Header(HeaderNode {
                level: 2,
                content: vec![InlineNode::Text("Remediation Suggestions".to_string())],
                priority: None,
            }));

            let suggestions = Self::generate_remediation_suggestions(&report.findings);
            doc.nodes.push(DxmNode::List(ListNode {
                ordered: false,
                items: suggestions
                    .into_iter()
                    .map(|s| ListItem {
                        content: vec![InlineNode::Text(s)],
                        nested: None,
                    })
                    .collect(),
            }));
        } else {
            doc.nodes.push(DxmNode::SemanticBlock(SemanticBlockNode {
                block_type: SemanticBlockType::Info,
                content: vec![InlineNode::Text(
                    "No security findings detected. Great job!".to_string(),
                )],
                priority: None,
            }));
        }

        doc
    }

    /// Generate a visual score indicator
    fn score_indicator(score: u8) -> &'static str {
        match score {
            90..=100 => "🟢 Excellent",
            80..=89 => "🟢 Good",
            70..=79 => "🟡 Fair",
            50..=69 => "🟠 Needs Improvement",
            _ => "🔴 Critical",
        }
    }

    /// Generate score interpretation text
    fn score_interpretation(score: u8) -> String {
        match score {
            90..=100 => "Your codebase has excellent security posture. Continue monitoring for new vulnerabilities.".to_string(),
            80..=89 => "Your codebase has good security. Address any remaining findings to improve further.".to_string(),
            70..=79 => "Your codebase has fair security. Review and address the identified findings.".to_string(),
            50..=69 => "Your codebase needs security improvements. Prioritize fixing high and critical findings.".to_string(),
            _ => "Your codebase has critical security issues. Immediate action is required to address vulnerabilities.".to_string(),
        }
    }

    /// Format timestamp as human-readable string
    fn format_timestamp(timestamp: u64) -> String {
        // Simple UTC timestamp formatting
        let secs = timestamp;
        let days = secs / 86400;
        let years = 1970 + days / 365;
        let remaining_days = days % 365;
        let months = remaining_days / 30 + 1;
        let day = remaining_days % 30 + 1;
        format!("{:04}-{:02}-{:02} UTC", years, months, day)
    }

    /// Format git hash as hex string
    fn format_git_hash(hash: &[u8; 20]) -> String {
        if hash.iter().all(|&b| b == 0) {
            "N/A".to_string()
        } else {
            hash.iter().take(7).map(|b| format!("{:02x}", b)).collect::<String>()
        }
    }

    /// Count findings by severity
    fn count_by_severity(findings: &[BinaryFinding]) -> (usize, usize, usize, usize) {
        let mut critical = 0;
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        for finding in findings {
            match finding.severity {
                4 => critical += 1,
                3 => high += 1,
                2 => medium += 1,
                1 => low += 1,
                _ => {}
            }
        }

        (critical, high, medium, low)
    }

    /// Build findings table
    fn build_findings_table(
        findings: &[BinaryFinding],
        path_map: &std::collections::HashMap<u64, String>,
    ) -> TableNode {
        let schema = vec![
            ColumnDef {
                name: "Severity".to_string(),
                type_hint: None,
            },
            ColumnDef {
                name: "Type".to_string(),
                type_hint: None,
            },
            ColumnDef {
                name: "Location".to_string(),
                type_hint: None,
            },
            ColumnDef {
                name: "Description".to_string(),
                type_hint: None,
            },
        ];

        let rows: Vec<Vec<CellValue>> = findings
            .iter()
            .map(|f| {
                let severity = Self::severity_label(f.severity);
                let finding_type = Self::finding_type_label(f.finding_type);
                let location = path_map
                    .get(&f.file_hash)
                    .map(|p| format!("{}:{}", p, f.line_number))
                    .unwrap_or_else(|| format!("line {}", f.line_number));

                vec![
                    CellValue::Text(severity.to_string()),
                    CellValue::Text(finding_type.to_string()),
                    CellValue::Text(location),
                    CellValue::Text(f.message.clone()),
                ]
            })
            .collect();

        TableNode { schema, rows }
    }

    /// Get severity label
    fn severity_label(severity: u8) -> &'static str {
        match severity {
            4 => "🔴 Critical",
            3 => "🟠 High",
            2 => "🟡 Medium",
            1 => "🟢 Low",
            _ => "⚪ None",
        }
    }

    /// Get finding type label
    fn finding_type_label(finding_type: u8) -> &'static str {
        match finding_type {
            0 => "Vulnerability",
            1 => "Secret",
            2 => "Rule Violation",
            _ => "Unknown",
        }
    }

    /// Generate remediation suggestions based on findings
    fn generate_remediation_suggestions(findings: &[BinaryFinding]) -> Vec<String> {
        let mut suggestions = Vec::new();
        let mut has_secrets = false;
        let mut has_vulnerabilities = false;
        let mut has_critical = false;

        for finding in findings {
            if finding.finding_type == 1 {
                has_secrets = true;
            }
            if finding.finding_type == 0 {
                has_vulnerabilities = true;
            }
            if finding.severity == 4 {
                has_critical = true;
            }
        }

        if has_critical {
            suggestions.push(
                "Address critical findings immediately - they pose significant security risks"
                    .to_string(),
            );
        }

        if has_secrets {
            suggestions.push("Rotate any exposed secrets and API keys immediately".to_string());
            suggestions.push(
                "Add secret patterns to .gitignore and use environment variables".to_string(),
            );
            suggestions
                .push("Consider using a secrets manager for sensitive credentials".to_string());
        }

        if has_vulnerabilities {
            suggestions
                .push("Update vulnerable dependencies to their latest secure versions".to_string());
            suggestions.push(
                "Review CVE details for each vulnerability to understand the risk".to_string(),
            );
        }

        if suggestions.is_empty() {
            suggestions.push(
                "Continue regular security scanning to maintain security posture".to_string(),
            );
        }

        suggestions
    }

    /// Export markdown report to file
    pub fn export(report: &SecurityReport, path: &Path) -> Result<()> {
        let markdown = Self::generate(report);
        std::fs::write(path, markdown)?;
        Ok(())
    }

    /// Export markdown report with file paths to file
    pub fn export_with_paths(
        report: &SecurityReport,
        path_map: &std::collections::HashMap<u64, String>,
        path: &Path,
    ) -> Result<()> {
        let markdown = Self::generate_with_paths(report, path_map);
        std::fs::write(path, markdown)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_binary_finding_roundtrip() {
        let finding = BinaryFinding::new(
            FindingType::Secret,
            Severity::Critical,
            0x123456789ABCDEF0,
            42,
            10,
            "AWS access key detected".to_string(),
        );

        let bytes = finding.to_bytes();
        let (parsed, len) = BinaryFinding::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.finding_type, finding.finding_type);
        assert_eq!(parsed.severity, finding.severity);
        assert_eq!(parsed.file_hash, finding.file_hash);
        assert_eq!(parsed.line_number, finding.line_number);
        assert_eq!(parsed.column, finding.column);
        assert_eq!(parsed.message, finding.message);
        assert_eq!(len, bytes.len());
    }

    #[test]
    fn test_security_report_roundtrip() {
        let mut report = SecurityReport::new(85, [0u8; 20]);
        report.add_finding(BinaryFinding::new(
            FindingType::Vulnerability,
            Severity::High,
            0x1234,
            10,
            0,
            "CVE-2021-12345".to_string(),
        ));
        report.add_finding(BinaryFinding::new(
            FindingType::Secret,
            Severity::Critical,
            0x5678,
            20,
            5,
            "API key exposed".to_string(),
        ));

        let bytes = report.to_bytes();
        let parsed = SecurityReport::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.header.score, report.header.score);
        assert_eq!(parsed.findings.len(), 2);
        assert_eq!(parsed.findings[0].message, "CVE-2021-12345");
        assert_eq!(parsed.findings[1].message, "API key exposed");
    }

    #[test]
    fn test_export_import_unsigned() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.sr");

        let mut report = SecurityReport::new(90, [1u8; 20]);
        report.add_finding(BinaryFinding::new(
            FindingType::Secret,
            Severity::High,
            0xABCD,
            5,
            0,
            "Test finding".to_string(),
        ));

        ReportExporter::export_unsigned(&report, &path).unwrap();

        let (imported, signed) = ReportExporter::import(&path).unwrap();
        assert!(signed.is_none());
        assert_eq!(imported.header.score, 90);
        assert_eq!(imported.findings.len(), 1);
    }

    #[test]
    fn test_export_import_signed() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report_signed.sr");

        // Create keypair
        let secret_bytes: [u8; 32] = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1c, 0xae, 0x7f, 0x60,
        ];
        let key = SigningKey::from_bytes(&secret_bytes);

        let mut report = SecurityReport::new(75, [2u8; 20]);
        report.add_finding(BinaryFinding::new(
            FindingType::Vulnerability,
            Severity::Critical,
            0xDEAD,
            100,
            0,
            "Critical vulnerability".to_string(),
        ));

        ReportExporter::export(&report, &key, &path).unwrap();

        // Import and verify
        let imported = ReportExporter::import_verified(&path, &key.verifying_key()).unwrap();
        assert_eq!(imported.header.score, 75);
        assert_eq!(imported.findings.len(), 1);
    }

    #[test]
    fn test_hash_file_path() {
        let path1 = Path::new("src/main.rs");
        let path2 = Path::new("src/lib.rs");

        let hash1 = hash_file_path(path1);
        let hash2 = hash_file_path(path2);

        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash_file_path(path1)); // Deterministic
    }

    #[test]
    fn test_markdown_report_generation() {
        let mut report = SecurityReport::new(75, [0u8; 20]);
        report.add_finding(BinaryFinding::new(
            FindingType::Vulnerability,
            Severity::High,
            0x1234,
            10,
            0,
            "CVE-2021-12345: Buffer overflow".to_string(),
        ));
        report.add_finding(BinaryFinding::new(
            FindingType::Secret,
            Severity::Critical,
            0x5678,
            20,
            5,
            "AWS access key exposed".to_string(),
        ));

        let markdown = MarkdownReportGenerator::generate(&report);

        // Check that key sections are present
        assert!(markdown.contains("Security Scan Report"));
        assert!(markdown.contains("Score Summary"));
        assert!(markdown.contains("75"));
        assert!(markdown.contains("Findings"));
        assert!(markdown.contains("CVE-2021-12345"));
        assert!(markdown.contains("AWS access key"));
        assert!(markdown.contains("Remediation"));
    }

    #[test]
    fn test_markdown_report_with_paths() {
        let mut report = SecurityReport::new(90, [0u8; 20]);
        let file_hash = hash_file_path(Path::new("src/main.rs"));
        report.add_finding(BinaryFinding::new(
            FindingType::Secret,
            Severity::High,
            file_hash,
            42,
            0,
            "API key detected".to_string(),
        ));

        let mut path_map = std::collections::HashMap::new();
        path_map.insert(file_hash, "src/main.rs".to_string());

        let markdown = MarkdownReportGenerator::generate_with_paths(&report, &path_map);

        assert!(markdown.contains("src/main.rs:42"));
    }

    #[test]
    fn test_markdown_report_no_findings() {
        let report = SecurityReport::new(100, [0u8; 20]);
        let markdown = MarkdownReportGenerator::generate(&report);

        assert!(markdown.contains("Security Scan Report"));
        assert!(markdown.contains("100"));
        assert!(markdown.contains("Excellent"));
        assert!(markdown.contains("No security findings"));
    }

    #[test]
    fn test_markdown_export() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("report.md");

        let mut report = SecurityReport::new(85, [0u8; 20]);
        report.add_finding(BinaryFinding::new(
            FindingType::Vulnerability,
            Severity::Medium,
            0xABCD,
            5,
            0,
            "Test finding".to_string(),
        ));

        MarkdownReportGenerator::export(&report, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("Security Scan Report"));
        assert!(content.contains("85"));
    }

    #[test]
    fn test_score_indicators() {
        // Test various score ranges
        assert_eq!(MarkdownReportGenerator::score_indicator(100), "🟢 Excellent");
        assert_eq!(MarkdownReportGenerator::score_indicator(85), "🟢 Good");
        assert_eq!(MarkdownReportGenerator::score_indicator(75), "🟡 Fair");
        assert_eq!(MarkdownReportGenerator::score_indicator(60), "🟠 Needs Improvement");
        assert_eq!(MarkdownReportGenerator::score_indicator(30), "🔴 Critical");
    }

    #[test]
    fn test_severity_labels() {
        assert_eq!(MarkdownReportGenerator::severity_label(4), "🔴 Critical");
        assert_eq!(MarkdownReportGenerator::severity_label(3), "🟠 High");
        assert_eq!(MarkdownReportGenerator::severity_label(2), "🟡 Medium");
        assert_eq!(MarkdownReportGenerator::severity_label(1), "🟢 Low");
    }
}
