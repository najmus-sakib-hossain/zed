//! Graceful Degradation Module
//!
//! Provides infrastructure for handling incompatibilities gracefully,
//! including detailed error messages, workaround suggestions, and
//! compatibility checking before execution.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Errors related to graceful degradation
#[derive(Debug, Error)]
pub enum DegradationError {
    #[error("Extension load failure: {0}")]
    ExtensionLoadFailure(String),

    #[error("Missing API: {0}")]
    MissingApi(String),

    #[error("Partial compatibility: {0}")]
    PartialCompatibility(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Detailed error information for extension failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionFailureInfo {
    /// Name of the extension that failed
    pub extension_name: String,
    /// Path to the extension file (if found)
    pub extension_path: Option<PathBuf>,
    /// Specific reason for the failure
    pub failure_reason: FailureReason,
    /// Human-readable error message
    pub error_message: String,
    /// Suggested workarounds
    pub workarounds: Vec<Workaround>,
    /// Related documentation links
    pub documentation_links: Vec<String>,
}

impl ExtensionFailureInfo {
    pub fn new(name: impl Into<String>, reason: FailureReason) -> Self {
        let name = name.into();
        let error_message = reason.to_error_message(&name);
        let workarounds = reason.suggested_workarounds(&name);
        let documentation_links = reason.documentation_links();

        Self {
            extension_name: name,
            extension_path: None,
            failure_reason: reason,
            error_message,
            workarounds,
            documentation_links,
        }
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.extension_path = Some(path.into());
        self
    }

    /// Generate a detailed error message for display
    pub fn detailed_message(&self) -> String {
        let mut msg = String::new();
        msg.push_str(&format!("❌ Extension Load Failure: {}\n", self.extension_name));
        msg.push_str(&format!("   Reason: {}\n", self.error_message));

        if let Some(ref path) = self.extension_path {
            msg.push_str(&format!("   Path: {}\n", path.display()));
        }

        if !self.workarounds.is_empty() {
            msg.push_str("\n   💡 Suggested Workarounds:\n");
            for (i, workaround) in self.workarounds.iter().enumerate() {
                msg.push_str(&format!("      {}. {}\n", i + 1, workaround.description));
                if let Some(ref cmd) = workaround.command {
                    msg.push_str(&format!("         Command: {}\n", cmd));
                }
            }
        }

        if !self.documentation_links.is_empty() {
            msg.push_str("\n   📚 Documentation:\n");
            for link in &self.documentation_links {
                msg.push_str(&format!("      - {}\n", link));
            }
        }

        msg
    }
}

/// Specific reasons for extension failures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureReason {
    /// Extension file not found
    NotFound { searched_paths: Vec<PathBuf> },
    /// ABI version mismatch
    AbiMismatch {
        expected_version: String,
        found_version: String,
    },
    /// Missing dependencies
    MissingDependencies { dependencies: Vec<String> },
    /// Unsupported platform
    UnsupportedPlatform { platform: String },
    /// Missing CPython API functions
    MissingApiFunctions { functions: Vec<String> },
    /// Initialization failure
    InitializationFailed { reason: String },
    /// Permission denied
    PermissionDenied,
    /// Corrupted extension file
    CorruptedFile,
    /// Unknown error
    Unknown { details: String },
}

impl FailureReason {
    /// Generate a human-readable error message
    pub fn to_error_message(&self, extension_name: &str) -> String {
        match self {
            FailureReason::NotFound { searched_paths } => {
                let paths: Vec<String> =
                    searched_paths.iter().map(|p| p.display().to_string()).collect();
                format!(
                    "Extension '{}' not found. Searched in: {}",
                    extension_name,
                    paths.join(", ")
                )
            }
            FailureReason::AbiMismatch {
                expected_version,
                found_version,
            } => {
                format!(
                    "ABI version mismatch for '{}': DX-Py expects {}, but extension was built for {}",
                    extension_name, expected_version, found_version
                )
            }
            FailureReason::MissingDependencies { dependencies } => {
                format!(
                    "Extension '{}' requires missing dependencies: {}",
                    extension_name,
                    dependencies.join(", ")
                )
            }
            FailureReason::UnsupportedPlatform { platform } => {
                format!(
                    "Extension '{}' is not available for platform: {}",
                    extension_name, platform
                )
            }
            FailureReason::MissingApiFunctions { functions } => {
                format!(
                    "Extension '{}' requires unimplemented CPython API functions: {}",
                    extension_name,
                    functions.join(", ")
                )
            }
            FailureReason::InitializationFailed { reason } => {
                format!("Extension '{}' failed to initialize: {}", extension_name, reason)
            }
            FailureReason::PermissionDenied => {
                format!("Permission denied when loading extension '{}'", extension_name)
            }
            FailureReason::CorruptedFile => {
                format!("Extension file '{}' appears to be corrupted", extension_name)
            }
            FailureReason::Unknown { details } => {
                format!("Unknown error loading extension '{}': {}", extension_name, details)
            }
        }
    }

    /// Get suggested workarounds for this failure
    pub fn suggested_workarounds(&self, extension_name: &str) -> Vec<Workaround> {
        match self {
            FailureReason::NotFound { .. } => vec![
                Workaround::new(
                    format!("Install {} using pip", extension_name),
                    Some(format!("pip install {}", extension_name)),
                ),
                Workaround::new("Check if the extension is in your PYTHONPATH".to_string(), None),
                Workaround::new(
                    "Verify the extension is compatible with your Python version".to_string(),
                    None,
                ),
            ],
            FailureReason::AbiMismatch { .. } => vec![
                Workaround::new(
                    format!("Rebuild {} for DX-Py's Python version", extension_name),
                    Some(format!("pip install --force-reinstall {}", extension_name)),
                ),
                Workaround::new("Use a pure-Python alternative if available".to_string(), None),
            ],
            FailureReason::MissingDependencies { dependencies } => {
                let mut workarounds = vec![Workaround::new(
                    "Install missing dependencies".to_string(),
                    Some(format!("pip install {}", dependencies.join(" "))),
                )];
                workarounds.push(Workaround::new(
                    "Check system package manager for native libraries".to_string(),
                    None,
                ));
                workarounds
            }
            FailureReason::UnsupportedPlatform { .. } => vec![
                Workaround::new("Use a pure-Python alternative".to_string(), None),
                Workaround::new(
                    "Build the extension from source for your platform".to_string(),
                    None,
                ),
            ],
            FailureReason::MissingApiFunctions { functions } => vec![
                Workaround::new(
                    format!("Report missing APIs to DX-Py: {}", functions.join(", ")),
                    None,
                ),
                Workaround::new("Use a pure-Python alternative if available".to_string(), None),
                Workaround::new(
                    "Check DX-Py compatibility matrix for supported extensions".to_string(),
                    None,
                ),
            ],
            FailureReason::InitializationFailed { .. } => vec![
                Workaround::new(
                    "Check extension documentation for initialization requirements".to_string(),
                    None,
                ),
                Workaround::new(
                    "Verify all environment variables are set correctly".to_string(),
                    None,
                ),
            ],
            FailureReason::PermissionDenied => vec![
                Workaround::new("Check file permissions on the extension".to_string(), None),
                Workaround::new("Run with appropriate permissions".to_string(), None),
            ],
            FailureReason::CorruptedFile => vec![
                Workaround::new(
                    format!("Reinstall {}", extension_name),
                    Some(format!("pip install --force-reinstall {}", extension_name)),
                ),
                Workaround::new("Verify download integrity".to_string(), None),
            ],
            FailureReason::Unknown { .. } => vec![
                Workaround::new("Check DX-Py logs for more details".to_string(), None),
                Workaround::new("Report this issue to DX-Py developers".to_string(), None),
            ],
        }
    }

    /// Get relevant documentation links
    pub fn documentation_links(&self) -> Vec<String> {
        vec![
            "https://dx-py.dev/docs/compatibility".to_string(),
            "https://dx-py.dev/docs/c-extensions".to_string(),
        ]
    }
}

/// A suggested workaround for an incompatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workaround {
    /// Description of the workaround
    pub description: String,
    /// Optional command to run
    pub command: Option<String>,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f64,
}

impl Workaround {
    pub fn new(description: impl Into<String>, command: Option<String>) -> Self {
        Self {
            description: description.into(),
            command,
            confidence: 0.5,
        }
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Feature-level compatibility information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureCompatibility {
    /// Feature name
    pub name: String,
    /// Whether the feature is supported
    pub supported: bool,
    /// Support level (full, partial, none)
    pub support_level: SupportLevel,
    /// Notes about the feature
    pub notes: Option<String>,
    /// Sub-features
    pub sub_features: Vec<FeatureCompatibility>,
}

impl FeatureCompatibility {
    pub fn new(name: impl Into<String>, supported: bool) -> Self {
        Self {
            name: name.into(),
            supported,
            support_level: if supported {
                SupportLevel::Full
            } else {
                SupportLevel::None
            },
            notes: None,
            sub_features: Vec::new(),
        }
    }

    pub fn partial(name: impl Into<String>, notes: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            supported: true,
            support_level: SupportLevel::Partial,
            notes: Some(notes.into()),
            sub_features: Vec::new(),
        }
    }

    pub fn with_sub_features(mut self, features: Vec<FeatureCompatibility>) -> Self {
        self.sub_features = features;
        self
    }
}

/// Level of support for a feature
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SupportLevel {
    /// Fully supported
    Full,
    /// Partially supported
    Partial,
    /// Not supported
    None,
}

impl SupportLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupportLevel::Full => "Full",
            SupportLevel::Partial => "Partial",
            SupportLevel::None => "None",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            SupportLevel::Full => "✅",
            SupportLevel::Partial => "⚠️",
            SupportLevel::None => "❌",
        }
    }
}

/// Partial compatibility report for a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialCompatibilityReport {
    /// Framework name
    pub framework: String,
    /// Framework version
    pub version: String,
    /// Overall support level
    pub overall_support: SupportLevel,
    /// Feature-level compatibility
    pub features: Vec<FeatureCompatibility>,
    /// Known issues
    pub known_issues: Vec<KnownIssue>,
    /// Recommendations
    pub recommendations: Vec<String>,
}

impl PartialCompatibilityReport {
    pub fn new(framework: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            framework: framework.into(),
            version: version.into(),
            overall_support: SupportLevel::None,
            features: Vec::new(),
            known_issues: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    pub fn with_features(mut self, features: Vec<FeatureCompatibility>) -> Self {
        self.features = features;
        self.update_overall_support();
        self
    }

    pub fn with_known_issues(mut self, issues: Vec<KnownIssue>) -> Self {
        self.known_issues = issues;
        self
    }

    pub fn with_recommendations(mut self, recommendations: Vec<String>) -> Self {
        self.recommendations = recommendations;
        self
    }

    fn update_overall_support(&mut self) {
        if self.features.is_empty() {
            self.overall_support = SupportLevel::None;
            return;
        }

        let all_full = self.features.iter().all(|f| f.support_level == SupportLevel::Full);
        let any_supported = self.features.iter().any(|f| f.supported);

        self.overall_support = if all_full {
            SupportLevel::Full
        } else if any_supported {
            SupportLevel::Partial
        } else {
            SupportLevel::None
        };
    }

    /// Generate a markdown report
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# {} {} Compatibility Report\n\n", self.framework, self.version));
        md.push_str(&format!(
            "Overall Support: {} {}\n\n",
            self.overall_support.emoji(),
            self.overall_support.as_str()
        ));

        md.push_str("## Features\n\n");
        md.push_str("| Feature | Support | Notes |\n");
        md.push_str("|---------|---------|-------|\n");

        for feature in &self.features {
            let notes = feature.notes.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| {} | {} {} | {} |\n",
                feature.name,
                feature.support_level.emoji(),
                feature.support_level.as_str(),
                notes
            ));
        }

        if !self.known_issues.is_empty() {
            md.push_str("\n## Known Issues\n\n");
            for issue in &self.known_issues {
                md.push_str(&format!(
                    "- **{}**: {} (Severity: {})\n",
                    issue.title, issue.description, issue.severity
                ));
            }
        }

        if !self.recommendations.is_empty() {
            md.push_str("\n## Recommendations\n\n");
            for rec in &self.recommendations {
                md.push_str(&format!("- {}\n", rec));
            }
        }

        md
    }
}

/// A known issue with a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownIssue {
    /// Issue title
    pub title: String,
    /// Issue description
    pub description: String,
    /// Severity level
    pub severity: IssueSeverity,
    /// Affected features
    pub affected_features: Vec<String>,
    /// Workaround if available
    pub workaround: Option<String>,
}

impl KnownIssue {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        severity: IssueSeverity,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            severity,
            affected_features: Vec::new(),
            workaround: None,
        }
    }

    pub fn with_workaround(mut self, workaround: impl Into<String>) -> Self {
        self.workaround = Some(workaround.into());
        self
    }

    pub fn with_affected_features(mut self, features: Vec<String>) -> Self {
        self.affected_features = features;
        self
    }
}

/// Severity level for known issues
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Critical,
    High,
    Medium,
    Low,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueSeverity::Critical => write!(f, "Critical"),
            IssueSeverity::High => write!(f, "High"),
            IssueSeverity::Medium => write!(f, "Medium"),
            IssueSeverity::Low => write!(f, "Low"),
        }
    }
}

/// Compatibility checker for pre-execution scanning
pub struct CompatibilityChecker {
    /// Known incompatibilities database
    known_incompatibilities: HashMap<String, Vec<IncompatibilityInfo>>,
    /// Supported extensions
    supported_extensions: HashMap<String, SupportLevel>,
}

impl CompatibilityChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            known_incompatibilities: HashMap::new(),
            supported_extensions: HashMap::new(),
        };
        checker.load_defaults();
        checker
    }

    fn load_defaults(&mut self) {
        // Load known incompatibilities for common packages
        self.add_incompatibility(
            "tensorflow",
            IncompatibilityInfo::new(
                "TensorFlow C extensions",
                "TensorFlow uses custom C extensions that require specific CPython internals",
                IncompatibilityType::CExtension,
            ),
        );

        self.add_incompatibility(
            "torch",
            IncompatibilityInfo::new(
                "PyTorch C++ extensions",
                "PyTorch uses custom C++ extensions with CUDA support",
                IncompatibilityType::CExtension,
            ),
        );

        // Add supported extensions
        self.supported_extensions.insert("numpy".to_string(), SupportLevel::Full);
        self.supported_extensions.insert("pandas".to_string(), SupportLevel::Full);
        self.supported_extensions.insert("django".to_string(), SupportLevel::Full);
        self.supported_extensions.insert("flask".to_string(), SupportLevel::Full);
        self.supported_extensions.insert("fastapi".to_string(), SupportLevel::Full);
        self.supported_extensions.insert("requests".to_string(), SupportLevel::Full);
        self.supported_extensions
            .insert("sqlalchemy".to_string(), SupportLevel::Partial);
    }

    fn add_incompatibility(&mut self, package: &str, info: IncompatibilityInfo) {
        self.known_incompatibilities.entry(package.to_string()).or_default().push(info);
    }

    /// Check compatibility of a list of imports
    pub fn check_imports(&self, imports: &[String]) -> CompatibilityCheckResult {
        let mut result = CompatibilityCheckResult::new();

        for import in imports {
            // Extract base package name
            let base_package = import.split('.').next().unwrap_or(import);

            // Check for known incompatibilities
            if let Some(incompatibilities) = self.known_incompatibilities.get(base_package) {
                for incompatibility in incompatibilities {
                    result.add_incompatibility(base_package.to_string(), incompatibility.clone());
                }
            }

            // Check support level
            if let Some(level) = self.supported_extensions.get(base_package) {
                result.add_support_info(base_package.to_string(), *level);
            }
        }

        result
    }

    /// Scan a Python file for imports and check compatibility
    pub fn scan_file(&self, content: &str) -> CompatibilityCheckResult {
        let imports = self.extract_imports(content);
        self.check_imports(&imports)
    }

    /// Extract import statements from Python code
    pub fn extract_imports(&self, content: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Handle "import x" and "import x, y, z"
            if trimmed.starts_with("import ") {
                let rest = trimmed.strip_prefix("import ").unwrap_or("");
                for part in rest.split(',') {
                    let module = part.split_whitespace().next().unwrap_or("");
                    if !module.is_empty() {
                        imports.push(module.to_string());
                    }
                }
            }
            // Handle "from x import y"
            else if trimmed.starts_with("from ") {
                if let Some(rest) = trimmed.strip_prefix("from ") {
                    if let Some(module) = rest.split_whitespace().next() {
                        imports.push(module.to_string());
                    }
                }
            }
        }

        imports
    }

    /// Get support level for a package
    pub fn get_support_level(&self, package: &str) -> Option<SupportLevel> {
        self.supported_extensions.get(package).copied()
    }

    /// Check if a package has known incompatibilities
    pub fn has_incompatibilities(&self, package: &str) -> bool {
        self.known_incompatibilities.contains_key(package)
    }
}

impl Default for CompatibilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a known incompatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompatibilityInfo {
    /// Short title
    pub title: String,
    /// Detailed description
    pub description: String,
    /// Type of incompatibility
    pub incompatibility_type: IncompatibilityType,
    /// Severity
    pub severity: IssueSeverity,
    /// Workaround if available
    pub workaround: Option<String>,
}

impl IncompatibilityInfo {
    pub fn new(
        title: impl Into<String>,
        description: impl Into<String>,
        incompatibility_type: IncompatibilityType,
    ) -> Self {
        Self {
            title: title.into(),
            description: description.into(),
            incompatibility_type,
            severity: IssueSeverity::High,
            workaround: None,
        }
    }

    pub fn with_severity(mut self, severity: IssueSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_workaround(mut self, workaround: impl Into<String>) -> Self {
        self.workaround = Some(workaround.into());
        self
    }
}

/// Type of incompatibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncompatibilityType {
    /// C extension incompatibility
    CExtension,
    /// Missing CPython API
    MissingApi,
    /// Async/await issues
    AsyncBehavior,
    /// Platform-specific
    Platform,
    /// Other
    Other,
}

impl std::fmt::Display for IncompatibilityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncompatibilityType::CExtension => write!(f, "C Extension"),
            IncompatibilityType::MissingApi => write!(f, "Missing API"),
            IncompatibilityType::AsyncBehavior => write!(f, "Async Behavior"),
            IncompatibilityType::Platform => write!(f, "Platform"),
            IncompatibilityType::Other => write!(f, "Other"),
        }
    }
}

/// Result of a compatibility check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityCheckResult {
    /// Whether all imports are compatible
    pub is_compatible: bool,
    /// Incompatibilities found
    pub incompatibilities: HashMap<String, Vec<IncompatibilityInfo>>,
    /// Support levels for packages
    pub support_levels: HashMap<String, SupportLevel>,
    /// Warnings
    pub warnings: Vec<String>,
}

impl CompatibilityCheckResult {
    pub fn new() -> Self {
        Self {
            is_compatible: true,
            incompatibilities: HashMap::new(),
            support_levels: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_incompatibility(&mut self, package: String, info: IncompatibilityInfo) {
        self.is_compatible = false;
        self.incompatibilities.entry(package).or_default().push(info);
    }

    pub fn add_support_info(&mut self, package: String, level: SupportLevel) {
        if level == SupportLevel::Partial {
            self.warnings.push(format!(
                "Package '{}' has partial support - some features may not work",
                package
            ));
        }
        self.support_levels.insert(package, level);
    }

    /// Generate a report for display
    pub fn generate_report(&self) -> String {
        let mut report = String::new();

        if self.is_compatible && self.warnings.is_empty() {
            report.push_str("✅ All imports appear to be compatible with DX-Py\n");
            return report;
        }

        if !self.incompatibilities.is_empty() {
            report.push_str("❌ Incompatibilities Found:\n\n");
            for (package, infos) in &self.incompatibilities {
                for info in infos {
                    report.push_str(&format!("  {} - {}\n", package, info.title));
                    report.push_str(&format!("    Type: {}\n", info.incompatibility_type));
                    report.push_str(&format!("    Severity: {}\n", info.severity));
                    report.push_str(&format!("    {}\n", info.description));
                    if let Some(ref workaround) = info.workaround {
                        report.push_str(&format!("    Workaround: {}\n", workaround));
                    }
                    report.push('\n');
                }
            }
        }

        if !self.warnings.is_empty() {
            report.push_str("⚠️ Warnings:\n");
            for warning in &self.warnings {
                report.push_str(&format!("  - {}\n", warning));
            }
        }

        report
    }
}

impl Default for CompatibilityCheckResult {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_failure_info() {
        let info = ExtensionFailureInfo::new(
            "numpy",
            FailureReason::AbiMismatch {
                expected_version: "3.11".to_string(),
                found_version: "3.10".to_string(),
            },
        );

        assert_eq!(info.extension_name, "numpy");
        assert!(!info.workarounds.is_empty());
        assert!(info.error_message.contains("ABI"));
    }

    #[test]
    fn test_failure_reason_messages() {
        let reason = FailureReason::NotFound {
            searched_paths: vec![
                PathBuf::from("/usr/lib"),
                PathBuf::from("/home/user/.local"),
            ],
        };
        let msg = reason.to_error_message("test_ext");
        assert!(msg.contains("not found"));
        assert!(msg.contains("/usr/lib"));
    }

    #[test]
    fn test_workaround() {
        let workaround = Workaround::new("Install package", Some("pip install pkg".to_string()))
            .with_confidence(0.8);

        assert_eq!(workaround.confidence, 0.8);
        assert!(workaround.command.is_some());
    }

    #[test]
    fn test_feature_compatibility() {
        let feature = FeatureCompatibility::partial("ORM", "Some advanced queries not supported");
        assert!(feature.supported);
        assert_eq!(feature.support_level, SupportLevel::Partial);
    }

    #[test]
    fn test_partial_compatibility_report() {
        let report = PartialCompatibilityReport::new("Django", "4.2").with_features(vec![
            FeatureCompatibility::new("Core", true),
            FeatureCompatibility::partial("ORM", "Complex queries limited"),
            FeatureCompatibility::new("Admin", false),
        ]);

        assert_eq!(report.overall_support, SupportLevel::Partial);
        let md = report.generate_markdown();
        assert!(md.contains("Django"));
        assert!(md.contains("Partial"));
    }

    #[test]
    fn test_compatibility_checker() {
        let checker = CompatibilityChecker::new();

        // Check supported package
        assert_eq!(checker.get_support_level("numpy"), Some(SupportLevel::Full));

        // Check package with known incompatibilities
        assert!(checker.has_incompatibilities("tensorflow"));
    }

    #[test]
    fn test_import_extraction() {
        let checker = CompatibilityChecker::new();
        let code = r#"
import numpy
import pandas as pd
from django.db import models
from flask import Flask, request
import os, sys
"#;

        let imports = checker.extract_imports(code);
        assert!(imports.contains(&"numpy".to_string()));
        assert!(imports.contains(&"pandas".to_string()));
        assert!(imports.contains(&"django.db".to_string()));
        assert!(imports.contains(&"flask".to_string()));
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"sys".to_string()));
    }

    #[test]
    fn test_compatibility_check() {
        let checker = CompatibilityChecker::new();
        let result = checker.check_imports(&["numpy".to_string(), "tensorflow".to_string()]);

        assert!(!result.is_compatible);
        assert!(result.incompatibilities.contains_key("tensorflow"));
        assert_eq!(result.support_levels.get("numpy"), Some(&SupportLevel::Full));
    }

    #[test]
    fn test_support_level() {
        assert_eq!(SupportLevel::Full.as_str(), "Full");
        assert_eq!(SupportLevel::Partial.emoji(), "⚠️");
        assert_eq!(SupportLevel::None.emoji(), "❌");
    }

    #[test]
    fn test_known_issue() {
        let issue = KnownIssue::new(
            "Memory leak",
            "Memory leak in large DataFrame operations",
            IssueSeverity::High,
        )
        .with_workaround("Use chunked processing")
        .with_affected_features(vec!["DataFrame".to_string()]);

        assert!(issue.workaround.is_some());
        assert!(!issue.affected_features.is_empty());
    }
}
