//! Atomizer - Converts JavaScript/TypeScript to DXM binary format
//!
//! This runs at install time (or lazily) to pre-compile node_modules.
//! The result is a .dxm file ready for zero-parse bundling.

use crate::format::{fnv1a_hash, DxmModule};
use std::fs;
use std::path::Path;

/// Atomizer configuration
#[derive(Debug, Clone)]
pub struct AtomizerConfig {
    /// Minify the output
    pub minify: bool,
    /// Strip comments
    pub strip_comments: bool,
    /// Preserve source maps
    pub source_maps: bool,
}

impl Default for AtomizerConfig {
    fn default() -> Self {
        Self {
            minify: true,
            strip_comments: true,
            source_maps: false,
        }
    }
}

/// Result of atomization
#[derive(Debug)]
pub struct AtomizeResult {
    /// The DXM module
    pub module: DxmModule,
    /// List of export names
    pub exports: Vec<String>,
    /// List of import specifiers
    pub imports: Vec<String>,
    /// Original size in bytes
    pub original_size: usize,
    /// Atomized size in bytes
    pub atomized_size: usize,
}

/// Atomize a JavaScript/TypeScript source into DXM format
pub fn atomize(source: &str, config: &AtomizerConfig) -> AtomizeResult {
    let source_hash = fnv1a_hash(source);
    let mut module = DxmModule::new(source_hash);

    // Step 1: Extract exports
    let exports = extract_exports(source);

    // Step 2: Extract imports
    let imports = extract_imports(source);

    // Step 3: Process the body (minify, strip, etc.)
    let processed = process_body(source, config);

    // Step 4: Calculate export offsets (simplified - real impl would parse AST)
    for export_name in exports.iter() {
        // Find the export in the processed code
        if let Some(offset) = find_export_offset(&processed, export_name) {
            module.add_export(export_name, offset as u32, 100); // Simplified length
        }
    }

    // Step 5: Set the body
    let body_bytes = processed.into_bytes();
    let atomized_size = module.total_size() + body_bytes.len();

    module.set_body(body_bytes);

    AtomizeResult {
        module,
        exports,
        imports,
        original_size: source.len(),
        atomized_size,
    }
}

/// Atomize an npm package from node_modules
pub fn atomize_package(
    package_path: &Path,
    config: &AtomizerConfig,
) -> Result<AtomizeResult, String> {
    // Read package.json to find entry point
    let pkg_json_path = package_path.join("package.json");
    if !pkg_json_path.exists() {
        return Err(format!("No package.json found in {:?}", package_path));
    }

    let pkg_json = fs::read_to_string(&pkg_json_path)
        .map_err(|e| format!("Failed to read package.json: {}", e))?;

    // Parse entry point (simplified - real impl would use serde_json)
    let main = extract_main_from_package_json(&pkg_json).unwrap_or("index.js".to_string());

    let entry_path = package_path.join(&main);
    if !entry_path.exists() {
        return Err(format!("Entry point not found: {:?}", entry_path));
    }

    let source = fs::read_to_string(&entry_path)
        .map_err(|e| format!("Failed to read entry point: {}", e))?;

    Ok(atomize(&source, config))
}

/// Write atomized module to .dxm file
pub fn write_dxm(module: &DxmModule, output_path: &Path) -> Result<(), String> {
    let bytes = module.to_bytes();
    fs::write(output_path, bytes).map_err(|e| format!("Failed to write DXM: {}", e))
}

/// Read DXM module from file
pub fn read_dxm(path: &Path) -> Result<DxmModule, String> {
    let bytes = fs::read(path).map_err(|e| format!("Failed to read DXM: {}", e))?;
    DxmModule::from_bytes(&bytes).map_err(|e| e.to_string())
}

// ============== Internal Functions ==============

/// Extract export names from source
fn extract_exports(source: &str) -> Vec<String> {
    let mut exports = Vec::new();

    // Pattern: export { name1, name2 }
    // Pattern: export const/let/var/function/class name
    // Pattern: export default

    for line in source.lines() {
        let trimmed = line.trim();

        // export { ... }
        if trimmed.starts_with("export {") {
            if let Some(start) = trimmed.find('{') {
                if let Some(end) = trimmed.find('}') {
                    let names = &trimmed[start + 1..end];
                    for name in names.split(',') {
                        let name = name.trim();
                        // Handle "name as alias" syntax
                        let name = name.split(" as ").next().unwrap_or(name).trim();
                        if !name.is_empty() {
                            exports.push(name.to_string());
                        }
                    }
                }
            }
        }
        // export const/let/var/function/class name
        else if trimmed.starts_with("export const ")
            || trimmed.starts_with("export let ")
            || trimmed.starts_with("export var ")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("export class ")
        {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[2].trim_end_matches(['=', '(', ')', '{']);
                exports.push(name.to_string());
            }
        }
        // export default
        else if trimmed.starts_with("export default") {
            exports.push("default".to_string());
        }
    }

    exports
}

/// Extract import specifiers from source
fn extract_imports(source: &str) -> Vec<String> {
    let mut imports = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // import ... from 'module'
        if trimmed.starts_with("import ") {
            if let Some(from_idx) = trimmed.find(" from ") {
                let module_part = &trimmed[from_idx + 7..];
                // Extract module name from quotes
                let module = module_part
                    .trim()
                    .trim_end_matches(';')
                    .trim_matches(|c| c == '\'' || c == '"');
                imports.push(module.to_string());
            }
        }
        // require('module')
        else if trimmed.contains("require(") {
            if let Some(start) = trimmed.find("require(") {
                let rest = &trimmed[start + 8..];
                if let Some(end) = rest.find(')') {
                    let module = rest[..end].trim_matches(|c| c == '\'' || c == '"');
                    imports.push(module.to_string());
                }
            }
        }
    }

    imports
}

/// Process the body (minify, strip comments, etc.)
fn process_body(source: &str, config: &AtomizerConfig) -> String {
    let mut result = source.to_string();

    if config.strip_comments {
        result = strip_comments(&result);
    }

    if config.minify {
        result = minify_js(&result);
    }

    result
}

/// Strip comments from JavaScript
fn strip_comments(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Single line comment
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            // Skip until newline
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        }
        // Multi-line comment
        else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2; // Skip */
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Basic JavaScript minification
fn minify_js(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut last_was_space = false;
    let mut in_string = false;
    let mut string_char = '"';

    for ch in source.chars() {
        if in_string {
            result.push(ch);
            if ch == string_char {
                in_string = false;
            }
        } else if ch == '"' || ch == '\'' || ch == '`' {
            in_string = true;
            string_char = ch;
            result.push(ch);
        } else if ch.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(ch);
            last_was_space = false;
        }
    }

    // Remove unnecessary spaces
    result = result
        .replace(" ;", ";")
        .replace("; ", ";")
        .replace(" {", "{")
        .replace("{ ", "{")
        .replace(" }", "}")
        .replace("} ", "}")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(") ", ")")
        .replace(" =", "=")
        .replace("= ", "=")
        .replace(" ,", ",")
        .replace(", ", ",");

    result
}

/// Find the offset of an export in the code
fn find_export_offset(code: &str, export_name: &str) -> Option<usize> {
    // Look for function/const/class definitions
    let patterns = [
        format!("function {}", export_name),
        format!("const {}=", export_name),
        format!("const {} =", export_name),
        format!("let {}=", export_name),
        format!("var {}=", export_name),
        format!("class {}", export_name),
    ];

    for pattern in &patterns {
        if let Some(offset) = code.find(pattern.as_str()) {
            return Some(offset);
        }
    }

    None
}

/// Extract main entry point from package.json
fn extract_main_from_package_json(json: &str) -> Option<String> {
    // Look for "main": "..." or "module": "..." or "exports": {...}
    // Simplified parser - real impl would use serde_json

    // Try "module" first (ESM)
    if let Some(idx) = json.find("\"module\"") {
        if let Some(start) = json[idx..].find(':') {
            let rest = &json[idx + start + 1..];
            let rest = rest.trim();
            if let Some(quote_start) = rest.find('"') {
                let rest = &rest[quote_start + 1..];
                if let Some(quote_end) = rest.find('"') {
                    return Some(rest[..quote_end].to_string());
                }
            }
        }
    }

    // Fall back to "main"
    if let Some(idx) = json.find("\"main\"") {
        if let Some(start) = json[idx..].find(':') {
            let rest = &json[idx + start + 1..];
            let rest = rest.trim();
            if let Some(quote_start) = rest.find('"') {
                let rest = &rest[quote_start + 1..];
                if let Some(quote_end) = rest.find('"') {
                    return Some(rest[..quote_end].to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_exports() {
        let source = r#"
            export const useState = () => {};
            export function useEffect() {}
            export { memo, useCallback };
            export default function App() {}
        "#;

        let exports = extract_exports(source);
        assert!(exports.contains(&"useState".to_string()));
        assert!(exports.contains(&"useEffect".to_string()));
        assert!(exports.contains(&"memo".to_string()));
        assert!(exports.contains(&"useCallback".to_string()));
        assert!(exports.contains(&"default".to_string()));
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
            import React from 'react';
            import { useState } from 'react';
            const fs = require('fs');
        "#;

        let imports = extract_imports(source);
        assert!(imports.contains(&"react".to_string()));
        assert!(imports.contains(&"fs".to_string()));
    }

    #[test]
    fn test_atomize() {
        let source = r#"
            export const useState = (initial) => {
                let state = initial;
                return [state, (v) => { state = v; }];
            };
        "#;

        let config = AtomizerConfig::default();
        let result = atomize(source, &config);

        assert_eq!(result.exports.len(), 1);
        assert_eq!(result.exports[0], "useState");
        assert!(result.atomized_size < result.original_size);
    }
}
