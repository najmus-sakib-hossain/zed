//! # Script Compiler
//!
//! Compiles component scripts from multiple languages (Rust, Python, JavaScript, Go)
//! into binary format for WASM or native execution.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::config::{DxConfig, ScriptLanguage};
use crate::error::{DxError, DxResult};
use crate::parser::ParsedScript;

/// Script binary format magic number
const SCRIPT_MAGIC: &[u8; 4] = b"DXS1";

/// Script binary version
const SCRIPT_VERSION: u8 = 1;

/// Instruction opcodes for the script VM
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptOpcode {
    Nop = 0x00,
    PushConst = 0x01,
    PushLocal = 0x02,
    PopLocal = 0x03,
    Dup = 0x04,
    Pop = 0x05,
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,
    Mod = 0x14,
    Neg = 0x15,
    Eq = 0x20,
    Ne = 0x21,
    Lt = 0x22,
    Le = 0x23,
    Gt = 0x24,
    Ge = 0x25,
    And = 0x30,
    Or = 0x31,
    Not = 0x32,
    Jump = 0x40,
    JumpIf = 0x41,
    JumpIfNot = 0x42,
    Call = 0x43,
    Return = 0x44,
    GetProp = 0x50,
    SetProp = 0x51,
    GetIndex = 0x52,
    SetIndex = 0x53,
    BindState = 0x60,
    UpdateState = 0x61,
    EmitEvent = 0x62,
    Subscribe = 0x63,
    CallNative = 0xF0,
    Breakpoint = 0xFE,
    Halt = 0xFF,
}

/// Compiles parsed scripts to binary format.
pub struct ScriptCompiler {
    config: DxConfig,
    string_pool: Vec<String>,
    string_map: HashMap<String, u32>,
}

/// A constant value in the constant pool.
#[derive(Debug, Clone)]
pub enum Constant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(u32),
}

/// Compiled function metadata.
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub param_count: u8,
    pub local_count: u8,
    pub bytecode_offset: u32,
    pub bytecode_length: u32,
    pub is_data_loader: bool,
    pub is_event_handler: bool,
}

/// Export entry for binary object.
#[derive(Debug, Clone)]
pub struct ScriptExport {
    pub name: String,
    pub export_type: ExportType,
    pub function_index: Option<u32>,
}

/// Type of export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExportType {
    Function = 0,
    Type = 1,
    Const = 2,
    Props = 3,
    DataLoader = 4,
}

impl ScriptCompiler {
    /// Create a new script compiler.
    pub fn new(config: &DxConfig) -> Self {
        Self {
            config: config.clone(),
            string_pool: Vec::new(),
            string_map: HashMap::new(),
        }
    }

    /// Compile a parsed script to binary format.
    pub fn compile(&self, script: &ParsedScript) -> DxResult<Vec<u8>> {
        self.validate_syntax(script)?;

        match script.language {
            ScriptLanguage::Rust => self.compile_rust(script),
            ScriptLanguage::JavaScript | ScriptLanguage::TypeScript => {
                self.compile_javascript(script)
            }
            ScriptLanguage::Python => self.compile_python(script),
            ScriptLanguage::Go => self.compile_go(script),
        }
    }

    /// Validate script syntax before compilation.
    pub fn validate_syntax(&self, script: &ParsedScript) -> DxResult<()> {
        match script.language {
            ScriptLanguage::Rust => self.validate_rust_syntax(&script.source),
            ScriptLanguage::JavaScript | ScriptLanguage::TypeScript => {
                self.validate_js_syntax(&script.source)
            }
            ScriptLanguage::Python => self.validate_python_syntax(&script.source),
            ScriptLanguage::Go => self.validate_go_syntax(&script.source),
        }
    }

    fn compile_rust(&self, script: &ParsedScript) -> DxResult<Vec<u8>> {
        let mut output = Vec::new();
        output.extend_from_slice(SCRIPT_MAGIC);
        output.push(SCRIPT_VERSION);
        output.push(ScriptLanguage::Rust as u8);

        let mut flags: u8 = 0;
        if self.config.build.target.is_wasm() {
            flags |= 0x01;
        }
        output.push(flags);
        output.extend_from_slice(&[0u8; 2]);

        let exports = self.extract_rust_exports(&script.source);
        self.write_exports(&mut output, &exports)?;

        let source_bytes = script.source.as_bytes();
        output.extend_from_slice(&(source_bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(source_bytes);

        Ok(output)
    }

    fn extract_rust_exports(&self, source: &str) -> Vec<ScriptExport> {
        let mut exports = Vec::new();

        let fn_re = regex::Regex::new(r"pub\s+(?:async\s+)?fn\s+(\w+)").unwrap();
        for cap in fn_re.captures_iter(source) {
            let name = cap[1].to_string();
            let export_type = if name == "load" || name == "data_loader" {
                ExportType::DataLoader
            } else {
                ExportType::Function
            };
            exports.push(ScriptExport {
                name,
                export_type,
                function_index: Some(exports.len() as u32),
            });
        }

        let struct_re = regex::Regex::new(r"pub\s+struct\s+(\w+)").unwrap();
        for cap in struct_re.captures_iter(source) {
            let name = cap[1].to_string();
            let export_type = if name == "Props" {
                ExportType::Props
            } else {
                ExportType::Type
            };
            exports.push(ScriptExport {
                name,
                export_type,
                function_index: None,
            });
        }

        exports
    }

    fn validate_rust_syntax(&self, source: &str) -> DxResult<()> {
        let mut brace_count = 0i32;
        let mut paren_count = 0i32;
        let mut bracket_count = 0i32;

        for ch in source.chars() {
            match ch {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                _ => {}
            }

            if brace_count < 0 || paren_count < 0 || bracket_count < 0 {
                return Err(DxError::SyntaxError {
                    message: "Unmatched closing bracket".to_string(),
                    file: None,
                    line: None,
                    column: None,
                });
            }
        }

        if brace_count != 0 || paren_count != 0 || bracket_count != 0 {
            return Err(DxError::SyntaxError {
                message: "Unmatched opening bracket".to_string(),
                file: None,
                line: None,
                column: None,
            });
        }

        Ok(())
    }

    fn compile_javascript(&self, script: &ParsedScript) -> DxResult<Vec<u8>> {
        let mut output = Vec::new();
        output.extend_from_slice(SCRIPT_MAGIC);
        output.push(SCRIPT_VERSION);
        output.push(script.language as u8);

        let mut flags: u8 = 0;
        if script.language == ScriptLanguage::TypeScript {
            flags |= 0x02;
        }
        output.push(flags);
        output.extend_from_slice(&[0u8; 2]);

        let exports = self.extract_js_exports(&script.source);
        self.write_exports(&mut output, &exports)?;

        let minified = self.minify_javascript(&script.source);
        let source_bytes = minified.as_bytes();
        output.extend_from_slice(&(source_bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(source_bytes);

        Ok(output)
    }

    fn extract_js_exports(&self, source: &str) -> Vec<ScriptExport> {
        let mut exports = Vec::new();

        let fn_re =
            regex::Regex::new(r"export\s+(?:async\s+)?function\s+(\w+)|export\s+const\s+(\w+)\s*=")
                .unwrap();

        for cap in fn_re.captures_iter(source) {
            let name = cap
                .get(1)
                .or_else(|| cap.get(2))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            if !name.is_empty() {
                let export_type = if name == "load" || name == "getServerSideProps" {
                    ExportType::DataLoader
                } else {
                    ExportType::Function
                };
                exports.push(ScriptExport {
                    name,
                    export_type,
                    function_index: Some(exports.len() as u32),
                });
            }
        }

        if source.contains("export default") {
            exports.push(ScriptExport {
                name: "default".to_string(),
                export_type: ExportType::Function,
                function_index: Some(exports.len() as u32),
            });
        }

        exports
    }

    fn minify_javascript(&self, source: &str) -> String {
        let mut result = String::with_capacity(source.len());
        let mut in_string = false;
        let mut string_char = ' ';
        let mut in_line_comment = false;
        let mut in_block_comment = false;
        let mut last_char = ' ';
        let mut chars = source.chars().peekable();

        while let Some(ch) = chars.next() {
            if !in_string {
                if ch == '/' {
                    if let Some(&next) = chars.peek() {
                        if next == '/' {
                            in_line_comment = true;
                            chars.next();
                            continue;
                        } else if next == '*' {
                            in_block_comment = true;
                            chars.next();
                            continue;
                        }
                    }
                }
            }

            if in_line_comment {
                if ch == '\n' {
                    in_line_comment = false;
                    if !result.ends_with(' ') && !result.ends_with('\n') {
                        result.push(' ');
                    }
                }
                continue;
            }

            if in_block_comment {
                if ch == '*' {
                    if let Some(&next) = chars.peek() {
                        if next == '/' {
                            in_block_comment = false;
                            chars.next();
                        }
                    }
                }
                continue;
            }

            if (ch == '"' || ch == '\'' || ch == '`') && last_char != '\\' {
                if in_string && ch == string_char {
                    in_string = false;
                } else if !in_string {
                    in_string = true;
                    string_char = ch;
                }
            }

            if !in_string {
                if ch.is_whitespace() {
                    if !result.is_empty()
                        && !result.ends_with(' ')
                        && !result.ends_with(|c: char| "({[,;:".contains(c))
                    {
                        result.push(' ');
                    }
                    last_char = ' ';
                    continue;
                }

                if ")}];,".contains(ch) && result.ends_with(' ') {
                    result.pop();
                }
            }

            result.push(ch);
            last_char = ch;
        }

        result
    }

    fn validate_js_syntax(&self, source: &str) -> DxResult<()> {
        self.validate_rust_syntax(source)
    }

    fn compile_python(&self, script: &ParsedScript) -> DxResult<Vec<u8>> {
        let mut output = Vec::new();
        output.extend_from_slice(SCRIPT_MAGIC);
        output.push(SCRIPT_VERSION);
        output.push(ScriptLanguage::Python as u8);
        output.push(0u8);
        output.extend_from_slice(&[0u8; 2]);

        let exports = self.extract_python_exports(&script.source);
        self.write_exports(&mut output, &exports)?;

        let source_bytes = script.source.as_bytes();
        output.extend_from_slice(&(source_bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(source_bytes);

        Ok(output)
    }

    fn extract_python_exports(&self, source: &str) -> Vec<ScriptExport> {
        let mut exports = Vec::new();

        let fn_re = regex::Regex::new(r"(?:async\s+)?def\s+([a-zA-Z][a-zA-Z0-9_]*)").unwrap();

        for cap in fn_re.captures_iter(source) {
            let name = cap[1].to_string();
            if !name.starts_with('_') {
                let export_type = if name == "load" || name == "get_data" {
                    ExportType::DataLoader
                } else {
                    ExportType::Function
                };
                exports.push(ScriptExport {
                    name,
                    export_type,
                    function_index: Some(exports.len() as u32),
                });
            }
        }

        let class_re = regex::Regex::new(r"class\s+([A-Z][a-zA-Z0-9_]*)").unwrap();
        for cap in class_re.captures_iter(source) {
            let name = cap[1].to_string();
            let export_type = if name == "Props" {
                ExportType::Props
            } else {
                ExportType::Type
            };
            exports.push(ScriptExport {
                name,
                export_type,
                function_index: None,
            });
        }

        exports
    }

    fn validate_python_syntax(&self, source: &str) -> DxResult<()> {
        let mut paren_count = 0i32;
        let mut bracket_count = 0i32;
        let mut brace_count = 0i32;

        for ch in source.chars() {
            match ch {
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                _ => {}
            }
        }

        if paren_count != 0 || bracket_count != 0 || brace_count != 0 {
            return Err(DxError::SyntaxError {
                message: "Unmatched brackets".to_string(),
                file: None,
                line: None,
                column: None,
            });
        }

        Ok(())
    }

    fn compile_go(&self, script: &ParsedScript) -> DxResult<Vec<u8>> {
        let mut output = Vec::new();
        output.extend_from_slice(SCRIPT_MAGIC);
        output.push(SCRIPT_VERSION);
        output.push(ScriptLanguage::Go as u8);
        output.push(0x01);
        output.extend_from_slice(&[0u8; 2]);

        let exports = self.extract_go_exports(&script.source);
        self.write_exports(&mut output, &exports)?;

        let source_bytes = script.source.as_bytes();
        output.extend_from_slice(&(source_bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(source_bytes);

        Ok(output)
    }

    fn extract_go_exports(&self, source: &str) -> Vec<ScriptExport> {
        let mut exports = Vec::new();

        let fn_re = regex::Regex::new(r"func\s+([A-Z][a-zA-Z0-9_]*)").unwrap();

        for cap in fn_re.captures_iter(source) {
            let name = cap[1].to_string();
            let export_type = if name == "Load" || name == "GetData" {
                ExportType::DataLoader
            } else {
                ExportType::Function
            };
            exports.push(ScriptExport {
                name,
                export_type,
                function_index: Some(exports.len() as u32),
            });
        }

        let type_re = regex::Regex::new(r"type\s+([A-Z][a-zA-Z0-9_]*)\s+struct").unwrap();
        for cap in type_re.captures_iter(source) {
            let name = cap[1].to_string();
            let export_type = if name == "Props" {
                ExportType::Props
            } else {
                ExportType::Type
            };
            exports.push(ScriptExport {
                name,
                export_type,
                function_index: None,
            });
        }

        exports
    }

    fn validate_go_syntax(&self, source: &str) -> DxResult<()> {
        self.validate_rust_syntax(source)
    }

    fn write_exports(&self, output: &mut Vec<u8>, exports: &[ScriptExport]) -> DxResult<()> {
        output.extend_from_slice(&(exports.len() as u16).to_le_bytes());

        for export in exports {
            output.push(export.export_type as u8);
            let name_bytes = export.name.as_bytes();
            output.push(name_bytes.len() as u8);
            output.extend_from_slice(name_bytes);

            if let Some(idx) = export.function_index {
                output.extend_from_slice(&idx.to_le_bytes());
            } else {
                output.extend_from_slice(&0u32.to_le_bytes());
            }
        }

        Ok(())
    }

    /// Intern a string and return its index.
    pub fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.string_map.get(s) {
            return idx;
        }
        let idx = self.string_pool.len() as u32;
        self.string_pool.push(s.to_string());
        self.string_map.insert(s.to_string(), idx);
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> DxConfig {
        DxConfig::default()
    }

    #[test]
    fn test_compile_rust_script() {
        let config = make_config();
        let compiler = ScriptCompiler::new(&config);

        let script = ParsedScript {
            language: ScriptLanguage::Rust,
            source: r#"pub fn greet(name: &str) -> String { format!("Hello, {}!", name) }"#
                .to_string(),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            has_data_loader: false,
            has_props: false,
        };

        let result = compiler.compile(&script);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], SCRIPT_MAGIC);
    }

    #[test]
    fn test_validate_syntax_valid() {
        let config = make_config();
        let compiler = ScriptCompiler::new(&config);

        let script = ParsedScript {
            language: ScriptLanguage::Rust,
            source: "fn main() { let x = 1; }".to_string(),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            has_data_loader: false,
            has_props: false,
        };

        assert!(compiler.validate_syntax(&script).is_ok());
    }

    #[test]
    fn test_validate_syntax_invalid() {
        let config = make_config();
        let compiler = ScriptCompiler::new(&config);

        let script = ParsedScript {
            language: ScriptLanguage::Rust,
            source: "fn main() { let x = 1; ".to_string(),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            has_data_loader: false,
            has_props: false,
        };

        assert!(compiler.validate_syntax(&script).is_err());
    }
}
