//! manylinux and musllinux tag support
//!
//! Parses and validates manylinux/musllinux wheel tags.

use std::str::FromStr;

/// manylinux tag representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManylinuxTag {
    /// manylinux1 (glibc 2.5)
    Manylinux1,
    /// manylinux2010 (glibc 2.12)
    Manylinux2010,
    /// manylinux2014 (glibc 2.17)
    Manylinux2014,
    /// manylinux_x_y (glibc x.y)
    ManylinuxXY { major: u32, minor: u32 },
}

impl ManylinuxTag {
    /// Get the minimum glibc version required
    pub fn min_glibc_version(&self) -> (u32, u32) {
        match self {
            ManylinuxTag::Manylinux1 => (2, 5),
            ManylinuxTag::Manylinux2010 => (2, 12),
            ManylinuxTag::Manylinux2014 => (2, 17),
            ManylinuxTag::ManylinuxXY { major, minor } => (*major, *minor),
        }
    }

    /// Check if this tag is compatible with a given glibc version
    pub fn is_compatible_with_glibc(&self, major: u32, minor: u32) -> bool {
        let (req_major, req_minor) = self.min_glibc_version();
        major > req_major || (major == req_major && minor >= req_minor)
    }

    /// Get the tag string
    pub fn as_str(&self) -> String {
        match self {
            ManylinuxTag::Manylinux1 => "manylinux1".to_string(),
            ManylinuxTag::Manylinux2010 => "manylinux2010".to_string(),
            ManylinuxTag::Manylinux2014 => "manylinux2014".to_string(),
            ManylinuxTag::ManylinuxXY { major, minor } => format!("manylinux_{}_{}", major, minor),
        }
    }
}

impl FromStr for ManylinuxTag {
    type Err = ManylinuxParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        // Remove architecture suffix if present (e.g., _x86_64, _aarch64)
        // Architecture suffixes are at the end and follow a pattern like _arch
        let tag_part = strip_arch_suffix(&s);

        if tag_part == "manylinux1" {
            Ok(ManylinuxTag::Manylinux1)
        } else if tag_part == "manylinux2010" {
            Ok(ManylinuxTag::Manylinux2010)
        } else if tag_part == "manylinux2014" {
            Ok(ManylinuxTag::Manylinux2014)
        } else if tag_part.starts_with("manylinux_") {
            let parts: Vec<&str> =
                tag_part.strip_prefix("manylinux_").unwrap().split('_').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse().map_err(|_| ManylinuxParseError::InvalidVersion)?;
                let minor = parts[1].parse().map_err(|_| ManylinuxParseError::InvalidVersion)?;
                Ok(ManylinuxTag::ManylinuxXY { major, minor })
            } else {
                Err(ManylinuxParseError::InvalidFormat)
            }
        } else {
            Err(ManylinuxParseError::InvalidFormat)
        }
    }
}

/// musllinux tag representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusllinuxTag {
    /// Major version
    pub major: u32,
    /// Minor version
    pub minor: u32,
}

impl MusllinuxTag {
    /// Create a new musllinux tag
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Check if this tag is compatible with a given musl version
    pub fn is_compatible_with_musl(&self, major: u32, minor: u32) -> bool {
        major > self.major || (major == self.major && minor >= self.minor)
    }

    /// Get the tag string
    pub fn as_str(&self) -> String {
        format!("musllinux_{}_{}", self.major, self.minor)
    }
}

impl FromStr for MusllinuxTag {
    type Err = ManylinuxParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();

        if !s.starts_with("musllinux_") {
            return Err(ManylinuxParseError::InvalidFormat);
        }

        let version_part = s.strip_prefix("musllinux_").unwrap();
        let parts: Vec<&str> = version_part.split('_').take(2).collect();

        if parts.len() < 2 {
            return Err(ManylinuxParseError::InvalidFormat);
        }

        let major = parts[0].parse().map_err(|_| ManylinuxParseError::InvalidVersion)?;
        let minor = parts[1].parse().map_err(|_| ManylinuxParseError::InvalidVersion)?;

        Ok(MusllinuxTag { major, minor })
    }
}

/// Error parsing manylinux/musllinux tags
#[derive(Debug, Clone, thiserror::Error)]
pub enum ManylinuxParseError {
    #[error("Invalid manylinux/musllinux tag format")]
    InvalidFormat,
    #[error("Invalid version number")]
    InvalidVersion,
}

/// Strip architecture suffix from a tag string
/// e.g., "manylinux1_x86_64" -> "manylinux1"
/// e.g., "manylinux_2_28_x86_64" -> "manylinux_2_28"
fn strip_arch_suffix(s: &str) -> &str {
    // Known architecture suffixes
    const ARCH_SUFFIXES: &[&str] = &[
        "_x86_64",
        "_i686",
        "_aarch64",
        "_armv7l",
        "_ppc64le",
        "_s390x",
        "_arm64",
        "_universal2",
    ];

    for suffix in ARCH_SUFFIXES {
        if let Some(stripped) = s.strip_suffix(suffix) {
            return stripped;
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manylinux1() {
        let tag: ManylinuxTag = "manylinux1_x86_64".parse().unwrap();
        assert_eq!(tag, ManylinuxTag::Manylinux1);
        assert_eq!(tag.min_glibc_version(), (2, 5));
    }

    #[test]
    fn test_parse_manylinux2014() {
        let tag: ManylinuxTag = "manylinux2014_x86_64".parse().unwrap();
        assert_eq!(tag, ManylinuxTag::Manylinux2014);
        assert_eq!(tag.min_glibc_version(), (2, 17));
    }

    #[test]
    fn test_parse_manylinux_x_y() {
        let tag: ManylinuxTag = "manylinux_2_28_x86_64".parse().unwrap();
        assert_eq!(
            tag,
            ManylinuxTag::ManylinuxXY {
                major: 2,
                minor: 28
            }
        );
        assert_eq!(tag.min_glibc_version(), (2, 28));
    }

    #[test]
    fn test_parse_musllinux() {
        let tag: MusllinuxTag = "musllinux_1_2_x86_64".parse().unwrap();
        assert_eq!(tag.major, 1);
        assert_eq!(tag.minor, 2);
    }

    #[test]
    fn test_glibc_compatibility() {
        let tag = ManylinuxTag::Manylinux2014;
        assert!(tag.is_compatible_with_glibc(2, 17));
        assert!(tag.is_compatible_with_glibc(2, 31));
        assert!(!tag.is_compatible_with_glibc(2, 12));
    }
}
