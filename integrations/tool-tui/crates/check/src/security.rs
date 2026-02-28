//! Security Scanner Module
//!
//! Detects security vulnerabilities and unsafe patterns in code.
//! Integrates with the 500-point scoring system (Security category).
//!
//! **Requirements: 3.1, 3.2**

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use crate::scoring_impl::Severity;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// Security vulnerability pattern database
pub struct VulnerabilityDatabase {
    /// Regex patterns for vulnerability detection
    patterns: Vec<VulnerabilityPattern>,
    /// Secret detection patterns
    secret_patterns: Vec<SecretPattern>,
    /// Language-specific unsafe patterns
    language_patterns: HashMap<String, Vec<LanguagePattern>>,
}

/// A vulnerability pattern
#[derive(Clone)]
pub struct VulnerabilityPattern {
    pub id: String,
    pub name: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub message: String,
    pub remediation: String,
    pub cwe_id: Option<String>,
    pub owasp_category: Option<String>,
}

/// A secret detection pattern
#[derive(Clone)]
pub struct SecretPattern {
    pub id: String,
    pub name: String,
    pub pattern: Regex,
    pub entropy_threshold: Option<f64>,
    pub severity: Severity,
    pub message: String,
}

/// Language-specific unsafe pattern
#[derive(Clone)]
pub struct LanguagePattern {
    pub id: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub message: String,
}

impl VulnerabilityDatabase {
    /// Create a new vulnerability database with default patterns
    #[must_use]
    pub fn new() -> Self {
        let mut db = Self {
            patterns: Vec::new(),
            secret_patterns: Vec::new(),
            language_patterns: HashMap::new(),
        };

        db.load_default_patterns();
        db.load_secret_patterns();
        db.load_language_patterns();

        db
    }

    /// Load default vulnerability patterns
    fn load_default_patterns(&mut self) {
        // SQL Injection patterns
        self.patterns.push(VulnerabilityPattern {
            id: "sql-injection".to_string(),
            name: "SQL Injection".to_string(),
            pattern: Regex::new(r"(?i)(execute|query|exec)\s*\([^)]*\+[^)]*\)")
                .expect("Invalid regex"),
            severity: Severity::Critical,
            message: "Potential SQL injection vulnerability detected".to_string(),
            remediation: "Use parameterized queries or prepared statements".to_string(),
            cwe_id: Some("CWE-89".to_string()),
            owasp_category: Some("A03:2021-Injection".to_string()),
        });

        // Command Injection patterns
        self.patterns.push(VulnerabilityPattern {
            id: "command-injection".to_string(),
            name: "Command Injection".to_string(),
            pattern: Regex::new(r"(?i)(exec|system|spawn|shell)\s*\(\s*.*\+.*\)")
                .expect("Invalid regex"),
            severity: Severity::Critical,
            message: "Potential command injection vulnerability detected".to_string(),
            remediation: "Avoid shell execution with user input, use safe APIs".to_string(),
            cwe_id: Some("CWE-78".to_string()),
            owasp_category: Some("A03:2021-Injection".to_string()),
        });

        // Path Traversal patterns
        self.patterns.push(VulnerabilityPattern {
            id: "path-traversal".to_string(),
            name: "Path Traversal".to_string(),
            pattern: Regex::new(r"(?i)(readFile|writeFile|open)\s*\(\s*.*\+.*\)")
                .expect("Invalid regex"),
            severity: Severity::High,
            message: "Potential path traversal vulnerability detected".to_string(),
            remediation: "Validate and sanitize file paths, use path.join()".to_string(),
            cwe_id: Some("CWE-22".to_string()),
            owasp_category: Some("A01:2021-Broken Access Control".to_string()),
        });

        // XSS patterns
        self.patterns.push(VulnerabilityPattern {
            id: "xss-innerHTML".to_string(),
            name: "XSS via innerHTML".to_string(),
            pattern: Regex::new(r"\.innerHTML\s*=").expect("Invalid regex"),
            severity: Severity::High,
            message: "Potential XSS vulnerability via innerHTML".to_string(),
            remediation: "Use textContent or sanitize HTML input".to_string(),
            cwe_id: Some("CWE-79".to_string()),
            owasp_category: Some("A03:2021-Injection".to_string()),
        });

        // Insecure Deserialization
        self.patterns.push(VulnerabilityPattern {
            id: "insecure-deserialization".to_string(),
            name: "Insecure Deserialization".to_string(),
            pattern: Regex::new(r"(?i)(pickle\.loads|yaml\.load|unserialize)\s*\(")
                .expect("Invalid regex"),
            severity: Severity::Critical,
            message: "Insecure deserialization detected".to_string(),
            remediation: "Use safe deserialization methods (yaml.safe_load, etc.)".to_string(),
            cwe_id: Some("CWE-502".to_string()),
            owasp_category: Some("A08:2021-Software and Data Integrity Failures".to_string()),
        });

        // Weak Cryptography
        self.patterns.push(VulnerabilityPattern {
            id: "weak-crypto".to_string(),
            name: "Weak Cryptography".to_string(),
            pattern: Regex::new(r"(?i)(md5|sha1|des|rc4)\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "Weak cryptographic algorithm detected".to_string(),
            remediation: "Use strong algorithms (SHA-256, AES-256, etc.)".to_string(),
            cwe_id: Some("CWE-327".to_string()),
            owasp_category: Some("A02:2021-Cryptographic Failures".to_string()),
        });
    }

    /// Load secret detection patterns
    fn load_secret_patterns(&mut self) {
        // ===== AWS Secrets =====

        // AWS Access Key
        self.secret_patterns.push(SecretPattern {
            id: "aws-access-key".to_string(),
            name: "AWS Access Key".to_string(),
            pattern: Regex::new(r"AKIA[0-9A-Z]{16}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "AWS Access Key detected".to_string(),
        });

        // AWS Secret Key
        self.secret_patterns.push(SecretPattern {
            id: "aws-secret-key".to_string(),
            name: "AWS Secret Key".to_string(),
            pattern: Regex::new(
                r#"(?i)aws[_-]?secret[_-]?key['"]\s*[:=]\s*['"][0-9a-zA-Z/+=]{40}['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "AWS Secret Key detected".to_string(),
        });

        // AWS Session Token
        self.secret_patterns.push(SecretPattern {
            id: "aws-session-token".to_string(),
            name: "AWS Session Token".to_string(),
            pattern: Regex::new(
                r#"(?i)aws[_-]?session[_-]?token['":\s]*[:=]\s*['"]([a-zA-Z0-9/+=]{100,})['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::High,
            message: "AWS Session Token detected".to_string(),
        });

        // ===== Google Cloud Secrets =====

        // Google Cloud API Key
        self.secret_patterns.push(SecretPattern {
            id: "google-api-key".to_string(),
            name: "Google API Key".to_string(),
            pattern: Regex::new(r"AIza[0-9A-Za-z\-_]{35}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Google Cloud API Key detected".to_string(),
        });

        // Google OAuth Access Token
        self.secret_patterns.push(SecretPattern {
            id: "google-oauth-token".to_string(),
            name: "Google OAuth Token".to_string(),
            pattern: Regex::new(r"ya29\.[0-9A-Za-z\-_]+").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Google OAuth Access Token detected".to_string(),
        });

        // Google Cloud Service Account Key
        self.secret_patterns.push(SecretPattern {
            id: "google-service-account".to_string(),
            name: "Google Service Account".to_string(),
            pattern: Regex::new(r#"(?i)"type"\s*:\s*"service_account""#).expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Google Cloud Service Account JSON detected".to_string(),
        });

        // ===== Azure Secrets =====

        // Azure Storage Account Key
        self.secret_patterns.push(SecretPattern {
            id: "azure-storage-key".to_string(),
            name: "Azure Storage Key".to_string(),
            pattern: Regex::new(
                r"(?i)(?:DefaultEndpointsProtocol|AccountKey)\s*=\s*[a-zA-Z0-9+/=]{88}",
            )
            .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Azure Storage Account Key detected".to_string(),
        });

        // Azure Client Secret
        self.secret_patterns.push(SecretPattern {
            id: "azure-client-secret".to_string(),
            name: "Azure Client Secret".to_string(),
            pattern: Regex::new(r#"(?i)(?:client[_-]?secret|azure[_-]?secret)['":\s]*[:=]\s*['"]([0-9a-zA-Z\-_~.]{34,})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Azure Client Secret detected".to_string(),
        });

        // Azure Subscription Key
        self.secret_patterns.push(SecretPattern {
            id: "azure-subscription-key".to_string(),
            name: "Azure Subscription Key".to_string(),
            pattern: Regex::new(r#"(?i)(?:ocp-apim-subscription-key|subscription[_-]?key)['":\s]*[:=]\s*['"]([0-9a-f]{32})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Azure Subscription Key detected".to_string(),
        });

        // ===== GitHub Secrets =====

        // GitHub Personal Access Token
        self.secret_patterns.push(SecretPattern {
            id: "github-token".to_string(),
            name: "GitHub Token".to_string(),
            pattern: Regex::new(r"ghp_[0-9a-zA-Z]{36}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "GitHub Personal Access Token detected".to_string(),
        });

        // GitHub OAuth Token
        self.secret_patterns.push(SecretPattern {
            id: "github-oauth-token".to_string(),
            name: "GitHub OAuth Token".to_string(),
            pattern: Regex::new(r"gho_[0-9a-zA-Z]{36}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "GitHub OAuth Access Token detected".to_string(),
        });

        // GitHub App Token
        self.secret_patterns.push(SecretPattern {
            id: "github-app-token".to_string(),
            name: "GitHub App Token".to_string(),
            pattern: Regex::new(r"(?:ghu|ghs)_[0-9a-zA-Z]{36}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "GitHub App Token detected".to_string(),
        });

        // GitHub Refresh Token
        self.secret_patterns.push(SecretPattern {
            id: "github-refresh-token".to_string(),
            name: "GitHub Refresh Token".to_string(),
            pattern: Regex::new(r"ghr_[0-9a-zA-Z]{36}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "GitHub Refresh Token detected".to_string(),
        });

        // ===== Database Credentials =====

        // Generic Database Connection String
        self.secret_patterns.push(SecretPattern {
            id: "database-connection-string".to_string(),
            name: "Database Connection String".to_string(),
            pattern: Regex::new(
                r"(?i)(?:mongodb|mysql|postgresql|postgres|mssql|oracle)://[^:]+:[^@]+@",
            )
            .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Database connection string with credentials detected".to_string(),
        });

        // PostgreSQL Connection String
        self.secret_patterns.push(SecretPattern {
            id: "postgres-connection".to_string(),
            name: "PostgreSQL Connection".to_string(),
            pattern: Regex::new(r"(?i)(?:postgres|postgresql)://[a-zA-Z0-9_-]+:[^@\s]+@[a-zA-Z0-9.-]+(?::\d+)?/[a-zA-Z0-9_-]+")
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "PostgreSQL connection string with password detected".to_string(),
        });

        // MySQL Connection String
        self.secret_patterns.push(SecretPattern {
            id: "mysql-connection".to_string(),
            name: "MySQL Connection".to_string(),
            pattern: Regex::new(
                r"(?i)mysql://[a-zA-Z0-9_-]+:[^@\s]+@[a-zA-Z0-9.-]+(?::\d+)?/[a-zA-Z0-9_-]+",
            )
            .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "MySQL connection string with password detected".to_string(),
        });

        // MongoDB Connection String
        self.secret_patterns.push(SecretPattern {
            id: "mongodb-connection".to_string(),
            name: "MongoDB Connection".to_string(),
            pattern: Regex::new(r"(?i)mongodb(?:\+srv)?://[a-zA-Z0-9_-]+:[^@\s]+@")
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "MongoDB connection string with password detected".to_string(),
        });

        // Redis Connection String
        self.secret_patterns.push(SecretPattern {
            id: "redis-connection".to_string(),
            name: "Redis Connection".to_string(),
            pattern: Regex::new(r"(?i)redis://[^:]*:[^@\s]+@").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Redis connection string with password detected".to_string(),
        });

        // Database Password in Config
        self.secret_patterns.push(SecretPattern {
            id: "database-password".to_string(),
            name: "Database Password".to_string(),
            pattern: Regex::new(r#"(?i)(?:db|database)[_-]?(?:password|passwd|pwd)['":\s]*[:=]\s*['"]([^'"]{8,})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: Some(3.5),
            severity: Severity::Critical,
            message: "Database password detected in configuration".to_string(),
        });

        // ===== Private Keys =====

        // RSA Private Key
        self.secret_patterns.push(SecretPattern {
            id: "rsa-private-key".to_string(),
            name: "RSA Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN RSA PRIVATE KEY-----").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "RSA private key detected in source code".to_string(),
        });

        // EC Private Key
        self.secret_patterns.push(SecretPattern {
            id: "ec-private-key".to_string(),
            name: "EC Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN EC PRIVATE KEY-----").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "EC private key detected in source code".to_string(),
        });

        // DSA Private Key
        self.secret_patterns.push(SecretPattern {
            id: "dsa-private-key".to_string(),
            name: "DSA Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN DSA PRIVATE KEY-----").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "DSA private key detected in source code".to_string(),
        });

        // OpenSSH Private Key
        self.secret_patterns.push(SecretPattern {
            id: "openssh-private-key".to_string(),
            name: "OpenSSH Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN OPENSSH PRIVATE KEY-----").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "OpenSSH private key detected in source code".to_string(),
        });

        // PGP Private Key
        self.secret_patterns.push(SecretPattern {
            id: "pgp-private-key".to_string(),
            name: "PGP Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN PGP PRIVATE KEY BLOCK-----").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "PGP private key detected in source code".to_string(),
        });

        // Generic Private Key
        self.secret_patterns.push(SecretPattern {
            id: "private-key".to_string(),
            name: "Private Key".to_string(),
            pattern: Regex::new(r"-----BEGIN PRIVATE KEY-----").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Private key detected in source code".to_string(),
        });

        // ===== OAuth and JWT Tokens =====

        // JWT Token
        self.secret_patterns.push(SecretPattern {
            id: "jwt-token".to_string(),
            name: "JWT Token".to_string(),
            pattern: Regex::new(r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::High,
            message: "JWT token detected in source code".to_string(),
        });

        // OAuth Access Token (generic)
        self.secret_patterns.push(SecretPattern {
            id: "oauth-access-token".to_string(),
            name: "OAuth Access Token".to_string(),
            pattern: Regex::new(r#"(?i)(?:access[_-]?token|bearer[_-]?token)['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~+/]{40,})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: Some(4.0),
            severity: Severity::High,
            message: "OAuth access token detected".to_string(),
        });

        // OAuth Refresh Token
        self.secret_patterns.push(SecretPattern {
            id: "oauth-refresh-token".to_string(),
            name: "OAuth Refresh Token".to_string(),
            pattern: Regex::new(
                r#"(?i)refresh[_-]?token['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~+/]{40,})['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: Some(4.0),
            severity: Severity::High,
            message: "OAuth refresh token detected".to_string(),
        });

        // OAuth Client Secret
        self.secret_patterns.push(SecretPattern {
            id: "oauth-client-secret".to_string(),
            name: "OAuth Client Secret".to_string(),
            pattern: Regex::new(r#"(?i)(?:client[_-]?secret|consumer[_-]?secret)['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~]{32,})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "OAuth client secret detected".to_string(),
        });

        // JWT Secret Key
        self.secret_patterns.push(SecretPattern {
            id: "jwt-secret".to_string(),
            name: "JWT Secret".to_string(),
            pattern: Regex::new(
                r#"(?i)jwt[_-]?secret['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~+/=]{32,})['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: Some(4.0),
            severity: Severity::Critical,
            message: "JWT secret key detected".to_string(),
        });

        // ===== Other API Keys =====

        // Stripe API Key
        self.secret_patterns.push(SecretPattern {
            id: "stripe-key".to_string(),
            name: "Stripe API Key".to_string(),
            pattern: Regex::new(r"sk_live_[0-9a-zA-Z]{24,}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Stripe Live API Key detected".to_string(),
        });

        // Stripe Restricted Key
        self.secret_patterns.push(SecretPattern {
            id: "stripe-restricted-key".to_string(),
            name: "Stripe Restricted Key".to_string(),
            pattern: Regex::new(r"rk_live_[0-9a-zA-Z]{24,}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Stripe Restricted API Key detected".to_string(),
        });

        // Slack Token
        self.secret_patterns.push(SecretPattern {
            id: "slack-token".to_string(),
            name: "Slack Token".to_string(),
            pattern: Regex::new(r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24,}")
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::High,
            message: "Slack API token detected".to_string(),
        });

        // Slack Webhook
        self.secret_patterns.push(SecretPattern {
            id: "slack-webhook".to_string(),
            name: "Slack Webhook".to_string(),
            pattern: Regex::new(
                r"https://hooks\.slack\.com/services/T[a-zA-Z0-9_]+/B[a-zA-Z0-9_]+/[a-zA-Z0-9_]+",
            )
            .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::High,
            message: "Slack webhook URL detected".to_string(),
        });

        // Twilio API Key
        self.secret_patterns.push(SecretPattern {
            id: "twilio-api-key".to_string(),
            name: "Twilio API Key".to_string(),
            pattern: Regex::new(r"SK[0-9a-fA-F]{32}").expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "Twilio API Key detected".to_string(),
        });

        // SendGrid API Key
        self.secret_patterns.push(SecretPattern {
            id: "sendgrid-api-key".to_string(),
            name: "SendGrid API Key".to_string(),
            pattern: Regex::new(r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}")
                .expect("Invalid regex"),
            entropy_threshold: None,
            severity: Severity::Critical,
            message: "SendGrid API Key detected".to_string(),
        });

        // ===== Generic High-Entropy Patterns =====

        // Generic Password
        self.secret_patterns.push(SecretPattern {
            id: "generic-password".to_string(),
            name: "Generic Password".to_string(),
            pattern: Regex::new(r#"(?i)(?:password|passwd|pwd)['":\s]*[:=]\s*['"]([^'"]{8,})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: Some(3.5),
            severity: Severity::Medium,
            message: "Password detected in source code".to_string(),
        });

        // Generic API Key
        self.secret_patterns.push(SecretPattern {
            id: "generic-api-key".to_string(),
            name: "Generic API Key".to_string(),
            pattern: Regex::new(
                r#"(?i)api[_-]?key['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~+/=]{32,})['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: Some(4.0),
            severity: Severity::High,
            message: "API key detected in source code".to_string(),
        });

        // Generic Secret
        self.secret_patterns.push(SecretPattern {
            id: "generic-secret".to_string(),
            name: "Generic Secret".to_string(),
            pattern: Regex::new(
                r#"(?i)(?:secret|secret[_-]?key)['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~+/=]{32,})['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: Some(4.0),
            severity: Severity::High,
            message: "Secret key detected in source code".to_string(),
        });

        // Generic Token
        self.secret_patterns.push(SecretPattern {
            id: "generic-token".to_string(),
            name: "Generic Token".to_string(),
            pattern: Regex::new(
                r#"(?i)(?:auth[_-]?token|token)['":\s]*[:=]\s*['"]([a-zA-Z0-9\-._~+/=]{32,})['"]"#,
            )
            .expect("Invalid regex"),
            entropy_threshold: Some(4.0),
            severity: Severity::Medium,
            message: "Authentication token detected in source code".to_string(),
        });

        // High-entropy string (catch-all for potential secrets)
        self.secret_patterns.push(SecretPattern {
            id: "high-entropy-string".to_string(),
            name: "High Entropy String".to_string(),
            pattern: Regex::new(r#"(?i)(?:key|secret|password|token|credential)['":\s]*[:=]\s*['"]([a-zA-Z0-9+/=]{32,})['"]"#)
                .expect("Invalid regex"),
            entropy_threshold: Some(4.5),
            severity: Severity::Medium,
            message: "High-entropy string detected (potential secret)".to_string(),
        });
    }

    /// Load language-specific patterns
    fn load_language_patterns(&mut self) {
        // JavaScript/TypeScript patterns
        let mut js_patterns = Vec::new();

        // eval() detection
        js_patterns.push(LanguagePattern {
            id: "js-eval".to_string(),
            pattern: Regex::new(r"\beval\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "Use of eval() is dangerous and should be avoided".to_string(),
        });

        // Function constructor detection
        js_patterns.push(LanguagePattern {
            id: "js-function-constructor".to_string(),
            pattern: Regex::new(r"new\s+Function\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "Function constructor can execute arbitrary code".to_string(),
        });

        // innerHTML detection (XSS risk)
        js_patterns.push(LanguagePattern {
            id: "js-inner-html".to_string(),
            pattern: Regex::new(r"\.innerHTML\s*=").expect("Invalid regex"),
            severity: Severity::High,
            message: "innerHTML assignment can lead to XSS vulnerabilities, use textContent or sanitize input".to_string(),
        });

        // dangerouslySetInnerHTML detection (React)
        js_patterns.push(LanguagePattern {
            id: "js-dangerously-set-inner-html".to_string(),
            pattern: Regex::new(r"dangerouslySetInnerHTML").expect("Invalid regex"),
            severity: Severity::High,
            message: "dangerouslySetInnerHTML can lead to XSS vulnerabilities, sanitize HTML or use safe alternatives".to_string(),
        });

        // document.write detection
        js_patterns.push(LanguagePattern {
            id: "js-document-write".to_string(),
            pattern: Regex::new(r"document\.write\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "document.write() can be exploited for XSS attacks".to_string(),
        });

        self.language_patterns.insert("javascript".to_string(), js_patterns.clone());
        self.language_patterns.insert("typescript".to_string(), js_patterns);

        // Python patterns
        let mut py_patterns = Vec::new();

        // eval() detection
        py_patterns.push(LanguagePattern {
            id: "py-eval".to_string(),
            pattern: Regex::new(r"\beval\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "Use of eval() is dangerous and can execute arbitrary code".to_string(),
        });

        // exec() detection
        py_patterns.push(LanguagePattern {
            id: "py-exec".to_string(),
            pattern: Regex::new(r"\bexec\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "Use of exec() is dangerous and can execute arbitrary code".to_string(),
        });

        // pickle.loads() detection
        py_patterns.push(LanguagePattern {
            id: "py-pickle-loads".to_string(),
            pattern: Regex::new(r"pickle\.loads\s*\(").expect("Invalid regex"),
            severity: Severity::Critical,
            message: "pickle.loads() can execute arbitrary code during deserialization, use safer alternatives".to_string(),
        });

        // yaml.load() detection (unsafe)
        py_patterns.push(LanguagePattern {
            id: "py-yaml-load".to_string(),
            pattern: Regex::new(r"yaml\.load\s*\(").expect("Invalid regex"),
            severity: Severity::Critical,
            message: "yaml.load() can execute arbitrary code, use yaml.safe_load() instead"
                .to_string(),
        });

        // yaml.unsafe_load() detection
        py_patterns.push(LanguagePattern {
            id: "py-yaml-unsafe-load".to_string(),
            pattern: Regex::new(r"yaml\.unsafe_load\s*\(").expect("Invalid regex"),
            severity: Severity::Critical,
            message: "yaml.unsafe_load() can execute arbitrary code".to_string(),
        });

        // compile() with user input
        py_patterns.push(LanguagePattern {
            id: "py-compile".to_string(),
            pattern: Regex::new(r"\bcompile\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "compile() with untrusted input can be dangerous".to_string(),
        });

        // __import__() detection
        py_patterns.push(LanguagePattern {
            id: "py-import".to_string(),
            pattern: Regex::new(r"__import__\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "__import__() with user input can be exploited".to_string(),
        });

        self.language_patterns.insert("python".to_string(), py_patterns);

        // Rust patterns
        let mut rs_patterns = Vec::new();

        // Unsafe block without SAFETY comment (simplified check)
        rs_patterns.push(LanguagePattern {
            id: "rs-unsafe-undocumented".to_string(),
            pattern: Regex::new(r"unsafe\s*\{").expect("Invalid regex"),
            severity: Severity::High,
            message:
                "Unsafe block should be documented with a SAFETY comment explaining why it's safe"
                    .to_string(),
        });

        // Unsafe function without documentation
        rs_patterns.push(LanguagePattern {
            id: "rs-unsafe-fn-undocumented".to_string(),
            pattern: Regex::new(r"unsafe\s+fn\s+\w+").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "Unsafe function should be documented explaining safety requirements"
                .to_string(),
        });

        // Unsafe trait implementation
        rs_patterns.push(LanguagePattern {
            id: "rs-unsafe-impl".to_string(),
            pattern: Regex::new(r"unsafe\s+impl").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "Unsafe trait implementation should be carefully reviewed and documented"
                .to_string(),
        });

        // Raw pointer dereference
        rs_patterns.push(LanguagePattern {
            id: "rs-raw-pointer-deref".to_string(),
            pattern: Regex::new(r"\*(?:const|mut)\s+\w+").expect("Invalid regex"),
            severity: Severity::Low,
            message: "Raw pointer usage requires unsafe block and careful validation".to_string(),
        });

        self.language_patterns.insert("rust".to_string(), rs_patterns);

        // Go patterns
        let mut go_patterns = Vec::new();

        // unsafe package import
        go_patterns.push(LanguagePattern {
            id: "go-unsafe-import".to_string(),
            pattern: Regex::new(r#"import\s+(?:"unsafe"|_\s+"unsafe")"#).expect("Invalid regex"),
            severity: Severity::High,
            message:
                "Use of unsafe package bypasses Go's type safety and should be carefully reviewed"
                    .to_string(),
        });

        // unsafe.Pointer usage
        go_patterns.push(LanguagePattern {
            id: "go-unsafe-pointer".to_string(),
            pattern: Regex::new(r"unsafe\.Pointer").expect("Invalid regex"),
            severity: Severity::High,
            message: "unsafe.Pointer can lead to memory corruption if used incorrectly".to_string(),
        });

        // Arbitrary pointer arithmetic
        go_patterns.push(LanguagePattern {
            id: "go-uintptr-arithmetic".to_string(),
            pattern: Regex::new(r"uintptr\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "Converting pointers to uintptr for arithmetic is unsafe and error-prone"
                .to_string(),
        });

        // reflect package with unsafe
        go_patterns.push(LanguagePattern {
            id: "go-reflect-unsafe".to_string(),
            pattern: Regex::new(r"reflect\..*Unsafe").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "Unsafe reflection operations should be carefully reviewed".to_string(),
        });

        self.language_patterns.insert("go".to_string(), go_patterns);

        // C/C++ patterns
        let mut cpp_patterns = Vec::new();

        // strcpy - buffer overflow risk
        cpp_patterns.push(LanguagePattern {
            id: "cpp-strcpy".to_string(),
            pattern: Regex::new(r"\bstrcpy\s*\(").expect("Invalid regex"),
            severity: Severity::Critical,
            message: "strcpy() can cause buffer overflows, use strncpy() or safer alternatives like strlcpy()".to_string(),
        });

        // gets - always unsafe
        cpp_patterns.push(LanguagePattern {
            id: "cpp-gets".to_string(),
            pattern: Regex::new(r"\bgets\s*\(").expect("Invalid regex"),
            severity: Severity::Critical,
            message: "gets() is inherently unsafe and deprecated, use fgets() instead".to_string(),
        });

        // strcat - buffer overflow risk
        cpp_patterns.push(LanguagePattern {
            id: "cpp-strcat".to_string(),
            pattern: Regex::new(r"\bstrcat\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "strcat() can cause buffer overflows, use strncat() or safer alternatives"
                .to_string(),
        });

        // sprintf - buffer overflow risk
        cpp_patterns.push(LanguagePattern {
            id: "cpp-sprintf".to_string(),
            pattern: Regex::new(r"\bsprintf\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "sprintf() can cause buffer overflows, use snprintf() instead".to_string(),
        });

        // scanf without width specifier
        cpp_patterns.push(LanguagePattern {
            id: "cpp-scanf".to_string(),
            pattern: Regex::new(r"\bscanf\s*\([^)]*%s").expect("Invalid regex"),
            severity: Severity::High,
            message: "scanf() with %s without width specifier can cause buffer overflows"
                .to_string(),
        });

        // strncpy without null termination check
        cpp_patterns.push(LanguagePattern {
            id: "cpp-strncpy".to_string(),
            pattern: Regex::new(r"\bstrncpy\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "strncpy() may not null-terminate, ensure proper null termination".to_string(),
        });

        // memcpy with overlapping regions
        cpp_patterns.push(LanguagePattern {
            id: "cpp-memcpy".to_string(),
            pattern: Regex::new(r"\bmemcpy\s*\(").expect("Invalid regex"),
            severity: Severity::Low,
            message:
                "memcpy() with overlapping memory regions is undefined behavior, use memmove()"
                    .to_string(),
        });

        // alloca - stack overflow risk
        cpp_patterns.push(LanguagePattern {
            id: "cpp-alloca".to_string(),
            pattern: Regex::new(r"\balloca\s*\(").expect("Invalid regex"),
            severity: Severity::High,
            message: "alloca() can cause stack overflow and is not portable, use malloc() or VLAs carefully".to_string(),
        });

        // Unchecked malloc/calloc
        cpp_patterns.push(LanguagePattern {
            id: "cpp-unchecked-malloc".to_string(),
            pattern: Regex::new(r"=\s*(?:malloc|calloc|realloc)\s*\(").expect("Invalid regex"),
            severity: Severity::Medium,
            message: "Memory allocation should be checked for NULL before use".to_string(),
        });

        // Use after free pattern (simplified detection)
        cpp_patterns.push(LanguagePattern {
            id: "cpp-double-free".to_string(),
            pattern: Regex::new(r"free\s*\([^)]+\);\s*free\s*\(").expect("Invalid regex"),
            severity: Severity::Critical,
            message: "Potential double-free detected, can lead to memory corruption".to_string(),
        });

        self.language_patterns.insert("c".to_string(), cpp_patterns.clone());
        self.language_patterns.insert("cpp".to_string(), cpp_patterns);
    }
}

impl Default for VulnerabilityDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Security scanner with pattern matching and entropy analysis
pub struct SecurityScanner {
    database: VulnerabilityDatabase,
}

impl SecurityScanner {
    /// Create a new security scanner
    #[must_use]
    pub fn new() -> Self {
        Self {
            database: VulnerabilityDatabase::new(),
        }
    }

    /// Scan source code for security vulnerabilities
    #[must_use]
    pub fn scan(&self, source: &str, file_path: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Detect language from file extension
        let language = self.detect_language(file_path);

        // Scan for general vulnerability patterns
        diagnostics.extend(self.scan_vulnerabilities(source, file_path));

        // Scan for secrets
        diagnostics.extend(self.scan_secrets(source, file_path));

        // Scan for language-specific patterns
        if let Some(lang) = language {
            diagnostics.extend(self.scan_language_patterns(source, file_path, &lang));
        }

        diagnostics
    }

    /// Detect language from file extension
    fn detect_language(&self, path: &Path) -> Option<String> {
        path.extension().and_then(|ext| ext.to_str()).and_then(|ext| match ext {
            "js" | "jsx" | "mjs" => Some("javascript".to_string()),
            "ts" | "tsx" => Some("typescript".to_string()),
            "py" | "pyi" => Some("python".to_string()),
            "rs" => Some("rust".to_string()),
            "go" => Some("go".to_string()),
            "c" | "h" => Some("c".to_string()),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some("cpp".to_string()),
            _ => None,
        })
    }

    /// Scan for general vulnerability patterns
    fn scan_vulnerabilities(&self, source: &str, file_path: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for pattern in &self.database.patterns {
            for capture in pattern.pattern.find_iter(source) {
                let line = source[..capture.start()].lines().count() as u32;
                let column = source[..capture.start()].lines().last().map_or(0, |l| l.len() as u32);

                diagnostics.push(Diagnostic {
                    severity: match pattern.severity {
                        Severity::Critical => DiagnosticSeverity::Error,
                        Severity::High => DiagnosticSeverity::Error,
                        Severity::Medium => DiagnosticSeverity::Warning,
                        Severity::Low => DiagnosticSeverity::Info,
                    },
                    message: format!(
                        "{}\nRemediation: {}\nCWE: {}\nOWASP: {}",
                        pattern.message,
                        pattern.remediation,
                        pattern.cwe_id.as_deref().unwrap_or("N/A"),
                        pattern.owasp_category.as_deref().unwrap_or("N/A")
                    ),
                    rule_id: pattern.id.clone(),
                    file: file_path.to_path_buf(),
                    span: Span {
                        start: line,
                        end: column,
                    },
                    suggestion: Some(pattern.remediation.clone()),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// Scan for hardcoded secrets
    fn scan_secrets(&self, source: &str, file_path: &Path) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for pattern in &self.database.secret_patterns {
            for capture in pattern.pattern.find_iter(source) {
                let matched_text = capture.as_str();

                // Check entropy if threshold is set
                if let Some(threshold) = pattern.entropy_threshold {
                    let entropy = self.calculate_entropy(matched_text);
                    if entropy < threshold {
                        continue;
                    }
                }

                let line = source[..capture.start()].lines().count() as u32;
                let column = source[..capture.start()].lines().last().map_or(0, |l| l.len() as u32);

                diagnostics.push(Diagnostic {
                    severity: match pattern.severity {
                        Severity::Critical => DiagnosticSeverity::Error,
                        Severity::High => DiagnosticSeverity::Error,
                        Severity::Medium => DiagnosticSeverity::Warning,
                        Severity::Low => DiagnosticSeverity::Info,
                    },
                    message: format!(
                        "{}\nRemove hardcoded secrets and use environment variables or secret management",
                        pattern.message
                    ),
                    rule_id: pattern.id.clone(),
                    file: file_path.to_path_buf(),
                    span: Span {
                        start: line,
                        end: column,
                    },
                    suggestion: Some("Use environment variables or secret management".to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// Scan for language-specific patterns
    fn scan_language_patterns(
        &self,
        source: &str,
        file_path: &Path,
        language: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Some(patterns) = self.database.language_patterns.get(language) {
            for pattern in patterns {
                for capture in pattern.pattern.find_iter(source) {
                    let line = source[..capture.start()].lines().count() as u32;
                    let column =
                        source[..capture.start()].lines().last().map_or(0, |l| l.len() as u32);

                    diagnostics.push(Diagnostic {
                        severity: match pattern.severity {
                            Severity::Critical => DiagnosticSeverity::Error,
                            Severity::High => DiagnosticSeverity::Error,
                            Severity::Medium => DiagnosticSeverity::Warning,
                            Severity::Low => DiagnosticSeverity::Info,
                        },
                        message: pattern.message.clone(),
                        rule_id: pattern.id.clone(),
                        file: file_path.to_path_buf(),
                        span: Span {
                            start: line,
                            end: column,
                        },
                        suggestion: None,
                        related: Vec::new(),
                        fix: None,
                    });
                }
            }
        }

        diagnostics
    }

    /// Calculate Shannon entropy of a string
    fn calculate_entropy(&self, s: &str) -> f64 {
        if s.is_empty() {
            return 0.0;
        }

        let mut freq: HashMap<char, usize> = HashMap::new();
        for c in s.chars() {
            *freq.entry(c).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for count in freq.values() {
            let p = *count as f64 / len;
            entropy -= p * p.log2();
        }

        entropy
    }

    /// Get the vulnerability database (for testing/inspection)
    #[must_use]
    pub fn database(&self) -> &VulnerabilityDatabase {
        &self.database
    }
}

impl Default for SecurityScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_scanner_creation() {
        let scanner = SecurityScanner::new();
        assert!(!scanner.database.patterns.is_empty());
        assert!(!scanner.database.secret_patterns.is_empty());
    }

    #[test]
    fn test_detect_sql_injection() {
        let scanner = SecurityScanner::new();
        let source = r#"query("SELECT * FROM users WHERE id = " + userId)"#;
        let diagnostics = scanner.scan(source, Path::new("test.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("sql")));
    }

    #[test]
    fn test_detect_aws_key() {
        let scanner = SecurityScanner::new();
        let source = r#"const key = "AKIAIOSFODNN7EXAMPLE";"#;
        let diagnostics = scanner.scan(source, Path::new("test.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("aws")));
    }

    #[test]
    fn test_detect_eval_javascript() {
        let scanner = SecurityScanner::new();
        let source = r#"eval("console.log('test')");"#;
        let diagnostics = scanner.scan(source, Path::new("test.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("eval")));
    }

    #[test]
    fn test_detect_inner_html_javascript() {
        let scanner = SecurityScanner::new();
        let source = r#"element.innerHTML = userInput;"#;
        let diagnostics = scanner.scan(source, Path::new("test.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("inner-html")));
    }

    #[test]
    fn test_detect_dangerously_set_inner_html() {
        let scanner = SecurityScanner::new();
        let source = r#"<div dangerouslySetInnerHTML={{__html: content}} />"#;
        let diagnostics = scanner.scan(source, Path::new("test.jsx"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("dangerously")));
    }

    #[test]
    fn test_detect_eval_python() {
        let scanner = SecurityScanner::new();
        let source = r#"eval("print('test')")"#;
        let diagnostics = scanner.scan(source, Path::new("test.py"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("eval")));
    }

    #[test]
    fn test_detect_exec_python() {
        let scanner = SecurityScanner::new();
        let source = r#"exec("import os")"#;
        let diagnostics = scanner.scan(source, Path::new("test.py"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("exec")));
    }

    #[test]
    fn test_detect_pickle_loads_python() {
        let scanner = SecurityScanner::new();
        let source = r#"import pickle; data = pickle.loads(user_data)"#;
        let diagnostics = scanner.scan(source, Path::new("test.py"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("pickle")));
    }

    #[test]
    fn test_detect_yaml_load_python() {
        let scanner = SecurityScanner::new();
        let source = r#"import yaml; data = yaml.load(file)"#;
        let diagnostics = scanner.scan(source, Path::new("test.py"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("yaml")));
    }

    #[test]
    fn test_detect_unsafe_rust() {
        let scanner = SecurityScanner::new();
        let source = r#"
            unsafe {
                let x = *ptr;
            }
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.rs"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("unsafe")));
    }

    #[test]
    fn test_unsafe_rust_with_safety_comment() {
        let scanner = SecurityScanner::new();
        let source = r#"
            unsafe {
                // SAFETY: ptr is valid and aligned
                let x = *ptr;
            }
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.rs"));

        // Will still detect unsafe block, but message reminds to document
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("unsafe")));
    }

    #[test]
    fn test_detect_unsafe_go() {
        let scanner = SecurityScanner::new();
        let source = r#"
            import "unsafe"
            
            func main() {
                ptr := unsafe.Pointer(&x)
            }
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.go"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("unsafe")));
    }

    #[test]
    fn test_detect_strcpy_c() {
        let scanner = SecurityScanner::new();
        let source = r#"
            char dest[10];
            strcpy(dest, src);
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.c"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("strcpy")));
    }

    #[test]
    fn test_detect_gets_c() {
        let scanner = SecurityScanner::new();
        let source = r#"
            char buffer[100];
            gets(buffer);
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.c"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("gets")));
    }

    #[test]
    fn test_detect_sprintf_cpp() {
        let scanner = SecurityScanner::new();
        let source = r#"
            char buffer[100];
            sprintf(buffer, "%s", input);
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.cpp"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("sprintf")));
    }

    #[test]
    fn test_entropy_calculation() {
        let scanner = SecurityScanner::new();

        // Low entropy (repeated characters)
        let low_entropy = scanner.calculate_entropy("aaaaaaaaaa");
        assert!(low_entropy < 1.0);

        // High entropy (random-looking string)
        let high_entropy = scanner.calculate_entropy("aB3xK9mP2qL7");
        assert!(high_entropy > 3.0);
    }

    #[test]
    fn test_language_detection() {
        let scanner = SecurityScanner::new();

        assert_eq!(scanner.detect_language(Path::new("test.js")), Some("javascript".to_string()));
        assert_eq!(scanner.detect_language(Path::new("test.py")), Some("python".to_string()));
        assert_eq!(scanner.detect_language(Path::new("test.rs")), Some("rust".to_string()));
        assert_eq!(scanner.detect_language(Path::new("test.go")), Some("go".to_string()));
    }

    #[test]
    fn test_no_false_positives_on_clean_code() {
        let scanner = SecurityScanner::new();
        let source = r#"
            const x = 1;
            function add(a, b) {
                return a + b;
            }
        "#;
        let diagnostics = scanner.scan(source, Path::new("test.js"));

        assert!(diagnostics.is_empty());
    }

    // ===== Azure Secret Detection Tests =====

    #[test]
    fn test_detect_azure_storage_key() {
        let scanner = SecurityScanner::new();
        let source = r#"DefaultEndpointsProtocol=https;AccountKey=Eby8vdM02xNOcqFlqUwJPLlmEtlCDXJ1OUzFT50uSRZ6IFsuFq2UVErCz4I6tq/K1SZFPTOtr/KBHBeksoGMGw==;"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("azure-storage")));
    }

    #[test]
    fn test_detect_azure_client_secret() {
        let scanner = SecurityScanner::new();
        let source = r#"const client_secret = "8Q~abcdefghijklmnopqrstuvwxyz1234567890";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("azure-client-secret")));
    }

    #[test]
    fn test_detect_azure_subscription_key() {
        let scanner = SecurityScanner::new();
        // Azure subscription key is 32 hex characters (0-9, a-f)
        let source = r#"const ocp_apim_subscription_key = "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("azure-subscription")));
    }

    // ===== Database Credential Detection Tests =====

    #[test]
    fn test_detect_postgres_connection_string() {
        let scanner = SecurityScanner::new();
        let source = r#"const dbUrl = "postgresql://user:password123@localhost:5432/mydb";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("postgres")));
    }

    #[test]
    fn test_detect_mysql_connection_string() {
        let scanner = SecurityScanner::new();
        let source = r#"const dbUrl = "mysql://admin:secretpass@db.example.com:3306/production";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("mysql")));
    }

    #[test]
    fn test_detect_mongodb_connection_string() {
        let scanner = SecurityScanner::new();
        let source = r#"const mongoUri = "mongodb://dbuser:dbpass123@cluster0.mongodb.net/test";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("mongodb")));
    }

    #[test]
    fn test_detect_mongodb_srv_connection_string() {
        let scanner = SecurityScanner::new();
        let source = r#"const mongoUri = "mongodb+srv://user:pass@cluster.mongodb.net/db";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("mongodb")));
    }

    #[test]
    fn test_detect_redis_connection_string() {
        let scanner = SecurityScanner::new();
        let source = r#"const redisUrl = "redis://:mypassword@redis.example.com:6379";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("redis")));
    }

    #[test]
    fn test_detect_database_password() {
        let scanner = SecurityScanner::new();
        let source = r#"const db_password = "MySecureP@ssw0rd123";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("database-password")));
    }

    // ===== Private Key Detection Tests =====

    #[test]
    fn test_detect_rsa_private_key() {
        let scanner = SecurityScanner::new();
        let source = r#"
            const key = `-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA...
-----END RSA PRIVATE KEY-----`;
        "#;
        let diagnostics = scanner.scan(source, Path::new("keys.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("rsa-private-key")));
    }

    #[test]
    fn test_detect_ec_private_key() {
        let scanner = SecurityScanner::new();
        let source = r#"-----BEGIN EC PRIVATE KEY-----
MHcCAQEEIIGlRQKt...
-----END EC PRIVATE KEY-----"#;
        let diagnostics = scanner.scan(source, Path::new("keys.pem"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("ec-private-key")));
    }

    #[test]
    fn test_detect_openssh_private_key() {
        let scanner = SecurityScanner::new();
        let source = r#"-----BEGIN OPENSSH PRIVATE KEY-----
b3BlbnNzaC1rZXktdjEA...
-----END OPENSSH PRIVATE KEY-----"#;
        let diagnostics = scanner.scan(source, Path::new("id_rsa"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("openssh-private-key")));
    }

    #[test]
    fn test_detect_pgp_private_key() {
        let scanner = SecurityScanner::new();
        let source = r#"-----BEGIN PGP PRIVATE KEY BLOCK-----
Version: GnuPG v2
lQOYBF...
-----END PGP PRIVATE KEY BLOCK-----"#;
        let diagnostics = scanner.scan(source, Path::new("private.asc"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("pgp-private-key")));
    }

    // ===== OAuth and JWT Token Detection Tests =====

    #[test]
    fn test_detect_oauth_access_token() {
        let scanner = SecurityScanner::new();
        let source =
            r#"const accessToken = "ya29.a0AfH6SMBx1234567890abcdefghijklmnopqrstuvwxyz";"#;
        let diagnostics = scanner.scan(source, Path::new("auth.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("oauth") || d.rule_id.contains("google"))
        );
    }

    #[test]
    fn test_detect_oauth_refresh_token() {
        let scanner = SecurityScanner::new();
        let source = r#"const refresh_token = "1//0gAbCdEfGhIjKlMnOpQrStUvWxYz1234567890";"#;
        let diagnostics = scanner.scan(source, Path::new("auth.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("refresh-token") || d.rule_id.contains("token"))
        );
    }

    #[test]
    fn test_detect_oauth_client_secret() {
        let scanner = SecurityScanner::new();
        let source = r#"const client_secret = "GOCSPX-1234567890abcdefghijklmnopqr";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("client-secret") || d.rule_id.contains("secret"))
        );
    }

    #[test]
    fn test_detect_jwt_secret() {
        let scanner = SecurityScanner::new();
        let source = r#"const jwt_secret = "my-super-secret-jwt-key-12345678";"#;
        let diagnostics = scanner.scan(source, Path::new("auth.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("jwt-secret") || d.rule_id.contains("secret"))
        );
    }

    // ===== Additional API Key Detection Tests =====

    #[test]
    fn test_detect_github_oauth_token() {
        let scanner = SecurityScanner::new();
        let source = r#"const token = "gho_16C7e42F292c6912E7710c838347Ae178B4a";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("github")));
    }

    #[test]
    fn test_detect_github_app_token() {
        let scanner = SecurityScanner::new();
        let source = r#"const token = "ghu_16C7e42F292c6912E7710c838347Ae178B4a";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("github")));
    }

    #[test]
    fn test_detect_slack_token() {
        let scanner = SecurityScanner::new();
        let source =
            r#"const slackToken = "xoxb-1234567890-1234567890123-abcdefghijklmnopqrstuvwx";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("slack")));
    }

    #[test]
    fn test_detect_slack_webhook() {
        let scanner = SecurityScanner::new();
        let source = r#"const webhook = "https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("slack")));
    }

    #[test]
    fn test_detect_twilio_api_key() {
        let scanner = SecurityScanner::new();
        let source = r#"const twilioKey = "SK1234567890abcdef1234567890abcdef";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("twilio")));
    }

    #[test]
    fn test_detect_sendgrid_api_key() {
        let scanner = SecurityScanner::new();
        // SendGrid format: SG.{22 chars}.{43 chars}
        let source = r#"const sendgridKey = "SG.1234567890abcdefghijkl.1234567890abcdefghijklmnopqrstuvwxyz1234567";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("sendgrid")));
    }

    #[test]
    fn test_detect_google_oauth_token() {
        let scanner = SecurityScanner::new();
        let source = r#"const token = "ya29.a0AfH6SMBx...";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("google")));
    }

    #[test]
    fn test_detect_google_service_account() {
        let scanner = SecurityScanner::new();
        let source = r#"{
            "type": "service_account",
            "project_id": "my-project",
            "private_key": "-----BEGIN PRIVATE KEY-----\n..."
        }"#;
        let diagnostics = scanner.scan(source, Path::new("service-account.json"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("google-service-account")
                || d.rule_id.contains("private-key"))
        );
    }

    // ===== Generic Secret Detection Tests =====

    #[test]
    fn test_detect_generic_password() {
        let scanner = SecurityScanner::new();
        let source = r#"const password = "MyP@ssw0rd123456";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("password") || d.rule_id.contains("secret"))
        );
    }

    #[test]
    fn test_detect_generic_api_key() {
        let scanner = SecurityScanner::new();
        let source = r#"const api_key = "1234567890abcdef1234567890abcdef12345678";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("api-key") || d.rule_id.contains("secret"))
        );
    }

    #[test]
    fn test_detect_generic_secret() {
        let scanner = SecurityScanner::new();
        let source = r#"const secret = "abcdefghijklmnopqrstuvwxyz1234567890";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().any(|d| d.rule_id.contains("secret")));
    }

    // ===== Entropy-Based Detection Tests =====

    #[test]
    fn test_high_entropy_detection() {
        let scanner = SecurityScanner::new();
        // High entropy random-looking string with proper key name
        let source = r#"const secret_key = "aB3xK9mP2qL7wN5vR8tY4uI6oP1sD0fG";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("secret") || d.rule_id.contains("entropy"))
        );
    }

    #[test]
    fn test_low_entropy_not_detected() {
        let scanner = SecurityScanner::new();
        // Low entropy string (repeated characters)
        let source = r#"const value = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        // Should not be detected as a secret due to low entropy
        assert!(
            diagnostics.is_empty() || !diagnostics.iter().any(|d| d.rule_id.contains("entropy"))
        );
    }

    // ===== AWS Additional Patterns Tests =====

    #[test]
    fn test_detect_aws_session_token() {
        let scanner = SecurityScanner::new();
        let source = r#"const aws_session_token = "FwoGZXIvYXdzEBYaDHVzLWVhc3QtMSJHMEUCIQDExampleTokenHereWithMoreThan100CharactersToMatchThePatternRequirementForSessionTokens1234567890";"#;
        let diagnostics = scanner.scan(source, Path::new("config.js"));

        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule_id.contains("aws") || d.rule_id.contains("session"))
        );
    }

    // ===== Comprehensive Pattern Coverage Test =====

    #[test]
    fn test_secret_pattern_count() {
        let scanner = SecurityScanner::new();
        let db = scanner.database();

        // Verify we have comprehensive coverage
        assert!(db.secret_patterns.len() >= 40, "Should have at least 40 secret patterns");

        // Verify key categories are covered
        let pattern_ids: Vec<String> = db.secret_patterns.iter().map(|p| p.id.clone()).collect();

        // AWS patterns
        assert!(pattern_ids.iter().any(|id| id.contains("aws")));

        // Google Cloud patterns
        assert!(pattern_ids.iter().any(|id| id.contains("google")));

        // Azure patterns
        assert!(pattern_ids.iter().any(|id| id.contains("azure")));

        // GitHub patterns
        assert!(pattern_ids.iter().any(|id| id.contains("github")));

        // Database patterns
        assert!(
            pattern_ids.iter().any(|id| id.contains("database")
                || id.contains("postgres")
                || id.contains("mysql"))
        );

        // Private key patterns
        assert!(
            pattern_ids
                .iter()
                .any(|id| id.contains("private-key") || id.contains("rsa") || id.contains("ssh"))
        );

        // OAuth/JWT patterns
        assert!(pattern_ids.iter().any(|id| id.contains("oauth") || id.contains("jwt")));

        // Generic patterns
        assert!(pattern_ids.iter().any(|id| id.contains("generic") || id.contains("entropy")));
    }
}
