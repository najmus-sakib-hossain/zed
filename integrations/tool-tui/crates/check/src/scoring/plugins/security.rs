//! Security Scoring Plugin
//!
//! Detects security vulnerabilities and secrets in code.

use crate::scoring::plugin::{RuleDefinition, ScoringPlugin};
use crate::scoring_impl::{Category, Severity, Violation};
use std::any::Any;
use std::path::Path;

/// Security plugin for vulnerability and secret detection
pub struct SecurityPlugin {
    rules: Vec<RuleDefinition>,
    patterns: Vec<SecurityPattern>,
}

struct SecurityPattern {
    rule_id: &'static str,
    regex: regex::Regex,
    severity: Severity,
    message: &'static str,
    extensions: Option<&'static [&'static str]>,
}

impl SecurityPlugin {
    /// Create a new security plugin
    #[must_use]
    pub fn new() -> Self {
        let rules = vec![
            // Secret detection
            RuleDefinition::new("security/aws-access-key", "AWS Access Key", Category::Security)
                .with_severity(Severity::Critical)
                .with_description("Hardcoded AWS access key detected"),
            RuleDefinition::new("security/aws-secret-key", "AWS Secret Key", Category::Security)
                .with_severity(Severity::Critical)
                .with_description("Hardcoded AWS secret key detected"),
            RuleDefinition::new("security/github-token", "GitHub Token", Category::Security)
                .with_severity(Severity::Critical)
                .with_description("Hardcoded GitHub token detected"),
            RuleDefinition::new("security/private-key", "Private Key", Category::Security)
                .with_severity(Severity::Critical)
                .with_description("Private key content detected"),
            RuleDefinition::new("security/api-key", "Generic API Key", Category::Security)
                .with_severity(Severity::High)
                .with_description("Potential API key detected"),
            // Vulnerability patterns
            RuleDefinition::new("security/sql-injection", "SQL Injection Risk", Category::Security)
                .with_severity(Severity::Critical)
                .with_description("Potential SQL injection vulnerability"),
            RuleDefinition::new(
                "security/command-injection",
                "Command Injection",
                Category::Security,
            )
            .with_severity(Severity::Critical)
            .with_description("Potential command injection vulnerability"),
            RuleDefinition::new("security/xss", "Cross-Site Scripting", Category::Security)
                .with_severity(Severity::High)
                .with_description("Potential XSS vulnerability"),
            RuleDefinition::new("security/eval", "Dangerous Eval", Category::Security)
                .with_severity(Severity::High)
                .with_description("Use of eval() or similar dangerous function"),
            RuleDefinition::new(
                "security/unsafe-deserialization",
                "Unsafe Deserialization",
                Category::Security,
            )
            .with_severity(Severity::High)
            .with_description("Potentially unsafe deserialization"),
            RuleDefinition::new("security/weak-crypto", "Weak Cryptography", Category::Security)
                .with_severity(Severity::Medium)
                .with_description("Use of weak cryptographic algorithm"),
            RuleDefinition::new(
                "security/hardcoded-password",
                "Hardcoded Password",
                Category::Security,
            )
            .with_severity(Severity::High)
            .with_description("Hardcoded password detected"),
        ];

        let patterns = vec![
            // AWS Keys
            SecurityPattern {
                rule_id: "security/aws-access-key",
                regex: regex::Regex::new(r"(?i)AKIA[0-9A-Z]{16}").unwrap(),
                severity: Severity::Critical,
                message: "AWS Access Key ID detected",
                extensions: None,
            },
            SecurityPattern {
                rule_id: "security/aws-secret-key",
                regex: regex::Regex::new(r#"(?i)aws.{0,20}secret.{0,20}['"][0-9a-zA-Z/+]{40}['"]"#).unwrap(),
                severity: Severity::Critical,
                message: "AWS Secret Access Key detected",
                extensions: None,
            },
            // GitHub tokens
            SecurityPattern {
                rule_id: "security/github-token",
                regex: regex::Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
                severity: Severity::Critical,
                message: "GitHub token detected",
                extensions: None,
            },
            // Private keys
            SecurityPattern {
                rule_id: "security/private-key",
                regex: regex::Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap(),
                severity: Severity::Critical,
                message: "Private key detected in code",
                extensions: None,
            },
            // Generic API keys
            SecurityPattern {
                rule_id: "security/api-key",
                regex: regex::Regex::new(r#"(?i)api[_-]?key\s*[=:]\s*['"][a-zA-Z0-9_\-]{20,}['"]"#).unwrap(),
                severity: Severity::High,
                message: "Potential API key detected",
                extensions: None,
            },
            // SQL Injection - JavaScript/Python
            SecurityPattern {
                rule_id: "security/sql-injection",
                regex: regex::Regex::new(r#"(?i)(execute|query|raw)\s*\(\s*['"`].*\+.*['"`]|f['"].*\{.*\}.*(?:SELECT|INSERT|UPDATE|DELETE)"#).unwrap(),
                severity: Severity::Critical,
                message: "Potential SQL injection: use parameterized queries",
                extensions: Some(&["js", "ts", "py", "rb"]),
            },
            // Command injection
            SecurityPattern {
                rule_id: "security/command-injection",
                regex: regex::Regex::new(r"(?i)(exec|system|popen|subprocess\.call|subprocess\.run|child_process)\s*\([^)]*\+").unwrap(),
                severity: Severity::Critical,
                message: "Potential command injection: avoid string concatenation in shell commands",
                extensions: Some(&["js", "ts", "py", "rb", "php"]),
            },
            // XSS - innerHTML
            SecurityPattern {
                rule_id: "security/xss",
                regex: regex::Regex::new(r"(?i)(innerHTML|outerHTML)\s*=|dangerouslySetInnerHTML").unwrap(),
                severity: Severity::High,
                message: "Potential XSS: avoid innerHTML with user input",
                extensions: Some(&["js", "jsx", "ts", "tsx"]),
            },
            // Eval usage
            SecurityPattern {
                rule_id: "security/eval",
                regex: regex::Regex::new(r"\b(eval|Function)\s*\(").unwrap(),
                severity: Severity::High,
                message: "Avoid using eval() - it can execute arbitrary code",
                extensions: Some(&["js", "ts", "py"]),
            },
            // Python pickle (unsafe deserialization)
            SecurityPattern {
                rule_id: "security/unsafe-deserialization",
                regex: regex::Regex::new(r"pickle\.(load|loads)\s*\(").unwrap(),
                severity: Severity::High,
                message: "pickle.load() can execute arbitrary code - use json or safer alternatives",
                extensions: Some(&["py"]),
            },
            // YAML unsafe load
            SecurityPattern {
                rule_id: "security/unsafe-deserialization",
                regex: regex::Regex::new(r"yaml\.(load|unsafe_load)\s*\([^)]*\)\s*(?!\s*,\s*Loader)").unwrap(),
                severity: Severity::High,
                message: "yaml.load() without Loader is unsafe - use yaml.safe_load()",
                extensions: Some(&["py"]),
            },
            // Weak crypto - MD5/SHA1
            SecurityPattern {
                rule_id: "security/weak-crypto",
                regex: regex::Regex::new(r"(?i)(md5|sha1)\s*\(").unwrap(),
                severity: Severity::Medium,
                message: "MD5/SHA1 are weak - use SHA256 or stronger",
                extensions: None,
            },
            // Hardcoded passwords
            SecurityPattern {
                rule_id: "security/hardcoded-password",
                regex: regex::Regex::new(r#"(?i)password\s*[=:]\s*['"][^'"]{8,}['"]"#).unwrap(),
                severity: Severity::High,
                message: "Hardcoded password detected - use environment variables",
                extensions: None,
            },
        ];

        Self { rules, patterns }
    }
}

impl Default for SecurityPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ScoringPlugin for SecurityPlugin {
    fn id(&self) -> &'static str {
        "security"
    }

    fn name(&self) -> &'static str {
        "Security Analysis"
    }

    fn category(&self) -> Category {
        Category::Security
    }

    fn analyze(&self, path: &Path, content: &[u8], _ast: Option<&dyn Any>) -> Vec<Violation> {
        let mut violations = Vec::new();

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let text = match std::str::from_utf8(content) {
            Ok(t) => t,
            Err(_) => return violations, // Skip binary files
        };

        for (line_num, line) in text.lines().enumerate() {
            for pattern in &self.patterns {
                // Check if pattern applies to this file extension
                if let Some(exts) = pattern.extensions
                    && !exts.contains(&ext)
                {
                    continue;
                }

                if pattern.regex.is_match(line) {
                    violations.push(Violation {
                        category: Category::Security,
                        severity: pattern.severity,
                        file: path.to_path_buf(),
                        line: (line_num + 1) as u32,
                        column: 1,
                        rule_id: pattern.rule_id.to_string(),
                        message: pattern.message.to_string(),
                        points: pattern.severity.points(),
                    });
                }
            }
        }

        violations
    }

    fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    fn description(&self) -> &'static str {
        "Detects security vulnerabilities, hardcoded secrets, and dangerous code patterns"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_plugin_creation() {
        let plugin = SecurityPlugin::new();
        assert_eq!(plugin.id(), "security");
        assert_eq!(plugin.category(), Category::Security);
        assert!(!plugin.rules().is_empty());
    }

    #[test]
    fn test_aws_key_detection() {
        let plugin = SecurityPlugin::new();
        let content = b"const key = 'AKIAIOSFODNN7EXAMPLE';";
        let path = Path::new("test.js");

        let violations = plugin.analyze(path, content, None);
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.rule_id == "security/aws-access-key"));
    }

    #[test]
    fn test_eval_detection() {
        let plugin = SecurityPlugin::new();
        let content = b"eval(userInput);";
        let path = Path::new("test.js");

        let violations = plugin.analyze(path, content, None);
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.rule_id == "security/eval"));
    }

    #[test]
    fn test_private_key_detection() {
        let plugin = SecurityPlugin::new();
        let content = b"-----BEGIN RSA PRIVATE KEY-----\nMIIE...";
        let path = Path::new("test.txt");

        let violations = plugin.analyze(path, content, None);
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.rule_id == "security/private-key"));
    }
}
