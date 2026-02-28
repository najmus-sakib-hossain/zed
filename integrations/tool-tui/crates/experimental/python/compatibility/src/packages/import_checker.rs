//! Import checking for Python packages
//!
//! Provides functionality to check if packages can be imported
//! and detect import errors.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

/// Result of an import check
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// Package name
    pub package: String,
    /// Whether import succeeded
    pub success: bool,
    /// Import time
    pub duration: Duration,
    /// Error if import failed
    pub error: Option<ImportError>,
    /// Submodules that were imported
    pub submodules: Vec<String>,
}

/// Import error details
#[derive(Debug, Clone)]
pub struct ImportError {
    /// Error type (ImportError, ModuleNotFoundError, etc.)
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Missing module if applicable
    pub missing_module: Option<String>,
    /// Traceback
    pub traceback: Option<String>,
}

/// Import checker for Python packages
pub struct ImportChecker {
    python_path: PathBuf,
    timeout: Duration,
    cache: HashMap<String, ImportResult>,
}

impl ImportChecker {
    /// Create a new import checker
    pub fn new(python_path: PathBuf) -> Self {
        Self {
            python_path,
            timeout: Duration::from_secs(30),
            cache: HashMap::new(),
        }
    }

    /// Set timeout for import checks
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if a package can be imported
    pub fn check_import(&mut self, package: &str) -> ImportResult {
        // Check cache first
        if let Some(result) = self.cache.get(package) {
            return result.clone();
        }

        let result = self.do_check_import(package);
        self.cache.insert(package.to_string(), result.clone());
        result
    }

    /// Check multiple packages
    pub fn check_imports(&mut self, packages: &[&str]) -> Vec<ImportResult> {
        packages.iter().map(|p| self.check_import(p)).collect()
    }

    /// Check import with submodules
    pub fn check_import_deep(&mut self, package: &str) -> ImportResult {
        let mut result = self.check_import(package);

        if result.success {
            result.submodules = self.discover_submodules(package);
        }

        result
    }

    /// Perform the actual import check
    fn do_check_import(&self, package: &str) -> ImportResult {
        let start = Instant::now();
        let import_name = Self::normalize_import(package);

        let code = format!(
            r#"
import sys
import traceback

try:
    import {}
    print("SUCCESS")
except ImportError as e:
    print("IMPORT_ERROR")
    print(str(e))
    traceback.print_exc()
except ModuleNotFoundError as e:
    print("MODULE_NOT_FOUND")
    print(str(e))
    if hasattr(e, 'name'):
        print("MISSING:" + str(e.name))
    traceback.print_exc()
except Exception as e:
    print("ERROR")
    print(str(e))
    traceback.print_exc()
"#,
            import_name
        );

        let output = Command::new(&self.python_path).args(["-c", &code]).output();

        let duration = start.elapsed();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);

                if stdout.starts_with("SUCCESS") {
                    ImportResult {
                        package: package.to_string(),
                        success: true,
                        duration,
                        error: None,
                        submodules: vec![],
                    }
                } else {
                    let lines: Vec<&str> = stdout.lines().collect();
                    let error_type = lines.first().unwrap_or(&"ERROR").to_string();
                    let message = lines.get(1).unwrap_or(&"Unknown error").to_string();
                    let missing_module = lines
                        .iter()
                        .find(|l| l.starts_with("MISSING:"))
                        .map(|l| l.trim_start_matches("MISSING:").to_string());

                    ImportResult {
                        package: package.to_string(),
                        success: false,
                        duration,
                        error: Some(ImportError {
                            error_type,
                            message,
                            missing_module,
                            traceback: Some(stderr.to_string()),
                        }),
                        submodules: vec![],
                    }
                }
            }
            Err(e) => ImportResult {
                package: package.to_string(),
                success: false,
                duration,
                error: Some(ImportError {
                    error_type: "ProcessError".to_string(),
                    message: e.to_string(),
                    missing_module: None,
                    traceback: None,
                }),
                submodules: vec![],
            },
        }
    }

    /// Discover submodules of a package
    fn discover_submodules(&self, package: &str) -> Vec<String> {
        let import_name = Self::normalize_import(package);

        let code = format!(
            r#"
import {}
import pkgutil
import sys

module = sys.modules['{}']
if hasattr(module, '__path__'):
    for importer, modname, ispkg in pkgutil.iter_modules(module.__path__):
        print(modname)
"#,
            import_name, import_name
        );

        let output = Command::new(&self.python_path).args(["-c", &code]).output();

        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).lines().map(|s| s.to_string()).collect()
            }
            _ => vec![],
        }
    }

    /// Normalize package name to import name
    fn normalize_import(package: &str) -> &str {
        match package {
            "Pillow" | "pillow" => "PIL",
            "scikit-learn" => "sklearn",
            "beautifulsoup4" => "bs4",
            "python-dateutil" => "dateutil",
            "PyYAML" => "yaml",
            "typing-extensions" => "typing_extensions",
            "importlib-metadata" => "importlib_metadata",
            _ => package,
        }
    }

    /// Clear the import cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cached results
    pub fn cached_results(&self) -> &HashMap<String, ImportResult> {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_error_creation() {
        let error = ImportError {
            error_type: "ModuleNotFoundError".to_string(),
            message: "No module named 'nonexistent'".to_string(),
            missing_module: Some("nonexistent".to_string()),
            traceback: None,
        };

        assert_eq!(error.error_type, "ModuleNotFoundError");
        assert_eq!(error.missing_module, Some("nonexistent".to_string()));
    }

    #[test]
    fn test_import_result_creation() {
        let result = ImportResult {
            package: "requests".to_string(),
            success: true,
            duration: Duration::from_millis(100),
            error: None,
            submodules: vec!["adapters".to_string(), "auth".to_string()],
        };

        assert!(result.success);
        assert_eq!(result.submodules.len(), 2);
    }
}
