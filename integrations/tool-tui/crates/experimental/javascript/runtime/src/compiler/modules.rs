//! Module System Implementation
//!
//! Supports:
//! - ES6 modules (import/export)
//! - CommonJS (require/module.exports)
//! - Dynamic imports
//! - Module resolution
//! - Package.json parsing with proper JSON parsing
//! - Module graph building and linking
//! - Circular dependency detection

use crate::error::{DxError, DxResult};
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Module types
#[derive(Debug, Clone, PartialEq)]
pub enum ModuleType {
    ESModule,
    CommonJS,
    JSON,
    WASM,
}

/// Package.json structure for module resolution
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct PackageJson {
    /// Package name
    pub name: Option<String>,
    /// Package version
    pub version: Option<String>,
    /// Main entry point (CommonJS)
    pub main: Option<String>,
    /// Module entry point (ESM)
    pub module: Option<String>,
    /// Browser entry point
    pub browser: Option<String>,
    /// Module type: "module" for ESM, "commonjs" for CJS
    #[serde(rename = "type")]
    pub module_type: Option<String>,
    /// Exports field for conditional exports
    pub exports: Option<ExportsField>,
}

/// Exports field can be a string, object, or array
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ExportsField {
    /// Simple string export: "exports": "./index.js"
    String(String),
    /// Object exports: "exports": { ".": "./index.js", "./utils": "./utils.js" }
    Object(HashMap<String, ExportsValue>),
    /// Array exports (fallback): "exports": ["./index.js", "./fallback.js"]
    Array(Vec<ExportsValue>),
}

/// Value in exports object
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ExportsValue {
    /// Simple string
    String(String),
    /// Conditional exports: { "import": "./index.mjs", "require": "./index.cjs" }
    Conditional(HashMap<String, ExportsValue>),
    /// Array fallback
    Array(Vec<ExportsValue>),
}

impl PackageJson {
    /// Parse package.json from content
    pub fn parse(content: &str) -> DxResult<Self> {
        serde_json::from_str(content).map_err(|e| {
            DxError::ParseError(format!(
                "Invalid package.json: {} at line {} column {}",
                e,
                e.line(),
                e.column()
            ))
        })
    }

    /// Read package.json from file
    pub fn read(path: &Path) -> DxResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| DxError::IoError(e.to_string()))?;
        Self::parse(&content)
    }

    /// Check if this package uses ESM by default
    pub fn is_esm(&self) -> bool {
        self.module_type.as_deref() == Some("module")
    }

    /// Get the main entry point for the given conditions
    ///
    /// Conditions are checked in order: "import", "require", "default"
    pub fn resolve_entry(&self, subpath: &str, is_esm: bool) -> Option<String> {
        // First try exports field (Node.js 12.7+)
        if let Some(exports) = &self.exports {
            if let Some(resolved) = self.resolve_exports(exports, subpath, is_esm) {
                return Some(resolved);
            }
        }

        // Fall back to main/module fields for root entry
        if subpath == "." || subpath.is_empty() {
            // Prefer module field for ESM
            if is_esm {
                if let Some(module) = &self.module {
                    return Some(module.clone());
                }
            }

            // Fall back to main
            if let Some(main) = &self.main {
                return Some(main.clone());
            }

            // Default to index.js
            return Some("index.js".to_string());
        }

        None
    }

    /// Resolve exports field according to Node.js algorithm
    fn resolve_exports(
        &self,
        exports: &ExportsField,
        subpath: &str,
        is_esm: bool,
    ) -> Option<String> {
        match exports {
            ExportsField::String(s) => {
                if subpath == "." || subpath.is_empty() {
                    Some(s.clone())
                } else {
                    None
                }
            }
            ExportsField::Object(map) => {
                // Normalize subpath
                let key = if subpath.is_empty() || subpath == "." {
                    ".".to_string()
                } else if subpath.starts_with("./") {
                    subpath.to_string()
                } else {
                    format!("./{}", subpath)
                };

                // Try exact match first
                if let Some(value) = map.get(&key) {
                    return Self::resolve_exports_value(self, value, is_esm);
                }

                // Try pattern matching (e.g., "./*")
                for (pattern, value) in map {
                    if let Some(prefix) = pattern.strip_suffix('*') {
                        if let Some(suffix) = key.strip_prefix(prefix) {
                            if let Some(resolved) = Self::resolve_exports_value(self, value, is_esm)
                            {
                                return Some(resolved.replace('*', suffix));
                            }
                        }
                    }
                }

                None
            }
            ExportsField::Array(arr) => {
                // Try each in order
                for value in arr {
                    if let Some(resolved) = Self::resolve_exports_value(self, value, is_esm) {
                        return Some(resolved);
                    }
                }
                None
            }
        }
    }

    /// Resolve a single exports value
    fn resolve_exports_value(_self: &Self, value: &ExportsValue, is_esm: bool) -> Option<String> {
        match value {
            ExportsValue::String(s) => Some(s.clone()),
            ExportsValue::Conditional(conditions) => {
                // Check conditions in priority order
                let condition_order = if is_esm {
                    &["import", "module", "default", "require"][..]
                } else {
                    &["require", "default", "import", "module"][..]
                };

                for condition in condition_order {
                    if let Some(value) = conditions.get(*condition) {
                        return Self::resolve_exports_value(_self, value, is_esm);
                    }
                }

                // Try "node" condition as fallback
                if let Some(value) = conditions.get("node") {
                    return Self::resolve_exports_value(_self, value, is_esm);
                }

                None
            }
            ExportsValue::Array(arr) => {
                for v in arr {
                    if let Some(resolved) = Self::resolve_exports_value(_self, v, is_esm) {
                        return Some(resolved);
                    }
                }
                None
            }
        }
    }
}

/// A compiled module
#[derive(Debug, Clone)]
pub struct Module {
    /// Module path
    pub path: PathBuf,
    /// Module type
    pub module_type: ModuleType,
    /// Exported values
    pub exports: HashMap<String, usize>, // Export name -> value pointer
    /// Dependencies
    pub dependencies: Vec<String>,
}

/// Module resolver
pub struct ModuleResolver {
    /// Resolved modules cache
    modules: HashMap<PathBuf, Module>,
    /// Module search paths
    search_paths: Vec<PathBuf>,
    /// Package.json cache
    package_cache: HashMap<PathBuf, PackageJson>,
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            search_paths: vec![PathBuf::from("node_modules"), PathBuf::from(".")],
            package_cache: HashMap::new(),
        }
    }

    /// Resolve a module specifier to a file path
    pub fn resolve(&mut self, specifier: &str, from: &Path) -> DxResult<PathBuf> {
        // Relative imports
        if specifier.starts_with("./") || specifier.starts_with("../") {
            let base = from.parent().unwrap_or(Path::new("."));
            let path = base.join(specifier);
            let is_esm = self.is_esm_context(from);
            return self.resolve_file(&path, is_esm);
        }

        // Absolute imports
        if specifier.starts_with('/') {
            return self.resolve_file(Path::new(specifier), false);
        }

        // Package imports (may include subpath like "lodash/get")
        let (package_name, subpath) = self.parse_package_specifier(specifier);
        let is_esm = self.is_esm_context(from);
        self.resolve_package(&package_name, &subpath, is_esm)
    }

    /// Parse a package specifier into package name and subpath
    fn parse_package_specifier(&self, specifier: &str) -> (String, String) {
        // Handle scoped packages (@scope/package)
        if let Some(after_at) = specifier.strip_prefix('@') {
            if let Some(slash_pos) = after_at.find('/') {
                let scope_end = slash_pos + 1;
                if let Some(next_slash) = specifier[scope_end + 1..].find('/') {
                    let package_end = scope_end + 1 + next_slash;
                    return (
                        specifier[..package_end].to_string(),
                        specifier[package_end..].to_string(),
                    );
                }
                return (specifier.to_string(), ".".to_string());
            }
        }

        // Regular packages
        if let Some((package, subpath)) = specifier.split_once('/') {
            (package.to_string(), format!("./{subpath}"))
        } else {
            (specifier.to_string(), ".".to_string())
        }
    }

    /// Check if the context is ESM
    fn is_esm_context(&mut self, from: &Path) -> bool {
        // Check file extension
        if let Some(ext) = from.extension().and_then(|s| s.to_str()) {
            match ext {
                "mjs" | "mts" => return true,
                "cjs" | "cts" => return false,
                _ => {}
            }
        }

        // Check nearest package.json for "type": "module"
        if let Some(pkg_dir) = self.find_package_dir(from) {
            let pkg_json_path = pkg_dir.join("package.json");
            if let Ok(pkg) = self.get_or_load_package(&pkg_json_path) {
                return pkg.is_esm();
            }
        }

        false
    }

    /// Find the nearest directory containing package.json
    fn find_package_dir(&self, from: &Path) -> Option<PathBuf> {
        let mut current = from.parent()?;
        loop {
            if current.join("package.json").exists() {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }

    /// Get or load a package.json
    fn get_or_load_package(&mut self, path: &Path) -> DxResult<&PackageJson> {
        let path_buf = path.to_path_buf();
        if !self.package_cache.contains_key(&path_buf) {
            let pkg = PackageJson::read(path)?;
            self.package_cache.insert(path_buf.clone(), pkg);
        }
        self.package_cache
            .get(&path_buf)
            .ok_or_else(|| DxError::ModuleNotFound(path.display().to_string()))
    }

    /// Resolve a file path
    fn resolve_file(&self, path: &Path, is_esm: bool) -> DxResult<PathBuf> {
        // Try exact match
        if path.exists() && path.is_file() {
            return Ok(path.to_path_buf());
        }

        // Try with extensions
        let extensions = if is_esm {
            &[".mjs", ".mts", ".js", ".ts", ".tsx", ".jsx", ".cjs", ".cts"][..]
        } else {
            &[".js", ".ts", ".tsx", ".jsx", ".cjs", ".cts", ".mjs", ".mts"][..]
        };

        for ext in extensions {
            let path_with_ext = if path.extension().is_some() {
                path.to_path_buf()
            } else {
                PathBuf::from(format!("{}{}", path.display(), ext))
            };
            if path_with_ext.exists() && path_with_ext.is_file() {
                return Ok(path_with_ext);
            }
        }

        // Try index files
        if path.is_dir() {
            let index_files = if is_esm {
                &[
                    "index.mjs",
                    "index.mts",
                    "index.js",
                    "index.ts",
                    "index.tsx",
                    "index.jsx",
                ][..]
            } else {
                &[
                    "index.js",
                    "index.ts",
                    "index.tsx",
                    "index.jsx",
                    "index.mjs",
                    "index.mts",
                ][..]
            };

            for index in index_files {
                let index_path = path.join(index);
                if index_path.exists() {
                    return Ok(index_path);
                }
            }
        }

        Err(DxError::ModuleNotFound(path.display().to_string()))
    }

    /// Resolve a package
    fn resolve_package(&mut self, name: &str, subpath: &str, is_esm: bool) -> DxResult<PathBuf> {
        for search_path in self.search_paths.clone() {
            let package_path = search_path.join(name);

            // Try package.json
            let package_json_path = package_path.join("package.json");
            if package_json_path.exists() {
                // Load and parse package.json
                let entry = {
                    let pkg = self.get_or_load_package(&package_json_path)?;
                    pkg.resolve_entry(subpath, is_esm)
                };

                if let Some(entry) = entry {
                    let main_path = package_path.join(&entry);
                    if let Ok(resolved) = self.resolve_file(&main_path, is_esm) {
                        return Ok(resolved);
                    }
                }
            }

            // Try index files
            if let Ok(resolved) = self.resolve_file(&package_path, is_esm) {
                return Ok(resolved);
            }
        }

        Err(DxError::ModuleNotFound(name.to_string()))
    }

    /// Load a module
    pub fn load(&mut self, path: &PathBuf) -> DxResult<&Module> {
        if !self.modules.contains_key(path) {
            let module = self.compile_module(path)?;
            self.modules.insert(path.clone(), module);
        }
        self.modules
            .get(path)
            .ok_or_else(|| DxError::ModuleNotFound(path.display().to_string()))
    }

    /// Compile a module
    fn compile_module(&mut self, path: &PathBuf) -> DxResult<Module> {
        let content = std::fs::read_to_string(path).map_err(|e| DxError::IoError(e.to_string()))?;

        // Detect module type
        let ext = path.extension().and_then(|s| s.to_str());
        let module_type = if ext == Some("json") {
            ModuleType::JSON
        } else if ext == Some("mjs") || ext == Some("mts") {
            ModuleType::ESModule
        } else if ext == Some("cjs") || ext == Some("cts") {
            ModuleType::CommonJS
        } else if content.contains("import ")
            || content.contains("export ")
            || self.is_esm_context(path)
        {
            ModuleType::ESModule
        } else {
            ModuleType::CommonJS
        };

        // Extract dependencies using AST-based parsing
        let dependencies = match module_type {
            ModuleType::ESModule => {
                ESModuleParser::extract_imports(&content)
                    .into_iter()
                    .filter(|imp| !imp.is_type_only) // Skip type-only imports
                    .map(|imp| imp.specifier)
                    .collect()
            }
            ModuleType::CommonJS => CommonJSParser::extract_requires(&content),
            ModuleType::JSON | ModuleType::WASM => Vec::new(),
        };

        // Extract exports using AST-based parsing
        let export_statements = match module_type {
            ModuleType::ESModule => ESModuleParser::extract_exports(&content),
            ModuleType::CommonJS => Vec::new(), // CommonJS exports are dynamic
            ModuleType::JSON | ModuleType::WASM => vec![ExportStatement {
                name: "default".to_string(),
                is_default: true,
            }],
        };

        let mut exports = HashMap::new();
        for (i, export) in export_statements.iter().enumerate() {
            exports.insert(export.name.clone(), i);
        }

        let module = Module {
            path: path.clone(),
            module_type,
            exports,
            dependencies,
        };

        Ok(module)
    }
}

/// ES Module parser using OXC AST
pub struct ESModuleParser;

impl ESModuleParser {
    /// Extract imports from source using proper AST parsing
    pub fn extract_imports(source: &str) -> Vec<ImportStatement> {
        let allocator = oxc_allocator::Allocator::default();
        let source_type = oxc_span::SourceType::default().with_module(true);
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();

        if !parsed.errors.is_empty() {
            // Fall back to simple parsing on error
            return Self::extract_imports_simple(source);
        }

        let mut imports = Vec::new();

        for stmt in &parsed.program.body {
            match stmt {
                oxc_ast::ast::Statement::ImportDeclaration(import_decl) => {
                    // Get the raw string value and strip any trailing quotes
                    // Note: OXC's Atom may include trailing quote character in some versions
                    let raw_value = import_decl.source.value.as_str();
                    // Strip trailing quote if present (OXC bug workaround)
                    let specifier = {
                        let bytes = raw_value.as_bytes();
                        if !bytes.is_empty()
                            && (bytes[bytes.len() - 1] == b'\'' || bytes[bytes.len() - 1] == b'"')
                        {
                            String::from_utf8_lossy(&bytes[..bytes.len() - 1]).to_string()
                        } else {
                            raw_value.to_string()
                        }
                    };
                    let mut import_names = Vec::new();

                    // Collect imported names
                    if let Some(specifiers) = &import_decl.specifiers {
                        for spec in specifiers {
                            match spec {
                                oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                                    import_names.push(s.local.name.to_string());
                                }
                                oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                                    import_names.push(s.local.name.to_string());
                                }
                                oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                                    import_names.push(format!("* as {}", s.local.name));
                                }
                            }
                        }
                    }

                    imports.push(ImportStatement {
                        specifier,
                        imports: import_names,
                        is_type_only: import_decl.import_kind.is_type(),
                    });
                }
                oxc_ast::ast::Statement::ExportNamedDeclaration(export_decl) => {
                    // Handle re-exports: export { foo } from 'bar'
                    if let Some(source) = &export_decl.source {
                        let raw_value = source.value.as_str();
                        let specifier = {
                            let bytes = raw_value.as_bytes();
                            if !bytes.is_empty()
                                && (bytes[bytes.len() - 1] == b'\''
                                    || bytes[bytes.len() - 1] == b'"')
                            {
                                String::from_utf8_lossy(&bytes[..bytes.len() - 1]).to_string()
                            } else {
                                raw_value.to_string()
                            }
                        };
                        let mut import_names = Vec::new();

                        for spec in &export_decl.specifiers {
                            import_names.push(spec.local.name().to_string());
                        }

                        imports.push(ImportStatement {
                            specifier,
                            imports: import_names,
                            is_type_only: export_decl.export_kind.is_type(),
                        });
                    }
                }
                oxc_ast::ast::Statement::ExportAllDeclaration(export_all) => {
                    // Handle: export * from 'bar'
                    let raw_value = export_all.source.value.as_str();
                    let specifier = {
                        let bytes = raw_value.as_bytes();
                        if !bytes.is_empty()
                            && (bytes[bytes.len() - 1] == b'\'' || bytes[bytes.len() - 1] == b'"')
                        {
                            String::from_utf8_lossy(&bytes[..bytes.len() - 1]).to_string()
                        } else {
                            raw_value.to_string()
                        }
                    };
                    imports.push(ImportStatement {
                        specifier,
                        imports: vec!["*".to_string()],
                        is_type_only: export_all.export_kind.is_type(),
                    });
                }
                _ => {}
            }
        }

        imports
    }

    /// Simple fallback import extraction (for when AST parsing fails)
    fn extract_imports_simple(source: &str) -> Vec<ImportStatement> {
        let mut imports = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") {
                if let Some(from_pos) = trimmed.find(" from ") {
                    let after_from = &trimmed[from_pos + 6..];
                    // Remove leading/trailing whitespace, then quotes, then semicolon
                    let specifier = after_from
                        .trim()
                        .trim_end_matches(';')
                        .trim_matches(|c| c == '"' || c == '\'');
                    imports.push(ImportStatement {
                        specifier: specifier.to_string(),
                        imports: Vec::new(),
                        is_type_only: trimmed.contains("import type "),
                    });
                }
            }
        }

        imports
    }

    /// Extract exports from source using proper AST parsing
    pub fn extract_exports(source: &str) -> Vec<ExportStatement> {
        let allocator = oxc_allocator::Allocator::default();
        let source_type = oxc_span::SourceType::default().with_module(true);
        let parsed = oxc_parser::Parser::new(&allocator, source, source_type).parse();

        if !parsed.errors.is_empty() {
            // Fall back to simple parsing on error
            return Self::extract_exports_simple(source);
        }

        let mut exports = Vec::new();

        for stmt in &parsed.program.body {
            match stmt {
                oxc_ast::ast::Statement::ExportDefaultDeclaration(_) => {
                    exports.push(ExportStatement {
                        name: "default".to_string(),
                        is_default: true,
                    });
                }
                oxc_ast::ast::Statement::ExportNamedDeclaration(export_decl) => {
                    // Named exports from declaration
                    if let Some(decl) = &export_decl.declaration {
                        match decl {
                            oxc_ast::ast::Declaration::VariableDeclaration(var_decl) => {
                                for declarator in &var_decl.declarations {
                                    if let oxc_ast::ast::BindingPatternKind::BindingIdentifier(
                                        ident,
                                    ) = &declarator.id.kind
                                    {
                                        exports.push(ExportStatement {
                                            name: ident.name.to_string(),
                                            is_default: false,
                                        });
                                    }
                                }
                            }
                            oxc_ast::ast::Declaration::FunctionDeclaration(func_decl) => {
                                if let Some(id) = &func_decl.id {
                                    exports.push(ExportStatement {
                                        name: id.name.to_string(),
                                        is_default: false,
                                    });
                                }
                            }
                            oxc_ast::ast::Declaration::ClassDeclaration(class_decl) => {
                                if let Some(id) = &class_decl.id {
                                    exports.push(ExportStatement {
                                        name: id.name.to_string(),
                                        is_default: false,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }

                    // Named exports from specifiers
                    for spec in &export_decl.specifiers {
                        exports.push(ExportStatement {
                            name: spec.exported.name().to_string(),
                            is_default: false,
                        });
                    }
                }
                oxc_ast::ast::Statement::ExportAllDeclaration(export_all) => {
                    // export * from 'module' - re-exports all
                    if let Some(exported) = &export_all.exported {
                        exports.push(ExportStatement {
                            name: exported.name().to_string(),
                            is_default: false,
                        });
                    }
                }
                _ => {}
            }
        }

        exports
    }

    /// Simple fallback export extraction
    fn extract_exports_simple(source: &str) -> Vec<ExportStatement> {
        let mut exports = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("export ") {
                exports.push(ExportStatement {
                    name: "default".to_string(),
                    is_default: trimmed.contains("export default"),
                });
            }
        }

        exports
    }
}

#[derive(Debug, Clone)]
pub struct ImportStatement {
    pub specifier: String,
    pub imports: Vec<String>,
    pub is_type_only: bool,
}

#[derive(Debug, Clone)]
pub struct ExportStatement {
    pub name: String,
    pub is_default: bool,
}

/// CommonJS parser
pub struct CommonJSParser;

impl CommonJSParser {
    /// Extract requires from source
    pub fn extract_requires(source: &str) -> Vec<String> {
        let mut requires = Vec::new();

        for line in source.lines() {
            if let Some(pos) = line.find("require(") {
                let after = &line[pos + 8..];
                if let Some(quote_start) = after.find(['"', '\'']) {
                    if let Some(quote_char) = after.chars().nth(quote_start) {
                        let after_quote = &after[quote_start + 1..];
                        if let Some(quote_end) = after_quote.find(quote_char) {
                            requires.push(after_quote[..quote_end].to_string());
                        }
                    }
                }
            }
        }

        requires
    }
}

// ============================================================================
// CommonJS Module Implementation
// ============================================================================

/// Represents a CommonJS module with its exports
#[derive(Debug, Clone)]
pub struct CommonJSModule {
    /// Module ID (usually the resolved file path)
    pub id: String,
    /// The filename of the module
    pub filename: PathBuf,
    /// The directory name of the module
    pub dirname: PathBuf,
    /// The exports object
    pub exports: HashMap<String, usize>,
    /// Whether the module has been loaded
    pub loaded: bool,
    /// Child modules required by this module
    pub children: Vec<String>,
    /// Parent module that required this module
    pub parent: Option<String>,
    /// Paths to search for modules
    pub paths: Vec<PathBuf>,
}

impl CommonJSModule {
    /// Create a new CommonJS module
    pub fn new(filename: PathBuf) -> Self {
        let dirname = filename.parent().unwrap_or(Path::new(".")).to_path_buf();
        let id = filename.to_string_lossy().to_string();

        // Build module search paths
        let mut paths = Vec::new();
        let mut current = dirname.clone();
        loop {
            paths.push(current.join("node_modules"));
            if let Some(parent) = current.parent() {
                current = parent.to_path_buf();
            } else {
                break;
            }
        }

        Self {
            id,
            filename,
            dirname,
            exports: HashMap::new(),
            loaded: false,
            children: Vec::new(),
            parent: None,
            paths,
        }
    }

    /// Set the parent module
    pub fn set_parent(&mut self, parent: String) {
        self.parent = Some(parent);
    }

    /// Add a child module
    pub fn add_child(&mut self, child: String) {
        if !self.children.contains(&child) {
            self.children.push(child);
        }
    }

    /// Mark the module as loaded
    pub fn mark_loaded(&mut self) {
        self.loaded = true;
    }

    /// Set an export value
    pub fn set_export(&mut self, name: String, value: usize) {
        self.exports.insert(name, value);
    }

    /// Get an export value
    pub fn get_export(&self, name: &str) -> Option<usize> {
        self.exports.get(name).copied()
    }

    /// Get all exports
    pub fn get_exports(&self) -> &HashMap<String, usize> {
        &self.exports
    }
}

/// CommonJS module cache
#[derive(Debug, Default)]
pub struct CommonJSCache {
    /// Cached modules by their resolved path
    modules: HashMap<PathBuf, CommonJSModule>,
}

impl CommonJSCache {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Get a cached module
    pub fn get(&self, path: &Path) -> Option<&CommonJSModule> {
        self.modules.get(path)
    }

    /// Get a mutable cached module
    pub fn get_mut(&mut self, path: &Path) -> Option<&mut CommonJSModule> {
        self.modules.get_mut(path)
    }

    /// Cache a module
    pub fn insert(&mut self, path: PathBuf, module: CommonJSModule) {
        self.modules.insert(path, module);
    }

    /// Check if a module is cached
    pub fn contains(&self, path: &Path) -> bool {
        self.modules.contains_key(path)
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.modules.clear();
    }
}

/// CommonJS require function implementation
pub struct CommonJSRequire {
    /// Module resolver
    resolver: ModuleResolver,
    /// Module cache
    cache: CommonJSCache,
    /// Current module stack (for circular dependency detection)
    module_stack: Vec<PathBuf>,
}

impl CommonJSRequire {
    pub fn new() -> Self {
        Self {
            resolver: ModuleResolver::new(),
            cache: CommonJSCache::new(),
            module_stack: Vec::new(),
        }
    }

    /// Require a module
    pub fn require(&mut self, specifier: &str, from: &Path) -> DxResult<&CommonJSModule> {
        // Resolve the module path
        let resolved = self.resolver.resolve(specifier, from)?;

        // Check cache first
        if self.cache.contains(&resolved) {
            return self
                .cache
                .get(&resolved)
                .ok_or_else(|| DxError::ModuleNotFound(resolved.display().to_string()));
        }

        // Check for circular dependency
        if self.module_stack.contains(&resolved) {
            // Return partially loaded module (Node.js behavior)
            return self.cache.get(&resolved).ok_or_else(|| {
                DxError::ModuleNotFound(format!(
                    "Circular dependency detected: {}",
                    resolved.display()
                ))
            });
        }

        // Create new module
        let module = CommonJSModule::new(resolved.clone());
        self.cache.insert(resolved.clone(), module);

        // Push to stack for circular dependency detection
        self.module_stack.push(resolved.clone());

        // Load and execute the module
        self.load_module(&resolved)?;

        // Pop from stack
        self.module_stack.pop();

        // Return the loaded module
        self.cache
            .get(&resolved)
            .ok_or_else(|| DxError::ModuleNotFound(resolved.display().to_string()))
    }

    /// Load a module from disk
    fn load_module(&mut self, path: &Path) -> DxResult<()> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| DxError::IoError(format!("Failed to read {}: {}", path.display(), e)))?;

        // Detect if it's JSON
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            // Parse JSON and set as default export
            // For now, just mark as loaded
            if let Some(module) = self.cache.get_mut(path) {
                module.mark_loaded();
            }
            return Ok(());
        }

        // Extract requires for dependency tracking
        let requires = CommonJSParser::extract_requires(&content);

        // Load dependencies
        for req in requires {
            if let Ok(dep_path) = self.resolver.resolve(&req, path) {
                if !self.cache.contains(&dep_path) {
                    // Recursively load dependency
                    let _ = self.require(&req, path);
                }

                // Add as child
                if let Some(module) = self.cache.get_mut(path) {
                    module.add_child(dep_path.to_string_lossy().to_string());
                }
            }
        }

        // Mark as loaded
        if let Some(module) = self.cache.get_mut(path) {
            module.mark_loaded();
        }

        Ok(())
    }

    /// Get the module cache
    pub fn cache(&self) -> &CommonJSCache {
        &self.cache
    }

    /// Clear the module cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for CommonJSRequire {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// ESM/CJS Interop
// ============================================================================

/// Handles interoperability between ES Modules and CommonJS
pub struct ModuleInterop;

impl ModuleInterop {
    /// Import a CommonJS module from ESM
    /// Returns the module's exports as a namespace object
    pub fn import_cjs_from_esm(cjs_module: &CommonJSModule) -> HashMap<String, usize> {
        let mut namespace = HashMap::new();

        // Copy all exports
        for (name, &value) in cjs_module.get_exports() {
            namespace.insert(name.clone(), value);
        }

        // If there's no default export, create one from module.exports
        if !namespace.contains_key("default") {
            // In real implementation, this would be the module.exports value
            // For now, we just note that default should be the exports object
        }

        namespace
    }

    /// Require an ES Module from CommonJS
    /// Returns the module's default export (with limitations)
    pub fn require_esm_from_cjs(esm_exports: &HashMap<String, usize>) -> Option<usize> {
        // CommonJS require of ESM returns the default export
        // or the namespace object if no default
        esm_exports.get("default").copied()
    }

    /// Check if a module is ESM based on file extension and package.json
    pub fn is_esm_module(path: &Path, _resolver: &mut ModuleResolver) -> bool {
        // Check extension
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext {
                "mjs" | "mts" => return true,
                "cjs" | "cts" => return false,
                _ => {}
            }
        }

        // Check package.json
        let mut current = path.parent();
        while let Some(dir) = current {
            let pkg_path = dir.join("package.json");
            if pkg_path.exists() {
                if let Ok(pkg) = PackageJson::read(&pkg_path) {
                    return pkg.is_esm();
                }
            }
            current = dir.parent();
        }

        false
    }
}

// ============================================================================
// Module Loader - Full ES Module Loading Implementation
// ============================================================================

/// Module status in the loading lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    /// Module has been discovered but not yet fetched
    Unlinked,
    /// Module source has been fetched and parsed
    Linking,
    /// Module dependencies are being resolved
    Linked,
    /// Module is being evaluated
    Evaluating,
    /// Module evaluation is complete
    Evaluated,
    /// Module evaluation failed
    Error,
}

/// A node in the module graph
#[derive(Debug, Clone)]
pub struct ModuleGraphNode {
    /// Unique module ID
    pub id: usize,
    /// Resolved file path
    pub path: PathBuf,
    /// Module type (ESM, CJS, JSON)
    pub module_type: ModuleType,
    /// Current status
    pub status: ModuleStatus,
    /// Source code (if loaded)
    pub source: Option<String>,
    /// Parsed imports
    pub imports: Vec<ImportStatement>,
    /// Parsed exports
    pub exports: Vec<ExportStatement>,
    /// IDs of modules this module depends on
    pub dependencies: Vec<usize>,
    /// IDs of modules that depend on this module
    pub dependents: Vec<usize>,
    /// Export bindings: export name -> (source module id, source name)
    pub export_bindings: HashMap<String, (usize, String)>,
    /// Namespace object (for import * as ns)
    pub namespace: Option<HashMap<String, usize>>,
    /// Error if evaluation failed
    pub error: Option<String>,
}

impl ModuleGraphNode {
    pub fn new(id: usize, path: PathBuf, module_type: ModuleType) -> Self {
        Self {
            id,
            path,
            module_type,
            status: ModuleStatus::Unlinked,
            source: None,
            imports: Vec::new(),
            exports: Vec::new(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            export_bindings: HashMap::new(),
            namespace: None,
            error: None,
        }
    }
}

/// The module graph tracks all loaded modules and their relationships
#[derive(Debug)]
pub struct ModuleGraph {
    /// All modules indexed by ID
    nodes: Vec<ModuleGraphNode>,
    /// Path to module ID mapping
    path_to_id: HashMap<PathBuf, usize>,
    /// Entry point module ID
    entry_point: Option<usize>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            path_to_id: HashMap::new(),
            entry_point: None,
        }
    }

    /// Add a new module to the graph
    pub fn add_module(&mut self, path: PathBuf, module_type: ModuleType) -> usize {
        if let Some(&id) = self.path_to_id.get(&path) {
            return id;
        }
        let id = self.nodes.len();
        let node = ModuleGraphNode::new(id, path.clone(), module_type);
        self.nodes.push(node);
        self.path_to_id.insert(path, id);
        id
    }

    /// Get a module by ID
    pub fn get(&self, id: usize) -> Option<&ModuleGraphNode> {
        self.nodes.get(id)
    }

    /// Get a mutable module by ID
    pub fn get_mut(&mut self, id: usize) -> Option<&mut ModuleGraphNode> {
        self.nodes.get_mut(id)
    }

    /// Get module ID by path
    pub fn get_id(&self, path: &Path) -> Option<usize> {
        self.path_to_id.get(path).copied()
    }

    /// Add a dependency edge
    pub fn add_dependency(&mut self, from: usize, to: usize) {
        if let Some(node) = self.nodes.get_mut(from) {
            if !node.dependencies.contains(&to) {
                node.dependencies.push(to);
            }
        }
        if let Some(node) = self.nodes.get_mut(to) {
            if !node.dependents.contains(&from) {
                node.dependents.push(from);
            }
        }
    }

    /// Get topological order for evaluation (dependencies first)
    pub fn topological_order(&self) -> DxResult<Vec<usize>> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();

        fn visit(
            graph: &ModuleGraph,
            id: usize,
            visited: &mut HashSet<usize>,
            in_stack: &mut HashSet<usize>,
            result: &mut Vec<usize>,
        ) -> DxResult<()> {
            if in_stack.contains(&id) {
                // Circular dependency - this is allowed in ES modules
                // The module will be partially initialized
                return Ok(());
            }
            if visited.contains(&id) {
                return Ok(());
            }

            in_stack.insert(id);

            if let Some(node) = graph.get(id) {
                for &dep_id in &node.dependencies {
                    visit(graph, dep_id, visited, in_stack, result)?;
                }
            }

            in_stack.remove(&id);
            visited.insert(id);
            result.push(id);
            Ok(())
        }

        // Start from entry point or all roots
        if let Some(entry) = self.entry_point {
            visit(self, entry, &mut visited, &mut in_stack, &mut result)?;
        } else {
            for id in 0..self.nodes.len() {
                visit(self, id, &mut visited, &mut in_stack, &mut result)?;
            }
        }

        Ok(result)
    }

    /// Detect circular dependencies
    pub fn find_cycles(&self) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut path_set = HashSet::new();

        fn dfs(
            graph: &ModuleGraph,
            id: usize,
            visited: &mut HashSet<usize>,
            path: &mut Vec<usize>,
            path_set: &mut HashSet<usize>,
            cycles: &mut Vec<Vec<usize>>,
        ) {
            if path_set.contains(&id) {
                // Found a cycle
                let cycle_start = path.iter().position(|&x| x == id).unwrap();
                cycles.push(path[cycle_start..].to_vec());
                return;
            }
            if visited.contains(&id) {
                return;
            }

            visited.insert(id);
            path.push(id);
            path_set.insert(id);

            if let Some(node) = graph.get(id) {
                for &dep_id in &node.dependencies {
                    dfs(graph, dep_id, visited, path, path_set, cycles);
                }
            }

            path.pop();
            path_set.remove(&id);
        }

        for id in 0..self.nodes.len() {
            dfs(self, id, &mut visited, &mut path, &mut path_set, &mut cycles);
        }

        cycles
    }

    /// Get all modules
    pub fn modules(&self) -> &[ModuleGraphNode] {
        &self.nodes
    }

    /// Set entry point
    pub fn set_entry_point(&mut self, id: usize) {
        self.entry_point = Some(id);
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// The ModuleLoader handles the complete module loading lifecycle
pub struct ModuleLoader {
    /// Module resolver for path resolution
    resolver: ModuleResolver,
    /// Module graph
    graph: ModuleGraph,
    /// Dynamic import promises (module_id -> promise_id) - reserved for dynamic import()
    #[allow(dead_code)]
    pending_dynamic_imports: HashMap<usize, usize>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        Self {
            resolver: ModuleResolver::new(),
            graph: ModuleGraph::new(),
            pending_dynamic_imports: HashMap::new(),
        }
    }

    /// Create with custom search paths
    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        let mut loader = Self::new();
        loader.resolver.search_paths = search_paths;
        loader
    }

    /// Load an entry point module and all its dependencies
    pub fn load_entry(&mut self, entry_path: &Path) -> DxResult<usize> {
        let resolved = self.resolver.resolve_file(entry_path, true)?;
        let module_type = self.detect_module_type(&resolved)?;
        let entry_id = self.graph.add_module(resolved.clone(), module_type);
        self.graph.set_entry_point(entry_id);

        // Load the module and its dependencies
        self.load_module(entry_id)?;

        Ok(entry_id)
    }

    /// Load a module by ID
    fn load_module(&mut self, id: usize) -> DxResult<()> {
        // Get module info
        let (path, module_type) = {
            let node = self
                .graph
                .get(id)
                .ok_or_else(|| DxError::ModuleNotFound(format!("Module {} not found", id)))?;
            if node.status != ModuleStatus::Unlinked {
                return Ok(()); // Already loaded
            }
            (node.path.clone(), node.module_type.clone())
        };

        // Read source
        let source = std::fs::read_to_string(&path)
            .map_err(|e| DxError::IoError(format!("Failed to read {}: {}", path.display(), e)))?;

        // Parse imports and exports
        let (imports, exports) = match module_type {
            ModuleType::ESModule => {
                let imports = ESModuleParser::extract_imports(&source);
                let exports = ESModuleParser::extract_exports(&source);
                (imports, exports)
            }
            ModuleType::CommonJS => {
                let requires = CommonJSParser::extract_requires(&source);
                let imports: Vec<ImportStatement> = requires
                    .into_iter()
                    .map(|specifier| ImportStatement {
                        specifier,
                        imports: Vec::new(),
                        is_type_only: false,
                    })
                    .collect();
                (imports, Vec::new())
            }
            ModuleType::JSON => {
                let exports = vec![ExportStatement {
                    name: "default".to_string(),
                    is_default: true,
                }];
                (Vec::new(), exports)
            }
            ModuleType::WASM => {
                // WASM modules export their functions
                // For now, we export a default that represents the module instance
                let exports = vec![ExportStatement {
                    name: "default".to_string(),
                    is_default: true,
                }];
                (Vec::new(), exports)
            }
        };

        // Update node with parsed info
        {
            let node = self.graph.get_mut(id).unwrap();
            node.source = Some(source);
            node.imports = imports.clone();
            node.exports = exports;
            node.status = ModuleStatus::Linking;
        }

        // Resolve and load dependencies
        let mut dep_ids = Vec::new();
        for import in imports {
            if import.is_type_only {
                continue; // Skip type-only imports
            }

            let resolved = self.resolver.resolve(&import.specifier, &path)?;
            let dep_type = self.detect_module_type(&resolved)?;
            let dep_id = self.graph.add_module(resolved, dep_type);
            dep_ids.push(dep_id);
            self.graph.add_dependency(id, dep_id);
        }

        // Update status to linked
        {
            let node = self.graph.get_mut(id).unwrap();
            node.dependencies = dep_ids.clone();
            node.status = ModuleStatus::Linked;
        }

        // Recursively load dependencies
        for dep_id in dep_ids {
            self.load_module(dep_id)?;
        }

        Ok(())
    }

    /// Detect module type from file path
    fn detect_module_type(&self, path: &Path) -> DxResult<ModuleType> {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        match ext {
            "json" => Ok(ModuleType::JSON),
            "wasm" => Ok(ModuleType::WASM),
            "mjs" | "mts" => Ok(ModuleType::ESModule),
            "cjs" | "cts" => Ok(ModuleType::CommonJS),
            _ => {
                // Check package.json for type field
                if let Some(pkg_dir) = self.find_package_dir(path) {
                    let pkg_json_path = pkg_dir.join("package.json");
                    if pkg_json_path.exists() {
                        if let Ok(pkg) = PackageJson::read(&pkg_json_path) {
                            if pkg.is_esm() {
                                return Ok(ModuleType::ESModule);
                            }
                        }
                    }
                }

                // Default to ESModule for .js/.ts files
                Ok(ModuleType::ESModule)
            }
        }
    }

    /// Find the nearest directory containing package.json
    fn find_package_dir(&self, from: &Path) -> Option<PathBuf> {
        let mut current = from.parent()?;
        loop {
            if current.join("package.json").exists() {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }

    /// Get the module graph
    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }

    /// Get mutable module graph
    pub fn graph_mut(&mut self) -> &mut ModuleGraph {
        &mut self.graph
    }

    /// Get evaluation order (dependencies first)
    pub fn evaluation_order(&self) -> DxResult<Vec<usize>> {
        self.graph.topological_order()
    }

    /// Link all modules (resolve export bindings)
    pub fn link(&mut self) -> DxResult<()> {
        let order = self.evaluation_order()?;

        for &id in &order {
            self.link_module(id)?;
        }

        Ok(())
    }

    /// Link a single module's exports
    fn link_module(&mut self, id: usize) -> DxResult<()> {
        let (exports, imports, dependencies) = {
            let node = self
                .graph
                .get(id)
                .ok_or_else(|| DxError::ModuleNotFound(format!("Module {} not found", id)))?;
            (node.exports.clone(), node.imports.clone(), node.dependencies.clone())
        };

        // Build export bindings
        let mut export_bindings = HashMap::new();

        // Direct exports
        for export in &exports {
            export_bindings.insert(export.name.clone(), (id, export.name.clone()));
        }

        // Re-exports from imports
        for (i, import) in imports.iter().enumerate() {
            if i < dependencies.len() {
                let dep_id = dependencies[i];

                // Check for re-exports (export { x } from 'y' or export * from 'y')
                for name in &import.imports {
                    if name == "*" {
                        // export * from 'module' - copy all exports
                        if let Some(dep_node) = self.graph.get(dep_id) {
                            for dep_export in &dep_node.exports {
                                if dep_export.name != "default" {
                                    export_bindings.insert(
                                        dep_export.name.clone(),
                                        (dep_id, dep_export.name.clone()),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Update node
        if let Some(node) = self.graph.get_mut(id) {
            node.export_bindings = export_bindings;
        }

        Ok(())
    }

    /// Resolve a dynamic import
    pub fn resolve_dynamic_import(&mut self, specifier: &str, from: &Path) -> DxResult<usize> {
        let resolved = self.resolver.resolve(specifier, from)?;
        let module_type = self.detect_module_type(&resolved)?;
        let id = self.graph.add_module(resolved, module_type);

        // Load if not already loaded
        self.load_module(id)?;
        self.link_module(id)?;

        Ok(id)
    }

    /// Get resolver for external use
    pub fn resolver(&self) -> &ModuleResolver {
        &self.resolver
    }

    /// Get mutable resolver
    pub fn resolver_mut(&mut self) -> &mut ModuleResolver {
        &mut self.resolver
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Dynamic Import Support
// ============================================================================

/// Represents a pending dynamic import
#[derive(Debug, Clone)]
pub struct DynamicImport {
    /// The specifier being imported
    pub specifier: String,
    /// The module requesting the import
    pub referrer: PathBuf,
    /// Promise ID to resolve when complete
    pub promise_id: usize,
}

/// Dynamic import handler for runtime integration
pub struct DynamicImportHandler {
    /// Module loader
    loader: Arc<RwLock<ModuleLoader>>,
    /// Pending imports
    pending: VecDeque<DynamicImport>,
}

impl DynamicImportHandler {
    pub fn new(loader: Arc<RwLock<ModuleLoader>>) -> Self {
        Self {
            loader,
            pending: VecDeque::new(),
        }
    }

    /// Queue a dynamic import
    pub fn queue_import(&mut self, specifier: String, referrer: PathBuf, promise_id: usize) {
        self.pending.push_back(DynamicImport {
            specifier,
            referrer,
            promise_id,
        });
    }

    /// Process pending imports
    pub fn process_pending(&mut self) -> Vec<(usize, Result<usize, String>)> {
        let mut results = Vec::new();

        while let Some(import) = self.pending.pop_front() {
            let result = {
                let mut loader = self.loader.write().unwrap();
                loader.resolve_dynamic_import(&import.specifier, &import.referrer)
            };

            match result {
                Ok(module_id) => {
                    results.push((import.promise_id, Ok(module_id)));
                }
                Err(e) => {
                    results.push((import.promise_id, Err(e.to_string())));
                }
            }
        }

        results
    }

    /// Check if there are pending imports
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_graph_add_module() {
        let mut graph = ModuleGraph::new();
        let id1 = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
        let id2 = graph.add_module(PathBuf::from("b.js"), ModuleType::ESModule);

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);

        // Adding same path returns same ID
        let id1_again = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
        assert_eq!(id1_again, id1);
    }

    #[test]
    fn test_module_graph_dependencies() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
        let b = graph.add_module(PathBuf::from("b.js"), ModuleType::ESModule);
        let c = graph.add_module(PathBuf::from("c.js"), ModuleType::ESModule);

        // a -> b -> c
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);

        assert_eq!(graph.get(a).unwrap().dependencies, vec![b]);
        assert_eq!(graph.get(b).unwrap().dependencies, vec![c]);
        assert_eq!(graph.get(b).unwrap().dependents, vec![a]);
        assert_eq!(graph.get(c).unwrap().dependents, vec![b]);
    }

    #[test]
    fn test_module_graph_topological_order() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
        let b = graph.add_module(PathBuf::from("b.js"), ModuleType::ESModule);
        let c = graph.add_module(PathBuf::from("c.js"), ModuleType::ESModule);

        // a -> b -> c (a depends on b, b depends on c)
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);
        graph.set_entry_point(a);

        let order = graph.topological_order().unwrap();

        // c should come before b, b before a
        let c_pos = order.iter().position(|&x| x == c).unwrap();
        let b_pos = order.iter().position(|&x| x == b).unwrap();
        let a_pos = order.iter().position(|&x| x == a).unwrap();

        assert!(c_pos < b_pos);
        assert!(b_pos < a_pos);
    }

    #[test]
    fn test_module_graph_cycle_detection() {
        let mut graph = ModuleGraph::new();
        let a = graph.add_module(PathBuf::from("a.js"), ModuleType::ESModule);
        let b = graph.add_module(PathBuf::from("b.js"), ModuleType::ESModule);
        let c = graph.add_module(PathBuf::from("c.js"), ModuleType::ESModule);

        // a -> b -> c -> a (cycle)
        graph.add_dependency(a, b);
        graph.add_dependency(b, c);
        graph.add_dependency(c, a);

        let cycles = graph.find_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_package_json_parse() {
        let json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "type": "module",
            "main": "index.js",
            "module": "index.mjs",
            "exports": {
                ".": {
                    "import": "./index.mjs",
                    "require": "./index.cjs"
                },
                "./utils": "./utils.js"
            }
        }"#;

        let pkg = PackageJson::parse(json).unwrap();
        assert_eq!(pkg.name, Some("test-package".to_string()));
        assert!(pkg.is_esm());

        // Test exports resolution
        let entry = pkg.resolve_entry(".", true);
        assert_eq!(entry, Some("./index.mjs".to_string()));

        let entry_cjs = pkg.resolve_entry(".", false);
        assert_eq!(entry_cjs, Some("./index.cjs".to_string()));
    }

    #[test]
    fn test_esmodule_parser_imports() {
        let source = r#"
            import { foo, bar } from './utils.js';
            import defaultExport from 'lodash';
            import * as ns from './namespace.js';
            import type { Type } from './types.ts';
        "#;

        let imports = ESModuleParser::extract_imports(source);

        assert_eq!(imports.len(), 4);
        assert_eq!(imports[0].specifier, "./utils.js");
        assert_eq!(imports[1].specifier, "lodash");
        assert_eq!(imports[2].specifier, "./namespace.js");
        assert!(imports[3].is_type_only);
    }

    #[test]
    fn test_esmodule_parser_exports() {
        let source = r#"
            export const foo = 1;
            export function bar() {}
            export class Baz {}
            export default function() {}
            export { x, y } from './other.js';
        "#;

        let exports = ESModuleParser::extract_exports(source);

        // Should have: foo, bar, Baz, default, x, y
        assert!(exports.iter().any(|e| e.name == "foo"));
        assert!(exports.iter().any(|e| e.name == "bar"));
        assert!(exports.iter().any(|e| e.name == "Baz"));
        assert!(exports.iter().any(|e| e.is_default));
    }

    #[test]
    fn test_commonjs_parser_requires() {
        let source = r#"
            const fs = require('fs');
            const path = require("path");
            const local = require('./local');
        "#;

        let requires = CommonJSParser::extract_requires(source);

        assert_eq!(requires.len(), 3);
        assert!(requires.contains(&"fs".to_string()));
        assert!(requires.contains(&"path".to_string()));
        assert!(requires.contains(&"./local".to_string()));
    }

    #[test]
    fn test_commonjs_module_creation() {
        let module = CommonJSModule::new(PathBuf::from("/project/src/index.js"));

        assert_eq!(module.id, "/project/src/index.js");
        assert_eq!(module.filename, PathBuf::from("/project/src/index.js"));
        assert_eq!(module.dirname, PathBuf::from("/project/src"));
        assert!(!module.loaded);
        assert!(module.children.is_empty());
        assert!(module.parent.is_none());
    }

    #[test]
    fn test_commonjs_module_exports() {
        let mut module = CommonJSModule::new(PathBuf::from("/project/index.js"));

        module.set_export("foo".to_string(), 42);
        module.set_export("bar".to_string(), 100);

        assert_eq!(module.get_export("foo"), Some(42));
        assert_eq!(module.get_export("bar"), Some(100));
        assert_eq!(module.get_export("baz"), None);
    }

    #[test]
    fn test_commonjs_module_parent_child() {
        let mut parent = CommonJSModule::new(PathBuf::from("/project/parent.js"));
        let mut child = CommonJSModule::new(PathBuf::from("/project/child.js"));

        child.set_parent(parent.id.clone());
        parent.add_child(child.id.clone());

        assert_eq!(child.parent, Some("/project/parent.js".to_string()));
        assert!(parent.children.contains(&"/project/child.js".to_string()));
    }

    #[test]
    fn test_commonjs_cache() {
        let mut cache = CommonJSCache::new();
        let path = PathBuf::from("/project/module.js");
        let module = CommonJSModule::new(path.clone());

        assert!(!cache.contains(&path));

        cache.insert(path.clone(), module);

        assert!(cache.contains(&path));
        assert!(cache.get(&path).is_some());
    }

    #[test]
    fn test_module_interop_cjs_to_esm() {
        let mut cjs_module = CommonJSModule::new(PathBuf::from("/project/cjs.js"));
        cjs_module.set_export("foo".to_string(), 1);
        cjs_module.set_export("bar".to_string(), 2);

        let namespace = ModuleInterop::import_cjs_from_esm(&cjs_module);

        assert_eq!(namespace.get("foo"), Some(&1));
        assert_eq!(namespace.get("bar"), Some(&2));
    }

    #[test]
    fn test_module_interop_esm_to_cjs() {
        let mut esm_exports = HashMap::new();
        esm_exports.insert("default".to_string(), 42);
        esm_exports.insert("named".to_string(), 100);

        let default_export = ModuleInterop::require_esm_from_cjs(&esm_exports);

        assert_eq!(default_export, Some(42));
    }
}
