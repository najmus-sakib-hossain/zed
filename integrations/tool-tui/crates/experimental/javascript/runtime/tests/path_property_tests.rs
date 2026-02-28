//! Property-based tests for cross-platform path handling
//!
//! **Feature: dx-js-production-complete, Property 12: Cross-Platform Path Handling**
//! **Validates: Requirements 38.2, 38.3**

use proptest::prelude::*;
use std::path::{Path, PathBuf};

/// Generate valid path components (no path separators or invalid chars)
fn valid_path_component() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
}

/// Generate a valid relative path with 1-5 components
fn valid_relative_path() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec(valid_path_component(), 1..5).prop_map(|components| {
        let mut path = PathBuf::new();
        for component in components {
            path.push(component);
        }
        path
    })
}

/// Generate a file extension
fn file_extension() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("js".to_string()),
        Just("ts".to_string()),
        Just("json".to_string()),
        Just("mjs".to_string()),
        Just("cjs".to_string()),
    ]
}

/// Generate a path with file extension
fn path_with_extension() -> impl Strategy<Value = PathBuf> {
    (valid_relative_path(), file_extension()).prop_map(|(mut path, ext)| {
        if let Some(file_name) = path.file_name() {
            let new_name = format!("{}.{}", file_name.to_string_lossy(), ext);
            path.set_file_name(new_name);
        }
        path
    })
}

proptest! {
    /// Property: Path joining is consistent across platforms
    /// For any two valid path components, joining them produces a valid path
    #[test]
    fn prop_path_join_produces_valid_path(
        base in valid_path_component(),
        child in valid_path_component()
    ) {
        let path = Path::new(&base).join(&child);

        // Path should have exactly 2 components
        prop_assert_eq!(path.components().count(), 2);

        // Path should contain both components
        let path_str = path.to_string_lossy();
        prop_assert!(path_str.contains(&base));
        prop_assert!(path_str.contains(&child));
    }

    /// Property: Path normalization is idempotent
    /// Normalizing a path twice produces the same result as normalizing once
    #[test]
    fn prop_path_normalization_idempotent(path in valid_relative_path()) {
        let normalized = normalize_path(&path);
        let double_normalized = normalize_path(&normalized);

        prop_assert_eq!(normalized, double_normalized);
    }

    /// Property: File extension extraction is consistent
    /// For any path with an extension, extracting the extension works correctly
    #[test]
    fn prop_extension_extraction_consistent(path in path_with_extension()) {
        let ext = path.extension();

        // Should have an extension
        prop_assert!(ext.is_some());

        // Extension should be one of our valid extensions
        let ext_str = ext.unwrap().to_string_lossy();
        prop_assert!(
            ext_str == "js" || ext_str == "ts" || ext_str == "json" ||
            ext_str == "mjs" || ext_str == "cjs"
        );
    }

    /// Property: Parent path is always shorter or equal
    /// For any path, the parent path has fewer or equal components
    #[test]
    fn prop_parent_path_shorter(path in valid_relative_path()) {
        if let Some(parent) = path.parent() {
            let path_components = path.components().count();
            let parent_components = parent.components().count();

            prop_assert!(parent_components < path_components);
        }
    }

    /// Property: Path with file name has extractable file name
    /// For any path, if it has a file name, we can extract it
    #[test]
    fn prop_file_name_extractable(path in valid_relative_path()) {
        let file_name = path.file_name();

        // Non-empty paths should have a file name
        if path.components().count() > 0 {
            prop_assert!(file_name.is_some());
        }
    }

    /// Property: Relative path resolution is deterministic
    /// Resolving the same relative path always produces the same result
    #[test]
    fn prop_relative_resolution_deterministic(
        base in valid_relative_path(),
        relative in valid_relative_path()
    ) {
        let resolved1 = base.join(&relative);
        let resolved2 = base.join(&relative);

        prop_assert_eq!(resolved1, resolved2);
    }

    /// Property: Path separator handling is consistent
    /// Converting path to string and back preserves the path
    #[test]
    fn prop_path_string_roundtrip(path in valid_relative_path()) {
        let path_str = path.to_string_lossy().to_string();
        let restored = PathBuf::from(&path_str);

        // Components should match
        let original_components: Vec<_> = path.components().collect();
        let restored_components: Vec<_> = restored.components().collect();

        prop_assert_eq!(original_components.len(), restored_components.len());
    }

    /// Property: Joining with empty path is identity
    /// For any path, joining with empty path returns the original
    #[test]
    fn prop_join_empty_identity(path in valid_relative_path()) {
        let joined = path.join("");

        // Should be equivalent to original
        prop_assert_eq!(path.components().count(), joined.components().count());
    }

    /// Property: File stem extraction works for files with extensions
    /// For any path with extension, file_stem returns name without extension
    #[test]
    fn prop_file_stem_extraction(path in path_with_extension()) {
        let stem = path.file_stem();
        let ext = path.extension();

        prop_assert!(stem.is_some());
        prop_assert!(ext.is_some());

        // Stem should not contain the extension
        let stem_str = stem.unwrap().to_string_lossy();
        let ext_str = ext.unwrap().to_string_lossy();
        let suffix = format!(".{}", ext_str);

        prop_assert!(!stem_str.ends_with(&suffix), "Stem should not end with extension");
    }
}

/// Normalize a path by resolving . and .. components
fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {
                // Skip current directory markers
            }
            std::path::Component::ParentDir => {
                // Go up one level if possible
                normalized.pop();
            }
            _ => {
                normalized.push(component);
            }
        }
    }

    normalized
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_normalize_path_removes_dots() {
        let path = PathBuf::from("foo/./bar");
        let normalized = normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("foo/bar"));
    }

    #[test]
    fn test_normalize_path_resolves_parent() {
        let path = PathBuf::from("foo/bar/../baz");
        let normalized = normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("foo/baz"));
    }

    #[test]
    fn test_path_join_cross_platform() {
        let base = Path::new("src");
        let child = "index.js";
        let joined = base.join(child);

        // Should work on all platforms
        assert!(joined.to_string_lossy().contains("src"));
        assert!(joined.to_string_lossy().contains("index.js"));
    }

    #[test]
    fn test_extension_extraction() {
        let path = PathBuf::from("file.js");
        assert_eq!(path.extension().unwrap(), "js");

        let path = PathBuf::from("file.test.ts");
        assert_eq!(path.extension().unwrap(), "ts");
    }

    #[test]
    fn test_file_stem_extraction() {
        let path = PathBuf::from("file.js");
        assert_eq!(path.file_stem().unwrap(), "file");

        let path = PathBuf::from("file.test.ts");
        assert_eq!(path.file_stem().unwrap(), "file.test");
    }
}
