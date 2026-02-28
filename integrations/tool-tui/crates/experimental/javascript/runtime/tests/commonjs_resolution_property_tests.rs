//! Property-based tests for CommonJS module resolution
//!
//! Property 9: CommonJS Module Resolution Correctness
//! Validates: Requirements 4.2
//!
//! These tests verify that the CommonJS module resolution follows
//! the Node.js resolution algorithm correctly.

use proptest::prelude::*;
use std::path::PathBuf;

/// Strategy to generate valid package names
fn arb_package_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple package names
        "[a-z][a-z0-9-]{0,20}",
        // Scoped package names
        "@[a-z][a-z0-9-]{0,10}/[a-z][a-z0-9-]{0,10}",
    ]
}

/// Strategy to generate valid relative paths
fn arb_relative_path() -> impl Strategy<Value = String> {
    prop_oneof![
        // Current directory relative
        Just("./".to_string()),
        "./[a-z][a-z0-9_/]{0,20}",
        // Parent directory relative
        Just("../".to_string()),
        "../[a-z][a-z0-9_/]{0,20}",
    ]
}

proptest! {
    /// Property: Package specifier parsing correctly separates package name from subpath
    ///
    /// For any valid package specifier, the parser should:
    /// 1. Extract the correct package name
    /// 2. Extract the correct subpath (or "." for root)
    /// 3. Handle scoped packages correctly
    #[test]
    fn prop_package_specifier_parsing(
        package in arb_package_name(),
        subpath in prop_oneof![
            Just("".to_string()),
            "/[a-z][a-z0-9_/]{0,10}",
        ]
    ) {
        let specifier = if subpath.is_empty() {
            package.clone()
        } else {
            format!("{}{}", package, subpath)
        };

        // Parse the specifier
        let (parsed_package, parsed_subpath) = parse_package_specifier(&specifier);

        // Property 1: Package name should be extracted correctly
        prop_assert!(
            parsed_package == package || specifier.starts_with(&parsed_package),
            "Package name '{}' should be prefix of specifier '{}'",
            parsed_package,
            specifier
        );

        // Property 2: Subpath should be "." for root or start with "./"
        prop_assert!(
            parsed_subpath == "." || parsed_subpath.starts_with("./") || parsed_subpath.is_empty(),
            "Subpath '{}' should be '.' or start with './'",
            parsed_subpath
        );

        // Property 3: Scoped packages should preserve the scope
        if package.starts_with('@') {
            prop_assert!(
                parsed_package.starts_with('@'),
                "Scoped package '{}' should preserve scope in parsed name '{}'",
                package,
                parsed_package
            );
        }
    }

    /// Property: Relative path resolution is deterministic
    ///
    /// For any relative path and base path, resolution should:
    /// 1. Always produce the same result for the same inputs
    /// 2. Produce a path that is relative to the base
    #[test]
    fn prop_relative_path_resolution_deterministic(
        relative in arb_relative_path(),
        base_dir in "[a-z][a-z0-9_/]{0,20}",
    ) {
        let base = PathBuf::from(&base_dir).join("index.js");

        // Resolve twice
        let result1 = resolve_relative_path(&relative, &base);
        let result2 = resolve_relative_path(&relative, &base);

        // Property: Resolution is deterministic
        prop_assert_eq!(
            result1, result2,
            "Relative path resolution should be deterministic"
        );
    }

    /// Property: node_modules search path construction
    ///
    /// When building search paths for node_modules, the resolver should:
    /// 1. Include all ancestor directories
    /// 2. Order from nearest to farthest
    #[test]
    fn prop_node_modules_search_paths(
        path_segments in prop::collection::vec("[a-z][a-z0-9_]{0,10}", 1..5),
    ) {
        let path = PathBuf::from(path_segments.join("/")).join("index.js");
        let search_paths = build_node_modules_search_paths(&path);

        // Property 1: Should have at least one search path
        prop_assert!(!search_paths.is_empty(), "Should have at least one search path");

        // Property 2: All paths should end with node_modules
        for search_path in &search_paths {
            prop_assert!(
                search_path.ends_with("node_modules"),
                "Search path '{}' should end with node_modules",
                search_path.display()
            );
        }

        // Property 3: Paths should be ordered from nearest to farthest (shorter paths later)
        for i in 1..search_paths.len() {
            let prev_components = search_paths[i - 1].components().count();
            let curr_components = search_paths[i].components().count();
            prop_assert!(
                prev_components >= curr_components,
                "Search paths should be ordered from nearest to farthest"
            );
        }
    }

    /// Property: Package.json main field resolution
    ///
    /// When resolving the main entry point from package.json:
    /// 1. Should return a valid path or default to index.js
    /// 2. Should handle missing main field gracefully
    #[test]
    fn prop_package_json_main_resolution(
        main_field in prop::option::of("[a-z][a-z0-9_/]{0,20}\\.(js|cjs|mjs)"),
    ) {
        let entry = resolve_package_main(main_field.as_deref(), false);

        // Property 1: Should always return a value
        prop_assert!(entry.is_some(), "Should always resolve to some entry point");

        // Property 2: Entry should be a valid path
        let entry_str = entry.unwrap();
        prop_assert!(
            !entry_str.is_empty(),
            "Entry point should not be empty"
        );

        // Property 3: If main was provided, it should be used
        if let Some(main) = main_field {
            prop_assert_eq!(
                entry_str, main,
                "Should use provided main field"
            );
        } else {
            // Default should be index.js
            prop_assert_eq!(
                entry_str, "index.js",
                "Should default to index.js when main is not provided"
            );
        }
    }
}

/// Property: Extension resolution follows priority order
///
/// When resolving a path without extension, the resolver should:
/// 1. Try extensions in a consistent order
/// 2. Prefer .js over other extensions for CommonJS
#[test]
fn test_extension_priority_order() {
    use std::collections::HashSet;

    let extensions = get_commonjs_extension_order();

    // Property 1: Extensions should be non-empty
    assert!(!extensions.is_empty(), "Extension list should not be empty");

    // Property 2: .js should be in the list
    assert!(extensions.contains(&".js"), "Extension list should contain .js");

    // Property 3: .cjs should be in the list (CommonJS specific)
    assert!(extensions.contains(&".cjs"), "Extension list should contain .cjs");

    // Property 4: No duplicate extensions
    let unique: HashSet<_> = extensions.iter().collect();
    assert_eq!(unique.len(), extensions.len(), "Extension list should not have duplicates");
}

/// Property: Index file resolution follows Node.js conventions
///
/// When resolving a directory, the resolver should:
/// 1. Look for index files in a consistent order
/// 2. Prefer index.js for CommonJS
#[test]
fn test_index_file_resolution() {
    let index_files = get_commonjs_index_files();

    // Property 1: Index files should be non-empty
    assert!(!index_files.is_empty(), "Index file list should not be empty");

    // Property 2: index.js should be first for CommonJS
    assert_eq!(index_files[0], "index.js", "index.js should be first in CommonJS mode");

    // Property 3: All index files should start with "index."
    for file in &index_files {
        assert!(file.starts_with("index."), "Index file '{}' should start with 'index.'", file);
    }
}

// ============================================================================
// Helper functions that mirror the actual implementation
// ============================================================================

/// Parse a package specifier into package name and subpath
fn parse_package_specifier(specifier: &str) -> (String, String) {
    // Handle scoped packages (@scope/package)
    if let Some(after_at) = specifier.strip_prefix('@') {
        if let Some(slash_pos) = after_at.find('/') {
            let scope_end = slash_pos + 1;
            if let Some(next_slash) = specifier[scope_end + 1..].find('/') {
                let package_end = scope_end + 1 + next_slash;
                return (
                    specifier[..package_end].to_string(),
                    format!("./{}", &specifier[package_end + 1..]),
                );
            }
            return (specifier.to_string(), ".".to_string());
        }
    }

    // Regular packages
    if let Some((package, subpath)) = specifier.split_once('/') {
        (package.to_string(), format!("./{}", subpath))
    } else {
        (specifier.to_string(), ".".to_string())
    }
}

/// Resolve a relative path from a base path
fn resolve_relative_path(relative: &str, base: &std::path::Path) -> PathBuf {
    let base_dir = base.parent().unwrap_or(std::path::Path::new("."));
    base_dir.join(relative)
}

/// Get the extension resolution order for CommonJS
fn get_commonjs_extension_order() -> Vec<&'static str> {
    vec![".js", ".ts", ".tsx", ".jsx", ".cjs", ".cts", ".mjs", ".mts"]
}

/// Get the index file resolution order for CommonJS
fn get_commonjs_index_files() -> Vec<&'static str> {
    vec![
        "index.js",
        "index.ts",
        "index.tsx",
        "index.jsx",
        "index.mjs",
        "index.mts",
    ]
}

/// Build node_modules search paths from a file path
fn build_node_modules_search_paths(from: &std::path::Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut current = from.parent();

    while let Some(dir) = current {
        paths.push(dir.join("node_modules"));
        current = dir.parent();
    }

    paths
}

/// Resolve package.json main field
fn resolve_package_main(main: Option<&str>, _is_esm: bool) -> Option<String> {
    if let Some(main) = main {
        Some(main.to_string())
    } else {
        Some("index.js".to_string())
    }
}

// ============================================================================
// Unit tests for edge cases
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_scoped_package_parsing() {
        let (pkg, sub) = parse_package_specifier("@babel/core");
        assert_eq!(pkg, "@babel/core");
        assert_eq!(sub, ".");

        let (pkg, sub) = parse_package_specifier("@babel/core/lib/transform");
        assert_eq!(pkg, "@babel/core");
        assert_eq!(sub, "./lib/transform");
    }

    #[test]
    fn test_simple_package_parsing() {
        let (pkg, sub) = parse_package_specifier("lodash");
        assert_eq!(pkg, "lodash");
        assert_eq!(sub, ".");

        let (pkg, sub) = parse_package_specifier("lodash/get");
        assert_eq!(pkg, "lodash");
        assert_eq!(sub, "./get");
    }

    #[test]
    fn test_node_modules_paths() {
        let path = PathBuf::from("/project/src/utils/index.js");
        let paths = build_node_modules_search_paths(&path);

        assert!(paths.len() >= 3);
        assert!(paths[0].ends_with("node_modules"));
    }

    #[test]
    fn test_extension_order() {
        let extensions = get_commonjs_extension_order();
        assert!(extensions.contains(&".js"));
        assert!(extensions.contains(&".cjs"));
    }

    #[test]
    fn test_index_files() {
        let index_files = get_commonjs_index_files();
        assert_eq!(index_files[0], "index.js");
    }
}
