//! Platform detection module
//!
//! Provides functionality for detecting OS, architecture, ABI, and generating wheel tags.

mod detector;
mod manylinux;
mod wheel_tags;

pub use detector::{Libc, Os, Platform, PlatformDetector};
pub use manylinux::{ManylinuxTag, MusllinuxTag};
pub use wheel_tags::{WheelTag, WheelTagGenerator};

use serde::{Deserialize, Serialize};

/// CPU architecture
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Architecture {
    /// x86_64 / AMD64
    X86_64,
    /// x86 / i686
    X86,
    /// ARM64 / AArch64
    Aarch64,
    /// ARM 32-bit
    Arm,
    /// Other architecture
    Other(String),
}

impl Default for Architecture {
    fn default() -> Self {
        #[cfg(target_arch = "x86_64")]
        return Architecture::X86_64;
        #[cfg(target_arch = "x86")]
        return Architecture::X86;
        #[cfg(target_arch = "aarch64")]
        return Architecture::Aarch64;
        #[cfg(target_arch = "arm")]
        return Architecture::Arm;
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        return Architecture::Other(std::env::consts::ARCH.to_string());
    }
}

impl Architecture {
    /// Parse architecture from string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "x86_64" | "amd64" | "x64" => Architecture::X86_64,
            "x86" | "i386" | "i686" => Architecture::X86,
            "aarch64" | "arm64" => Architecture::Aarch64,
            "arm" | "armv7l" | "armv6l" => Architecture::Arm,
            other => Architecture::Other(other.to_string()),
        }
    }

    /// Get the wheel tag platform string
    pub fn wheel_platform(&self) -> &str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::X86 => "i686",
            Architecture::Aarch64 => "aarch64",
            Architecture::Arm => "armv7l",
            Architecture::Other(s) => s,
        }
    }
}
