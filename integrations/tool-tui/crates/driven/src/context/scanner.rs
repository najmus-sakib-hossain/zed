//! Project structure scanner

use crate::{DrivenError, Result};
use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

/// Result of scanning a project
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    /// Detected programming languages
    pub languages: Vec<String>,
    /// Detected frameworks
    pub frameworks: Vec<String>,
    /// Project type
    pub project_type: Option<String>,
    /// Key directories
    pub key_directories: Vec<String>,
    /// Configuration files
    pub config_files: Vec<String>,
    /// Has test files
    pub has_tests: bool,
    /// Has CI configuration
    pub has_ci: bool,
    /// Has documentation
    pub has_docs: bool,
    /// File count by extension
    pub file_counts: std::collections::HashMap<String, usize>,
}

/// Scans project structure
#[derive(Debug, Default)]
pub struct ProjectScanner {
    /// Maximum depth to scan
    max_depth: usize,
    /// Patterns to ignore
    ignore_patterns: Vec<String>,
}

impl ProjectScanner {
    /// Create a new scanner
    pub fn new() -> Self {
        Self {
            max_depth: 10,
            ignore_patterns: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
                ".next".to_string(),
                "vendor".to_string(),
            ],
        }
    }

    /// Set maximum scan depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Scan a project directory
    pub fn scan(&self, path: &Path) -> Result<ScanResult> {
        if !path.exists() {
            return Err(DrivenError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path does not exist: {}", path.display()),
            )));
        }

        let mut result = ScanResult::default();
        let mut languages = HashSet::new();
        let mut frameworks = HashSet::new();

        for entry in WalkDir::new(path)
            .max_depth(self.max_depth)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e))
            .filter_map(|e| e.ok())
        {
            let entry_path = entry.path();

            if entry_path.is_file() {
                // Detect language from extension
                if let Some(ext) = entry_path.extension().and_then(|e| e.to_str()) {
                    *result.file_counts.entry(ext.to_string()).or_insert(0) += 1;

                    if let Some(lang) = self.extension_to_language(ext) {
                        languages.insert(lang);
                    }
                }

                // Detect configuration files
                if let Some(file_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    if self.is_config_file(file_name) {
                        result.config_files.push(file_name.to_string());

                        // Detect frameworks from config files
                        if let Some(framework) = self.config_to_framework(file_name) {
                            frameworks.insert(framework);
                        }
                    }

                    // Detect tests
                    if file_name.contains("test") || file_name.contains("spec") {
                        result.has_tests = true;
                    }
                }
            } else if entry_path.is_dir() {
                if let Some(dir_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    // Detect key directories
                    if self.is_key_directory(dir_name) {
                        result.key_directories.push(dir_name.to_string());
                    }

                    // Detect CI
                    if dir_name == ".github" || dir_name == ".gitlab-ci" || dir_name == ".circleci"
                    {
                        result.has_ci = true;
                    }

                    // Detect docs
                    if dir_name == "docs" || dir_name == "documentation" {
                        result.has_docs = true;
                    }

                    // Detect tests directory
                    if dir_name == "tests" || dir_name == "test" || dir_name == "__tests__" {
                        result.has_tests = true;
                    }
                }
            }
        }

        result.languages = languages.into_iter().collect();
        result.frameworks = frameworks.into_iter().collect();

        // Detect project type
        result.project_type = self.detect_project_type(&result);

        Ok(result)
    }

    fn should_ignore(&self, entry: &walkdir::DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| self.ignore_patterns.iter().any(|p| s == p))
            .unwrap_or(false)
    }

    fn extension_to_language(&self, ext: &str) -> Option<String> {
        match ext.to_lowercase().as_str() {
            "rs" => Some("Rust".to_string()),
            "ts" | "tsx" => Some("TypeScript".to_string()),
            "js" | "jsx" | "mjs" | "cjs" => Some("JavaScript".to_string()),
            "py" => Some("Python".to_string()),
            "go" => Some("Go".to_string()),
            "java" => Some("Java".to_string()),
            "kt" | "kts" => Some("Kotlin".to_string()),
            "swift" => Some("Swift".to_string()),
            "c" | "h" => Some("C".to_string()),
            "cpp" | "cc" | "hpp" => Some("C++".to_string()),
            "cs" => Some("C#".to_string()),
            "rb" => Some("Ruby".to_string()),
            "php" => Some("PHP".to_string()),
            "md" => Some("Markdown".to_string()),
            _ => None,
        }
    }

    fn is_config_file(&self, name: &str) -> bool {
        matches!(
            name,
            "Cargo.toml"
                | "package.json"
                | "tsconfig.json"
                | "pyproject.toml"
                | "go.mod"
                | "pom.xml"
                | "build.gradle"
                | "Gemfile"
                | "composer.json"
                | ".eslintrc.json"
                | ".prettierrc"
                | "tailwind.config.js"
                | "next.config.js"
                | "vite.config.ts"
                | "webpack.config.js"
        )
    }

    fn config_to_framework(&self, name: &str) -> Option<String> {
        match name {
            "next.config.js" | "next.config.mjs" => Some("Next.js".to_string()),
            "vite.config.ts" | "vite.config.js" => Some("Vite".to_string()),
            "tailwind.config.js" => Some("Tailwind CSS".to_string()),
            "angular.json" => Some("Angular".to_string()),
            "vue.config.js" => Some("Vue".to_string()),
            _ => None,
        }
    }

    fn is_key_directory(&self, name: &str) -> bool {
        matches!(
            name,
            "src"
                | "lib"
                | "crates"
                | "packages"
                | "apps"
                | "components"
                | "api"
                | "server"
                | "client"
                | "frontend"
                | "backend"
        )
    }

    fn detect_project_type(&self, result: &ScanResult) -> Option<String> {
        // Check for specific patterns
        if result.config_files.contains(&"Cargo.toml".to_string()) {
            if result.key_directories.contains(&"crates".to_string()) {
                return Some("Rust Workspace".to_string());
            }
            if result.key_directories.contains(&"src".to_string()) {
                return Some("Rust Project".to_string());
            }
        }

        if result.config_files.contains(&"package.json".to_string()) {
            if result.key_directories.contains(&"packages".to_string())
                || result.key_directories.contains(&"apps".to_string())
            {
                return Some("Node.js Monorepo".to_string());
            }
            if result.frameworks.iter().any(|f| f == "Next.js") {
                return Some("Next.js App".to_string());
            }
            return Some("Node.js Project".to_string());
        }

        if result.config_files.contains(&"pyproject.toml".to_string()) {
            return Some("Python Project".to_string());
        }

        if result.config_files.contains(&"go.mod".to_string()) {
            return Some("Go Project".to_string());
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_new() {
        let scanner = ProjectScanner::new();
        assert_eq!(scanner.max_depth, 10);
    }

    #[test]
    fn test_extension_to_language() {
        let scanner = ProjectScanner::new();
        assert_eq!(scanner.extension_to_language("rs"), Some("Rust".to_string()));
        assert_eq!(scanner.extension_to_language("ts"), Some("TypeScript".to_string()));
        assert_eq!(scanner.extension_to_language("unknown"), None);
    }
}
