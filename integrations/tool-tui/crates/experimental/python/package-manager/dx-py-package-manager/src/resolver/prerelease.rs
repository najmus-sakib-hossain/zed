//! Pre-release Version Handling
//!
//! Implements PEP 440 pre-release version handling for dependency resolution.
//! Pre-release versions (alpha, beta, rc, dev) are excluded by default unless:
//! - Explicitly requested via version constraint (e.g., `>=1.0.0a1`)
//! - The `--pre` flag is used
//! - No stable version satisfies the constraint
//!
//! # Examples
//! ```ignore
//! use dx_py_package_manager::resolver::prerelease::{PreReleasePolicy, PreReleaseFilter};
//!
//! let filter = PreReleaseFilter::new(PreReleasePolicy::ExplicitOnly);
//! let versions = filter.filter_versions(&all_versions, &constraint);
//! ```

use std::collections::HashSet;

use dx_py_core::pep440::Pep440Version;
use dx_py_core::version::PackedVersion;

/// Policy for handling pre-release versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreReleasePolicy {
    /// Never include pre-releases (strictest)
    Never,
    /// Include pre-releases only if explicitly requested in constraint
    #[default]
    ExplicitOnly,
    /// Include pre-releases if no stable version satisfies constraint
    IfNeeded,
    /// Always include pre-releases
    Always,
}

impl PreReleasePolicy {
    /// Parse from string (for CLI flags)
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "never" | "no" | "false" => Some(PreReleasePolicy::Never),
            "explicit" | "explicit-only" => Some(PreReleasePolicy::ExplicitOnly),
            "if-needed" | "ifneeded" | "needed" => Some(PreReleasePolicy::IfNeeded),
            "always" | "yes" | "true" | "pre" => Some(PreReleasePolicy::Always),
            _ => None,
        }
    }
}

/// Pre-release version filter
#[derive(Debug, Clone)]
pub struct PreReleaseFilter {
    /// Policy for handling pre-releases
    policy: PreReleasePolicy,
    /// Packages that explicitly allow pre-releases
    explicit_prerelease_packages: HashSet<String>,
}

impl Default for PreReleaseFilter {
    fn default() -> Self {
        Self::new(PreReleasePolicy::default())
    }
}

impl PreReleaseFilter {
    /// Create a new pre-release filter with the given policy
    pub fn new(policy: PreReleasePolicy) -> Self {
        Self {
            policy,
            explicit_prerelease_packages: HashSet::new(),
        }
    }

    /// Allow pre-releases for a specific package
    pub fn allow_prerelease_for(&mut self, package: &str) {
        self.explicit_prerelease_packages.insert(package.to_lowercase());
    }

    /// Check if pre-releases are allowed for a package
    pub fn allows_prerelease(&self, package: &str) -> bool {
        match self.policy {
            PreReleasePolicy::Never => false,
            PreReleasePolicy::Always => true,
            PreReleasePolicy::ExplicitOnly | PreReleasePolicy::IfNeeded => {
                self.explicit_prerelease_packages.contains(&package.to_lowercase())
            }
        }
    }

    /// Check if a version constraint explicitly requests pre-releases
    ///
    /// A constraint explicitly requests pre-releases if it includes a pre-release
    /// version specifier (e.g., `>=1.0.0a1`, `==2.0.0b2`).
    pub fn constraint_requests_prerelease(constraint: &str) -> bool {
        // Check for pre-release indicators in the constraint
        let lower = constraint.to_lowercase();

        // Look for pre-release suffixes
        let prerelease_patterns = [
            "a", "alpha", "b", "beta", "c", "rc", "pre", "preview", "dev",
        ];

        for pattern in prerelease_patterns {
            // Check for patterns like "1.0.0a1", "1.0.0.dev1"
            if lower.contains(&format!(".{}", pattern))
                || lower.chars().any(|c| c.is_ascii_digit())
                    && lower.contains(pattern)
                    && !lower.contains(&format!("_{}", pattern))
            {
                // More precise check: look for version-like patterns
                for part in lower.split([',', ' ', '>', '<', '=', '!', '~']) {
                    let part = part.trim();
                    if !part.is_empty() && Self::version_has_prerelease(part) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Check if a version string contains a pre-release suffix
    fn version_has_prerelease(version: &str) -> bool {
        if let Ok(parsed) = Pep440Version::parse(version) {
            parsed.is_prerelease()
        } else {
            // Fallback: simple pattern matching
            let lower = version.to_lowercase();
            lower.contains("dev") 
                || lower.contains("alpha") 
                || lower.contains("beta")
                || lower.contains("rc")
                || lower.contains("pre")
                // Check for short forms like "1.0a1"
                || lower.chars().enumerate().any(|(i, c)| {
                    c.is_ascii_digit() && lower[i+1..].starts_with(['a', 'b', 'c'])
                        && lower[i+2..].chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                })
        }
    }

    /// Filter versions based on pre-release policy
    ///
    /// Returns versions that should be considered for resolution.
    pub fn filter_versions(
        &self,
        package: &str,
        versions: &[(PackedVersion, String)],
        constraint: Option<&str>,
    ) -> Vec<(PackedVersion, String)> {
        match self.policy {
            PreReleasePolicy::Always => versions.to_vec(),
            PreReleasePolicy::Never => versions
                .iter()
                .filter(|(_, v)| !Self::is_prerelease_version(v))
                .cloned()
                .collect(),
            PreReleasePolicy::ExplicitOnly => {
                let allow_pre = self.explicit_prerelease_packages.contains(&package.to_lowercase())
                    || constraint.map(Self::constraint_requests_prerelease).unwrap_or(false);

                if allow_pre {
                    versions.to_vec()
                } else {
                    versions
                        .iter()
                        .filter(|(_, v)| !Self::is_prerelease_version(v))
                        .cloned()
                        .collect()
                }
            }
            PreReleasePolicy::IfNeeded => {
                let allow_pre = self.explicit_prerelease_packages.contains(&package.to_lowercase())
                    || constraint.map(Self::constraint_requests_prerelease).unwrap_or(false);

                if allow_pre {
                    return versions.to_vec();
                }

                // First try without pre-releases
                let stable: Vec<_> = versions
                    .iter()
                    .filter(|(_, v)| !Self::is_prerelease_version(v))
                    .cloned()
                    .collect();

                // If no stable versions, include pre-releases
                if stable.is_empty() {
                    versions.to_vec()
                } else {
                    stable
                }
            }
        }
    }

    /// Check if a version string represents a pre-release
    pub fn is_prerelease_version(version: &str) -> bool {
        if let Ok(parsed) = Pep440Version::parse(version) {
            parsed.is_prerelease()
        } else {
            // Fallback pattern matching
            let lower = version.to_lowercase();
            lower.contains("dev")
                || lower.contains("alpha")
                || lower.contains("beta")
                || lower.contains("rc")
                || lower.contains("pre")
                || lower.contains("preview")
                // Short forms
                || Self::has_short_prerelease_suffix(&lower)
        }
    }

    /// Check for short pre-release suffixes like "1.0a1", "2.0b2", "3.0rc1"
    fn has_short_prerelease_suffix(version: &str) -> bool {
        let chars: Vec<char> = version.chars().collect();
        for i in 0..chars.len().saturating_sub(2) {
            if chars[i].is_ascii_digit() {
                let suffix = &version[i + 1..];
                if (suffix.starts_with('a') || suffix.starts_with('b') || suffix.starts_with('c'))
                    && suffix.len() > 1
                    && suffix.chars().nth(1).map(|c| c.is_ascii_digit()).unwrap_or(false)
                {
                    return true;
                }
            }
        }
        false
    }

    /// Sort versions with pre-releases ordered correctly
    ///
    /// Pre-releases come before their corresponding release:
    /// 1.0.0a1 < 1.0.0b1 < 1.0.0rc1 < 1.0.0 < 1.0.0.post1
    pub fn sort_versions(versions: &mut [(PackedVersion, String)]) {
        versions.sort_by(|(_, a), (_, b)| {
            match (Pep440Version::parse(a), Pep440Version::parse(b)) {
                (Ok(va), Ok(vb)) => va.cmp(&vb),
                (Ok(_), Err(_)) => std::cmp::Ordering::Less,
                (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
                (Err(_), Err(_)) => a.cmp(b),
            }
        });
    }

    /// Get the latest version, respecting pre-release policy
    pub fn latest_version(
        &self,
        package: &str,
        versions: &[(PackedVersion, String)],
        constraint: Option<&str>,
    ) -> Option<(PackedVersion, String)> {
        let filtered = self.filter_versions(package, versions, constraint);

        // Sort and get the highest
        let mut sorted = filtered;
        Self::sort_versions(&mut sorted);
        sorted.into_iter().last()
    }
}

/// Parse a version string and check if it's a pre-release
pub fn is_prerelease(version: &str) -> bool {
    PreReleaseFilter::is_prerelease_version(version)
}

/// Compare two versions, handling pre-releases correctly
pub fn compare_versions_pep440(a: &str, b: &str) -> std::cmp::Ordering {
    match (Pep440Version::parse(a), Pep440Version::parse(b)) {
        (Ok(va), Ok(vb)) => va.cmp(&vb),
        (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
        (Err(_), Ok(_)) => std::cmp::Ordering::Less,
        (Err(_), Err(_)) => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prerelease_detection() {
        // Pre-release versions
        assert!(PreReleaseFilter::is_prerelease_version("1.0.0a1"));
        assert!(PreReleaseFilter::is_prerelease_version("1.0.0b2"));
        assert!(PreReleaseFilter::is_prerelease_version("1.0.0rc1"));
        assert!(PreReleaseFilter::is_prerelease_version("1.0.0.dev1"));
        assert!(PreReleaseFilter::is_prerelease_version("1.0.0alpha1"));
        assert!(PreReleaseFilter::is_prerelease_version("1.0.0beta2"));

        // Stable versions
        assert!(!PreReleaseFilter::is_prerelease_version("1.0.0"));
        assert!(!PreReleaseFilter::is_prerelease_version("2.3.4"));
        assert!(!PreReleaseFilter::is_prerelease_version("1.0.0.post1"));
    }

    #[test]
    fn test_constraint_requests_prerelease() {
        // Constraints that request pre-releases
        assert!(PreReleaseFilter::constraint_requests_prerelease(">=1.0.0a1"));
        assert!(PreReleaseFilter::constraint_requests_prerelease("==2.0.0b2"));
        assert!(PreReleaseFilter::constraint_requests_prerelease(">=1.0.0.dev1"));

        // Constraints that don't request pre-releases
        assert!(!PreReleaseFilter::constraint_requests_prerelease(">=1.0.0"));
        assert!(!PreReleaseFilter::constraint_requests_prerelease("==2.0.0"));
        assert!(!PreReleaseFilter::constraint_requests_prerelease(">=1.0,<2.0"));
    }

    #[test]
    fn test_filter_versions_never() {
        let filter = PreReleaseFilter::new(PreReleasePolicy::Never);
        let versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0".to_string()),
            (PackedVersion::new(1, 1, 0), "1.1.0b1".to_string()),
            (PackedVersion::new(1, 1, 0), "1.1.0".to_string()),
        ];

        let filtered = filter.filter_versions("pkg", &versions, None);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|(_, v)| !PreReleaseFilter::is_prerelease_version(v)));
    }

    #[test]
    fn test_filter_versions_always() {
        let filter = PreReleaseFilter::new(PreReleasePolicy::Always);
        let versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0".to_string()),
        ];

        let filtered = filter.filter_versions("pkg", &versions, None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_versions_explicit_only() {
        let filter = PreReleaseFilter::new(PreReleasePolicy::ExplicitOnly);
        let versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0".to_string()),
        ];

        // Without explicit constraint
        let filtered = filter.filter_versions("pkg", &versions, Some(">=1.0.0"));
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1, "1.0.0");

        // With explicit pre-release constraint
        let filtered = filter.filter_versions("pkg", &versions, Some(">=1.0.0a1"));
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_filter_versions_if_needed() {
        let filter = PreReleaseFilter::new(PreReleasePolicy::IfNeeded);

        // With stable versions available
        let versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0".to_string()),
        ];
        let filtered = filter.filter_versions("pkg", &versions, None);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1, "1.0.0");

        // Without stable versions
        let versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0b1".to_string()),
        ];
        let filtered = filter.filter_versions("pkg", &versions, None);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_explicit_package_allowlist() {
        let mut filter = PreReleaseFilter::new(PreReleasePolicy::ExplicitOnly);
        filter.allow_prerelease_for("mypackage");

        let versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0".to_string()),
        ];

        // Allowed package gets pre-releases
        let filtered = filter.filter_versions("mypackage", &versions, None);
        assert_eq!(filtered.len(), 2);

        // Other packages don't
        let filtered = filter.filter_versions("otherpackage", &versions, None);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_version_sorting() {
        let mut versions = vec![
            (PackedVersion::new(1, 0, 0), "1.0.0".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0a1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0rc1".to_string()),
            (PackedVersion::new(1, 0, 0), "1.0.0b1".to_string()),
        ];

        PreReleaseFilter::sort_versions(&mut versions);

        assert_eq!(versions[0].1, "1.0.0a1");
        assert_eq!(versions[1].1, "1.0.0b1");
        assert_eq!(versions[2].1, "1.0.0rc1");
        assert_eq!(versions[3].1, "1.0.0");
    }

    #[test]
    fn test_policy_from_str() {
        assert_eq!(PreReleasePolicy::from_str("never"), Some(PreReleasePolicy::Never));
        assert_eq!(PreReleasePolicy::from_str("always"), Some(PreReleasePolicy::Always));
        assert_eq!(PreReleasePolicy::from_str("pre"), Some(PreReleasePolicy::Always));
        assert_eq!(PreReleasePolicy::from_str("explicit"), Some(PreReleasePolicy::ExplicitOnly));
        assert_eq!(PreReleasePolicy::from_str("if-needed"), Some(PreReleasePolicy::IfNeeded));
        assert_eq!(PreReleasePolicy::from_str("invalid"), None);
    }

    #[test]
    fn test_compare_versions_pep440() {
        assert_eq!(compare_versions_pep440("1.0.0a1", "1.0.0"), std::cmp::Ordering::Less);
        assert_eq!(compare_versions_pep440("1.0.0", "1.0.0.post1"), std::cmp::Ordering::Less);
        assert_eq!(compare_versions_pep440("1.0.0a1", "1.0.0b1"), std::cmp::Ordering::Less);
        assert_eq!(compare_versions_pep440("2.0.0", "1.0.0"), std::cmp::Ordering::Greater);
    }
}
