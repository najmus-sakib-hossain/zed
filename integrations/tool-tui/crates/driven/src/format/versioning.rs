//! Format versioning and migration

use crate::{DrivenError, Result};

/// Format version information
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FormatVersion {
    pub major: u8,
    pub minor: u8,
}

impl FormatVersion {
    /// Current version
    pub const CURRENT: FormatVersion = FormatVersion { major: 1, minor: 0 };

    /// Create a new version
    pub const fn new(major: u8, minor: u8) -> Self {
        Self { major, minor }
    }

    /// Convert to u16 for binary format
    pub const fn to_u16(self) -> u16 {
        (self.major as u16) << 8 | self.minor as u16
    }

    /// Create from u16
    pub const fn from_u16(value: u16) -> Self {
        Self {
            major: (value >> 8) as u8,
            minor: (value & 0xFF) as u8,
        }
    }

    /// Check if this version is compatible with current
    #[allow(clippy::absurd_extreme_comparisons)]
    pub fn is_compatible(&self) -> bool {
        self.major == Self::CURRENT.major && self.minor <= Self::CURRENT.minor
    }
}

impl std::fmt::Display for FormatVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Version migration handler
#[derive(Debug)]
pub struct VersionMigrator;

impl VersionMigrator {
    /// Migrate data from an older version to current
    pub fn migrate(data: &[u8], from: FormatVersion) -> Result<Vec<u8>> {
        if from >= FormatVersion::CURRENT {
            // No migration needed
            return Ok(data.to_vec());
        }

        if from.major != FormatVersion::CURRENT.major {
            return Err(DrivenError::InvalidBinary(format!(
                "Cannot migrate from version {} to {} (major version mismatch)",
                from,
                FormatVersion::CURRENT
            )));
        }

        // Minor version migrations
        let mut migrated = data.to_vec();

        // Add migration logic here as versions evolve
        // Example:
        // if from.minor < 1 {
        //     migrated = migrate_0_to_1(&migrated)?;
        // }

        Ok(migrated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_roundtrip() {
        let version = FormatVersion::new(1, 5);
        let as_u16 = version.to_u16();
        let back = FormatVersion::from_u16(as_u16);

        assert_eq!(version, back);
    }

    #[test]
    fn test_version_ordering() {
        let v1_0 = FormatVersion::new(1, 0);
        let v1_1 = FormatVersion::new(1, 1);
        let v2_0 = FormatVersion::new(2, 0);

        assert!(v1_0 < v1_1);
        assert!(v1_1 < v2_0);
    }

    #[test]
    fn test_compatibility() {
        let current = FormatVersion::CURRENT;
        assert!(current.is_compatible());

        let older = FormatVersion::new(current.major, 0);
        assert!(older.is_compatible());

        let newer_minor = FormatVersion::new(current.major, current.minor + 1);
        assert!(!newer_minor.is_compatible());

        let newer_major = FormatVersion::new(current.major + 1, 0);
        assert!(!newer_major.is_compatible());
    }

    #[test]
    fn test_display() {
        let version = FormatVersion::new(1, 5);
        assert_eq!(format!("{}", version), "1.5");
    }
}
