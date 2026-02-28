//! Cross-platform path manipulation.
//!
//! This module provides Node.js `path` module compatibility.

use std::path::{Path, PathBuf, MAIN_SEPARATOR};

/// Platform-specific path separator.
pub const SEP: char = MAIN_SEPARATOR;

/// Platform-specific path delimiter (`:` on Unix, `;` on Windows).
#[cfg(unix)]
pub const DELIMITER: char = ':';
/// Platform-specific path delimiter (`:` on Unix, `;` on Windows).
#[cfg(windows)]
pub const DELIMITER: char = ';';

/// Join path segments.
pub fn join(paths: &[&str]) -> PathBuf {
    let mut result = PathBuf::new();
    for path in paths {
        if path.is_empty() {
            continue;
        }
        if Path::new(path).is_absolute() {
            result = PathBuf::from(path);
        } else {
            result.push(path);
        }
    }
    result
}

/// Resolve to absolute path.
pub fn resolve(paths: &[&str]) -> PathBuf {
    let mut result = std::env::current_dir().unwrap_or_default();

    for path in paths {
        if path.is_empty() {
            continue;
        }
        let p = Path::new(path);
        if p.is_absolute() {
            result = p.to_path_buf();
        } else {
            result.push(p);
        }
    }

    normalize_path(&result)
}

/// Get directory name.
pub fn dirname(path: &str) -> &str {
    Path::new(path).parent().and_then(|p| p.to_str()).unwrap_or("")
}

/// Get base name.
pub fn basename(path: &str, ext: Option<&str>) -> String {
    let name = Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("");

    if let Some(ext) = ext {
        if let Some(stripped) = name.strip_suffix(ext) {
            return stripped.to_string();
        }
    }

    name.to_string()
}

/// Get extension.
pub fn extname(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default()
}

/// Normalize path (resolve . and ..).
pub fn normalize(path: &str) -> PathBuf {
    normalize_path(Path::new(path))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !components.is_empty() {
                    components.pop();
                }
            }
            c => components.push(c),
        }
    }

    components.iter().collect()
}

/// Check if path is absolute.
pub fn is_absolute(path: &str) -> bool {
    Path::new(path).is_absolute()
}

/// Parse path into components.
pub fn parse(path: &str) -> ParsedPath {
    let p = Path::new(path);
    ParsedPath {
        root: p
            .components()
            .next()
            .and_then(|c| {
                if matches!(c, std::path::Component::Prefix(_) | std::path::Component::RootDir) {
                    Some(c.as_os_str().to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .unwrap_or_default(),
        dir: dirname(path).to_string(),
        base: basename(path, None),
        ext: extname(path),
        name: p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string(),
    }
}

/// Parsed path components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPath {
    /// Root of the path
    pub root: String,
    /// Directory portion
    pub dir: String,
    /// Base name with extension
    pub base: String,
    /// Extension with dot
    pub ext: String,
    /// Name without extension
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join() {
        let result = join(&["foo", "bar", "baz"]);
        assert!(result.to_string_lossy().contains("foo"));
        assert!(result.to_string_lossy().contains("bar"));
        assert!(result.to_string_lossy().contains("baz"));
    }

    #[test]
    fn test_dirname() {
        assert_eq!(dirname("/foo/bar/baz.txt"), "/foo/bar");
        assert_eq!(dirname("foo.txt"), "");
    }

    #[test]
    fn test_basename() {
        assert_eq!(basename("/foo/bar/baz.txt", None), "baz.txt");
        assert_eq!(basename("/foo/bar/baz.txt", Some(".txt")), "baz");
    }

    #[test]
    fn test_extname() {
        assert_eq!(extname("foo.txt"), ".txt");
        assert_eq!(extname("foo"), "");
        assert_eq!(extname("foo.bar.baz"), ".baz");
    }

    #[test]
    fn test_is_absolute() {
        #[cfg(unix)]
        {
            assert!(is_absolute("/foo/bar"));
            assert!(!is_absolute("foo/bar"));
        }
        #[cfg(windows)]
        {
            assert!(is_absolute("C:\\foo\\bar"));
            assert!(!is_absolute("foo\\bar"));
        }
    }

    #[test]
    fn test_normalize() {
        let result = normalize("foo/bar/../baz");
        assert!(result.to_string_lossy().contains("foo"));
        assert!(result.to_string_lossy().contains("baz"));
        assert!(!result.to_string_lossy().contains(".."));
    }
}
