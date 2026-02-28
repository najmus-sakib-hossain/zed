//! # Parser Module - AST-Based TSX Parser
//!
//! Parses `.tsx` files using OXC (Oxidation Compiler) for accurate AST analysis.
//! Falls back to regex-based parsing when OXC feature is disabled.
//!
//! ## Features
//!
//! - Full JSX/TSX syntax support via OXC
//! - Component extraction with props and state
//! - Import/export analysis
//! - Security validation (banned keywords)
//! - Content hashing for cache invalidation

use crate::linker::SymbolTable;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Banned keywords that will fail the build immediately (security)
const BANNED_KEYWORDS: &[&str] = &[
    "eval",
    "innerHTML",
    "outerHTML",
    "document.write",
    "Function(",
    "dangerouslySetInnerHTML",
    "javascript:",
    "data:text/html",
];

/// Parsed module with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModule {
    pub path: PathBuf,
    pub imports: Vec<ImportDecl>,
    pub exports: Vec<ExportDecl>,
    pub components: Vec<Component>,
    pub hash: String,
}

/// Import declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportDecl {
    pub source: String,
    pub specifiers: Vec<ImportSpecifier>,
    pub is_type_only: bool,
}

/// Import specifier (named, default, or namespace)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImportSpecifier {
    Named { local: String, imported: String },
    Default { local: String },
    Namespace { local: String },
}

/// Export declaration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDecl {
    pub name: String,
    pub is_default: bool,
    pub is_type_only: bool,
}

/// Component definition extracted from the AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub props: Vec<PropDef>,
    pub state: Vec<StateDef>,
    pub jsx_body: String,
    pub hooks: Vec<HookCall>,
    pub is_async: bool,
    pub has_children: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropDef {
    pub name: String,
    pub type_annotation: String,
    pub is_optional: bool,
    pub default_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDef {
    pub name: String,
    pub setter_name: String,
    pub initial_value: String,
    pub type_annotation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookCall {
    pub hook_name: String,
    pub args: Vec<String>,
    pub dependencies: Option<Vec<String>>,
}

/// Parse the entry file or directory
pub fn parse_entry(
    entry: &Path,
    symbol_table: &SymbolTable,
    verbose: bool,
) -> Result<Vec<ParsedModule>> {
    let mut modules = Vec::new();

    if entry.is_dir() {
        if verbose {
            println!("  📂 Parsing directory: {}", entry.display());
        }

        for entry in walkdir::WalkDir::new(entry)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if is_parseable_file(path) {
                if verbose {
                    println!("  📄 Parsing: {}", path.display());
                }

                match parse_single_module(path, symbol_table, verbose) {
                    Ok(module) => modules.push(module),
                    Err(e) => {
                        // In production, we fail fast on parse errors
                        return Err(e)
                            .with_context(|| format!("Failed to parse {}", path.display()));
                    }
                }
            }
        }
    } else {
        if verbose {
            println!("  📄 Parsing entry file: {}", entry.display());
        }
        modules.push(parse_single_module(entry, symbol_table, verbose)?);
    }

    if verbose {
        println!("  ✓ Parsed {} modules", modules.len());
    }

    Ok(modules)
}

/// Check if file should be parsed
fn is_parseable_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| {
        ext == "tsx" || ext == "ts" || ext == "jsx" || ext == "js" || ext == "dx"
    })
}

/// Parse a single module file
fn parse_single_module(
    path: &Path,
    symbol_table: &SymbolTable,
    verbose: bool,
) -> Result<ParsedModule> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}\n  help: Check that the file exists and you have read permissions.", path.display()))?;

    // Security validation
    validate_security(&source, path)?;

    // Compute hash for cache invalidation
    let hash = blake3::hash(source.as_bytes()).to_hex().to_string();

    // Parse using OXC if available, otherwise fall back to regex
    #[cfg(feature = "oxc")]
    let (imports, exports, components) = parse_with_oxc(&source, path, verbose)
        .with_context(|| format!("Failed to parse TSX/JSX in {}", path.display()))?;

    #[cfg(not(feature = "oxc"))]
    let (imports, exports, components) = parse_with_regex(&source, symbol_table, verbose)
        .with_context(|| format!("Failed to parse TSX/JSX in {}", path.display()))?;

    // Auto-import resolution
    let imports = resolve_auto_imports(imports, &components, symbol_table, verbose);

    if verbose && !components.is_empty() {
        println!("    Found {} components in {}", components.len(), path.display());
    }

    Ok(ParsedModule {
        path: path.to_path_buf(),
        imports,
        exports,
        components,
        hash,
    })
}

/// Validate source against security rules
fn validate_security(source: &str, path: &Path) -> Result<()> {
    use crate::errors::DxError;

    for banned in BANNED_KEYWORDS {
        if source.contains(banned) {
            let err = DxError::security_violation(path, *banned, Some(source));
            return Err(anyhow::anyhow!("{}", err.format_detailed()));
        }
    }
    Ok(())
}

/// Parse using OXC (Oxidation Compiler) for accurate AST analysis
#[cfg(feature = "oxc")]
fn parse_with_oxc(
    source: &str,
    path: &Path,
    verbose: bool,
) -> Result<(Vec<ImportDecl>, Vec<ExportDecl>, Vec<Component>)> {
    use crate::errors::DxError;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let allocator = Allocator::default();
    let source_type = SourceType::from_path(path).unwrap_or_default();
    let parser = Parser::new(&allocator, source, source_type);
    let result = parser.parse();

    if !result.errors.is_empty() {
        // Format errors with line/column information
        let mut error_messages = Vec::new();
        for err in &result.errors {
            let err_str = err.to_string();
            // OXC errors typically include span info, but we enhance with suggestions
            let suggestion = crate::errors::suggestions::for_parse_error(&err_str);
            let mut msg = err_str.clone();
            if let Some(sug) = suggestion {
                msg.push_str(&format!("\n  help: {}", sug));
            }
            error_messages.push(msg);
        }

        return Err(anyhow!(
            "Parse errors in {}:\n{}",
            path.display(),
            error_messages.join("\n\n")
        ));
    }

    let program = result.program;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut components = Vec::new();

    // Extract imports
    for stmt in &program.body {
        if let oxc_ast::ast::Statement::ImportDeclaration(import) = stmt {
            let mut specifiers = Vec::new();

            for spec in &import.specifiers {
                match spec {
                    oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                        specifiers.push(ImportSpecifier::Named {
                            local: s.local.name.to_string(),
                            imported: s.imported.name().to_string(),
                        });
                    }
                    oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(s) => {
                        specifiers.push(ImportSpecifier::Default {
                            local: s.local.name.to_string(),
                        });
                    }
                    oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(s) => {
                        specifiers.push(ImportSpecifier::Namespace {
                            local: s.local.name.to_string(),
                        });
                    }
                }
            }

            imports.push(ImportDecl {
                source: import.source.value.to_string(),
                specifiers,
                is_type_only: import.import_kind.is_type(),
            });
        }
    }

    // Extract exports and components
    for stmt in &program.body {
        match stmt {
            oxc_ast::ast::Statement::ExportDefaultDeclaration(export) => {
                if let Some(component) = extract_component_from_export_default(export, source) {
                    exports.push(ExportDecl {
                        name: component.name.clone(),
                        is_default: true,
                        is_type_only: false,
                    });
                    components.push(component);
                }
            }
            oxc_ast::ast::Statement::ExportNamedDeclaration(export) => {
                if let Some(decl) = &export.declaration {
                    if let Some(component) = extract_component_from_declaration(decl, source) {
                        exports.push(ExportDecl {
                            name: component.name.clone(),
                            is_default: false,
                            is_type_only: export.export_kind.is_type(),
                        });
                        components.push(component);
                    }
                }
            }
            oxc_ast::ast::Statement::FunctionDeclaration(func) => {
                if let Some(component) = extract_component_from_function(func, source) {
                    components.push(component);
                }
            }
            oxc_ast::ast::Statement::VariableDeclaration(var_decl) => {
                for decl in &var_decl.declarations {
                    if let Some(component) = extract_component_from_var_decl(decl, source) {
                        components.push(component);
                    }
                }
            }
            _ => {}
        }
    }

    Ok((imports, exports, components))
}

/// Fallback regex-based parser (when OXC is not available)
#[cfg(not(feature = "oxc"))]
fn parse_with_regex(
    source: &str,
    _symbol_table: &SymbolTable,
    _verbose: bool,
) -> Result<(Vec<ImportDecl>, Vec<ExportDecl>, Vec<Component>)> {
    use regex::Regex;

    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut components = Vec::new();

    // Import regex
    let import_re =
        Regex::new(r#"import\s+(?:(?:\{([^}]+)\})|(?:(\w+)))\s+from\s+['"]([^'"]+)['"]"#).unwrap();

    for cap in import_re.captures_iter(source) {
        let source_path = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();
        let mut specifiers = Vec::new();

        if let Some(named) = cap.get(1) {
            for name in named.as_str().split(',') {
                let name = name.trim();
                if !name.is_empty() {
                    let parts: Vec<&str> = name.split(" as ").collect();
                    if parts.len() == 2 {
                        specifiers.push(ImportSpecifier::Named {
                            local: parts[1].trim().to_string(),
                            imported: parts[0].trim().to_string(),
                        });
                    } else {
                        specifiers.push(ImportSpecifier::Named {
                            local: name.to_string(),
                            imported: name.to_string(),
                        });
                    }
                }
            }
        } else if let Some(default) = cap.get(2) {
            specifiers.push(ImportSpecifier::Default {
                local: default.as_str().to_string(),
            });
        }

        imports.push(ImportDecl {
            source: source_path,
            specifiers,
            is_type_only: false,
        });
    }

    // Export regex
    let export_re =
        Regex::new(r"export\s+(default\s+)?(async\s+)?(?:function|const|class)\s+([A-Z]\w*)")
            .unwrap();

    for cap in export_re.captures_iter(source) {
        let is_default = cap.get(1).is_some();
        let name = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_default();

        exports.push(ExportDecl {
            name,
            is_default,
            is_type_only: false,
        });
    }

    // Component regex (functions starting with uppercase)
    let component_re = Regex::new(
        r"(?:export\s+(?:default\s+)?)?(?:async\s+)?(?:function|const)\s+([A-Z]\w*)\s*(?:\(|=)",
    )
    .unwrap();

    for cap in component_re.captures_iter(source) {
        let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();

        // Extract state declarations
        let state = extract_state_regex(source);

        // Extract hooks
        let hooks = extract_hooks_regex(source);

        // Extract JSX body
        let jsx_body = extract_jsx_body_regex(source, &name);

        // Extract props
        let props = extract_props_regex(source, &name);

        components.push(Component {
            name,
            props,
            state,
            jsx_body,
            hooks,
            is_async: source.contains("async function") || source.contains("async ("),
            has_children: source.contains("children") || source.contains("{props.children}"),
        });
    }

    Ok((imports, exports, components))
}

/// Extract state declarations using regex
#[cfg(not(feature = "oxc"))]
fn extract_state_regex(source: &str) -> Vec<StateDef> {
    use regex::Regex;

    let mut states = Vec::new();
    let state_re =
        Regex::new(r"const\s+\[(\w+),\s*(set\w+)\]\s*=\s*useState(?:<([^>]+)>)?\(([^)]*)\)")
            .unwrap();

    for cap in state_re.captures_iter(source) {
        let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let setter_name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
        let type_annotation = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_else(|| {
            // Infer type from initial value
            let init = cap.get(4).map(|m| m.as_str()).unwrap_or("");
            infer_type(init)
        });
        let initial_value = cap.get(4).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

        states.push(StateDef {
            name,
            setter_name,
            initial_value,
            type_annotation,
        });
    }

    states
}

/// Extract hook calls using regex
#[cfg(not(feature = "oxc"))]
fn extract_hooks_regex(source: &str) -> Vec<HookCall> {
    use regex::Regex;

    let mut hooks = Vec::new();
    let hook_re = Regex::new(r"(use\w+)\s*\(([^)]*)\)").unwrap();

    for cap in hook_re.captures_iter(source) {
        let hook_name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
        let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

        // Skip useState as it's handled separately
        if hook_name == "useState" {
            continue;
        }

        // Parse dependencies for useEffect/useMemo/useCallback
        let dependencies =
            if hook_name == "useEffect" || hook_name == "useMemo" || hook_name == "useCallback" {
                extract_dependencies(args_str)
            } else {
                None
            };

        hooks.push(HookCall {
            hook_name,
            args: vec![args_str.to_string()],
            dependencies,
        });
    }

    hooks
}

/// Extract dependencies array from hook arguments
#[cfg(not(feature = "oxc"))]
fn extract_dependencies(args: &str) -> Option<Vec<String>> {
    use regex::Regex;

    let deps_re = Regex::new(r"\[([^\]]*)\]\s*$").unwrap();
    deps_re.captures(args).map(|cap| {
        cap.get(1)
            .map(|m| m.as_str())
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

/// Extract JSX body from component
#[cfg(not(feature = "oxc"))]
fn extract_jsx_body_regex(source: &str, component_name: &str) -> String {
    use regex::Regex;

    // Try to find return statement with JSX
    let pattern = format!(
        r"(?s)(?:function|const)\s+{}\s*[^{{]*\{{.*?return\s*\(?\s*(<[^;]+>)\s*\)?;",
        regex::escape(component_name)
    );

    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(source) {
            return cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        }
    }

    String::new()
}

/// Extract props from component
#[cfg(not(feature = "oxc"))]
fn extract_props_regex(source: &str, component_name: &str) -> Vec<PropDef> {
    use regex::Regex;

    let mut props = Vec::new();

    // Look for destructured props pattern
    let pattern = format!(
        r"(?:function|const)\s+{}\s*\(\s*\{{\s*([^}}]+)\s*\}}",
        regex::escape(component_name)
    );

    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(source) {
            let props_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            for prop in props_str.split(',') {
                let prop = prop.trim();
                if prop.is_empty() {
                    continue;
                }

                // Handle default values: name = defaultValue
                let (name, default_value) = if prop.contains('=') {
                    let parts: Vec<&str> = prop.splitn(2, '=').collect();
                    (parts[0].trim().to_string(), Some(parts[1].trim().to_string()))
                } else {
                    (prop.to_string(), None)
                };

                // Handle optional props: name?
                let (name, is_optional) = if name.ends_with('?') {
                    (name.trim_end_matches('?').to_string(), true)
                } else {
                    (name, default_value.is_some())
                };

                props.push(PropDef {
                    name,
                    type_annotation: "unknown".to_string(),
                    is_optional,
                    default_value,
                });
            }
        }
    }

    props
}

/// Infer type from value
fn infer_type(value: &str) -> String {
    let value = value.trim();

    if value.starts_with('"') || value.starts_with('\'') || value.starts_with('`') {
        "string".to_string()
    } else if value == "true" || value == "false" {
        "boolean".to_string()
    } else if value.starts_with('[') {
        "array".to_string()
    } else if value.starts_with('{') {
        "object".to_string()
    } else if value.parse::<f64>().is_ok() {
        "number".to_string()
    } else if value == "null" {
        "null".to_string()
    } else if value == "undefined" {
        "undefined".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Resolve auto-imports based on symbol table
fn resolve_auto_imports(
    mut imports: Vec<ImportDecl>,
    components: &[Component],
    symbol_table: &SymbolTable,
    verbose: bool,
) -> Vec<ImportDecl> {
    // Collect all JSX tags used in components
    let mut used_tags: HashSet<String> = HashSet::new();

    for component in components {
        // Simple regex to find JSX tags
        let tag_re = regex::Regex::new(r"<([A-Z]\w*)").unwrap();
        for cap in tag_re.captures_iter(&component.jsx_body) {
            if let Some(tag) = cap.get(1) {
                used_tags.insert(tag.as_str().to_string());
            }
        }
    }

    // Check which tags need auto-import
    let already_imported: HashSet<String> = imports
        .iter()
        .flat_map(|i| i.specifiers.iter())
        .filter_map(|s| match s {
            ImportSpecifier::Named { local, .. } => Some(local.clone()),
            ImportSpecifier::Default { local } => Some(local.clone()),
            ImportSpecifier::Namespace { .. } => None,
        })
        .collect();

    for tag in used_tags {
        if already_imported.contains(&tag) {
            continue;
        }

        // Look up in symbol table
        if let Some(path) = symbol_table.components.get(&tag) {
            if verbose {
                println!("    ✨ Auto-importing {} from {}", tag, path.display());
            }

            imports.push(ImportDecl {
                source: path.to_string_lossy().to_string(),
                specifiers: vec![ImportSpecifier::Default { local: tag }],
                is_type_only: false,
            });
        }
    }

    imports
}

/// Tree shake unused imports and exports
pub fn tree_shake(modules: Vec<ParsedModule>, verbose: bool) -> Result<Vec<ParsedModule>> {
    if verbose {
        println!("  🌳 Tree shaking unused code...");
    }

    // Build usage graph
    let mut used_exports: HashSet<(PathBuf, String)> = HashSet::new();
    let mut used_imports: HashSet<(PathBuf, String)> = HashSet::new();

    // Mark entry points as used
    for module in &modules {
        for export in &module.exports {
            if export.is_default {
                used_exports.insert((module.path.clone(), export.name.clone()));
            }
        }
    }

    // Propagate usage through imports
    let mut changed = true;
    while changed {
        changed = false;

        for module in &modules {
            for import in &module.imports {
                // Find the source module
                let source_path = resolve_import_path(&module.path, &import.source);

                for spec in &import.specifiers {
                    let local_name = match spec {
                        ImportSpecifier::Named { local, .. } => local,
                        ImportSpecifier::Default { local } => local,
                        ImportSpecifier::Namespace { local } => local,
                    };

                    // Check if this import is used in any component
                    let is_used = module.components.iter().any(|c| {
                        c.jsx_body.contains(local_name)
                            || c.state.iter().any(|s| s.initial_value.contains(local_name))
                            || c.hooks.iter().any(|h| h.args.iter().any(|a| a.contains(local_name)))
                    });

                    if is_used {
                        let key = (module.path.clone(), local_name.clone());
                        if used_imports.insert(key) {
                            changed = true;

                            // Mark the export as used
                            let export_name = match spec {
                                ImportSpecifier::Named { imported, .. } => imported.clone(),
                                ImportSpecifier::Default { .. } => "default".to_string(),
                                ImportSpecifier::Namespace { .. } => "*".to_string(),
                            };

                            if let Some(source_path) = &source_path {
                                used_exports.insert((source_path.clone(), export_name));
                            }
                        }
                    }
                }
            }
        }
    }

    // Filter modules
    let shaken_modules: Vec<ParsedModule> = modules
        .into_iter()
        .map(|mut module| {
            // Remove unused imports
            module.imports.retain(|import| {
                import.specifiers.iter().any(|spec| {
                    let local = match spec {
                        ImportSpecifier::Named { local, .. } => local,
                        ImportSpecifier::Default { local } => local,
                        ImportSpecifier::Namespace { local } => local,
                    };
                    used_imports.contains(&(module.path.clone(), local.clone()))
                })
            });

            module
        })
        .collect();

    if verbose {
        let total_imports: usize = shaken_modules.iter().map(|m| m.imports.len()).sum();
        println!("    ✓ {} imports remaining after tree shaking", total_imports);
    }

    Ok(shaken_modules)
}

/// Resolve import path relative to importing module
fn resolve_import_path(from: &Path, import_source: &str) -> Option<PathBuf> {
    if import_source.starts_with('.') {
        // Relative import
        let parent = from.parent()?;
        let resolved = parent.join(import_source);

        // Try with extensions
        for ext in &["tsx", "ts", "jsx", "js", "dx"] {
            let with_ext = resolved.with_extension(ext);
            if with_ext.exists() {
                return Some(with_ext);
            }
        }

        // Try index file
        for ext in &["tsx", "ts", "jsx", "js"] {
            let index = resolved.join(format!("index.{}", ext));
            if index.exists() {
                return Some(index);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banned_keywords_detection() {
        let source = r#"
            function App() {
                eval("dangerous code");
                return <div>Hello</div>;
            }
        "#;

        let result = validate_security(source, Path::new("test.tsx"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("eval"));
    }

    #[test]
    fn test_safe_code_passes() {
        let source = r#"
            function App() {
                const [count, setCount] = useState(0);
                return <div>{count}</div>;
            }
        "#;

        let result = validate_security(source, Path::new("test.tsx"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_infer_type() {
        assert_eq!(infer_type("\"hello\""), "string");
        assert_eq!(infer_type("'world'"), "string");
        assert_eq!(infer_type("true"), "boolean");
        assert_eq!(infer_type("false"), "boolean");
        assert_eq!(infer_type("42"), "number");
        assert_eq!(infer_type("3.14"), "number");
        assert_eq!(infer_type("[]"), "array");
        assert_eq!(infer_type("{}"), "object");
        assert_eq!(infer_type("null"), "null");
        assert_eq!(infer_type("undefined"), "undefined");
    }

    #[cfg(not(feature = "oxc"))]
    #[test]
    fn test_extract_state_regex() {
        let source = r#"
            const [count, setCount] = useState(0);
            const [name, setName] = useState<string>("John");
            const [items, setItems] = useState([]);
        "#;

        let states = extract_state_regex(source);
        assert_eq!(states.len(), 3);
        assert_eq!(states[0].name, "count");
        assert_eq!(states[0].setter_name, "setCount");
        assert_eq!(states[0].initial_value, "0");
        assert_eq!(states[1].name, "name");
        assert_eq!(states[1].type_annotation, "string");
        assert_eq!(states[2].name, "items");
    }

    #[cfg(not(feature = "oxc"))]
    #[test]
    fn test_extract_hooks_regex() {
        let source = r#"
            useEffect(() => {
                console.log("mounted");
            }, []);
            const memoized = useMemo(() => expensive(), [dep1, dep2]);
        "#;

        let hooks = extract_hooks_regex(source);
        assert!(hooks.iter().any(|h| h.hook_name == "useEffect"));
        assert!(hooks.iter().any(|h| h.hook_name == "useMemo"));
    }
}

// OXC helper functions for component extraction
#[cfg(feature = "oxc")]
fn extract_component_from_export_default(
    export: &oxc_ast::ast::ExportDefaultDeclaration,
    source: &str,
) -> Option<Component> {
    use oxc_ast::ast::ExportDefaultDeclarationKind;

    match &export.declaration {
        ExportDefaultDeclarationKind::FunctionDeclaration(func) => {
            let name = func.id.as_ref()?.name.to_string();
            if !name.chars().next()?.is_uppercase() {
                return None;
            }
            Some(create_component_from_function_body(&name, func.r#async, source))
        }
        ExportDefaultDeclarationKind::Identifier(ident) => {
            let name = ident.name.to_string();
            if !name.chars().next()?.is_uppercase() {
                return None;
            }
            Some(create_component_from_function_body(&name, false, source))
        }
        _ => None,
    }
}

#[cfg(feature = "oxc")]
fn extract_component_from_declaration(
    decl: &oxc_ast::ast::Declaration,
    source: &str,
) -> Option<Component> {
    use oxc_ast::ast::Declaration;

    match decl {
        Declaration::FunctionDeclaration(func) => {
            let name = func.id.as_ref()?.name.to_string();
            if !name.chars().next()?.is_uppercase() {
                return None;
            }
            Some(create_component_from_function_body(&name, func.r#async, source))
        }
        Declaration::VariableDeclaration(var_decl) => {
            for decl in &var_decl.declarations {
                if let Some(component) = extract_component_from_var_decl(decl, source) {
                    return Some(component);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(feature = "oxc")]
fn extract_component_from_function(
    func: &oxc_ast::ast::Function,
    source: &str,
) -> Option<Component> {
    let name = func.id.as_ref()?.name.to_string();
    if !name.chars().next()?.is_uppercase() {
        return None;
    }
    Some(create_component_from_function_body(&name, func.r#async, source))
}

#[cfg(feature = "oxc")]
fn extract_component_from_var_decl(
    decl: &oxc_ast::ast::VariableDeclarator,
    source: &str,
) -> Option<Component> {
    use oxc_ast::ast::BindingPatternKind;

    let name = match &decl.id.kind {
        BindingPatternKind::BindingIdentifier(ident) => ident.name.to_string(),
        _ => return None,
    };

    if !name.chars().next()?.is_uppercase() {
        return None;
    }

    // Check if init is an arrow function or function expression
    let is_async = decl.init.as_ref().map_or(false, |init| {
        matches!(
            init,
            oxc_ast::ast::Expression::ArrowFunctionExpression(f) if f.r#async
        ) || matches!(
            init,
            oxc_ast::ast::Expression::FunctionExpression(f) if f.r#async
        )
    });

    Some(create_component_from_function_body(&name, is_async, source))
}

#[cfg(feature = "oxc")]
fn create_component_from_function_body(name: &str, is_async: bool, source: &str) -> Component {
    // Use regex to extract state, hooks, and JSX from source
    // This is a simplified approach - full AST traversal would be more accurate
    let state = extract_state_from_source(source);
    let hooks = extract_hooks_from_source(source);
    let jsx_body = extract_jsx_from_source(source, name);
    let props = extract_props_from_source(source, name);
    let has_children = source.contains("children") || source.contains("{props.children}");

    Component {
        name: name.to_string(),
        props,
        state,
        jsx_body,
        hooks,
        is_async,
        has_children,
    }
}

#[cfg(feature = "oxc")]
fn extract_state_from_source(source: &str) -> Vec<StateDef> {
    use regex::Regex;

    let mut states = Vec::new();
    let state_re =
        Regex::new(r"const\s+\[(\w+),\s*(set\w+)\]\s*=\s*useState(?:<([^>]+)>)?\(([^)]*)\)").ok();

    if let Some(re) = state_re {
        for cap in re.captures_iter(source) {
            let name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let setter_name = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
            let type_annotation = cap.get(3).map(|m| m.as_str().to_string()).unwrap_or_else(|| {
                let init = cap.get(4).map(|m| m.as_str()).unwrap_or("");
                infer_type(init)
            });
            let initial_value =
                cap.get(4).map(|m| m.as_str().trim().to_string()).unwrap_or_default();

            states.push(StateDef {
                name,
                setter_name,
                initial_value,
                type_annotation,
            });
        }
    }

    states
}

#[cfg(feature = "oxc")]
fn extract_hooks_from_source(source: &str) -> Vec<HookCall> {
    use regex::Regex;

    let mut hooks = Vec::new();
    let hook_re = Regex::new(r"(use\w+)\s*\(([^)]*)\)").ok();

    if let Some(re) = hook_re {
        for cap in re.captures_iter(source) {
            let hook_name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            let args_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            if hook_name == "useState" {
                continue;
            }

            let dependencies =
                if hook_name == "useEffect" || hook_name == "useMemo" || hook_name == "useCallback"
                {
                    extract_deps_from_args(args_str)
                } else {
                    None
                };

            hooks.push(HookCall {
                hook_name,
                args: vec![args_str.to_string()],
                dependencies,
            });
        }
    }

    hooks
}

#[cfg(feature = "oxc")]
fn extract_deps_from_args(args: &str) -> Option<Vec<String>> {
    use regex::Regex;

    let deps_re = Regex::new(r"\[([^\]]*)\]\s*$").ok()?;
    deps_re.captures(args).map(|cap| {
        cap.get(1)
            .map(|m| m.as_str())
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

#[cfg(feature = "oxc")]
fn extract_jsx_from_source(source: &str, component_name: &str) -> String {
    use regex::Regex;

    let pattern = format!(
        r"(?s)(?:function|const)\s+{}\s*[^{{]*\{{.*?return\s*\(?\s*(<[^;]+>)\s*\)?;",
        regex::escape(component_name)
    );

    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(source) {
            return cap.get(1).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
        }
    }

    String::new()
}

#[cfg(feature = "oxc")]
fn extract_props_from_source(source: &str, component_name: &str) -> Vec<PropDef> {
    use regex::Regex;

    let mut props = Vec::new();

    let pattern = format!(
        r"(?:function|const)\s+{}\s*\(\s*\{{\s*([^}}]+)\s*\}}",
        regex::escape(component_name)
    );

    if let Ok(re) = Regex::new(&pattern) {
        if let Some(cap) = re.captures(source) {
            let props_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");

            for prop in props_str.split(',') {
                let prop = prop.trim();
                if prop.is_empty() {
                    continue;
                }

                let (name, default_value) = if prop.contains('=') {
                    let parts: Vec<&str> = prop.splitn(2, '=').collect();
                    (parts[0].trim().to_string(), Some(parts[1].trim().to_string()))
                } else {
                    (prop.to_string(), None)
                };

                let (name, is_optional) = if name.ends_with('?') {
                    (name.trim_end_matches('?').to_string(), true)
                } else {
                    (name, default_value.is_some())
                };

                props.push(PropDef {
                    name,
                    type_annotation: "unknown".to_string(),
                    is_optional,
                    default_value,
                });
            }
        }
    }

    props
}
