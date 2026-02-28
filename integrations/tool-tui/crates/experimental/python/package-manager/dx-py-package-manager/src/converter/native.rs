//! Native extension packaging for DPP packages
//!
//! Handles platform-specific native extensions (.so/.pyd files) during package build.

use std::path::Path;

/// Platform tag for native extensions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformTag {
    /// OS name (e.g., "linux", "win", "macosx")
    pub os: String,
    /// Architecture (e.g., "x86_64", "aarch64", "arm64")
    pub arch: String,
    /// ABI tag (e.g., "cp312", "abi3")
    pub abi: Option<String>,
    /// Platform-specific tag (e.g., "manylinux_2_17", "win_amd64")
    pub platform: String,
}

impl PlatformTag {
    /// Create a new platform tag
    pub fn new(os: &str, arch: &str, platform: &str) -> Self {
        Self {
            os: os.to_string(),
            arch: arch.to_string(),
            abi: None,
            platform: platform.to_string(),
        }
    }

    /// Create a platform tag with ABI
    pub fn with_abi(os: &str, arch: &str, abi: &str, platform: &str) -> Self {
        Self {
            os: os.to_string(),
            arch: arch.to_string(),
            abi: Some(abi.to_string()),
            platform: platform.to_string(),
        }
    }

    /// Detect the current platform
    pub fn current() -> Self {
        let os = if cfg!(target_os = "windows") {
            "win"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "macos") {
            "macosx"
        } else {
            "unknown"
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else if cfg!(target_arch = "x86") {
            "x86"
        } else {
            "unknown"
        };

        let platform = match (os, arch) {
            ("win", "x86_64") => "win_amd64",
            ("win", "x86") => "win32",
            ("win", "aarch64") => "win_arm64",
            ("linux", "x86_64") => "manylinux_2_17_x86_64",
            ("linux", "aarch64") => "manylinux_2_17_aarch64",
            ("macosx", "x86_64") => "macosx_10_9_x86_64",
            ("macosx", "aarch64") => "macosx_11_0_arm64",
            _ => "unknown",
        };

        Self::new(os, arch, platform)
    }

    /// Parse from a wheel filename tag
    pub fn from_wheel_tag(tag: &str) -> Option<Self> {
        // Wheel tags are like: cp312-cp312-manylinux_2_17_x86_64
        // or: py3-none-any
        let parts: Vec<&str> = tag.split('-').collect();
        if parts.len() < 3 {
            return None;
        }

        let platform = parts[2];

        // Parse platform tag
        if platform.starts_with("manylinux") {
            // Extract arch from platform like "manylinux_2_17_x86_64"
            // The arch is the last part after the version numbers
            let arch = Self::extract_arch_from_manylinux(platform)?;
            Some(Self::with_abi("linux", arch, parts[1], platform))
        } else if platform.starts_with("win") {
            let arch = if platform.contains("amd64") {
                "x86_64"
            } else if platform.contains("arm64") {
                "aarch64"
            } else {
                "x86"
            };
            Some(Self::with_abi("win", arch, parts[1], platform))
        } else if platform.starts_with("macosx") {
            // Extract arch from platform like "macosx_10_9_x86_64" or "macosx_11_0_arm64"
            let arch = Self::extract_arch_from_macosx(platform)?;
            Some(Self::with_abi("macosx", arch, parts[1], platform))
        } else if platform == "any" {
            Some(Self::new("any", "any", "any"))
        } else {
            None
        }
    }

    /// Extract architecture from manylinux platform string
    fn extract_arch_from_manylinux(platform: &str) -> Option<&str> {
        // Format: manylinux_2_17_x86_64 or manylinux2014_x86_64
        if platform.contains("x86_64") {
            Some("x86_64")
        } else if platform.contains("aarch64") {
            Some("aarch64")
        } else if platform.contains("i686") {
            Some("i686")
        } else if platform.contains("armv7l") {
            Some("armv7l")
        } else {
            None
        }
    }

    /// Extract architecture from macosx platform string
    fn extract_arch_from_macosx(platform: &str) -> Option<&str> {
        // Format: macosx_10_9_x86_64 or macosx_11_0_arm64
        if platform.contains("x86_64") {
            Some("x86_64")
        } else if platform.contains("arm64") {
            Some("aarch64")
        } else if platform.contains("universal2") {
            Some("universal2")
        } else {
            None
        }
    }

    /// Check if this platform is compatible with another
    pub fn is_compatible_with(&self, other: &PlatformTag) -> bool {
        // "any" platform is compatible with everything
        if self.platform == "any" || other.platform == "any" {
            return true;
        }

        // Same platform is always compatible
        if self.platform == other.platform {
            return true;
        }

        // Check OS and arch compatibility
        if self.os != other.os || self.arch != other.arch {
            return false;
        }

        // Check manylinux compatibility (newer versions are compatible with older)
        if self.os == "linux" {
            return self.check_manylinux_compat(&other.platform);
        }

        false
    }

    /// Check manylinux version compatibility
    fn check_manylinux_compat(&self, other_platform: &str) -> bool {
        let self_version = Self::parse_manylinux_version(&self.platform);
        let other_version = Self::parse_manylinux_version(other_platform);

        match (self_version, other_version) {
            (Some((self_major, self_minor)), Some((other_major, other_minor))) => {
                // Newer glibc can run older manylinux binaries
                self_major > other_major || (self_major == other_major && self_minor >= other_minor)
            }
            _ => false,
        }
    }

    /// Parse manylinux version from platform string
    fn parse_manylinux_version(platform: &str) -> Option<(u32, u32)> {
        // Format: manylinux_2_17_x86_64 or manylinux2014_x86_64
        if platform.starts_with("manylinux_") {
            let parts: Vec<&str> = platform.split('_').collect();
            if parts.len() >= 3 {
                let major = parts[1].parse().ok()?;
                let minor = parts[2].parse().ok()?;
                return Some((major, minor));
            }
        } else if platform.starts_with("manylinux2014") {
            return Some((2, 17));
        } else if platform.starts_with("manylinux2010") {
            return Some((2, 12));
        } else if platform.starts_with("manylinux1") {
            return Some((2, 5));
        }
        None
    }

    /// Convert to string representation
    pub fn to_tag_string(&self) -> String {
        self.platform.clone()
    }
}

impl std::fmt::Display for PlatformTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.platform)
    }
}

/// Native extension file
#[derive(Debug, Clone)]
pub struct NativeExtension {
    /// Path within the package
    pub path: String,
    /// Extension content (compiled .so/.pyd)
    pub content: Vec<u8>,
    /// Platform tag
    pub platform: PlatformTag,
    /// Python version compatibility (e.g., "cp312", "abi3")
    pub python_abi: String,
}

impl NativeExtension {
    /// Create a new native extension
    pub fn new(path: &str, content: Vec<u8>, platform: PlatformTag, python_abi: &str) -> Self {
        Self {
            path: path.to_string(),
            content,
            platform,
            python_abi: python_abi.to_string(),
        }
    }

    /// Check if this extension is a native library
    pub fn is_native_library(path: &Path) -> bool {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        matches!(ext, "so" | "pyd" | "dylib" | "dll")
    }

    /// Extract platform from filename
    pub fn platform_from_filename(filename: &str) -> Option<PlatformTag> {
        // Native extension filenames often contain platform info
        // e.g., numpy.cpython-312-x86_64-linux-gnu.so
        // or: _ssl.cp312-win_amd64.pyd

        if filename.contains("linux") {
            let arch = if filename.contains("x86_64") {
                "x86_64"
            } else if filename.contains("aarch64") {
                "aarch64"
            } else {
                return None;
            };
            return Some(PlatformTag::new("linux", arch, &format!("manylinux_2_17_{}", arch)));
        }

        if filename.contains("win") {
            let arch = if filename.contains("amd64") {
                "x86_64"
            } else if filename.contains("arm64") {
                "aarch64"
            } else {
                "x86"
            };
            let platform = if arch == "x86_64" {
                "win_amd64"
            } else if arch == "aarch64" {
                "win_arm64"
            } else {
                "win32"
            };
            return Some(PlatformTag::new("win", arch, platform));
        }

        if filename.contains("darwin") || filename.contains("macosx") {
            let arch = if filename.contains("arm64") {
                "aarch64"
            } else {
                "x86_64"
            };
            let platform = if arch == "aarch64" {
                "macosx_11_0_arm64"
            } else {
                "macosx_10_9_x86_64"
            };
            return Some(PlatformTag::new("macosx", arch, platform));
        }

        None
    }

    /// Get the file extension for native libraries on the current platform
    pub fn native_extension() -> &'static str {
        if cfg!(target_os = "windows") {
            "pyd"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        }
    }
}

/// Native extension packager
pub struct NativePackager {
    /// Current platform
    current_platform: PlatformTag,
    /// Collected extensions
    extensions: Vec<NativeExtension>,
}

impl NativePackager {
    /// Create a new native packager
    pub fn new() -> Self {
        Self {
            current_platform: PlatformTag::current(),
            extensions: Vec::new(),
        }
    }

    /// Get the current platform
    pub fn current_platform(&self) -> &PlatformTag {
        &self.current_platform
    }

    /// Add a native extension
    pub fn add_extension(&mut self, extension: NativeExtension) {
        self.extensions.push(extension);
    }

    /// Add a native extension from file content
    pub fn add_from_content(
        &mut self,
        path: &str,
        content: Vec<u8>,
        platform: Option<PlatformTag>,
    ) {
        let platform = platform
            .or_else(|| NativeExtension::platform_from_filename(path))
            .unwrap_or_else(PlatformTag::current);

        let python_abi = Self::extract_python_abi(path);

        self.extensions.push(NativeExtension::new(path, content, platform, &python_abi));
    }

    /// Extract Python ABI from filename
    fn extract_python_abi(filename: &str) -> String {
        // Look for patterns like cp312, cp311, abi3
        if filename.contains("abi3") {
            return "abi3".to_string();
        }

        for version in ["cp313", "cp312", "cp311", "cp310", "cp39", "cp38"] {
            if filename.contains(version) {
                return version.to_string();
            }
        }

        "unknown".to_string()
    }

    /// Get all extensions
    pub fn extensions(&self) -> &[NativeExtension] {
        &self.extensions
    }

    /// Get extensions compatible with the current platform
    pub fn compatible_extensions(&self) -> Vec<&NativeExtension> {
        self.extensions
            .iter()
            .filter(|ext| ext.platform.is_compatible_with(&self.current_platform))
            .collect()
    }

    /// Serialize extensions to binary format
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Write extension count
        data.extend_from_slice(&(self.extensions.len() as u32).to_le_bytes());

        for ext in &self.extensions {
            // Path
            let path_bytes = ext.path.as_bytes();
            data.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(path_bytes);

            // Platform tag
            let platform_string = ext.platform.to_tag_string();
            let platform_bytes = platform_string.as_bytes();
            data.extend_from_slice(&(platform_bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(platform_bytes);

            // Python ABI
            let abi_bytes = ext.python_abi.as_bytes();
            data.extend_from_slice(&(abi_bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(abi_bytes);

            // Content
            data.extend_from_slice(&(ext.content.len() as u64).to_le_bytes());
            data.extend_from_slice(&ext.content);
        }

        data
    }

    /// Deserialize extensions from binary format
    pub fn deserialize(data: &[u8]) -> Option<Vec<NativeExtension>> {
        let mut offset = 0;

        if data.len() < 4 {
            return None;
        }

        let count = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;

        let mut extensions = Vec::with_capacity(count);

        for _ in 0..count {
            // Path
            if offset + 2 > data.len() {
                return None;
            }
            let path_len = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?) as usize;
            offset += 2;

            if offset + path_len > data.len() {
                return None;
            }
            let path = String::from_utf8_lossy(&data[offset..offset + path_len]).to_string();
            offset += path_len;

            // Platform tag
            if offset + 2 > data.len() {
                return None;
            }
            let platform_len =
                u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?) as usize;
            offset += 2;

            if offset + platform_len > data.len() {
                return None;
            }
            let platform_str =
                String::from_utf8_lossy(&data[offset..offset + platform_len]).to_string();
            offset += platform_len;

            // Python ABI
            if offset + 2 > data.len() {
                return None;
            }
            let abi_len = u16::from_le_bytes(data[offset..offset + 2].try_into().ok()?) as usize;
            offset += 2;

            if offset + abi_len > data.len() {
                return None;
            }
            let python_abi = String::from_utf8_lossy(&data[offset..offset + abi_len]).to_string();
            offset += abi_len;

            // Content
            if offset + 8 > data.len() {
                return None;
            }
            let content_len =
                u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?) as usize;
            offset += 8;

            if offset + content_len > data.len() {
                return None;
            }
            let content = data[offset..offset + content_len].to_vec();
            offset += content_len;

            // Create platform tag from string
            let platform = PlatformTag::from_wheel_tag(&format!("py3-none-{}", platform_str))
                .unwrap_or_else(|| PlatformTag::new("unknown", "unknown", &platform_str));

            extensions.push(NativeExtension::new(&path, content, platform, &python_abi));
        }

        Some(extensions)
    }
}

impl Default for NativePackager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_tag_current() {
        let tag = PlatformTag::current();
        assert!(!tag.os.is_empty());
        assert!(!tag.arch.is_empty());
        assert!(!tag.platform.is_empty());
    }

    #[test]
    fn test_platform_tag_from_wheel_tag() {
        let tag = PlatformTag::from_wheel_tag("cp312-cp312-manylinux_2_17_x86_64").unwrap();
        assert_eq!(tag.os, "linux");
        assert_eq!(tag.arch, "x86_64");
        assert_eq!(tag.platform, "manylinux_2_17_x86_64");

        let tag = PlatformTag::from_wheel_tag("cp312-cp312-win_amd64").unwrap();
        assert_eq!(tag.os, "win");
        assert_eq!(tag.arch, "x86_64");
        assert_eq!(tag.platform, "win_amd64");
    }

    #[test]
    fn test_platform_compatibility() {
        let any = PlatformTag::new("any", "any", "any");
        let linux = PlatformTag::new("linux", "x86_64", "manylinux_2_17_x86_64");

        assert!(any.is_compatible_with(&linux));
        assert!(linux.is_compatible_with(&any));
    }

    #[test]
    fn test_native_extension_detection() {
        use std::path::Path;

        assert!(NativeExtension::is_native_library(Path::new("numpy.so")));
        assert!(NativeExtension::is_native_library(Path::new("_ssl.pyd")));
        assert!(!NativeExtension::is_native_library(Path::new("module.py")));
    }

    #[test]
    fn test_native_packager_serialize_roundtrip() {
        let mut packager = NativePackager::new();

        packager.add_extension(NativeExtension::new(
            "test.so",
            b"fake binary content".to_vec(),
            PlatformTag::new("linux", "x86_64", "manylinux_2_17_x86_64"),
            "cp312",
        ));

        let serialized = packager.serialize();
        let deserialized = NativePackager::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.len(), 1);
        assert_eq!(deserialized[0].path, "test.so");
        assert_eq!(deserialized[0].content, b"fake binary content");
        assert_eq!(deserialized[0].python_abi, "cp312");
    }
}
