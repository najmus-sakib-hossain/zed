//! Dependency Extras Resolution
//!
//! Implements PEP 508 extras handling for dependency resolution.
//! Extras allow packages to declare optional dependencies that are only
//! installed when explicitly requested (e.g., `requests[security]`).
//!
//! # Examples
//! ```ignore
//! use dx_py_package_manager::resolver::extras::{ExtrasResolver, ExtrasDependency};
//!
//! let resolver = ExtrasResolver::new();
//! let deps = resolver.resolve_extras("requests", &["security", "socks"], all_deps);
//! ```

use std::collections::{HashMap, HashSet};

use crate::registry::DependencySpec;
use crate::{Error, Result};

/// Represents a package with its requested extras
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageWithExtras {
    /// Package name (normalized)
    pub name: String,
    /// Requested extras
    pub extras: HashSet<String>,
}

impl std::hash::Hash for PackageWithExtras {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        // Hash extras in sorted order for consistency
        let mut extras: Vec<_> = self.extras.iter().collect();
        extras.sort();
        for extra in extras {
            extra.hash(state);
        }
    }
}

impl PackageWithExtras {
    /// Create a new package with extras
    pub fn new(name: &str, extras: impl IntoIterator<Item = impl AsRef<str>>) -> Self {
        Self {
            name: normalize_package_name(name),
            extras: extras.into_iter().map(|e| e.as_ref().to_lowercase()).collect(),
        }
    }

    /// Create a package without extras
    pub fn without_extras(name: &str) -> Self {
        Self {
            name: normalize_package_name(name),
            extras: HashSet::new(),
        }
    }

    /// Check if this package has any extras
    pub fn has_extras(&self) -> bool {
        !self.extras.is_empty()
    }

    /// Add an extra to this package
    pub fn add_extra(&mut self, extra: &str) {
        self.extras.insert(extra.to_lowercase());
    }

    /// Merge extras from another package (same name)
    pub fn merge_extras(&mut self, other: &PackageWithExtras) {
        if self.name == other.name {
            self.extras.extend(other.extras.iter().cloned());
        }
    }
}

impl std::fmt::Display for PackageWithExtras {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if !self.extras.is_empty() {
            let mut extras: Vec<_> = self.extras.iter().cloned().collect();
            extras.sort();
            write!(f, "[{}]", extras.join(","))?;
        }
        Ok(())
    }
}

/// Extras resolver for handling optional dependencies
#[derive(Debug, Default)]
pub struct ExtrasResolver {
    /// Cache of resolved extras per package
    extras_cache: HashMap<String, HashSet<String>>,
}

impl ExtrasResolver {
    /// Create a new extras resolver
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse extras from a dependency string (e.g., "requests[security,socks]")
    pub fn parse_extras(spec: &str) -> Result<(String, Vec<String>)> {
        let spec = spec.trim();

        if let Some(bracket_start) = spec.find('[') {
            if let Some(bracket_end) = spec.find(']') {
                if bracket_end > bracket_start {
                    let name = spec[..bracket_start].trim();
                    let extras_str = &spec[bracket_start + 1..bracket_end];
                    let extras: Vec<String> = extras_str
                        .split(',')
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    return Ok((normalize_package_name(name), extras));
                }
            }
        }

        Ok((normalize_package_name(spec), Vec::new()))
    }

    /// Filter dependencies based on active extras
    ///
    /// Given a list of all dependencies for a package and the active extras,
    /// returns only the dependencies that should be installed.
    pub fn filter_dependencies_by_extras(
        &self,
        all_deps: &[DependencySpec],
        active_extras: &HashSet<String>,
        marker_env: &dx_py_compat::markers::MarkerEnvironment,
    ) -> Vec<DependencySpec> {
        all_deps
            .iter()
            .filter(|dep| self.should_include_dependency(dep, active_extras, marker_env))
            .cloned()
            .collect()
    }

    /// Check if a dependency should be included based on its markers and active extras
    fn should_include_dependency(
        &self,
        dep: &DependencySpec,
        active_extras: &HashSet<String>,
        marker_env: &dx_py_compat::markers::MarkerEnvironment,
    ) -> bool {
        if let Some(ref markers) = dep.markers {
            // Check if this is an extra-conditional dependency
            if let Some(required_extra) = self.extract_extra_from_marker(markers) {
                // Only include if the required extra is active
                if !active_extras.contains(&required_extra.to_lowercase()) {
                    return false;
                }
            }

            // Evaluate the full marker expression
            let extras_vec: Vec<String> = active_extras.iter().cloned().collect();
            dx_py_compat::markers::MarkerEvaluator::evaluate(markers, marker_env, &extras_vec)
        } else {
            // No markers means always include
            true
        }
    }

    /// Extract the extra name from a marker expression if it's an extra-conditional dependency
    ///
    /// Handles patterns like:
    /// - `extra == 'security'`
    /// - `extra == "security"`
    /// - `(extra == 'security')`
    fn extract_extra_from_marker(&self, marker: &str) -> Option<String> {
        let marker = marker.trim();

        // Simple pattern matching for common cases
        // Pattern: extra == 'name' or extra == "name"
        if let Some(idx) = marker.find("extra") {
            let after_extra = marker[idx + 5..].trim_start();
            if let Some(after_eq) = after_extra.strip_prefix("==") {
                let after_eq = after_eq.trim_start();
                // Extract quoted string
                if let Some(quote_char) = after_eq.chars().next() {
                    if quote_char == '\'' || quote_char == '"' {
                        if let Some(end_idx) = after_eq[1..].find(quote_char) {
                            return Some(after_eq[1..end_idx + 1].to_lowercase());
                        }
                    }
                }
            }
        }

        None
    }

    /// Resolve all extras for a package, including transitive extras
    ///
    /// Some packages may have extras that depend on other extras.
    /// This method resolves the full set of extras needed.
    pub fn resolve_transitive_extras(
        &mut self,
        package: &str,
        requested_extras: &[String],
        all_deps: &[DependencySpec],
    ) -> HashSet<String> {
        let package = normalize_package_name(package);
        let mut resolved = HashSet::new();
        let mut to_process: Vec<String> = requested_extras.to_vec();

        while let Some(extra) = to_process.pop() {
            if resolved.contains(&extra) {
                continue;
            }
            resolved.insert(extra.clone());

            // Check if this extra has dependencies that require other extras
            for dep in all_deps {
                if let Some(ref markers) = dep.markers {
                    if let Some(required_extra) = self.extract_extra_from_marker(markers) {
                        if required_extra == extra {
                            // This dependency is activated by the current extra
                            // Check if it requires additional extras
                            for dep_extra in &dep.extras {
                                if !resolved.contains(dep_extra) {
                                    to_process.push(dep_extra.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Cache the result
        self.extras_cache.insert(package, resolved.clone());

        resolved
    }

    /// Get available extras for a package from its dependencies
    ///
    /// Scans the dependency list to find all extras that are referenced
    /// in marker expressions.
    pub fn get_available_extras(deps: &[DependencySpec]) -> HashSet<String> {
        let mut extras = HashSet::new();

        for dep in deps {
            if let Some(ref markers) = dep.markers {
                // Look for extra == 'name' patterns
                let marker = markers.to_lowercase();
                let mut search_from = 0;

                while let Some(idx) = marker[search_from..].find("extra") {
                    let abs_idx = search_from + idx;
                    let after_extra = marker[abs_idx + 5..].trim_start();

                    if let Some(after_eq) = after_extra.strip_prefix("==") {
                        let after_eq = after_eq.trim_start();
                        // Extract quoted string
                        if let Some(quote_char) = after_eq.chars().next() {
                            if quote_char == '\'' || quote_char == '"' {
                                if let Some(end_idx) = after_eq[1..].find(quote_char) {
                                    extras.insert(after_eq[1..end_idx + 1].to_string());
                                }
                            }
                        }
                    }

                    search_from = abs_idx + 5;
                }
            }
        }

        extras
    }

    /// Validate that requested extras are available for a package
    pub fn validate_extras(requested: &[String], available: &HashSet<String>) -> Result<()> {
        let mut invalid = Vec::new();

        for extra in requested {
            if !available.contains(&extra.to_lowercase()) {
                invalid.push(extra.clone());
            }
        }

        if !invalid.is_empty() {
            return Err(Error::InvalidExtra(format!(
                "Unknown extras: {}. Available: {}",
                invalid.join(", "),
                available.iter().cloned().collect::<Vec<_>>().join(", ")
            )));
        }

        Ok(())
    }
}

/// Normalize a package name according to PEP 503
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '.'], "_")
}

/// Parsed dependency result type
pub type ParsedDependency = (String, Vec<String>, Option<String>, Option<String>);

/// Parse a dependency specification with extras
///
/// Handles formats like:
/// - `requests`
/// - `requests[security]`
/// - `requests[security,socks]>=2.0`
/// - `requests[security]>=2.0; python_version >= '3.8'`
pub fn parse_dependency_with_extras(spec: &str) -> Result<ParsedDependency> {
    let dep_spec = DependencySpec::parse(spec)?;
    Ok((dep_spec.name, dep_spec.extras, dep_spec.version_constraint, dep_spec.markers))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extras_simple() {
        let (name, extras) = ExtrasResolver::parse_extras("requests[security]").unwrap();
        assert_eq!(name, "requests");
        assert_eq!(extras, vec!["security"]);
    }

    #[test]
    fn test_parse_extras_multiple() {
        let (name, extras) = ExtrasResolver::parse_extras("requests[security, socks]").unwrap();
        assert_eq!(name, "requests");
        assert_eq!(extras, vec!["security", "socks"]);
    }

    #[test]
    fn test_parse_extras_none() {
        let (name, extras) = ExtrasResolver::parse_extras("requests").unwrap();
        assert_eq!(name, "requests");
        assert!(extras.is_empty());
    }

    #[test]
    fn test_parse_extras_normalized() {
        let (name, extras) = ExtrasResolver::parse_extras("My-Package[DEV]").unwrap();
        assert_eq!(name, "my_package");
        assert_eq!(extras, vec!["dev"]);
    }

    #[test]
    fn test_package_with_extras_display() {
        let pkg = PackageWithExtras::new("requests", ["security", "socks"]);
        let display = pkg.to_string();
        assert!(display.starts_with("requests["));
        assert!(display.contains("security"));
        assert!(display.contains("socks"));
    }

    #[test]
    fn test_extract_extra_from_marker() {
        let resolver = ExtrasResolver::new();

        assert_eq!(
            resolver.extract_extra_from_marker("extra == 'security'"),
            Some("security".to_string())
        );

        assert_eq!(resolver.extract_extra_from_marker("extra == \"dev\""), Some("dev".to_string()));

        assert_eq!(resolver.extract_extra_from_marker("python_version >= '3.8'"), None);
    }

    #[test]
    fn test_get_available_extras() {
        let deps = vec![
            DependencySpec {
                name: "pyopenssl".to_string(),
                version_constraint: Some(">=0.14".to_string()),
                extras: vec![],
                markers: Some("extra == 'security'".to_string()),
                url: None,
                path: None,
            },
            DependencySpec {
                name: "pysocks".to_string(),
                version_constraint: Some(">=1.5.6".to_string()),
                extras: vec![],
                markers: Some("extra == 'socks'".to_string()),
                url: None,
                path: None,
            },
            DependencySpec {
                name: "urllib3".to_string(),
                version_constraint: Some(">=1.21.1".to_string()),
                extras: vec![],
                markers: None,
                url: None,
                path: None,
            },
        ];

        let available = ExtrasResolver::get_available_extras(&deps);
        assert!(available.contains("security"));
        assert!(available.contains("socks"));
        assert_eq!(available.len(), 2);
    }

    #[test]
    fn test_filter_dependencies_by_extras() {
        let resolver = ExtrasResolver::new();
        let marker_env = dx_py_compat::markers::MarkerEnvironment::current();

        let deps = vec![
            DependencySpec {
                name: "urllib3".to_string(),
                version_constraint: Some(">=1.21.1".to_string()),
                extras: vec![],
                markers: None,
                url: None,
                path: None,
            },
            DependencySpec {
                name: "pyopenssl".to_string(),
                version_constraint: Some(">=0.14".to_string()),
                extras: vec![],
                markers: Some("extra == 'security'".to_string()),
                url: None,
                path: None,
            },
        ];

        // Without security extra
        let no_extras: HashSet<String> = HashSet::new();
        let filtered = resolver.filter_dependencies_by_extras(&deps, &no_extras, &marker_env);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "urllib3");

        // With security extra
        let with_security: HashSet<String> = ["security".to_string()].into_iter().collect();
        let filtered = resolver.filter_dependencies_by_extras(&deps, &with_security, &marker_env);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_validate_extras() {
        let available: HashSet<String> =
            ["security", "socks", "dev"].iter().map(|s| s.to_string()).collect();

        // Valid extras
        assert!(ExtrasResolver::validate_extras(&["security".to_string()], &available).is_ok());
        assert!(ExtrasResolver::validate_extras(
            &["security".to_string(), "socks".to_string()],
            &available
        )
        .is_ok());

        // Invalid extras
        assert!(ExtrasResolver::validate_extras(&["unknown".to_string()], &available).is_err());
    }

    #[test]
    fn test_normalize_package_name() {
        assert_eq!(normalize_package_name("My-Package"), "my_package");
        assert_eq!(normalize_package_name("my.package"), "my_package");
        assert_eq!(normalize_package_name("MY_PACKAGE"), "my_package");
    }
}
