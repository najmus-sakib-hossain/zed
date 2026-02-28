//! Platform detection
//!
//! Detects current platform properties.

use super::Architecture;
use serde::{Deserialize, Serialize};

/// Operating system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Os {
    /// Linux
    Linux,
    /// Windows
    Windows,
    /// macOS
    MacOs,
    /// FreeBSD
    FreeBsd,
    /// Other OS
    Other(String),
}

impl Default for Os {
    fn default() -> Self {
        #[cfg(target_os = "linux")]
        return Os::Linux;
        #[cfg(target_os = "windows")]
        return Os::Windows;
        #[cfg(target_os = "macos")]
        return Os::MacOs;
        #[cfg(target_os = "freebsd")]
        return Os::FreeBsd;
        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
            target_os = "freebsd"
        )))]
        return Os::Other(std::env::consts::OS.to_string());
    }
}

/// C library type (Linux only)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Libc {
    /// GNU libc (glibc)
    Glibc { major: u32, minor: u32 },
    /// musl libc
    Musl { major: u32, minor: u32 },
}

impl Libc {
    /// Get glibc version string for wheel tags
    pub fn glibc_version_string(&self) -> Option<String> {
        match self {
            Libc::Glibc { major, minor } => Some(format!("{}_{}", major, minor)),
            Libc::Musl { .. } => None,
        }
    }

    /// Get musl version string for wheel tags
    pub fn musl_version_string(&self) -> Option<String> {
        match self {
            Libc::Musl { major, minor } => Some(format!("{}_{}", major, minor)),
            Libc::Glibc { .. } => None,
        }
    }
}

/// Platform information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Platform {
    /// Operating system
    pub os: Os,
    /// CPU architecture
    pub arch: Architecture,
    /// ABI tag (optional)
    pub abi: Option<String>,
    /// C library type (Linux only)
    pub libc: Option<Libc>,
}

impl Default for Platform {
    fn default() -> Self {
        PlatformDetector::detect()
    }
}

/// Platform detection
pub struct PlatformDetector;

impl PlatformDetector {
    /// Detect current platform
    pub fn detect() -> Platform {
        let os = Os::default();
        let arch = Architecture::default();
        let libc = Self::detect_libc();
        let abi = Self::detect_abi(&os);

        Platform {
            os,
            arch,
            abi,
            libc,
        }
    }

    /// Detect C library type on Linux
    fn detect_libc() -> Option<Libc> {
        #[cfg(target_os = "linux")]
        {
            // Try to detect glibc version
            if let Some(version) = Self::detect_glibc_version() {
                return Some(Libc::Glibc {
                    major: version.0,
                    minor: version.1,
                });
            }
            // Try to detect musl version
            if let Some(version) = Self::detect_musl_version() {
                return Some(Libc::Musl {
                    major: version.0,
                    minor: version.1,
                });
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    fn detect_glibc_version() -> Option<(u32, u32)> {
        use std::process::Command;

        // Try ldd --version
        if let Ok(output) = Command::new("ldd").arg("--version").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("{}{}", stdout, stderr);

            // Parse version from output like "ldd (GNU libc) 2.31"
            for line in combined.lines() {
                if line.contains("GLIBC") || line.contains("GNU libc") || line.contains("glibc") {
                    if let Some(version) = Self::parse_version_from_line(line) {
                        return Some(version);
                    }
                }
            }
        }

        // Fallback: assume glibc 2.17 (manylinux2014 baseline)
        Some((2, 17))
    }

    #[cfg(target_os = "linux")]
    fn detect_musl_version() -> Option<(u32, u32)> {
        use std::process::Command;

        // Check if we're on Alpine/musl
        if let Ok(output) = Command::new("ldd").arg("--version").output() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("musl") {
                // Parse musl version
                for line in stderr.lines() {
                    if line.contains("musl") {
                        if let Some(version) = Self::parse_version_from_line(line) {
                            return Some(version);
                        }
                    }
                }
                // Default musl version
                return Some((1, 2));
            }
        }
        None
    }

    #[cfg(target_os = "linux")]
    fn parse_version_from_line(line: &str) -> Option<(u32, u32)> {
        // Look for version pattern like "2.31" or "1.2.3"
        let re = regex::Regex::new(r"(\d+)\.(\d+)").ok()?;
        if let Some(caps) = re.captures(line) {
            let major = caps.get(1)?.as_str().parse().ok()?;
            let minor = caps.get(2)?.as_str().parse().ok()?;
            return Some((major, minor));
        }
        None
    }

    /// Detect ABI tag
    fn detect_abi(os: &Os) -> Option<String> {
        match os {
            Os::Windows => Some("win".to_string()),
            Os::MacOs => Some("darwin".to_string()),
            Os::Linux => Some("linux".to_string()),
            Os::FreeBsd => Some("freebsd".to_string()),
            Os::Other(s) => Some(s.clone()),
        }
    }
}

impl Platform {
    /// Get the platform tag for wheel filenames
    pub fn wheel_platform_tag(&self) -> String {
        match &self.os {
            Os::Windows => match &self.arch {
                Architecture::X86_64 => "win_amd64".to_string(),
                Architecture::X86 => "win32".to_string(),
                Architecture::Aarch64 => "win_arm64".to_string(),
                _ => format!("win_{}", self.arch.wheel_platform()),
            },
            Os::MacOs => {
                let arch = match &self.arch {
                    Architecture::X86_64 => "x86_64",
                    Architecture::Aarch64 => "arm64",
                    _ => self.arch.wheel_platform(),
                };
                format!("macosx_10_9_{}", arch)
            }
            Os::Linux => {
                let arch = self.arch.wheel_platform();
                if let Some(Libc::Musl { major, minor }) = &self.libc {
                    format!("musllinux_{}_{}", major, minor)
                } else if let Some(Libc::Glibc { major, minor }) = &self.libc {
                    format!("manylinux_{}_{}", major, minor)
                } else {
                    format!("linux_{}", arch)
                }
            }
            Os::FreeBsd => format!("freebsd_{}", self.arch.wheel_platform()),
            Os::Other(s) => format!("{}_{}", s, self.arch.wheel_platform()),
        }
    }
}
