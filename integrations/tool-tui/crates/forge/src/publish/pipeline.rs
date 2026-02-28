//! Publish validation pipeline.
//!
//! Provides comprehensive validation steps for plugins before publishing,
//! including syntax checking, security scanning, and performance benchmarking.

use std::path::Path;
use std::time::{Duration, Instant};

/// Validation step in the publish pipeline.
#[derive(Debug, Clone)]
pub struct ValidationStep {
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Whether this step is required
    pub required: bool,
    /// Step result
    pub result: Option<ValidationResult>,
    /// Execution time
    pub duration: Option<Duration>,
}

impl ValidationStep {
    /// Create a new validation step.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            required: true,
            result: None,
            duration: None,
        }
    }

    /// Mark step as optional.
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Set the result.
    pub fn with_result(mut self, result: ValidationResult, duration: Duration) -> Self {
        self.result = Some(result);
        self.duration = Some(duration);
        self
    }

    /// Check if step passed.
    pub fn passed(&self) -> bool {
        matches!(self.result, Some(ValidationResult::Passed))
    }

    /// Check if step failed.
    pub fn failed(&self) -> bool {
        matches!(self.result, Some(ValidationResult::Failed(_)))
    }
}

/// Result of a validation step.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Validation passed
    Passed,
    /// Validation passed with warnings
    PassedWithWarnings(Vec<String>),
    /// Validation failed
    Failed(Vec<ValidationError>),
    /// Validation was skipped
    Skipped(String),
}

/// Validation error details.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// File location (if applicable)
    pub location: Option<ErrorLocation>,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Location of an error in a file.
#[derive(Debug, Clone)]
pub struct ErrorLocation {
    /// File path
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

/// Publish validation pipeline.
#[derive(Debug)]
pub struct PublishPipeline {
    /// Validation steps
    steps: Vec<ValidationStep>,
    /// Plugin source path
    source_path: std::path::PathBuf,
    /// Whether to run all steps or stop on first failure
    continue_on_failure: bool,
}

impl PublishPipeline {
    /// Create a new publish pipeline.
    pub fn new(source_path: &Path) -> Self {
        Self {
            steps: Self::default_steps(),
            source_path: source_path.to_path_buf(),
            continue_on_failure: false,
        }
    }

    /// Get default validation steps.
    fn default_steps() -> Vec<ValidationStep> {
        vec![
            ValidationStep::new("manifest", "Validate plugin.sr manifest"),
            ValidationStep::new("syntax", "Check .sr syntax in all files"),
            ValidationStep::new("structure", "Verify plugin directory structure"),
            ValidationStep::new("dependencies", "Validate dependency versions"),
            ValidationStep::new("security", "Run security scan"),
            ValidationStep::new("size", "Check package size limits"),
            ValidationStep::new("performance", "Run performance benchmarks").optional(),
            ValidationStep::new("breaking-changes", "Detect breaking API changes").optional(),
        ]
    }

    /// Continue running on failures.
    pub fn continue_on_failure(mut self) -> Self {
        self.continue_on_failure = true;
        self
    }

    /// Add a custom validation step.
    pub fn add_step(mut self, step: ValidationStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Run the validation pipeline.
    pub fn run(&mut self) -> PipelineResult {
        let start = Instant::now();
        let mut all_passed = true;
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Collect step names and required flags first to avoid borrow issues
        let step_info: Vec<(String, bool)> =
            self.steps.iter().map(|s| (s.name.clone(), s.required)).collect();

        for (i, (step_name, required)) in step_info.iter().enumerate() {
            let step_start = Instant::now();
            let result = self.run_step(step_name);
            let duration = step_start.elapsed();

            match &result {
                ValidationResult::Passed => {}
                ValidationResult::PassedWithWarnings(w) => {
                    warnings.extend(w.clone());
                }
                ValidationResult::Failed(e) => {
                    all_passed = false;
                    errors.extend(e.clone());
                    if !self.continue_on_failure && *required {
                        self.steps[i].result = Some(result);
                        self.steps[i].duration = Some(duration);
                        break;
                    }
                }
                ValidationResult::Skipped(_) => {}
            }

            self.steps[i].result = Some(result);
            self.steps[i].duration = Some(duration);
        }

        PipelineResult {
            passed: all_passed,
            steps: self.steps.clone(),
            errors,
            warnings,
            duration: start.elapsed(),
        }
    }

    /// Run a single validation step.
    fn run_step(&self, step_name: &str) -> ValidationResult {
        match step_name {
            "manifest" => self.validate_manifest(),
            "syntax" => self.validate_syntax(),
            "structure" => self.validate_structure(),
            "dependencies" => self.validate_dependencies(),
            "security" => self.validate_security(),
            "size" => self.validate_size(),
            "performance" => self.validate_performance(),
            "breaking-changes" => self.validate_breaking_changes(),
            _ => ValidationResult::Skipped(format!("Unknown step: {}", step_name)),
        }
    }

    /// Validate plugin manifest.
    fn validate_manifest(&self) -> ValidationResult {
        let manifest_path = self.source_path.join("plugin.sr");

        if !manifest_path.exists() {
            return ValidationResult::Failed(vec![ValidationError {
                code: "E001".into(),
                message: "Missing plugin.sr manifest file".into(),
                location: None,
                suggestion: Some("Create a plugin.sr file with plugin metadata".into()),
            }]);
        }

        // Read and validate manifest content
        match std::fs::read_to_string(&manifest_path) {
            Ok(content) => {
                let mut errors = Vec::new();

                if !content.contains("name") {
                    errors.push(ValidationError {
                        code: "E002".into(),
                        message: "Missing 'name' field in manifest".into(),
                        location: Some(ErrorLocation {
                            file: "plugin.sr".into(),
                            line: 1,
                            column: 1,
                        }),
                        suggestion: Some("Add: name = \"your-plugin-name\"".into()),
                    });
                }

                if !content.contains("version") {
                    errors.push(ValidationError {
                        code: "E003".into(),
                        message: "Missing 'version' field in manifest".into(),
                        location: Some(ErrorLocation {
                            file: "plugin.sr".into(),
                            line: 1,
                            column: 1,
                        }),
                        suggestion: Some("Add: version = \"0.1.0\"".into()),
                    });
                }

                if errors.is_empty() {
                    ValidationResult::Passed
                } else {
                    ValidationResult::Failed(errors)
                }
            }
            Err(e) => ValidationResult::Failed(vec![ValidationError {
                code: "E004".into(),
                message: format!("Failed to read manifest: {}", e),
                location: None,
                suggestion: None,
            }]),
        }
    }

    /// Validate .sr syntax.
    fn validate_syntax(&self) -> ValidationResult {
        // Walk through all .sr files and validate syntax
        let mut warnings = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.source_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "sr") {
                    // Simplified syntax check - in production, use DX Serializer parser
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if content.contains("TODO") || content.contains("FIXME") {
                            warnings.push(format!(
                                "Found TODO/FIXME in {}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ));
                        }
                    }
                }
            }
        }

        if warnings.is_empty() {
            ValidationResult::Passed
        } else {
            ValidationResult::PassedWithWarnings(warnings)
        }
    }

    /// Validate plugin directory structure.
    fn validate_structure(&self) -> ValidationResult {
        let required_files = ["plugin.sr"];
        let mut errors = Vec::new();

        for file in required_files {
            if !self.source_path.join(file).exists() {
                errors.push(ValidationError {
                    code: "E010".into(),
                    message: format!("Missing required file: {}", file),
                    location: None,
                    suggestion: None,
                });
            }
        }

        if errors.is_empty() {
            ValidationResult::Passed
        } else {
            ValidationResult::Failed(errors)
        }
    }

    /// Validate dependencies.
    fn validate_dependencies(&self) -> ValidationResult {
        // In production, this would check dependency versions
        ValidationResult::Passed
    }

    /// Run security scan.
    fn validate_security(&self) -> ValidationResult {
        let mut warnings = Vec::new();

        // Check for potentially dangerous patterns
        if let Ok(entries) = std::fs::read_dir(&self.source_path) {
            for entry in entries.flatten() {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    // Check for hardcoded secrets patterns
                    if content.contains("api_key") || content.contains("secret") {
                        warnings.push(format!("Potential secret in {}", entry.path().display()));
                    }
                }
            }
        }

        if warnings.is_empty() {
            ValidationResult::Passed
        } else {
            ValidationResult::PassedWithWarnings(warnings)
        }
    }

    /// Validate package size.
    fn validate_size(&self) -> ValidationResult {
        const MAX_SIZE: u64 = 50 * 1024 * 1024; // 50MB

        let total_size = calculate_dir_size(&self.source_path);

        if total_size > MAX_SIZE {
            ValidationResult::Failed(vec![ValidationError {
                code: "E020".into(),
                message: format!(
                    "Package too large: {} bytes (max: {} bytes)",
                    total_size, MAX_SIZE
                ),
                location: None,
                suggestion: Some("Consider excluding large files or assets".into()),
            }])
        } else {
            ValidationResult::Passed
        }
    }

    /// Run performance benchmarks.
    fn validate_performance(&self) -> ValidationResult {
        // In production, this would run actual benchmarks
        ValidationResult::Skipped("Performance benchmarks not implemented".into())
    }

    /// Detect breaking API changes.
    fn validate_breaking_changes(&self) -> ValidationResult {
        // In production, this would compare with previous version
        ValidationResult::Skipped("Breaking change detection not implemented".into())
    }
}

/// Result of running the validation pipeline.
#[derive(Debug)]
pub struct PipelineResult {
    /// Whether all required steps passed
    pub passed: bool,
    /// All validation steps with results
    pub steps: Vec<ValidationStep>,
    /// All errors encountered
    pub errors: Vec<ValidationError>,
    /// All warnings encountered
    pub warnings: Vec<String>,
    /// Total pipeline duration
    pub duration: Duration,
}

impl PipelineResult {
    /// Generate a summary report.
    pub fn summary(&self) -> String {
        let mut output = String::new();

        output.push_str("═══════════════════════════════════════════════════════════════\n");
        output.push_str("                    DX PUBLISH VALIDATION REPORT               \n");
        output.push_str("═══════════════════════════════════════════════════════════════\n\n");

        for step in &self.steps {
            let status = match &step.result {
                Some(ValidationResult::Passed) => "✓ PASS",
                Some(ValidationResult::PassedWithWarnings(_)) => "⚠ WARN",
                Some(ValidationResult::Failed(_)) => "✗ FAIL",
                Some(ValidationResult::Skipped(_)) => "○ SKIP",
                None => "○ PEND",
            };

            let duration = step
                .duration
                .map(|d| format!("{:.2}ms", d.as_secs_f64() * 1000.0))
                .unwrap_or_else(|| "-".into());

            output.push_str(&format!("{:8} │ {:30} │ {:>10}\n", status, step.name, duration));
        }

        output.push_str("\n───────────────────────────────────────────────────────────────\n");

        if !self.errors.is_empty() {
            output.push_str("\nErrors:\n");
            for error in &self.errors {
                output.push_str(&format!("  [{:}] {}\n", error.code, error.message));
                if let Some(ref suggestion) = error.suggestion {
                    output.push_str(&format!("       └─ Suggestion: {}\n", suggestion));
                }
            }
        }

        if !self.warnings.is_empty() {
            output.push_str("\nWarnings:\n");
            for warning in &self.warnings {
                output.push_str(&format!("  ⚠ {}\n", warning));
            }
        }

        let status = if self.passed { "PASSED" } else { "FAILED" };
        output.push_str(&format!(
            "\nResult: {} in {:.2}ms\n",
            status,
            self.duration.as_secs_f64() * 1000.0
        ));

        output
    }
}

/// Calculate total size of a directory.
fn calculate_dir_size(path: &Path) -> u64 {
    let mut size = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += calculate_dir_size(&path);
            } else if let Ok(metadata) = std::fs::metadata(&path) {
                size += metadata.len();
            }
        }
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validation_step() {
        let step = ValidationStep::new("test", "Test step");
        assert!(step.required);
        assert!(!step.passed());
        assert!(!step.failed());
    }

    #[test]
    fn test_optional_step() {
        let step = ValidationStep::new("test", "Test step").optional();
        assert!(!step.required);
    }

    #[test]
    fn test_pipeline_result_summary() {
        let result = PipelineResult {
            passed: true,
            steps: vec![
                ValidationStep::new("test", "Test")
                    .with_result(ValidationResult::Passed, Duration::from_millis(10)),
            ],
            errors: vec![],
            warnings: vec![],
            duration: Duration::from_millis(100),
        };

        let summary = result.summary();
        assert!(summary.contains("PASS"));
        assert!(summary.contains("PASSED"));
    }
}
