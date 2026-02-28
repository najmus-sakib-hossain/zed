//! Core data types for LLM and Human format serialization
//!
//! This module defines the value types for the DX **LLM format** (text, token-efficient).
//! The LLM format is optimized for minimal token usage in LLM context windows,
//! achieving 73%+ token savings compared to JSON.
//!
//! # Format Comparison
//!
//! DX provides two serialization formats:
//!
//! | Format | Type | Use Case | Performance |
//! |--------|------|----------|-------------|
//! | **Machine** | [`DxValue`](crate::DxValue) | Binary, zero-copy, runtime | RKYV format |
//! | **LLM** | [`DxLlmValue`] | Text, token-efficient, LLM context | 73%+ token savings |
//!
//! # When to Use This Module
//!
//! Use [`DxLlmValue`] and the LLM format when:
//! - You're preparing data for LLM context windows
//! - Token efficiency is critical (API costs, context limits)
//! - You need human-readable output
//! - You're converting to/from JSON, YAML, or TOML
//!
//! Use [`DxValue`](crate::DxValue) when:
//! - You need binary serialization with RKYV
//! - You're working with binary data or network protocols
//! - You want zero-copy deserialization
//!
//! # Document Structure
//!
//! The LLM format uses a document-based structure:
//!
//! - [`DxDocument`]: Top-level container with context, refs, and sections
//! - [`DxSection`]: A data section with schema and rows (tabular data)
//! - [`DxLlmValue`]: Individual values within sections
//!
//! # Thread Safety
//!
//! All types in this module implement `Send + Sync` and can be safely shared
//! between threads:
//!
//! - [`DxDocument`]: The main document type, safe for concurrent read access
//! - [`DxSection`]: Section data, safe for concurrent read access
//! - [`DxLlmValue`]: Value enum, all variants are thread-safe
//!
//! For mutation, clone the document or use appropriate synchronization primitives
//! like `Arc<Mutex<DxDocument>>` or `Arc<RwLock<DxDocument>>`.

use indexmap::IndexMap;
use std::fmt;

/// Top-level document container for the DX LLM format.
///
/// A `DxDocument` represents a complete DX document in the LLM format,
/// containing context configuration, reference definitions, and data sections.
///
/// # Structure
///
/// ```text
/// #c context_key=value          // Context section
/// #: ref_key=ref_value          // Reference definitions
/// #d(id,name)[                  // Data section 'd' with schema
///   1,Alice
///   2,Bob
/// ]
/// ```
///
/// # Examples
///
/// ## Creating a Document
///
/// ```rust
/// use serializer::llm::{DxDocument, DxSection, DxLlmValue};
///
/// let mut doc = DxDocument::new();
///
/// // Add context
/// doc.context.insert("version".to_string(), DxLlmValue::Str("1.0".to_string()));
///
/// // Add a reference
/// doc.refs.insert("company".to_string(), "Acme Corp".to_string());
///
/// // Add a data section
/// let mut section = DxSection::new(vec!["id".to_string(), "name".to_string()]);
/// section.add_row(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("Alice".to_string())]).unwrap();
/// doc.sections.insert('d', section);
/// ```
///
/// # Thread Safety
///
/// `DxDocument` implements `Send + Sync` and can be safely shared between threads
/// for concurrent read access. For mutation, use appropriate synchronization.
///
/// # See Also
///
/// - [`DxSection`] - Data section with schema and rows
/// - [`DxLlmValue`] - Value type for section data
#[derive(Debug, Clone, PartialEq)]
pub struct DxDocument {
    /// Context/config section (`#c`).
    ///
    /// Contains configuration key-value pairs that apply to the entire document.
    pub context: IndexMap<String, DxLlmValue>,
    /// Reference definitions (`#:`).
    ///
    /// Defines reusable string values that can be referenced with `^key` syntax.
    /// This enables deduplication of repeated values for token efficiency.
    pub refs: IndexMap<String, String>,
    /// Data sections (`#<letter>`).
    ///
    /// Each section is identified by a single character (e.g., 'd' for `#d`).
    /// Sections contain tabular data with a schema and rows.
    pub sections: IndexMap<char, DxSection>,
    /// Section names mapping (char ID -> full name like "dependencies")
    ///
    /// Maps single-character section IDs to their full names for human-readable output.
    pub section_names: IndexMap<char, String>,
    /// Entry order tracking
    ///
    /// Tracks the order of all entries (context keys and section IDs) as they appear
    /// in the original document. Each entry is either a context key or a section ID.
    pub entry_order: Vec<EntryRef>,
}

/// Reference to an entry in the document (context or section)
#[derive(Debug, Clone, PartialEq)]
pub enum EntryRef {
    /// Reference to a context entry by key
    Context(String),
    /// Reference to a section by its char ID
    Section(char),
}

impl DxDocument {
    /// Create a new empty document
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: IndexMap::new(),
            refs: IndexMap::new(),
            sections: IndexMap::new(),
            section_names: IndexMap::new(),
            entry_order: Vec::new(),
        }
    }

    /// Check if the document is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.context.is_empty() && self.refs.is_empty() && self.sections.is_empty()
    }
}

impl Default for DxDocument {
    fn default() -> Self {
        Self::new()
    }
}

/// A data section with schema-defined columns and rows.
///
/// `DxSection` represents tabular data in the DX LLM format. Each section
/// has a schema (column names) and rows of [`DxLlmValue`] data.
///
/// # LLM Format Syntax
///
/// ```text
/// #d(id,name,active)[    // Section 'd' with schema: id, name, active
///   1,Alice,+            // Row 1
///   2,Bob,-              // Row 2
/// ]
/// ```
///
/// # Examples
///
/// ## Creating a Section
///
/// ```rust
/// use serializer::llm::{DxSection, DxLlmValue};
///
/// let mut section = DxSection::new(vec![
///     "id".to_string(),
///     "name".to_string(),
///     "active".to_string(),
/// ]);
///
/// // Add rows (must match schema length)
/// section.add_row(vec![
///     DxLlmValue::Num(1.0),
///     DxLlmValue::Str("Alice".to_string()),
///     DxLlmValue::Bool(true),
/// ]).unwrap();
///
/// assert_eq!(section.row_count(), 1);
/// assert_eq!(section.column_count(), 3);
/// ```
///
/// ## Error Handling
///
/// ```rust
/// use serializer::llm::{DxSection, DxLlmValue};
///
/// let mut section = DxSection::new(vec!["a".to_string(), "b".to_string()]);
///
/// // Wrong number of columns returns an error
/// let result = section.add_row(vec![DxLlmValue::Num(1.0)]);
/// assert!(result.is_err());
/// ```
///
/// # Thread Safety
///
/// `DxSection` implements `Send + Sync` and can be safely shared between threads.
///
/// # See Also
///
/// - [`DxDocument`] - Parent container for sections
/// - [`DxLlmValue`] - Value type for row data
#[derive(Debug, Clone, PartialEq)]
pub struct DxSection {
    /// Column names from the schema.
    ///
    /// Defines the structure of each row. All rows must have exactly
    /// `schema.len()` values.
    pub schema: Vec<String>,
    /// Row data.
    ///
    /// Each row is a vector of [`DxLlmValue`] with length matching the schema.
    pub rows: Vec<Vec<DxLlmValue>>,
}

impl DxSection {
    /// Create a new section with the given schema
    #[must_use]
    pub fn new(schema: Vec<String>) -> Self {
        Self {
            schema,
            rows: Vec::new(),
        }
    }

    /// Add a row to the section
    ///
    /// # Errors
    ///
    /// Returns an error if the row length doesn't match the schema length.
    pub fn add_row(&mut self, row: Vec<DxLlmValue>) -> Result<(), String> {
        if row.len() != self.schema.len() {
            return Err(format!(
                "Row length {} doesn't match schema length {}",
                row.len(),
                self.schema.len()
            ));
        }
        self.rows.push(row);
        Ok(())
    }

    /// Get the number of rows
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Get the number of columns
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.schema.len()
    }
}

/// Value type for the DX **LLM format** (text, token-efficient).
///
/// `DxLlmValue` represents all possible values in the DX LLM format, optimized
/// for minimal token usage in LLM context windows. This format achieves 73%+
/// token savings compared to JSON.
///
/// # Relationship to DxValue
///
/// DX provides two value types for different use cases:
///
/// - **[`DxValue`](crate::DxValue)**: For the binary machine format. Use when you need
///   maximum performance, zero-copy parsing, or runtime data structures.
///
/// - **`DxLlmValue`** (this type): For the text LLM format. Use when preparing
///   data for LLM context windows or when token efficiency matters.
///
/// ## Key Differences
///
/// | Aspect | `DxValue` | `DxLlmValue` |
/// |--------|-----------|--------------|
/// | Format | Binary | Text (Dx Serializer) |
/// | Numbers | Separate `Int`/`Float` | Single `Num` |
/// | Structured data | `Object`, `Table` | `Obj` variant or `DxSection` |
/// | References | `Ref(usize)` | `Ref(String)` |
/// | Optimization | Speed | Token count |
///
/// The type differences reflect the different use cases:
/// - LLMs don't distinguish integer vs float, so `Num` is sufficient
/// - LLM format uses string-based references for readability
/// - Inline objects use `Obj` variant, tabular data uses `DxSection`
///
/// # When to Use DxLlmValue
///
/// Choose `DxLlmValue` when:
/// - **Token efficiency matters**: 73%+ savings vs JSON reduces API costs
/// - **LLM context limits**: Fit more data in limited context windows
/// - **Human readability**: Text format is easier to inspect and debug
/// - **Format conversion**: Converting to/from JSON, YAML, TOML
///
/// # Examples
///
/// ## Creating Values
///
/// ```rust
/// use serializer::llm::DxLlmValue;
/// use std::collections::HashMap;
///
/// // Primitive values
/// let null = DxLlmValue::Null;
/// let boolean = DxLlmValue::Bool(true);
/// let number = DxLlmValue::Num(42.0);
/// let string = DxLlmValue::Str("hello".to_string());
///
/// // Object value
/// let mut fields = HashMap::new();
/// fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
/// fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
/// let obj = DxLlmValue::Obj(fields);
/// ```
///
/// ## Using From Traits
///
/// ```rust
/// use serializer::llm::DxLlmValue;
///
/// // Convenient conversions
/// let from_str: DxLlmValue = "hello".into();
/// let from_string: DxLlmValue = String::from("world").into();
/// let from_int: DxLlmValue = 42i64.into();
/// let from_float: DxLlmValue = 3.14f64.into();
/// let from_bool: DxLlmValue = true.into();
/// ```
///
/// ## Working with Arrays
///
/// ```rust
/// use serializer::llm::DxLlmValue;
///
/// let arr = DxLlmValue::Arr(vec![
///     DxLlmValue::Num(1.0),
///     DxLlmValue::Num(2.0),
///     DxLlmValue::Num(3.0),
/// ]);
///
/// if let DxLlmValue::Arr(values) = &arr {
///     assert_eq!(values.len(), 3);
/// }
/// ```
///
/// ## Type Inspection
///
/// ```rust
/// use serializer::llm::DxLlmValue;
///
/// let value = DxLlmValue::Num(42.0);
/// assert_eq!(value.type_name(), "number");
/// assert_eq!(value.as_num(), Some(42.0));
/// assert!(value.as_str().is_none());
/// ```
///
/// ## Display Formatting
///
/// ```rust
/// use serializer::llm::DxLlmValue;
///
/// // Values format nicely for display
/// assert_eq!(format!("{}", DxLlmValue::Num(42.0)), "42");
/// assert_eq!(format!("{}", DxLlmValue::Bool(true)), "true");
/// assert_eq!(format!("{}", DxLlmValue::Null), "null");
/// assert_eq!(format!("{}", DxLlmValue::Ref("A".to_string())), "^A");
/// ```
///
/// # Thread Safety
///
/// `DxLlmValue` implements `Send + Sync` and can be safely shared between threads.
/// This is verified at compile time via static assertions.
///
/// # See Also
///
/// - [`DxValue`](crate::DxValue) - Value type for the binary machine format
/// - [`DxDocument`] - Top-level document container for LLM format
/// - [`DxSection`] - Section with schema and rows for tabular data
#[derive(Debug, Clone, PartialEq)]
pub enum DxLlmValue {
    /// String value.
    ///
    /// In LLM format, strings are represented without quotes when possible,
    /// contributing to token efficiency.
    Str(String),
    /// Numeric value (integer or float).
    ///
    /// Unlike [`DxValue`](crate::DxValue) which has separate `Int` and `Float`
    /// variants, `DxLlmValue` uses a single `Num` variant because LLMs don't
    /// distinguish between integer and floating-point numbers.
    ///
    /// Integers are stored as `f64` but display without decimal points when
    /// the fractional part is zero (e.g., `42` not `42.0`).
    Num(f64),
    /// Boolean value.
    ///
    /// In LLM format: `true` or `false`
    /// In Human format: `true` or `false`
    Bool(bool),
    /// Null value.
    ///
    /// In LLM format: `null`
    /// In Human format: `null`
    Null,
    /// Array value.
    ///
    /// In LLM format: `*a,b,c` (inline array syntax)
    Arr(Vec<DxLlmValue>),
    /// Object value with key-value pairs.
    ///
    /// In LLM format: `name[key=value,key2=value2]` (inline object syntax)
    ///
    /// Objects are stored as an IndexMap for efficient key lookup while
    /// maintaining insertion order and type safety. This is preferred over encoding objects
    /// as strings, which would lose type information.
    Obj(IndexMap<String, DxLlmValue>),
    /// Reference pointer to a defined ref.
    ///
    /// In LLM format: `^key` references a value defined in the `#:` refs section.
    /// This enables deduplication of repeated values, further reducing token count.
    ///
    /// Unlike [`DxValue::Ref`](crate::DxValue::Ref) which uses numeric indices,
    /// `DxLlmValue::Ref` uses string keys for human readability.
    Ref(String),
}

impl DxLlmValue {
    /// Check if this value is null
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, DxLlmValue::Null)
    }

    /// Get the type name for error messages
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            DxLlmValue::Str(_) => "string",
            DxLlmValue::Num(_) => "number",
            DxLlmValue::Bool(_) => "bool",
            DxLlmValue::Null => "null",
            DxLlmValue::Arr(_) => "array",
            DxLlmValue::Obj(_) => "object",
            DxLlmValue::Ref(_) => "ref",
        }
    }

    /// Try to get as string
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DxLlmValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as number
    #[must_use]
    pub fn as_num(&self) -> Option<f64> {
        match self {
            DxLlmValue::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to get as bool
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DxLlmValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Try to get as array
    #[must_use]
    pub fn as_arr(&self) -> Option<&Vec<DxLlmValue>> {
        match self {
            DxLlmValue::Arr(arr) => Some(arr),
            _ => None,
        }
    }

    /// Try to get as object
    #[must_use]
    pub fn as_obj(&self) -> Option<&IndexMap<String, DxLlmValue>> {
        match self {
            DxLlmValue::Obj(obj) => Some(obj),
            _ => None,
        }
    }

    /// Try to get as reference key
    #[must_use]
    pub fn as_ref(&self) -> Option<&str> {
        match self {
            DxLlmValue::Ref(key) => Some(key),
            _ => None,
        }
    }
}

impl fmt::Display for DxLlmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DxLlmValue::Str(s) => write!(f, "{}", s),
            DxLlmValue::Num(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            DxLlmValue::Bool(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            DxLlmValue::Null => write!(f, "null"),
            DxLlmValue::Arr(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            DxLlmValue::Obj(obj) => {
                write!(f, "[")?;
                for (i, (k, v)) in obj.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}={}", k, v)?;
                }
                write!(f, "]")
            }
            DxLlmValue::Ref(key) => write!(f, "^{}", key),
        }
    }
}

impl From<&str> for DxLlmValue {
    fn from(s: &str) -> Self {
        DxLlmValue::Str(s.to_string())
    }
}

impl From<String> for DxLlmValue {
    fn from(s: String) -> Self {
        DxLlmValue::Str(s)
    }
}

impl From<f64> for DxLlmValue {
    fn from(n: f64) -> Self {
        DxLlmValue::Num(n)
    }
}

impl From<i64> for DxLlmValue {
    fn from(n: i64) -> Self {
        DxLlmValue::Num(n as f64)
    }
}

impl From<bool> for DxLlmValue {
    fn from(b: bool) -> Self {
        DxLlmValue::Bool(b)
    }
}

// =============================================================================
// Thread Safety Compile-Time Assertions
// =============================================================================

// These static assertions verify at compile time that our types are thread-safe.
// If any of these types stop implementing Send or Sync, compilation will fail.

/// Compile-time assertion that a type implements Send
const fn _assert_send<T: Send>() {}

/// Compile-time assertion that a type implements Sync
const fn _assert_sync<T: Sync>() {}

// Verify DxDocument is Send + Sync
const _: () = _assert_send::<DxDocument>();
const _: () = _assert_sync::<DxDocument>();

// Verify DxSection is Send + Sync
const _: () = _assert_send::<DxSection>();
const _: () = _assert_sync::<DxSection>();

// Verify DxLlmValue is Send + Sync
const _: () = _assert_send::<DxLlmValue>();
const _: () = _assert_sync::<DxLlmValue>();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dx_document_new() {
        let doc = DxDocument::new();
        assert!(doc.is_empty());
        assert!(doc.context.is_empty());
        assert!(doc.refs.is_empty());
        assert!(doc.sections.is_empty());
    }

    #[test]
    fn test_dx_section_add_row() {
        let mut section = DxSection::new(vec!["id".to_string(), "name".to_string()]);

        // Valid row
        let result =
            section.add_row(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("Test".to_string())]);
        assert!(result.is_ok());
        assert_eq!(section.row_count(), 1);

        // Invalid row (wrong length)
        let result = section.add_row(vec![DxLlmValue::Num(2.0)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_dx_llm_value_type_name() {
        assert_eq!(DxLlmValue::Str("test".to_string()).type_name(), "string");
        assert_eq!(DxLlmValue::Num(42.0).type_name(), "number");
        assert_eq!(DxLlmValue::Bool(true).type_name(), "bool");
        assert_eq!(DxLlmValue::Null.type_name(), "null");
        assert_eq!(DxLlmValue::Arr(vec![]).type_name(), "array");
        assert_eq!(DxLlmValue::Obj(IndexMap::new()).type_name(), "object");
        assert_eq!(DxLlmValue::Ref("key".to_string()).type_name(), "ref");
    }

    #[test]
    #[allow(clippy::approx_constant)] // Using 3.14 intentionally for test data
    fn test_dx_llm_value_display() {
        assert_eq!(format!("{}", DxLlmValue::Str("hello".to_string())), "hello");
        assert_eq!(format!("{}", DxLlmValue::Num(42.0)), "42");
        assert_eq!(format!("{}", DxLlmValue::Num(3.14)), "3.14");
        assert_eq!(format!("{}", DxLlmValue::Bool(true)), "true");
        assert_eq!(format!("{}", DxLlmValue::Bool(false)), "false");
        assert_eq!(format!("{}", DxLlmValue::Null), "null");
        assert_eq!(format!("{}", DxLlmValue::Ref("A".to_string())), "^A");
    }

    #[test]
    fn test_dx_llm_value_obj() {
        let mut fields = IndexMap::new();
        fields.insert("host".to_string(), DxLlmValue::Str("localhost".to_string()));
        fields.insert("port".to_string(), DxLlmValue::Num(8080.0));
        let obj = DxLlmValue::Obj(fields);

        assert_eq!(obj.type_name(), "object");
        assert!(obj.as_obj().is_some());
        let obj_map = obj.as_obj().unwrap();
        assert_eq!(obj_map.len(), 2);
        assert_eq!(obj_map.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(obj_map.get("port").unwrap().as_num(), Some(8080.0));
    }

    // Note: Send+Sync assertions are now compile-time checks at module level,
    // not runtime tests. See the const assertions above the test module.
}
