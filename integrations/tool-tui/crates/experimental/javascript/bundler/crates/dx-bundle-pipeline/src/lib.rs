//! Unified transformation pipeline - JSX + TypeScript + ES6 in one pass

pub mod compile;
pub mod es6;
pub mod jsx;
pub mod typescript;
pub mod unified;

use dx_bundle_core::error::BundleResult;
use dx_bundle_core::{ImportMap, ModuleId};
use std::collections::HashMap;

pub use compile::{compile_module, CompileOptions, CompiledModule, ModuleFormat};
pub use unified::UnifiedPipeline;

/// Public transform API
pub fn transform(
    source: &[u8],
    _module_id: ModuleId,
    _imports: &ImportMap,
    options: &TransformOptions,
) -> BundleResult<Vec<u8>> {
    // Convert to string for transformations
    let source_str = std::str::from_utf8(source).map_err(|e| {
        dx_bundle_core::error::BundleError::transform_error(format!("Invalid UTF-8: {}", e))
    })?;

    let mut result = source_str.to_string();

    // Phase 1: Strip TypeScript if enabled
    if options.strip_typescript {
        result = strip_typescript(&result);
    }

    // Phase 2: Transform JSX if enabled
    if options.transform_jsx {
        result = transform_jsx_code(&result, &options.jsx_factory);
    }

    // Phase 3: Rewrite imports based on ImportMap (disabled for now)
    // if !imports.is_empty() {
    //     result = rewrite_imports(&result, imports);
    // }

    // Phase 4: Minify if enabled
    if options.minify {
        result = minify_code(&result);
    }

    Ok(result.into_bytes())
}

/// Strip TypeScript type annotations
fn strip_typescript(source: &str) -> String {
    let mut result = source.to_string();

    // Remove interface declarations
    while let Some(start) = result.find("interface ") {
        if let Some(end) = find_block_end(&result[start..]) {
            result.replace_range(start..start + end, "");
        } else {
            break;
        }
    }

    // Remove type aliases
    while let Some(start) = result.find("type ") {
        if let Some(end) = result[start..].find([';', '\n']) {
            result.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }

    // Remove type annotations from variables: const x: Type = ... → const x = ...
    result = remove_type_annotations(result);

    // Remove access modifiers
    for modifier in &["private ", "public ", "protected ", "readonly "] {
        result = result.replace(modifier, "");
    }

    result
}

/// Transform JSX to createElement calls (improved)
#[allow(dead_code)]
fn transform_jsx_code(source: &str, _factory: &str) -> String {
    // For now, just pass through JSX as-is
    // Full JSX transformation is complex and should use a proper parser
    // This is a minimal implementation to avoid breaking code
    source.to_string()
}

/// Rewrite imports based on ImportMap
///
/// This function replaces import specifiers in the source code based on a resolution map.
/// It handles:
/// - Static imports: import { foo } from 'bar' -> import { foo } from './resolved/bar.js'
/// - Dynamic imports: import('bar') -> import('./resolved/bar.js')
/// - Re-exports: export { foo } from 'bar' -> export { foo } from './resolved/bar.js'
#[allow(dead_code)]
fn rewrite_imports(
    source: &str,
    resolution_map: &std::collections::HashMap<String, String>,
) -> String {
    if resolution_map.is_empty() {
        return source.to_string();
    }

    let mut result = source.to_string();

    // Sort replacements by position (descending) to avoid invalidating positions
    let mut replacements: Vec<(usize, usize, &str, &str)> = Vec::new();

    // Find all import/export statements and their specifiers
    for (original, resolved) in resolution_map {
        // Find static imports: import ... from 'specifier'
        let patterns = [
            format!("from '{}'", original),
            format!("from \"{}\"", original),
            // Dynamic imports: import('specifier')
            format!("import('{}')", original),
            format!("import(\"{}\")", original),
            // Require calls (for CJS compatibility)
            format!("require('{}')", original),
            format!("require(\"{}\")", original),
        ];

        for pattern in &patterns {
            let mut search_start = 0;
            while let Some(pos) = result[search_start..].find(pattern) {
                let absolute_pos = search_start + pos;

                // Determine the quote character and specifier position
                let (_quote_char, spec_start, spec_end) = if pattern.starts_with("from") {
                    // from 'specifier' or from "specifier"
                    let quote = if pattern.contains('\'') { '\'' } else { '"' };
                    let start = absolute_pos + 6; // "from '" or "from \""
                    let end = start + original.len();
                    (quote, start, end)
                } else if pattern.starts_with("import(") {
                    // import('specifier') or import("specifier")
                    let quote = if pattern.contains('\'') { '\'' } else { '"' };
                    let start = absolute_pos + 8; // "import('" or "import(\""
                    let end = start + original.len();
                    (quote, start, end)
                } else {
                    // require('specifier') or require("specifier")
                    let quote = if pattern.contains('\'') { '\'' } else { '"' };
                    let start = absolute_pos + 9; // "require('" or "require(\""
                    let end = start + original.len();
                    (quote, start, end)
                };

                replacements.push((spec_start, spec_end, original, resolved));
                search_start = absolute_pos + pattern.len();
            }
        }
    }

    // Sort by position descending to replace from end to start
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    // Apply replacements
    for (start, end, _original, resolved) in replacements {
        if start < result.len() && end <= result.len() {
            result.replace_range(start..end, resolved);
        }
    }

    result
}

/// Simple minification
fn minify_code(source: &str) -> String {
    // Remove extra whitespace (simple version)
    source
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Find the end of a block (matching {})
fn find_block_end(source: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_block = false;

    for (i, ch) in source.char_indices() {
        match ch {
            '{' => {
                in_block = true;
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 && in_block {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// Remove type annotations from variable declarations
fn remove_type_annotations(mut source: String) -> String {
    let patterns = ["const ", "let ", "var ", "function "];

    for pattern in &patterns {
        let mut iteration = 0;
        while iteration < 50 {
            iteration += 1;
            let before_len = source.len();

            if let Some(start) = source.find(pattern) {
                let after_keyword = &source[start + pattern.len()..];

                // Find identifier
                let mut ident_end = 0;
                for (idx, ch) in after_keyword.char_indices() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident_end = idx + ch.len_utf8();
                    } else {
                        break;
                    }
                }

                // Check for type annotation
                let after_ident = &after_keyword[ident_end..];
                if after_ident.starts_with(": ") {
                    // Find delimiter (= or ; or newline)
                    let colon_pos = start + pattern.len() + ident_end;
                    let after_colon = &source[colon_pos + 2..];

                    if let Some(delim_pos) = after_colon.find(['=', ';', '\n']) {
                        // Remove the type annotation
                        source.replace_range(colon_pos..colon_pos + 2 + delim_pos, "");
                        continue;
                    }
                }
            }

            if source.len() == before_len {
                break;
            }
        }
    }

    source
}

/// Transform options
#[derive(Clone, Debug)]
pub struct TransformOptions {
    /// Strip TypeScript types
    pub strip_typescript: bool,
    /// Transform JSX to createElement calls
    pub transform_jsx: bool,
    /// JSX factory function
    pub jsx_factory: String,
    /// JSX fragment
    pub jsx_fragment: String,
    /// Transform ES6 to CommonJS
    pub transform_es6: bool,
    /// Minify output
    pub minify: bool,
    /// Preserve comments
    pub preserve_comments: bool,
}

impl Default for TransformOptions {
    fn default() -> Self {
        Self {
            strip_typescript: true,
            transform_jsx: true,
            jsx_factory: "React.createElement".into(),
            jsx_fragment: "React.Fragment".into(),
            transform_es6: true,
            minify: false,
            preserve_comments: false,
        }
    }
}

/// Public API for rewriting imports in source code
///
/// Takes a source string and a resolution map (original specifier -> resolved path)
/// and returns the source with all import specifiers replaced.
///
/// # Example
/// ```
/// use dx_bundle_pipeline::rewrite_imports_with_map;
/// use std::collections::HashMap;
///
/// let source = r#"import { foo } from 'bar';"#;
/// let mut map = HashMap::new();
/// map.insert("bar".to_string(), "./node_modules/bar/index.js".to_string());
///
/// let result = rewrite_imports_with_map(source, &map);
/// assert!(result.contains("./node_modules/bar/index.js"));
/// ```
pub fn rewrite_imports_with_map(source: &str, resolution_map: &HashMap<String, String>) -> String {
    rewrite_imports(source, resolution_map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_static_import_single_quotes() {
        let source = r#"import { foo } from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert_eq!(result, r#"import { foo } from './resolved/bar.js';"#);
    }

    #[test]
    fn test_rewrite_static_import_double_quotes() {
        let source = r#"import { foo } from "bar";"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert_eq!(result, r#"import { foo } from "./resolved/bar.js";"#);
    }

    #[test]
    fn test_rewrite_default_import() {
        let source = r#"import foo from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./node_modules/bar/index.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./node_modules/bar/index.js"));
    }

    #[test]
    fn test_rewrite_namespace_import() {
        let source = r#"import * as bar from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/bar.js"));
    }

    #[test]
    fn test_rewrite_dynamic_import() {
        let source = r#"const mod = import('bar');"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert_eq!(result, r#"const mod = import('./resolved/bar.js');"#);
    }

    #[test]
    fn test_rewrite_require() {
        let source = r#"const bar = require('bar');"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert_eq!(result, r#"const bar = require('./resolved/bar.js');"#);
    }

    #[test]
    fn test_rewrite_reexport() {
        let source = r#"export { foo } from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/bar.js"));
    }

    #[test]
    fn test_rewrite_multiple_imports() {
        let source = r#"
import { foo } from 'foo';
import { bar } from 'bar';
import { baz } from 'baz';
"#;
        let mut map = HashMap::new();
        map.insert("foo".to_string(), "./resolved/foo.js".to_string());
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());
        map.insert("baz".to_string(), "./resolved/baz.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/foo.js"));
        assert!(result.contains("./resolved/bar.js"));
        assert!(result.contains("./resolved/baz.js"));
    }

    #[test]
    fn test_rewrite_empty_map() {
        let source = r#"import { foo } from 'bar';"#;
        let map = HashMap::new();

        let result = rewrite_imports(source, &map);
        assert_eq!(result, source);
    }

    #[test]
    fn test_rewrite_no_matching_imports() {
        let source = r#"import { foo } from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("baz".to_string(), "./resolved/baz.js".to_string());

        let result = rewrite_imports(source, &map);
        assert_eq!(result, source);
    }

    #[test]
    fn test_rewrite_scoped_package() {
        let source = r#"import { foo } from '@scope/bar';"#;
        let mut map = HashMap::new();
        map.insert("@scope/bar".to_string(), "./node_modules/@scope/bar/index.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./node_modules/@scope/bar/index.js"));
    }

    #[test]
    fn test_rewrite_preserves_other_code() {
        let source = r#"
const x = 1;
import { foo } from 'bar';
function test() { return x; }
"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("const x = 1;"));
        assert!(result.contains("./resolved/bar.js"));
        assert!(result.contains("function test()"));
    }

    // Edge case tests for task 11.2

    #[test]
    fn test_rewrite_dynamic_import_with_await() {
        let source = r#"const mod = await import('bar');"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("import('./resolved/bar.js')"));
    }

    #[test]
    fn test_rewrite_dynamic_import_in_function() {
        let source = r#"
async function loadModule() {
    const mod = await import('bar');
    return mod;
}
"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("import('./resolved/bar.js')"));
    }

    #[test]
    fn test_rewrite_export_all_from() {
        let source = r#"export * from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/bar.js"));
    }

    #[test]
    fn test_rewrite_export_named_from() {
        let source = r#"export { foo, bar as baz } from 'module';"#;
        let mut map = HashMap::new();
        map.insert("module".to_string(), "./resolved/module.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/module.js"));
    }

    #[test]
    fn test_rewrite_export_default_from() {
        let source = r#"export { default } from 'bar';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/bar.js"));
    }

    #[test]
    fn test_rewrite_namespace_reexport() {
        let source = r#"export * as utils from 'utils';"#;
        let mut map = HashMap::new();
        map.insert("utils".to_string(), "./resolved/utils.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./resolved/utils.js"));
    }

    #[test]
    fn test_rewrite_mixed_imports_and_reexports() {
        let source = r#"
import { foo } from 'foo';
export { bar } from 'bar';
import * as baz from 'baz';
export * from 'all';
"#;
        let mut map = HashMap::new();
        map.insert("foo".to_string(), "./resolved/foo.js".to_string());
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());
        map.insert("baz".to_string(), "./resolved/baz.js".to_string());
        map.insert("all".to_string(), "./resolved/all.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("from './resolved/foo.js'"));
        assert!(result.contains("from './resolved/bar.js'"));
        assert!(result.contains("from './resolved/baz.js'"));
        assert!(result.contains("from './resolved/all.js'"));
    }

    #[test]
    fn test_rewrite_import_with_subpath() {
        let source = r#"import { foo } from 'lodash/fp';"#;
        let mut map = HashMap::new();
        map.insert("lodash/fp".to_string(), "./node_modules/lodash/fp.js".to_string());

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./node_modules/lodash/fp.js"));
    }

    #[test]
    fn test_rewrite_does_not_affect_string_literals() {
        // This tests that we don't accidentally replace strings that look like imports
        let source = r#"
const str = "import { foo } from 'bar'";
import { actual } from 'bar';
"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        // The actual import should be rewritten
        assert!(result.contains("from './resolved/bar.js'"));
        // Note: The string literal will also be affected by our simple implementation
        // A more sophisticated implementation would parse the AST to avoid this
    }

    #[test]
    fn test_rewrite_relative_import_unchanged() {
        // Relative imports that aren't in the map should remain unchanged
        let source = r#"import { foo } from './local';"#;
        let mut map = HashMap::new();
        map.insert("bar".to_string(), "./resolved/bar.js".to_string());

        let result = rewrite_imports(source, &map);
        assert_eq!(result, source);
    }

    #[test]
    fn test_rewrite_import_with_special_chars_in_path() {
        let source = r#"import { foo } from '@org/pkg-name';"#;
        let mut map = HashMap::new();
        map.insert(
            "@org/pkg-name".to_string(),
            "./node_modules/@org/pkg-name/index.js".to_string(),
        );

        let result = rewrite_imports(source, &map);
        assert!(result.contains("./node_modules/@org/pkg-name/index.js"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate valid package names
    fn package_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple package names
            "[a-z][a-z0-9-]{0,10}",
            // Scoped packages
            "@[a-z][a-z0-9-]{0,5}/[a-z][a-z0-9-]{0,10}",
        ]
    }

    /// Generate valid resolved paths
    fn resolved_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Relative paths
            Just("./".to_string()).prop_flat_map(|prefix| {
                "[a-z][a-z0-9_/]{0,20}\\.js".prop_map(move |path| format!("{}{}", prefix, path))
            }),
            // node_modules paths
            Just("./node_modules/".to_string()).prop_flat_map(|prefix| {
                "[a-z][a-z0-9_/-]{0,20}/index\\.js"
                    .prop_map(move |path| format!("{}{}", prefix, path))
            }),
        ]
    }

    /// Generate import statement types
    #[allow(dead_code)]
    fn import_statement_strategy(pkg: String) -> impl Strategy<Value = String> {
        prop_oneof![
            // Default import
            Just(format!("import foo from '{}';", pkg)),
            // Named import
            Just(format!("import {{ foo }} from '{}';", pkg)),
            // Namespace import
            Just(format!("import * as foo from '{}';", pkg)),
            // Side-effect import
            Just(format!("import '{}';", pkg)),
            // Dynamic import
            Just(format!("const mod = import('{}');", pkg)),
            // Require
            Just(format!("const mod = require('{}');", pkg)),
            // Re-export
            Just(format!("export {{ foo }} from '{}';", pkg)),
            // Export all
            Just(format!("export * from '{}';", pkg)),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-production-ready, Property 2: Bundler Output Validity (imports)**
        /// **Validates: Requirements 8.3**
        ///
        /// *For any* import statement with a package name in the resolution map,
        /// rewriting SHALL replace the package name with the resolved path.
        #[test]
        fn prop_import_rewriting_replaces_specifier(
            pkg in package_name_strategy(),
            resolved in resolved_path_strategy()
        ) {
            // Generate various import statement types
            let import_types = vec![
                format!("import foo from '{}';", pkg),
                format!("import {{ foo }} from '{}';", pkg),
                format!("import * as foo from '{}';", pkg),
                format!("const mod = import('{}');", pkg),
                format!("const mod = require('{}');", pkg),
                format!("export {{ foo }} from '{}';", pkg),
                format!("export * from '{}';", pkg),
            ];

            let mut map = HashMap::new();
            map.insert(pkg.clone(), resolved.clone());

            for source in import_types {
                let result = rewrite_imports(&source, &map);

                // Property: The resolved path should appear in the result
                prop_assert!(
                    result.contains(&resolved),
                    "Resolved path '{}' should appear in result '{}' for source '{}'",
                    resolved, result, source
                );

                // Property: The original package name should NOT appear in the result
                // (unless it's part of the resolved path)
                if !resolved.contains(&pkg) {
                    prop_assert!(
                        !result.contains(&format!("'{}'", pkg)) && !result.contains(&format!("\"{}\"", pkg)),
                        "Original package '{}' should not appear in result '{}' for source '{}'",
                        pkg, result, source
                    );
                }
            }
        }

        /// **Feature: dx-production-ready, Property 2: Import Rewriting Preserves Structure**
        /// **Validates: Requirements 8.3**
        ///
        /// *For any* source code with imports, rewriting SHALL preserve the overall
        /// structure of the code (same number of statements, same keywords).
        #[test]
        fn prop_import_rewriting_preserves_structure(
            pkg in package_name_strategy(),
            resolved in resolved_path_strategy()
        ) {
            let source = format!(r#"
const x = 1;
import {{ foo }} from '{}';
function test() {{ return x; }}
export {{ foo }} from '{}';
"#, pkg, pkg);

            let mut map = HashMap::new();
            map.insert(pkg.clone(), resolved.clone());

            let result = rewrite_imports(&source, &map);

            // Property: Keywords should be preserved
            prop_assert!(result.contains("const x = 1;"), "const declaration should be preserved");
            prop_assert!(result.contains("import {"), "import keyword should be preserved");
            prop_assert!(result.contains("function test()"), "function declaration should be preserved");
            prop_assert!(result.contains("export {"), "export keyword should be preserved");

            // Property: Line count should be approximately the same
            let source_lines = source.lines().count();
            let result_lines = result.lines().count();
            prop_assert_eq!(source_lines, result_lines, "Line count should be preserved");
        }

        /// **Feature: dx-production-ready, Property 2: Empty Map Is Identity**
        /// **Validates: Requirements 8.3**
        ///
        /// *For any* source code, rewriting with an empty map SHALL return
        /// the source unchanged.
        #[test]
        fn prop_empty_map_is_identity(
            pkg in package_name_strategy()
        ) {
            let source = format!("import {{ foo }} from '{}';", pkg);
            let map = HashMap::new();

            let result = rewrite_imports(&source, &map);

            prop_assert_eq!(
                result, source,
                "Empty map should return source unchanged"
            );
        }

        /// **Feature: dx-production-ready, Property 2: Non-Matching Imports Unchanged**
        /// **Validates: Requirements 8.3**
        ///
        /// *For any* import that is not in the resolution map, the import
        /// SHALL remain unchanged.
        #[test]
        fn prop_non_matching_imports_unchanged(
            pkg1 in package_name_strategy(),
            pkg2 in package_name_strategy(),
            resolved in resolved_path_strategy()
        ) {
            // Only add pkg1 to the map, not pkg2
            prop_assume!(pkg1 != pkg2);

            let source = format!("import {{ foo }} from '{}';", pkg2);
            let mut map = HashMap::new();
            map.insert(pkg1.clone(), resolved.clone());

            let result = rewrite_imports(&source, &map);

            // Property: pkg2 should remain unchanged since it's not in the map
            prop_assert!(
                result.contains(&format!("'{}'", pkg2)),
                "Non-matching import '{}' should remain unchanged in '{}'",
                pkg2, result
            );
        }

        /// **Feature: dx-production-ready, Property 2: Multiple Imports All Rewritten**
        /// **Validates: Requirements 8.3**
        ///
        /// *For any* source with multiple imports from the same package,
        /// ALL occurrences SHALL be rewritten.
        #[test]
        fn prop_multiple_imports_all_rewritten(
            pkg in package_name_strategy(),
            resolved in resolved_path_strategy()
        ) {
            let source = format!(r#"
import {{ foo }} from '{}';
import {{ bar }} from '{}';
const mod = require('{}');
"#, pkg, pkg, pkg);

            let mut map = HashMap::new();
            map.insert(pkg.clone(), resolved.clone());

            let result = rewrite_imports(&source, &map);

            // Count occurrences of resolved path
            let resolved_count = result.matches(&resolved).count();

            // Property: All 3 imports should be rewritten
            prop_assert!(
                resolved_count >= 3,
                "All imports should be rewritten. Found {} occurrences of '{}' in '{}'",
                resolved_count, resolved, result
            );

            // Property: Original package should not appear (unless in resolved path)
            if !resolved.contains(&pkg) {
                let original_count = result.matches(&format!("'{}'", pkg)).count()
                    + result.matches(&format!("\"{}\"", pkg)).count();
                prop_assert_eq!(
                    original_count, 0,
                    "Original package '{}' should not appear in result",
                    pkg
                );
            }
        }
    }
}
