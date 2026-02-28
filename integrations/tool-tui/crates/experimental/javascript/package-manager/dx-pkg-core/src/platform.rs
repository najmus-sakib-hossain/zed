//! Cross-platform utilities for path handling, symlinks, and permissions
//!
//! This module provides platform-agnostic abstractions for:
//! - Path separator handling
//! - Symlink creation
//! - File permission management

use crate::error::Error;
use crate::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Normalize path separators for the current platform
pub fn normalize_path(path: &str) -> PathBuf {
    // Convert forward slashes to platform-native separators
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

/// Check if a path is safe (no traversal attacks)
pub fn is_safe_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();

    // Check for path traversal patterns
    if path_str.contains("..") {
        return false;
    }

    // Check for absolute paths on Windows
    #[cfg(windows)]
    {
        if path_str.contains(':') && !path_str.starts_with('.') {
            return false;
        }
    }

    // Check for absolute paths on Unix
    #[cfg(unix)]
    {
        if path_str.starts_with('/') {
            return false;
        }
    }

    true
}

/// Create a symlink (cross-platform)
pub fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link)?;
    }

    #[cfg(windows)]
    {
        // On Windows, we need to determine if target is a file or directory
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link)?;
        } else {
            std::os::windows::fs::symlink_file(target, link)?;
        }
    }

    Ok(())
}

/// Create a junction point (Windows) or symlink (Unix)
/// Junction points don't require admin privileges on Windows
pub fn create_junction(target: &Path, link: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent)?;
    }

    #[cfg(windows)]
    {
        // Use junction crate for Windows junction points
        // Junction points work without admin privileges
        junction::create(target, link)
            .map_err(|e| Error::io_with_path(std::io::Error::other(e.to_string()), link))?;
    }

    #[cfg(not(windows))]
    {
        // On Unix, just use symlinks
        std::os::unix::fs::symlink(target, link)?;
    }

    Ok(())
}

/// Check if a path is a symlink
pub fn is_symlink(path: &Path) -> bool {
    path.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

/// Read symlink target
pub fn read_link(path: &Path) -> Result<PathBuf> {
    fs::read_link(path).map_err(Error::from)
}

/// File permissions abstraction
#[derive(Debug, Clone, Copy)]
pub struct FilePermissions {
    pub readable: bool,
    pub writable: bool,
    pub executable: bool,
}

impl FilePermissions {
    /// Create from Unix mode
    pub fn from_mode(mode: u32) -> Self {
        Self {
            readable: mode & 0o400 != 0,
            writable: mode & 0o200 != 0,
            executable: mode & 0o100 != 0,
        }
    }

    /// Convert to Unix mode
    pub fn to_mode(&self) -> u32 {
        let mut mode = 0o644; // Default: rw-r--r--
        if self.executable {
            mode |= 0o111; // Add execute bits
        }
        mode
    }

    /// Default permissions for files
    pub fn default_file() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
        }
    }

    /// Default permissions for executables
    pub fn default_executable() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: true,
        }
    }
}

/// Set file permissions (cross-platform)
pub fn set_permissions(path: &Path, perms: FilePermissions) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = perms.to_mode();
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }

    #[cfg(windows)]
    {
        // Windows doesn't have Unix-style permissions
        // We can only set read-only flag
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(!perms.writable);
        fs::set_permissions(path, permissions)?;
    }

    Ok(())
}

/// Get file permissions (cross-platform)
pub fn get_permissions(path: &Path) -> Result<FilePermissions> {
    let metadata = fs::metadata(path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        Ok(FilePermissions::from_mode(mode))
    }

    #[cfg(windows)]
    {
        let readonly = metadata.permissions().readonly();
        Ok(FilePermissions {
            readable: true,
            writable: !readonly,
            executable: path
                .extension()
                .map(|ext| ext == "exe" || ext == "bat" || ext == "cmd")
                .unwrap_or(false),
        })
    }
}

/// Copy file with permissions preserved
pub fn copy_with_permissions(src: &Path, dst: &Path) -> Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    // Copy file
    fs::copy(src, dst)?;

    // Preserve permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let src_perms = fs::metadata(src)?.permissions();
        fs::set_permissions(dst, fs::Permissions::from_mode(src_perms.mode()))?;
    }

    Ok(())
}

/// Get the platform-specific executable extension
pub fn executable_extension() -> &'static str {
    if cfg!(windows) {
        ".exe"
    } else {
        ""
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
    fn test_is_safe_path() {
        assert!(is_safe_path(Path::new("foo/bar")));
        assert!(is_safe_path(Path::new("./foo/bar")));
        assert!(!is_safe_path(Path::new("../foo/bar")));
        assert!(!is_safe_path(Path::new("foo/../bar")));
    }

    #[test]
    fn test_file_permissions() {
        let perms = FilePermissions::from_mode(0o755);
        assert!(perms.readable);
        assert!(perms.writable);
        assert!(perms.executable);

        let perms = FilePermissions::from_mode(0o644);
        assert!(perms.readable);
        assert!(perms.writable);
        assert!(!perms.executable);
    }

    #[test]
    fn test_symlink_creation() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.txt");
        let link = temp.path().join("link.txt");

        // Create target file
        fs::write(&target, "hello").unwrap();

        // Create symlink
        let result = create_symlink(&target, &link);

        // On some systems (Windows without admin), this might fail
        if result.is_ok() {
            assert!(is_symlink(&link));
            let read_target = read_link(&link).unwrap();
            assert_eq!(read_target, target);
        }
    }

    #[test]
    fn test_copy_with_permissions() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("src.txt");
        let dst = temp.path().join("dst.txt");

        fs::write(&src, "hello").unwrap();
        copy_with_permissions(&src, &dst).unwrap();

        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
    }

    /// Test path normalization with forward slashes on Windows
    #[test]
    fn test_normalize_forward_slashes() {
        let path = normalize_path("node_modules/lodash/index.js");

        // Should work on all platforms
        assert!(path.to_string_lossy().contains("node_modules"));
        assert!(path.to_string_lossy().contains("lodash"));
        assert!(path.to_string_lossy().contains("index.js"));

        // On Windows, should convert to backslashes
        #[cfg(windows)]
        {
            assert!(path.to_string_lossy().contains("\\"));
        }

        // On Unix, should keep forward slashes
        #[cfg(not(windows))]
        {
            assert!(path.to_string_lossy().contains("/"));
        }
    }

    /// Test path normalization with backslashes on Unix
    #[test]
    fn test_normalize_backslashes() {
        let path = normalize_path("node_modules\\lodash\\index.js");

        // Should work on all platforms
        assert!(path.to_string_lossy().contains("node_modules"));
        assert!(path.to_string_lossy().contains("lodash"));
        assert!(path.to_string_lossy().contains("index.js"));

        // On Unix, should convert to forward slashes
        #[cfg(not(windows))]
        {
            assert!(path.to_string_lossy().contains("/"));
        }
    }

    /// Test join_paths with mixed separators
    #[test]
    fn test_join_paths_mixed_separators() {
        let temp = TempDir::new().unwrap();
        let base = temp.path();

        // Join with forward slashes
        let path1 = join_paths(base, "foo/bar/baz.txt");
        assert!(path1.to_string_lossy().contains("foo"));
        assert!(path1.to_string_lossy().contains("bar"));
        assert!(path1.to_string_lossy().contains("baz.txt"));

        // Join with backslashes
        let path2 = join_paths(base, "foo\\bar\\baz.txt");
        assert!(path2.to_string_lossy().contains("foo"));
        assert!(path2.to_string_lossy().contains("bar"));
        assert!(path2.to_string_lossy().contains("baz.txt"));
    }

    /// Test that file operations work with normalized paths
    #[test]
    fn test_file_operations_with_normalized_paths() {
        let temp = TempDir::new().unwrap();

        // Create directory structure using forward slashes
        let dir_path = join_paths(temp.path(), "a/b/c");
        fs::create_dir_all(&dir_path).unwrap();

        // Create file using forward slashes
        let file_path = join_paths(temp.path(), "a/b/c/test.txt");
        fs::write(&file_path, "hello").unwrap();

        // Read file using backslashes (should still work after normalization)
        let read_path = join_paths(temp.path(), "a\\b\\c\\test.txt");
        let content = fs::read_to_string(&read_path).unwrap();
        assert_eq!(content, "hello");
    }

    /// Test to_unix_path round-trip
    #[test]
    fn test_unix_path_round_trip() {
        let original = "node_modules/lodash/index.js";
        let path = normalize_path(original);
        let unix = to_unix_path(&path);

        // Should get back the original Unix-style path
        assert_eq!(unix, original);
    }

    /// Test path safety with various attack patterns
    #[test]
    fn test_path_safety_attacks() {
        // Path traversal attacks
        assert!(!is_safe_path(Path::new("../../../etc/passwd")));
        assert!(!is_safe_path(Path::new("foo/../../bar")));
        assert!(!is_safe_path(Path::new("..\\..\\windows\\system32")));

        // Safe paths
        assert!(is_safe_path(Path::new("node_modules/lodash")));
        assert!(is_safe_path(Path::new("./src/index.js")));
        assert!(is_safe_path(Path::new("@types/node/index.d.ts")));
    }

    /// Test platform detection
    #[test]
    fn test_platform_detection() {
        // At least one of these should be true
        let is_known_platform = is_windows() || is_macos() || is_linux();

        // On CI, we should be on a known platform
        // This test just verifies the functions don't panic
        let _ = is_known_platform;

        // Verify path separator matches platform
        let sep = path_separator();
        #[cfg(windows)]
        assert_eq!(sep, '\\');
        #[cfg(not(windows))]
        assert_eq!(sep, '/');
    }

    /// Test executable extension
    #[test]
    fn test_executable_extension() {
        let ext = executable_extension();

        #[cfg(windows)]
        assert_eq!(ext, ".exe");

        #[cfg(not(windows))]
        assert_eq!(ext, "");
    }
}

/// Property-based tests for cross-platform path handling
/// **Property 4: Cross-Platform Path Handling**
/// **Validates: Requirements 7.4**
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate valid path components (no special characters that would break paths)
    fn valid_path_component() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
    }

    /// Generate a valid relative path with 1-4 components
    fn valid_relative_path() -> impl Strategy<Value = String> {
        prop::collection::vec(valid_path_component(), 1..=4)
            .prop_map(|components| components.join("/"))
    }

    proptest! {
        /// Property: normalize_path followed by to_unix_path should produce a valid Unix path
        /// *For any* valid path string, normalizing and converting back to Unix format
        /// should produce a path with forward slashes only
        #[test]
        fn prop_normalize_then_unix_produces_forward_slashes(path in valid_relative_path()) {
            let normalized = normalize_path(&path);
            let unix = to_unix_path(&normalized);

            // Should not contain backslashes
            prop_assert!(!unix.contains('\\'), "Unix path should not contain backslashes: {}", unix);

            // Should contain forward slashes if multi-component
            if path.contains('/') {
                prop_assert!(unix.contains('/'), "Multi-component path should contain forward slashes: {}", unix);
            }
        }

        /// Property: normalize_path preserves all path components
        /// *For any* valid path, all original components should be present after normalization
        #[test]
        fn prop_normalize_preserves_components(path in valid_relative_path()) {
            let components: Vec<&str> = path.split('/').collect();
            let normalized = normalize_path(&path);
            let normalized_str = normalized.to_string_lossy();

            for component in components {
                prop_assert!(
                    normalized_str.contains(component),
                    "Component '{}' missing from normalized path '{}'",
                    component,
                    normalized_str
                );
            }
        }

        /// Property: to_unix_path is idempotent
        /// *For any* path, converting to Unix format twice should produce the same result
        #[test]
        fn prop_to_unix_path_idempotent(path in valid_relative_path()) {
            let normalized = normalize_path(&path);
            let unix1 = to_unix_path(&normalized);
            let unix2 = to_unix_path(&PathBuf::from(&unix1));

            prop_assert_eq!(unix1, unix2, "to_unix_path should be idempotent");
        }

        /// Property: join_paths produces valid paths
        /// *For any* base path and relative path, joining should produce a path
        /// that contains both the base and relative components
        #[test]
        fn prop_join_paths_contains_both(
            base_component in valid_path_component(),
            relative in valid_relative_path()
        ) {
            let base = PathBuf::from(&base_component);
            let joined = join_paths(&base, &relative);
            let joined_str = joined.to_string_lossy();

            // Should contain base
            prop_assert!(
                joined_str.contains(&base_component),
                "Joined path should contain base: {} not in {}",
                base_component,
                joined_str
            );

            // Should contain all relative components
            for component in relative.split('/') {
                prop_assert!(
                    joined_str.contains(component),
                    "Joined path should contain relative component: {} not in {}",
                    component,
                    joined_str
                );
            }
        }

        /// Property: is_safe_path rejects all path traversal attempts
        /// *For any* path containing "..", it should be rejected as unsafe
        #[test]
        fn prop_safe_path_rejects_traversal(
            prefix in valid_relative_path(),
            suffix in valid_relative_path()
        ) {
            // Create a path with traversal
            let traversal_path = format!("{}/../{}", prefix, suffix);
            let path = Path::new(&traversal_path);

            prop_assert!(
                !is_safe_path(path),
                "Path with traversal should be unsafe: {}",
                traversal_path
            );
        }

        /// Property: Safe paths remain safe after normalization
        /// *For any* safe relative path, it should remain safe after normalization
        #[test]
        fn prop_safe_paths_stay_safe(path in valid_relative_path()) {
            let original_path = Path::new(&path);

            // Our generated paths should be safe
            prop_assert!(
                is_safe_path(original_path),
                "Generated path should be safe: {}",
                path
            );

            // After normalization, should still be safe
            let normalized = normalize_path(&path);
            prop_assert!(
                is_safe_path(&normalized),
                "Normalized path should be safe: {}",
                normalized.display()
            );
        }
    }
}
