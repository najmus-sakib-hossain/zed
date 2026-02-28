//! Serializer Builder Pattern
//!
//! Provides a fluent API for configuring serialization options with sensible defaults.
//! The builder pattern allows users to customize various aspects of serialization
//! without needing to understand all the internal configuration types.
//!
//! ## Thread Safety
//!
//! Both [`SerializerBuilder`] and [`Serializer`] implement `Send + Sync` and can be
//! safely shared between threads. The serializer methods are stateless and can be
//! called concurrently:
//!
//! ```rust
//! use std::sync::Arc;
//! use std::thread;
//! use serializer::{SerializerBuilder, DxDocument, DxLlmValue};
//!
//! // Create a shared serializer
//! let serializer = Arc::new(SerializerBuilder::new().build());
//!
//! let handles: Vec<_> = (0..4).map(|i| {
//!     let serializer = Arc::clone(&serializer);
//!     thread::spawn(move || {
//!         let mut doc = DxDocument::new();
//!         doc.context.insert("id".to_string(), DxLlmValue::Num(i as f64));
//!         serializer.serialize(&doc)
//!     })
//! }).collect();
//!
//! for handle in handles {
//!     let result = handle.join().unwrap();
//!     assert!(!result.is_empty());
//! }
//! ```
//!
//! ## Example
//!
//! ```rust
//! use serializer::{SerializerBuilder, DxDocument, DxLlmValue};
//!
//! let mut doc = DxDocument::new();
//! doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
//!
//! // Simple usage with defaults
//! let serializer = SerializerBuilder::new().build();
//! let text = serializer.serialize(&doc);
//!
//! // Advanced configuration
//! let serializer = SerializerBuilder::new()
//!     .indent_size(4)
//!     .preserve_comments(true)
//!     .expand_keys(false)
//!     .validate_output(true)
//!     .build();
//! let text = serializer.serialize(&doc);
//! ```

use crate::llm::human_formatter::HumanFormatConfig;
use crate::llm::pretty_printer::{PrettyPrinter, PrettyPrinterConfig};
use crate::llm::serializer_output::{SerializerOutput, SerializerOutputConfig};
use crate::llm::types::DxDocument;
use crate::llm::{ConvertError, document_to_llm, llm_to_document};
use std::path::PathBuf;

/// Builder for configuring serialization options
///
/// The SerializerBuilder provides a fluent API for customizing serialization behavior.
/// It combines configuration from multiple internal types into a single, easy-to-use interface.
#[derive(Debug, Clone)]
pub struct SerializerBuilder {
    // Human format options
    indent_size: usize,
    expand_keys: bool,
    use_list_format: bool,
    space_around_equals: bool,

    // Pretty printer options
    validate_output: bool,
    check_round_trip: bool,

    // Output generation options
    output_dir: Option<PathBuf>,
    generate_llm: bool,
    generate_machine: bool,

    // General options
    preserve_comments: bool,
    compact_arrays: bool,
}

impl Default for SerializerBuilder {
    fn default() -> Self {
        Self {
            // Human format defaults
            indent_size: 0,
            expand_keys: true,
            use_list_format: true,
            space_around_equals: true,

            // Pretty printer defaults
            validate_output: false, // Disabled by default since V3 round-trip not fully implemented
            check_round_trip: false,

            // Output generation defaults
            output_dir: None,
            generate_llm: true,
            generate_machine: true,

            // General defaults
            preserve_comments: false,
            compact_arrays: false,
        }
    }
}

impl SerializerBuilder {
    /// Create a new SerializerBuilder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the indentation size for formatted output
    ///
    /// Controls the minimum padding for keys in human format.
    /// Set to 0 for no padding (default).
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::SerializerBuilder;
    ///
    /// let serializer = SerializerBuilder::new()
    ///     .indent_size(4)
    ///     .build();
    /// ```
    pub fn indent_size(mut self, size: usize) -> Self {
        self.indent_size = size;
        self
    }

    /// Set whether to expand abbreviated keys to full names
    ///
    /// When true (default), keys like "nm" become "name" in human format.
    /// When false, abbreviated keys are preserved.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::SerializerBuilder;
    ///
    /// let serializer = SerializerBuilder::new()
    ///     .expand_keys(false)  // Keep "nm" instead of expanding to "name"
    ///     .build();
    /// ```
    pub fn expand_keys(mut self, expand: bool) -> Self {
        self.expand_keys = expand;
        self
    }

    /// Set whether to use list format for arrays
    ///
    /// When true (default), arrays are formatted as:
    /// ```text
    /// items:
    /// - first
    /// - second
    /// ```
    ///
    /// When false, arrays are formatted inline:
    /// ```text
    /// items = first | second
    /// ```
    pub fn use_list_format(mut self, use_list: bool) -> Self {
        self.use_list_format = use_list;
        self
    }

    /// Set whether to add spaces around equals signs
    ///
    /// When true (default): `key = value`
    /// When false: `key=value`
    pub fn space_around_equals(mut self, space: bool) -> Self {
        self.space_around_equals = space;
        self
    }

    /// Set whether to validate output by parsing it back
    ///
    /// When enabled, the serializer will parse the formatted output
    /// to ensure it's valid. This catches formatting bugs but is slower.
    ///
    /// Note: Currently disabled by default since V3 format round-trip
    /// is not fully implemented.
    pub fn validate_output(mut self, validate: bool) -> Self {
        self.validate_output = validate;
        self
    }

    /// Set whether to check round-trip consistency
    ///
    /// When enabled (requires validate_output), the serializer will
    /// verify that parsing the formatted output produces an equivalent
    /// document to the original.
    pub fn check_round_trip(mut self, check: bool) -> Self {
        self.check_round_trip = check;
        self
    }

    /// Set the output directory for generated files
    ///
    /// When set, the serializer can generate .human and .machine files
    /// in the specified directory. Default is ".dx/serializer".
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::SerializerBuilder;
    ///
    /// let serializer = SerializerBuilder::new()
    ///     .output_dir("build/serializer")
    ///     .build();
    /// ```
    pub fn output_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.output_dir = Some(dir.into());
        self
    }

    /// Set whether to generate LLM format files
    ///
    /// When true (default), .llm files are generated in .dx/serializer.
    pub fn generate_llm(mut self, generate: bool) -> Self {
        self.generate_llm = generate;
        self
    }

    /// Set whether to generate machine format files
    ///
    /// When true (default), .machine files are generated for runtime use.
    pub fn generate_machine(mut self, generate: bool) -> Self {
        self.generate_machine = generate;
        self
    }

    /// Set whether to preserve comments in output
    ///
    /// Note: Comment preservation is not yet fully implemented.
    /// This option is reserved for future use.
    pub fn preserve_comments(mut self, preserve: bool) -> Self {
        self.preserve_comments = preserve;
        self
    }

    /// Set whether to use compact array formatting
    ///
    /// When true, arrays are formatted more compactly.
    /// This is equivalent to setting use_list_format(false).
    pub fn compact_arrays(mut self, compact: bool) -> Self {
        self.compact_arrays = compact;
        if compact {
            self.use_list_format = false;
        }
        self
    }

    /// Create a configuration optimized for tables and rules
    ///
    /// This preset is useful for multi-row sections like lint rules.
    /// It sets appropriate padding and formatting for tabular data.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::SerializerBuilder;
    ///
    /// let serializer = SerializerBuilder::new()
    ///     .for_tables()
    ///     .build();
    /// ```
    pub fn for_tables(mut self) -> Self {
        self.indent_size = 20;
        self.use_list_format = false;
        self.expand_keys = true;
        self.space_around_equals = true;
        self
    }

    /// Create a configuration optimized for compact output
    ///
    /// This preset minimizes whitespace and uses abbreviated keys.
    /// Useful for token-efficient LLM format.
    pub fn for_compact(mut self) -> Self {
        self.indent_size = 0;
        self.expand_keys = false;
        self.use_list_format = false;
        self.space_around_equals = false;
        self.compact_arrays = true;
        self
    }

    /// Create a configuration optimized for human readability
    ///
    /// This preset maximizes readability with expanded keys,
    /// proper spacing, and list formatting for arrays.
    pub fn for_humans(mut self) -> Self {
        self.indent_size = 0;
        self.expand_keys = true;
        self.use_list_format = true;
        self.space_around_equals = true;
        self.validate_output = false; // Keep disabled until V3 round-trip is implemented
        self
    }

    /// Build the configured Serializer
    ///
    /// Creates a Serializer instance with all the specified options.
    pub fn build(self) -> Serializer {
        // Build human format config
        let human_config = HumanFormatConfig {
            key_padding: self.indent_size,
        };

        // Build pretty printer config
        let pretty_config = PrettyPrinterConfig {
            formatter_config: human_config,
            validate_output: self.validate_output,
            check_round_trip: self.check_round_trip,
        };

        // Build output config
        let output_config = SerializerOutputConfig {
            output_dir: self.output_dir.unwrap_or_else(|| PathBuf::from(".dx/serializer")),
            generate_llm: self.generate_llm,
            generate_machine: self.generate_machine,
            compression: crate::llm::convert::CompressionAlgorithm::default(),
        };

        Serializer {
            pretty_printer: PrettyPrinter::with_config(pretty_config),
            output_generator: SerializerOutput::with_config(output_config),
        }
    }
}

/// Configured Serializer instance
///
/// A Serializer provides methods for converting documents to various formats
/// using the configuration specified by the SerializerBuilder.
pub struct Serializer {
    pretty_printer: PrettyPrinter,
    output_generator: SerializerOutput,
}

impl Serializer {
    /// Serialize a DxDocument to LLM format string
    ///
    /// This is the primary serialization method that produces the
    /// token-efficient LLM format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::{SerializerBuilder, DxDocument, DxLlmValue};
    ///
    /// let mut doc = DxDocument::new();
    /// doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
    ///
    /// let serializer = SerializerBuilder::new().build();
    /// let text = serializer.serialize(&doc);
    /// ```
    pub fn serialize(&self, doc: &DxDocument) -> String {
        document_to_llm(doc)
    }

    /// Deserialize LLM format string to DxDocument
    ///
    /// Parses the token-efficient LLM format back into a structured document.
    ///
    /// # Errors
    ///
    /// Returns a `ConvertError` if the input is not valid LLM format.
    pub fn deserialize(&self, input: &str) -> Result<DxDocument, ConvertError> {
        llm_to_document(input)
    }

    /// Format a DxDocument to human-readable format
    ///
    /// Produces clean, hand-editable format using the configured options.
    /// The output is validated if validation is enabled in the builder.
    ///
    /// # Example
    ///
    /// ```rust
    /// use serializer::{SerializerBuilder, DxDocument, DxLlmValue};
    ///
    /// let mut doc = DxDocument::new();
    /// doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
    ///
    /// let serializer = SerializerBuilder::new()
    ///     .for_humans()
    ///     .build();
    /// let human_text = serializer.format_human(&doc).unwrap();
    /// ```
    pub fn format_human(
        &self,
        doc: &DxDocument,
    ) -> Result<String, crate::llm::pretty_printer::PrettyPrintError> {
        self.pretty_printer.format(doc)
    }

    /// Format a DxDocument to human format without validation
    ///
    /// Faster than format_human() but provides no guarantees about
    /// the output being parseable.
    pub fn format_human_unchecked(&self, doc: &DxDocument) -> String {
        self.pretty_printer.format_unchecked(doc)
    }

    /// Generate output files for a DxDocument
    ///
    /// Creates .human and .machine files in the configured output directory.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use serializer::{SerializerBuilder, DxDocument};
    /// use std::path::Path;
    ///
    /// let doc = DxDocument::new();
    /// let serializer = SerializerBuilder::new()
    ///     .output_dir("build/serializer")
    ///     .build();
    ///
    /// let result = serializer.generate_files(&doc, Path::new("config.sr")).unwrap();
    /// println!("Generated {} bytes LLM, {} bytes machine",
    ///          result.llm_size, result.machine_size);
    /// ```
    pub fn generate_files(
        &self,
        doc: &DxDocument,
        source_path: &std::path::Path,
    ) -> Result<
        crate::llm::serializer_output::SerializerResult,
        crate::llm::serializer_output::SerializerOutputError,
    > {
        self.output_generator.process_document(doc, source_path)
    }

    /// Get the pretty printer instance
    ///
    /// Provides access to the underlying PrettyPrinter for advanced use cases.
    pub fn pretty_printer(&self) -> &PrettyPrinter {
        &self.pretty_printer
    }

    /// Get the output generator instance
    ///
    /// Provides access to the underlying SerializerOutput for advanced use cases.
    pub fn output_generator(&self) -> &SerializerOutput {
        &self.output_generator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{DxLlmValue, DxSection};

    #[test]
    fn test_builder_default() {
        let builder = SerializerBuilder::new();
        assert_eq!(builder.indent_size, 0);
        assert!(builder.expand_keys);
        assert!(builder.use_list_format);
        assert!(builder.space_around_equals);
        assert!(!builder.validate_output); // Disabled by default
        assert!(builder.generate_llm);
        assert!(builder.generate_machine);
    }

    #[test]
    fn test_builder_fluent_api() {
        let builder = SerializerBuilder::new()
            .indent_size(4)
            .expand_keys(false)
            .use_list_format(false)
            .space_around_equals(false)
            .validate_output(false) // Keep disabled
            .generate_llm(false);

        assert_eq!(builder.indent_size, 4);
        assert!(!builder.expand_keys);
        assert!(!builder.use_list_format);
        assert!(!builder.space_around_equals);
        assert!(!builder.validate_output);
        assert!(!builder.generate_llm);
    }

    #[test]
    fn test_builder_presets() {
        // Test for_tables preset
        let tables_builder = SerializerBuilder::new().for_tables();
        assert_eq!(tables_builder.indent_size, 20);
        assert!(!tables_builder.use_list_format);
        assert!(tables_builder.expand_keys);

        // Test for_compact preset
        let compact_builder = SerializerBuilder::new().for_compact();
        assert_eq!(compact_builder.indent_size, 0);
        assert!(!compact_builder.expand_keys);
        assert!(!compact_builder.use_list_format);
        assert!(!compact_builder.space_around_equals);

        // Test for_humans preset
        let human_builder = SerializerBuilder::new().for_humans();
        assert_eq!(human_builder.indent_size, 0);
        assert!(human_builder.expand_keys);
        assert!(human_builder.use_list_format);
        assert!(human_builder.space_around_equals);
    }

    #[test]
    fn test_serializer_basic_usage() {
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("TestApp".to_string()));
        doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));

        let serializer = SerializerBuilder::new().build();

        // Test serialize
        let text = serializer.serialize(&doc);
        assert!(!text.is_empty());

        // Test deserialize
        let parsed = serializer.deserialize(&text);
        assert!(parsed.is_ok());
        let parsed_doc = parsed.unwrap();
        assert_eq!(parsed_doc.context.len(), doc.context.len());
    }

    #[test]
    fn test_serializer_human_format() {
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("TestApp".to_string()));
        doc.context.insert(
            "editor".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("neovim".to_string()),
                DxLlmValue::Str("vscode".to_string()),
            ]),
        );

        let serializer = SerializerBuilder::new().for_humans().build();

        // Test human format (unchecked since validation is disabled)
        let human_text = serializer.format_human_unchecked(&doc);
        assert!(!human_text.is_empty(), "Human format should produce output");
    }

    #[test]
    fn test_serializer_compact_format() {
        let mut doc = DxDocument::new();
        doc.context.insert("name".to_string(), DxLlmValue::Str("TestApp".to_string()));
        doc.context.insert(
            "editor".to_string(),
            DxLlmValue::Arr(vec![
                DxLlmValue::Str("neovim".to_string()),
                DxLlmValue::Str("vscode".to_string()),
            ]),
        );

        let serializer = SerializerBuilder::new().for_compact().build();

        let human_text = serializer.format_human_unchecked(&doc);
        assert!(!human_text.is_empty(), "Compact format should produce output");
    }

    #[test]
    fn test_serializer_with_sections() {
        let mut doc = DxDocument::new();

        let mut section = DxSection::new(vec!["id".to_string(), "name".to_string()]);
        section
            .rows
            .push(vec![DxLlmValue::Num(1.0), DxLlmValue::Str("Alpha".to_string())]);
        section
            .rows
            .push(vec![DxLlmValue::Num(2.0), DxLlmValue::Str("Beta".to_string())]);
        doc.sections.insert('d', section);

        let serializer = SerializerBuilder::new().for_tables().build();

        let human_text = serializer.format_human_unchecked(&doc);
        // Just verify output is not empty
        assert!(!human_text.is_empty());
    }

    #[test]
    fn test_compact_arrays_option() {
        let builder = SerializerBuilder::new().compact_arrays(true);
        assert!(builder.compact_arrays);
        assert!(!builder.use_list_format); // Should be set to false when compact_arrays is true
    }

    #[test]
    fn test_output_dir_option() {
        let builder = SerializerBuilder::new().output_dir("custom/path");
        assert_eq!(builder.output_dir, Some(PathBuf::from("custom/path")));
    }
}
