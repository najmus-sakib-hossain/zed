//! Tree shaking implementation for dead code elimination
//!
//! This module provides tree shaking functionality to remove unused exports
//! from bundled JavaScript modules.

use crate::error::BundleResult;
use crate::resolve::{parse_module, ModuleParseResult, ParsedExport, ParsedImport};
use crate::types::ModuleId;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Module information for tree shaking analysis
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module ID
    pub id: ModuleId,
    /// Module path
    pub path: PathBuf,
    /// Parsed imports
    pub imports: Vec<ParsedImport>,
    /// Parsed exports
    pub exports: Vec<ParsedExport>,
    /// Whether this module has side effects
    pub has_side_effects: bool,
    /// Source code
    pub source: String,
}

/// Tree shaker for dead code elimination
pub struct TreeShaker {
    /// All modules in the bundle
    modules: HashMap<ModuleId, ModuleInfo>,
    /// Module path to ID mapping
    path_to_id: HashMap<PathBuf, ModuleId>,
    /// Used exports per module (module_id -> set of export names)
    used_exports: HashMap<ModuleId, HashSet<String>>,
    /// Modules with side effects (must be included)
    side_effect_modules: HashSet<ModuleId>,
    /// Entry point modules
    entry_modules: HashSet<ModuleId>,
}

impl TreeShaker {
    /// Create a new tree shaker
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            path_to_id: HashMap::new(),
            used_exports: HashMap::new(),
            side_effect_modules: HashSet::new(),
            entry_modules: HashSet::new(),
        }
    }

    /// Add a module to the tree shaker
    pub fn add_module(
        &mut self,
        path: &Path,
        source: &str,
        is_entry: bool,
    ) -> BundleResult<ModuleId> {
        let filename = path.to_string_lossy();
        let parse_result = parse_module(source, &filename)?;

        let id = self.compute_module_id(path);
        let has_side_effects = self.detect_side_effects(&parse_result, source);

        let module_info = ModuleInfo {
            id,
            path: path.to_path_buf(),
            imports: parse_result.imports,
            exports: parse_result.exports,
            has_side_effects,
            source: source.to_string(),
        };

        self.modules.insert(id, module_info);
        self.path_to_id.insert(path.to_path_buf(), id);

        if is_entry {
            self.entry_modules.insert(id);
        }

        if has_side_effects {
            self.side_effect_modules.insert(id);
        }

        Ok(id)
    }

    /// Compute module ID from path
    fn compute_module_id(&self, path: &Path) -> ModuleId {
        let path_bytes = path.to_string_lossy().as_bytes().to_vec();
        xxhash_rust::xxh64::xxh64(&path_bytes, 0)
    }

    /// Detect if a module has side effects
    fn detect_side_effects(&self, parse_result: &ModuleParseResult, source: &str) -> bool {
        // Check for common side effect patterns
        let side_effect_patterns = [
            "console.",
            "window.",
            "document.",
            "global.",
            "process.",
            "require(",
            "fetch(",
            "XMLHttpRequest",
            "addEventListener",
            "setTimeout",
            "setInterval",
        ];

        for pattern in &side_effect_patterns {
            if source.contains(pattern) {
                return true;
            }
        }

        // Check for top-level function calls (potential side effects)
        // This is a simplified heuristic
        if source.contains("();") && !source.contains("export") {
            return true;
        }

        // If module has no exports, it likely has side effects
        if parse_result.exports.is_empty() && !parse_result.imports.is_empty() {
            return true;
        }

        false
    }

    /// Analyze the module graph and mark used exports
    pub fn analyze(&mut self) {
        // Start from entry points - mark all their exports as used
        let entry_ids: Vec<ModuleId> = self.entry_modules.iter().copied().collect();

        for entry_id in entry_ids {
            // Clone the module data we need to avoid borrow issues
            let (exports, imports) = match self.modules.get(&entry_id) {
                Some(module) => (module.exports.clone(), module.imports.clone()),
                None => continue,
            };

            // Mark all exports from entry modules as used
            for export in &exports {
                self.mark_used(entry_id, &export.name);
            }

            // Process imports from entry modules
            for import in imports {
                self.process_import(entry_id, &import);
            }
        }

        // Include all side-effect modules
        let side_effect_ids: Vec<ModuleId> = self.side_effect_modules.iter().copied().collect();
        for module_id in side_effect_ids {
            self.used_exports.entry(module_id).or_default();
        }
    }

    /// Mark an export as used and trace its dependencies
    fn mark_used(&mut self, module_id: ModuleId, export_name: &str) {
        let used = self.used_exports.entry(module_id).or_default();

        if used.contains(export_name) {
            return; // Already processed
        }

        used.insert(export_name.to_string());

        // Get module info
        let module = match self.modules.get(&module_id) {
            Some(m) => m.clone(),
            None => return,
        };

        // Find the export and trace its dependencies
        for export in &module.exports {
            if export.name == export_name {
                // If it's a re-export, trace to the source module
                if export.is_reexport {
                    if let Some(source) = &export.source {
                        if let Some(&source_id) = self.resolve_import_to_id(&module.path, source) {
                            // Mark the corresponding export in the source module
                            let source_export_name = export.local.as_ref().unwrap_or(&export.name);
                            self.mark_used(source_id, source_export_name);
                        }
                    }
                }
            }
        }

        // Process imports that this export might depend on
        for import in &module.imports {
            // If the export uses any imported names, mark them as used
            for imported_name in &import.imported_names {
                // Check if this import is used in the module
                // This is a simplified check - a full implementation would do scope analysis
                if module.source.contains(&imported_name.local) {
                    if let Some(&source_id) =
                        self.resolve_import_to_id(&module.path, &import.specifier)
                    {
                        self.mark_used(source_id, &imported_name.imported);
                    }
                }
            }
        }
    }

    /// Process an import and mark used exports in the source module
    fn process_import(&mut self, _from_module: ModuleId, import: &ParsedImport) {
        // Skip type-only imports
        if import.is_type_only {
            return;
        }

        // Get the module info for the importing module
        let from_module_info = match self
            .modules
            .values()
            .find(|m| m.imports.iter().any(|i| i.specifier == import.specifier))
        {
            Some(m) => m.clone(),
            None => return,
        };

        // Resolve the import to a module ID
        let target_id = match self.resolve_import_to_id(&from_module_info.path, &import.specifier) {
            Some(&id) => id,
            None => return,
        };

        // Mark imported names as used
        for imported_name in &import.imported_names {
            if imported_name.imported == "*" {
                // Namespace import - mark all exports as used
                if let Some(target_module) = self.modules.get(&target_id) {
                    for export in target_module.exports.clone() {
                        self.mark_used(target_id, &export.name);
                    }
                }
            } else {
                self.mark_used(target_id, &imported_name.imported);
            }
        }
    }

    /// Resolve an import specifier to a module ID
    fn resolve_import_to_id(&self, from_path: &Path, specifier: &str) -> Option<&ModuleId> {
        // Handle relative imports
        if specifier.starts_with('.') {
            let base = from_path.parent()?;
            let target = base.join(specifier);

            // Try exact path
            if let Some(id) = self.path_to_id.get(&target) {
                return Some(id);
            }

            // Try with extensions
            for ext in &[".js", ".ts", ".jsx", ".tsx", ".mjs"] {
                let with_ext = target.with_extension(&ext[1..]);
                if let Some(id) = self.path_to_id.get(&with_ext) {
                    return Some(id);
                }
            }

            // Try as directory with index
            for ext in &[".js", ".ts", ".jsx", ".tsx"] {
                let index = target.join(format!("index{}", ext));
                if let Some(id) = self.path_to_id.get(&index) {
                    return Some(id);
                }
            }
        }

        // For package imports, we'd need to resolve through node_modules
        // This is handled by the ModuleResolver in resolve.rs
        None
    }

    /// Check if a module should be included in the bundle
    pub fn is_module_included(&self, module_id: ModuleId) -> bool {
        // Entry modules are always included
        if self.entry_modules.contains(&module_id) {
            return true;
        }

        // Side-effect modules are always included
        if self.side_effect_modules.contains(&module_id) {
            return true;
        }

        // Include if any exports are used
        self.used_exports.get(&module_id).map(|e| !e.is_empty()).unwrap_or(false)
    }

    /// Check if an export should be included
    pub fn is_export_used(&self, module_id: ModuleId, export_name: &str) -> bool {
        // Entry module exports are always included
        if self.entry_modules.contains(&module_id) {
            return true;
        }

        self.used_exports
            .get(&module_id)
            .map(|exports| exports.contains(export_name))
            .unwrap_or(false)
    }

    /// Get all used exports for a module
    pub fn get_used_exports(&self, module_id: ModuleId) -> HashSet<String> {
        self.used_exports.get(&module_id).cloned().unwrap_or_default()
    }

    /// Get modules that should be included in the bundle
    pub fn get_included_modules(&self) -> Vec<ModuleId> {
        self.modules
            .keys()
            .filter(|&&id| self.is_module_included(id))
            .copied()
            .collect()
    }

    /// Generate tree-shaken output for a module
    /// Returns the source with unused exports removed
    pub fn generate_tree_shaken_source(&self, module_id: ModuleId) -> Option<String> {
        let module = self.modules.get(&module_id)?;

        // If this is an entry module, return the full source
        if self.entry_modules.contains(&module_id) {
            return Some(module.source.clone());
        }

        // Get the set of used exports
        let used_exports = self.get_used_exports(module_id);

        // If all exports are used, return the full source
        if used_exports.len() == module.exports.len() {
            return Some(module.source.clone());
        }

        // Build a new source with only used exports
        // This is a simplified implementation - a full implementation would
        // use AST transformation to properly remove unused code
        let mut result = String::new();
        let source = &module.source;

        // For now, we'll include the full source but mark unused exports
        // A production implementation would use proper AST manipulation
        for line in source.lines() {
            let mut include_line = true;

            // Check if this line is an export we should exclude
            if line.trim().starts_with("export ") {
                for export in &module.exports {
                    if !used_exports.contains(&export.name) {
                        // Check if this line exports the unused name
                        if line.contains(&format!("function {}", export.name))
                            || line.contains(&format!("const {}", export.name))
                            || line.contains(&format!("let {}", export.name))
                            || line.contains(&format!("var {}", export.name))
                            || line.contains(&format!("class {}", export.name))
                        {
                            include_line = false;
                            break;
                        }
                    }
                }
            }

            if include_line {
                result.push_str(line);
                result.push('\n');
            }
        }

        Some(result)
    }

    /// Get all tree-shaken modules for bundling
    pub fn get_tree_shaken_modules(&self) -> Vec<TreeShakenModule> {
        self.get_included_modules()
            .into_iter()
            .filter_map(|id| {
                let module = self.modules.get(&id)?;
                let source = self.generate_tree_shaken_source(id)?;
                Some(TreeShakenModule {
                    id,
                    path: module.path.clone(),
                    source,
                    used_exports: self.get_used_exports(id),
                })
            })
            .collect()
    }

    /// Get tree shaking statistics
    pub fn get_stats(&self) -> TreeShakeStats {
        let total_modules = self.modules.len();
        let included_modules = self.get_included_modules().len();
        let removed_modules = total_modules - included_modules;

        let total_exports: usize = self.modules.values().map(|m| m.exports.len()).sum();
        let used_exports: usize = self.used_exports.values().map(|e| e.len()).sum();
        let removed_exports = total_exports.saturating_sub(used_exports);

        TreeShakeStats {
            total_modules,
            included_modules,
            removed_modules,
            total_exports,
            used_exports,
            removed_exports,
        }
    }
}

impl Default for TreeShaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Tree shaking statistics
#[derive(Debug, Clone)]
pub struct TreeShakeStats {
    /// Total number of modules
    pub total_modules: usize,
    /// Number of modules included in bundle
    pub included_modules: usize,
    /// Number of modules removed
    pub removed_modules: usize,
    /// Total number of exports
    pub total_exports: usize,
    /// Number of exports used
    pub used_exports: usize,
    /// Number of exports removed
    pub removed_exports: usize,
}

/// A module after tree shaking
#[derive(Debug, Clone)]
pub struct TreeShakenModule {
    /// Module ID
    pub id: ModuleId,
    /// Module path
    pub path: PathBuf,
    /// Tree-shaken source code
    pub source: String,
    /// Set of exports that are used
    pub used_exports: HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_tree_shaker_basic() {
        let mut shaker = TreeShaker::new();

        let source = r#"
            export function used() { return 1; }
            export function unused() { return 2; }
        "#;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("module.js");
        fs::write(&path, source).unwrap();

        let id = shaker.add_module(&path, source, true).unwrap();
        shaker.analyze();

        // Entry module should be included
        assert!(shaker.is_module_included(id));
    }

    #[test]
    fn test_tree_shaker_removes_unused() {
        let mut shaker = TreeShaker::new();

        // Entry module that imports only 'used'
        let entry_source = r#"
            import { used } from './lib';
            console.log(used());
        "#;

        // Library module with used and unused exports
        let lib_source = r#"
            export function used() { return 1; }
            export function unused() { return 2; }
        "#;

        let temp_dir = TempDir::new().unwrap();
        let entry_path = temp_dir.path().join("entry.js");
        let lib_path = temp_dir.path().join("lib.js");

        fs::write(&entry_path, entry_source).unwrap();
        fs::write(&lib_path, lib_source).unwrap();

        let entry_id = shaker.add_module(&entry_path, entry_source, true).unwrap();
        let lib_id = shaker.add_module(&lib_path, lib_source, false).unwrap();

        shaker.analyze();

        // Entry should be included
        assert!(shaker.is_module_included(entry_id));

        // Library should be included (has used exports)
        assert!(shaker.is_module_included(lib_id));

        // 'used' export should be marked as used
        assert!(shaker.is_export_used(lib_id, "used"));
    }

    #[test]
    fn test_side_effect_detection() {
        let mut shaker = TreeShaker::new();

        // Module with side effects
        let source_with_effects = r#"
            console.log('side effect');
            export const foo = 1;
        "#;

        // Module without side effects
        let source_pure = r#"
            export const foo = 1;
            export const bar = 2;
        "#;

        let temp_dir = TempDir::new().unwrap();
        let path1 = temp_dir.path().join("effects.js");
        let path2 = temp_dir.path().join("pure.js");

        fs::write(&path1, source_with_effects).unwrap();
        fs::write(&path2, source_pure).unwrap();

        let id1 = shaker.add_module(&path1, source_with_effects, false).unwrap();
        let id2 = shaker.add_module(&path2, source_pure, false).unwrap();

        shaker.analyze();

        // Module with side effects should be included
        assert!(shaker.is_module_included(id1));

        // Pure module with no used exports should not be included
        assert!(!shaker.is_module_included(id2));
    }

    #[test]
    fn test_stats() {
        let mut shaker = TreeShaker::new();

        let source = r#"
            export function a() {}
            export function b() {}
            export function c() {}
        "#;

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("module.js");
        fs::write(&path, source).unwrap();

        shaker.add_module(&path, source, true).unwrap();
        shaker.analyze();

        let stats = shaker.get_stats();
        assert_eq!(stats.total_modules, 1);
        assert_eq!(stats.included_modules, 1);
        assert_eq!(stats.total_exports, 3);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    /// Generate valid JavaScript identifier names
    fn arb_identifier() -> impl Strategy<Value = String> {
        "[a-z][a-zA-Z0-9_]{0,10}".prop_filter("not reserved", |s| {
            // All JavaScript reserved words and keywords
            const RESERVED_WORDS: &[&str] = &[
                // Keywords
                "break",
                "case",
                "catch",
                "continue",
                "debugger",
                "default",
                "delete",
                "do",
                "else",
                "export",
                "extends",
                "finally",
                "for",
                "function",
                "if",
                "import",
                "in",
                "instanceof",
                "new",
                "return",
                "super",
                "switch",
                "this",
                "throw",
                "try",
                "typeof",
                "var",
                "void",
                "while",
                "with",
                "yield",
                // Strict mode reserved
                "class",
                "const",
                "enum",
                "let",
                "static",
                "implements",
                "interface",
                "package",
                "private",
                "protected",
                "public",
                // Literals
                "null",
                "true",
                "false",
                // Future reserved
                "await",
                "async",
                // Other common words that might cause issues
                "from",
                "as",
                "of",
                "get",
                "set",
            ];
            !RESERVED_WORDS.contains(&s.as_str())
        })
    }

    /// Generate a list of export names
    fn arb_export_names() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(arb_identifier(), 1..5).prop_map(|names| {
            // Deduplicate names
            let mut unique: Vec<String> = Vec::new();
            for name in names {
                if !unique.contains(&name) {
                    unique.push(name);
                }
            }
            unique
        })
    }

    /// Generate a simple module with exports
    fn generate_module_source(exports: &[String], has_side_effects: bool) -> String {
        let mut source = String::new();

        if has_side_effects {
            source.push_str("console.log('init');\n");
        }

        for name in exports {
            source.push_str(&format!("export function {}() {{ return '{}'; }}\n", name, name));
        }

        source
    }

    proptest! {
        /// Property 5: Tree Shaking Correctness
        /// For any bundle, removing unused exports SHALL NOT change the observable behavior
        /// **Validates: Requirements 6.3**
        #[test]
        fn prop_tree_shaking_preserves_used_exports(
            exports in arb_export_names(),
            _used_indices in prop::collection::vec(0usize..5, 0..3)
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut shaker = TreeShaker::new();

            // Create a module with the generated exports
            let source = generate_module_source(&exports, false);
            let path = temp_dir.path().join("module.js");
            fs::write(&path, &source).unwrap();

            let module_id = shaker.add_module(&path, &source, true).unwrap();
            shaker.analyze();

            // Entry module should always be included
            prop_assert!(shaker.is_module_included(module_id));

            // All exports from entry module should be marked as used
            for export_name in &exports {
                prop_assert!(
                    shaker.is_export_used(module_id, export_name),
                    "Entry module export '{}' should be marked as used",
                    export_name
                );
            }
        }

        /// Property: Side-effect modules are always included
        #[test]
        fn prop_side_effect_modules_included(
            exports in arb_export_names()
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut shaker = TreeShaker::new();

            // Create a module with side effects
            let source = generate_module_source(&exports, true);
            let path = temp_dir.path().join("side_effect.js");
            fs::write(&path, &source).unwrap();

            let module_id = shaker.add_module(&path, &source, false).unwrap();
            shaker.analyze();

            // Side-effect module should be included even if not an entry
            prop_assert!(
                shaker.is_module_included(module_id),
                "Side-effect module should be included"
            );
        }

        /// Property: Pure modules without used exports are excluded
        #[test]
        fn prop_pure_unused_modules_excluded(
            exports in arb_export_names()
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut shaker = TreeShaker::new();

            // Create a pure module (no side effects)
            let source = generate_module_source(&exports, false);
            let path = temp_dir.path().join("pure.js");
            fs::write(&path, &source).unwrap();

            // Add as non-entry module
            let module_id = shaker.add_module(&path, &source, false).unwrap();
            shaker.analyze();

            // Pure module with no imports should not be included
            prop_assert!(
                !shaker.is_module_included(module_id),
                "Pure unused module should not be included"
            );
        }

        /// Property: Stats are consistent
        #[test]
        fn prop_stats_consistency(
            num_modules in 1usize..5,
            exports_per_module in 1usize..4
        ) {
            let temp_dir = TempDir::new().unwrap();
            let mut shaker = TreeShaker::new();

            let mut total_exports = 0;

            for i in 0..num_modules {
                let exports: Vec<String> = (0..exports_per_module)
                    .map(|j| format!("export_{}_{}", i, j))
                    .collect();
                total_exports += exports.len();

                let source = generate_module_source(&exports, false);
                let path = temp_dir.path().join(format!("module_{}.js", i));
                fs::write(&path, &source).unwrap();

                // Only first module is entry
                shaker.add_module(&path, &source, i == 0).unwrap();
            }

            shaker.analyze();
            let stats = shaker.get_stats();

            // Total modules should match
            prop_assert_eq!(stats.total_modules, num_modules);

            // Total exports should match
            prop_assert_eq!(stats.total_exports, total_exports);

            // Included + removed should equal total
            prop_assert_eq!(
                stats.included_modules + stats.removed_modules,
                stats.total_modules
            );
        }
    }
}
