//! Semantic versioning support for package management
//!
//! This module provides types and functions for working with semantic versions
//! and version constraints as used in npm package management.
//!
//! # Examples
//!
//! ```
//! use dx_pkg_core::version::{Version, VersionConstraint};
//!
//! // Parse a version
//! let v = Version::parse("1.2.3").unwrap();
//! assert_eq!(v.major, 1);
//!
//! // Parse a constraint and check if a version matches
//! let constraint = VersionConstraint::parse("^1.0.0").unwrap();
//! assert!(constraint.matches(&Version::new(1, 5, 0)));
//! assert!(!constraint.matches(&Version::new(2, 0, 0)));
//! ```

use crate::error::{Error, Result};
use std::fmt;

/// A semantic version following the semver specification.
///
/// Semantic versions consist of three numeric components:
/// - `major`: Incremented for incompatible API changes
/// - `minor`: Incremented for backwards-compatible functionality additions
/// - `patch`: Incremented for backwards-compatible bug fixes
///
/// # Examples
///
/// ```
/// use dx_pkg_core::version::Version;
///
/// let v = Version::new(1, 2, 3);
/// assert_eq!(v.to_string(), "1.2.3");
///
/// let parsed = Version::parse("v4.17.21").unwrap();
/// assert_eq!(parsed.major, 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// Major version number (breaking changes)
    pub major: u32,
    /// Minor version number (new features, backwards compatible)
    pub minor: u32,
    /// Patch version number (bug fixes, backwards compatible)
    pub patch: u32,
}

impl Version {
    /// Create a new version with the given major, minor, and patch numbers.
    ///
    /// # Examples
    ///
    /// ```
    /// use dx_pkg_core::version::Version;
    ///
    /// let v = Version::new(1, 2, 3);
    /// assert_eq!(v.major, 1);
    /// assert_eq!(v.minor, 2);
    /// assert_eq!(v.patch, 3);
    /// ```
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse a version string.
    ///
    /// Accepts versions in the following formats:
    /// - `1.2.3` - Standard semver
    /// - `v1.2.3` - With 'v' prefix
    /// - `1.2.3-beta.1` - With prerelease (prerelease is stripped)
    /// - `1.2` - Two-part version (patch defaults to 0)
    ///
    /// # Errors
    ///
    /// Returns an error if the version string is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use dx_pkg_core::version::Version;
    ///
    /// let v = Version::parse("1.2.3").unwrap();
    /// assert_eq!(v, Version::new(1, 2, 3));
    ///
    /// let v = Version::parse("v4.17.21").unwrap();
    /// assert_eq!(v.major, 4);
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim_start_matches('v');
        // Handle prerelease versions like "1.0.0-beta.1"
        let s = s.split('-').next().unwrap_or(s);
        let parts: Vec<&str> = s.split('.').collect();

        if parts.len() < 2 {
            return Err(Error::invalid_version(s));
        }

        Ok(Self {
            major: parts[0].parse().map_err(|_| Error::invalid_version(s))?,
            minor: parts[1].parse().map_err(|_| Error::invalid_version(s))?,
            patch: parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
        })
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A version constraint that can match multiple versions.
///
/// Version constraints are used in package.json to specify which versions
/// of a dependency are acceptable. This type supports all npm constraint syntaxes.
///
/// # Constraint Types
///
/// - `Exact`: Matches exactly one version (e.g., `1.2.3`)
/// - `Caret`: Allows changes that do not modify the left-most non-zero digit (e.g., `^1.2.3`)
/// - `Tilde`: Allows patch-level changes (e.g., `~1.2.3`)
/// - `Range`: Matches versions within a range (e.g., `>=1.0.0 <2.0.0`)
/// - `Or`: Matches if any sub-constraint matches (e.g., `^1.0.0 || ^2.0.0`)
/// - `Any`: Matches any version (e.g., `*` or `latest`)
///
/// # Examples
///
/// ```
/// use dx_pkg_core::version::{Version, VersionConstraint};
///
/// // Caret constraint: ^1.2.3 matches >=1.2.3 <2.0.0
/// let caret = VersionConstraint::parse("^1.2.3").unwrap();
/// assert!(caret.matches(&Version::new(1, 5, 0)));
/// assert!(!caret.matches(&Version::new(2, 0, 0)));
///
/// // Tilde constraint: ~1.2.3 matches >=1.2.3 <1.3.0
/// let tilde = VersionConstraint::parse("~1.2.3").unwrap();
/// assert!(tilde.matches(&Version::new(1, 2, 9)));
/// assert!(!tilde.matches(&Version::new(1, 3, 0)));
///
/// // OR constraint
/// let or = VersionConstraint::parse("^1.0.0 || ^2.0.0").unwrap();
/// assert!(or.matches(&Version::new(1, 5, 0)));
/// assert!(or.matches(&Version::new(2, 5, 0)));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum VersionConstraint {
    /// Matches exactly one version
    Exact(Version),
    /// Matches versions within a range (inclusive)
    Range { min: Version, max: Version },
    /// Caret constraint: ^1.2.3 = >=1.2.3 <2.0.0
    Caret(Version),
    /// Tilde constraint: ~1.2.3 = >=1.2.3 <1.3.0
    Tilde(Version),
    /// OR constraint: matches if any sub-constraint matches
    Or(Vec<VersionConstraint>),
    /// Matches any version (* or latest)
    Any,
}

impl VersionConstraint {
    /// Parse a version constraint string.
    ///
    /// Supports all npm version constraint syntaxes:
    /// - Exact: `1.2.3`, `=1.2.3`
    /// - Caret: `^1.2.3`
    /// - Tilde: `~1.2.3`
    /// - Range: `>=1.0.0`, `<2.0.0`, `1.0.0 - 2.0.0`, `>=1.0.0 <2.0.0`
    /// - OR: `^1.0.0 || ^2.0.0`
    /// - Any: `*`, `latest`, empty string
    ///
    /// # Errors
    ///
    /// Returns an error if the constraint string is malformed.
    ///
    /// # Examples
    ///
    /// ```
    /// use dx_pkg_core::version::VersionConstraint;
    ///
    /// let c = VersionConstraint::parse("^1.0.0").unwrap();
    /// let any = VersionConstraint::parse("*").unwrap();
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        // Handle OR syntax (||)
        if s.contains("||") {
            let parts: Vec<&str> = s.split("||").collect();
            let constraints: Result<Vec<VersionConstraint>> =
                parts.iter().map(|p| VersionConstraint::parse(p.trim())).collect();
            return Ok(VersionConstraint::Or(constraints?));
        }

        // Handle special cases
        if s == "*" || s == "latest" || s.is_empty() {
            return Ok(VersionConstraint::Any);
        }

        // Handle caret (^)
        if let Some(rest) = s.strip_prefix('^') {
            let version = Version::parse(rest)?;
            return Ok(VersionConstraint::Caret(version));
        }

        // Handle tilde (~)
        if let Some(rest) = s.strip_prefix('~') {
            let version = Version::parse(rest)?;
            return Ok(VersionConstraint::Tilde(version));
        }

        // Handle hyphen range (x.y.z - a.b.c)
        if s.contains(" - ") {
            let parts: Vec<&str> = s.split(" - ").collect();
            if parts.len() == 2 {
                let min = Version::parse(parts[0].trim())?;
                let max = Version::parse(parts[1].trim())?;
                return Ok(VersionConstraint::Range { min, max });
            }
        }

        // Handle compound range constraints like ">= 2.1.2 < 3" or ">=1.0.0 <2.0.0"
        // Split on spaces and look for multiple comparison operators
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() >= 2 {
            let mut min = Version::new(0, 0, 0);
            let mut max = Version::new(u32::MAX, u32::MAX, u32::MAX);
            let mut has_min = false;
            let mut has_max = false;

            let mut i = 0;
            while i < parts.len() {
                let part = parts[i];

                if let Some(rest) = part.strip_prefix(">=") {
                    let version_str = if rest.is_empty() && i + 1 < parts.len() {
                        i += 1;
                        parts[i]
                    } else {
                        rest
                    };
                    min = Version::parse(version_str)?;
                    has_min = true;
                } else if let Some(rest) = part.strip_prefix("<=") {
                    let version_str = if rest.is_empty() && i + 1 < parts.len() {
                        i += 1;
                        parts[i]
                    } else {
                        rest
                    };
                    max = Version::parse(version_str)?;
                    has_max = true;
                } else if let Some(rest) = part.strip_prefix('>') {
                    let version_str = if rest.is_empty() && i + 1 < parts.len() {
                        i += 1;
                        parts[i]
                    } else {
                        rest
                    };
                    let v = Version::parse(version_str)?;
                    min = Version::new(v.major, v.minor, v.patch + 1);
                    has_min = true;
                } else if let Some(rest) = part.strip_prefix('<') {
                    let version_str = if rest.is_empty() && i + 1 < parts.len() {
                        i += 1;
                        parts[i]
                    } else {
                        rest
                    };
                    let v = Version::parse(version_str)?;
                    max = if v.patch > 0 {
                        Version::new(v.major, v.minor, v.patch - 1)
                    } else if v.minor > 0 {
                        Version::new(v.major, v.minor - 1, u32::MAX)
                    } else if v.major > 0 {
                        Version::new(v.major - 1, u32::MAX, u32::MAX)
                    } else {
                        return Err(Error::invalid_version(s));
                    };
                    has_max = true;
                }
                i += 1;
            }

            if has_min || has_max {
                return Ok(VersionConstraint::Range { min, max });
            }
        }

        // Handle single comparison operators
        if let Some(rest) = s.strip_prefix(">=") {
            let version = Version::parse(rest.trim())?;
            return Ok(VersionConstraint::Range {
                min: version,
                max: Version::new(u32::MAX, u32::MAX, u32::MAX),
            });
        }

        if let Some(rest) = s.strip_prefix("<=") {
            let version = Version::parse(rest.trim())?;
            return Ok(VersionConstraint::Range {
                min: Version::new(0, 0, 0),
                max: version,
            });
        }

        if let Some(rest) = s.strip_prefix('>') {
            let version = Version::parse(rest.trim())?;
            return Ok(VersionConstraint::Range {
                min: Version::new(version.major, version.minor, version.patch + 1),
                max: Version::new(u32::MAX, u32::MAX, u32::MAX),
            });
        }

        if let Some(rest) = s.strip_prefix('<') {
            let version = Version::parse(rest.trim())?;
            let max = if version.patch > 0 {
                Version::new(version.major, version.minor, version.patch - 1)
            } else if version.minor > 0 {
                Version::new(version.major, version.minor - 1, u32::MAX)
            } else if version.major > 0 {
                Version::new(version.major - 1, u32::MAX, u32::MAX)
            } else {
                return Err(Error::invalid_version(s));
            };
            return Ok(VersionConstraint::Range {
                min: Version::new(0, 0, 0),
                max,
            });
        }

        // Handle exact version (with optional = prefix)
        let version_str = s.strip_prefix('=').unwrap_or(s);
        let version = Version::parse(version_str)?;
        Ok(VersionConstraint::Exact(version))
    }

    /// Check if a version satisfies this constraint.
    ///
    /// # Examples
    ///
    /// ```
    /// use dx_pkg_core::version::{Version, VersionConstraint};
    ///
    /// let constraint = VersionConstraint::parse("^1.2.0").unwrap();
    /// assert!(constraint.matches(&Version::new(1, 5, 0)));
    /// assert!(!constraint.matches(&Version::new(2, 0, 0)));
    /// ```
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::Range { min, max } => version >= min && version <= max,
            VersionConstraint::Caret(v) => {
                // ^1.2.3 = >=1.2.3 <2.0.0 (for major > 0)
                // ^0.2.3 = >=0.2.3 <0.3.0 (for major = 0, minor > 0)
                // ^0.0.3 = >=0.0.3 <0.0.4 (for major = 0, minor = 0)
                if version < v {
                    return false;
                }
                if v.major > 0 {
                    version.major == v.major
                } else if v.minor > 0 {
                    version.major == 0 && version.minor == v.minor
                } else {
                    version.major == 0 && version.minor == 0 && version.patch == v.patch
                }
            }
            VersionConstraint::Tilde(v) => {
                // ~1.2.3 = >=1.2.3 <1.3.0
                version >= v && version.major == v.major && version.minor == v.minor
            }
            VersionConstraint::Or(constraints) => constraints.iter().any(|c| c.matches(version)),
            VersionConstraint::Any => true,
        }
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionConstraint::Exact(v) => write!(f, "{}", v),
            VersionConstraint::Range { min, max } => write!(f, ">={} <={}", min, max),
            VersionConstraint::Caret(v) => write!(f, "^{}", v),
            VersionConstraint::Tilde(v) => write!(f, "~{}", v),
            VersionConstraint::Or(constraints) => {
                let parts: Vec<String> = constraints.iter().map(|c| c.to_string()).collect();
                write!(f, "{}", parts.join(" || "))
            }
            VersionConstraint::Any => write!(f, "*"),
        }
    }
}

/// Encode a version to a u64 for efficient storage and comparison.
///
/// The encoding uses bit packing: `major << 40 | minor << 20 | patch`.
/// This allows versions to be compared using simple integer comparison.
///
/// # Examples
///
/// ```
/// use dx_pkg_core::version::{Version, encode_version, decode_version};
///
/// let v = Version::new(4, 17, 21);
/// let encoded = encode_version(&v);
/// let decoded = decode_version(encoded);
/// assert_eq!(v, decoded);
/// ```
pub fn encode_version(version: &Version) -> u64 {
    ((version.major as u64) << 40) | ((version.minor as u64) << 20) | (version.patch as u64)
}

/// Decode a version from a u64 encoded value.
///
/// This is the inverse of [`encode_version`].
///
/// # Examples
///
/// ```
/// use dx_pkg_core::version::{Version, encode_version, decode_version};
///
/// let v = Version::new(4, 17, 21);
/// let encoded = encode_version(&v);
/// let decoded = decode_version(encoded);
/// assert_eq!(v, decoded);
/// ```
pub fn decode_version(encoded: u64) -> Version {
    Version {
        major: ((encoded >> 40) & 0xFFFFFF) as u32,
        minor: ((encoded >> 20) & 0xFFFFF) as u32,
        patch: (encoded & 0xFFFFF) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_parse_version_with_v() {
        let v = Version::parse("v1.2.3").unwrap();
        assert_eq!(v, Version::new(1, 2, 3));
    }

    #[test]
    fn test_parse_version_with_prerelease() {
        let v = Version::parse("1.0.0-beta.1").unwrap();
        assert_eq!(v, Version::new(1, 0, 0));
    }

    #[test]
    fn test_encode_decode() {
        let v = Version::new(4, 17, 21);
        let encoded = encode_version(&v);
        let decoded = decode_version(encoded);
        assert_eq!(v, decoded);
    }

    #[test]
    fn test_version_ordering() {
        let v1 = Version::new(1, 0, 0);
        let v2 = Version::new(1, 2, 0);
        let v3 = Version::new(2, 0, 0);
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_constraint_exact() {
        let c = VersionConstraint::parse("1.2.3").unwrap();
        assert!(c.matches(&Version::new(1, 2, 3)));
        assert!(!c.matches(&Version::new(1, 2, 4)));
    }

    #[test]
    fn test_constraint_caret() {
        let c = VersionConstraint::parse("^1.2.3").unwrap();
        assert!(c.matches(&Version::new(1, 2, 3)));
        assert!(c.matches(&Version::new(1, 9, 0)));
        assert!(!c.matches(&Version::new(2, 0, 0)));
        assert!(!c.matches(&Version::new(1, 2, 2)));
    }

    #[test]
    fn test_constraint_tilde() {
        let c = VersionConstraint::parse("~1.2.3").unwrap();
        assert!(c.matches(&Version::new(1, 2, 3)));
        assert!(c.matches(&Version::new(1, 2, 9)));
        assert!(!c.matches(&Version::new(1, 3, 0)));
    }

    #[test]
    fn test_constraint_or() {
        let c = VersionConstraint::parse("^1.0.0 || ^2.0.0").unwrap();
        assert!(c.matches(&Version::new(1, 5, 0)));
        assert!(c.matches(&Version::new(2, 5, 0)));
        assert!(!c.matches(&Version::new(3, 0, 0)));
    }

    #[test]
    fn test_constraint_or_multiple() {
        let c = VersionConstraint::parse("1.0.0 || 2.0.0 || 3.0.0").unwrap();
        assert!(c.matches(&Version::new(1, 0, 0)));
        assert!(c.matches(&Version::new(2, 0, 0)));
        assert!(c.matches(&Version::new(3, 0, 0)));
        assert!(!c.matches(&Version::new(4, 0, 0)));
    }

    #[test]
    fn test_constraint_range() {
        let c = VersionConstraint::parse("1.0.0 - 2.0.0").unwrap();
        assert!(c.matches(&Version::new(1, 0, 0)));
        assert!(c.matches(&Version::new(1, 5, 0)));
        assert!(c.matches(&Version::new(2, 0, 0)));
        assert!(!c.matches(&Version::new(2, 0, 1)));
    }

    #[test]
    fn test_constraint_any() {
        let c = VersionConstraint::parse("*").unwrap();
        assert!(c.matches(&Version::new(0, 0, 0)));
        assert!(c.matches(&Version::new(999, 999, 999)));
    }

    #[test]
    fn test_constraint_gte() {
        let c = VersionConstraint::parse(">=1.0.0").unwrap();
        assert!(c.matches(&Version::new(1, 0, 0)));
        assert!(c.matches(&Version::new(2, 0, 0)));
        assert!(!c.matches(&Version::new(0, 9, 0)));
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary version
    fn arb_version() -> impl Strategy<Value = Version> {
        (0u32..100, 0u32..100, 0u32..100)
            .prop_map(|(major, minor, patch)| Version::new(major, minor, patch))
    }

    /// Generate arbitrary version constraint
    fn arb_constraint() -> impl Strategy<Value = VersionConstraint> {
        prop_oneof![
            arb_version().prop_map(VersionConstraint::Exact),
            arb_version().prop_map(VersionConstraint::Caret),
            arb_version().prop_map(VersionConstraint::Tilde),
            Just(VersionConstraint::Any),
        ]
    }

    proptest! {
        /// Property 18: Version Range OR Syntax
        /// For any version range containing ||, the parser SHALL correctly identify
        /// all satisfying versions as the union of the individual ranges.
        ///
        /// **Validates: Requirements 24.7**
        #[test]
        fn prop_version_or_union(
            v in arb_version(),
            c1 in arb_constraint(),
            c2 in arb_constraint()
        ) {
            // Create OR constraint
            let or_constraint = VersionConstraint::Or(vec![c1.clone(), c2.clone()]);

            // A version matches the OR constraint iff it matches at least one sub-constraint
            let matches_or = or_constraint.matches(&v);
            let matches_c1 = c1.matches(&v);
            let matches_c2 = c2.matches(&v);

            prop_assert_eq!(matches_or, matches_c1 || matches_c2,
                "OR constraint should match iff at least one sub-constraint matches");
        }

        /// Property: Version constraint parsing round-trip
        #[test]
        fn prop_constraint_display_parse_roundtrip(c in arb_constraint()) {
            let displayed = c.to_string();
            let parsed = VersionConstraint::parse(&displayed);

            // Should parse successfully
            prop_assert!(parsed.is_ok(), "Failed to parse: {}", displayed);

            // Should produce equivalent constraint
            let reparsed = parsed.unwrap();

            // Test with some sample versions
            for major in 0..5 {
                for minor in 0..5 {
                    for patch in 0..5 {
                        let v = Version::new(major, minor, patch);
                        prop_assert_eq!(c.matches(&v), reparsed.matches(&v),
                            "Constraint {} and reparsed {} differ on version {}",
                            c, reparsed, v);
                    }
                }
            }
        }

        /// Property: Caret constraint semantics
        #[test]
        fn prop_caret_semantics(base in arb_version(), test in arb_version()) {
            let caret = VersionConstraint::Caret(base);
            let matches = caret.matches(&test);

            // Caret should not match versions below base
            if test < base {
                prop_assert!(!matches, "Caret ^{} should not match {} (below base)", base, test);
            }

            // For major > 0: should match same major only
            if base.major > 0 && test >= base {
                prop_assert_eq!(matches, test.major == base.major,
                    "Caret ^{} should match {} iff same major", base, test);
            }
        }

        /// Property: Tilde constraint semantics
        #[test]
        fn prop_tilde_semantics(base in arb_version(), test in arb_version()) {
            let tilde = VersionConstraint::Tilde(base);
            let matches = tilde.matches(&test);

            // Tilde should not match versions below base
            if test < base {
                prop_assert!(!matches, "Tilde ~{} should not match {} (below base)", base, test);
            }

            // Should match same major.minor only
            if test >= base {
                prop_assert_eq!(matches, test.major == base.major && test.minor == base.minor,
                    "Tilde ~{} should match {} iff same major.minor", base, test);
            }
        }

        /// Property: Any constraint matches everything
        #[test]
        fn prop_any_matches_all(v in arb_version()) {
            let any = VersionConstraint::Any;
            prop_assert!(any.matches(&v), "Any constraint should match all versions");
        }
    }
}
