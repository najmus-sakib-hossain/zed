//! # Script Parser
//!
//! Parses the `<script>` section of component files.

use std::path::Path;

use super::{Export, ExportKind};
use crate::config::ScriptLanguage;
use crate::error::DxResult;

// =============================================================================
// Parsed Script
// =============================================================================

/// A parsed script section.
#[derive(Debug, Clone)]
pub struct ParsedScript {
    /// Source code
    pub source: String,

    /// Script language
    pub language: ScriptLanguage,

    /// Whether this script has a data loader function
    pub has_data_loader: bool,

    /// Whether this script has a Props type
    pub has_props: bool,

    /// Exported items
    pub exports: Vec<Export>,

    /// Import statements
    pub imports: Vec<Import>,

    /// Detected functions
    pub functions: Vec<FunctionInfo>,
}

/// An import statement.
#[derive(Debug, Clone)]
pub struct Import {
    /// The module being imported
    pub module: String,
    /// Items being imported
    pub items: Vec<String>,
    /// Whether this is a default import
    pub is_default: bool,
}

/// Information about a function.
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Whether the function is async
    pub is_async: bool,
    /// Whether the function is public
    pub is_public: bool,
    /// Parameter count
    pub param_count: usize,
}

// =============================================================================
// Script Parser
// =============================================================================

/// Parser for script sections.
#[derive(Debug, Default)]
pub struct ScriptParser {
    // Configuration options can be added here
}

impl ScriptParser {
    /// Create a new script parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a script section.
    pub fn parse(
        &self,
        source: &str,
        language: ScriptLanguage,
        _path: &Path,
    ) -> DxResult<ParsedScript> {
        let source = source.trim().to_string();

        // Analyze the script based on language
        let (exports, imports, functions) = match language {
            ScriptLanguage::Rust => self.analyze_rust(&source),
            ScriptLanguage::TypeScript | ScriptLanguage::JavaScript => self.analyze_js(&source),
            ScriptLanguage::Python => self.analyze_python(&source),
            ScriptLanguage::Go => self.analyze_go(&source),
        };

        // Check for data loader
        let has_data_loader = functions.iter().any(|f| f.name == "load" && f.is_async);

        // Check for Props
        let has_props = exports.iter().any(|e| e.kind == ExportKind::Props);

        Ok(ParsedScript {
            source,
            language,
            has_data_loader,
            has_props,
            exports,
            imports,
            functions,
        })
    }

    /// Analyze Rust script.
    fn analyze_rust(&self, source: &str) -> (Vec<Export>, Vec<Import>, Vec<FunctionInfo>) {
        let mut exports = Vec::new();
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            // Check for imports
            if trimmed.starts_with("use ") {
                if let Some(module) = trimmed.strip_prefix("use ").and_then(|s| s.strip_suffix(';'))
                {
                    imports.push(Import {
                        module: module.to_string(),
                        items: Vec::new(),
                        is_default: false,
                    });
                }
            }

            // Check for public structs (Props)
            if trimmed.starts_with("pub struct Props") {
                exports.push(Export {
                    name: "Props".to_string(),
                    kind: ExportKind::Props,
                });
            }

            // Check for public functions
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn ") {
                let is_async = trimmed.contains("async");
                let name = self.extract_rust_fn_name(trimmed);
                if let Some(name) = name {
                    let kind = if name == "load" {
                        ExportKind::DataLoader
                    } else {
                        ExportKind::Function
                    };

                    exports.push(Export {
                        name: name.clone(),
                        kind,
                    });

                    functions.push(FunctionInfo {
                        name,
                        is_async,
                        is_public: true,
                        param_count: 0, // Simplified
                    });
                }
            }
        }

        (exports, imports, functions)
    }

    /// Extract function name from Rust function declaration.
    fn extract_rust_fn_name(&self, line: &str) -> Option<String> {
        let line = line.trim_start_matches("pub ");
        let line = line.trim_start_matches("async ");
        let line = line.trim_start_matches("fn ");

        let name_end = line.find('(')?;
        Some(line[..name_end].to_string())
    }

    /// Analyze JavaScript/TypeScript script.
    fn analyze_js(&self, source: &str) -> (Vec<Export>, Vec<Import>, Vec<FunctionInfo>) {
        let mut exports = Vec::new();
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            // Check for imports
            if trimmed.starts_with("import ") {
                imports.push(Import {
                    module: trimmed.to_string(),
                    items: Vec::new(),
                    is_default: false,
                });
            }

            // Check for exported functions
            if trimmed.starts_with("export ") {
                if trimmed.contains("function") || trimmed.contains("const") {
                    let is_async = trimmed.contains("async");
                    let name = self.extract_js_export_name(trimmed);
                    if let Some(name) = name {
                        let kind = if name == "load" {
                            ExportKind::DataLoader
                        } else {
                            ExportKind::Function
                        };

                        exports.push(Export {
                            name: name.clone(),
                            kind,
                        });

                        functions.push(FunctionInfo {
                            name,
                            is_async,
                            is_public: true,
                            param_count: 0,
                        });
                    }
                }

                // Check for exported types
                if trimmed.contains("interface") || trimmed.contains("type") {
                    if trimmed.contains("Props") {
                        exports.push(Export {
                            name: "Props".to_string(),
                            kind: ExportKind::Props,
                        });
                    }
                }
            }
        }

        (exports, imports, functions)
    }

    /// Extract export name from JavaScript.
    fn extract_js_export_name(&self, line: &str) -> Option<String> {
        // Handle: export function name(...) or export async function name(...)
        if line.contains("function ") {
            let start = line.find("function ")? + 9;
            let rest = &line[start..];
            let end = rest.find('(')?;
            return Some(rest[..end].trim().to_string());
        }

        // Handle: export const name = ...
        if line.contains("const ") {
            let start = line.find("const ")? + 6;
            let rest = &line[start..];
            let end = rest.find(|c| c == ' ' || c == '=')?;
            return Some(rest[..end].trim().to_string());
        }

        None
    }

    /// Analyze Python script.
    fn analyze_python(&self, source: &str) -> (Vec<Export>, Vec<Import>, Vec<FunctionInfo>) {
        let mut exports = Vec::new();
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            // Check for imports
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                imports.push(Import {
                    module: trimmed.to_string(),
                    items: Vec::new(),
                    is_default: false,
                });
            }

            // Check for function definitions
            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                let is_async = trimmed.starts_with("async");
                let name = self.extract_python_fn_name(trimmed);
                if let Some(name) = name {
                    // Public functions don't start with underscore
                    let is_public = !name.starts_with('_');

                    let kind = if name == "load" {
                        ExportKind::DataLoader
                    } else {
                        ExportKind::Function
                    };

                    if is_public {
                        exports.push(Export {
                            name: name.clone(),
                            kind,
                        });
                    }

                    functions.push(FunctionInfo {
                        name,
                        is_async,
                        is_public,
                        param_count: 0,
                    });
                }
            }

            // Check for class Props
            if trimmed.starts_with("class Props") {
                exports.push(Export {
                    name: "Props".to_string(),
                    kind: ExportKind::Props,
                });
            }
        }

        (exports, imports, functions)
    }

    /// Extract function name from Python.
    fn extract_python_fn_name(&self, line: &str) -> Option<String> {
        let line = line.trim_start_matches("async ");
        let line = line.trim_start_matches("def ");
        let end = line.find('(')?;
        Some(line[..end].to_string())
    }

    /// Analyze Go script.
    fn analyze_go(&self, source: &str) -> (Vec<Export>, Vec<Import>, Vec<FunctionInfo>) {
        let mut exports = Vec::new();
        let mut imports = Vec::new();
        let mut functions = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            // Check for imports
            if trimmed.starts_with("import ") {
                imports.push(Import {
                    module: trimmed.to_string(),
                    items: Vec::new(),
                    is_default: false,
                });
            }

            // Check for function definitions
            if trimmed.starts_with("func ") {
                let name = self.extract_go_fn_name(trimmed);
                if let Some(name) = name {
                    // Public functions start with uppercase
                    let is_public = name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

                    let kind = if name == "Load" {
                        ExportKind::DataLoader
                    } else {
                        ExportKind::Function
                    };

                    if is_public {
                        exports.push(Export {
                            name: name.clone(),
                            kind,
                        });
                    }

                    functions.push(FunctionInfo {
                        name,
                        is_async: false, // Go uses goroutines differently
                        is_public,
                        param_count: 0,
                    });
                }
            }

            // Check for Props struct
            if trimmed.starts_with("type Props struct") {
                exports.push(Export {
                    name: "Props".to_string(),
                    kind: ExportKind::Props,
                });
            }
        }

        (exports, imports, functions)
    }

    /// Extract function name from Go.
    fn extract_go_fn_name(&self, line: &str) -> Option<String> {
        let line = line.trim_start_matches("func ");
        let end = line.find('(')?;
        Some(line[..end].to_string())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_parse_rust_script() {
        let parser = ScriptParser::new();
        let source = r#"
use serde::{Deserialize, Serialize};

pub struct Props {
    title: String,
}

pub async fn load() -> Props {
    Props { title: "Hello".into() }
}

pub fn on_click() {
    // handler
}
"#;

        let result = parser.parse(source, ScriptLanguage::Rust, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.has_data_loader);
        assert!(script.has_props);
        assert!(script.exports.iter().any(|e| e.name == "Props"));
        assert!(script.exports.iter().any(|e| e.name == "load"));
        assert!(script.exports.iter().any(|e| e.name == "on_click"));
    }

    #[test]
    fn test_parse_js_script() {
        let parser = ScriptParser::new();
        let source = r#"
import { useState } from 'react';

export interface Props {
    title: string;
}

export async function load() {
    return { title: "Hello" };
}

export const handleClick = () => {};
"#;

        let result = parser.parse(source, ScriptLanguage::TypeScript, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.has_data_loader);
    }

    #[test]
    fn test_parse_python_script() {
        let parser = ScriptParser::new();
        let source = r#"
from dataclasses import dataclass

@dataclass
class Props:
    title: str

async def load():
    return Props(title="Hello")

def on_click():
    pass
"#;

        let result = parser.parse(source, ScriptLanguage::Python, &PathBuf::from("test.pg"));
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.has_data_loader);
    }
}
