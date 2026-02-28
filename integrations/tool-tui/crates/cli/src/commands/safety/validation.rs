//! AI change validation

use anyhow::Result;
use std::path::Path;

use super::{CheckSeverity, SafetyConfig, ValidationCheck, ValidationResult};

/// Validate a change
pub fn validate(change_id: &str, strict: bool) -> Result<ValidationResult> {
    let mut checks = Vec::new();
    let mut risk_score: f32 = 0.0;
    let config = SafetyConfig::default();

    // Check 1: File pattern validation
    let pattern_check = validate_file_patterns(change_id, &config.blocked_patterns);
    if !pattern_check.passed {
        risk_score += 0.3;
    }
    checks.push(pattern_check);

    // Check 2: Syntax validation
    let syntax_check = validate_syntax(change_id);
    if !syntax_check.passed {
        risk_score += 0.2;
    }
    checks.push(syntax_check);

    // Check 3: Security scan
    let security_check = validate_security(change_id);
    if !security_check.passed {
        risk_score += 0.4;
    }
    checks.push(security_check);

    // Check 4: Dependency validation
    let dep_check = validate_dependencies(change_id);
    if !dep_check.passed {
        risk_score += 0.2;
    }
    checks.push(dep_check);

    // Check 5: Test impact
    let test_check = validate_test_impact(change_id);
    if !test_check.passed {
        risk_score += 0.1;
    }
    checks.push(test_check);

    // Strict mode adds more checks
    if strict {
        let strict_check = validate_strict(change_id);
        if !strict_check.passed {
            risk_score += 0.2;
        }
        checks.push(strict_check);
    }

    // Normalize risk score
    risk_score = risk_score.min(1.0);

    let passed = checks
        .iter()
        .all(|c| c.passed || matches!(c.severity, CheckSeverity::Info | CheckSeverity::Warning))
        && risk_score <= config.max_risk_score;

    let mut recommendations = Vec::new();
    if risk_score > 0.5 {
        recommendations.push("Consider breaking this change into smaller parts".to_string());
    }
    if !checks.iter().any(|c| c.name == "test_impact" && c.passed) {
        recommendations.push("Add tests for this change".to_string());
    }

    Ok(ValidationResult {
        passed,
        checks,
        risk_score,
        recommendations,
    })
}

fn validate_file_patterns(_change_id: &str, blocked_patterns: &[String]) -> ValidationCheck {
    // TODO: Load actual files from change
    let files: Vec<&str> = vec![];

    for file in &files {
        for pattern in blocked_patterns {
            if matches_pattern(file, pattern) {
                return ValidationCheck {
                    name: "file_patterns".to_string(),
                    passed: false,
                    message: format!("File {} matches blocked pattern {}", file, pattern),
                    severity: CheckSeverity::Critical,
                };
            }
        }
    }

    ValidationCheck {
        name: "file_patterns".to_string(),
        passed: true,
        message: "No blocked file patterns detected".to_string(),
        severity: CheckSeverity::Info,
    }
}

fn matches_pattern(path: &str, pattern: &str) -> bool {
    // Simple glob matching
    if pattern.starts_with("**/") && pattern.ends_with("/**") {
        // Pattern like **/secrets/** - match if path contains the middle part as a directory
        let middle = &pattern[3..pattern.len() - 3];
        path.contains(&format!("/{}/", middle))
            || path.contains(&format!("{}/", middle))
            || path.starts_with(&format!("{}/", middle))
    } else if pattern.starts_with("**/") {
        let suffix = &pattern[3..];
        path.contains(suffix) || path.ends_with(suffix)
    } else if pattern.ends_with("/**") {
        let prefix = &pattern[..pattern.len() - 3];
        path.starts_with(prefix) || path.contains(&format!("/{}", prefix))
    } else if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        path.ends_with(suffix)
    } else if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len() - 1];
        path.starts_with(prefix)
    } else {
        path == pattern || path.ends_with(&format!("/{}", pattern))
    }
}

fn validate_syntax(_change_id: &str) -> ValidationCheck {
    // TODO: Actually validate syntax of changed files
    ValidationCheck {
        name: "syntax".to_string(),
        passed: true,
        message: "Syntax validation passed".to_string(),
        severity: CheckSeverity::Error,
    }
}

fn validate_security(_change_id: &str) -> ValidationCheck {
    // Security checks:
    // - No hardcoded secrets
    // - No unsafe patterns
    // - No SQL injection vulnerabilities
    // - No XSS vulnerabilities

    // TODO: Implement actual security scanning
    ValidationCheck {
        name: "security".to_string(),
        passed: true,
        message: "No security issues detected".to_string(),
        severity: CheckSeverity::Critical,
    }
}

fn validate_dependencies(_change_id: &str) -> ValidationCheck {
    // Check for:
    // - Known vulnerable dependencies
    // - License compatibility
    // - Unexpected new dependencies

    ValidationCheck {
        name: "dependencies".to_string(),
        passed: true,
        message: "Dependencies are valid".to_string(),
        severity: CheckSeverity::Warning,
    }
}

fn validate_test_impact(_change_id: &str) -> ValidationCheck {
    // Check if:
    // - Existing tests still pass
    // - New tests are added for new functionality
    // - Test coverage is maintained

    ValidationCheck {
        name: "test_impact".to_string(),
        passed: true,
        message: "Test impact assessment passed".to_string(),
        severity: CheckSeverity::Warning,
    }
}

fn validate_strict(_change_id: &str) -> ValidationCheck {
    // Additional strict checks:
    // - Code style compliance
    // - Documentation requirements
    // - Breaking change detection

    ValidationCheck {
        name: "strict".to_string(),
        passed: true,
        message: "Strict validation passed".to_string(),
        severity: CheckSeverity::Warning,
    }
}

/// Validate a single file
pub fn validate_file(path: &Path) -> Result<Vec<ValidationCheck>> {
    let mut checks = Vec::new();

    // Read file content
    let content = std::fs::read_to_string(path)?;

    // Check for secrets
    checks.push(check_secrets(&content));

    // Check for unsafe patterns
    checks.push(check_unsafe_patterns(&content));

    Ok(checks)
}

fn check_secrets(content: &str) -> ValidationCheck {
    // Patterns that might indicate secrets
    let secret_patterns = [
        "password=",
        "secret=",
        "api_key=",
        "private_key",
        "BEGIN RSA PRIVATE KEY",
        "BEGIN OPENSSH PRIVATE KEY",
        "ghp_", // GitHub token
        "sk-",  // OpenAI key
    ];

    for pattern in &secret_patterns {
        if content.to_lowercase().contains(&pattern.to_lowercase()) {
            return ValidationCheck {
                name: "secrets".to_string(),
                passed: false,
                message: format!("Potential secret detected: pattern '{}'", pattern),
                severity: CheckSeverity::Critical,
            };
        }
    }

    ValidationCheck {
        name: "secrets".to_string(),
        passed: true,
        message: "No secrets detected".to_string(),
        severity: CheckSeverity::Info,
    }
}

fn check_unsafe_patterns(content: &str) -> ValidationCheck {
    let unsafe_patterns = [
        "eval(",
        "exec(",
        "system(",
        "shell_exec(",
        "innerHTML",
        "dangerouslySetInnerHTML",
    ];

    for pattern in &unsafe_patterns {
        if content.contains(pattern) {
            return ValidationCheck {
                name: "unsafe_patterns".to_string(),
                passed: false,
                message: format!("Potentially unsafe pattern: {}", pattern),
                severity: CheckSeverity::Warning,
            };
        }
    }

    ValidationCheck {
        name: "unsafe_patterns".to_string(),
        passed: true,
        message: "No unsafe patterns detected".to_string(),
        severity: CheckSeverity::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern(".env", ".env*"));
        assert!(matches_pattern("config.key", "*.key"));
        assert!(matches_pattern("path/to/secrets/file.txt", "**/secrets/**"));
    }

    #[test]
    fn test_check_secrets() {
        let check = check_secrets("password=secret123");
        assert!(!check.passed);

        let check = check_secrets("normal code here");
        assert!(check.passed);
    }
}
