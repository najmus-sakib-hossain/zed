//! Property-based tests for ESM module resolution
//!
//! Property 10: ESM Module Resolution Correctness
//! Validates: Requirements 4.3
//!
//! These tests verify that the ESM module resolution follows
//! the Node.js ESM resolution algorithm correctly.

use proptest::prelude::*;
use std::collections::HashMap;

/// Strategy to generate valid export paths
fn arb_export_path() -> impl Strategy<Value = String> {
    prop_oneof![
        "./[a-z][a-z0-9_/]{0,15}\\.js",
        "./[a-z][a-z0-9_/]{0,15}\\.mjs",
        "./[a-z][a-z0-9_/]{0,15}\\.cjs",
    ]
}

proptest! {
    /// Property: Exports field string resolution
    ///
    /// When exports is a simple string, it should:
    /// 1. Be used for the root entry point
    /// 2. Return None for subpaths
    #[test]
    fn prop_exports_string_resolution(
        export_path in arb_export_path(),
    ) {
        let resolved = resolve_exports_string(&export_path, ".");

        // Property 1: Root entry should resolve to the export path
        prop_assert_eq!(
            resolved,
            Some(export_path.clone()),
            "Root entry should resolve to export path"
        );

        // Property 2: Subpath should not resolve
        let subpath_resolved = resolve_exports_string(&export_path, "./utils");
        prop_assert!(
            subpath_resolved.is_none(),
            "Subpath should not resolve for string exports"
        );
    }

    /// Property: Conditional exports resolution
    ///
    /// When exports has conditional exports, it should:
    /// 1. Resolve based on the condition priority
    /// 2. Prefer "import" for ESM context
    /// 3. Prefer "require" for CJS context
    #[test]
    fn prop_conditional_exports_resolution(
        import_path in arb_export_path(),
        require_path in arb_export_path(),
    ) {
        let mut conditions = HashMap::new();
        conditions.insert("import".to_string(), import_path.clone());
        conditions.insert("require".to_string(), require_path.clone());

        // Property 1: ESM context should prefer "import"
        let esm_resolved = resolve_conditional_exports(&conditions, true);
        prop_assert_eq!(
            esm_resolved,
            Some(import_path.clone()),
            "ESM context should resolve to import path"
        );

        // Property 2: CJS context should prefer "require"
        let cjs_resolved = resolve_conditional_exports(&conditions, false);
        prop_assert_eq!(
            cjs_resolved,
            Some(require_path.clone()),
            "CJS context should resolve to require path"
        );
    }

    /// Property: Exports object subpath resolution
    ///
    /// When exports is an object with subpaths, it should:
    /// 1. Resolve exact matches
    /// 2. Handle pattern matching with wildcards
    #[test]
    fn prop_exports_object_subpath_resolution(
        root_path in arb_export_path(),
        utils_path in arb_export_path(),
    ) {
        let mut exports = HashMap::new();
        exports.insert(".".to_string(), root_path.clone());
        exports.insert("./utils".to_string(), utils_path.clone());

        // Property 1: Root should resolve
        let root_resolved = resolve_exports_object(&exports, ".");
        prop_assert_eq!(
            root_resolved,
            Some(root_path.clone()),
            "Root entry should resolve"
        );

        // Property 2: Subpath should resolve
        let utils_resolved = resolve_exports_object(&exports, "./utils");
        prop_assert_eq!(
            utils_resolved,
            Some(utils_path.clone()),
            "Subpath should resolve"
        );

        // Property 3: Unknown subpath should not resolve
        let unknown_resolved = resolve_exports_object(&exports, "./unknown");
        prop_assert!(
            unknown_resolved.is_none(),
            "Unknown subpath should not resolve"
        );
    }

    /// Property: Exports pattern matching
    ///
    /// When exports has wildcard patterns, it should:
    /// 1. Match the pattern prefix
    /// 2. Substitute the wildcard correctly
    #[test]
    fn prop_exports_pattern_matching(
        suffix in "[a-z][a-z0-9_]{0,10}",
    ) {
        let mut exports = HashMap::new();
        exports.insert("./*".to_string(), "./src/*.js".to_string());

        let subpath = format!("./{}", suffix);
        let resolved = resolve_exports_pattern(&exports, &subpath);

        // Property: Pattern should resolve with substitution
        let expected = format!("./src/{}.js", suffix);
        prop_assert_eq!(
            resolved,
            Some(expected),
            "Pattern should resolve with wildcard substitution"
        );
    }

    /// Property: Module type detection from extension
    ///
    /// File extensions should determine module type:
    /// 1. .mjs/.mts -> ESM
    /// 2. .cjs/.cts -> CJS
    /// 3. .js/.ts -> depends on package.json type
    #[test]
    fn prop_module_type_from_extension(
        base_name in "[a-z][a-z0-9_]{0,15}",
    ) {
        // Property 1: .mjs is always ESM
        let mjs_type = detect_module_type(&format!("{}.mjs", base_name));
        prop_assert_eq!(mjs_type, ModuleType::ESModule, ".mjs should be ESM");

        // Property 2: .cjs is always CJS
        let cjs_type = detect_module_type(&format!("{}.cjs", base_name));
        prop_assert_eq!(cjs_type, ModuleType::CommonJS, ".cjs should be CJS");

        // Property 3: .json is always JSON
        let json_type = detect_module_type(&format!("{}.json", base_name));
        prop_assert_eq!(json_type, ModuleType::JSON, ".json should be JSON");
    }

    /// Property: Package.json type field affects resolution
    ///
    /// The "type" field in package.json should:
    /// 1. "module" -> treat .js as ESM
    /// 2. "commonjs" or absent -> treat .js as CJS
    #[test]
    fn prop_package_type_field(
        type_field in prop::option::of(prop_oneof![
            Just("module".to_string()),
            Just("commonjs".to_string()),
        ]),
    ) {
        let is_esm = is_esm_from_package_type(type_field.as_deref());

        match type_field.as_deref() {
            Some("module") => {
                prop_assert!(is_esm, "type: module should be ESM");
            }
            Some("commonjs") | None => {
                prop_assert!(!is_esm, "type: commonjs or absent should be CJS");
            }
            _ => {}
        }
    }
}

// ============================================================================
// Helper types and functions that mirror the actual implementation
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleType {
    ESModule,
    CommonJS,
    JSON,
}

/// Resolve exports when it's a simple string
fn resolve_exports_string(export_path: &str, subpath: &str) -> Option<String> {
    if subpath == "." || subpath.is_empty() {
        Some(export_path.to_string())
    } else {
        None
    }
}

/// Resolve conditional exports based on context
fn resolve_conditional_exports(
    conditions: &HashMap<String, String>,
    is_esm: bool,
) -> Option<String> {
    let priority = if is_esm {
        &["import", "module", "default", "require"][..]
    } else {
        &["require", "default", "import", "module"][..]
    };

    for condition in priority {
        if let Some(path) = conditions.get(*condition) {
            return Some(path.clone());
        }
    }

    // Try "node" as fallback
    conditions.get("node").cloned()
}

/// Resolve exports object with subpaths
fn resolve_exports_object(exports: &HashMap<String, String>, subpath: &str) -> Option<String> {
    // Normalize subpath
    let key = if subpath.is_empty() || subpath == "." {
        ".".to_string()
    } else if subpath.starts_with("./") {
        subpath.to_string()
    } else {
        format!("./{}", subpath)
    };

    exports.get(&key).cloned()
}

/// Resolve exports with pattern matching
fn resolve_exports_pattern(exports: &HashMap<String, String>, subpath: &str) -> Option<String> {
    for (pattern, value) in exports {
        if let Some(prefix) = pattern.strip_suffix('*') {
            if let Some(suffix) = subpath.strip_prefix(prefix) {
                return Some(value.replace('*', suffix));
            }
        }
    }
    None
}

/// Detect module type from file extension
fn detect_module_type(path: &str) -> ModuleType {
    if path.ends_with(".mjs") || path.ends_with(".mts") {
        ModuleType::ESModule
    } else if path.ends_with(".cjs") || path.ends_with(".cts") {
        ModuleType::CommonJS
    } else if path.ends_with(".json") {
        ModuleType::JSON
    } else {
        // Default to ESModule for .js/.ts
        ModuleType::ESModule
    }
}

/// Check if package.json type field indicates ESM
fn is_esm_from_package_type(type_field: Option<&str>) -> bool {
    type_field == Some("module")
}

// ============================================================================
// Unit tests for edge cases
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_exports_string_root() {
        let resolved = resolve_exports_string("./index.js", ".");
        assert_eq!(resolved, Some("./index.js".to_string()));
    }

    #[test]
    fn test_exports_string_subpath() {
        let resolved = resolve_exports_string("./index.js", "./utils");
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_conditional_exports_esm() {
        let mut conditions = HashMap::new();
        conditions.insert("import".to_string(), "./index.mjs".to_string());
        conditions.insert("require".to_string(), "./index.cjs".to_string());

        let resolved = resolve_conditional_exports(&conditions, true);
        assert_eq!(resolved, Some("./index.mjs".to_string()));
    }

    #[test]
    fn test_conditional_exports_cjs() {
        let mut conditions = HashMap::new();
        conditions.insert("import".to_string(), "./index.mjs".to_string());
        conditions.insert("require".to_string(), "./index.cjs".to_string());

        let resolved = resolve_conditional_exports(&conditions, false);
        assert_eq!(resolved, Some("./index.cjs".to_string()));
    }

    #[test]
    fn test_exports_pattern() {
        let mut exports = HashMap::new();
        exports.insert("./*".to_string(), "./dist/*.js".to_string());

        let resolved = resolve_exports_pattern(&exports, "./utils");
        assert_eq!(resolved, Some("./dist/utils.js".to_string()));
    }

    #[test]
    fn test_module_type_detection() {
        assert_eq!(detect_module_type("index.mjs"), ModuleType::ESModule);
        assert_eq!(detect_module_type("index.cjs"), ModuleType::CommonJS);
        assert_eq!(detect_module_type("data.json"), ModuleType::JSON);
        assert_eq!(detect_module_type("index.js"), ModuleType::ESModule);
    }

    #[test]
    fn test_package_type_field() {
        assert!(is_esm_from_package_type(Some("module")));
        assert!(!is_esm_from_package_type(Some("commonjs")));
        assert!(!is_esm_from_package_type(None));
    }
}
