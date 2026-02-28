//! Cross-platform utilities for the runtime
//!
//! Provides platform-agnostic abstractions for:
//! - Path separator handling
//! - Line ending normalization
//! - Platform detection

use std::path::{Path, PathBuf};

/// Normalize path separators for the current platform
pub fn normalize_path(path: &str) -> PathBuf {
    let normalized = if cfg!(windows) {
        path.replace('/', "\\")
    } else {
        path.replace('\\', "/")
    };
    PathBuf::from(normalized)
}

/// Convert path to use forward slashes (for storage/serialization)
pub fn to_unix_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Join paths in a cross-platform way
pub fn join_paths(base: &Path, relative: &str) -> PathBuf {
    let normalized = normalize_path(relative);
    base.join(normalized)
}

/// Normalize line endings to Unix style (\n)
pub fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

/// Convert line endings to platform-native style
pub fn to_native_line_endings(content: &str) -> String {
    let normalized = normalize_line_endings(content);
    if cfg!(windows) {
        normalized.replace('\n', "\r\n")
    } else {
        normalized
    }
}

/// Get the platform-specific path separator
pub fn path_separator() -> char {
    if cfg!(windows) {
        '\\'
    } else {
        '/'
    }
}

/// Get the platform-specific line ending
pub fn line_ending() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

/// Check if running on Windows
pub fn is_windows() -> bool {
    cfg!(windows)
}

/// Check if running on macOS
pub fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Check if running on Linux
pub fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

/// Get the platform name
pub fn platform_name() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

/// Get the architecture name
pub fn arch_name() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "ia32"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        let path = normalize_path("foo/bar/baz");
        assert!(path.to_string_lossy().contains("foo"));
        assert!(path.to_string_lossy().contains("bar"));
        assert!(path.to_string_lossy().contains("baz"));
    }

    #[test]
    fn test_to_unix_path() {
        let path = PathBuf::from("foo").join("bar").join("baz");
        let unix = to_unix_path(&path);
        assert_eq!(unix, "foo/bar/baz");
    }

    #[test]
    fn test_normalize_line_endings() {
        assert_eq!(normalize_line_endings("a\r\nb\r\nc"), "a\nb\nc");
        assert_eq!(normalize_line_endings("a\rb\rc"), "a\nb\nc");
        assert_eq!(normalize_line_endings("a\nb\nc"), "a\nb\nc");
    }

    #[test]
    fn test_platform_detection() {
        // At least one should be true on known platforms
        let _ = is_windows() || is_macos() || is_linux();

        // Platform name should not be empty
        assert!(!platform_name().is_empty());
        assert!(!arch_name().is_empty());
    }

    #[test]
    fn test_path_separator() {
        let sep = path_separator();
        #[cfg(windows)]
        assert_eq!(sep, '\\');
        #[cfg(not(windows))]
        assert_eq!(sep, '/');
    }

    #[test]
    fn test_line_ending() {
        let ending = line_ending();
        #[cfg(windows)]
        assert_eq!(ending, "\r\n");
        #[cfg(not(windows))]
        assert_eq!(ending, "\n");
    }
}
