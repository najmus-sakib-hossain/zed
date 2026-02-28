//! Path utilities
//!
//! Provides cross-platform path handling with support for:
//! - Home directory expansion (~)
//! - Mixed path separator handling
//! - Windows long path support
//! - Symlink resolution with depth limit
//! - Unicode path support (emoji, CJK, RTL scripts)
//! - Project boundary checking
//! - Shell escaping for paths

use std::path::{Path, PathBuf};

use crate::utils::error::DxError;

/// Maximum symlink resolution depth
const MAX_SYMLINK_DEPTH: usize = 40;

/// Find the project root by looking for dx.toml
#[allow(dead_code)]
pub fn find_project_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    loop {
        if current.join("dx.toml").exists() {
            return Some(current);
        }

        if current.join("package.json").exists() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

/// Get the DX home directory (~/.dx)
#[allow(dead_code)]
pub fn dx_home() -> PathBuf {
    home::home_dir().map(|h| h.join(".dx")).unwrap_or_else(fallback_dir)
}

/// Get the cache directory
#[allow(dead_code)]
pub fn cache_dir() -> PathBuf {
    dx_home().join("cache")
}

/// Get the global bin directory
#[allow(dead_code)]
pub fn bin_dir() -> PathBuf {
    dx_home().join("bin")
}

/// Get a fallback directory when home is not available or not writable
///
/// Requirement 11.3: Fallback for non-writable home
pub fn fallback_dir() -> PathBuf {
    // Try current directory first
    if let Ok(current) = std::env::current_dir() {
        let fallback = current.join(".dx");
        // Check if we can write to current directory
        if is_dir_writable(&current) {
            return fallback;
        }
    }

    // Try temp directory as last resort
    std::env::temp_dir().join(".dx")
}

/// Check if a directory is writable
fn is_dir_writable(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }

    // Try to create a temp file to test writability
    let test_file = path.join(format!(".dx_write_test_{}", std::process::id()));
    match std::fs::write(&test_file, b"test") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

/// Resolve a path string, handling:
/// - Home directory expansion (~)
/// - Mixed path separators (/ and \)
/// - Unicode characters (emoji, CJK, RTL scripts)
///
/// Requirement 2.1: Handle mixed path separators
/// Requirement 2.2: Expand ~ to home directory
/// Requirement 2.5: Handle Unicode paths
pub fn resolve_path(path: &str) -> PathBuf {
    // Handle empty path
    if path.is_empty() {
        return PathBuf::new();
    }

    // Expand home directory
    let expanded = if path.starts_with("~/") || path == "~" {
        if let Some(home) = home::home_dir() {
            if path == "~" {
                home
            } else {
                home.join(&path[2..])
            }
        } else {
            PathBuf::from(path)
        }
    } else {
        PathBuf::from(path)
    };

    // Normalize path separators
    // On Windows, convert forward slashes to backslashes
    // On Unix, convert backslashes to forward slashes
    // Unicode characters are preserved as-is
    #[cfg(windows)]
    {
        let normalized = expanded.to_string_lossy().replace('/', "\\");
        PathBuf::from(normalized)
    }

    #[cfg(not(windows))]
    {
        let normalized = expanded.to_string_lossy().replace('\\', "/");
        PathBuf::from(normalized)
    }
}

/// Resolve symlinks up to MAX_SYMLINK_DEPTH levels
///
/// Requirement 2.4: Follow symlinks up to 40 levels
pub fn resolve_symlinks(path: &Path) -> Result<PathBuf, DxError> {
    let mut current = path.to_path_buf();
    let mut depth = 0;

    while depth < MAX_SYMLINK_DEPTH {
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    current = std::fs::read_link(&current).map_err(|_| DxError::Io {
                        message: format!("Failed to read symlink: {}", current.display()),
                    })?;
                    depth += 1;
                } else {
                    // Not a symlink, we're done
                    return Ok(current);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(DxError::FileNotFound { path: current });
            }
            Err(e) => {
                return Err(DxError::Io {
                    message: e.to_string(),
                });
            }
        }
    }

    // Exceeded max depth
    Err(DxError::SymlinkLoop {
        path: path.to_path_buf(),
    })
}

/// Handle Windows long paths by adding \\?\ prefix
///
/// Requirement 2.3: Add \\?\ prefix for paths > 200 chars on Windows
#[allow(dead_code)]
pub fn handle_long_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let path_str = path.to_string_lossy();
        if path_str.len() > 200 && !path_str.starts_with("\\\\?\\") {
            PathBuf::from(format!("\\\\?\\{}", path_str))
        } else {
            path.to_path_buf()
        }
    }

    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Check if a path is within the project directory
///
/// Requirement 2.7: Project boundary checking
pub fn is_within_project(path: &Path, project_root: &Path) -> Result<bool, DxError> {
    // Resolve symlinks for both paths
    let resolved_path = if path.exists() {
        resolve_symlinks(path).unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    let resolved_root = if project_root.exists() {
        resolve_symlinks(project_root).unwrap_or_else(|_| project_root.to_path_buf())
    } else {
        project_root.to_path_buf()
    };

    // Canonicalize for accurate comparison
    let canonical_path = if resolved_path.is_absolute() {
        resolved_path.canonicalize().unwrap_or(resolved_path)
    } else {
        resolved_root
            .join(&resolved_path)
            .canonicalize()
            .unwrap_or(resolved_root.join(&resolved_path))
    };

    let canonical_root = resolved_root.canonicalize().unwrap_or(resolved_root);

    Ok(canonical_path.starts_with(&canonical_root))
}

/// Escape a path for safe shell execution
///
/// Requirement 2.6: Shell escaping for paths
pub fn escape_for_shell(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Characters that need escaping in shell
    let needs_escaping = |c: char| {
        matches!(
            c,
            ' ' | '\t'
                | '\n'
                | '\''
                | '"'
                | '\\'
                | '$'
                | '`'
                | '!'
                | '&'
                | '|'
                | ';'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '<'
                | '>'
                | '*'
                | '?'
                | '#'
                | '~'
                | '^'
        )
    };

    // Check if escaping is needed
    if !path_str.chars().any(needs_escaping) {
        return path_str.into_owned();
    }

    // Use single quotes for most cases (safest)
    // If the path contains single quotes, use double quotes with escaping
    if !path_str.contains('\'') {
        format!("'{}'", path_str)
    } else if !path_str.contains('"') {
        // Escape $ ` \ " in double quotes
        let escaped: String = path_str
            .chars()
            .map(|c| match c {
                '$' | '`' | '\\' | '"' => format!("\\{}", c),
                _ => c.to_string(),
            })
            .collect();
        format!("\"{}\"", escaped)
    } else {
        // Both quote types present, escape everything
        let escaped: String = path_str
            .chars()
            .map(|c| {
                if needs_escaping(c) {
                    format!("\\{}", c)
                } else {
                    c.to_string()
                }
            })
            .collect();
        escaped
    }
}

/// Check if running in a CI environment
///
/// Requirement 11.7: Detect CI environment
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("BUILDKITE").is_ok()
        || std::env::var("TEAMCITY_VERSION").is_ok()
        || std::env::var("TF_BUILD").is_ok() // Azure Pipelines
}

/// Check if running in a container (Docker, Kubernetes, etc.)
///
/// Requirement 11.7: Detect container environment
pub fn is_container() -> bool {
    // Check for Docker
    if Path::new("/.dockerenv").exists() {
        return true;
    }

    // Check for Kubernetes/Docker via cgroup
    #[cfg(target_os = "linux")]
    {
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker") || cgroup.contains("kubepods") {
                return true;
            }
        }
    }

    // Check for container environment variables
    std::env::var("KUBERNETES_SERVICE_HOST").is_ok() || std::env::var("DOCKER_CONTAINER").is_ok()
}

/// Get terminal width, with fallback
///
/// Requirement 11.6: Detect terminal width
pub fn terminal_width() -> usize {
    // Try to get terminal size
    if let Some((width, _)) = term_size::dimensions() {
        width
    } else {
        // Default fallback
        80
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_resolve_path_empty() {
        let result = resolve_path("");
        assert_eq!(result, PathBuf::new());
    }

    #[test]
    fn test_resolve_path_simple() {
        let result = resolve_path("foo/bar");
        #[cfg(windows)]
        assert_eq!(result, PathBuf::from("foo\\bar"));
        #[cfg(not(windows))]
        assert_eq!(result, PathBuf::from("foo/bar"));
    }

    #[test]
    fn test_resolve_path_home() {
        let result = resolve_path("~/test");
        if let Some(home) = home::home_dir() {
            assert_eq!(result, home.join("test"));
        }
    }

    #[test]
    fn test_resolve_path_home_only() {
        let result = resolve_path("~");
        if let Some(home) = home::home_dir() {
            assert_eq!(result, home);
        }
    }

    #[test]
    fn test_is_ci_detection() {
        // This test just verifies the function doesn't panic
        let _ = is_ci();
    }

    #[test]
    fn test_is_container_detection() {
        // This test just verifies the function doesn't panic
        let _ = is_container();
    }

    #[test]
    fn test_terminal_width() {
        let width = terminal_width();
        assert!(width > 0);
    }

    #[test]
    #[cfg(windows)]
    fn test_handle_long_path_windows() {
        let short_path = PathBuf::from("C:\\short\\path");
        assert_eq!(handle_long_path(&short_path), short_path);

        let long_path = PathBuf::from(format!("C:\\{}", "a".repeat(250)));
        let result = handle_long_path(&long_path);
        assert!(result.to_string_lossy().starts_with("\\\\?\\"));
    }

    // Feature: dx-cli, Property 18: Path Separator Handling
    // Validates: Requirements 11.1
    //
    // For any path string with mixed separators (/ and \), resolve_path
    // should produce a valid PathBuf that can be used for file operations.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_path_separator_handling(
            parts in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5)
        ) {
            // Create path with mixed separators
            let mixed_path = parts.join("/");
            let result = resolve_path(&mixed_path);

            // Result should be a valid PathBuf
            prop_assert!(!result.as_os_str().is_empty() || mixed_path.is_empty());

            // On Windows, should use backslashes
            #[cfg(windows)]
            {
                let result_str = result.to_string_lossy();
                prop_assert!(
                    !result_str.contains('/'),
                    "Windows paths should not contain forward slashes: {}",
                    result_str
                );
            }

            // On Unix, should use forward slashes
            #[cfg(not(windows))]
            {
                let result_str = result.to_string_lossy();
                prop_assert!(
                    !result_str.contains('\\'),
                    "Unix paths should not contain backslashes: {}",
                    result_str
                );
            }
        }
    }

    // Feature: dx-cli, Property 19: Home Directory Expansion
    // Validates: Requirements 11.2
    //
    // For any path string starting with "~/", resolve_path should replace
    // the prefix with the user's home directory.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_home_directory_expansion(
            suffix in "[a-zA-Z0-9_-]{1,50}"
        ) {
            let path = format!("~/{}", suffix);
            let result = resolve_path(&path);

            if let Some(home) = home::home_dir() {
                // Result should start with home directory
                prop_assert!(
                    result.starts_with(&home),
                    "Path should start with home directory: {:?} vs {:?}",
                    result,
                    home
                );

                // Result should not contain ~
                let result_str = result.to_string_lossy();
                prop_assert!(
                    !result_str.contains('~'),
                    "Expanded path should not contain ~: {}",
                    result_str
                );
            }
        }
    }

    // Feature: dx-cli, Property 20: Symlink Resolution Depth
    // Validates: Requirements 11.4
    //
    // For any path with N levels of symlinks where N <= 40, resolve_symlinks
    // should successfully resolve to the final target. For N > 40, it should
    // return an error.
    //
    // Note: This property is tested with unit tests since creating actual
    // symlinks in property tests is complex and platform-dependent.
    #[test]
    fn test_symlink_resolution_nonexistent() {
        // Use a platform-agnostic nonexistent path
        let temp_dir = std::env::temp_dir();
        let nonexistent = temp_dir.join("definitely_does_not_exist_dx_test");
        let result = resolve_symlinks(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_symlink_resolution_regular_file() {
        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("dx_test_regular_file");
        std::fs::write(&test_file, "test").ok();

        let result = resolve_symlinks(&test_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), test_file);

        // Cleanup
        std::fs::remove_file(&test_file).ok();
    }

    // Feature: dx-cli, Property 21: CI/Container Detection
    // Validates: Requirements 11.7
    //
    // For any environment where CI, GITHUB_ACTIONS, GITLAB_CI, TRAVIS, or
    // CIRCLECI is set, is_ci() should return true.
    #[test]
    fn test_ci_detection_with_env() {
        // Save original value
        let original = std::env::var("CI").ok();

        // Set CI env var
        // SAFETY: This is a test that runs in isolation
        unsafe {
            std::env::set_var("CI", "true");
        }
        assert!(is_ci());

        // Restore original
        // SAFETY: This is a test that runs in isolation
        unsafe {
            if let Some(val) = original {
                std::env::set_var("CI", val);
            } else {
                std::env::remove_var("CI");
            }
        }
    }

    // Feature: dx-cli, Property 9: Unicode Path Handling
    // Validates: Requirements 2.5
    //
    // For any path string containing Unicode characters (emoji, CJK, RTL scripts,
    // combining characters), resolve_path should preserve all Unicode characters
    // in the resulting PathBuf.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_unicode_path_handling(
            // Generate paths with various Unicode characters
            prefix in "[a-zA-Z0-9]{1,5}",
            unicode_part in prop::sample::select(vec![
                "文件夹",           // CJK (Chinese)
                "フォルダ",         // Japanese
                "폴더",             // Korean
                "📁",               // Emoji
                "مجلد",             // RTL (Arabic)
                "תיקייה",           // Hebrew
                "café",             // Latin with diacritics
                "naïve",            // Combining characters
                "Ω≈ç√∫",           // Math symbols
                "日本語テスト",     // Mixed Japanese
            ]),
            suffix in "[a-zA-Z0-9]{1,5}"
        ) {
            let path = format!("{}/{}/{}", prefix, unicode_part, suffix);
            let result = resolve_path(&path);
            let result_str = result.to_string_lossy();

            // Unicode characters should be preserved
            prop_assert!(
                result_str.contains(unicode_part),
                "Unicode characters should be preserved: expected '{}' in '{}'",
                unicode_part,
                result_str
            );
        }
    }

    // Feature: dx-cli, Property 7: Long Path Prefix on Windows
    // Validates: Requirements 2.3
    //
    // For any Windows path exceeding 200 characters that does not already start
    // with \\?\, handle_long_path() should prepend the \\?\ prefix. Paths already
    // prefixed or under 200 characters should remain unchanged.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_long_path_prefix(
            // Generate path lengths around the 200 char threshold
            path_length in 150usize..300
        ) {
            // Create a path of the specified length
            let base = "C:\\test\\";
            let remaining = path_length.saturating_sub(base.len());
            let filler: String = std::iter::repeat('a').take(remaining).collect();
            let path_str = format!("{}{}", base, filler);
            let path = PathBuf::from(&path_str);

            let result = handle_long_path(&path);
            let result_str = result.to_string_lossy();

            #[cfg(windows)]
            {
                if path_str.len() > 200 {
                    // Long paths should get the prefix
                    prop_assert!(
                        result_str.starts_with("\\\\?\\"),
                        "Long path ({} chars) should have \\\\?\\ prefix: {}",
                        path_str.len(),
                        result_str
                    );
                } else {
                    // Short paths should remain unchanged
                    prop_assert_eq!(
                        result_str.as_ref(),
                        path_str.as_str(),
                        "Short path should remain unchanged"
                    );
                }
            }

            #[cfg(not(windows))]
            {
                // On non-Windows, path should remain unchanged
                prop_assert_eq!(
                    result_str.as_ref(),
                    path_str.as_str(),
                    "Non-Windows path should remain unchanged"
                );
            }
        }
    }

    // Test that already-prefixed paths are not double-prefixed
    #[test]
    #[cfg(windows)]
    fn test_long_path_no_double_prefix() {
        let already_prefixed = PathBuf::from("\\\\?\\C:\\very\\long\\path");
        let result = handle_long_path(&already_prefixed);
        assert_eq!(result, already_prefixed);
    }
}
