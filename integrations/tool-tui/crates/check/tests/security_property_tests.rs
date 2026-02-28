//! Property-Based Tests for Security Scanner
//!
//! Task 6.4: Write property test for secret detection
//! Property 5: Secret pattern coverage
//! Validates: Requirements 3.2

use dx_check::security::SecurityScanner;
use proptest::prelude::*;
use std::path::Path;

// ============================================================================
// Property 5: Secret Pattern Coverage
// ============================================================================

/// Generate AWS access keys (AKIA + 16 alphanumeric chars)
fn aws_access_key_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("AKIA[0-9A-Z]{16}").unwrap()
}

/// Generate GitHub personal access tokens (ghp_ + 36 alphanumeric chars)
fn github_token_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("ghp_[0-9a-zA-Z]{36}").unwrap()
}

/// Generate Google API keys (AIza + 35 alphanumeric chars)
fn google_api_key_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("AIza[0-9A-Za-z\\-_]{35}").unwrap()
}

/// Generate JWT tokens (3 base64 segments separated by dots)
fn jwt_token_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex(
        "eyJ[A-Za-z0-9_-]{10,50}\\.[A-Za-z0-9_-]{10,50}\\.[A-Za-z0-9_-]{10,50}",
    )
    .unwrap()
}

/// Generate RSA private key headers
fn rsa_private_key_strategy() -> impl Strategy<Value = String> {
    Just(
        "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----"
            .to_string(),
    )
}

/// Generate database connection strings with credentials
fn postgres_connection_strategy() -> impl Strategy<Value = String> {
    ("[a-z]{4,10}", "[a-zA-Z0-9]{8,16}", "[a-z]{4,10}", "[a-z]{4,10}").prop_map(
        |(user, pass, host, db)| format!("postgresql://{}:{}@{}:5432/{}", user, pass, host, db),
    )
}

/// Generate high-entropy strings (potential secrets)
fn high_entropy_string_strategy() -> impl Strategy<Value = String> {
    // Generate strings with high character diversity (high entropy)
    prop::collection::vec(
        prop::sample::select(vec![
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q',
            'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h',
            'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y',
            'z', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+', '/', '=',
        ]),
        32..64,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.2**
    ///
    /// Property 5: Secret pattern coverage
    ///
    /// Test that any string matching known secret patterns is detected.
    /// This property verifies that the SecurityScanner correctly identifies
    /// all types of secrets across different pattern categories.
    #[test]
    fn test_aws_access_key_detection(key in aws_access_key_strategy()) {
        let scanner = SecurityScanner::new();
        let source = format!(r#"const awsKey = "{}";"#, key);
        let diagnostics = scanner.scan(&source, Path::new("config.js"));

        // AWS access keys should always be detected
        prop_assert!(
            !diagnostics.is_empty(),
            "AWS access key not detected: {}",
            key
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("aws")),
            "AWS access key detected but wrong rule_id: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_github_token_detection(token in github_token_strategy()) {
        let scanner = SecurityScanner::new();
        let source = format!(r#"const githubToken = "{}";"#, token);
        let diagnostics = scanner.scan(&source, Path::new("auth.js"));

        // GitHub tokens should always be detected
        prop_assert!(
            !diagnostics.is_empty(),
            "GitHub token not detected: {}",
            token
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("github")),
            "GitHub token detected but wrong rule_id: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_google_api_key_detection(key in google_api_key_strategy()) {
        let scanner = SecurityScanner::new();
        let source = format!(r#"const googleKey = "{}";"#, key);
        let diagnostics = scanner.scan(&source, Path::new("config.js"));

        // Google API keys should always be detected
        prop_assert!(
            !diagnostics.is_empty(),
            "Google API key not detected: {}",
            key
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("google")),
            "Google API key detected but wrong rule_id: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_jwt_token_detection(token in jwt_token_strategy()) {
        let scanner = SecurityScanner::new();
        let source = format!(r#"const jwt = "{}";"#, token);
        let diagnostics = scanner.scan(&source, Path::new("auth.js"));

        // JWT tokens should always be detected
        prop_assert!(
            !diagnostics.is_empty(),
            "JWT token not detected: {}",
            token
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("jwt")),
            "JWT token detected but wrong rule_id: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_rsa_private_key_detection(key in rsa_private_key_strategy()) {
        let scanner = SecurityScanner::new();
        let diagnostics = scanner.scan(&key, Path::new("keys.pem"));

        // RSA private keys should always be detected
        prop_assert!(
            !diagnostics.is_empty(),
            "RSA private key not detected"
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("rsa-private-key")),
            "RSA private key detected but wrong rule_id: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_postgres_connection_detection(conn_str in postgres_connection_strategy()) {
        let scanner = SecurityScanner::new();
        let source = format!(r#"const dbUrl = "{}";"#, conn_str);
        let diagnostics = scanner.scan(&source, Path::new("config.js"));

        // PostgreSQL connection strings with credentials should always be detected
        prop_assert!(
            !diagnostics.is_empty(),
            "PostgreSQL connection string not detected: {}",
            conn_str
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("postgres") || d.rule_id.contains("database")),
            "PostgreSQL connection detected but wrong rule_id: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }
}

// ============================================================================
// Property: High-Entropy String Detection
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Validates: Requirements 3.2**
    ///
    /// Property 5: Secret pattern coverage (high-entropy strings)
    ///
    /// Test that high-entropy strings above threshold are flagged.
    /// This verifies the entropy analysis component of secret detection.
    #[test]
    fn test_high_entropy_string_detection(secret in high_entropy_string_strategy()) {
        let scanner = SecurityScanner::new();

        // Wrap in a context that looks like a secret assignment
        let source = format!(r#"const api_key = "{}";"#, secret);
        let diagnostics = scanner.scan(&source, Path::new("config.js"));

        // Calculate entropy to verify it's actually high
        let entropy = calculate_test_entropy(&secret);

        if entropy >= 4.0 {
            // High-entropy strings in secret-like contexts should be detected
            prop_assert!(
                !diagnostics.is_empty(),
                "High-entropy string (entropy={:.2}) not detected: {}",
                entropy,
                secret
            );
        }
    }
}

// ============================================================================
// Property: Pattern Coverage Completeness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Validates: Requirements 3.2**
    ///
    /// Property 5: Secret pattern coverage (multiple secrets)
    ///
    /// Test that multiple secrets in the same file are all detected.
    /// This verifies that the scanner doesn't stop after finding one secret.
    #[test]
    fn test_multiple_secrets_detection(
        aws_key in aws_access_key_strategy(),
        github_token in github_token_strategy(),
        google_key in google_api_key_strategy(),
    ) {
        let scanner = SecurityScanner::new();

        let source = format!(
            r#"
            const awsKey = "{}";
            const githubToken = "{}";
            const googleKey = "{}";
            "#,
            aws_key, github_token, google_key
        );

        let diagnostics = scanner.scan(&source, Path::new("secrets.js"));

        // All three secrets should be detected
        prop_assert!(
            diagnostics.len() >= 3,
            "Expected at least 3 secrets detected, found {}",
            diagnostics.len()
        );

        // Verify each type is detected
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("aws")),
            "AWS key not detected in multi-secret file"
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("github")),
            "GitHub token not detected in multi-secret file"
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("google")),
            "Google key not detected in multi-secret file"
        );
    }
}

// ============================================================================
// Property: No False Positives on Clean Code
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Validates: Requirements 3.2**
    ///
    /// Property 5: Secret pattern coverage (negative cases)
    ///
    /// Test that normal code without secrets doesn't trigger false positives.
    /// This verifies the specificity of secret detection patterns.
    #[test]
    fn test_no_false_positives_on_normal_code(
        var_name in "[a-z]{4,10}",
        value in "[a-z]{4,10}",
    ) {
        let scanner = SecurityScanner::new();

        // Generate normal-looking code without secrets
        let source = format!(
            r#"
            const {} = "{}";
            function add(a, b) {{
                return a + b;
            }}
            "#,
            var_name, value
        );

        let diagnostics = scanner.scan(&source, Path::new("utils.js"));

        // Normal code should not trigger secret detection
        prop_assert!(
            diagnostics.is_empty(),
            "False positive detected in clean code: {:?}",
            diagnostics.iter().map(|d| &d.rule_id).collect::<Vec<_>>()
        );
    }
}

// ============================================================================
// Property: Secret Detection Across File Types
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// **Validates: Requirements 3.2**
    ///
    /// Property 5: Secret pattern coverage (file type independence)
    ///
    /// Test that secrets are detected regardless of file extension.
    /// This verifies that secret detection is language-agnostic.
    #[test]
    fn test_secret_detection_across_file_types(
        key in aws_access_key_strategy(),
        ext in prop::sample::select(vec!["js", "py", "rs", "go", "java", "txt", "md"]),
    ) {
        let scanner = SecurityScanner::new();
        let source = format!(r#"aws_key = "{}""#, key);
        let file_path = format!("config.{}", ext);

        let diagnostics = scanner.scan(&source, Path::new(&file_path));

        // Secrets should be detected in any file type
        prop_assert!(
            !diagnostics.is_empty(),
            "AWS key not detected in .{} file",
            ext
        );
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id.contains("aws")),
            "AWS key detected but wrong rule_id in .{} file",
            ext
        );
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate Shannon entropy for testing purposes
fn calculate_test_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }

    let mut freq = std::collections::HashMap::new();
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
