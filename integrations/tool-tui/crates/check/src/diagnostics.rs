//! Binary Diagnostics
//!
//! Compact binary format for diagnostics - 33 bytes vs 300-500 bytes JSON.
//! Enables real-time 60fps linting updates.

use bytemuck::{Pod, Zeroable};
use std::fmt;
use std::path::PathBuf;

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum DiagnosticSeverity {
    /// Hint - informational suggestion
    Hint = 0,
    /// Info - informational message
    Info = 1,
    /// Warning - potential issue
    Warning = 2,
    /// Error - definite problem
    Error = 3,
}

impl DiagnosticSeverity {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hint => "hint",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    #[must_use]
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Hint => "💡",
            Self::Info => "ℹ️",
            Self::Warning => "⚠️",
            Self::Error => "❌",
        }
    }
}

/// Source span (byte offsets)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Pod, Zeroable)]
#[repr(C)]
pub struct Span {
    /// Start byte offset
    pub start: u32,
    /// End byte offset
    pub end: u32,
}

impl Span {
    #[must_use]
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    #[must_use]
    pub fn len(&self) -> u32 {
        self.end - self.start
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Convert to line/column using line index
    #[must_use]
    pub fn to_line_col(&self, line_index: &LineIndex) -> (LineCol, LineCol) {
        (line_index.line_col(self.start), line_index.line_col(self.end))
    }
}

impl From<oxc_span::Span> for Span {
    fn from(span: oxc_span::Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

/// Line and column (1-indexed for display)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

/// Index for fast byte offset to line/column conversion
pub struct LineIndex {
    /// Byte offsets of line starts
    line_starts: Vec<u32>,
}

impl LineIndex {
    #[must_use]
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { line_starts }
    }

    #[must_use]
    pub fn line_col(&self, offset: u32) -> LineCol {
        let line = self.line_starts.partition_point(|&start| start <= offset).saturating_sub(1);
        let line_start = self.line_starts[line];
        LineCol {
            line: (line + 1) as u32,        // 1-indexed
            col: (offset - line_start) + 1, // 1-indexed
        }
    }
}

/// Binary diagnostic format (33 bytes - compact for network transfer)
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C, packed)]
pub struct BinaryDiagnostic {
    /// File ID (index into file table)
    pub file_id: u32,
    /// Start byte offset
    pub start_byte: u32,
    /// End byte offset
    pub end_byte: u32,
    /// Severity (0=hint, 1=info, 2=warn, 3=error)
    pub severity: u8,
    /// Rule ID (index into rule table)
    pub rule_id: u16,
    /// Message template ID
    pub message_id: u16,
    /// Template parameters (for parameterized messages)
    pub captures: [u32; 4],
}

impl BinaryDiagnostic {
    #[must_use]
    pub fn span(&self) -> Span {
        Span {
            start: self.start_byte,
            end: self.end_byte,
        }
    }

    #[must_use]
    pub fn severity(&self) -> DiagnosticSeverity {
        match self.severity {
            0 => DiagnosticSeverity::Hint,
            1 => DiagnosticSeverity::Info,
            2 => DiagnosticSeverity::Warning,
            _ => DiagnosticSeverity::Error,
        }
    }
}

/// Full diagnostic with all context
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Source file path
    pub file: PathBuf,
    /// Span in source
    pub span: Span,
    /// Severity level
    pub severity: DiagnosticSeverity,
    /// Rule that produced this diagnostic
    pub rule_id: String,
    /// Human-readable message
    pub message: String,
    /// Optional suggestion for fixing
    pub suggestion: Option<String>,
    /// Related information
    pub related: Vec<RelatedInfo>,
    /// Quick fix if available
    pub fix: Option<Fix>,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(file: PathBuf, span: Span, rule_id: &str, message: impl Into<String>) -> Self {
        Self {
            file,
            span,
            severity: DiagnosticSeverity::Error,
            rule_id: rule_id.to_string(),
            message: message.into(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warn(file: PathBuf, span: Span, rule_id: &str, message: impl Into<String>) -> Self {
        Self {
            file,
            span,
            severity: DiagnosticSeverity::Warning,
            rule_id: rule_id.to_string(),
            message: message.into(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        }
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add a fix
    #[must_use]
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Add related information
    #[must_use]
    pub fn with_related(mut self, related: RelatedInfo) -> Self {
        self.related.push(related);
        self
    }

    /// Check if this diagnostic has all required fields
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.file.as_os_str().is_empty() && !self.rule_id.is_empty() && !self.message.is_empty()
    }

    /// Check if this diagnostic is fixable
    #[must_use]
    pub fn is_fixable(&self) -> bool {
        self.fix.is_some()
    }

    /// Convert to binary format for network transfer
    #[must_use]
    pub fn to_binary(&self, file_id: u32, rule_table: &RuleTable) -> BinaryDiagnostic {
        BinaryDiagnostic {
            file_id,
            start_byte: self.span.start,
            end_byte: self.span.end,
            severity: self.severity as u8,
            rule_id: rule_table.get_id(&self.rule_id).unwrap_or(0),
            message_id: 0, // TODO: implement message templates
            captures: [0; 4],
        }
    }

    /// Format for terminal output
    #[must_use]
    pub fn format(&self, source: &str) -> String {
        let line_index = LineIndex::new(source);
        let (start_lc, _end_lc) = self.span.to_line_col(&line_index);

        format!(
            "{} {}[{}]: {}\n  --> {}:{}:{}\n",
            self.severity.symbol(),
            self.severity.as_str(),
            self.rule_id,
            self.message,
            self.file.display(),
            start_lc.line,
            start_lc.col,
        )
    }
}

/// Builder for constructing diagnostics with fluent API
#[derive(Debug, Clone)]
pub struct DiagnosticBuilder {
    file: Option<PathBuf>,
    span: Option<Span>,
    severity: DiagnosticSeverity,
    rule_id: Option<String>,
    message: Option<String>,
    suggestion: Option<String>,
    related: Vec<RelatedInfo>,
    fix: Option<Fix>,
}

impl DiagnosticBuilder {
    /// Create a new diagnostic builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            file: None,
            span: None,
            severity: DiagnosticSeverity::Error,
            rule_id: None,
            message: None,
            suggestion: None,
            related: Vec::new(),
            fix: None,
        }
    }

    /// Create a builder for an error diagnostic
    #[must_use]
    pub fn error() -> Self {
        Self::new().severity(DiagnosticSeverity::Error)
    }

    /// Create a builder for a warning diagnostic
    #[must_use]
    pub fn warning() -> Self {
        Self::new().severity(DiagnosticSeverity::Warning)
    }

    /// Create a builder for an info diagnostic
    #[must_use]
    pub fn info() -> Self {
        Self::new().severity(DiagnosticSeverity::Info)
    }

    /// Create a builder for a hint diagnostic
    #[must_use]
    pub fn hint() -> Self {
        Self::new().severity(DiagnosticSeverity::Hint)
    }

    /// Set the file path
    pub fn file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set the span
    #[must_use]
    pub fn span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    /// Set the span from start and end positions
    #[must_use]
    pub fn span_range(mut self, start: u32, end: u32) -> Self {
        self.span = Some(Span::new(start, end));
        self
    }

    /// Set the severity
    #[must_use]
    pub fn severity(mut self, severity: DiagnosticSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set the rule ID
    pub fn rule_id(mut self, rule_id: impl Into<String>) -> Self {
        self.rule_id = Some(rule_id.into());
        self
    }

    /// Set the message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the suggestion
    pub fn suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add related information
    #[must_use]
    pub fn related(mut self, related: RelatedInfo) -> Self {
        self.related.push(related);
        self
    }

    /// Add a fix
    #[must_use]
    pub fn fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Add a replacement fix
    pub fn fix_replace(
        mut self,
        description: impl Into<String>,
        span: Span,
        new_text: impl Into<String>,
    ) -> Self {
        self.fix = Some(Fix::replace(description, span, new_text));
        self
    }

    /// Add a deletion fix
    pub fn fix_delete(mut self, description: impl Into<String>, span: Span) -> Self {
        self.fix = Some(Fix::delete(description, span));
        self
    }

    /// Add an insertion fix
    pub fn fix_insert(
        mut self,
        description: impl Into<String>,
        position: u32,
        text: impl Into<String>,
    ) -> Self {
        self.fix = Some(Fix::insert(description, position, text));
        self
    }

    /// Validate the builder has all required fields
    pub fn validate(&self) -> Result<(), DiagnosticBuilderError> {
        if self.file.is_none() {
            return Err(DiagnosticBuilderError::MissingField("file"));
        }
        if self.span.is_none() {
            return Err(DiagnosticBuilderError::MissingField("span"));
        }
        if self.rule_id.is_none() {
            return Err(DiagnosticBuilderError::MissingField("rule_id"));
        }
        if self.message.is_none() {
            return Err(DiagnosticBuilderError::MissingField("message"));
        }
        Ok(())
    }

    /// Build the diagnostic, returning an error if required fields are missing
    pub fn build(self) -> Result<Diagnostic, DiagnosticBuilderError> {
        self.validate()?;

        Ok(Diagnostic {
            file: self.file.unwrap(),
            span: self.span.unwrap(),
            severity: self.severity,
            rule_id: self.rule_id.unwrap(),
            message: self.message.unwrap(),
            suggestion: self.suggestion,
            related: self.related,
            fix: self.fix,
        })
    }

    /// Build the diagnostic, panicking if required fields are missing
    /// Use only when you're certain all fields are set
    #[must_use]
    pub fn build_unchecked(self) -> Diagnostic {
        self.build().expect("DiagnosticBuilder missing required fields")
    }
}

impl Default for DiagnosticBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for diagnostic builder validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticBuilderError {
    /// A required field is missing
    MissingField(&'static str),
}

impl std::fmt::Display for DiagnosticBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {field}"),
        }
    }
}

impl std::error::Error for DiagnosticBuilderError {}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]: {} at {:?}",
            self.severity.as_str(),
            self.rule_id,
            self.message,
            self.file
        )
    }
}

/// Related diagnostic information
#[derive(Debug, Clone)]
pub struct RelatedInfo {
    pub file: PathBuf,
    pub span: Span,
    pub message: String,
}

/// A code fix/edit
#[derive(Debug, Clone)]
pub struct Fix {
    /// Description of the fix
    pub description: String,
    /// Edits to apply
    pub edits: Vec<Edit>,
}

/// A single text edit
#[derive(Debug, Clone)]
pub struct Edit {
    /// Span to replace
    pub span: Span,
    /// New text
    pub new_text: String,
}

impl Fix {
    /// Create a fix that replaces the span with new text
    pub fn replace(
        description: impl Into<String>,
        span: Span,
        new_text: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            edits: vec![Edit {
                span,
                new_text: new_text.into(),
            }],
        }
    }

    /// Create a fix that deletes the span
    pub fn delete(description: impl Into<String>, span: Span) -> Self {
        Self::replace(description, span, "")
    }

    /// Create a fix that inserts text at a position
    pub fn insert(description: impl Into<String>, position: u32, text: impl Into<String>) -> Self {
        Self::replace(description, Span::new(position, position), text)
    }
}

/// Rule ID to index mapping for binary format
pub struct RuleTable {
    rules: Vec<String>,
}

impl RuleTable {
    #[must_use]
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn register(&mut self, rule_id: &str) -> u16 {
        if let Some(idx) = self.rules.iter().position(|r| r == rule_id) {
            idx as u16
        } else {
            let idx = self.rules.len() as u16;
            self.rules.push(rule_id.to_string());
            idx
        }
    }

    #[must_use]
    pub fn get_id(&self, rule_id: &str) -> Option<u16> {
        self.rules.iter().position(|r| r == rule_id).map(|i| i as u16)
    }

    #[must_use]
    pub fn get_name(&self, id: u16) -> Option<&str> {
        self.rules.get(id as usize).map(std::string::String::as_str)
    }
}

impl Default for RuleTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Differential diagnostic update for incremental updates
#[derive(Debug, Clone)]
pub struct DiagnosticPatch {
    /// Indices of removed diagnostics
    pub removed: Vec<u32>,
    /// New diagnostics
    pub added: Vec<BinaryDiagnostic>,
}

impl DiagnosticPatch {
    /// Compute minimal patch between old and new diagnostics
    #[must_use]
    pub fn compute(old: &[BinaryDiagnostic], new: &[BinaryDiagnostic]) -> Self {
        use std::collections::HashSet;

        let old_set: HashSet<_> = old.iter().map(Self::hash_diagnostic).collect();
        let new_set: HashSet<_> = new.iter().map(Self::hash_diagnostic).collect();

        Self {
            removed: old
                .iter()
                .enumerate()
                .filter(|(_, d)| !new_set.contains(&Self::hash_diagnostic(d)))
                .map(|(i, _)| i as u32)
                .collect(),
            added: new
                .iter()
                .filter(|d| !old_set.contains(&Self::hash_diagnostic(d)))
                .copied()
                .collect(),
        }
    }

    fn hash_diagnostic(d: &BinaryDiagnostic) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Copy fields to avoid unaligned reference issues with packed struct
        let file_id = d.file_id;
        let start_byte = d.start_byte;
        let end_byte = d.end_byte;
        let rule_id = d.rule_id;
        file_id.hash(&mut hasher);
        start_byte.hash(&mut hasher);
        end_byte.hash(&mut hasher);
        rule_id.hash(&mut hasher);
        hasher.finish()
    }

    /// Serialize to bytes (typically 10-100 bytes vs full array)
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend(&(self.removed.len() as u16).to_le_bytes());
        for idx in &self.removed {
            buf.extend(&idx.to_le_bytes());
        }
        buf.extend(&(self.added.len() as u16).to_le_bytes());
        for diag in &self.added {
            buf.extend(bytemuck::bytes_of(diag));
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span() {
        let span = Span::new(10, 20);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn test_line_index() {
        let source = "line1\nline2\nline3";
        let index = LineIndex::new(source);

        assert_eq!(index.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(index.line_col(6), LineCol { line: 2, col: 1 });
        assert_eq!(index.line_col(12), LineCol { line: 3, col: 1 });
    }

    #[test]
    fn test_binary_diagnostic_size() {
        assert_eq!(std::mem::size_of::<BinaryDiagnostic>(), 33);
    }

    #[test]
    fn test_diagnostic_builder_success() {
        let diagnostic = DiagnosticBuilder::error()
            .file("test.js")
            .span_range(10, 20)
            .rule_id("no-console")
            .message("Unexpected console statement")
            .suggestion("Remove console.log or use a logger")
            .build()
            .unwrap();

        assert_eq!(diagnostic.file, PathBuf::from("test.js"));
        assert_eq!(diagnostic.span.start, 10);
        assert_eq!(diagnostic.span.end, 20);
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
        assert_eq!(diagnostic.rule_id, "no-console");
        assert_eq!(diagnostic.message, "Unexpected console statement");
        assert_eq!(diagnostic.suggestion, Some("Remove console.log or use a logger".to_string()));
    }

    #[test]
    fn test_diagnostic_builder_with_fix() {
        let diagnostic = DiagnosticBuilder::warning()
            .file("test.js")
            .span_range(0, 10)
            .rule_id("eqeqeq")
            .message("Use === instead of ==")
            .fix_replace("Replace == with ===", Span::new(5, 7), "===")
            .build()
            .unwrap();

        assert!(diagnostic.is_fixable());
        let fix = diagnostic.fix.unwrap();
        assert_eq!(fix.description, "Replace == with ===");
        assert_eq!(fix.edits.len(), 1);
        assert_eq!(fix.edits[0].new_text, "===");
    }

    #[test]
    fn test_diagnostic_builder_missing_file() {
        let result = DiagnosticBuilder::error()
            .span_range(0, 10)
            .rule_id("test")
            .message("test message")
            .build();

        assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("file"))));
    }

    #[test]
    fn test_diagnostic_builder_missing_span() {
        let result = DiagnosticBuilder::error()
            .file("test.js")
            .rule_id("test")
            .message("test message")
            .build();

        assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("span"))));
    }

    #[test]
    fn test_diagnostic_builder_missing_rule_id() {
        let result = DiagnosticBuilder::error()
            .file("test.js")
            .span_range(0, 10)
            .message("test message")
            .build();

        assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("rule_id"))));
    }

    #[test]
    fn test_diagnostic_builder_missing_message() {
        let result = DiagnosticBuilder::error()
            .file("test.js")
            .span_range(0, 10)
            .rule_id("test")
            .build();

        assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("message"))));
    }

    #[test]
    fn test_diagnostic_is_valid() {
        let valid = Diagnostic::error(
            PathBuf::from("test.js"),
            Span::new(0, 10),
            "test-rule",
            "Test message",
        );
        assert!(valid.is_valid());

        let invalid = Diagnostic {
            file: PathBuf::new(),
            span: Span::new(0, 0),
            severity: DiagnosticSeverity::Error,
            rule_id: String::new(),
            message: String::new(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_diagnostic_with_related() {
        let diagnostic = Diagnostic::error(
            PathBuf::from("test.js"),
            Span::new(0, 10),
            "test-rule",
            "Test message",
        )
        .with_related(RelatedInfo {
            file: PathBuf::from("other.js"),
            span: Span::new(20, 30),
            message: "Related location".to_string(),
        });

        assert_eq!(diagnostic.related.len(), 1);
        assert_eq!(diagnostic.related[0].message, "Related location");
    }

    #[test]
    fn test_diagnostic_builder_all_severities() {
        let error = DiagnosticBuilder::error()
            .file("test.js")
            .span_range(0, 1)
            .rule_id("r")
            .message("m")
            .build()
            .unwrap();
        assert_eq!(error.severity, DiagnosticSeverity::Error);

        let warning = DiagnosticBuilder::warning()
            .file("test.js")
            .span_range(0, 1)
            .rule_id("r")
            .message("m")
            .build()
            .unwrap();
        assert_eq!(warning.severity, DiagnosticSeverity::Warning);

        let info = DiagnosticBuilder::info()
            .file("test.js")
            .span_range(0, 1)
            .rule_id("r")
            .message("m")
            .build()
            .unwrap();
        assert_eq!(info.severity, DiagnosticSeverity::Info);

        let hint = DiagnosticBuilder::hint()
            .file("test.js")
            .span_range(0, 1)
            .rule_id("r")
            .message("m")
            .build()
            .unwrap();
        assert_eq!(hint.severity, DiagnosticSeverity::Hint);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Generators for diagnostic components
    fn arb_severity() -> impl Strategy<Value = DiagnosticSeverity> {
        prop_oneof![
            Just(DiagnosticSeverity::Hint),
            Just(DiagnosticSeverity::Info),
            Just(DiagnosticSeverity::Warning),
            Just(DiagnosticSeverity::Error),
        ]
    }

    fn arb_span() -> impl Strategy<Value = Span> {
        (0u32..10000u32, 0u32..10000u32).prop_map(|(a, b)| {
            let (start, end) = if a <= b { (a, b) } else { (b, a) };
            Span::new(start, end)
        })
    }

    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        "[a-z]{1,10}\\.(js|ts|jsx|tsx)".prop_map(PathBuf::from)
    }

    fn arb_rule_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}".prop_map(String::from)
    }

    fn arb_message() -> impl Strategy<Value = String> {
        "[A-Za-z0-9 ]{1,100}".prop_map(String::from)
    }

    fn arb_edit() -> impl Strategy<Value = Edit> {
        (arb_span(), "[A-Za-z0-9 ]{0,50}").prop_map(|(span, new_text)| Edit { span, new_text })
    }

    fn arb_fix() -> impl Strategy<Value = Fix> {
        (arb_message(), prop::collection::vec(arb_edit(), 1..5))
            .prop_map(|(description, edits)| Fix { description, edits })
    }

    fn arb_diagnostic() -> impl Strategy<Value = Diagnostic> {
        (
            arb_file_path(),
            arb_span(),
            arb_severity(),
            arb_rule_id(),
            arb_message(),
            prop::option::of(arb_message()),
            prop::option::of(arb_fix()),
        )
            .prop_map(|(file, span, severity, rule_id, message, suggestion, fix)| {
                Diagnostic {
                    file,
                    span,
                    severity,
                    rule_id,
                    message,
                    suggestion,
                    related: Vec::new(),
                    fix,
                }
            })
    }

    fn arb_fixable_diagnostic() -> impl Strategy<Value = Diagnostic> {
        (
            arb_file_path(),
            arb_span(),
            arb_severity(),
            arb_rule_id(),
            arb_message(),
            prop::option::of(arb_message()),
            arb_fix(),
        )
            .prop_map(|(file, span, severity, rule_id, message, suggestion, fix)| {
                Diagnostic {
                    file,
                    span,
                    severity,
                    rule_id,
                    message,
                    suggestion,
                    related: Vec::new(),
                    fix: Some(fix),
                }
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Property 17: Diagnostic Format Completeness**
        /// *For any* diagnostic, it SHALL include all required fields: file path, span, severity, rule ID, and message.
        /// For fixable diagnostics, it SHALL also include fix description and edits.
        /// **Validates: Requirements 8.2, 8.3**
        #[test]
        fn prop_diagnostic_format_completeness(diagnostic in arb_diagnostic()) {
            // All diagnostics must have required fields
            prop_assert!(!diagnostic.file.as_os_str().is_empty(), "File path must not be empty");
            prop_assert!(!diagnostic.rule_id.is_empty(), "Rule ID must not be empty");
            prop_assert!(!diagnostic.message.is_empty(), "Message must not be empty");

            // Span must be valid (start <= end)
            prop_assert!(diagnostic.span.start <= diagnostic.span.end, "Span start must be <= end");

            // Severity must be one of the valid values
            let valid_severity = matches!(
                diagnostic.severity,
                DiagnosticSeverity::Hint
                    | DiagnosticSeverity::Info
                    | DiagnosticSeverity::Warning
                    | DiagnosticSeverity::Error
            );
            prop_assert!(valid_severity, "Severity must be a valid value");

            // is_valid() should return true for well-formed diagnostics
            prop_assert!(diagnostic.is_valid(), "Diagnostic should be valid");
        }

        /// **Property 17 (continued): Fixable Diagnostic Completeness**
        /// For fixable diagnostics, fix description and edits must be present.
        /// **Validates: Requirements 8.2, 8.3**
        #[test]
        fn prop_fixable_diagnostic_completeness(diagnostic in arb_fixable_diagnostic()) {
            // Fixable diagnostics must have a fix
            prop_assert!(diagnostic.is_fixable(), "Fixable diagnostic must have a fix");

            let fix = diagnostic.fix.as_ref().unwrap();

            // Fix must have a description
            prop_assert!(!fix.description.is_empty(), "Fix description must not be empty");

            // Fix must have at least one edit
            prop_assert!(!fix.edits.is_empty(), "Fix must have at least one edit");

            // Each edit must have a valid span
            for edit in &fix.edits {
                prop_assert!(edit.span.start <= edit.span.end, "Edit span start must be <= end");
            }
        }

        /// **Property 17 (continued): DiagnosticBuilder Validation**
        /// DiagnosticBuilder must reject diagnostics with missing required fields.
        /// **Validates: Requirements 8.2**
        #[test]
        fn prop_diagnostic_builder_validates_required_fields(
            file in arb_file_path(),
            span in arb_span(),
            rule_id in arb_rule_id(),
            message in arb_message()
        ) {
            // Complete builder should succeed
            let result = DiagnosticBuilder::error()
                .file(file.clone())
                .span(span)
                .rule_id(rule_id.clone())
                .message(message.clone())
                .build();
            prop_assert!(result.is_ok(), "Complete builder should succeed");

            // Missing file should fail
            let result = DiagnosticBuilder::error()
                .span(span)
                .rule_id(rule_id.clone())
                .message(message.clone())
                .build();
            prop_assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("file"))));

            // Missing span should fail
            let result = DiagnosticBuilder::error()
                .file(file.clone())
                .rule_id(rule_id.clone())
                .message(message.clone())
                .build();
            prop_assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("span"))));

            // Missing rule_id should fail
            let result = DiagnosticBuilder::error()
                .file(file.clone())
                .span(span)
                .message(message.clone())
                .build();
            prop_assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("rule_id"))));

            // Missing message should fail
            let result = DiagnosticBuilder::error()
                .file(file)
                .span(span)
                .rule_id(rule_id)
                .build();
            prop_assert!(matches!(result, Err(DiagnosticBuilderError::MissingField("message"))));
        }
    }
}
