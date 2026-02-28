//! Compatibility report generation
//!
//! Generates detailed reports about package compatibility with DX-Py.

use std::collections::HashMap;
use std::time::Duration;

use super::verifier::{VerificationResult, VerificationStatus};

/// Overall compatibility report
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    /// Total packages tested
    pub total_packages: usize,
    /// Compatible packages
    pub compatible: usize,
    /// Partially compatible packages
    pub partially_compatible: usize,
    /// Incompatible packages
    pub incompatible: usize,
    /// Not installed packages
    pub not_installed: usize,
    /// Error during verification
    pub errors: usize,
    /// Total verification time
    pub total_duration: Duration,
    /// Individual package results
    pub packages: Vec<PackageCompatibility>,
    /// Compatibility by category
    pub by_category: HashMap<String, CategoryCompatibility>,
}

/// Compatibility info for a single package
#[derive(Debug, Clone)]
pub struct PackageCompatibility {
    /// Package name
    pub name: String,
    /// Compatibility level
    pub level: CompatibilityLevel,
    /// Import works
    pub import_ok: bool,
    /// Basic operations work
    pub basic_ops_ok: bool,
    /// Verification duration
    pub duration: Duration,
    /// Issues found
    pub issues: Vec<String>,
    /// Category (web, data, etc.)
    pub category: String,
}

/// Compatibility level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityLevel {
    /// Fully compatible
    Full,
    /// Mostly compatible with minor issues
    High,
    /// Partially compatible with some features not working
    Partial,
    /// Low compatibility, many features broken
    Low,
    /// Not compatible
    None,
    /// Unknown (not tested or error)
    Unknown,
}

/// Category compatibility summary
#[derive(Debug, Clone)]
pub struct CategoryCompatibility {
    /// Category name
    pub name: String,
    /// Total packages in category
    pub total: usize,
    /// Compatible packages
    pub compatible: usize,
    /// Compatibility percentage
    pub percentage: f64,
}

impl CompatibilityReport {
    /// Create a new report from verification results
    pub fn from_results(results: Vec<VerificationResult>) -> Self {
        let total_packages = results.len();
        let mut compatible = 0;
        let mut partially_compatible = 0;
        let mut incompatible = 0;
        let mut not_installed = 0;
        let mut errors = 0;
        let mut total_duration = Duration::ZERO;
        let mut packages = Vec::new();
        let mut category_counts: HashMap<String, (usize, usize)> = HashMap::new();

        for result in results {
            total_duration += result.duration;

            let category = Self::categorize_package(&result.package);
            let entry = category_counts.entry(category.clone()).or_insert((0, 0));
            entry.0 += 1;

            let (level, issues) = match result.status {
                VerificationStatus::Compatible => {
                    compatible += 1;
                    entry.1 += 1;
                    (CompatibilityLevel::Full, vec![])
                }
                VerificationStatus::PartiallyCompatible => {
                    partially_compatible += 1;
                    let issues: Vec<String> = result
                        .test_results
                        .iter()
                        .filter(|t| !t.passed)
                        .map(|t| {
                            t.message.clone().unwrap_or_else(|| format!("Test '{}' failed", t.name))
                        })
                        .collect();
                    (CompatibilityLevel::Partial, issues)
                }
                VerificationStatus::Incompatible => {
                    incompatible += 1;
                    (CompatibilityLevel::None, vec![result.error.unwrap_or_default()])
                }
                VerificationStatus::NotInstalled => {
                    not_installed += 1;
                    (CompatibilityLevel::Unknown, vec!["Package not installed".to_string()])
                }
                VerificationStatus::Error => {
                    errors += 1;
                    (CompatibilityLevel::Unknown, vec![result.error.unwrap_or_default()])
                }
            };

            packages.push(PackageCompatibility {
                name: result.package,
                level,
                import_ok: result.import_ok,
                basic_ops_ok: result.basic_ops_ok,
                duration: result.duration,
                issues,
                category,
            });
        }

        let by_category: HashMap<String, CategoryCompatibility> = category_counts
            .into_iter()
            .map(|(name, (total, compat))| {
                let percentage = if total > 0 {
                    (compat as f64 / total as f64) * 100.0
                } else {
                    0.0
                };
                (
                    name.clone(),
                    CategoryCompatibility {
                        name,
                        total,
                        compatible: compat,
                        percentage,
                    },
                )
            })
            .collect();

        Self {
            total_packages,
            compatible,
            partially_compatible,
            incompatible,
            not_installed,
            errors,
            total_duration,
            packages,
            by_category,
        }
    }

    /// Get overall compatibility percentage
    pub fn compatibility_percentage(&self) -> f64 {
        let tested = self.total_packages - self.not_installed - self.errors;
        if tested == 0 {
            return 0.0;
        }
        ((self.compatible + self.partially_compatible) as f64 / tested as f64) * 100.0
    }

    /// Get full compatibility percentage (only fully compatible)
    pub fn full_compatibility_percentage(&self) -> f64 {
        let tested = self.total_packages - self.not_installed - self.errors;
        if tested == 0 {
            return 0.0;
        }
        (self.compatible as f64 / tested as f64) * 100.0
    }

    /// Generate a text summary
    pub fn summary(&self) -> String {
        let mut output = String::new();

        output.push_str("=== DX-Py Package Compatibility Report ===\n\n");

        output.push_str(&format!("Total packages tested: {}\n", self.total_packages));
        output.push_str(&format!(
            "  Compatible:          {} ({:.1}%)\n",
            self.compatible,
            self.full_compatibility_percentage()
        ));
        output.push_str(&format!("  Partially compatible: {}\n", self.partially_compatible));
        output.push_str(&format!("  Incompatible:        {}\n", self.incompatible));
        output.push_str(&format!("  Not installed:       {}\n", self.not_installed));
        output.push_str(&format!("  Errors:              {}\n", self.errors));
        output.push_str(&format!(
            "\nOverall compatibility: {:.1}%\n",
            self.compatibility_percentage()
        ));
        output.push_str(&format!(
            "Total verification time: {:.2}s\n",
            self.total_duration.as_secs_f64()
        ));

        output.push_str("\n--- By Category ---\n");
        for (name, cat) in &self.by_category {
            output.push_str(&format!(
                "  {}: {}/{} ({:.1}%)\n",
                name, cat.compatible, cat.total, cat.percentage
            ));
        }

        if !self.packages.iter().any(|p| p.level == CompatibilityLevel::None) {
            return output;
        }

        output.push_str("\n--- Incompatible Packages ---\n");
        for pkg in &self.packages {
            if pkg.level == CompatibilityLevel::None {
                output.push_str(&format!(
                    "  {}: {}\n",
                    pkg.name,
                    pkg.issues.first().unwrap_or(&"Unknown error".to_string())
                ));
            }
        }

        output
    }

    /// Generate JSON report
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n");

        json.push_str(&format!("  \"total_packages\": {},\n", self.total_packages));
        json.push_str(&format!("  \"compatible\": {},\n", self.compatible));
        json.push_str(&format!("  \"partially_compatible\": {},\n", self.partially_compatible));
        json.push_str(&format!("  \"incompatible\": {},\n", self.incompatible));
        json.push_str(&format!("  \"not_installed\": {},\n", self.not_installed));
        json.push_str(&format!("  \"errors\": {},\n", self.errors));
        json.push_str(&format!(
            "  \"compatibility_percentage\": {:.2},\n",
            self.compatibility_percentage()
        ));
        json.push_str(&format!("  \"total_duration_ms\": {},\n", self.total_duration.as_millis()));

        json.push_str("  \"packages\": [\n");
        for (i, pkg) in self.packages.iter().enumerate() {
            json.push_str("    {\n");
            json.push_str(&format!("      \"name\": \"{}\",\n", pkg.name));
            json.push_str(&format!("      \"level\": \"{:?}\",\n", pkg.level));
            json.push_str(&format!("      \"import_ok\": {},\n", pkg.import_ok));
            json.push_str(&format!("      \"basic_ops_ok\": {},\n", pkg.basic_ops_ok));
            json.push_str(&format!("      \"category\": \"{}\"\n", pkg.category));
            json.push_str(&format!(
                "    }}{}\n",
                if i < self.packages.len() - 1 { "," } else { "" }
            ));
        }
        json.push_str("  ]\n");

        json.push_str("}\n");
        json
    }

    /// Categorize a package
    fn categorize_package(package: &str) -> String {
        match package {
            "requests" | "flask" | "django" | "fastapi" | "aiohttp" | "httpx" | "urllib3"
            | "starlette" | "uvicorn" | "gunicorn" | "werkzeug" | "jinja2" => "web".to_string(),

            "numpy" | "pandas" | "scipy" | "matplotlib" | "scikit-learn" | "tensorflow"
            | "torch" | "keras" | "xgboost" | "lightgbm" => "data_science".to_string(),

            "sqlalchemy" | "psycopg2" | "pymysql" | "redis" | "pymongo" | "aiosqlite"
            | "asyncpg" | "motor" | "elasticsearch" => "database".to_string(),

            "pytest" | "unittest" | "mock" | "coverage" | "hypothesis" | "faker" | "responses" => {
                "testing".to_string()
            }

            "click" | "typer" | "rich" | "tqdm" | "colorama" | "argparse" | "fire" | "docopt" => {
                "cli".to_string()
            }

            "asyncio" | "trio" | "anyio" | "aiofiles" => "async".to_string(),

            "cryptography" | "pycryptodome" | "bcrypt" => "security".to_string(),

            "Pillow" | "opencv-python" | "imageio" | "scikit-image" => "imaging".to_string(),

            "boto3" | "botocore" | "aiobotocore" | "s3fs" => "aws".to_string(),

            _ => "other".to_string(),
        }
    }
}

impl CompatibilityLevel {
    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            CompatibilityLevel::Full => "Fully compatible",
            CompatibilityLevel::High => "Highly compatible",
            CompatibilityLevel::Partial => "Partially compatible",
            CompatibilityLevel::Low => "Low compatibility",
            CompatibilityLevel::None => "Not compatible",
            CompatibilityLevel::Unknown => "Unknown",
        }
    }

    /// Get emoji representation
    pub fn emoji(&self) -> &'static str {
        match self {
            CompatibilityLevel::Full => "✅",
            CompatibilityLevel::High => "🟢",
            CompatibilityLevel::Partial => "🟡",
            CompatibilityLevel::Low => "🟠",
            CompatibilityLevel::None => "❌",
            CompatibilityLevel::Unknown => "❓",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compatibility_level_description() {
        assert_eq!(CompatibilityLevel::Full.description(), "Fully compatible");
        assert_eq!(CompatibilityLevel::None.description(), "Not compatible");
    }

    #[test]
    fn test_categorize_package() {
        assert_eq!(CompatibilityReport::categorize_package("requests"), "web");
        assert_eq!(CompatibilityReport::categorize_package("numpy"), "data_science");
        assert_eq!(CompatibilityReport::categorize_package("sqlalchemy"), "database");
        assert_eq!(CompatibilityReport::categorize_package("pytest"), "testing");
    }

    #[test]
    fn test_report_from_results() {
        let results = vec![
            VerificationResult {
                package: "requests".to_string(),
                status: VerificationStatus::Compatible,
                duration: Duration::from_millis(100),
                import_ok: true,
                basic_ops_ok: true,
                error: None,
                test_results: vec![],
            },
            VerificationResult {
                package: "nonexistent".to_string(),
                status: VerificationStatus::NotInstalled,
                duration: Duration::from_millis(10),
                import_ok: false,
                basic_ops_ok: false,
                error: Some("Not installed".to_string()),
                test_results: vec![],
            },
        ];

        let report = CompatibilityReport::from_results(results);

        assert_eq!(report.total_packages, 2);
        assert_eq!(report.compatible, 1);
        assert_eq!(report.not_installed, 1);
    }

    #[test]
    fn test_compatibility_percentage() {
        let results = vec![
            VerificationResult {
                package: "pkg1".to_string(),
                status: VerificationStatus::Compatible,
                duration: Duration::from_millis(100),
                import_ok: true,
                basic_ops_ok: true,
                error: None,
                test_results: vec![],
            },
            VerificationResult {
                package: "pkg2".to_string(),
                status: VerificationStatus::Incompatible,
                duration: Duration::from_millis(100),
                import_ok: false,
                basic_ops_ok: false,
                error: Some("Error".to_string()),
                test_results: vec![],
            },
        ];

        let report = CompatibilityReport::from_results(results);

        assert_eq!(report.full_compatibility_percentage(), 50.0);
    }
}
