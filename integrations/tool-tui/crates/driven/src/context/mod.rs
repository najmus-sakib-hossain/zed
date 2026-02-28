//! AI Context Intelligence
//!
//! Deep project analysis for AI guidance.

mod analyzer;
mod extractor;
mod indexer;
mod provider;
mod scanner;

pub use analyzer::PatternAnalyzer;
pub use extractor::ConventionExtractor;
pub use indexer::CodebaseIndexer;
pub use provider::ContextProvider;
pub use scanner::ProjectScanner;

use crate::Result;
use std::path::Path;

/// Analyzed project context
#[derive(Debug, Clone, Default)]
pub struct ProjectContext {
    /// Detected programming languages
    pub languages: Vec<String>,
    /// Detected frameworks
    pub frameworks: Vec<String>,
    /// Project type (library, binary, webapp, etc.)
    pub project_type: Option<String>,
    /// Key directories
    pub directories: Vec<String>,
    /// Configuration files found
    pub config_files: Vec<String>,
    /// Detected naming conventions
    pub naming_conventions: NamingConventions,
    /// Dependencies (name, version)
    pub dependencies: Vec<(String, String)>,
    /// Key patterns detected
    pub patterns: Vec<String>,
}

/// Naming convention detection
#[derive(Debug, Clone, Default)]
pub struct NamingConventions {
    /// Function naming style
    pub functions: Option<NamingStyle>,
    /// Type naming style
    pub types: Option<NamingStyle>,
    /// Variable naming style
    pub variables: Option<NamingStyle>,
    /// File naming style
    pub files: Option<NamingStyle>,
}

/// Naming style types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingStyle {
    /// snake_case
    SnakeCase,
    /// camelCase
    CamelCase,
    /// PascalCase
    PascalCase,
    /// kebab-case
    KebabCase,
    /// SCREAMING_SNAKE_CASE
    ScreamingSnakeCase,
}

impl std::fmt::Display for NamingStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NamingStyle::SnakeCase => write!(f, "snake_case"),
            NamingStyle::CamelCase => write!(f, "camelCase"),
            NamingStyle::PascalCase => write!(f, "PascalCase"),
            NamingStyle::KebabCase => write!(f, "kebab-case"),
            NamingStyle::ScreamingSnakeCase => write!(f, "SCREAMING_SNAKE_CASE"),
        }
    }
}

/// Analyze a project and return context
pub struct ProjectAnalyzer {
    scanner: ProjectScanner,
    extractor: ConventionExtractor,
}

impl ProjectAnalyzer {
    /// Create a new project analyzer
    pub fn new() -> Self {
        Self {
            scanner: ProjectScanner::new(),
            extractor: ConventionExtractor::new(),
        }
    }

    /// Analyze a project at the given path
    pub fn analyze(&self, path: &Path) -> Result<ProjectContext> {
        // Scan the project structure
        let scan_result = self.scanner.scan(path)?;

        // Extract conventions
        let conventions = self.extractor.extract(&scan_result)?;

        // Build context
        let mut context = ProjectContext {
            languages: scan_result.languages.clone(),
            frameworks: scan_result.frameworks.clone(),
            project_type: scan_result.project_type.clone(),
            directories: scan_result.key_directories.clone(),
            config_files: scan_result.config_files.clone(),
            naming_conventions: conventions,
            ..Default::default()
        };

        // Detect patterns
        context.patterns = self.detect_patterns(&scan_result);

        Ok(context)
    }

    fn detect_patterns(&self, scan_result: &scanner::ScanResult) -> Vec<String> {
        let mut patterns = Vec::new();

        if scan_result.has_tests {
            patterns.push("Has test suite".to_string());
        }

        if scan_result.has_ci {
            patterns.push("Has CI/CD configuration".to_string());
        }

        if scan_result.has_docs {
            patterns.push("Has documentation".to_string());
        }

        patterns
    }
}

impl Default for ProjectAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naming_style_display() {
        assert_eq!(format!("{}", NamingStyle::SnakeCase), "snake_case");
        assert_eq!(format!("{}", NamingStyle::CamelCase), "camelCase");
        assert_eq!(format!("{}", NamingStyle::PascalCase), "PascalCase");
    }

    #[test]
    fn test_project_context_default() {
        let context = ProjectContext::default();
        assert!(context.languages.is_empty());
        assert!(context.project_type.is_none());
    }
}
