//! DX Serializer LLM Format Parser
//!
//! Parses the token-optimized LLM format into DxDocument.
//! 52-73% more token-efficient than JSON.
//!
//! ## LLM Format Syntax (Wrapped Dataframe)
//!
//! ```text
//! # Key-Value Pairs
//! name=MyApp
//! port=8080
//! description="Multi word string"
//!
//! # Arrays (square brackets)
//! tags=[rust performance serialization]
//! editors=[neovim zed "firebase studio"]
//!
//! # Objects (parentheses)
//! config(host=localhost port=5432 debug=true)
//! server(url="https://api.example.com" timeout=30)
//!
//! # Tables (wrapped dataframes - deterministic parsing)
//! users[id name email](
//! 1 Alice alice@ex.com
//! 2 Bob bob@ex.com
//! 3 Carol carol@ex.com
//! )
//!
//! # Multi-word values use quotes
//! employees[id name dept](
//! 1 "James Smith" Engineering
//! 2 "Mary Johnson" "Research and Development"
//! )
//! ```
//!
//! ## Removed Features (for TypeScript compatibility)
//!
//! The following features were removed to maintain compatibility with the TypeScript implementation:
//!
//! - **Boolean shorthand**: `+` for true, `-` for false (use `true`/`false` instead)
//! - **Null shorthand**: `~` for null (use `null` instead)

use crate::llm::types::{DxDocument, DxLlmValue, DxSection};
use indexmap::IndexMap;
use thiserror::Error;

/// Parse errors for Dx Serializer format
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected character '{ch}' at position {pos}")]
    UnexpectedChar { ch: char, pos: usize },

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Invalid value format: {value}")]
    InvalidValue { value: String },

    #[error("Schema mismatch: expected {expected} columns, got {got}")]
    SchemaMismatch { expected: usize, got: usize },

    #[error("Invalid UTF-8 at byte offset {offset}")]
    Utf8Error { offset: usize },

    #[error("Input too large: {size} bytes exceeds maximum of {max} bytes")]
    InputTooLarge { size: usize, max: usize },

    #[error("Unclosed bracket at position {pos}")]
    UnclosedBracket { pos: usize },

    #[error("Unclosed parenthesis at position {pos}")]
    UnclosedParen { pos: usize },

    #[error("Missing value after '=' at position {pos}")]
    MissingValue { pos: usize },

    #[error("Invalid table format: {msg}")]
    InvalidTable { msg: String },
}

/// Dx Serializer format parser
pub struct LlmParser;

impl LlmParser {
    /// Parse Dx Serializer format string into DxDocument
    pub fn parse(input: &str) -> Result<DxDocument, ParseError> {
        if input.len() > crate::error::MAX_INPUT_SIZE {
            return Err(ParseError::InputTooLarge {
                size: input.len(),
                max: crate::error::MAX_INPUT_SIZE,
            });
        }

        let mut doc = DxDocument::new();
        let mut parser = DsrParser::new(input);
        parser.parse_document(&mut doc)?;

        Ok(doc)
    }

    /// Parse Dx Serializer format from bytes with UTF-8 validation
    pub fn parse_bytes(input: &[u8]) -> Result<DxDocument, ParseError> {
        if input.len() > crate::error::MAX_INPUT_SIZE {
            return Err(ParseError::InputTooLarge {
                size: input.len(),
                max: crate::error::MAX_INPUT_SIZE,
            });
        }

        let input_str = std::str::from_utf8(input).map_err(|e| ParseError::Utf8Error {
            offset: e.valid_up_to(),
        })?;
        Self::parse(input_str)
    }

    /// Parse a single value string preserving natural spaces
    pub fn parse_value(s: &str) -> DxLlmValue {
        let s = s.trim();

        // Boolean values: true/false only (removed +/- shorthand)
        if s == "true" {
            return DxLlmValue::Bool(true);
        }
        if s == "false" {
            return DxLlmValue::Bool(false);
        }
        if s == "null" {
            return DxLlmValue::Null;
        }

        // Try to parse as number
        if let Ok(n) = s.parse::<i64>() {
            return DxLlmValue::Num(n as f64);
        }
        if let Ok(n) = s.parse::<f64>() {
            return DxLlmValue::Num(n);
        }

        // Handle quoted strings - strip quotes
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            let unquoted = &s[1..s.len() - 1];
            // Unescape any escaped quotes
            let unescaped = unquoted.replace("\\\"", "\"");
            return DxLlmValue::Str(unescaped);
        }

        DxLlmValue::Str(s.to_string())
    }

    /// Resolve references in a document
    ///
    /// Replaces `DxLlmValue::Ref` values with their resolved values from `doc.refs`.
    pub fn resolve_refs(doc: &DxDocument) -> Result<DxDocument, ParseError> {
        let mut resolved = doc.clone();

        // Resolve context values
        for value in resolved.context.values_mut() {
            Self::resolve_value(value, &doc.refs)?;
        }

        // Resolve section values
        for section in resolved.sections.values_mut() {
            for row in &mut section.rows {
                for value in row {
                    Self::resolve_value(value, &doc.refs)?;
                }
            }
        }

        Ok(resolved)
    }

    /// Resolve a single value recursively
    fn resolve_value(
        value: &mut DxLlmValue,
        refs: &IndexMap<String, String>,
    ) -> Result<(), ParseError> {
        match value {
            DxLlmValue::Ref(key) => {
                if let Some(resolved) = refs.get(key) {
                    *value = DxLlmValue::Str(resolved.clone());
                }
            }
            DxLlmValue::Arr(items) => {
                for item in items {
                    Self::resolve_value(item, refs)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Prefix and suffix information for prefix elimination
#[derive(Debug, Clone, Default)]
struct PrefixInfo {
    prefixes: Vec<String>,
    suffixes: Vec<String>,
}

/// Internal parser state
struct DsrParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> DsrParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_document(&mut self, doc: &mut DxDocument) -> Result<(), ParseError> {
        self.skip_whitespace();

        while self.pos < self.input.len() {
            self.parse_statement(doc)?;
            self.skip_whitespace();
        }

        Ok(())
    }

    fn parse_statement(&mut self, doc: &mut DxDocument) -> Result<(), ParseError> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Ok(());
        }

        // Parse identifier (may contain dots for nested paths)
        let name = self.parse_identifier()?;
        if name.is_empty() {
            // If we couldn't parse an identifier, return an error
            // to ensure we don't return partial results on invalid input
            let ch = self.current_char();
            return Err(ParseError::UnexpectedChar { ch, pos: self.pos });
        }

        self.skip_whitespace();
        if self.pos >= self.input.len() {
            // Just a name with no value - treat as empty string
            doc.context.insert(name.clone(), DxLlmValue::Str(String::new()));
            doc.entry_order.push(crate::llm::types::EntryRef::Context(name));
            return Ok(());
        }

        let ch = self.peek_char();

        match ch {
            Some('[') => {
                // NEW FORMAT: Check if it's wrapped dataframe table: name[headers](rows)
                // or array: name=[items]
                // or OLD FORMAT: name[count] or name[key=value]

                // Peek ahead to see what follows the bracket
                let start_pos = self.pos;
                self.advance(); // consume '['

                // Read content inside brackets
                let bracket_content = self.parse_until_char(']')?;
                self.expect_char(']')?;

                self.skip_whitespace();
                let next_ch = self.peek_char();

                if next_ch == Some('(') {
                    // Wrapped dataframe table: name[headers](rows)
                    let schema: Vec<String> =
                        bracket_content.split_whitespace().map(|s| s.to_string()).collect();

                    if schema.is_empty() {
                        return Err(ParseError::InvalidTable {
                            msg: "Empty schema in wrapped dataframe".to_string(),
                        });
                    }

                    // Parse wrapped rows
                    self.advance(); // consume '('
                    let section = self.parse_wrapped_dataframe_rows(&schema)?;
                    self.expect_char(')')?;

                    // Use first character of name as section ID
                    let section_id = name.chars().next().unwrap_or('a');
                    doc.sections.insert(section_id, section);
                    doc.section_names.insert(section_id, name.clone());
                } else {
                    // Restore position and try old format parsing
                    self.pos = start_pos;

                    // Check if it's array syntax: name[n]: or name[n]=
                    if let Some(_count) = self.try_parse_array_count() {
                        // Array: name[n]: items or name[n]= items
                        self.skip_whitespace();
                        let delimiter = self.peek_char();

                        if delimiter == Some(':') {
                            self.advance();
                            // Check for leaf inlining (::)
                            if self.peek_char() == Some(':') {
                                self.advance(); // consume second ':'
                            }
                            self.skip_whitespace();
                            let items_str = self.parse_until_delimiter(&['\n', '\r'])?;
                            let items: Vec<DxLlmValue> = items_str
                                .split(',')
                                .map(|s| LlmParser::parse_value(s.trim()))
                                .collect();
                            doc.context.insert(name.clone(), DxLlmValue::Arr(items));
                        } else if delimiter == Some('=') {
                            self.advance();
                            let items_str = self.parse_until_delimiter(&['\n', '\r'])?;
                            let items: Vec<DxLlmValue> = items_str
                                .split(',')
                                .map(|s| LlmParser::parse_value(s.trim()))
                                .collect();
                            doc.context.insert(name.clone(), DxLlmValue::Arr(items));
                        }
                    } else {
                        // Object: name[key=value,key2=value2]
                        let obj = self.parse_inline_object(&name)?;
                        doc.context.insert(name.clone(), obj);
                    }
                }
            }
            Some('(') => {
                // NEW FORMAT: Inline object: name(key=val key2=val2)
                self.advance(); // consume '('
                let obj = self.parse_parenthesized_object()?;
                self.expect_char(')')?;
                doc.context.insert(name.clone(), obj);
            }
            Some('=') => {
                // NEW FORMAT: Simple key=value or array: name=[item1 item2]
                self.advance(); // consume '='

                // Check if it's an array: =[...]
                if self.peek_char() == Some('[') {
                    self.advance(); // consume '['
                    let items_str = self.parse_until_char(']')?;
                    self.expect_char(']')?;

                    // Parse space-separated items (with quote support)
                    let items = self.parse_quoted_items(&items_str);
                    doc.context.insert(name.clone(), DxLlmValue::Arr(items));
                } else {
                    // Simple value
                    let value = self.parse_value_until_delimiter(&[',', '\n', '\r', ']'])?;
                    doc.context.insert(name.clone(), value);
                }
            }
            Some(':') => {
                self.advance();

                // Check for leaf inlining (::)
                let is_leaf = self.peek_char() == Some(':');
                if is_leaf {
                    self.advance(); // consume second ':'
                }

                self.skip_whitespace();

                // Check what follows
                let next = self.peek_char();

                if !is_leaf && next == Some('(') {
                    // Table without count: name:(schema)[data]
                    let section = self.parse_table("")?;
                    // Use first character of name as section ID
                    let section_id = name.chars().next().unwrap_or('a');
                    doc.sections.insert(section_id, section);
                    doc.section_names.insert(section_id, name.clone());
                } else if !is_leaf && next.map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    // Could be table with count: name:100(schema)[data]
                    // Or inline object with count: name:3[key=value key2=value2]
                    // Or simple array: name:3=item1 item2 item3
                    // Or could be a version number like 2.2.16
                    // Peek ahead to see what follows the digits
                    let start_pos = self.pos;
                    let mut count_str = String::new();
                    while self.pos < self.input.len() {
                        let ch = self.current_char();
                        if ch.is_ascii_digit() {
                            count_str.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }

                    self.skip_whitespace();

                    if self.peek_char() == Some('(') {
                        // It's a table: name:count(schema)[data]
                        let section = self.parse_table(&count_str)?;
                        // Use first character of name as section ID
                        let section_id = name.chars().next().unwrap_or('a');
                        doc.sections.insert(section_id, section);
                        doc.section_names.insert(section_id, name.clone());
                    } else if self.peek_char() == Some('@') {
                        // Check for compact syntax: name:count@=[key value key value]
                        let next_pos = self.pos + 1;
                        if next_pos < self.input.len() && self.input[next_pos..].starts_with('=') {
                            // It's compact syntax
                            let count = count_str.parse::<usize>().unwrap_or(0);
                            let obj = self.parse_compact_object(&name, count)?;
                            doc.context.insert(name.clone(), obj);
                        } else {
                            // Not compact syntax, restore position and parse as regular value
                            self.pos = start_pos;
                            let value_str = self.parse_until_delimiter(&['\n', '\r'])?;
                            doc.context.insert(name.clone(), LlmParser::parse_value(&value_str));
                        }
                    } else if self.peek_char() == Some('[') {
                        // It's an inline object with count: name:count[key=value key2=value2]
                        let obj = self.parse_inline_object(&name)?;
                        doc.context.insert(name.clone(), obj);
                    } else if self.peek_char() == Some('=') {
                        // Simple array: name:count=item1 item2 item3 (space-separated)
                        // or name:count=item1,item2,item3 (comma-separated, legacy)
                        self.advance();
                        let items_str = self.parse_until_delimiter(&['\n', '\r'])?;

                        // Auto-detect separator: comma (legacy) or space (new format)
                        let items: Vec<DxLlmValue> = if items_str.contains(',') {
                            // Comma-separated (legacy format)
                            items_str.split(',').map(|s| LlmParser::parse_value(s.trim())).collect()
                        } else {
                            // Space-separated (new format)
                            items_str.split_whitespace().map(LlmParser::parse_value).collect()
                        };
                        doc.context.insert(name.clone(), DxLlmValue::Arr(items));
                    } else {
                        // Not a table or inline object - restore position and parse as regular value
                        self.pos = start_pos;
                        let value_str = self.parse_until_delimiter(&['\n', '\r'])?;
                        doc.context.insert(name.clone(), LlmParser::parse_value(&value_str));
                    }
                } else {
                    // Standard or leaf value: name: value or name:: value
                    let value_str = self.parse_until_delimiter(&['\n', '\r'])?;
                    doc.context.insert(name.clone(), LlmParser::parse_value(&value_str));
                }
            }
            Some('|') => {
                // Pipe format: key|value (used in LLM format)
                self.advance();
                let value = self.parse_value_until_delimiter(&['\n', '\r'])?;
                doc.context.insert(name.clone(), value);
            }
            _ => {
                // No delimiter - might be end of line or next statement
                doc.context.insert(name.clone(), DxLlmValue::Str(String::new()));
            }
        }

        // Track entry order: check if this name was added as context or section
        // We need to check both because the name could have been added to either
        if doc.context.contains_key(&name) {
            // Only add if not already in entry_order (avoid duplicates)
            let entry_ref = crate::llm::types::EntryRef::Context(name.clone());
            if !doc.entry_order.contains(&entry_ref) {
                doc.entry_order.push(entry_ref);
            }
        } else {
            // Check if it was added as a section
            let section_id = name.chars().next().unwrap_or('a');
            if doc.sections.contains_key(&section_id)
                && doc.section_names.get(&section_id) == Some(&name)
            {
                let entry_ref = crate::llm::types::EntryRef::Section(section_id);
                if !doc.entry_order.contains(&entry_ref) {
                    doc.entry_order.push(entry_ref);
                }
            }
        }

        Ok(())
    }

    /// Try to parse array count syntax: [n] where n is a number
    /// Returns Some(count) if successful, None otherwise (restores position)
    fn try_parse_array_count(&mut self) -> Option<usize> {
        let start_pos = self.pos;

        if self.peek_char() != Some('[') {
            return None;
        }
        self.advance();

        let mut num_str = String::new();
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                num_str.push(ch);
                self.advance();
            } else if ch == ']' {
                self.advance();
                if let Ok(count) = num_str.parse::<usize>() {
                    return Some(count);
                }
                break;
            } else {
                break;
            }
        }

        // Not array count syntax, restore position
        self.pos = start_pos;
        None
    }

    fn parse_inline_object(&mut self, _name: &str) -> Result<DxLlmValue, ParseError> {
        let start_pos = self.pos;
        self.expect_char('[')?;

        // Detect separator: scan ahead to see if we have commas or spaces between fields
        let separator = self.detect_object_separator()?;

        let mut fields: IndexMap<String, DxLlmValue> = IndexMap::new();
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 100_000; // Safety limit

        loop {
            iteration_count += 1;
            if iteration_count > MAX_ITERATIONS {
                return Err(ParseError::InvalidTable {
                    msg: "Object parsing exceeded maximum iterations".to_string(),
                });
            }

            self.skip_whitespace();

            if self.peek_char() == Some(']') {
                self.advance();
                break;
            }

            if self.pos >= self.input.len() {
                return Err(ParseError::UnclosedBracket { pos: start_pos });
            }

            // Parse key=value
            let key = self.parse_identifier()?;
            if key.is_empty() {
                if self.peek_char() == Some(']') {
                    self.advance();
                    break;
                }
                // Return error on unexpected character instead of silently skipping
                let ch = self.current_char();
                return Err(ParseError::UnexpectedChar { ch, pos: self.pos });
            }

            self.skip_whitespace();

            if self.peek_char() == Some('=') {
                self.advance();
                // Parse value until we hit the separator or closing bracket
                let delimiters = if separator == ' ' {
                    vec![' ', ']', '\n']
                } else {
                    vec![',', ']', '\n']
                };
                let value = self.parse_value_until_delimiter(&delimiters)?;
                fields.insert(key, value);
            } else if self.peek_char() == Some('[') {
                // Check if it's a nested array: key[count]= or nested object: key[...]
                if let Some(_count) = self.try_parse_array_count() {
                    // Nested array: key[count]=item1 item2 item3
                    self.skip_whitespace();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        let items = self.parse_space_separated_items(separator)?;
                        fields.insert(key, DxLlmValue::Arr(items));
                    } else {
                        // No '=' after [count], treat as error or null
                        fields.insert(key, DxLlmValue::Null);
                    }
                } else {
                    // Nested object
                    let nested = self.parse_inline_object(&key)?;
                    fields.insert(key, nested);
                }
            } else if self.peek_char() == Some(':') {
                // Nested table or array
                self.advance();
                let rest = self.parse_until_delimiter(&['(', '=', ',', ']', '\n'])?;

                if self.peek_char() == Some('(') {
                    let section = self.parse_table(&rest)?;
                    // Convert section to array of objects
                    let arr = self.section_to_array(&section);
                    fields.insert(key, arr);
                } else if self.peek_char() == Some('=') {
                    self.advance();
                    let items_str = self.parse_until_delimiter(&[',', ']', '\n'])?;

                    // Auto-detect separator: comma (legacy) or space (new format)
                    let items: Vec<DxLlmValue> = if items_str.contains(',') {
                        // Comma-separated (legacy format)
                        items_str.split(',').map(|s| LlmParser::parse_value(s.trim())).collect()
                    } else {
                        // Space-separated (new format)
                        items_str.split_whitespace().map(LlmParser::parse_value).collect()
                    };
                    fields.insert(key, DxLlmValue::Arr(items));
                } else {
                    fields.insert(key, LlmParser::parse_value(&rest));
                }
            } else {
                fields.insert(key, DxLlmValue::Null);
            }

            // Skip separator if present
            self.skip_whitespace();
            if self.peek_char() == Some(separator) {
                self.advance();
                // For space separator, skip any additional spaces
                if separator == ' ' {
                    self.skip_whitespace();
                }
            }
        }

        // Return proper Obj variant for type safety
        if fields.is_empty() {
            Ok(DxLlmValue::Null)
        } else {
            Ok(DxLlmValue::Obj(fields))
        }
    }

    /// Detect the separator used in an inline object by scanning ahead
    /// Returns ',' for comma-separated (legacy) or ' ' for space-separated (new format)
    fn detect_object_separator(&self) -> Result<char, ParseError> {
        let mut temp_pos = self.pos;
        let mut depth = 0;
        let mut after_value = false;

        while temp_pos < self.input.len() {
            let ch = self.input[temp_pos..].chars().next().unwrap_or('\0');

            match ch {
                '[' | '(' => depth += 1,
                ']' | ')' => {
                    if depth == 0 {
                        // Reached end without finding separator, default to space
                        return Ok(' ');
                    }
                    depth -= 1;
                }
                '=' => {
                    // After '=', we're in a value
                    after_value = true;
                }
                ',' if depth == 0 && after_value => {
                    // Found comma separator at depth 0 after a value
                    return Ok(',');
                }
                ' ' if depth == 0 && after_value => {
                    // Check if this space is between fields (after a value, before next key)
                    // Look ahead to see if next non-space char is an identifier start
                    let mut next_pos = temp_pos + ch.len_utf8();
                    while next_pos < self.input.len() {
                        let next_ch = self.input[next_pos..].chars().next().unwrap_or('\0');
                        if next_ch == ' ' || next_ch == '\t' {
                            next_pos += next_ch.len_utf8();
                            continue;
                        }
                        // If next char is alphanumeric, this space is a separator
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            return Ok(' ');
                        }
                        break;
                    }
                }
                _ => {}
            }

            temp_pos += ch.len_utf8();
        }

        // Default to space-separated (new format)
        Ok(' ')
    }

    /// Detect the row separator used in tabular data by scanning ahead
    /// Returns ',', ';', ':', or '\n' based on content
    /// Tracks nesting depth to avoid detecting separators inside nested structures
    fn detect_row_separator(&self) -> Result<char, ParseError> {
        let mut temp_pos = self.pos;
        let mut depth = 0;

        while temp_pos < self.input.len() {
            let ch = self.input[temp_pos..].chars().next().unwrap_or('\0');

            match ch {
                '[' | '(' => depth += 1,
                ']' | ')' => {
                    if depth == 0 {
                        // Reached end of table without finding separator, default to newline
                        return Ok('\n');
                    }
                    depth -= 1;
                }
                ',' if depth == 0 => return Ok(','),
                ';' if depth == 0 => return Ok(';'),
                ':' if depth == 0 => return Ok(':'),
                '\n' if depth == 0 => return Ok('\n'),
                _ => {}
            }

            temp_pos += ch.len_utf8();
        }

        // Default to newline
        Ok('\n')
    }

    /// Parse space-separated array items for nested arrays in inline objects
    /// Handles both space-separated (new format) and comma-separated (legacy) items
    /// Used for syntax like: key[count]=item1 item2 item3
    fn parse_space_separated_items(
        &mut self,
        separator: char,
    ) -> Result<Vec<DxLlmValue>, ParseError> {
        let mut items = Vec::new();
        let mut current_item = String::new();
        let mut in_nested: i32 = 0;

        // Determine the item separator based on the object separator
        // If object uses space separator, array items are space-separated
        // If object uses comma separator, array items are comma-separated
        let item_separator = separator;

        while self.pos < self.input.len() {
            let ch = self.current_char();

            match ch {
                '[' | '(' => {
                    in_nested += 1;
                    current_item.push(ch);
                    self.advance();
                }
                ']' if in_nested == 0 => {
                    // End of inline object - don't consume the ']'
                    if !current_item.trim().is_empty() {
                        items.push(LlmParser::parse_value(current_item.trim()));
                    }
                    break;
                }
                ']' | ')' => {
                    in_nested = in_nested.saturating_sub(1);
                    current_item.push(ch);
                    self.advance();
                }
                '\n' | '\r' => {
                    // End of line - end of array items
                    if !current_item.trim().is_empty() {
                        items.push(LlmParser::parse_value(current_item.trim()));
                    }
                    break;
                }
                ' ' if item_separator == ' ' && in_nested == 0 => {
                    // For space-separated items, check if this space marks the end of array items
                    // Look ahead to see if we have key=value pattern (next field in object)
                    let mut temp_pos = self.pos + 1;

                    // Skip any additional spaces
                    while temp_pos < self.input.len() {
                        let next_ch = self.input[temp_pos..].chars().next().unwrap_or('\0');
                        if next_ch == ' ' || next_ch == '\t' {
                            temp_pos += next_ch.len_utf8();
                        } else {
                            break;
                        }
                    }

                    // Check if we have an identifier followed by '=' or '[' (next field)
                    let mut has_equals = false;
                    let mut has_bracket = false;
                    let mut identifier_chars = 0;

                    while temp_pos < self.input.len() {
                        let next_ch = self.input[temp_pos..].chars().next().unwrap_or('\0');
                        if next_ch.is_alphanumeric()
                            || next_ch == '_'
                            || next_ch == '-'
                            || next_ch == '.'
                        {
                            identifier_chars += 1;
                            temp_pos += next_ch.len_utf8();
                        } else if next_ch == '=' && identifier_chars > 0 {
                            has_equals = true;
                            break;
                        } else if next_ch == '[' && identifier_chars > 0 {
                            has_bracket = true;
                            break;
                        } else {
                            break;
                        }
                    }

                    if has_equals || has_bracket {
                        // This space marks the end of array items, next field starts
                        if !current_item.trim().is_empty() {
                            items.push(LlmParser::parse_value(current_item.trim()));
                        }
                        break;
                    } else {
                        // This is a space between array items
                        if !current_item.trim().is_empty() {
                            items.push(LlmParser::parse_value(current_item.trim()));
                            current_item.clear();
                        }
                        self.advance();
                        // Skip extra spaces
                        while self.pos < self.input.len() && self.current_char() == ' ' {
                            self.advance();
                        }
                    }
                }
                ',' if item_separator == ',' && in_nested == 0 => {
                    // For comma-separated items, check if this comma marks the end of array items
                    // Look ahead to see if we have key=value pattern (next field in object)
                    let mut temp_pos = self.pos + 1;

                    // Skip any spaces after comma
                    while temp_pos < self.input.len() {
                        let next_ch = self.input[temp_pos..].chars().next().unwrap_or('\0');
                        if next_ch == ' ' || next_ch == '\t' {
                            temp_pos += next_ch.len_utf8();
                        } else {
                            break;
                        }
                    }

                    // Check if we have an identifier followed by '=' or '[' (next field)
                    let mut has_equals = false;
                    let mut has_bracket = false;
                    let mut identifier_chars = 0;

                    while temp_pos < self.input.len() {
                        let next_ch = self.input[temp_pos..].chars().next().unwrap_or('\0');
                        if next_ch.is_alphanumeric()
                            || next_ch == '_'
                            || next_ch == '-'
                            || next_ch == '.'
                        {
                            identifier_chars += 1;
                            temp_pos += next_ch.len_utf8();
                        } else if next_ch == '=' && identifier_chars > 0 {
                            has_equals = true;
                            break;
                        } else if next_ch == '[' && identifier_chars > 0 {
                            has_bracket = true;
                            break;
                        } else {
                            break;
                        }
                    }

                    if has_equals || has_bracket {
                        // This comma marks the end of array items, next field starts
                        if !current_item.trim().is_empty() {
                            items.push(LlmParser::parse_value(current_item.trim()));
                        }
                        break;
                    } else {
                        // This is a comma between array items
                        if !current_item.trim().is_empty() {
                            items.push(LlmParser::parse_value(current_item.trim()));
                            current_item.clear();
                        }
                        self.advance();
                    }
                }
                _ => {
                    current_item.push(ch);
                    self.advance();
                }
            }
        }

        Ok(items)
    }

    /// Parse compact syntax object: name:count@=[key value key value]
    /// Format uses @= marker followed by space-separated key-value pairs without = signs
    /// Tokens are paired up: first token is key, second is value, third is key, fourth is value, etc.
    fn parse_compact_object(
        &mut self,
        _name: &str,
        _count: usize,
    ) -> Result<DxLlmValue, ParseError> {
        let start_pos = self.pos;

        // Expect @=[
        self.expect_char('@')?;
        self.expect_char('=')?;
        self.expect_char('[')?;

        let mut fields = IndexMap::new();
        let mut tokens = Vec::new();

        // Parse all tokens until ]
        loop {
            self.skip_whitespace();

            if self.peek_char() == Some(']') {
                self.advance();
                break;
            }

            if self.pos >= self.input.len() {
                return Err(ParseError::UnclosedBracket { pos: start_pos });
            }

            let token = self.parse_identifier()?;
            if token.is_empty() {
                break;
            }
            tokens.push(token);
        }

        // Pair up tokens as key-value pairs
        if tokens.len() % 2 != 0 {
            return Err(ParseError::InvalidTable {
                msg: format!("Compact syntax requires even number of tokens, got {}", tokens.len()),
            });
        }

        for chunk in tokens.chunks(2) {
            let key = chunk[0].clone();
            let value = LlmParser::parse_value(&chunk[1]);
            fields.insert(key, value);
        }

        Ok(DxLlmValue::Obj(fields))
    }

    fn parse_table(&mut self, count_str: &str) -> Result<DxSection, ParseError> {
        let start_pos = self.pos;

        // Parse count (optional)
        let _count: usize = count_str.trim().parse().unwrap_or(0);

        // Parse schema: (col1 col2 col3) or (col1,col2,col3)
        // Space-separated is the new default, comma-separated for backward compatibility
        self.expect_char('(')?;
        let schema_str = self.parse_until_char(')')?;
        self.expect_char(')')?;

        // Detect separator: if contains comma, use comma; otherwise use space
        let schema: Vec<String> = if schema_str.contains(',') {
            schema_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else {
            schema_str.split_whitespace().map(|s| s.to_string()).collect()
        };

        if schema.is_empty() {
            return Err(ParseError::InvalidTable {
                msg: "Empty schema".to_string(),
            });
        }

        // Parse prefix/suffix markers (if present)
        let prefix_info = self.parse_prefix_markers()?;

        // Parse data: [row1\nrow2\n...]
        self.skip_whitespace();
        self.expect_char('[')?;

        // Detect row separator by scanning ahead
        let row_separator = self.detect_row_separator()?;

        let mut section = DxSection::new(schema.clone());

        // Route to appropriate parser based on separator
        match row_separator {
            '\n' => {
                // Newline-separated rows (existing logic)
                let mut iteration_count = 0;
                const MAX_ITERATIONS: usize = 10_000_000; // Safety limit (matches MAX_TABLE_ROWS)

                // Parse rows until closing bracket
                loop {
                    iteration_count += 1;
                    if iteration_count > MAX_ITERATIONS {
                        return Err(ParseError::InvalidTable {
                            msg: "Table parsing exceeded maximum iterations".to_string(),
                        });
                    }

                    self.skip_whitespace_no_newline();

                    if self.peek_char() == Some(']') {
                        self.advance();
                        break;
                    }

                    if self.pos >= self.input.len() {
                        return Err(ParseError::UnclosedBracket { pos: start_pos });
                    }

                    // Skip empty lines
                    if self.peek_char() == Some('\n') || self.peek_char() == Some('\r') {
                        self.advance();
                        continue;
                    }

                    // Parse row
                    let mut row = self.parse_table_row(schema.len())?;
                    if !row.is_empty() {
                        if row.len() != schema.len() {
                            return Err(ParseError::SchemaMismatch {
                                expected: schema.len(),
                                got: row.len(),
                            });
                        }
                        // Apply prefixes/suffixes to the row
                        self.apply_prefixes_to_row(&mut row, &prefix_info, &schema);
                        section.rows.push(row);
                    }
                }
            }
            ',' | ';' | ':' => {
                // Inline separated rows
                self.parse_inline_separated_rows(
                    &mut section,
                    &schema,
                    row_separator,
                    &prefix_info,
                )?;
                self.expect_char(']')?;
            }
            _ => {
                return Err(ParseError::InvalidTable {
                    msg: format!("Unknown row separator: {}", row_separator),
                });
            }
        }

        Ok(section)
    }

    /// Parse inline separated rows for comma, semicolon, or colon separators
    /// Format: [row1, row2, row3] or [row1; row2; row3] or [row1: row2: row3]
    /// Each row contains space-separated column values
    fn parse_inline_separated_rows(
        &mut self,
        section: &mut DxSection,
        schema: &[String],
        separator: char,
        prefix_info: &PrefixInfo,
    ) -> Result<(), ParseError> {
        let mut iteration_count = 0;
        const MAX_ITERATIONS: usize = 10_000_000; // Safety limit

        loop {
            iteration_count += 1;
            if iteration_count > MAX_ITERATIONS {
                return Err(ParseError::InvalidTable {
                    msg: "Inline row parsing exceeded maximum iterations".to_string(),
                });
            }

            self.skip_whitespace_no_newline();

            if self.peek_char() == Some(']') {
                break;
            }

            if self.pos >= self.input.len() {
                return Err(ParseError::UnexpectedEof);
            }

            // Parse a single inline row
            let mut row = self.parse_inline_row(schema.len(), separator)?;
            if !row.is_empty() {
                if row.len() != schema.len() {
                    return Err(ParseError::SchemaMismatch {
                        expected: schema.len(),
                        got: row.len(),
                    });
                }
                // Apply prefixes/suffixes to the row
                self.apply_prefixes_to_row(&mut row, prefix_info, schema);
                section.rows.push(row);
            }

            // Skip separator if present
            self.skip_whitespace_no_newline();
            if self.peek_char() == Some(separator) {
                self.advance();
                self.skip_whitespace_no_newline();
            }
        }

        Ok(())
    }

    /// Parse a single inline row with space-separated column values
    /// Stops at the row separator (comma, semicolon, colon) or closing bracket
    fn parse_inline_row(
        &mut self,
        expected_cols: usize,
        row_separator: char,
    ) -> Result<Vec<DxLlmValue>, ParseError> {
        let mut values = Vec::with_capacity(expected_cols);
        let mut current_value = String::new();
        let mut in_nested: i32 = 0;

        while self.pos < self.input.len() {
            let ch = self.current_char();

            match ch {
                '[' | '(' => {
                    in_nested += 1;
                    current_value.push(ch);
                    self.advance();
                }
                ']' if in_nested == 0 => {
                    // End of table data - don't consume the ']'
                    if !current_value.trim().is_empty() {
                        values.push(self.parse_table_value(&current_value, ' '));
                    }
                    break;
                }
                ']' | ')' => {
                    in_nested = in_nested.saturating_sub(1);
                    current_value.push(ch);
                    self.advance();
                }
                c if c == row_separator && in_nested == 0 => {
                    // End of this row - don't consume the separator
                    if !current_value.trim().is_empty() {
                        values.push(self.parse_table_value(&current_value, ' '));
                    }
                    break;
                }
                ' ' if in_nested == 0 => {
                    // Space separates column values within a row
                    if !current_value.trim().is_empty() {
                        values.push(self.parse_table_value(&current_value, ' '));
                        current_value.clear();
                    }
                    self.advance();
                    // Skip extra spaces
                    while self.pos < self.input.len() && self.current_char() == ' ' {
                        self.advance();
                    }
                }
                '\n' | '\r' => {
                    // Newline within inline format - treat as end of row
                    if !current_value.trim().is_empty() {
                        values.push(self.parse_table_value(&current_value, ' '));
                    }
                    break;
                }
                _ => {
                    current_value.push(ch);
                    self.advance();
                }
            }
        }

        Ok(values)
    }

    fn parse_table_row(&mut self, expected_cols: usize) -> Result<Vec<DxLlmValue>, ParseError> {
        let mut values = Vec::with_capacity(expected_cols);
        let mut current_value = String::new();
        let mut in_parens: i32 = 0;

        // Detect separator: space or comma
        // Look ahead to determine which separator is used
        let _line_start = self.pos;
        let mut has_comma = false;
        let mut temp_pos = self.pos;
        while temp_pos < self.input.len() {
            let ch = self.input[temp_pos..].chars().next().unwrap_or('\0');
            if ch == '\n' || ch == '\r' || ch == ']' {
                break;
            }
            if ch == ',' {
                has_comma = true;
                break;
            }
            temp_pos += ch.len_utf8();
        }

        let separator = if has_comma { ',' } else { ' ' };

        while self.pos < self.input.len() {
            let ch = self.current_char();

            match ch {
                '\n' | '\r' => {
                    // End of row
                    if !current_value.is_empty() || !values.is_empty() {
                        values.push(self.parse_table_value(&current_value, separator));
                    }
                    self.advance();
                    break;
                }
                ']' if in_parens == 0 => {
                    // End of table data
                    if !current_value.is_empty() || !values.is_empty() {
                        values.push(self.parse_table_value(&current_value, separator));
                    }
                    break;
                }
                c if c == separator && in_parens == 0 => {
                    values.push(self.parse_table_value(&current_value, separator));
                    current_value.clear();
                    self.advance();
                    // Skip extra spaces if space-separated
                    if separator == ' ' {
                        while self.pos < self.input.len() && self.current_char() == ' ' {
                            self.advance();
                        }
                    }
                }
                '(' => {
                    in_parens += 1;
                    current_value.push(ch);
                    self.advance();
                }
                ')' => {
                    in_parens = in_parens.saturating_sub(1);
                    current_value.push(ch);
                    self.advance();
                }
                _ => {
                    current_value.push(ch);
                    self.advance();
                }
            }
        }

        Ok(values)
    }

    /// Parse a table value preserving natural spaces for optimal tokenization
    fn parse_table_value(&self, s: &str, _separator: char) -> DxLlmValue {
        let s = s.trim();

        // Boolean values: true/false only (removed +/- shorthand)
        if s == "true" {
            return DxLlmValue::Bool(true);
        }
        if s == "false" {
            return DxLlmValue::Bool(false);
        }
        if s == "null" {
            return DxLlmValue::Null;
        }

        // Try to parse as number
        if let Ok(n) = s.parse::<i64>() {
            return DxLlmValue::Num(n as f64);
        }
        if let Ok(n) = s.parse::<f64>() {
            return DxLlmValue::Num(n);
        }

        DxLlmValue::Str(s.to_string())
    }

    /// Parse prefix and suffix markers before table data
    /// Recognizes `@prefix` patterns and `@@suffix` patterns (double @)
    /// Format: @prefix1 @prefix2 @@suffix1 [table data]
    /// Returns PrefixInfo with collected prefixes and suffixes
    fn parse_prefix_markers(&mut self) -> Result<PrefixInfo, ParseError> {
        let mut prefixes = Vec::new();
        let mut suffixes = Vec::new();

        loop {
            self.skip_whitespace_no_newline();

            // Check if we're at a prefix/suffix marker
            if self.peek_char() != Some('@') {
                break;
            }

            self.advance(); // consume first @

            // Check for suffix marker (@@)
            let is_suffix = if self.peek_char() == Some('@') {
                self.advance(); // consume second @
                true
            } else {
                false
            };

            // Parse the prefix/suffix value until we hit a delimiter
            // Stop at: '[' (table start), ' ' (next marker), '\n', '\r'
            let mut value = String::new();
            while self.pos < self.input.len() {
                let ch = self.current_char();
                if ch == '[' || ch == ' ' || ch == '\n' || ch == '\r' {
                    break;
                }
                value.push(ch);
                self.advance();
            }

            if !value.is_empty() {
                if is_suffix {
                    suffixes.push(value);
                } else {
                    prefixes.push(value);
                }
            }
        }

        Ok(PrefixInfo { prefixes, suffixes })
    }

    /// Apply prefixes and suffixes to a row based on column name heuristics
    /// Modifies string values in-place by prepending prefixes or appending suffixes
    /// Uses column names to determine which columns should receive prefixes/suffixes
    fn apply_prefixes_to_row(
        &self,
        row: &mut [DxLlmValue],
        prefix_info: &PrefixInfo,
        schema: &[String],
    ) {
        // Apply prefixes to appropriate columns
        if !prefix_info.prefixes.is_empty() {
            for (i, value) in row.iter_mut().enumerate() {
                if let DxLlmValue::Str(s) = value {
                    let col_name = schema.get(i).map(|s| s.as_str()).unwrap_or("");
                    let col_lower = col_name.to_lowercase();

                    // Heuristic: Apply prefix to columns that look like they need it
                    // Common patterns: endpoint, path, url, route, uri, file, directory
                    if col_lower.contains("endpoint")
                        || col_lower.contains("path")
                        || col_lower.contains("url")
                        || col_lower.contains("route")
                        || col_lower.contains("uri")
                        || col_lower.contains("file")
                        || col_lower.contains("directory")
                        || col_lower.contains("dir")
                    {
                        // Apply first prefix (most common case)
                        *s = format!("{}{}", prefix_info.prefixes[0], s);
                    }
                }
            }
        }

        // Apply suffixes to appropriate columns
        if !prefix_info.suffixes.is_empty() {
            for (i, value) in row.iter_mut().enumerate() {
                if let DxLlmValue::Str(s) = value {
                    let col_name = schema.get(i).map(|s| s.as_str()).unwrap_or("");
                    let col_lower = col_name.to_lowercase();

                    // Heuristic: Apply suffix to columns that look like they need it
                    // Common patterns: email, domain, host, hostname
                    if col_lower.contains("email")
                        || col_lower.contains("domain")
                        || col_lower.contains("host")
                    {
                        // Apply first suffix (most common case)
                        *s = format!("{}{}", s, prefix_info.suffixes[0]);
                    }
                }
            }
        }
    }

    /// Parse wrapped dataframe rows: rows inside parentheses, one per line
    /// Format: (row1\nrow2\nrow3)
    fn parse_wrapped_dataframe_rows(&mut self, schema: &[String]) -> Result<DxSection, ParseError> {
        let mut section = DxSection::new(schema.to_vec());

        loop {
            self.skip_whitespace();

            // Check for end of wrapped rows
            if self.peek_char() == Some(')') {
                break;
            }

            if self.pos >= self.input.len() {
                return Err(ParseError::UnexpectedEof);
            }

            // Parse a row (space-separated values with quote support)
            let row = self.parse_wrapped_row(schema.len())?;
            if !row.is_empty() {
                if row.len() != schema.len() {
                    return Err(ParseError::SchemaMismatch {
                        expected: schema.len(),
                        got: row.len(),
                    });
                }
                section.rows.push(row);
            }
        }

        Ok(section)
    }

    /// Parse a single wrapped dataframe row (space-separated with quote support)
    fn parse_wrapped_row(&mut self, expected_cols: usize) -> Result<Vec<DxLlmValue>, ParseError> {
        let mut values = Vec::with_capacity(expected_cols);
        let mut current_value = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;

        while self.pos < self.input.len() {
            let ch = self.current_char();

            if escape_next {
                current_value.push(ch);
                escape_next = false;
                self.advance();
                continue;
            }

            match ch {
                '\\' if in_quotes => {
                    escape_next = true;
                    self.advance();
                }
                '"' => {
                    in_quotes = !in_quotes;
                    current_value.push(ch);
                    self.advance();
                }
                ' ' if !in_quotes => {
                    // Space separates values
                    if !current_value.trim().is_empty() {
                        values.push(LlmParser::parse_value(current_value.trim()));
                        current_value.clear();
                    }
                    self.advance();
                }
                '\n' | '\r' if !in_quotes => {
                    // End of row
                    if !current_value.trim().is_empty() {
                        values.push(LlmParser::parse_value(current_value.trim()));
                    }
                    self.advance();
                    break;
                }
                ')' if !in_quotes => {
                    // End of wrapped dataframe - don't consume
                    if !current_value.trim().is_empty() {
                        values.push(LlmParser::parse_value(current_value.trim()));
                    }
                    break;
                }
                _ => {
                    current_value.push(ch);
                    self.advance();
                }
            }
        }

        Ok(values)
    }

    /// Parse parenthesized object: (key=val key2=val2)
    fn parse_parenthesized_object(&mut self) -> Result<DxLlmValue, ParseError> {
        let mut fields = IndexMap::new();

        loop {
            self.skip_whitespace();

            if self.peek_char() == Some(')') {
                break;
            }

            if self.pos >= self.input.len() {
                return Err(ParseError::UnexpectedEof);
            }

            // Parse key
            let key = self.parse_identifier()?;
            if key.is_empty() {
                break;
            }

            self.skip_whitespace();

            // Expect '='
            if self.peek_char() != Some('=') {
                return Err(ParseError::UnexpectedChar {
                    ch: self.current_char(),
                    pos: self.pos,
                });
            }
            self.advance();

            // Parse value (until space or closing paren)
            // Check if value is an array: =[...]
            if self.peek_char() == Some('[') {
                self.advance(); // consume '['
                let items_str = self.parse_until_char(']')?;
                self.expect_char(']')?;
                let items = self.parse_quoted_items(&items_str);
                fields.insert(key, DxLlmValue::Arr(items));
            } else {
                let value_str = self.parse_until_delimiter(&[' ', ')', '\n'])?;
                fields.insert(key, LlmParser::parse_value(&value_str));
            }

            self.skip_whitespace();
        }

        Ok(DxLlmValue::Obj(fields))
    }

    /// Parse space-separated items with quote support
    fn parse_quoted_items(&self, input: &str) -> Vec<DxLlmValue> {
        let mut items = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in input.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_quotes => {
                    escape_next = true;
                }
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ' ' if !in_quotes => {
                    if !current.trim().is_empty() {
                        items.push(LlmParser::parse_value(current.trim()));
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        if !current.trim().is_empty() {
            items.push(LlmParser::parse_value(current.trim()));
        }

        items
    }

    fn parse_identifier(&mut self) -> Result<String, ParseError> {
        let mut name = String::new();

        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '.' || ch == '@' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(name)
    }

    fn parse_value_until_delimiter(
        &mut self,
        delimiters: &[char],
    ) -> Result<DxLlmValue, ParseError> {
        let value_str = self.parse_until_delimiter(delimiters)?;
        Ok(LlmParser::parse_value(&value_str))
    }

    fn parse_until_delimiter(&mut self, delimiters: &[char]) -> Result<String, ParseError> {
        let mut value = String::new();
        let mut in_nested = 0;

        while self.pos < self.input.len() {
            let ch = self.current_char();

            if ch == '[' || ch == '(' {
                in_nested += 1;
                value.push(ch);
                self.advance();
            } else if ch == ']' || ch == ')' {
                if in_nested > 0 {
                    in_nested -= 1;
                    value.push(ch);
                    self.advance();
                } else if delimiters.contains(&ch) {
                    break;
                } else {
                    value.push(ch);
                    self.advance();
                }
            } else if delimiters.contains(&ch) && in_nested == 0 {
                break;
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Ok(value.trim().to_string())
    }

    fn parse_until_char(&mut self, target: char) -> Result<String, ParseError> {
        let mut value = String::new();
        let mut depth = 0;

        while self.pos < self.input.len() {
            let ch = self.current_char();

            if ch == '(' {
                depth += 1;
                value.push(ch);
                self.advance();
            } else if ch == ')' {
                if depth > 0 {
                    depth -= 1;
                    value.push(ch);
                    self.advance();
                } else if ch == target {
                    // Found the closing paren we're looking for
                    break;
                } else {
                    value.push(ch);
                    self.advance();
                }
            } else if ch == target && depth == 0 {
                break;
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Ok(value)
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Err(ParseError::UnexpectedEof);
        }
        let ch = self.current_char();
        if ch != expected {
            return Err(ParseError::UnexpectedChar { ch, pos: self.pos });
        }
        self.advance();
        Ok(())
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap_or('\0')
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if self.pos < self.input.len() {
            self.pos += self.current_char().len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_whitespace_no_newline(&mut self) {
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn section_to_array(&self, section: &DxSection) -> DxLlmValue {
        let mut arr = Vec::new();
        for row in &section.rows {
            // Convert row to string representation
            let row_str = row.iter().map(Self::value_to_string).collect::<Vec<_>>().join(",");
            arr.push(DxLlmValue::Str(row_str));
        }
        DxLlmValue::Arr(arr)
    }

    fn value_to_string(v: &DxLlmValue) -> String {
        match v {
            DxLlmValue::Str(s) => s.clone(),
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            DxLlmValue::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            DxLlmValue::Null => "null".to_string(),
            DxLlmValue::Arr(items) => {
                let s: Vec<String> = items.iter().map(Self::value_to_string).collect();
                s.join(",")
            }
            DxLlmValue::Obj(fields) => {
                let s: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, Self::value_to_string(v)))
                    .collect();
                format!("[{}]", s.join(","))
            }
            DxLlmValue::Ref(r) => format!("^{}", r),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let doc = LlmParser::parse("").unwrap();
        assert!(doc.is_empty());
    }

    #[test]
    fn test_parse_simple_key_value() {
        let input = "environment: development\nversion: 2.2.16";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.context.len(), 2);
        assert_eq!(doc.context.get("environment").unwrap().as_str(), Some("development"));
        assert_eq!(doc.context.get("version").unwrap().as_str(), Some("2.2.16"));
    }

    #[test]
    fn test_parse_leaf_inlining() {
        // Leaf inlining with :: for primitive values
        let input = "forge.repository:: https://dx.vercel.app/user/repo\nstyle.path:: @/style";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.context.len(), 2);
        assert_eq!(
            doc.context.get("forge.repository").unwrap().as_str(),
            Some("https://dx.vercel.app/user/repo")
        );
        assert_eq!(doc.context.get("style.path").unwrap().as_str(), Some("@/style"));
    }

    #[test]
    fn test_parse_mixed_standard_and_leaf() {
        let input = "name: dx\nversion: 0.0.1\nforge.repository:: https://example.com\neditors.default: neovim";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.context.len(), 4);
        assert_eq!(doc.context.get("name").unwrap().as_str(), Some("dx"));
        assert_eq!(doc.context.get("version").unwrap().as_str(), Some("0.0.1"));
        assert_eq!(
            doc.context.get("forge.repository").unwrap().as_str(),
            Some("https://example.com")
        );
        assert_eq!(doc.context.get("editors.default").unwrap().as_str(), Some("neovim"));
    }

    #[test]
    fn test_parse_array_with_count() {
        let input = "editors.items[3]: neovim,zed,vscode";
        let doc = LlmParser::parse(input).unwrap();

        let items = doc.context.get("editors.items").unwrap();
        if let DxLlmValue::Arr(arr) = items {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].as_str(), Some("neovim"));
            assert_eq!(arr[1].as_str(), Some("zed"));
            assert_eq!(arr[2].as_str(), Some("vscode"));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_parse_object() {
        let input = "config[host=localhost,port=8080,debug=true]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        let config = doc.context.get("config").unwrap();

        // Verify it's an Obj variant with proper fields
        if let DxLlmValue::Obj(fields) = config {
            assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
            assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
            assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
        } else {
            panic!("Expected Obj variant, got {:?}", config);
        }
    }

    #[test]
    fn test_parse_table() {
        // Space-separated format (new default)
        let input = "metrics:3(date views clicks)[\n2025-01-01 4836 193\n2025-01-02 6525 196\n2025-01-03 7927 238]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["date", "views", "clicks"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_parse_table_comma_separated() {
        // Comma-separated format (backward compatibility)
        let input = "metrics:3(date,views,clicks)[\n2025-01-01,4836,193\n2025-01-02,6525,196\n2025-01-03,7927,238]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["date", "views", "clicks"]);
        assert_eq!(section.rows.len(), 3);
    }

    #[test]
    fn test_parse_table_with_quoted_names() {
        // NEW FORMAT: Wrapped dataframe with quoted strings
        let input =
            "employees[id name dept](\n1 \"James Smith\" Engineering\n2 \"Mary Johnson\" Sales\n)";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "dept"]);
        assert_eq!(section.rows.len(), 2);

        // Names with spaces use quotes
        assert_eq!(section.rows[0][1].as_str(), Some("James Smith"));
        assert_eq!(section.rows[1][1].as_str(), Some("Mary Johnson"));
    }

    #[test]
    fn test_parse_simple_array() {
        let input = "friends:3=ana,luis,sam";
        let doc = LlmParser::parse(input).unwrap();

        let friends = doc.context.get("friends").unwrap();
        if let DxLlmValue::Arr(items) = friends {
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].as_str(), Some("ana"));
            assert_eq!(items[1].as_str(), Some("luis"));
            assert_eq!(items[2].as_str(), Some("sam"));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_parse_inline_object_with_count() {
        // Test section:count[key=value key2=value2] syntax
        let input = "config:3[host=localhost port=8080 debug=true]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        let config = doc.context.get("config").unwrap();

        // Verify it's an Obj variant with proper fields
        if let DxLlmValue::Obj(fields) = config {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
            assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
            assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
        } else {
            panic!("Expected Obj variant, got {:?}", config);
        }
    }

    #[test]
    fn test_parse_inline_object_with_count_comma_separated() {
        // Test backward compatibility with comma-separated fields
        let input = "config:3[host=localhost,port=8080,debug=true]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        let config = doc.context.get("config").unwrap();

        if let DxLlmValue::Obj(fields) = config {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
            assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
            assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
        } else {
            panic!("Expected Obj variant, got {:?}", config);
        }
    }

    #[test]
    fn test_parse_compact_syntax() {
        // Test compact syntax: section:count@=[key value key value]
        let input = "config:3@=[host localhost port 8080 debug true]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        let config = doc.context.get("config").unwrap();

        if let DxLlmValue::Obj(fields) = config {
            assert_eq!(fields.len(), 3);
            assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
            assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
            assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
        } else {
            panic!("Expected Obj variant, got {:?}", config);
        }
    }

    #[test]
    fn test_parse_compact_syntax_odd_tokens() {
        // Test that compact syntax with odd number of tokens returns error
        let input = "config:2@=[host localhost port]";
        let result = LlmParser::parse(input);

        assert!(result.is_err());
        if let Err(ParseError::InvalidTable { msg }) = result {
            assert!(msg.contains("even number of tokens"));
        } else {
            panic!("Expected InvalidTable error for odd token count");
        }
    }

    #[test]
    fn test_parse_compact_syntax_empty() {
        // Test compact syntax with no tokens
        let input = "config:0@=[]";
        let doc = LlmParser::parse(input).unwrap();

        assert!(doc.context.contains_key("config"));
        let config = doc.context.get("config").unwrap();

        if let DxLlmValue::Obj(fields) = config {
            assert_eq!(fields.len(), 0);
        } else {
            panic!("Expected Obj variant, got {:?}", config);
        }
    }

    #[test]
    fn test_parse_booleans() {
        let input = "active: true\ndeleted: false";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.context.get("active").unwrap().as_bool(), Some(true));
        assert_eq!(doc.context.get("deleted").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn test_parse_numbers() {
        let input = "count: 42\nprice: 19.99";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.context.get("count").unwrap().as_num(), Some(42.0));
        assert_eq!(doc.context.get("price").unwrap().as_num(), Some(19.99));
    }

    #[test]
    fn test_parse_dots_in_keys() {
        // Dots in keys are allowed - no escaping needed
        let input = "js.dependencies.react:: 19.0.1\npython.dependencies.django:: latest";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.context.get("js.dependencies.react").unwrap().as_str(), Some("19.0.1"));
        assert_eq!(doc.context.get("python.dependencies.django").unwrap().as_str(), Some("latest"));
    }

    #[test]
    fn test_parse_bytes_valid_utf8() {
        let input = b"name: Test";
        let doc = LlmParser::parse_bytes(input).unwrap();
        assert_eq!(doc.context.get("name").unwrap().as_str(), Some("Test"));
    }

    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let input = &[0x6e, 0x61, 0x6d, 0x65, 0xFF, 0x3d, 0x54]; // "name" + invalid + "=T"
        let err = LlmParser::parse_bytes(input).unwrap_err();
        if let ParseError::Utf8Error { offset } = err {
            assert_eq!(offset, 4);
        } else {
            panic!("Expected Utf8Error, got {:?}", err);
        }
    }

    #[test]
    fn test_parse_table_inline_comma_separated_rows() {
        // Inline rows with comma separator
        let input =
            "users:3(id name email)[1 Alice alice@ex.com, 2 Bob bob@ex.com, 3 Carol carol@ex.com]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "email"]);
        assert_eq!(section.rows.len(), 3);

        // Verify first row
        assert_eq!(section.rows[0][0].as_num(), Some(1.0));
        assert_eq!(section.rows[0][1].as_str(), Some("Alice"));
        assert_eq!(section.rows[0][2].as_str(), Some("alice@ex.com"));

        // Verify second row
        assert_eq!(section.rows[1][0].as_num(), Some(2.0));
        assert_eq!(section.rows[1][1].as_str(), Some("Bob"));
        assert_eq!(section.rows[1][2].as_str(), Some("bob@ex.com"));

        // Verify third row
        assert_eq!(section.rows[2][0].as_num(), Some(3.0));
        assert_eq!(section.rows[2][1].as_str(), Some("Carol"));
        assert_eq!(section.rows[2][2].as_str(), Some("carol@ex.com"));
    }

    #[test]
    fn test_parse_table_inline_semicolon_separated_rows() {
        // Inline rows with semicolon separator
        let input = "products:2(id name price)[101 Widget 9.99; 102 Gadget 19.99]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["id", "name", "price"]);
        assert_eq!(section.rows.len(), 2);

        // Verify first row
        assert_eq!(section.rows[0][0].as_num(), Some(101.0));
        assert_eq!(section.rows[0][1].as_str(), Some("Widget"));
        assert_eq!(section.rows[0][2].as_num(), Some(9.99));

        // Verify second row
        assert_eq!(section.rows[1][0].as_num(), Some(102.0));
        assert_eq!(section.rows[1][1].as_str(), Some("Gadget"));
        assert_eq!(section.rows[1][2].as_num(), Some(19.99));
    }

    #[test]
    fn test_parse_table_inline_colon_separated_rows() {
        // Inline rows with colon separator (simple data without colons in values)
        let input = "status:3(code message count)[200 OK 150: 404 NotFound 25: 500 Error 5]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["code", "message", "count"]);
        assert_eq!(section.rows.len(), 3);

        // Verify first row
        assert_eq!(section.rows[0][0].as_num(), Some(200.0));
        assert_eq!(section.rows[0][1].as_str(), Some("OK"));
        assert_eq!(section.rows[0][2].as_num(), Some(150.0));

        // Verify second row
        assert_eq!(section.rows[1][0].as_num(), Some(404.0));
        assert_eq!(section.rows[1][1].as_str(), Some("NotFound"));
        assert_eq!(section.rows[1][2].as_num(), Some(25.0));

        // Verify third row
        assert_eq!(section.rows[2][0].as_num(), Some(500.0));
        assert_eq!(section.rows[2][1].as_str(), Some("Error"));
        assert_eq!(section.rows[2][2].as_num(), Some(5.0));
    }

    #[test]
    fn test_parse_table_inline_comma_with_whitespace() {
        // Inline rows with comma separator and extra whitespace
        let input = "data:2(x y)[ 1 2 ,  3 4  , 5 6 ]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["x", "y"]);
        assert_eq!(section.rows.len(), 3);

        assert_eq!(section.rows[0][0].as_num(), Some(1.0));
        assert_eq!(section.rows[0][1].as_num(), Some(2.0));
        assert_eq!(section.rows[1][0].as_num(), Some(3.0));
        assert_eq!(section.rows[1][1].as_num(), Some(4.0));
        assert_eq!(section.rows[2][0].as_num(), Some(5.0));
        assert_eq!(section.rows[2][1].as_num(), Some(6.0));
    }

    #[test]
    fn test_parse_table_inline_empty_rows() {
        // Inline format with empty table
        let input = "empty:0(a b c)[]";
        let doc = LlmParser::parse(input).unwrap();

        assert_eq!(doc.sections.len(), 1);
        let section = doc.sections.values().next().unwrap();
        assert_eq!(section.schema, vec!["a", "b", "c"]);
        assert_eq!(section.rows.len(), 0);
    }
}
