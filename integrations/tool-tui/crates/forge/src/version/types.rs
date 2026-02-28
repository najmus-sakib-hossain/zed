//! Version types - semantic versioning support

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Semantic version following semver 2.0.0 specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl Version {
    /// Create a new version
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build: None,
        }
    }

    /// Check if this version is compatible with another (same major version)
    pub fn is_compatible_with(&self, other: &Version) -> bool {
        self.major == other.major && self.major > 0
    }

    /// Check if this version satisfies a requirement
    pub fn satisfies(&self, req: &VersionReq) -> bool {
        match req {
            VersionReq::Exact(v) => self == v,
            VersionReq::GreaterThan(v) => self > v,
            VersionReq::GreaterOrEqual(v) => self >= v,
            VersionReq::LessThan(v) => self < v,
            VersionReq::LessOrEqual(v) => self <= v,
            VersionReq::Compatible(v) => self.is_compatible_with(v) && self >= v,
            VersionReq::Any => true,
        }
    }

    /// Check if this is a pre-release version
    pub fn is_prerelease(&self) -> bool {
        self.pre_release.is_some()
    }

    /// Check if this is a stable version (1.0.0 or greater, no pre-release)
    pub fn is_stable(&self) -> bool {
        self.major >= 1 && !self.is_prerelease()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl FromStr for Version {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        // Remove 'v' prefix if present
        let s = s.strip_prefix('v').unwrap_or(s);

        // Split on '+' for build metadata
        let (version_pre, build) = match s.split_once('+') {
            Some((v, b)) => (v, Some(b.to_string())),
            None => (s, None),
        };

        // Split on '-' for pre-release
        let (version, pre_release) = match version_pre.split_once('-') {
            Some((v, p)) => (v, Some(p.to_string())),
            None => (version_pre, None),
        };

        // Parse major.minor.patch
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid version format: {}", s));
        }

        let major = parts[0].parse().context("Failed to parse major version")?;
        let minor = parts[1].parse().context("Failed to parse minor version")?;
        let patch = parts[2].parse().context("Failed to parse patch version")?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build,
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major.minor.patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Pre-release versions have lower precedence
        match (&self.pre_release, &other.pre_release) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

/// Version requirement for dependency resolution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionReq {
    /// Exact version match (=1.2.3)
    Exact(Version),
    /// Greater than (>1.2.3)
    GreaterThan(Version),
    /// Greater or equal (>=1.2.3)
    GreaterOrEqual(Version),
    /// Less than (<1.2.3)
    LessThan(Version),
    /// Less or equal (<=1.2.3)
    LessOrEqual(Version),
    /// Compatible (^1.2.3 - same major, >= minor.patch)
    Compatible(Version),
    /// Any version (*)
    Any,
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionReq::Exact(v) => write!(f, "={}", v),
            VersionReq::GreaterThan(v) => write!(f, ">{}", v),
            VersionReq::GreaterOrEqual(v) => write!(f, ">={}", v),
            VersionReq::LessThan(v) => write!(f, "<{}", v),
            VersionReq::LessOrEqual(v) => write!(f, "<={}", v),
            VersionReq::Compatible(v) => write!(f, "^{}", v),
            VersionReq::Any => write!(f, "*"),
        }
    }
}

impl FromStr for VersionReq {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();

        if s == "*" {
            return Ok(VersionReq::Any);
        }

        if let Some(v) = s.strip_prefix(">=") {
            return Ok(VersionReq::GreaterOrEqual(v.trim().parse()?));
        }
        if let Some(v) = s.strip_prefix('>') {
            return Ok(VersionReq::GreaterThan(v.trim().parse()?));
        }
        if let Some(v) = s.strip_prefix("<=") {
            return Ok(VersionReq::LessOrEqual(v.trim().parse()?));
        }
        if let Some(v) = s.strip_prefix('<') {
            return Ok(VersionReq::LessThan(v.trim().parse()?));
        }
        if let Some(v) = s.strip_prefix('=') {
            return Ok(VersionReq::Exact(v.trim().parse()?));
        }
        if let Some(v) = s.strip_prefix('^') {
            return Ok(VersionReq::Compatible(v.trim().parse()?));
        }

        // Default to compatible
        Ok(VersionReq::Compatible(s.parse()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        let v = "1.2.3".parse::<Version>().unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        let v = "v2.0.0-beta.1".parse::<Version>().unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.pre_release, Some("beta.1".to_string()));

        let v = "1.0.0+build.123".parse::<Version>().unwrap();
        assert_eq!(v.build, Some("build.123".to_string()));
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 5, 0);
        let v3 = Version::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(!v1.is_compatible_with(&v3));
    }

    #[test]
    fn test_version_requirements() {
        let v = Version::new(1, 2, 3);

        let req = "^1.0.0".parse::<VersionReq>().unwrap();
        assert!(v.satisfies(&req));

        let req = ">=1.2.0".parse::<VersionReq>().unwrap();
        assert!(v.satisfies(&req));

        let req = ">2.0.0".parse::<VersionReq>().unwrap();
        assert!(!v.satisfies(&req));
    }
}
