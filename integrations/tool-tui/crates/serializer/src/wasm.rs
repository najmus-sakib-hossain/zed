//! WASM Bindings for DX Serializer VS Code Extension
//!
//! Provides the DxSerializer interface for the VS Code extension with:
//! - `to_human`: Transform LLM format to human-readable format
//! - `to_dense`: Transform human-readable format to LLM format
//! - `validate`: Validate content syntax with detailed error info
//! - `is_saveable`: Check if content is complete enough to save
//! - Security limits: max_input_size, max_recursion_depth, max_table_rows
//! - Token counting: count_tokens, count_tokens_all
//!
//! ## LLM Format (on disk)
//!
//! ```dsr
//! config[name=dx,version=0.0.1,title="Enhanced Developing Experience"]
//! workspace[paths=@/www,@/backend]
//! editors[items=neovim,zed,vscode,default=neovim]
//! ```
//!
//! ## Human Format (in editor)
//!
//! ```dx
//! name                = dx
//! version             = 0.0.1
//! title               = "Enhanced Developing Experience"
//!
//! [workspace]
//! paths:
//! - @/www
//! - @/backend
//!
//! [editors]
//! items:
//! - neovim
//! - zed
//! - vscode
//! default             = neovim
//! ```
//!
//! ## Usage from JavaScript
//!
//! ```javascript
//! import init, { DxSerializer } from 'dx_serializer';
//!
//! await init();
//!
//! const serializer = new DxSerializer();
//!
//! // Transform LLM to human (for editor display)
//! const result = serializer.toHuman('config[name=dx,version=0.0.1]');
//! if (result.success) {
//!     console.log(result.content);
//! }
//!
//! // Transform human to LLM (for disk storage)
//! const llmResult = serializer.toDense(humanContent);
//!
//! // Validate content
//! const validation = serializer.validate(content);
//! if (!validation.success) {
//!     console.log(`Error at line ${validation.line}: ${validation.error}`);
//! }
//! ```

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

use crate::error::{MAX_INPUT_SIZE, MAX_RECURSION_DEPTH, MAX_TABLE_ROWS};
use crate::llm::{human_to_llm, llm_to_human};

#[cfg(feature = "wasm")]
use crate::llm::tokens::{ModelType, TokenCounter};

/// Result of a transformation operation
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct TransformResult {
    success: bool,
    content: String,
    error: Option<String>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl TransformResult {
    /// Whether the transformation succeeded
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn success(&self) -> bool {
        self.success
    }

    /// The transformed content (empty if failed)
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn content(&self) -> String {
        self.content.clone()
    }

    /// Error message if transformation failed
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }
}

impl TransformResult {
    /// Create a successful result
    pub fn ok(content: String) -> Self {
        Self {
            success: true,
            content,
            error: None,
        }
    }

    /// Create a failed result
    pub fn err(error: String) -> Self {
        Self {
            success: false,
            content: String::new(),
            error: Some(error),
        }
    }
}

/// Result of a validation operation
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct ValidationResult {
    success: bool,
    error: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    hint: Option<String>,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl ValidationResult {
    /// Whether the content is valid
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn success(&self) -> bool {
        self.success
    }

    /// Error message if validation failed
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    /// Line number where error occurred (1-indexed)
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Column number where error occurred (1-indexed)
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn column(&self) -> Option<u32> {
        self.column
    }

    /// Actionable hint for fixing the error
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn hint(&self) -> Option<String> {
        self.hint.clone()
    }
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn valid() -> Self {
        Self {
            success: true,
            error: None,
            line: None,
            column: None,
            hint: None,
        }
    }

    /// Create a failed validation result
    pub fn invalid(error: String, line: u32, column: u32, hint: String) -> Self {
        Self {
            success: false,
            error: Some(error),
            line: Some(line),
            column: Some(column),
            hint: Some(hint),
        }
    }
}

/// Serializer configuration for the VS Code extension
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct SerializerConfig {
    /// Indentation size (2 or 4 spaces)
    indent_size: usize,
    /// Whether to preserve comments
    preserve_comments: bool,
    /// Whether to use smart quoting for special characters
    smart_quoting: bool,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl SerializerConfig {
    /// Create a new configuration with defaults
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self {
            indent_size: 2,
            preserve_comments: true,
            smart_quoting: true,
        }
    }

    /// Set the indent size (2 or 4)
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setIndentSize))]
    pub fn set_indent_size(&mut self, size: usize) {
        self.indent_size = if size == 4 { 4 } else { 2 };
    }

    /// Set whether to preserve comments
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setPreserveComments))]
    pub fn set_preserve_comments(&mut self, preserve: bool) {
        self.preserve_comments = preserve;
    }

    /// Set whether to use smart quoting
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = setSmartQuoting))]
    pub fn set_smart_quoting(&mut self, smart: bool) {
        self.smart_quoting = smart;
    }
}

impl Default for SerializerConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// DX Serializer for VS Code extension
///
/// Provides transformation between LLM (disk) and Human (editor) formats
/// with validation support. Uses the llm module for format conversion.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct DxSerializer {
    /// Configuration for serialization behavior.
    ///
    /// Reserved for future use: will enable custom formatting rules,
    /// validation strictness levels, and output style preferences.
    /// Currently stored but not actively used pending feature implementation.
    #[allow(dead_code)]
    // Reserved for future configuration features (custom formatting, validation rules)
    config: SerializerConfig,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl DxSerializer {
    /// Create a new DxSerializer with default configuration
    #[cfg_attr(feature = "wasm", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        let config = SerializerConfig::new();
        Self { config }
    }

    /// Create a DxSerializer with custom configuration
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = withConfig))]
    pub fn with_config(config: SerializerConfig) -> Self {
        Self { config }
    }

    /// Transform LLM format to human-readable format
    ///
    /// This is called when opening a .dx file in the editor.
    /// Converts sigil-based LLM format (`#c`, `#:`, `#<letter>`) to beautiful
    /// human-readable format with Unicode tables and expanded keys.
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = toHuman))]
    pub fn to_human(&self, llm_input: &str) -> TransformResult {
        // Handle empty input
        if llm_input.trim().is_empty() {
            return TransformResult::ok(String::new());
        }

        match llm_to_human(llm_input) {
            Ok(human) => TransformResult::ok(human),
            Err(e) => TransformResult::err(format!("Parse error: {}", e)),
        }
    }

    /// Transform human-readable format to LLM format
    ///
    /// This is called when saving a .dx file in the editor.
    /// Converts human-readable format back to token-optimized LLM format.
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = toDense))]
    pub fn to_dense(&self, human_input: &str) -> TransformResult {
        // Handle empty input
        if human_input.trim().is_empty() {
            return TransformResult::ok(String::new());
        }

        match human_to_llm(human_input) {
            Ok(llm) => TransformResult::ok(llm),
            Err(e) => TransformResult::err(format!("Parse error: {}", e)),
        }
    }

    /// Validate content syntax
    ///
    /// Returns detailed error information including line, column, and hints.
    #[cfg_attr(feature = "wasm", wasm_bindgen)]
    pub fn validate(&self, content: &str) -> ValidationResult {
        // Track bracket/quote state for validation
        let mut bracket_stack: Vec<(char, u32, u32)> = Vec::new();
        let mut in_string = false;
        let mut string_char = '"';
        let mut string_start: Option<(u32, u32)> = None;

        for (line_idx, line) in content.lines().enumerate() {
            let line_num = (line_idx + 1) as u32;
            let mut col = 0u32;
            let mut chars = line.chars().peekable();

            while let Some(ch) = chars.next() {
                col += 1;

                // Handle escape sequences in strings
                if in_string && ch == '\\' {
                    chars.next(); // Skip escaped character
                    col += 1;
                    continue;
                }

                // Handle string boundaries
                if !in_string && (ch == '"' || ch == '\'') {
                    in_string = true;
                    string_char = ch;
                    string_start = Some((line_num, col));
                    continue;
                }

                if in_string && ch == string_char {
                    in_string = false;
                    string_start = None;
                    continue;
                }

                // Skip bracket checking inside strings
                if in_string {
                    continue;
                }

                // Track brackets
                match ch {
                    '{' | '[' | '(' => {
                        bracket_stack.push((ch, line_num, col));
                    }
                    '}' | ']' | ')' => {
                        let expected = match ch {
                            '}' => '{',
                            ']' => '[',
                            ')' => '(',
                            _ => unreachable!(),
                        };

                        if let Some((open_char, open_line, open_col)) = bracket_stack.pop() {
                            if open_char != expected {
                                return ValidationResult::invalid(
                                    format!(
                                        "Mismatched bracket: expected '{}' but found '{}'",
                                        matching_close(open_char),
                                        ch
                                    ),
                                    line_num,
                                    col,
                                    format!(
                                        "Opening '{}' at line {}, column {} expects '{}'",
                                        open_char,
                                        open_line,
                                        open_col,
                                        matching_close(open_char)
                                    ),
                                );
                            }
                        } else {
                            return ValidationResult::invalid(
                                format!("Unexpected closing bracket '{}'", ch),
                                line_num,
                                col,
                                format!("No matching opening bracket for '{}'", ch),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check for unclosed strings
        if in_string {
            if let Some((line, col)) = string_start {
                return ValidationResult::invalid(
                    format!("Unclosed string starting with '{}'", string_char),
                    line,
                    col,
                    format!("Add a closing '{}' to complete the string", string_char),
                );
            }
        }

        // Check for unclosed brackets
        if let Some((ch, line, col)) = bracket_stack.pop() {
            return ValidationResult::invalid(
                format!("Unclosed bracket '{}'", ch),
                line,
                col,
                format!("Add a closing '{}' to match the opening '{}'", matching_close(ch), ch),
            );
        }

        ValidationResult::valid()
    }

    /// Check if content is complete enough to save
    ///
    /// Returns true if the content has no unclosed brackets or strings.
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = isSaveable))]
    pub fn is_saveable(&self, content: &str) -> bool {
        self.validate(content).success
    }

    /// Get the maximum input size limit (100 MB)
    ///
    /// Files larger than this will be rejected to prevent memory exhaustion.
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = maxInputSize))]
    pub fn max_input_size(&self) -> usize {
        MAX_INPUT_SIZE
    }

    /// Get the maximum recursion depth limit (1000 levels)
    ///
    /// Structures nested deeper than this will be rejected to prevent stack overflow.
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = maxRecursionDepth))]
    pub fn max_recursion_depth(&self) -> usize {
        MAX_RECURSION_DEPTH
    }

    /// Get the maximum table rows limit (10 million rows)
    ///
    /// Tables with more rows than this will be rejected to prevent memory exhaustion.
    #[cfg_attr(feature = "wasm", wasm_bindgen(js_name = maxTableRows))]
    pub fn max_table_rows(&self) -> usize {
        MAX_TABLE_ROWS
    }
}

impl Default for DxSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the matching closing bracket for an opening bracket
fn matching_close(open: char) -> char {
    match open {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        _ => open,
    }
}

/// Apply smart quoting to a string value
///
/// - If string contains apostrophe ('), wrap in double quotes
/// - If string contains both ' and ", use double quotes with escaped "
pub fn smart_quote(value: &str) -> String {
    let has_single = value.contains('\'');
    let has_double = value.contains('"');

    if !has_single && !has_double {
        // No quotes needed for simple strings without spaces/special chars
        if !value.contains(' ')
            && !value.contains('#')
            && !value.contains('|')
            && !value.contains('^')
            && !value.contains(':')
        {
            return value.to_string();
        }
        // Default to double quotes
        return format!("\"{}\"", value);
    }

    if has_single && !has_double {
        // Contains apostrophe - use double quotes
        return format!("\"{}\"", value);
    }

    if has_double && !has_single {
        // Contains double quotes - use single quotes
        return format!("'{}'", value);
    }

    // Contains both - use double quotes with escaped double quotes
    let escaped = value.replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

/// Initialize WASM module
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn init_wasm() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Get version information
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = "serializerVersion")]
pub fn serializer_version() -> String {
    format!(
        "dx-serializer v{} ({})",
        env!("CARGO_PKG_VERSION"),
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    )
}

// ============================================================================
// TOKEN COUNTING WASM EXPORTS
// ============================================================================

/// Result of token counting for a single model
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct TokenCountResult {
    count: usize,
    model: String,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl TokenCountResult {
    /// Get the token count
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get the model name
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn model(&self) -> String {
        self.model.clone()
    }
}

/// Result of token counting for all primary models
#[cfg_attr(feature = "wasm", wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct AllTokenCountsResult {
    gpt4o: usize,
    claude: usize,
    gemini: usize,
    other: usize,
}

#[cfg_attr(feature = "wasm", wasm_bindgen)]
impl AllTokenCountsResult {
    /// Get GPT-4o token count
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn gpt4o(&self) -> usize {
        self.gpt4o
    }

    /// Get Claude Sonnet 4 token count
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn claude(&self) -> usize {
        self.claude
    }

    /// Get Gemini 3 token count
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn gemini(&self) -> usize {
        self.gemini
    }

    /// Get Other model token count
    #[cfg_attr(feature = "wasm", wasm_bindgen(getter))]
    pub fn other(&self) -> usize {
        self.other
    }
}

/// Count tokens for a specific model
///
/// @param text - The text to tokenize
/// @param model - The model name: "gpt4o", "claude", "gemini", or "other"
/// @returns TokenCountResult with count and model name
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn count_tokens(text: &str, model: &str) -> TokenCountResult {
    let counter = TokenCounter::new();
    let model_type = match model.to_lowercase().as_str() {
        "gpt4o" | "gpt-4o" | "openai" => ModelType::Gpt4o,
        "claude" | "sonnet" | "claude-sonnet" => ModelType::ClaudeSonnet4,
        "gemini" | "gemini3" | "gemini-3" => ModelType::Gemini3,
        _ => ModelType::Other,
    };

    let info = counter.count(text, model_type);
    TokenCountResult {
        count: info.count,
        model: format!("{}", model_type),
    }
}

/// Count tokens for all primary models (GPT-4o, Claude, Gemini, Other)
///
/// @param text - The text to tokenize
/// @returns AllTokenCountsResult with counts for all 4 models
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn count_tokens_all(text: &str) -> AllTokenCountsResult {
    let counter = TokenCounter::new();
    let counts = counter.count_primary_models(text);

    AllTokenCountsResult {
        gpt4o: counts.get(&ModelType::Gpt4o).map(|i| i.count).unwrap_or(0),
        claude: counts.get(&ModelType::ClaudeSonnet4).map(|i| i.count).unwrap_or(0),
        gemini: counts.get(&ModelType::Gemini3).map(|i| i.count).unwrap_or(0),
        other: counts.get(&ModelType::Other).map(|i| i.count).unwrap_or(0),
    }
}

// ============================================================================
// FORMAT CONVERSION WASM EXPORTS
// ============================================================================

/// Convert DX format to JSON
///
/// @param dx - DX format string
/// @returns JSON string or error
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn dx_to_json(dx: &str) -> Result<String, JsValue> {
    use crate::parser::parse;

    let value =
        parse(dx.as_bytes()).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
    dx_value_to_json(&value).map_err(|e| JsValue::from_str(&e))
}

/// Convert DX format to YAML
///
/// @param dx - DX format string
/// @returns YAML string or error
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn dx_to_yaml(dx: &str) -> Result<String, JsValue> {
    use crate::parser::parse;

    let value =
        parse(dx.as_bytes()).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
    dx_value_to_yaml(&value).map_err(|e| JsValue::from_str(&e))
}

/// Convert DX format to TOML
///
/// @param dx - DX format string
/// @returns TOML string or error
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn dx_to_toml(dx: &str) -> Result<String, JsValue> {
    use crate::parser::parse;

    let value =
        parse(dx.as_bytes()).map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;
    dx_value_to_toml(&value).map_err(|e| JsValue::from_str(&e))
}

/// Convert DX format to TOON
///
/// @param dx - DX format string
/// @returns TOON string or error
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn dx_to_toon_wasm(dx: &str) -> Result<String, JsValue> {
    crate::converters::dx_to_toon(dx).map_err(|e| JsValue::from_str(&e))
}

/// Convert LLM format to Machine format (binary)
///
/// @param llm - LLM format string
/// @returns Uint8Array with binary machine format
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn llm_to_machine(llm: &str) -> Result<Vec<u8>, JsValue> {
    use crate::llm::convert;

    let machine = convert::llm_to_machine(llm)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert to machine format: {}", e)))?;

    Ok(machine.data)
}

/// Convert Human format to Machine format (binary)
///
/// @param human - Human format string
/// @returns Uint8Array with binary machine format
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn human_to_machine(human: &str) -> Result<Vec<u8>, JsValue> {
    use crate::llm::convert;

    let machine = convert::human_to_machine(human)
        .map_err(|e| JsValue::from_str(&format!("Failed to convert to machine format: {}", e)))?;

    Ok(machine.data)
}

// Helper functions for format conversion (non-WASM)
// Reserved for future public API to convert DxValue to various formats

#[cfg(any(feature = "wasm", feature = "converters", test))]
use crate::types::DxValue;

/// Convert DxValue to JSON string
#[allow(dead_code)] // Used by WASM and converter features
#[cfg(any(feature = "wasm", feature = "converters", test))]
fn dx_value_to_json(value: &DxValue) -> Result<String, String> {
    let json_value = dx_value_to_serde_json(value)?;
    serde_json::to_string_pretty(&json_value)
        .map_err(|e| format!("JSON serialization error: {}", e))
}

/// Convert DxValue to serde_json::Value
#[allow(dead_code)] // Used by dx_value_to_json
#[cfg(any(feature = "wasm", feature = "converters", test))]
fn dx_value_to_serde_json(value: &DxValue) -> Result<serde_json::Value, String> {
    match value {
        DxValue::Null => Ok(serde_json::Value::Null),
        DxValue::Bool(b) => Ok(serde_json::Value::Bool(*b)),
        DxValue::Int(i) => Ok(serde_json::Value::Number(serde_json::Number::from(*i))),
        DxValue::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .ok_or_else(|| "Invalid float value".to_string()),
        DxValue::String(s) => Ok(serde_json::Value::String(s.clone())),
        DxValue::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.values.iter().map(dx_value_to_serde_json).collect();
            Ok(serde_json::Value::Array(items?))
        }
        DxValue::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (k, v) in obj.iter() {
                map.insert(k.clone(), dx_value_to_serde_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        DxValue::Table(table) => {
            // Convert table to array of objects
            let mut rows = Vec::new();
            for row in &table.rows {
                let mut obj = serde_json::Map::new();
                for (i, col) in table.schema.columns.iter().enumerate() {
                    if let Some(val) = row.get(i) {
                        obj.insert(col.name.clone(), dx_value_to_serde_json(val)?);
                    }
                }
                rows.push(serde_json::Value::Object(obj));
            }
            Ok(serde_json::Value::Array(rows))
        }
        DxValue::Ref(id) => Ok(serde_json::Value::String(format!("@{}", id))),
    }
}

/// Convert DxValue to YAML string
#[cfg(any(feature = "wasm", test))]
fn dx_value_to_yaml(value: &DxValue) -> Result<String, String> {
    let mut output = String::new();
    dx_value_to_yaml_impl(value, &mut output, 0)?;
    Ok(output)
}

#[cfg(any(feature = "wasm", test))]
fn dx_value_to_yaml_impl(
    value: &DxValue,
    output: &mut String,
    indent: usize,
) -> Result<(), String> {
    let indent_str = "  ".repeat(indent);

    match value {
        DxValue::Null => output.push_str("null"),
        DxValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        DxValue::Int(i) => output.push_str(&i.to_string()),
        DxValue::Float(f) => output.push_str(&f.to_string()),
        DxValue::String(s) => {
            // Quote strings that need it
            if s.contains(':')
                || s.contains('#')
                || s.contains('\n')
                || s.starts_with(' ')
                || s.ends_with(' ')
            {
                output.push('"');
                output.push_str(&s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"));
                output.push('"');
            } else {
                output.push_str(s);
            }
        }
        DxValue::Array(arr) => {
            if arr.values.is_empty() {
                output.push_str("[]");
            } else {
                for (i, item) in arr.values.iter().enumerate() {
                    if i > 0 {
                        output.push('\n');
                        output.push_str(&indent_str);
                    }
                    output.push_str("- ");
                    dx_value_to_yaml_impl(item, output, indent + 1)?;
                }
            }
        }
        DxValue::Object(obj) => {
            for (i, (k, v)) in obj.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                    output.push_str(&indent_str);
                }
                output.push_str(k);
                output.push_str(": ");
                if matches!(v, DxValue::Object(_) | DxValue::Array(_)) {
                    output.push('\n');
                    output.push_str(&"  ".repeat(indent + 1));
                }
                dx_value_to_yaml_impl(v, output, indent + 1)?;
            }
        }
        DxValue::Table(table) => {
            // Convert table to array of objects in YAML
            for (i, row) in table.rows.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                    output.push_str(&indent_str);
                }
                output.push_str("- ");
                for (j, col) in table.schema.columns.iter().enumerate() {
                    if j > 0 {
                        output.push('\n');
                        output.push_str(&"  ".repeat(indent + 1));
                    }
                    output.push_str(&col.name);
                    output.push_str(": ");
                    if let Some(val) = row.get(j) {
                        dx_value_to_yaml_impl(val, output, indent + 2)?;
                    }
                }
            }
        }
        DxValue::Ref(id) => {
            output.push_str(&format!("\"@{}\"", id));
        }
    }
    Ok(())
}

/// Convert DxValue to TOML string
#[cfg(any(feature = "wasm", test))]
fn dx_value_to_toml(value: &DxValue) -> Result<String, String> {
    let mut output = String::new();

    match value {
        DxValue::Object(obj) => {
            // First pass: simple key-value pairs
            for (k, v) in obj.iter() {
                if !matches!(v, DxValue::Object(_) | DxValue::Array(_) | DxValue::Table(_)) {
                    output.push_str(k);
                    output.push_str(" = ");
                    dx_value_to_toml_value(v, &mut output)?;
                    output.push('\n');
                }
            }

            // Second pass: nested objects as sections
            for (k, v) in obj.iter() {
                if let DxValue::Object(nested) = v {
                    output.push('\n');
                    output.push('[');
                    output.push_str(k);
                    output.push_str("]\n");
                    for (nk, nv) in nested.iter() {
                        output.push_str(nk);
                        output.push_str(" = ");
                        dx_value_to_toml_value(nv, &mut output)?;
                        output.push('\n');
                    }
                }
            }

            // Third pass: arrays
            for (k, v) in obj.iter() {
                if let DxValue::Array(arr) = v {
                    output.push_str(k);
                    output.push_str(" = [");
                    for (i, item) in arr.values.iter().enumerate() {
                        if i > 0 {
                            output.push_str(", ");
                        }
                        dx_value_to_toml_value(item, &mut output)?;
                    }
                    output.push_str("]\n");
                }
            }
        }
        _ => {
            return Err("TOML root must be an object".to_string());
        }
    }

    Ok(output)
}

#[cfg(any(feature = "wasm", test))]
fn dx_value_to_toml_value(value: &DxValue, output: &mut String) -> Result<(), String> {
    match value {
        DxValue::Null => output.push_str("\"\""),
        DxValue::Bool(b) => output.push_str(if *b { "true" } else { "false" }),
        DxValue::Int(i) => output.push_str(&i.to_string()),
        DxValue::Float(f) => output.push_str(&f.to_string()),
        DxValue::String(s) => {
            output.push('"');
            output.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
            output.push('"');
        }
        DxValue::Array(arr) => {
            output.push('[');
            for (i, item) in arr.values.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                dx_value_to_toml_value(item, output)?;
            }
            output.push(']');
        }
        DxValue::Object(_) => output.push_str("{}"),
        DxValue::Table(_) => output.push_str("[[]]"),
        DxValue::Ref(id) => {
            output.push('"');
            output.push('@');
            output.push_str(&id.to_string());
            output.push('"');
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_result() {
        let ok = TransformResult::ok("content".to_string());
        assert!(ok.success());
        assert_eq!(ok.content(), "content");
        assert!(ok.error().is_none());

        let err = TransformResult::err("error".to_string());
        assert!(!err.success());
        assert!(err.content().is_empty());
        assert_eq!(err.error(), Some("error".to_string()));
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::valid();
        assert!(valid.success());
        assert!(valid.error().is_none());

        let invalid = ValidationResult::invalid("error".to_string(), 1, 5, "hint".to_string());
        assert!(!invalid.success());
        assert_eq!(invalid.error(), Some("error".to_string()));
        assert_eq!(invalid.line(), Some(1));
        assert_eq!(invalid.column(), Some(5));
        assert_eq!(invalid.hint(), Some("hint".to_string()));
    }

    #[test]
    fn test_serializer_to_human() {
        let serializer = DxSerializer::new();
        // Use LLM format: root-level key|value pairs
        let result = serializer.to_human("host|localhost\nport|5432");
        assert!(result.success(), "to_human failed: {:?}", result.error());
        assert!(result.content().contains("host") || result.content().contains("localhost"));
    }

    #[test]
    fn test_serializer_to_dense() {
        let serializer = DxSerializer::new();
        // Use LLM format for input
        let human = serializer.to_human("debug|+\nprod|-");
        assert!(human.success(), "to_human failed: {:?}", human.error());

        // Then deflate back
        let dense = serializer.to_dense(&human.content());
        assert!(dense.success(), "to_dense failed: {:?}", dense.error());
        // The result should contain the LLM format (key|value pairs)
        assert!(dense.content().contains("|") || dense.content().contains("debug"));
    }

    #[test]
    fn test_validate_valid_content() {
        let serializer = DxSerializer::new();
        let result = serializer.validate("key: value\nother: data");
        assert!(result.success());
    }

    #[test]
    fn test_validate_unclosed_bracket() {
        let serializer = DxSerializer::new();
        let result = serializer.validate("data: {\n  key: value");
        assert!(!result.success());
        assert!(result.error().unwrap().contains("Unclosed bracket"));
        assert_eq!(result.line(), Some(1));
        assert!(result.hint().is_some());
    }

    #[test]
    fn test_validate_unclosed_string() {
        let serializer = DxSerializer::new();
        let result = serializer.validate("key: \"unclosed string");
        assert!(!result.success());
        assert!(result.error().unwrap().contains("Unclosed string"));
        assert!(result.hint().is_some());
    }

    #[test]
    fn test_validate_mismatched_brackets() {
        let serializer = DxSerializer::new();
        let result = serializer.validate("data: [value}");
        assert!(!result.success());
        assert!(result.error().unwrap().contains("Mismatched bracket"));
    }

    #[test]
    fn test_is_saveable() {
        let serializer = DxSerializer::new();
        assert!(serializer.is_saveable("key: value"));
        assert!(!serializer.is_saveable("key: {unclosed"));
        assert!(!serializer.is_saveable("key: \"unclosed"));
    }

    #[test]
    fn test_smart_quote_simple() {
        assert_eq!(smart_quote("hello"), "hello");
        assert_eq!(smart_quote("hello world"), "\"hello world\"");
    }

    #[test]
    fn test_smart_quote_apostrophe() {
        // Strings with apostrophes should use double quotes
        assert_eq!(smart_quote("don't"), "\"don't\"");
        assert_eq!(smart_quote("it's working"), "\"it's working\"");
    }

    #[test]
    fn test_smart_quote_double_quotes() {
        // Strings with double quotes should use single quotes
        assert_eq!(smart_quote("say \"hello\""), "'say \"hello\"'");
    }

    #[test]
    fn test_smart_quote_both() {
        // Strings with both should escape double quotes
        assert_eq!(smart_quote("don't say \"hello\""), "\"don't say \\\"hello\\\"\"");
    }

    #[test]
    fn test_smart_quote_special_chars() {
        assert_eq!(smart_quote("key:value"), "\"key:value\"");
        assert_eq!(smart_quote("a|b|c"), "\"a|b|c\"");
        assert_eq!(smart_quote("a#b"), "\"a#b\"");
    }

    #[test]
    fn test_config() {
        let mut config = SerializerConfig::new();
        assert_eq!(config.indent_size, 2);

        config.set_indent_size(4);
        assert_eq!(config.indent_size, 4);

        config.set_indent_size(3); // Invalid, should default to 2
        assert_eq!(config.indent_size, 2);
    }

    #[test]
    fn test_empty_input() {
        let serializer = DxSerializer::new();

        let human = serializer.to_human("");
        assert!(human.success());
        assert!(human.content().is_empty());

        let dense = serializer.to_dense("");
        assert!(dense.success());
        assert!(dense.content().is_empty());
    }

    // Token counting tests
    #[test]
    fn test_dx_to_json() {
        let dx = "name:test\nversion:100";
        let result = dx_value_to_json(&crate::parser::parse(dx.as_bytes()).unwrap());
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("test"));
        assert!(json.contains("version"));
        assert!(json.contains("100"));
    }

    #[test]
    fn test_dx_to_yaml() {
        let dx = "name:test\nversion:100";
        let result = dx_value_to_yaml(&crate::parser::parse(dx.as_bytes()).unwrap());
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("name"));
        assert!(yaml.contains("test"));
    }

    #[test]
    fn test_dx_to_toml() {
        let dx = "name:test\nversion:100";
        let result = dx_value_to_toml(&crate::parser::parse(dx.as_bytes()).unwrap());
        assert!(result.is_ok());
        let toml = result.unwrap();
        assert!(toml.contains("name"));
        assert!(toml.contains("test"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Generators for valid LLM format content

    /// Generate a valid abbreviated key (2-3 lowercase letters)
    fn valid_abbrev_key() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z]{2,3}")
            .unwrap()
            .prop_filter("non-empty key", |s| !s.is_empty())
    }

    /// Generate a simple value for LLM format
    fn simple_value() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple strings (alphanumeric)
            prop::string::string_regex("[a-zA-Z][a-zA-Z0-9]{0,15}").unwrap(),
            // Numbers
            (1i32..10000).prop_map(|n| n.to_string()),
        ]
    }

    /// Generate a boolean value in LLM format (+ or -)
    /// Reserved for future property tests covering boolean round-trips
    #[allow(dead_code)] // Test utility for future property tests
    fn _llm_bool() -> impl Strategy<Value = String> {
        prop::bool::ANY.prop_map(|b| if b { "+".to_string() } else { "-".to_string() })
    }

    /// Generate a context section in LLM format: key|val\nkey|val
    /// Reserved for future property tests covering context section round-trips
    #[allow(dead_code)] // Test utility for future property tests
    fn _llm_context_section() -> impl Strategy<Value = String> {
        prop::collection::vec((valid_abbrev_key(), simple_value()), 1..4).prop_map(|pairs| {
            pairs
                .into_iter()
                .map(|(k, v)| format!("{}|{}", k, v))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    /// Generate a data section in LLM format: #d(schema)\nrow\nrow
    /// Reserved for future property tests covering data section round-trips
    #[allow(dead_code)] // Test utility for future property tests
    fn _llm_data_section() -> impl Strategy<Value = String> {
        (
            prop::string::string_regex("[a-z]").unwrap(), // section id (single letter)
            prop::collection::vec(valid_abbrev_key(), 2..4), // schema columns
            prop::collection::vec(simple_value(), 2..4),  // row values
        )
            .prop_filter("schema and row same length", |(_, schema, row)| schema.len() == row.len())
            .prop_map(|(id, schema, row)| {
                let schema_str = schema.join("|");
                let row_str = row.join("|");
                format!("#{}({})\n{}", id, schema_str, row_str)
            })
    }

    /// Generate valid LLM format content
    /// Reserved for future property tests covering full document round-trips
    #[allow(dead_code)] // Test utility for future property tests
    fn _valid_llm_content() -> impl Strategy<Value = String> {
        prop_oneof![_llm_context_section(), _llm_data_section(),]
    }

    // Feature: dx-serializer-extension-fix, Property 1: LLM to Human to LLM Round-Trip
    // For any valid LLM format string, converting to human format and back to LLM format
    // SHALL produce a document with equivalent data.
    // **Validates: Requirements 1.1-1.9, 2.1-2.6, 3.1-3.5, 3.6**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_llm_round_trip_context(
            pairs in prop::collection::vec(
                (valid_abbrev_key(), simple_value()),
                1..3,
            )
        ) {
            // Skip if keys are duplicated
            let keys: Vec<_> = pairs.iter().map(|(k, _)| k.clone()).collect();
            let unique_keys: std::collections::HashSet<_> = keys.iter().collect();
            prop_assume!(keys.len() == unique_keys.len());

            let serializer = DxSerializer::new();
            // Use new format: root-level key|value pairs (one per line)
            let llm: String = pairs
                .iter()
                .map(|(k, v)| format!("{}|{}", k, v))
                .collect::<Vec<_>>()
                .join("\n");

            // Transform to human
            let human_result = serializer.to_human(&llm);
            prop_assert!(human_result.success(), "to_human failed: {:?}", human_result.error());

            // Transform back to LLM
            let llm_result = serializer.to_dense(&human_result.content());
            prop_assert!(llm_result.success(), "to_dense failed: {:?}", llm_result.error());

            // Verify values are preserved
            let result = llm_result.content();
            for (_, value) in &pairs {
                prop_assert!(
                    result.contains(value),
                    "Value '{}' not found in result: '{}'", value, result
                );
            }
        }

        #[test]
        fn prop_llm_round_trip_booleans(
            key1 in valid_abbrev_key(),
            key2 in valid_abbrev_key(),
            bool1 in prop::bool::ANY,
            bool2 in prop::bool::ANY
        ) {
            prop_assume!(key1 != key2);

            let serializer = DxSerializer::new();
            let b1 = if bool1 { "true" } else { "false" };
            let b2 = if bool2 { "true" } else { "false" };
            // Use new format: root-level key=value pairs
            let llm = format!("{}={}\n{}={}", key1, b1, key2, b2);

            // Transform to human
            let human_result = serializer.to_human(&llm);
            prop_assert!(human_result.success(), "to_human failed: {:?}", human_result.error());

            // Human format should show true/false
            let human = human_result.content();
            if bool1 {
                prop_assert!(human.contains("true"),
                    "Boolean true not found in human format: '{}'", human);
            } else {
                prop_assert!(human.contains("false"),
                    "Boolean false not found in human format: '{}'", human);
            }

            // Transform back to LLM
            let llm_result = serializer.to_dense(&human);
            prop_assert!(llm_result.success(), "to_dense failed: {:?}", llm_result.error());

            // LLM format should have true or false
            let result = llm_result.content();
            prop_assert!(
                result.contains("true") || result.contains("false"),
                "Boolean values not found in LLM result: '{}'", result
            );
        }

        #[test]
        fn prop_empty_content_round_trip(content in "\\s*") {
            let serializer = DxSerializer::new();

            let human_result = serializer.to_human(&content);
            prop_assert!(human_result.success());

            let dense_result = serializer.to_dense(&human_result.content());
            prop_assert!(dense_result.success());
        }
    }
}

#[cfg(test)]
mod string_preservation_tests {
    use super::*;
    use proptest::prelude::*;

    // Feature: dx-serializer-extension, Property 3: String value preservation
    // For any string value (including URLs with query parameters, strings with
    // apostrophes, strings with both quote types, and strings with escape sequences),
    // transforming through the serializer SHALL preserve the exact string content.
    // **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

    /// Generate URL-like strings with query parameters
    fn url_string() -> impl Strategy<Value = String> {
        (
            prop::string::string_regex("https?://[a-z]+\\.[a-z]{2,4}").unwrap(),
            prop::string::string_regex("/[a-z]+").unwrap(),
            prop::collection::vec(
                (
                    prop::string::string_regex("[a-z]+").unwrap(),
                    prop::string::string_regex("[a-zA-Z0-9]+").unwrap(),
                ),
                0..3,
            ),
        )
            .prop_map(|(base, path, params)| {
                if params.is_empty() {
                    format!("{}{}", base, path)
                } else {
                    let query: String = params
                        .into_iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<_>>()
                        .join("&");
                    format!("{}{}?{}", base, path, query)
                }
            })
    }

    /// Generate strings with apostrophes
    fn apostrophe_string() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("don't".to_string()),
            Just("it's".to_string()),
            Just("won't".to_string()),
            Just("can't".to_string()),
            Just("I'm".to_string()),
            prop::string::string_regex("[A-Z][a-z]+'s [a-z]+").unwrap(),
        ]
    }

    /// Generate strings with double quotes
    fn double_quote_string() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("say \"hello\"".to_string()),
            Just("the \"best\" way".to_string()),
            prop::string::string_regex("[a-z]+ \"[a-z]+\" [a-z]+").unwrap(),
        ]
    }

    /// Generate strings with both quote types
    fn mixed_quote_string() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("don't say \"hello\"".to_string()),
            Just("it's \"great\"".to_string()),
            Just("can't \"stop\"".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_url_preservation(url in url_string()) {
            // Test that URLs are preserved through smart_quote
            let quoted = smart_quote(&url);

            // Extract the content (remove quotes if present)
            let extracted = if (quoted.starts_with('"') && quoted.ends_with('"'))
                || (quoted.starts_with('\'') && quoted.ends_with('\''))
            {
                quoted[1..quoted.len()-1].to_string()
            } else {
                quoted.clone()
            };

            prop_assert_eq!(
                url.clone(), extracted.clone(),
                "URL not preserved: original='{}', quoted='{}', extracted='{}'",
                url, quoted, extracted
            );
        }

        #[test]
        fn prop_apostrophe_uses_double_quotes(s in apostrophe_string()) {
            let quoted = smart_quote(&s);

            // Strings with apostrophes should use double quotes
            prop_assert!(
                quoted.starts_with('"') && quoted.ends_with('"'),
                "String with apostrophe should use double quotes: '{}' -> '{}'",
                s, quoted
            );

            // Content should be preserved
            let extracted = &quoted[1..quoted.len()-1];
            prop_assert_eq!(
                s.clone(), extracted.to_string(),
                "Apostrophe string not preserved: original='{}', extracted='{}'",
                s, extracted
            );
        }

        #[test]
        fn prop_double_quote_uses_single_quotes(s in double_quote_string()) {
            let quoted = smart_quote(&s);

            // Strings with double quotes should use single quotes
            prop_assert!(
                quoted.starts_with('\'') && quoted.ends_with('\''),
                "String with double quotes should use single quotes: '{}' -> '{}'",
                s, quoted
            );

            // Content should be preserved
            let extracted = &quoted[1..quoted.len()-1];
            prop_assert_eq!(
                s.clone(), extracted.to_string(),
                "Double quote string not preserved: original='{}', extracted='{}'",
                s, extracted
            );
        }

        #[test]
        fn prop_mixed_quotes_escapes_double(s in mixed_quote_string()) {
            let quoted = smart_quote(&s);

            // Should use double quotes with escaped internal double quotes
            prop_assert!(
                quoted.starts_with('"') && quoted.ends_with('"'),
                "Mixed quote string should use double quotes: '{}' -> '{}'",
                s, quoted
            );

            // Content should be preserved (after unescaping)
            let extracted = quoted[1..quoted.len()-1].replace("\\\"", "\"");
            prop_assert_eq!(
                s.clone(), extracted.clone(),
                "Mixed quote string not preserved: original='{}', extracted='{}'",
                s, extracted
            );
        }

        #[test]
        fn prop_simple_string_no_quotes(
            s in prop::string::string_regex("[a-zA-Z][a-zA-Z0-9]{0,15}").unwrap()
        ) {
            let quoted = smart_quote(&s);

            // Simple strings without special chars should not be quoted
            prop_assert_eq!(
                s.clone(), quoted.clone(),
                "Simple string should not be quoted: '{}' -> '{}'",
                s, quoted
            );
        }

        #[test]
        fn prop_string_with_spaces_quoted(
            word1 in prop::string::string_regex("[a-z]+").unwrap(),
            word2 in prop::string::string_regex("[a-z]+").unwrap()
        ) {
            let s = format!("{} {}", word1, word2);
            let quoted = smart_quote(&s);

            // Strings with spaces should be quoted
            prop_assert!(
                (quoted.starts_with('"') && quoted.ends_with('"')) ||
                (quoted.starts_with('\'') && quoted.ends_with('\'')),
                "String with spaces should be quoted: '{}' -> '{}'",
                s, quoted
            );
        }

        #[test]
        fn prop_special_chars_quoted(
            prefix in prop::string::string_regex("[a-z]+").unwrap(),
            suffix in prop::string::string_regex("[a-z]+").unwrap(),
            special in prop::sample::select(vec!['#', '|', '^', ':'])
        ) {
            let s = format!("{}{}{}", prefix, special, suffix);
            let quoted = smart_quote(&s);

            // Strings with special DX chars should be quoted
            prop_assert!(
                (quoted.starts_with('"') && quoted.ends_with('"')) ||
                (quoted.starts_with('\'') && quoted.ends_with('\'')),
                "String with special char '{}' should be quoted: '{}' -> '{}'",
                special, s, quoted
            );
        }
    }
}
