//! dx-pkg-audit: Security Vulnerability Auditing
//!
//! Provides:
//! - Vulnerability database checking
//! - Severity reporting
//! - Remediation suggestions
//! - Deprecation warnings

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Severity levels for vulnerabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Moderate,
    High,
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => write!(f, "low"),
            Severity::Moderate => write!(f, "moderate"),
            Severity::High => write!(f, "high"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// A security vulnerability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    /// Unique identifier (e.g., CVE-2021-12345 or GHSA-xxxx)
    pub id: String,
    /// Severity level
    pub severity: Severity,
    /// Short title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Affected package name
    pub package_name: String,
    /// Affected version range (semver)
    pub affected_versions: String,
    /// Fixed version (if available)
    pub patched_versions: Option<String>,
    /// Recommendation for remediation
    pub recommendation: String,
    /// URL for more information
    pub url: Option<String>,
}

/// Deprecation warning for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprecationWarning {
    /// Package name
    pub package_name: String,
    /// Package version
    pub version: String,
    /// Deprecation message
    pub message: String,
    /// Suggested replacement package
    pub replacement: Option<String>,
}

/// Result of an audit operation
#[derive(Debug, Clone, Default)]
pub struct AuditReport {
    /// Found vulnerabilities
    pub vulnerabilities: Vec<Vulnerability>,
    /// Deprecation warnings
    pub deprecations: Vec<DeprecationWarning>,
    /// Packages scanned
    pub packages_scanned: usize,
    /// Total dependencies checked
    pub dependencies_checked: usize,
}

impl AuditReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the audit passed (no critical/high vulnerabilities)
    pub fn passed(&self) -> bool {
        !self
            .vulnerabilities
            .iter()
            .any(|v| matches!(v.severity, Severity::Critical | Severity::High))
    }

    /// Count vulnerabilities by severity
    pub fn count_by_severity(&self) -> HashMap<Severity, usize> {
        let mut counts = HashMap::new();
        for vuln in &self.vulnerabilities {
            *counts.entry(vuln.severity).or_insert(0) += 1;
        }
        counts
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        let counts = self.count_by_severity();
        let critical = counts.get(&Severity::Critical).unwrap_or(&0);
        let high = counts.get(&Severity::High).unwrap_or(&0);
        let moderate = counts.get(&Severity::Moderate).unwrap_or(&0);
        let low = counts.get(&Severity::Low).unwrap_or(&0);

        format!(
            "{} vulnerabilities found ({} critical, {} high, {} moderate, {} low)",
            self.vulnerabilities.len(),
            critical,
            high,
            moderate,
            low
        )
    }
}

/// Package auditor for checking vulnerabilities
pub struct PackageAuditor {
    /// Known vulnerabilities database (in-memory for now)
    vulnerability_db: Vec<Vulnerability>,
    /// Known deprecations
    deprecation_db: HashMap<String, DeprecationWarning>,
}

impl PackageAuditor {
    /// Create a new auditor with default vulnerability database
    pub fn new() -> Self {
        Self {
            vulnerability_db: Self::load_default_vulnerabilities(),
            deprecation_db: Self::load_default_deprecations(),
        }
    }

    /// Audit a single package
    pub fn audit_package(&self, name: &str, version: &str) -> Vec<Vulnerability> {
        self.vulnerability_db
            .iter()
            .filter(|v| {
                v.package_name == name && Self::version_matches(version, &v.affected_versions)
            })
            .cloned()
            .collect()
    }

    /// Check if a package is deprecated
    pub fn check_deprecation(&self, name: &str, version: &str) -> Option<DeprecationWarning> {
        let key = format!("{}@{}", name, version);
        self.deprecation_db
            .get(&key)
            .cloned()
            .or_else(|| self.deprecation_db.get(name).cloned())
    }

    /// Audit all packages in a lockfile
    pub fn audit_lockfile(&self, lockfile_path: &Path) -> Result<AuditReport> {
        let mut report = AuditReport::new();

        // Read lockfile
        let content = std::fs::read_to_string(lockfile_path)?;
        let packages = Self::parse_lockfile(&content)?;

        report.packages_scanned = packages.len();
        report.dependencies_checked = packages.len();

        for (name, version) in &packages {
            // Check vulnerabilities
            let vulns = self.audit_package(name, version);
            report.vulnerabilities.extend(vulns);

            // Check deprecations
            if let Some(deprecation) = self.check_deprecation(name, version) {
                report.deprecations.push(deprecation);
            }
        }

        Ok(report)
    }

    /// Audit packages from package.json dependencies
    pub fn audit_dependencies(&self, deps: &HashMap<String, String>) -> AuditReport {
        let mut report = AuditReport::new();
        report.packages_scanned = deps.len();
        report.dependencies_checked = deps.len();

        for (name, version) in deps {
            // Strip version prefix (^, ~, etc.)
            let clean_version = version.trim_start_matches(['^', '~', '>', '<', '=']);

            let vulns = self.audit_package(name, clean_version);
            report.vulnerabilities.extend(vulns);

            if let Some(deprecation) = self.check_deprecation(name, clean_version) {
                report.deprecations.push(deprecation);
            }
        }

        report
    }

    /// Check if a version matches an affected version range
    fn version_matches(version: &str, affected: &str) -> bool {
        // Simple version matching - in production, use semver crate
        if affected == "*" {
            return true;
        }

        // Handle ranges like "<1.2.3" or ">=1.0.0 <2.0.0"
        if affected.starts_with('<') {
            let target = affected.trim_start_matches('<').trim_start_matches('=').trim();
            return Self::compare_versions(version, target) < 0;
        }

        if affected.starts_with('>') {
            let target = affected.trim_start_matches('>').trim_start_matches('=').trim();
            return Self::compare_versions(version, target) > 0;
        }

        // Exact match or prefix match
        version.starts_with(affected) || affected.starts_with(version)
    }

    /// Simple version comparison
    fn compare_versions(a: &str, b: &str) -> i32 {
        let parse = |s: &str| -> Vec<u32> { s.split('.').filter_map(|p| p.parse().ok()).collect() };

        let va = parse(a);
        let vb = parse(b);

        for i in 0..va.len().max(vb.len()) {
            let pa = va.get(i).unwrap_or(&0);
            let pb = vb.get(i).unwrap_or(&0);
            if pa < pb {
                return -1;
            }
            if pa > pb {
                return 1;
            }
        }
        0
    }

    /// Parse a lockfile to extract package names and versions
    fn parse_lockfile(content: &str) -> Result<Vec<(String, String)>> {
        let mut packages = Vec::new();

        // Try to parse as JSON (dx.lock format)
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(content) {
            if let Some(deps) = json.get("packages").and_then(|p| p.as_object()) {
                for (name, info) in deps {
                    if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                        packages.push((name.clone(), version.to_string()));
                    }
                }
            }
            return Ok(packages);
        }

        // Try to parse as simple key=value format
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((name, version)) = line.split_once('=') {
                packages.push((name.trim().to_string(), version.trim().to_string()));
            } else if let Some((name, version)) = line.split_once('@') {
                // Handle name@version format
                if !name.is_empty() {
                    packages.push((name.to_string(), version.to_string()));
                }
            }
        }

        Ok(packages)
    }

    /// Load default vulnerability database
    fn load_default_vulnerabilities() -> Vec<Vulnerability> {
        // In production, this would fetch from a vulnerability database API
        // For now, include some well-known vulnerabilities
        vec![
            Vulnerability {
                id: "GHSA-93q8-gq69-wqmw".to_string(),
                severity: Severity::Critical,
                title: "Prototype Pollution in lodash".to_string(),
                description: "Versions of lodash before 4.17.12 are vulnerable to Prototype Pollution.".to_string(),
                package_name: "lodash".to_string(),
                affected_versions: "<4.17.12".to_string(),
                patched_versions: Some(">=4.17.12".to_string()),
                recommendation: "Upgrade to lodash@4.17.21 or later".to_string(),
                url: Some("https://github.com/advisories/GHSA-93q8-gq69-wqmw".to_string()),
            },
            Vulnerability {
                id: "CVE-2021-23337".to_string(),
                severity: Severity::High,
                title: "Command Injection in lodash".to_string(),
                description: "Lodash versions prior to 4.17.21 are vulnerable to Command Injection via the template function.".to_string(),
                package_name: "lodash".to_string(),
                affected_versions: "<4.17.21".to_string(),
                patched_versions: Some(">=4.17.21".to_string()),
                recommendation: "Upgrade to lodash@4.17.21 or later".to_string(),
                url: Some("https://nvd.nist.gov/vuln/detail/CVE-2021-23337".to_string()),
            },
            Vulnerability {
                id: "CVE-2022-0155".to_string(),
                severity: Severity::High,
                title: "Exposure of Sensitive Information in follow-redirects".to_string(),
                description: "follow-redirects is vulnerable to Exposure of Sensitive Information to an Unauthorized Actor.".to_string(),
                package_name: "follow-redirects".to_string(),
                affected_versions: "<1.14.7".to_string(),
                patched_versions: Some(">=1.14.7".to_string()),
                recommendation: "Upgrade to follow-redirects@1.14.7 or later".to_string(),
                url: Some("https://nvd.nist.gov/vuln/detail/CVE-2022-0155".to_string()),
            },
            Vulnerability {
                id: "CVE-2021-3807".to_string(),
                severity: Severity::High,
                title: "Regular Expression Denial of Service in ansi-regex".to_string(),
                description: "ansi-regex is vulnerable to Inefficient Regular Expression Complexity.".to_string(),
                package_name: "ansi-regex".to_string(),
                affected_versions: "<5.0.1".to_string(),
                patched_versions: Some(">=5.0.1".to_string()),
                recommendation: "Upgrade to ansi-regex@5.0.1 or later".to_string(),
                url: Some("https://nvd.nist.gov/vuln/detail/CVE-2021-3807".to_string()),
            },
            Vulnerability {
                id: "CVE-2020-7598".to_string(),
                severity: Severity::Moderate,
                title: "Prototype Pollution in minimist".to_string(),
                description: "minimist before 1.2.2 could be tricked into adding or modifying properties of Object.prototype.".to_string(),
                package_name: "minimist".to_string(),
                affected_versions: "<1.2.2".to_string(),
                patched_versions: Some(">=1.2.2".to_string()),
                recommendation: "Upgrade to minimist@1.2.6 or later".to_string(),
                url: Some("https://nvd.nist.gov/vuln/detail/CVE-2020-7598".to_string()),
            },
        ]
    }

    /// Load default deprecation database
    fn load_default_deprecations() -> HashMap<String, DeprecationWarning> {
        let mut db = HashMap::new();

        // Well-known deprecated packages
        db.insert(
            "request".to_string(),
            DeprecationWarning {
                package_name: "request".to_string(),
                version: "*".to_string(),
                message: "request has been deprecated since February 2020".to_string(),
                replacement: Some("node-fetch, axios, or got".to_string()),
            },
        );

        db.insert(
            "uuid@3".to_string(),
            DeprecationWarning {
                package_name: "uuid".to_string(),
                version: "3.x".to_string(),
                message: "uuid@3 is deprecated, please upgrade to uuid@8 or later".to_string(),
                replacement: Some("uuid@9".to_string()),
            },
        );

        db.insert(
            "mkdirp@0".to_string(),
            DeprecationWarning {
                package_name: "mkdirp".to_string(),
                version: "0.x".to_string(),
                message: "mkdirp@0 is deprecated, use fs.mkdir with recursive option or mkdirp@1+"
                    .to_string(),
                replacement: Some("mkdirp@3 or fs.mkdir({ recursive: true })".to_string()),
            },
        );

        db.insert(
            "querystring".to_string(),
            DeprecationWarning {
                package_name: "querystring".to_string(),
                version: "*".to_string(),
                message: "querystring is deprecated, use URLSearchParams instead".to_string(),
                replacement: Some("URLSearchParams (built-in)".to_string()),
            },
        );

        db.insert(
            "colors".to_string(),
            DeprecationWarning {
                package_name: "colors".to_string(),
                version: "*".to_string(),
                message: "colors package was compromised in v1.4.1, use alternatives".to_string(),
                replacement: Some("chalk, picocolors, or kleur".to_string()),
            },
        );

        db.insert(
            "faker".to_string(),
            DeprecationWarning {
                package_name: "faker".to_string(),
                version: "*".to_string(),
                message: "faker was deprecated by its maintainer, use @faker-js/faker".to_string(),
                replacement: Some("@faker-js/faker".to_string()),
            },
        );

        db.insert(
            "node-uuid".to_string(),
            DeprecationWarning {
                package_name: "node-uuid".to_string(),
                version: "*".to_string(),
                message: "node-uuid is deprecated, use uuid package instead".to_string(),
                replacement: Some("uuid".to_string()),
            },
        );

        db.insert(
            "nomnom".to_string(),
            DeprecationWarning {
                package_name: "nomnom".to_string(),
                version: "*".to_string(),
                message: "nomnom is deprecated, use commander or yargs".to_string(),
                replacement: Some("commander or yargs".to_string()),
            },
        );

        db.insert(
            "optimist".to_string(),
            DeprecationWarning {
                package_name: "optimist".to_string(),
                version: "*".to_string(),
                message: "optimist is deprecated, use yargs or minimist".to_string(),
                replacement: Some("yargs or minimist".to_string()),
            },
        );

        db.insert(
            "jade".to_string(),
            DeprecationWarning {
                package_name: "jade".to_string(),
                version: "*".to_string(),
                message: "jade has been renamed to pug".to_string(),
                replacement: Some("pug".to_string()),
            },
        );

        db.insert(
            "istanbul".to_string(),
            DeprecationWarning {
                package_name: "istanbul".to_string(),
                version: "*".to_string(),
                message: "istanbul is deprecated, use nyc or c8".to_string(),
                replacement: Some("nyc or c8".to_string()),
            },
        );

        db.insert(
            "left-pad".to_string(),
            DeprecationWarning {
                package_name: "left-pad".to_string(),
                version: "*".to_string(),
                message: "left-pad is deprecated, use String.prototype.padStart()".to_string(),
                replacement: Some("String.prototype.padStart() (built-in)".to_string()),
            },
        );

        db.insert(
            "core-js@2".to_string(),
            DeprecationWarning {
                package_name: "core-js".to_string(),
                version: "2.x".to_string(),
                message: "core-js@2 is deprecated, upgrade to core-js@3".to_string(),
                replacement: Some("core-js@3".to_string()),
            },
        );

        db.insert(
            "babel-core".to_string(),
            DeprecationWarning {
                package_name: "babel-core".to_string(),
                version: "*".to_string(),
                message: "babel-core is deprecated, use @babel/core".to_string(),
                replacement: Some("@babel/core".to_string()),
            },
        );

        db.insert(
            "babel-preset-env".to_string(),
            DeprecationWarning {
                package_name: "babel-preset-env".to_string(),
                version: "*".to_string(),
                message: "babel-preset-env is deprecated, use @babel/preset-env".to_string(),
                replacement: Some("@babel/preset-env".to_string()),
            },
        );

        db
    }
}

impl Default for PackageAuditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Print audit report to console with colors
pub fn print_audit_report(report: &AuditReport) {
    println!("\n📋 Security Audit Report");
    println!("========================\n");

    if report.vulnerabilities.is_empty() && report.deprecations.is_empty() {
        println!("✅ No vulnerabilities or deprecations found!");
        println!("   Scanned {} packages\n", report.packages_scanned);
        return;
    }

    // Print vulnerabilities by severity
    let mut vulns_by_severity: Vec<_> = report.vulnerabilities.iter().collect();
    vulns_by_severity.sort_by(|a, b| b.severity.cmp(&a.severity));

    if !vulns_by_severity.is_empty() {
        println!("🔴 Vulnerabilities Found:\n");

        for vuln in vulns_by_severity {
            let severity_icon = match vuln.severity {
                Severity::Critical => "🔴",
                Severity::High => "🟠",
                Severity::Moderate => "🟡",
                Severity::Low => "🟢",
            };

            println!("{} {} [{}]", severity_icon, vuln.title, vuln.severity);
            println!("   Package: {}@{}", vuln.package_name, vuln.affected_versions);
            println!("   ID: {}", vuln.id);
            if let Some(ref patched) = vuln.patched_versions {
                println!("   Fix: Upgrade to {}", patched);
            }
            println!("   Recommendation: {}", vuln.recommendation);
            if let Some(ref url) = vuln.url {
                println!("   More info: {}", url);
            }
            println!();
        }
    }

    // Print deprecations
    if !report.deprecations.is_empty() {
        println!("⚠️  Deprecation Warnings:\n");

        for dep in &report.deprecations {
            println!("⚠️  {} is deprecated", dep.package_name);
            println!("   {}", dep.message);
            if let Some(ref replacement) = dep.replacement {
                println!("   Consider using: {}", replacement);
            }
            println!();
        }
    }

    // Print summary
    println!("📊 Summary");
    println!("   {}", report.summary());
    println!("   Packages scanned: {}", report.packages_scanned);

    if report.passed() {
        println!("\n✅ Audit passed (no critical or high severity vulnerabilities)");
    } else {
        println!("\n❌ Audit failed - please address critical and high severity vulnerabilities");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Moderate);
        assert!(Severity::Moderate > Severity::Low);
    }

    #[test]
    fn test_audit_package() {
        let auditor = PackageAuditor::new();

        // Should find vulnerabilities in old lodash
        let vulns = auditor.audit_package("lodash", "4.17.10");
        assert!(!vulns.is_empty());

        // Should not find vulnerabilities in new lodash
        let vulns = auditor.audit_package("lodash", "4.17.21");
        assert!(vulns.is_empty());
    }

    #[test]
    fn test_version_matches() {
        assert!(PackageAuditor::version_matches("4.17.10", "<4.17.12"));
        assert!(!PackageAuditor::version_matches("4.17.21", "<4.17.12"));
        assert!(PackageAuditor::version_matches("1.0.0", "*"));
    }

    #[test]
    fn test_audit_report_summary() {
        let mut report = AuditReport::new();
        report.vulnerabilities.push(Vulnerability {
            id: "TEST-001".to_string(),
            severity: Severity::Critical,
            title: "Test vulnerability".to_string(),
            description: "Test".to_string(),
            package_name: "test".to_string(),
            affected_versions: "*".to_string(),
            patched_versions: None,
            recommendation: "Test".to_string(),
            url: None,
        });

        assert!(!report.passed());
        assert!(report.summary().contains("1 vulnerabilities"));
    }

    #[test]
    fn test_check_deprecation() {
        let auditor = PackageAuditor::new();

        let dep = auditor.check_deprecation("request", "2.88.0");
        assert!(dep.is_some());

        let dep = auditor.check_deprecation("express", "4.18.0");
        assert!(dep.is_none());
    }
}

/// JSON-serializable audit report for CI/CD integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReportJson {
    pub vulnerabilities: Vec<VulnerabilityJson>,
    pub deprecations: Vec<DeprecationWarning>,
    pub metadata: AuditMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityJson {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub package_name: String,
    pub affected_versions: String,
    pub patched_versions: Option<String>,
    pub recommendation: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    pub packages_scanned: usize,
    pub dependencies_checked: usize,
    pub vulnerabilities_found: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub moderate_count: usize,
    pub low_count: usize,
    pub passed: bool,
}

impl AuditReport {
    /// Convert to JSON-serializable format
    pub fn to_json(&self) -> AuditReportJson {
        let counts = self.count_by_severity();

        AuditReportJson {
            vulnerabilities: self
                .vulnerabilities
                .iter()
                .map(|v| VulnerabilityJson {
                    id: v.id.clone(),
                    severity: v.severity.to_string(),
                    title: v.title.clone(),
                    description: v.description.clone(),
                    package_name: v.package_name.clone(),
                    affected_versions: v.affected_versions.clone(),
                    patched_versions: v.patched_versions.clone(),
                    recommendation: v.recommendation.clone(),
                    url: v.url.clone(),
                })
                .collect(),
            deprecations: self.deprecations.clone(),
            metadata: AuditMetadata {
                packages_scanned: self.packages_scanned,
                dependencies_checked: self.dependencies_checked,
                vulnerabilities_found: self.vulnerabilities.len(),
                critical_count: *counts.get(&Severity::Critical).unwrap_or(&0),
                high_count: *counts.get(&Severity::High).unwrap_or(&0),
                moderate_count: *counts.get(&Severity::Moderate).unwrap_or(&0),
                low_count: *counts.get(&Severity::Low).unwrap_or(&0),
                passed: self.passed(),
            },
        }
    }

    /// Export report as JSON string
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.to_json())
    }

    /// Export report as SARIF format (for GitHub Security tab)
    pub fn to_sarif(&self) -> serde_json::Value {
        let results: Vec<serde_json::Value> = self
            .vulnerabilities
            .iter()
            .map(|v| {
                serde_json::json!({
                    "ruleId": v.id,
                    "level": match v.severity {
                        Severity::Critical => "error",
                        Severity::High => "error",
                        Severity::Moderate => "warning",
                        Severity::Low => "note",
                    },
                    "message": {
                        "text": format!("{}: {}", v.title, v.description)
                    },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": format!("node_modules/{}/package.json", v.package_name)
                            }
                        }
                    }],
                    "fixes": v.patched_versions.as_ref().map(|p| vec![
                        serde_json::json!({
                            "description": {
                                "text": v.recommendation.clone()
                            },
                            "artifactChanges": [{
                                "artifactLocation": {
                                    "uri": "package.json"
                                },
                                "replacements": [{
                                    "deletedRegion": {},
                                    "insertedContent": {
                                        "text": format!("\"{}\"", p)
                                    }
                                }]
                            }]
                        })
                    ])
                })
            })
            .collect();

        let rules: Vec<serde_json::Value> = self
            .vulnerabilities
            .iter()
            .map(|v| {
                serde_json::json!({
                    "id": v.id,
                    "name": v.title,
                    "shortDescription": {
                        "text": v.title
                    },
                    "fullDescription": {
                        "text": v.description
                    },
                    "helpUri": v.url,
                    "defaultConfiguration": {
                        "level": match v.severity {
                            Severity::Critical => "error",
                            Severity::High => "error",
                            Severity::Moderate => "warning",
                            Severity::Low => "note",
                        }
                    }
                })
            })
            .collect();

        serde_json::json!({
            "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "dx-audit",
                        "version": "1.0.0",
                        "informationUri": "https://dx.dev/audit",
                        "rules": rules
                    }
                },
                "results": results
            }]
        })
    }
}

/// Print audit report in different formats
pub fn print_audit_report_format(report: &AuditReport, format: &str) {
    match format {
        "json" => {
            if let Ok(json) = report.to_json_string() {
                println!("{}", json);
            }
        }
        "sarif" => {
            let sarif = report.to_sarif();
            if let Ok(json) = serde_json::to_string_pretty(&sarif) {
                println!("{}", json);
            }
        }
        _ => {
            print_audit_report(report);
        }
    }
}
