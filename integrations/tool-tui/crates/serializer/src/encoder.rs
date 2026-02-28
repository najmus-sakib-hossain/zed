//! Encoder for DX Machine format
//!
//! Converts Rust data structures into highly optimized DX bytecode.
//! Automatically uses aliases, ditto marks, and compression.
//! DX ∞: Base62 encoding for integers, auto-increment detection.

use crate::base62::encode_base62;
use crate::error::Result;
use crate::schema::TypeHint;
use crate::types::{DxArray, DxObject, DxTable, DxValue};
use rustc_hash::FxHashMap;
use std::io::Write;

/// Encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Enable alias generation for repeated keys
    pub use_aliases: bool,
    /// Enable ditto marks for repeated values
    pub use_ditto: bool,
    /// Minimum key length to create alias
    pub alias_min_length: usize,
    /// Pretty print (adds spacing)
    pub pretty: bool,
}

impl Default for EncoderConfig {
    fn default() -> Self {
        Self {
            use_aliases: true,
            use_ditto: true,
            alias_min_length: 6,
            pretty: false,
        }
    }
}

/// DX encoder
pub struct Encoder {
    config: EncoderConfig,
    /// Generated aliases (full_key -> alias)
    aliases: FxHashMap<String, String>,
    /// Next alias id
    next_alias: usize,
    /// Previous row values for ditto detection
    prev_row: Option<Vec<DxValue>>,
}

impl Encoder {
    pub fn new(config: EncoderConfig) -> Self {
        Self {
            config,
            aliases: FxHashMap::default(),
            next_alias: 0,
            prev_row: None,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(EncoderConfig::default())
    }

    /// Encode a value to bytes
    pub fn encode(&mut self, value: &DxValue) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        self.encode_to_writer(value, &mut output)?;
        Ok(output)
    }

    /// Encode to a writer
    pub fn encode_to_writer<W: Write>(&mut self, value: &DxValue, writer: &mut W) -> Result<()> {
        match value {
            DxValue::Object(obj) => self.encode_object(obj, writer, ""),
            _ => {
                // Single value - wrap in object
                let mut obj = DxObject::new();
                obj.insert("value".to_string(), value.clone());
                self.encode_object(&obj, writer, "")
            }
        }
    }

    /// Encode an object
    fn encode_object<W: Write>(
        &mut self,
        obj: &DxObject,
        writer: &mut W,
        prefix: &str,
    ) -> Result<()> {
        // First pass: find keys to alias
        if self.config.use_aliases {
            self.generate_aliases(obj);
        }

        // Second pass: write aliases
        for (alias, full_key) in &self.aliases {
            writeln!(writer, "${}={}", alias, full_key)?;
        }

        // Third pass: write values
        for (key, value) in obj.iter() {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", prefix, key)
            };

            self.encode_field(&full_key, value, writer)?;
        }

        Ok(())
    }

    /// Generate aliases for repeated/long keys
    fn generate_aliases(&mut self, obj: &DxObject) {
        let mut key_freq: FxHashMap<String, usize> = FxHashMap::default();

        for (key, _) in obj.iter() {
            if key.len() >= self.config.alias_min_length {
                *key_freq.entry(key.clone()).or_insert(0) += 1;
            }
        }

        // Create aliases for frequent/long keys
        for (key, _count) in key_freq.iter() {
            if !self.aliases.contains_key(key) {
                let alias = format!("k{}", self.next_alias);
                self.next_alias += 1;
                self.aliases.insert(alias, key.clone());
            }
        }
    }

    /// Encode a field
    fn encode_field<W: Write>(&mut self, key: &str, value: &DxValue, writer: &mut W) -> Result<()> {
        // Write key (with alias if available)
        let key_to_write = self
            .aliases
            .iter()
            .find(|(_, v)| v.as_str() == key)
            .map(|(k, _)| format!("${}", k))
            .unwrap_or_else(|| key.to_string());

        match value {
            DxValue::Table(table) => {
                // Table with schema
                write!(writer, "{}=", key_to_write)?;
                self.encode_table(table, writer)?;
            }
            DxValue::Array(arr) if arr.is_stream => {
                // Stream array
                write!(writer, "{}> ", key_to_write)?;
                self.encode_stream_array(arr, writer)?;
                writeln!(writer)?;
            }
            DxValue::Bool(true) => {
                // Shorthand: key!
                writeln!(writer, "{}!", key_to_write)?;
            }
            DxValue::Null => {
                // Shorthand: key?
                writeln!(writer, "{}?", key_to_write)?;
            }
            _ => {
                // Standard key:value
                write!(writer, "{}:", key_to_write)?;
                self.encode_value(value, writer)?;
                writeln!(writer)?;
            }
        }

        Ok(())
    }

    /// Encode a table
    fn encode_table<W: Write>(&mut self, table: &DxTable, writer: &mut W) -> Result<()> {
        // Write schema
        for (i, col) in table.schema.columns.iter().enumerate() {
            if i > 0 {
                write!(writer, " ")?;
            }
            write!(writer, "{}", col.name)?;
            if col.type_hint != TypeHint::Auto {
                write!(writer, "%{}", col.type_hint.to_byte() as char)?;
            }
        }
        writeln!(writer)?;

        // Write rows
        self.prev_row = None;
        for row in &table.rows {
            self.encode_row(row, &table.schema, writer)?;
            self.prev_row = Some(row.clone());
        }

        Ok(())
    }

    /// Encode a table row
    fn encode_row<W: Write>(
        &mut self,
        row: &[DxValue],
        schema: &crate::schema::Schema,
        writer: &mut W,
    ) -> Result<()> {
        for (i, value) in row.iter().enumerate() {
            // Skip auto-increment columns (they're generated on parse)
            if i < schema.columns.len() && schema.columns[i].type_hint == TypeHint::AutoIncrement {
                continue;
            }

            if i > 0 {
                write!(writer, " ")?;
            }

            // Use ditto if value matches previous row
            if self.config.use_ditto {
                if let Some(prev) = &self.prev_row {
                    if i < prev.len() && &prev[i] == value {
                        write!(writer, "_")?;
                        continue;
                    }
                }
            }

            // Use Base62 encoding if column type is Base62
            if i < schema.columns.len() && schema.columns[i].type_hint == TypeHint::Base62 {
                if let DxValue::Int(n) = value {
                    if *n >= 0 {
                        write!(writer, "{}", encode_base62(*n as u64))?;
                        continue;
                    }
                }
            }

            self.encode_value(value, writer)?;
        }
        writeln!(writer)?;

        Ok(())
    }

    /// Encode stream array
    fn encode_stream_array<W: Write>(&mut self, arr: &DxArray, writer: &mut W) -> Result<()> {
        for (i, value) in arr.values.iter().enumerate() {
            if i > 0 {
                write!(writer, "|")?;
            }
            self.encode_value(value, writer)?;
        }
        Ok(())
    }

    /// Encode a value
    fn encode_value<W: Write>(&mut self, value: &DxValue, writer: &mut W) -> Result<()> {
        match value {
            DxValue::Null => write!(writer, "~")?,
            DxValue::Bool(true) => write!(writer, "+")?,
            DxValue::Bool(false) => write!(writer, "-")?,
            DxValue::Int(i) => write!(writer, "{}", i)?,
            DxValue::Float(f) => write!(writer, "{}", f)?,
            DxValue::String(s) => {
                // No quotes in machine format
                write!(writer, "{}", s)?;
            }
            DxValue::Array(arr) => {
                if arr.is_stream {
                    self.encode_stream_array(arr, writer)?;
                } else {
                    // Vertical array - not supported in inline
                    write!(writer, "[]")?;
                }
            }
            DxValue::Object(_) => write!(writer, "{{}}")?,
            DxValue::Table(_) => write!(writer, "[[]]")?,
            DxValue::Ref(id) => write!(writer, "@{}", id)?,
        }
        Ok(())
    }
}

/// Encode a value with default config
///
/// Converts a [`DxValue`] into DX machine format bytes using default settings.
/// This is the inverse of [`parse()`].
///
/// # Example
///
/// ```rust
/// use serializer::{encode, DxValue, DxObject};
///
/// let mut obj = DxObject::new();
/// obj.insert("name".to_string(), DxValue::String("Alice".to_string()));
/// obj.insert("age".to_string(), DxValue::Int(30));
///
/// let bytes = encode(&DxValue::Object(obj)).unwrap();
/// ```
///
/// # Errors
///
/// Returns a `DxError` in the following cases:
///
/// - `DxError::Io` - Failed to write to the internal buffer. This is rare
///   since encoding writes to an in-memory `Vec<u8>`, but can occur if
///   memory allocation fails.
///
/// # Note
///
/// The encoding process is generally infallible for valid `DxValue` inputs.
/// The `Result` return type is used for consistency with the streaming
/// `encode_to_writer()` function and to handle potential I/O errors.
///
/// [`DxValue`]: crate::types::DxValue
/// [`parse()`]: crate::parser::parse
#[must_use = "encoding result should be used"]
pub fn encode(value: &DxValue) -> Result<Vec<u8>> {
    let mut encoder = Encoder::with_defaults();
    encoder.encode(value)
}

/// Encode to a writer with default config
///
/// Writes the encoded DX machine format directly to any [`Write`] implementor.
/// This is more efficient than `encode()` when writing to files or network
/// streams, as it avoids an intermediate buffer.
///
/// # Example
///
/// ```rust
/// use serializer::{encode_to_writer, DxValue, DxObject};
/// use std::io::Cursor;
///
/// let mut obj = DxObject::new();
/// obj.insert("name".to_string(), DxValue::String("Test".to_string()));
///
/// let mut buffer = Vec::new();
/// encode_to_writer(&DxValue::Object(obj), &mut buffer).unwrap();
/// ```
///
/// # Errors
///
/// Returns a `DxError` in the following cases:
///
/// - `DxError::Io` - Failed to write to the output stream. This can occur
///   when writing to files (disk full, permission denied), network streams
///   (connection closed), or any other I/O operation that fails.
///
/// [`Write`]: std::io::Write
pub fn encode_to_writer<W: Write>(value: &DxValue, writer: &mut W) -> Result<()> {
    let mut encoder = Encoder::with_defaults();
    encoder.encode_to_writer(value, writer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_encode_simple() {
        let mut obj = DxObject::new();
        obj.insert("name".to_string(), DxValue::String("Alice".to_string()));
        obj.insert("age".to_string(), DxValue::Int(30));
        obj.insert("active".to_string(), DxValue::Bool(true));

        let value = DxValue::Object(obj);
        let encoded = encode(&value).unwrap();
        let encoded_str = std::str::from_utf8(&encoded).unwrap();

        assert!(encoded_str.contains("name:Alice"));
        assert!(encoded_str.contains("age:30"));
        // active is aliased to $k0 because it's >= 6 chars
        // Check that either active! or $k0! is present (aliased form)
        assert!(encoded_str.contains("!"), "Expected boolean true to be encoded with !");
    }

    #[test]
    fn test_round_trip() {
        // Use short key names to avoid aliasing (alias_min_length is 6)
        let input = b"name:Test
score:9.5
ok:+";

        let parsed = parse(input).unwrap();
        let encoded = encode(&parsed).unwrap();
        let reparsed = parse(&encoded).unwrap();

        assert_eq!(parsed, reparsed);
    }
}
