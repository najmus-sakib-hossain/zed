//! Module resolution with AST-based import parsing
//!
//! Provides:
//! - AST-based import extraction using OXC parser
//! - Node.js module resolution algorithm
//! - Conditional exports support
//! - Package.json parsing

use crate::error::{BundleError, BundleResult};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Parsed import information
#[derive(Debug, Clone)]
pub struct ParsedImport {
    /// Import specifier (the string in the import statement)
    pub specifier: String,
    /// Start position in source
    pub start: u32,
    /// End position in source
    pub end: u32,
    /// Whether this is a dynamic import
    pub is_dynamic: bool,
    /// Whether this is a type-only import (TypeScript)
    pub is_type_only: bool,
    /// Imported names (for named imports)
    pub imported_names: Vec<ImportedName>,
}

/// Individual imported name
#[derive(Debug, Clone)]
pub struct ImportedName {
    /// Local name (what it's called in this module)
    pub local: String,
    /// Imported name (what it's called in the source module)
    pub imported: String,
    /// Whether this is a type import
    pub is_type: bool,
}

/// Parsed export information
#[derive(Debug, Clone)]
pub struct ParsedExport {
    /// Exported name
    pub name: String,
    /// Local name (for `export { local as name }`)
    pub local: Option<String>,
    /// Whether this is a default export
    pub is_default: bool,
    /// Whether this is a re-export
    pub is_reexport: bool,
    /// Source module for re-exports
    pub source: Option<String>,
}

/// Module parse result
#[derive(Debug, Clone)]
pub struct ModuleParseResult {
    /// All imports found
    pub imports: Vec<ParsedImport>,
    /// All exports found
    pub exports: Vec<ParsedExport>,
    /// Whether the module has JSX
    pub has_jsx: bool,
    /// Whether the module has TypeScript
    pub has_typescript: bool,
}

/// Parse a JavaScript/TypeScript module and extract imports/exports
pub fn parse_module(source: &str, filename: &str) -> BundleResult<ModuleParseResult> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_default();

    let parser_result = Parser::new(&allocator, source, source_type).parse();

    if !parser_result.errors.is_empty() {
        let error_messages: Vec<String> =
            parser_result.errors.iter().map(|e| e.to_string()).collect();
        return Err(BundleError::parse_error(error_messages.join("\n")));
    }

    let program = parser_result.program;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let has_typescript = filename.ends_with(".ts") || filename.ends_with(".tsx");
    let has_jsx = filename.ends_with(".jsx") || filename.ends_with(".tsx");

    // Extract imports and exports from the AST
    for stmt in &program.body {
        match stmt {
            Statement::ImportDeclaration(import) => {
                let parsed = parse_import_declaration(import);
                imports.push(parsed);
            }
            Statement::ExportNamedDeclaration(export) => {
                let parsed = parse_export_named_declaration(export);
                exports.extend(parsed);
            }
            Statement::ExportDefaultDeclaration(export) => {
                exports.push(ParsedExport {
                    name: "default".to_string(),
                    local: get_default_export_local(export),
                    is_default: true,
                    is_reexport: false,
                    source: None,
                });
            }
            Statement::ExportAllDeclaration(export) => {
                exports.push(ParsedExport {
                    name: "*".to_string(),
                    local: None,
                    is_default: false,
                    is_reexport: true,
                    source: Some(export.source.value.to_string()),
                });
            }
            _ => {
                // Check for dynamic imports in expressions
                extract_dynamic_imports(stmt, &mut imports);
            }
        }
    }

    Ok(ModuleParseResult {
        imports,
        exports,
        has_jsx,
        has_typescript,
    })
}

fn parse_import_declaration(import: &ImportDeclaration) -> ParsedImport {
    let mut imported_names = Vec::new();

    if let Some(specifiers) = &import.specifiers {
        for spec in specifiers {
            match spec {
                ImportDeclarationSpecifier::ImportSpecifier(s) => {
                    imported_names.push(ImportedName {
                        local: s.local.name.to_string(),
                        imported: match &s.imported {
                            ModuleExportName::IdentifierName(id) => id.name.to_string(),
                            ModuleExportName::IdentifierReference(id) => id.name.to_string(),
                            ModuleExportName::StringLiteral(s) => s.value.to_string(),
                        },
                        is_type: s.import_kind.is_type(),
                    });
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                    imported_names.push(ImportedName {
                        local: s.local.name.to_string(),
                        imported: "default".to_string(),
                        is_type: false,
                    });
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                    imported_names.push(ImportedName {
                        local: s.local.name.to_string(),
                        imported: "*".to_string(),
                        is_type: false,
                    });
                }
            }
        }
    }

    ParsedImport {
        specifier: import.source.value.to_string(),
        start: import.span.start,
        end: import.span.end,
        is_dynamic: false,
        is_type_only: import.import_kind.is_type(),
        imported_names,
    }
}

fn parse_export_named_declaration(export: &ExportNamedDeclaration) -> Vec<ParsedExport> {
    let mut exports = Vec::new();

    // Handle re-exports: export { foo } from 'bar'
    if let Some(source) = &export.source {
        for spec in &export.specifiers {
            let (name, local) = match &spec.exported {
                ModuleExportName::IdentifierName(id) => (id.name.to_string(), None),
                ModuleExportName::IdentifierReference(id) => (id.name.to_string(), None),
                ModuleExportName::StringLiteral(s) => (s.value.to_string(), None),
            };

            exports.push(ParsedExport {
                name,
                local,
                is_default: false,
                is_reexport: true,
                source: Some(source.value.to_string()),
            });
        }
        return exports;
    }

    // Handle named exports: export { foo, bar as baz }
    for spec in &export.specifiers {
        let name = match &spec.exported {
            ModuleExportName::IdentifierName(id) => id.name.to_string(),
            ModuleExportName::IdentifierReference(id) => id.name.to_string(),
            ModuleExportName::StringLiteral(s) => s.value.to_string(),
        };

        let local = match &spec.local {
            ModuleExportName::IdentifierName(id) => Some(id.name.to_string()),
            ModuleExportName::IdentifierReference(id) => Some(id.name.to_string()),
            ModuleExportName::StringLiteral(s) => Some(s.value.to_string()),
        };

        exports.push(ParsedExport {
            name: name.clone(),
            local: if local.as_ref() == Some(&name) {
                None
            } else {
                local
            },
            is_default: false,
            is_reexport: false,
            source: None,
        });
    }

    // Handle declaration exports: export const foo = 1
    if let Some(decl) = &export.declaration {
        match decl {
            Declaration::VariableDeclaration(var) => {
                for declarator in &var.declarations {
                    if let Some(name) = get_binding_name(&declarator.id) {
                        exports.push(ParsedExport {
                            name,
                            local: None,
                            is_default: false,
                            is_reexport: false,
                            source: None,
                        });
                    }
                }
            }
            Declaration::FunctionDeclaration(func) => {
                if let Some(id) = &func.id {
                    exports.push(ParsedExport {
                        name: id.name.to_string(),
                        local: None,
                        is_default: false,
                        is_reexport: false,
                        source: None,
                    });
                }
            }
            Declaration::ClassDeclaration(class) => {
                if let Some(id) = &class.id {
                    exports.push(ParsedExport {
                        name: id.name.to_string(),
                        local: None,
                        is_default: false,
                        is_reexport: false,
                        source: None,
                    });
                }
            }
            _ => {}
        }
    }

    exports
}

fn get_default_export_local(export: &ExportDefaultDeclaration) -> Option<String> {
    match &export.declaration {
        ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
            func.id.as_ref().map(|id| id.name.to_string())
        }
        ExportDefaultDeclarationKind::ClassDeclaration(class) => {
            class.id.as_ref().map(|id| id.name.to_string())
        }
        _ => None,
    }
}

fn get_binding_name(pattern: &BindingPattern) -> Option<String> {
    match &pattern.kind {
        BindingPatternKind::BindingIdentifier(id) => Some(id.name.to_string()),
        _ => None,
    }
}

fn extract_dynamic_imports(stmt: &Statement, imports: &mut Vec<ParsedImport>) {
    // This is a simplified version - a full implementation would walk the entire AST
    // For now, we rely on the SIMD scanner to find dynamic imports
    // and this function handles the common cases

    if let Statement::ExpressionStatement(expr_stmt) = stmt {
        if let Expression::CallExpression(call) = &expr_stmt.expression {
            if is_import_call(call) {
                if let Some(Argument::StringLiteral(lit)) = call.arguments.first() {
                    imports.push(ParsedImport {
                        specifier: lit.value.to_string(),
                        start: call.span.start,
                        end: call.span.end,
                        is_dynamic: true,
                        is_type_only: false,
                        imported_names: vec![],
                    });
                }
            }
        }
    }
}

fn is_import_call(call: &CallExpression) -> bool {
    matches!(&call.callee, Expression::Identifier(id) if id.name == "import")
        || matches!(&call.callee, Expression::MetaProperty(_))
}

// ============================================================================
// Module Resolution Algorithm
// ============================================================================

/// Package.json structure (partial)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PackageJson {
    /// Package name
    pub name: Option<String>,
    /// Main entry point
    pub main: Option<String>,
    /// Module entry point (ESM)
    pub module: Option<String>,
    /// Browser entry point
    pub browser: Option<serde_json::Value>,
    /// Exports field (conditional exports)
    pub exports: Option<serde_json::Value>,
    /// Imports field (subpath imports)
    pub imports: Option<serde_json::Value>,
    /// Type field
    #[serde(rename = "type")]
    pub module_type: Option<String>,
}

/// Resolution conditions for conditional exports
#[derive(Debug, Clone)]
pub struct ResolveConditions {
    /// Target environment
    pub conditions: Vec<String>,
}

impl Default for ResolveConditions {
    fn default() -> Self {
        Self {
            conditions: vec![
                "import".to_string(),
                "module".to_string(),
                "default".to_string(),
            ],
        }
    }
}

impl ResolveConditions {
    /// Create conditions for browser bundling
    pub fn browser() -> Self {
        Self {
            conditions: vec![
                "browser".to_string(),
                "import".to_string(),
                "module".to_string(),
                "default".to_string(),
            ],
        }
    }

    /// Create conditions for Node.js
    pub fn node() -> Self {
        Self {
            conditions: vec![
                "node".to_string(),
                "import".to_string(),
                "module".to_string(),
                "default".to_string(),
            ],
        }
    }
}

/// Module resolver
pub struct ModuleResolver {
    /// Resolution conditions
    conditions: ResolveConditions,
    /// Package.json cache
    package_cache: HashMap<PathBuf, Option<PackageJson>>,
    /// Extensions to try
    extensions: Vec<String>,
}

impl ModuleResolver {
    /// Create a new resolver
    pub fn new(_base_dir: PathBuf) -> Self {
        Self {
            conditions: ResolveConditions::default(),
            package_cache: HashMap::new(),
            extensions: vec![
                ".js".to_string(),
                ".mjs".to_string(),
                ".cjs".to_string(),
                ".ts".to_string(),
                ".tsx".to_string(),
                ".jsx".to_string(),
                ".json".to_string(),
            ],
        }
    }

    /// Set resolution conditions
    pub fn with_conditions(mut self, conditions: ResolveConditions) -> Self {
        self.conditions = conditions;
        self
    }

    /// Resolve an import specifier from a given file
    pub fn resolve(&mut self, specifier: &str, from: &Path) -> BundleResult<PathBuf> {
        // Handle different specifier types
        if specifier.starts_with('/') {
            // Absolute path
            self.resolve_file(Path::new(specifier))
        } else if specifier.starts_with('.') {
            // Relative path
            self.resolve_relative(specifier, from)
        } else if specifier.starts_with('#') {
            // Subpath import
            self.resolve_subpath_import(specifier, from)
        } else {
            // Package import
            self.resolve_package(specifier, from)
        }
    }

    /// Resolve a relative import
    fn resolve_relative(&mut self, specifier: &str, from: &Path) -> BundleResult<PathBuf> {
        let base = from.parent().unwrap_or(Path::new("."));
        let target = base.join(specifier);
        self.resolve_file(&target)
    }

    /// Resolve a file path, trying extensions
    fn resolve_file(&self, path: &Path) -> BundleResult<PathBuf> {
        // Try exact path first
        if path.is_file() {
            return Ok(path.to_path_buf());
        }

        // Try with extensions
        for ext in &self.extensions {
            let with_ext = path.with_extension(&ext[1..]);
            if with_ext.is_file() {
                return Ok(with_ext);
            }
        }

        // Try as directory with index file
        if path.is_dir() {
            for ext in &self.extensions {
                let index = path.join(format!("index{}", ext));
                if index.is_file() {
                    return Ok(index);
                }
            }
        }

        // Try path/index with extensions
        let as_dir = path.to_path_buf();
        for ext in &self.extensions {
            let index = as_dir.join(format!("index{}", ext));
            if index.is_file() {
                return Ok(index);
            }
        }

        Err(BundleError::module_not_found(path))
    }

    /// Resolve a package import (bare specifier)
    fn resolve_package(&mut self, specifier: &str, from: &Path) -> BundleResult<PathBuf> {
        // Split package name and subpath
        let (package_name, subpath) = split_package_specifier(specifier);

        // Find node_modules directory
        let mut current = from.parent();

        while let Some(dir) = current {
            let node_modules = dir.join("node_modules").join(&package_name);

            if node_modules.is_dir() {
                // Found the package
                return self.resolve_package_entry(&node_modules, subpath.as_deref());
            }

            current = dir.parent();
        }

        Err(BundleError::module_not_found(Path::new(specifier)))
    }

    /// Resolve package entry point
    fn resolve_package_entry(
        &mut self,
        package_dir: &Path,
        subpath: Option<&str>,
    ) -> BundleResult<PathBuf> {
        let package_json_path = package_dir.join("package.json");

        // Load package.json
        let pkg = self.load_package_json(&package_json_path)?;

        // Handle subpath
        if let Some(subpath) = subpath {
            // Check exports field first
            if let Some(exports) = &pkg.exports {
                if let Some(resolved) = self.resolve_exports(exports, subpath, package_dir)? {
                    return Ok(resolved);
                }
            }

            // Fall back to direct file resolution
            let target = package_dir.join(subpath);
            return self.resolve_file(&target);
        }

        // Resolve main entry point
        // Check exports field first (for "." entry)
        if let Some(exports) = &pkg.exports {
            if let Some(resolved) = self.resolve_exports(exports, ".", package_dir)? {
                return Ok(resolved);
            }
        }

        // Try module field (ESM)
        if let Some(module) = &pkg.module {
            let module_path = package_dir.join(module);
            if module_path.is_file() {
                return Ok(module_path);
            }
        }

        // Try main field
        if let Some(main) = &pkg.main {
            let main_path = package_dir.join(main);
            return self.resolve_file(&main_path);
        }

        // Default to index.js
        self.resolve_file(&package_dir.join("index"))
    }

    /// Resolve conditional exports
    fn resolve_exports(
        &self,
        exports: &serde_json::Value,
        subpath: &str,
        package_dir: &Path,
    ) -> BundleResult<Option<PathBuf>> {
        let key = if subpath == "." {
            ".".to_string()
        } else {
            format!("./{}", subpath.trim_start_matches("./"))
        };

        match exports {
            // String export: "exports": "./index.js"
            serde_json::Value::String(s) if subpath == "." => {
                let target = package_dir.join(s.trim_start_matches("./"));
                return Ok(Some(target));
            }

            // Object exports
            serde_json::Value::Object(map) => {
                // Check for exact match
                if let Some(value) = map.get(&key) {
                    return self.resolve_export_value(value, package_dir);
                }

                // Check for pattern match (e.g., "./*")
                for (pattern, value) in map {
                    if pattern.contains('*') {
                        if let Some(matched) = match_export_pattern(pattern, &key) {
                            if let Some(resolved) = self.resolve_export_value(value, package_dir)? {
                                // Replace * in resolved path
                                let resolved_str = resolved.to_string_lossy();
                                let final_path = resolved_str.replace('*', &matched);
                                return Ok(Some(PathBuf::from(final_path)));
                            }
                        }
                    }
                }

                // If no subpath keys, treat as conditional exports for "."
                if subpath == "." && !map.keys().any(|k| k.starts_with('.')) {
                    return self.resolve_conditional_export(exports, package_dir);
                }
            }

            _ => {}
        }

        Ok(None)
    }

    /// Resolve a conditional export value
    fn resolve_conditional_export(
        &self,
        value: &serde_json::Value,
        package_dir: &Path,
    ) -> BundleResult<Option<PathBuf>> {
        match value {
            serde_json::Value::String(s) => {
                let target = package_dir.join(s.trim_start_matches("./"));
                Ok(Some(target))
            }
            serde_json::Value::Object(map) => {
                // Check conditions in order
                for condition in &self.conditions.conditions {
                    if let Some(value) = map.get(condition) {
                        return self.resolve_conditional_export(value, package_dir);
                    }
                }
                Ok(None)
            }
            serde_json::Value::Array(arr) => {
                // Try each option in order
                for item in arr {
                    if let Some(resolved) = self.resolve_conditional_export(item, package_dir)? {
                        if resolved.exists() {
                            return Ok(Some(resolved));
                        }
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn resolve_export_value(
        &self,
        value: &serde_json::Value,
        package_dir: &Path,
    ) -> BundleResult<Option<PathBuf>> {
        self.resolve_conditional_export(value, package_dir)
    }

    /// Resolve subpath imports (#imports)
    fn resolve_subpath_import(&mut self, specifier: &str, from: &Path) -> BundleResult<PathBuf> {
        // Find nearest package.json
        let mut current = from.parent();

        while let Some(dir) = current {
            let package_json_path = dir.join("package.json");

            if package_json_path.is_file() {
                let pkg = self.load_package_json(&package_json_path)?;

                if let Some(imports) = &pkg.imports {
                    if let Some(resolved) = self.resolve_exports(imports, specifier, dir)? {
                        return Ok(resolved);
                    }
                }
            }

            current = dir.parent();
        }

        Err(BundleError::module_not_found(Path::new(specifier)))
    }

    /// Load and cache package.json
    fn load_package_json(&mut self, path: &Path) -> BundleResult<PackageJson> {
        if let Some(cached) = self.package_cache.get(path) {
            return cached.clone().ok_or_else(|| BundleError::module_not_found(path));
        }

        let content =
            std::fs::read_to_string(path).map_err(|_| BundleError::module_not_found(path))?;

        let pkg: PackageJson = serde_json::from_str(&content)
            .map_err(|e| BundleError::parse_error(format!("Invalid package.json: {}", e)))?;

        self.package_cache.insert(path.to_path_buf(), Some(pkg.clone()));

        Ok(pkg)
    }
}

/// Split a package specifier into name and subpath
fn split_package_specifier(specifier: &str) -> (String, Option<String>) {
    if specifier.starts_with('@') {
        // Scoped package: @scope/name/subpath
        let parts: Vec<&str> = specifier.splitn(3, '/').collect();
        if parts.len() >= 2 {
            let name = format!("{}/{}", parts[0], parts[1]);
            let subpath = parts.get(2).map(|s| s.to_string());
            return (name, subpath);
        }
    } else {
        // Regular package: name/subpath
        let parts: Vec<&str> = specifier.splitn(2, '/').collect();
        if parts.len() == 2 {
            return (parts[0].to_string(), Some(parts[1].to_string()));
        }
    }

    (specifier.to_string(), None)
}

/// Match an export pattern like "./*" against a key
fn match_export_pattern(pattern: &str, key: &str) -> Option<String> {
    let star_pos = pattern.find('*')?;
    let prefix = &pattern[..star_pos];
    let suffix = &pattern[star_pos + 1..];

    if key.starts_with(prefix) && key.ends_with(suffix) {
        let matched = &key[prefix.len()..key.len() - suffix.len()];
        Some(matched.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_import() {
        let source = r#"import { foo } from 'bar';"#;
        let result = parse_module(source, "test.js").unwrap();

        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].specifier, "bar");
        assert!(!result.imports[0].is_dynamic);
    }

    #[test]
    fn test_parse_default_import() {
        let source = r#"import foo from 'bar';"#;
        let result = parse_module(source, "test.js").unwrap();

        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].imported_names.len(), 1);
        assert_eq!(result.imports[0].imported_names[0].imported, "default");
    }

    #[test]
    fn test_parse_namespace_import() {
        let source = r#"import * as foo from 'bar';"#;
        let result = parse_module(source, "test.js").unwrap();

        assert_eq!(result.imports.len(), 1);
        assert_eq!(result.imports[0].imported_names.len(), 1);
        assert_eq!(result.imports[0].imported_names[0].imported, "*");
    }

    #[test]
    fn test_parse_named_exports() {
        let source = r#"export { foo, bar as baz };"#;
        let result = parse_module(source, "test.js").unwrap();

        assert_eq!(result.exports.len(), 2);
        assert_eq!(result.exports[0].name, "foo");
        assert_eq!(result.exports[1].name, "baz");
    }

    #[test]
    fn test_parse_default_export() {
        let source = r#"export default function foo() {}"#;
        let result = parse_module(source, "test.js").unwrap();

        assert_eq!(result.exports.len(), 1);
        assert!(result.exports[0].is_default);
    }

    #[test]
    fn test_parse_reexport() {
        let source = r#"export { foo } from 'bar';"#;
        let result = parse_module(source, "test.js").unwrap();

        assert_eq!(result.exports.len(), 1);
        assert!(result.exports[0].is_reexport);
        assert_eq!(result.exports[0].source, Some("bar".to_string()));
    }

    #[test]
    fn test_split_package_specifier() {
        assert_eq!(split_package_specifier("lodash"), ("lodash".to_string(), None));
        assert_eq!(
            split_package_specifier("lodash/fp"),
            ("lodash".to_string(), Some("fp".to_string()))
        );
        assert_eq!(split_package_specifier("@scope/pkg"), ("@scope/pkg".to_string(), None));
        assert_eq!(
            split_package_specifier("@scope/pkg/sub"),
            ("@scope/pkg".to_string(), Some("sub".to_string()))
        );
    }

    #[test]
    fn test_match_export_pattern() {
        assert_eq!(match_export_pattern("./*", "./foo"), Some("foo".to_string()));
        assert_eq!(match_export_pattern("./lib/*", "./lib/utils"), Some("utils".to_string()));
        assert_eq!(match_export_pattern("./*.js", "./foo.js"), Some("foo".to_string()));
        assert_eq!(match_export_pattern("./*", "./foo/bar"), Some("foo/bar".to_string()));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    /// JavaScript reserved words that cannot be used as identifiers
    const JS_RESERVED_WORDS: &[&str] = &[
        "break", "case", "catch", "continue", "debugger", "default", "delete",
        "do", "else", "finally", "for", "function", "if", "in", "instanceof",
        "new", "return", "switch", "this", "throw", "try", "typeof", "var",
        "void", "while", "with", "class", "const", "enum", "export", "extends",
        "import", "super", "implements", "interface", "let", "package", "private",
        "protected", "public", "static", "yield", "await", "null", "true", "false",
    ];

    /// Generate valid JavaScript module names (excluding reserved words)
    fn arb_module_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}".prop_filter("Must not be a reserved word", |s| {
            !JS_RESERVED_WORDS.contains(&s.as_str())
        })
    }

    /// Generate valid import specifiers
    fn arb_import_specifier() -> impl Strategy<Value = String> {
        prop_oneof![
            // Relative imports
            Just("./".to_string()).prop_flat_map(|prefix| {
                arb_module_name().prop_map(move |name| format!("{}{}", prefix, name))
            }),
            // Parent imports
            Just("../".to_string()).prop_flat_map(|prefix| {
                arb_module_name().prop_map(move |name| format!("{}{}", prefix, name))
            }),
            // Package imports
            arb_module_name(),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 21: Module Resolution Correctness
        ///
        /// **Feature: production-readiness, Property 21: Module Resolution Correctness**
        /// **Validates: Requirements 15.3, 15.4, 15.5**
        ///
        /// For any valid import specifier:
        /// - Relative imports starting with ./ resolve relative to the importing file
        /// - Relative imports starting with ../ resolve to parent directory
        /// - Package imports resolve through node_modules
        #[test]
        fn prop_relative_import_resolution(
            module_name in arb_module_name(),
        ) {
            let temp_dir = TempDir::new().unwrap();
            let src_dir = temp_dir.path().join("src");
            fs::create_dir_all(&src_dir).unwrap();

            // Create a source file
            let source_file = src_dir.join(format!("{}.js", module_name));
            fs::write(&source_file, "export const x = 1;").unwrap();

            // Create an importing file
            let importer = src_dir.join("index.js");
            fs::write(&importer, format!("import {{ x }} from './{}'", module_name)).unwrap();

            // Resolve the import
            let mut resolver = ModuleResolver::new(temp_dir.path().to_path_buf());
            let specifier = format!("./{}", module_name);
            let result = resolver.resolve(&specifier, &importer);

            // Should resolve to the source file
            prop_assert!(result.is_ok(), "Failed to resolve: {:?}", result);
            let resolved = result.unwrap();
            prop_assert!(
                resolved.file_name().unwrap().to_string_lossy().starts_with(&module_name),
                "Resolved path should contain module name"
            );
        }

        /// Property 21b: Parent directory resolution
        ///
        /// For any valid module name, resolving ../<name> from a subdirectory
        /// should resolve to the parent directory.
        #[test]
        fn prop_parent_import_resolution(
            module_name in arb_module_name(),
        ) {
            let temp_dir = TempDir::new().unwrap();
            let src_dir = temp_dir.path().join("src");
            let sub_dir = src_dir.join("sub");
            fs::create_dir_all(&sub_dir).unwrap();

            // Create a source file in src/
            let source_file = src_dir.join(format!("{}.js", module_name));
            fs::write(&source_file, "export const x = 1;").unwrap();

            // Create an importing file in src/sub/
            let importer = sub_dir.join("index.js");
            fs::write(&importer, format!("import {{ x }} from '../{}'", module_name)).unwrap();

            // Resolve the import
            let mut resolver = ModuleResolver::new(temp_dir.path().to_path_buf());
            let specifier = format!("../{}", module_name);
            let result = resolver.resolve(&specifier, &importer);

            // Should resolve to the source file in parent directory
            prop_assert!(result.is_ok(), "Failed to resolve: {:?}", result);
            let resolved = result.unwrap();

            // Canonicalize the path to resolve ../ components
            let canonical = resolved.canonicalize().unwrap_or(resolved.clone());

            // Verify the resolved path is in the src directory
            let resolved_parent = canonical.parent();
            prop_assert!(resolved_parent.is_some(), "Resolved path should have a parent");
            let parent_name = resolved_parent.and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string());
            prop_assert_eq!(
                parent_name, Some("src".to_string()),
                "Resolved path should be in src directory, got: {:?}", canonical
            );
        }

        /// Property 21c: Import parsing round-trip
        ///
        /// For any valid JavaScript source with imports, parsing should
        /// correctly extract all import specifiers.
        #[test]
        fn prop_import_parsing_extracts_specifiers(
            specifier in arb_import_specifier(),
        ) {
            let source = format!(r#"import {{ foo }} from '{}';"#, specifier);
            let result = parse_module(&source, "test.js");

            prop_assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let parsed = result.unwrap();
            prop_assert_eq!(parsed.imports.len(), 1, "Should have exactly one import");
            prop_assert_eq!(
                &parsed.imports[0].specifier, &specifier,
                "Import specifier should match"
            );
        }

        /// Property 21d: Export parsing completeness
        ///
        /// For any valid export declaration, parsing should correctly
        /// extract the exported name.
        #[test]
        fn prop_export_parsing_extracts_names(
            name in arb_module_name(),
        ) {
            let source = format!("export const {} = 1;", name);
            let result = parse_module(&source, "test.js");

            prop_assert!(result.is_ok(), "Failed to parse: {:?}", result);
            let parsed = result.unwrap();
            prop_assert!(
                parsed.exports.iter().any(|e| e.name == name),
                "Should export the declared name"
            );
        }

        /// Property 21e: Package specifier splitting
        ///
        /// For any package specifier, splitting should correctly separate
        /// the package name from the subpath.
        #[test]
        fn prop_package_specifier_split(
            pkg_name in arb_module_name(),
            subpath in prop::option::of(arb_module_name()),
        ) {
            let specifier = match &subpath {
                Some(sub) => format!("{}/{}", pkg_name, sub),
                None => pkg_name.clone(),
            };

            let (name, path) = split_package_specifier(&specifier);

            prop_assert_eq!(name, pkg_name, "Package name should match");
            prop_assert_eq!(path, subpath, "Subpath should match");
        }
    }
}
