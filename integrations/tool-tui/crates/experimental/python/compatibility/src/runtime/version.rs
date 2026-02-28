//! Python version parsing and validation
//!
//! Supports PEP 440 version parsing.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Pre-release version component
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreRelease {
    /// Alpha release (e.g., 3.12.0a1)
    Alpha(u32),
    /// Beta release (e.g., 3.12.0b1)
    Beta(u32),
    /// Release candidate (e.g., 3.12.0rc1)
    ReleaseCandidate(u32),
}

impl PartialOrd for PreRelease {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PreRelease {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (PreRelease::Alpha(a), PreRelease::Alpha(b)) => a.cmp(b),
            (PreRelease::Alpha(_), _) => Ordering::Less,
            (PreRelease::Beta(_), PreRelease::Alpha(_)) => Ordering::Greater,
            (PreRelease::Beta(a), PreRelease::Beta(b)) => a.cmp(b),
            (PreRelease::Beta(_), PreRelease::ReleaseCandidate(_)) => Ordering::Less,
            (PreRelease::ReleaseCandidate(_), PreRelease::Alpha(_)) => Ordering::Greater,
            (PreRelease::ReleaseCandidate(_), PreRelease::Beta(_)) => Ordering::Greater,
            (PreRelease::ReleaseCandidate(a), PreRelease::ReleaseCandidate(b)) => a.cmp(b),
        }
    }
}

/// Python version with PEP 440 support
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythonVersion {
    /// Major version (e.g., 3)
    pub major: u8,
    /// Minor version (e.g., 12)
    pub minor: u8,
    /// Patch version (e.g., 0)
    pub patch: u8,
    /// Pre-release component (optional)
    pub pre_release: Option<PreRelease>,
}

impl PythonVersion {
    /// Create a new Python version
    pub fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
        }
    }

    /// Create a version with pre-release
    pub fn with_pre_release(mut self, pre_release: PreRelease) -> Self {
        self.pre_release = Some(pre_release);
        self
    }

    /// Check if this version is supported (3.8 - 3.13)
    pub fn is_supported(&self) -> bool {
        self.major == 3 && self.minor >= 8 && self.minor <= 13
    }

    /// Get the version as a tuple (major, minor)
    pub fn as_tuple(&self) -> (u8, u8) {
        (self.major, self.minor)
    }

    /// Get the full version as a tuple (major, minor, patch)
    pub fn as_full_tuple(&self) -> (u8, u8, u8) {
        (self.major, self.minor, self.patch)
    }

    /// Parse from a version string like "3.12.0" or "3.12.0a1"
    pub fn parse(s: &str) -> Result<Self, VersionParseError> {
        let s = s.trim();

        // Handle pre-release suffixes
        let (version_part, pre_release) = if let Some(idx) = s.find(|c: char| c.is_alphabetic()) {
            let (ver, pre) = s.split_at(idx);
            (ver, Some(Self::parse_pre_release(pre)?))
        } else {
            (s, None)
        };

        let parts: Vec<&str> = version_part.split('.').collect();

        if parts.is_empty() || parts.len() > 3 {
            return Err(VersionParseError::InvalidFormat(s.to_string()));
        }

        let major = parts[0]
            .parse::<u8>()
            .map_err(|_| VersionParseError::InvalidComponent("major".to_string()))?;

        let minor = if parts.len() > 1 {
            parts[1]
                .parse::<u8>()
                .map_err(|_| VersionParseError::InvalidComponent("minor".to_string()))?
        } else {
            0
        };

        let patch = if parts.len() > 2 {
            parts[2]
                .parse::<u8>()
                .map_err(|_| VersionParseError::InvalidComponent("patch".to_string()))?
        } else {
            0
        };

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
        })
    }

    fn parse_pre_release(s: &str) -> Result<PreRelease, VersionParseError> {
        let s = s.to_lowercase();

        if let Some(num) = s.strip_prefix("a") {
            let n = num
                .parse::<u32>()
                .map_err(|_| VersionParseError::InvalidPreRelease(s.clone()))?;
            Ok(PreRelease::Alpha(n))
        } else if let Some(num) = s.strip_prefix("b") {
            let n = num
                .parse::<u32>()
                .map_err(|_| VersionParseError::InvalidPreRelease(s.clone()))?;
            Ok(PreRelease::Beta(n))
        } else if let Some(num) = s.strip_prefix("rc") {
            let n = num
                .parse::<u32>()
                .map_err(|_| VersionParseError::InvalidPreRelease(s.clone()))?;
            Ok(PreRelease::ReleaseCandidate(n))
        } else {
            Err(VersionParseError::InvalidPreRelease(s))
        }
    }
}

impl PartialOrd for PythonVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PythonVersion {
    fn cmp(&self, other: &Self) -> Ordering {
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
        // Pre-release versions are less than release versions
        match (&self.pre_release, &other.pre_release) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

impl fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            match pre {
                PreRelease::Alpha(n) => write!(f, "a{}", n)?,
                PreRelease::Beta(n) => write!(f, "b{}", n)?,
                PreRelease::ReleaseCandidate(n) => write!(f, "rc{}", n)?,
            }
        }
        Ok(())
    }
}

impl FromStr for PythonVersion {
    type Err = VersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Error parsing a version string
#[derive(Debug, Clone, thiserror::Error)]
pub enum VersionParseError {
    #[error("Invalid version format: {0}")]
    InvalidFormat(String),
    #[error("Invalid version component: {0}")]
    InvalidComponent(String),
    #[error("Invalid pre-release: {0}")]
    InvalidPreRelease(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_version() {
        let v = PythonVersion::parse("3.12.0").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 12);
        assert_eq!(v.patch, 0);
        assert!(v.pre_release.is_none());
    }

    #[test]
    fn test_parse_version_with_alpha() {
        let v = PythonVersion::parse("3.13.0a1").unwrap();
        assert_eq!(v.major, 3);
        assert_eq!(v.minor, 13);
        assert_eq!(v.patch, 0);
        assert_eq!(v.pre_release, Some(PreRelease::Alpha(1)));
    }

    #[test]
    fn test_version_ordering() {
        let v1 = PythonVersion::parse("3.11.0").unwrap();
        let v2 = PythonVersion::parse("3.12.0").unwrap();
        let v3 = PythonVersion::parse("3.12.0a1").unwrap();
        let v4 = PythonVersion::parse("3.12.0b1").unwrap();
        let v5 = PythonVersion::parse("3.12.0rc1").unwrap();

        assert!(v1 < v2);
        assert!(v3 < v4);
        assert!(v4 < v5);
        assert!(v5 < v2); // rc < release
    }

    #[test]
    fn test_is_supported() {
        assert!(PythonVersion::new(3, 8, 0).is_supported());
        assert!(PythonVersion::new(3, 12, 0).is_supported());
        assert!(PythonVersion::new(3, 13, 0).is_supported());
        assert!(!PythonVersion::new(3, 7, 0).is_supported());
        assert!(!PythonVersion::new(3, 14, 0).is_supported());
        assert!(!PythonVersion::new(2, 7, 0).is_supported());
    }

    #[test]
    fn test_display() {
        assert_eq!(PythonVersion::new(3, 12, 0).to_string(), "3.12.0");
        assert_eq!(
            PythonVersion::new(3, 13, 0).with_pre_release(PreRelease::Alpha(1)).to_string(),
            "3.13.0a1"
        );
    }
}
