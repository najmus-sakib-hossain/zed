//! ABI version detection and compatibility checking
//!
//! Handles CPython ABI version compatibility for C extensions.

use serde::{Deserialize, Serialize};
use std::fmt;

/// ABI version information for CPython extensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AbiVersion {
    /// Python major version (e.g., 3)
    pub major: u32,
    /// Python minor version (e.g., 11, 12)
    pub minor: u32,
    /// ABI flags (debug, pymalloc, etc.)
    pub abi_flags: u32,
}

impl AbiVersion {
    /// Create a new ABI version
    pub const fn new(major: u32, minor: u32, abi_flags: u32) -> Self {
        Self {
            major,
            minor,
            abi_flags,
        }
    }

    /// DX-Py's supported ABI version (Python 3.11 compatible)
    pub const fn dx_py_abi() -> Self {
        Self::new(3, 11, 0)
    }

    /// Parse ABI version from extension filename
    ///
    /// Extension filenames follow the pattern:
    /// - Windows: `module.cp311-win_amd64.pyd`
    /// - Linux: `module.cpython-311-x86_64-linux-gnu.so`
    /// - macOS: `module.cpython-311-darwin.so`
    pub fn from_filename(filename: &str) -> Option<Self> {
        // Look for cpython version tag like "cp311" or "cpython-311"
        let lower = filename.to_lowercase();

        // Try cp{major}{minor} format (e.g., cp311, cp39)
        if let Some(pos) = lower.find("cp3") {
            let version_part = &lower[pos + 3..]; // Skip "cp3"
                                                  // Try two-digit minor version first (e.g., 11 from cp311)
            if let Some(minor_str) = version_part.get(0..2) {
                if let Ok(minor) = minor_str.parse::<u32>() {
                    return Some(Self::new(3, minor, 0));
                }
            }
            // Try single digit minor version (e.g., 9 from cp39)
            if let Some(minor_str) = version_part.get(0..1) {
                if let Ok(minor) = minor_str.parse::<u32>() {
                    return Some(Self::new(3, minor, 0));
                }
            }
        }

        // Try cpython-{major}{minor} format (e.g., cpython-311, cpython-39)
        if let Some(pos) = lower.find("cpython-3") {
            let version_part = &lower[pos + 9..]; // Skip "cpython-3"
                                                  // Try two-digit minor version first
            if let Some(minor_str) = version_part.get(0..2) {
                if let Ok(minor) = minor_str.parse::<u32>() {
                    return Some(Self::new(3, minor, 0));
                }
            }
            // Try single digit minor version
            if let Some(minor_str) = version_part.get(0..1) {
                if let Ok(minor) = minor_str.parse::<u32>() {
                    return Some(Self::new(3, minor, 0));
                }
            }
        }

        None
    }

    /// Parse ABI version from extension metadata (if available)
    ///
    /// This attempts to read the extension's embedded metadata to determine
    /// the exact ABI version it was built for.
    pub fn from_extension_metadata(path: &std::path::Path) -> Option<Self> {
        // First try to get from filename
        if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
            if let Some(abi) = Self::from_filename(filename) {
                return Some(abi);
            }
        }

        // Could potentially read ELF/PE headers here for more info
        // For now, return None if filename parsing fails
        None
    }

    /// Check if this ABI version is compatible with another
    pub fn is_compatible_with(&self, other: &AbiVersion) -> AbiCompatibility {
        if self.major != other.major {
            return AbiCompatibility::Incompatible {
                reason: format!("Major version mismatch: {} vs {}", self.major, other.major),
            };
        }

        if self.minor != other.minor {
            // Minor version differences may be compatible with stable ABI
            if self.minor > other.minor {
                return AbiCompatibility::Compatible {
                    warnings: vec![format!(
                        "Extension built for Python 3.{}, running on 3.{}",
                        other.minor, self.minor
                    )],
                };
            } else {
                return AbiCompatibility::Incompatible {
                    reason: format!(
                        "Extension requires Python 3.{}, but runtime is 3.{}",
                        other.minor, self.minor
                    ),
                };
            }
        }

        // Check ABI flags
        if self.abi_flags != other.abi_flags {
            let mut warnings = Vec::new();

            // Debug flag mismatch
            if (self.abi_flags & ABI_FLAG_DEBUG) != (other.abi_flags & ABI_FLAG_DEBUG) {
                warnings.push("Debug flag mismatch".to_string());
            }

            if !warnings.is_empty() {
                return AbiCompatibility::Compatible { warnings };
            }
        }

        AbiCompatibility::FullyCompatible
    }

    /// Get the expected extension suffix for this ABI version
    pub fn extension_suffix(&self) -> String {
        #[cfg(target_os = "windows")]
        {
            format!(".cp{}{}-win_amd64.pyd", self.major, self.minor)
        }
        #[cfg(target_os = "linux")]
        {
            format!(".cpython-{}{}-x86_64-linux-gnu.so", self.major, self.minor)
        }
        #[cfg(target_os = "macos")]
        {
            format!(".cpython-{}{}-darwin.so", self.major, self.minor)
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            format!(".cpython-{}{}.so", self.major, self.minor)
        }
    }

    /// Check if this version uses the stable ABI (abi3)
    pub fn is_stable_abi(&self) -> bool {
        // Stable ABI was introduced in Python 3.2
        self.major == 3 && self.minor >= 2
    }

    /// Get the minimum compatible version for stable ABI
    pub fn stable_abi_min_version() -> Self {
        Self::new(3, 2, 0)
    }

    /// Check if an extension uses the stable ABI (abi3)
    ///
    /// Stable ABI extensions have filenames like:
    /// - `module.abi3.so` (Linux/macOS)
    /// - `module.pyd` with abi3 tag (Windows)
    pub fn is_stable_abi_extension(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.contains("abi3") || lower.contains(".abi3.")
    }

    /// Verify stable ABI compatibility
    ///
    /// Stable ABI extensions built for Python 3.x should work on any Python 3.y where y >= x
    pub fn verify_stable_abi_compat(&self, extension_min_version: &AbiVersion) -> AbiCompatibility {
        if self.major != extension_min_version.major {
            return AbiCompatibility::Incompatible {
                reason: format!(
                    "Major version mismatch: runtime is Python {}, extension requires Python {}",
                    self.major, extension_min_version.major
                ),
            };
        }

        if self.minor >= extension_min_version.minor {
            AbiCompatibility::FullyCompatible
        } else {
            AbiCompatibility::Incompatible {
                reason: format!(
                    "Extension requires Python 3.{} or later, but runtime is Python 3.{}",
                    extension_min_version.minor, self.minor
                ),
            }
        }
    }
}

impl fmt::Display for AbiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Python {}.{} (ABI flags: 0x{:x})", self.major, self.minor, self.abi_flags)
    }
}

impl Default for AbiVersion {
    fn default() -> Self {
        Self::dx_py_abi()
    }
}

/// ABI flag constants
pub const ABI_FLAG_DEBUG: u32 = 0x01;
pub const ABI_FLAG_PYMALLOC: u32 = 0x02;
pub const ABI_FLAG_WITH_TRACE: u32 = 0x04;

/// Result of ABI compatibility check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiCompatibility {
    /// Fully compatible, no issues
    FullyCompatible,
    /// Compatible but with warnings
    Compatible { warnings: Vec<String> },
    /// Not compatible
    Incompatible { reason: String },
}

impl AbiCompatibility {
    /// Check if the extension can be loaded
    pub fn can_load(&self) -> bool {
        !matches!(self, AbiCompatibility::Incompatible { .. })
    }

    /// Get incompatibility reason if any
    pub fn incompatibility_reason(&self) -> Option<&str> {
        match self {
            AbiCompatibility::Incompatible { reason } => Some(reason),
            _ => None,
        }
    }

    /// Get warnings if any
    pub fn warnings(&self) -> &[String] {
        match self {
            AbiCompatibility::Compatible { warnings } => warnings,
            _ => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_version_from_filename_windows() {
        let abi = AbiVersion::from_filename("numpy.core._multiarray_umath.cp311-win_amd64.pyd");
        assert_eq!(abi, Some(AbiVersion::new(3, 11, 0)));
    }

    #[test]
    fn test_abi_version_from_filename_linux() {
        let abi = AbiVersion::from_filename(
            "numpy.core._multiarray_umath.cpython-311-x86_64-linux-gnu.so",
        );
        assert_eq!(abi, Some(AbiVersion::new(3, 11, 0)));
    }

    #[test]
    fn test_abi_version_from_filename_macos() {
        let abi = AbiVersion::from_filename("numpy.core._multiarray_umath.cpython-311-darwin.so");
        assert_eq!(abi, Some(AbiVersion::new(3, 11, 0)));
    }

    #[test]
    fn test_abi_compatibility_same_version() {
        let v1 = AbiVersion::new(3, 11, 0);
        let v2 = AbiVersion::new(3, 11, 0);
        assert_eq!(v1.is_compatible_with(&v2), AbiCompatibility::FullyCompatible);
    }

    #[test]
    fn test_abi_compatibility_newer_runtime() {
        let runtime = AbiVersion::new(3, 12, 0);
        let extension = AbiVersion::new(3, 11, 0);
        let compat = runtime.is_compatible_with(&extension);
        assert!(compat.can_load());
    }

    #[test]
    fn test_abi_compatibility_older_runtime() {
        let runtime = AbiVersion::new(3, 10, 0);
        let extension = AbiVersion::new(3, 11, 0);
        let compat = runtime.is_compatible_with(&extension);
        assert!(!compat.can_load());
    }

    #[test]
    fn test_abi_compatibility_major_mismatch() {
        let v1 = AbiVersion::new(3, 11, 0);
        let v2 = AbiVersion::new(2, 7, 0);
        let compat = v1.is_compatible_with(&v2);
        assert!(!compat.can_load());
    }

    #[test]
    fn test_stable_abi_detection() {
        assert!(AbiVersion::is_stable_abi_extension("module.abi3.so"));
        assert!(AbiVersion::is_stable_abi_extension(
            "module.cpython-311-x86_64-linux-gnu.abi3.so"
        ));
        assert!(!AbiVersion::is_stable_abi_extension("module.cpython-311-x86_64-linux-gnu.so"));
    }

    #[test]
    fn test_stable_abi_compat() {
        let runtime = AbiVersion::new(3, 11, 0);
        let ext_min = AbiVersion::new(3, 8, 0);

        let compat = runtime.verify_stable_abi_compat(&ext_min);
        assert!(compat.can_load());
    }

    #[test]
    fn test_stable_abi_compat_too_old_runtime() {
        let runtime = AbiVersion::new(3, 7, 0);
        let ext_min = AbiVersion::new(3, 8, 0);

        let compat = runtime.verify_stable_abi_compat(&ext_min);
        assert!(!compat.can_load());
    }
}
