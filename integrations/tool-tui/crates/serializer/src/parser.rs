//! Parser for DX Machine format
//!
//! Implements all DX features:
//! - Schema-guided vacuum parsing
//! - Alias system ($)
//! - Prefix inheritance (^)
//! - Vertical ditto (_)
//! - Type hints (%i, %s, %f, %b, %x, %#)
//! - Anchor references (@)
//! - DX ∞: Base62 integers (%x), Auto-increment (%#)
//!
//! ## Thread Safety
//!
//! The [`Parser`] struct is **not** thread-safe (`Send` but not `Sync`) because it
//! maintains mutable parsing state. However, the module-level [`parse()`] function
//! is completely stateless and can be called concurrently from multiple threads:
//!
//! ```rust
//! use std::thread;
//! use serializer::parse;
//!
//! // Safe: each thread creates its own Parser internally
//! let handles: Vec<_> = (0..4).map(|i| {
//!     thread::spawn(move || {
//!         let input = format!("key{}:value{}", i, i);
//!         parse(input.as_bytes())
//!     })
//! }).collect();
//!
//! for handle in handles {
//!     assert!(handle.join().unwrap().is_ok());
//! }
//! ```
//!
//! For parallel parsing, create a separate [`Parser`] instance per thread rather
//! than sharing one instance.

use crate::base62::decode_base62;
use crate::error::{DxError, Result};
use crate::schema::{Schema, TypeHint};
use crate::tokenizer::{Token, Tokenizer};
use crate::types::{DxArray, DxObject, DxTable, DxValue};
use rustc_hash::FxHashMap;

/// Parser state
pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
    /// Alias map ($c = context)
    aliases: FxHashMap<String, String>,
    /// Anchor storage (@1, @2, ...)
    anchors: Vec<DxValue>,
    /// Current prefix for inheritance (^)
    prefix_stack: Vec<String>,
    /// Schema registry for tables
    schemas: FxHashMap<String, Schema>,
    /// Auto-increment counters per table
    auto_counters: FxHashMap<String, i64>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            tokenizer: Tokenizer::new(input),
            aliases: FxHashMap::default(),
            anchors: Vec::new(),
            auto_counters: FxHashMap::default(),
            prefix_stack: Vec::new(),
            schemas: FxHashMap::default(),
        }
    }

    /// Parse the entire input
    pub fn parse(&mut self) -> Result<DxValue> {
        let mut root = DxObject::new();

        loop {
            self.tokenizer.skip_whitespace();
            if self.tokenizer.is_eof() {
                break;
            }

            let token = self.tokenizer.peek_token()?;
            match token {
                Token::Eof => break,
                Token::Newline => {
                    self.tokenizer.next_token()?;
                    continue;
                }
                Token::Dollar => {
                    // Could be alias definition ($c=context) or key reference ($c.task:value)
                    // Need to look ahead to determine which
                    if self.is_alias_definition()? {
                        self.parse_alias()?;
                    } else {
                        // Key reference using alias
                        let (key, value) = self.parse_key_value_with_alias()?;
                        root.insert(key, value);
                    }
                }
                Token::Ident(_) | Token::Caret => {
                    // Key-value pair or table
                    let (key, value) = self.parse_key_value()?;
                    root.insert(key, value);
                }
                _ => {
                    return Err(DxError::InvalidSyntax {
                        pos: self.tokenizer.pos(),
                        msg: format!("Unexpected token: {:?}", token),
                    });
                }
            }
        }

        Ok(DxValue::Object(root))
    }

    /// Check if the current $ starts an alias definition (has = after alias name)
    fn is_alias_definition(&mut self) -> Result<bool> {
        // Save position
        let saved_pos = self.tokenizer.pos();

        // Consume $
        self.tokenizer.next_token()?;

        // Get alias name (may include dots like c.task)
        let alias_token = self.tokenizer.next_token()?;
        if !matches!(alias_token, Token::Ident(_)) {
            // Reset and return false
            self.tokenizer.reset_to(saved_pos);
            return Ok(false);
        }

        // Check next token
        self.tokenizer.skip_whitespace();
        let next = self.tokenizer.peek_token()?;
        let is_definition = matches!(next, Token::Equals);

        // Reset position
        self.tokenizer.reset_to(saved_pos);

        Ok(is_definition)
    }

    /// Parse key-value pair that starts with $ (alias reference)
    fn parse_key_value_with_alias(&mut self) -> Result<(String, DxValue)> {
        // Consume $
        self.tokenizer.next_token()?;

        // Get the full identifier (may include dots like c.task)
        let full_ident = match self.tokenizer.next_token()? {
            Token::Ident(bytes) => std::str::from_utf8(bytes)?.to_string(),
            _ => {
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: "Expected alias name after $".to_string(),
                });
            }
        };

        // Split on first dot to get alias and suffix
        let (alias, suffix) = if let Some(dot_pos) = full_ident.find('.') {
            (&full_ident[..dot_pos], Some(&full_ident[dot_pos..]))
        } else {
            (full_ident.as_str(), None)
        };

        // Resolve alias
        let resolved = self
            .aliases
            .get(alias)
            .cloned()
            .ok_or_else(|| DxError::UnknownAlias(alias.to_string()))?;

        // Build full key
        let key = if let Some(suffix) = suffix {
            format!("{}{}", resolved, suffix)
        } else {
            resolved
        };

        // Read operator
        self.tokenizer.skip_whitespace();
        let operator = self.tokenizer.next_token()?;

        let value = match operator {
            Token::Colon => self.parse_value()?,
            Token::Bang => DxValue::Bool(true),
            Token::Void => DxValue::Null,
            _ => {
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: format!("Expected :, !, or ? after key, got {:?}", operator),
                });
            }
        };

        Ok((key, value))
    }

    /// Parse alias definition: $c=context
    fn parse_alias(&mut self) -> Result<()> {
        self.tokenizer.next_token()?; // consume $

        let alias = match self.tokenizer.next_token()? {
            Token::Ident(bytes) => std::str::from_utf8(bytes)?.to_string(),
            _ => {
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: "Expected alias name after $".to_string(),
                });
            }
        };

        // Expect =
        if !matches!(self.tokenizer.next_token()?, Token::Equals) {
            return Err(DxError::InvalidSyntax {
                pos: self.tokenizer.pos(),
                msg: "Expected = after alias".to_string(),
            });
        }

        let value = match self.tokenizer.next_token()? {
            Token::Ident(bytes) => std::str::from_utf8(bytes)?.to_string(),
            _ => {
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: "Expected value after alias =".to_string(),
                });
            }
        };

        self.aliases.insert(alias, value);
        Ok(())
    }

    /// Parse key-value pair or table definition
    fn parse_key_value(&mut self) -> Result<(String, DxValue)> {
        let mut key = String::new();

        // Handle prefix inheritance (^)
        if matches!(self.tokenizer.peek_token()?, Token::Caret) {
            self.tokenizer.next_token()?;
            if let Some(prefix) = self.prefix_stack.last() {
                key.push_str(prefix);
                key.push('.');
            }
        }

        // Defensive: Check recursion depth (prefix stack depth)
        if self.prefix_stack.len() > crate::error::MAX_RECURSION_DEPTH {
            return Err(DxError::recursion_limit_exceeded(self.prefix_stack.len()));
        }

        // Read the key
        match self.tokenizer.next_token()? {
            Token::Ident(bytes) => {
                let key_str = std::str::from_utf8(bytes)?;
                // Resolve alias if starts with $
                if let Some(alias_key) = key_str.strip_prefix('$') {
                    if let Some(resolved) = self.aliases.get(alias_key) {
                        key.push_str(resolved);
                    } else {
                        return Err(DxError::UnknownAlias(alias_key.to_string()));
                    }
                } else {
                    key.push_str(key_str);
                }
            }
            _ => {
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: "Expected key".to_string(),
                });
            }
        }

        // Save prefix for potential child keys
        let full_key = key.clone();

        // Read operator
        self.tokenizer.skip_whitespace();
        let operator = self.tokenizer.next_token()?;

        let value = match operator {
            Token::Colon => {
                // Simple key:value
                self.prefix_stack.push(full_key.clone());
                let val = self.parse_value()?;
                self.prefix_stack.pop();
                val
            }
            Token::Equals => {
                // Schema definition: table=col1%i col2%s...
                self.parse_table_definition(&key)?
            }
            Token::Stream => {
                // Stream array: key>val1|val2|val3
                self.parse_stream_array()?
            }
            Token::Bang => {
                // Implicit true: admin!
                DxValue::Bool(true)
            }
            Token::Void => {
                // Implicit null: error?
                DxValue::Null
            }
            _ => {
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: format!("Expected :, =, or > after key, got {:?}", operator),
                });
            }
        };

        Ok((key, value))
    }

    /// Parse a value
    ///
    /// # Errors
    ///
    /// Returns `DxError::UnexpectedEof` if end of input is reached before a value is found.
    /// Returns `DxError::InvalidSyntax` for unexpected tokens.
    fn parse_value(&mut self) -> Result<DxValue> {
        self.tokenizer.skip_whitespace();

        let token = self.tokenizer.next_token()?;
        match token {
            Token::Eof => Err(DxError::UnexpectedEof(self.tokenizer.pos())),
            Token::Newline => {
                // Newline without value - treat as unexpected EOF for value context
                Err(DxError::UnexpectedEof(self.tokenizer.pos()))
            }
            Token::Null | Token::Void => Ok(DxValue::Null),
            Token::True => Ok(DxValue::Bool(true)),
            Token::False => Ok(DxValue::Bool(false)),
            Token::Int(i) => Ok(DxValue::Int(i)),
            Token::Float(f) => Ok(DxValue::Float(f)),
            Token::Ditto => Err(DxError::DittoNoPrevious(self.tokenizer.pos())),
            Token::At => {
                // Anchor reference: @1
                let anchor_token = self.tokenizer.next_token()?;
                let anchor_id = match anchor_token {
                    Token::Eof => {
                        return Err(DxError::UnexpectedEof(self.tokenizer.pos()));
                    }
                    Token::Int(i) => i as usize,
                    _ => {
                        return Err(DxError::InvalidSyntax {
                            pos: self.tokenizer.pos(),
                            msg: "Expected number after @".to_string(),
                        });
                    }
                };
                self.anchors
                    .get(anchor_id)
                    .cloned()
                    .ok_or_else(|| DxError::UnknownAnchor(anchor_id.to_string()))
            }
            Token::Ident(bytes) => {
                let s = std::str::from_utf8(bytes)?;
                Ok(DxValue::String(s.to_string()))
            }
            _ => Err(DxError::InvalidSyntax {
                pos: self.tokenizer.pos(),
                msg: format!("Unexpected token in value: {:?}", token),
            }),
        }
    }

    /// Parse stream array: >a|b|c
    fn parse_stream_array(&mut self) -> Result<DxValue> {
        let mut values = Vec::new();

        loop {
            self.tokenizer.skip_whitespace();

            let token = self.tokenizer.peek_token()?;
            if matches!(token, Token::Newline | Token::Eof) {
                break;
            }

            let val = self.parse_value()?;
            values.push(val);

            self.tokenizer.skip_whitespace();
            if matches!(self.tokenizer.peek_token()?, Token::Pipe) {
                self.tokenizer.next_token()?; // consume |
            } else {
                break;
            }
        }

        Ok(DxValue::Array(DxArray::stream(values)))
    }

    /// Parse table definition and rows
    fn parse_table_definition(&mut self, name: &str) -> Result<DxValue> {
        // Read schema definition until newline
        self.tokenizer.skip_whitespace();
        let schema_line = self.tokenizer.read_until(b'\n');
        let schema_str = std::str::from_utf8(schema_line)?;

        let schema = Schema::parse_definition(name.to_string(), schema_str)?;
        self.schemas.insert(name.to_string(), schema.clone());

        // Consume newline
        if matches!(self.tokenizer.peek(), Some(b'\n')) {
            self.tokenizer.advance(1);
        }

        // Parse table rows
        let mut table = DxTable::new(schema.clone());
        let mut prev_row: Option<Vec<DxValue>> = None;

        loop {
            self.tokenizer.skip_whitespace();

            // Check if this line is still part of the table
            let token = self.tokenizer.peek_token()?;

            // End of file - we're done
            if matches!(token, Token::Eof) {
                break;
            }

            // Skip empty lines
            if matches!(token, Token::Newline) {
                self.tokenizer.next_token()?;
                continue;
            }

            // If we hit a key (identifier followed by : = or >), we're done
            if matches!(token, Token::Ident(_)) {
                let saved_pos = self.tokenizer.pos();
                self.tokenizer.next_token()?; // consume ident
                let next = self.tokenizer.peek_token()?;
                self.tokenizer.reset_to(saved_pos);

                if matches!(
                    next,
                    Token::Colon
                        | Token::Equals
                        | Token::Stream
                        | Token::Bang
                        | Token::Void
                        | Token::Caret
                        | Token::Dollar
                ) {
                    break;
                }
            }

            // Parse row based on schema
            let row = self.parse_table_row(&schema, prev_row.as_ref())?;

            if row.is_empty() {
                break;
            }

            prev_row = Some(row.clone());
            table.add_row(row).map_err(DxError::SchemaError)?;

            // Defensive: Check table row count limit
            if table.rows.len() > crate::error::MAX_TABLE_ROWS {
                return Err(DxError::table_too_large(table.rows.len()));
            }

            // Check for end of line
            self.tokenizer.skip_whitespace();
            if matches!(self.tokenizer.peek(), Some(b'\n')) {
                self.tokenizer.advance(1);
            }
        }

        Ok(DxValue::Table(table))
    }

    /// Parse a single table row based on schema
    ///
    /// # Errors
    ///
    /// Returns `DxError::UnexpectedEof` if end of input is reached before all columns are parsed.
    /// Returns `DxError::TypeMismatch` if a column value doesn't match the expected type.
    /// Returns `DxError::DittoNoPrevious` if ditto is used without a previous row.
    fn parse_table_row(
        &mut self,
        schema: &Schema,
        prev_row: Option<&Vec<DxValue>>,
    ) -> Result<Vec<DxValue>> {
        let mut row = Vec::with_capacity(schema.columns.len());

        for (col_idx, column) in schema.columns.iter().enumerate() {
            self.tokenizer.skip_whitespace();

            // Handle auto-increment: generate value without reading input
            if matches!(column.type_hint, TypeHint::AutoIncrement) {
                let counter = self.auto_counters.entry(schema.name.clone()).or_insert(1);
                row.push(DxValue::Int(*counter));
                *counter += 1;
                continue;
            }

            let token = self.tokenizer.peek_token()?;

            // Handle EOF gracefully - return partial row if we hit EOF mid-row
            if matches!(token, Token::Eof) {
                // If we haven't parsed any columns yet, return empty row to signal end
                if row.is_empty() {
                    return Ok(row);
                }
                // Otherwise, we have an incomplete row - return error with position
                return Err(DxError::UnexpectedEof(self.tokenizer.pos()));
            }

            // Handle newline - end of row (may be incomplete)
            if matches!(token, Token::Newline) {
                // If we haven't parsed any columns yet, return empty row
                if row.is_empty() {
                    return Ok(row);
                }
                // Otherwise, we have an incomplete row - return error with position
                return Err(DxError::InvalidSyntax {
                    pos: self.tokenizer.pos(),
                    msg: format!(
                        "Incomplete table row: expected {} columns, got {}",
                        schema.columns.len(),
                        row.len()
                    ),
                });
            }

            // Handle ditto (_)
            if matches!(token, Token::Ditto) {
                self.tokenizer.next_token()?;
                if let Some(prev) = prev_row {
                    if col_idx < prev.len() {
                        row.push(prev[col_idx].clone());
                    } else {
                        return Err(DxError::DittoNoPrevious(self.tokenizer.pos()));
                    }
                } else {
                    return Err(DxError::DittoNoPrevious(self.tokenizer.pos()));
                }
                continue;
            }

            // Parse value based on type hint
            let value = match column.type_hint {
                TypeHint::Int => match self.tokenizer.next_token()? {
                    Token::Eof => {
                        return Err(DxError::UnexpectedEof(self.tokenizer.pos()));
                    }
                    Token::Int(i) => DxValue::Int(i),
                    Token::Ditto => {
                        if let Some(prev) = prev_row {
                            if col_idx < prev.len() {
                                prev[col_idx].clone()
                            } else {
                                return Err(DxError::DittoNoPrevious(self.tokenizer.pos()));
                            }
                        } else {
                            return Err(DxError::DittoNoPrevious(self.tokenizer.pos()));
                        }
                    }
                    other => {
                        return Err(DxError::TypeMismatch {
                            expected: "int".to_string(),
                            actual: format!("{:?}", other),
                        });
                    }
                },
                TypeHint::Float => match self.tokenizer.next_token()? {
                    Token::Eof => {
                        return Err(DxError::UnexpectedEof(self.tokenizer.pos()));
                    }
                    Token::Float(f) => DxValue::Float(f),
                    Token::Int(i) => DxValue::Float(i as f64),
                    other => {
                        return Err(DxError::TypeMismatch {
                            expected: "float".to_string(),
                            actual: format!("{:?}", other),
                        });
                    }
                },
                TypeHint::Bool => match self.tokenizer.next_token()? {
                    Token::Eof => {
                        return Err(DxError::UnexpectedEof(self.tokenizer.pos()));
                    }
                    Token::True => DxValue::Bool(true),
                    Token::False => DxValue::Bool(false),
                    other => {
                        return Err(DxError::TypeMismatch {
                            expected: "bool".to_string(),
                            actual: format!("{:?}", other),
                        });
                    }
                },
                TypeHint::Base62 => {
                    // Parse Base62 encoded integer
                    match self.tokenizer.next_token()? {
                        Token::Eof => {
                            return Err(DxError::UnexpectedEof(self.tokenizer.pos()));
                        }
                        Token::Ident(bytes) => {
                            let s = std::str::from_utf8(bytes)?;
                            let n = decode_base62(s)?;
                            DxValue::Int(n as i64)
                        }
                        Token::Int(i) => DxValue::Int(i), // Fallback for regular numbers
                        other => {
                            return Err(DxError::TypeMismatch {
                                expected: "base62".to_string(),
                                actual: format!("{:?}", other),
                            });
                        }
                    }
                }
                TypeHint::String => {
                    // Vacuum parsing: read until next column type
                    let next_is_number = col_idx + 1 < schema.columns.len()
                        && matches!(
                            schema.columns[col_idx + 1].type_hint,
                            TypeHint::Int | TypeHint::Float | TypeHint::Base62
                        );
                    let bytes = self.tokenizer.read_string_vacuum(next_is_number);
                    let s = std::str::from_utf8(bytes)?.trim().to_string();
                    DxValue::String(s)
                }
                TypeHint::AutoIncrement => {
                    // Should not reach here (handled above)
                    unreachable!("AutoIncrement handled before loop")
                }
                TypeHint::Auto => self.parse_value()?,
            };

            row.push(value);
        }

        Ok(row)
    }
}

/// Parse DX bytes into a value
///
/// Parses the DX machine format (binary) into a structured [`DxValue`].
/// This is the primary entry point for parsing DX-formatted data.
///
/// # Example
///
/// ```rust
/// use serializer::parse;
///
/// let input = b"name:Alice\nage:30\nactive:+";
/// let value = parse(input).unwrap();
/// ```
///
/// # Errors
///
/// Returns a [`DxError`] in the following cases:
///
/// - [`DxError::InputTooLarge`] - Input exceeds `MAX_INPUT_SIZE` (100 MB).
///   This check happens before any allocation to prevent memory exhaustion.
///
/// - [`DxError::InvalidSyntax`] - Invalid syntax at a specific position:
///   - Unexpected token where a key, value, or operator was expected
///   - Invalid operator after key (expected `:`, `=`, `>`, `!`, or `?`)
///   - Unexpected character in value position
///
/// - [`DxError::UnexpectedEof`] - Input ends prematurely:
///   - EOF after `:` when a value was expected
///   - EOF after `@` when an anchor reference was expected
///   - EOF in the middle of a table row
///
/// - [`DxError::Utf8Error`] - Input contains invalid UTF-8 sequences.
///   The error includes the byte offset of the first invalid byte.
///
/// - [`DxError::UnknownAlias`] - Reference to an undefined alias (e.g., `$undefined`).
///
/// - [`DxError::UnknownAnchor`] - Reference to an undefined anchor (e.g., `@999`).
///
/// - [`DxError::TypeMismatch`] - Value doesn't match the expected type hint:
///   - Integer expected but got string
///   - Float expected but got boolean
///   - Boolean expected but got number
///
/// - [`DxError::DittoNoPrevious`] - Ditto operator (`_`) used without a previous row.
///
/// - [`DxError::RecursionLimitExceeded`] - Nesting depth exceeds `MAX_RECURSION_DEPTH` (1000).
///
/// - [`DxError::TableTooLarge`] - Table has more than `MAX_TABLE_ROWS` (10 million) rows.
///
/// - [`DxError::SchemaError`] - Invalid table schema definition.
///
/// - [`DxError::Base62Error`] - Invalid Base62 encoded value in a `%x` column.
///
/// # Example Error Handling
///
/// ```rust
/// use serializer::{parse, DxError};
///
/// let result = parse(b"key:");
/// match result {
///     Ok(value) => println!("Parsed successfully"),
///     Err(DxError::UnexpectedEof(pos)) => eprintln!("Unexpected EOF at position {}", pos),
///     Err(DxError::InvalidSyntax { pos, msg }) => eprintln!("Syntax error at {}: {}", pos, msg),
///     Err(DxError::InputTooLarge { size, max }) => eprintln!("Input {} bytes exceeds {} limit", size, max),
///     Err(e) => eprintln!("Other error: {}", e),
/// }
/// ```
///
/// [`DxError`]: crate::error::DxError
/// [`DxValue`]: crate::types::DxValue
#[must_use = "parsing result should be used"]
pub fn parse(input: &[u8]) -> Result<DxValue> {
    // Defensive: Check input size before parsing
    if input.len() > crate::error::MAX_INPUT_SIZE {
        return Err(DxError::input_too_large(input.len()));
    }

    let mut parser = Parser::new(input);
    parser.parse()
}

/// Parse DX from string
///
/// Convenience wrapper around [`parse()`] that accepts a string slice.
///
/// # Example
///
/// ```rust
/// use serializer::parser::parse_str;
///
/// let value = parse_str("name:Alice\nage:30").unwrap();
/// ```
///
/// # Errors
///
/// Returns the same errors as [`parse()`]. See that function for the complete
/// list of error conditions.
///
/// [`parse()`]: crate::parser::parse
pub fn parse_str(input: &str) -> Result<DxValue> {
    parse(input.as_bytes())
}

/// Stream parser for large files
///
/// Reads the entire contents of a reader into memory and parses it.
/// For very large files, consider using memory-mapped I/O instead.
///
/// # Example
///
/// ```rust
/// use serializer::parser::parse_stream;
/// use std::io::Cursor;
///
/// let data = Cursor::new(b"name:Test\nvalue:42");
/// let value = parse_stream(data).unwrap();
/// ```
///
/// # Errors
///
/// Returns a [`DxError`] in the following cases:
///
/// - [`DxError::Io`] - Failed to read from the input stream.
///
/// - All errors from [`parse()`] - After reading, the data is parsed using
///   the standard parser, which may return any of its error types.
///
/// [`DxError`]: crate::error::DxError
/// [`parse()`]: crate::parser::parse
pub fn parse_stream<R: std::io::Read>(reader: R) -> Result<DxValue> {
    let mut buffer = Vec::new();
    let mut reader = reader;
    reader.read_to_end(&mut buffer)?;
    parse(&buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_parse() {
        let input = b"name:Alice
age:30
active:+";

        let result = parse(input).unwrap();
        if let DxValue::Object(obj) = result {
            assert_eq!(obj.get("name"), Some(&DxValue::String("Alice".to_string())));
            assert_eq!(obj.get("age"), Some(&DxValue::Int(30)));
            assert_eq!(obj.get("active"), Some(&DxValue::Bool(true)));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_table_parse() {
        let input = b"users=id%i name%s active%b
1 Alice +
2 Bob -";

        let result = parse(input).unwrap();
        if let DxValue::Object(obj) = result {
            if let Some(DxValue::Table(table)) = obj.get("users") {
                assert_eq!(table.row_count(), 2);
                assert_eq!(table.rows[0][0], DxValue::Int(1));
                assert_eq!(table.rows[0][1], DxValue::String("Alice".to_string()));
            } else {
                panic!("Expected table");
            }
        }
    }

    #[test]
    fn test_stream_array() {
        let input = b"tags>alpha|beta|gamma";

        let result = parse(input).unwrap();
        if let DxValue::Object(obj) = result {
            if let Some(DxValue::Array(arr)) = obj.get("tags") {
                assert!(arr.is_stream);
                assert_eq!(arr.values.len(), 3);
            } else {
                panic!("Expected array");
            }
        }
    }

    #[test]
    fn test_alias() {
        let input = b"$c=context
$c.task:Mission";

        let result = parse(input).unwrap();
        if let DxValue::Object(obj) = result {
            assert_eq!(obj.get("context.task"), Some(&DxValue::String("Mission".to_string())));
        }
    }

    #[test]
    fn test_ditto() {
        let input = b"data=id%i name%s
1 Alice
_ Bob";

        let result = parse(input).unwrap();
        if let DxValue::Object(obj) = result {
            if let Some(DxValue::Table(table)) = obj.get("data") {
                assert_eq!(table.rows[1][0], DxValue::Int(1)); // Ditto copies from above
            }
        }
    }

    #[test]
    fn test_eof_in_value() {
        // Test that EOF after colon returns UnexpectedEof error
        let input = b"key:";
        let result = parse(input);
        assert!(result.is_err(), "EOF after colon should error");
        if let Err(DxError::UnexpectedEof(pos)) = result {
            assert!(pos <= input.len(), "Position should be within input bounds");
        }
    }

    #[test]
    fn test_eof_in_anchor_reference() {
        // Test that EOF after @ returns UnexpectedEof error
        let input = b"key:@";
        let result = parse(input);
        assert!(result.is_err(), "EOF after @ should error");
    }

    #[test]
    fn test_empty_input_parses() {
        // Empty input should parse to empty object
        let result = parse(b"");
        assert!(result.is_ok(), "Empty input should parse successfully");
        if let Ok(DxValue::Object(obj)) = result {
            assert!(obj.fields.is_empty(), "Empty input should produce empty object");
        }
    }

    // ==========================================================================
    // Thread Safety Compile-Time Assertions
    // ==========================================================================

    /// Compile-time assertion that DxValue implements Send
    fn _assert_dx_value_send<T: Send>() {}

    /// Compile-time assertion that DxValue implements Sync
    fn _assert_dx_value_sync<T: Sync>() {}

    #[test]
    fn test_dx_value_is_send_sync() {
        // These function calls verify at compile time that DxValue is Send + Sync
        _assert_dx_value_send::<DxValue>();
        _assert_dx_value_sync::<DxValue>();
    }

    #[test]
    fn test_parser_is_send() {
        // Parser should be Send (can be moved between threads)
        fn assert_send<T: Send>() {}
        assert_send::<Parser<'_>>();
    }

    #[test]
    fn test_parse_is_stateless() {
        // Verify that parse() can be called from multiple threads
        // by running the same input through multiple threads and
        // verifying consistent results
        use std::sync::Arc;
        use std::thread;

        let input = Arc::new(b"name:Test\nvalue:42".to_vec());
        let num_threads = 4;

        let handles: Vec<_> = (0..num_threads)
            .map(|_| {
                let input_clone = Arc::clone(&input);
                thread::spawn(move || parse(&input_clone))
            })
            .collect();

        let results: Vec<_> =
            handles.into_iter().map(|h| h.join().expect("Thread panicked")).collect();

        // All results should be identical
        let first = results[0].as_ref().expect("First parse failed");
        for (i, result) in results.iter().enumerate().skip(1) {
            let value = result.as_ref().unwrap_or_else(|_| panic!("Parse {} failed", i));
            assert_eq!(first, value, "Thread {} produced different result", i);
        }
    }
}
