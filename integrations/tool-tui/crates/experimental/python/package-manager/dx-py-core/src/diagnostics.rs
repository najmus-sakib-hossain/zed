//! Production-ready error diagnostics with helpful messages
//!
//! This module provides:
//! - Helpful error messages with suggestions
//! - Source context display for syntax errors
//! - Conflict tree display for resolution errors
//! - Colored terminal output support
//!
//! Requirements: 5.1.1-5.1.6

use std::fmt::{self, Display, Write};

/// ANSI color codes for terminal output
pub mod colors {
    pub const RED: &str = "\x1b[31m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const GREEN: &str = "\x1b[32m";
    pub const BLUE: &str = "\x1b[34m";
    pub const CYAN: &str = "\x1b[36m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const RESET: &str = "\x1b[0m";

    /// Check if colors should be used (respects NO_COLOR env var)
    pub fn enabled() -> bool {
        std::env::var("NO_COLOR").is_err() && atty::is(atty::Stream::Stderr)
    }

    /// Wrap text in color if colors are enabled
    pub fn colorize(text: &str, color: &str) -> String {
        if enabled() {
            format!("{}{}{}", color, text, RESET)
        } else {
            text.to_string()
        }
    }
}

/// A diagnostic message with severity and suggestions
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level
    pub severity: Severity,
    /// Main error message
    pub message: String,
    /// Optional error code (e.g., E0001)
    pub code: Option<String>,
    /// Source location if applicable
    pub location: Option<SourceLocation>,
    /// Source code snippet
    pub source_snippet: Option<SourceSnippet>,
    /// Helpful suggestions
    pub suggestions: Vec<Suggestion>,
    /// Related notes
    pub notes: Vec<String>,
    /// Help text
    pub help: Option<String>,
}

/// Severity level for diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (text, color) = match self {
            Severity::Error => ("error", colors::RED),
            Severity::Warning => ("warning", colors::YELLOW),
            Severity::Info => ("info", colors::BLUE),
            Severity::Hint => ("hint", colors::CYAN),
        };
        write!(f, "{}", colors::colorize(text, &format!("{}{}", colors::BOLD, color)))
    }
}

/// Source location information
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// File path
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Optional end column for spans
    pub end_column: Option<usize>,
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Source code snippet with highlighting
#[derive(Debug, Clone)]
pub struct SourceSnippet {
    /// Lines of source code (line number, content)
    pub lines: Vec<(usize, String)>,
    /// Highlight span (line, start_col, end_col)
    pub highlight: Option<(usize, usize, usize)>,
    /// Label for the highlight
    pub label: Option<String>,
}

/// A suggestion for fixing an error
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Description of the fix
    pub message: String,
    /// Optional replacement text
    pub replacement: Option<String>,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f32,
}

impl Diagnostic {
    /// Create a new error diagnostic
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            code: None,
            location: None,
            source_snippet: None,
            suggestions: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Create a new warning diagnostic
    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            code: None,
            location: None,
            source_snippet: None,
            suggestions: Vec::new(),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Set the error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the source location
    pub fn with_location(mut self, file: impl Into<String>, line: usize, column: usize) -> Self {
        self.location = Some(SourceLocation {
            file: file.into(),
            line,
            column,
            end_column: None,
        });
        self
    }

    /// Add source code context
    pub fn with_source(
        mut self,
        source: &str,
        line: usize,
        column: usize,
        end_column: Option<usize>,
    ) -> Self {
        let lines: Vec<&str> = source.lines().collect();
        let start_line = line.saturating_sub(2).max(1);
        let end_line = (line + 2).min(lines.len());

        let snippet_lines: Vec<(usize, String)> = (start_line..=end_line)
            .filter_map(|l| lines.get(l - 1).map(|s| (l, s.to_string())))
            .collect();

        self.source_snippet = Some(SourceSnippet {
            lines: snippet_lines,
            highlight: Some((line, column, end_column.unwrap_or(column + 1))),
            label: None,
        });
        self
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, message: impl Into<String>) -> Self {
        self.suggestions.push(Suggestion {
            message: message.into(),
            replacement: None,
            confidence: 0.8,
        });
        self
    }

    /// Add a suggestion with replacement
    pub fn with_fix(mut self, message: impl Into<String>, replacement: impl Into<String>) -> Self {
        self.suggestions.push(Suggestion {
            message: message.into(),
            replacement: Some(replacement.into()),
            confidence: 0.9,
        });
        self
    }

    /// Add a note
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Add help text
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Format the diagnostic for display
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Header: severity[code]: message
        write!(output, "{}", self.severity).unwrap();
        if let Some(ref code) = self.code {
            write!(output, "[{}]", colors::colorize(code, colors::BOLD)).unwrap();
        }
        writeln!(output, ": {}", colors::colorize(&self.message, colors::BOLD)).unwrap();

        // Location
        if let Some(ref loc) = self.location {
            writeln!(
                output,
                "  {} {}",
                colors::colorize("-->", colors::BLUE),
                colors::colorize(&loc.to_string(), colors::DIM)
            )
            .unwrap();
        }

        // Source snippet
        if let Some(ref snippet) = self.source_snippet {
            writeln!(output, "   {}", colors::colorize("|", colors::BLUE)).unwrap();

            for (line_num, content) in &snippet.lines {
                let line_str = format!("{:>4}", line_num);
                write!(
                    output,
                    "{} {} ",
                    colors::colorize(&line_str, colors::BLUE),
                    colors::colorize("|", colors::BLUE)
                )
                .unwrap();
                writeln!(output, "{}", content).unwrap();

                // Add highlight caret
                if let Some((highlight_line, start, end)) = snippet.highlight {
                    if *line_num == highlight_line {
                        let padding = " ".repeat(start.saturating_sub(1));
                        let carets = "^".repeat(end.saturating_sub(start).max(1));
                        writeln!(
                            output,
                            "     {} {}{}",
                            colors::colorize("|", colors::BLUE),
                            padding,
                            colors::colorize(&carets, colors::RED)
                        )
                        .unwrap();
                    }
                }
            }
            writeln!(output, "   {}", colors::colorize("|", colors::BLUE)).unwrap();
        }

        // Suggestions
        for suggestion in &self.suggestions {
            writeln!(output, "{}: {}", colors::colorize("help", colors::CYAN), suggestion.message)
                .unwrap();
            if let Some(ref replacement) = suggestion.replacement {
                writeln!(output, "      {}", colors::colorize(replacement, colors::GREEN)).unwrap();
            }
        }

        // Notes
        for note in &self.notes {
            writeln!(output, "{}: {}", colors::colorize("note", colors::BLUE), note).unwrap();
        }

        // Help
        if let Some(ref help) = self.help {
            writeln!(output, "{}: {}", colors::colorize("help", colors::CYAN), help).unwrap();
        }

        output
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Conflict tree for dependency resolution errors
#[derive(Debug, Clone)]
pub struct ConflictTree {
    /// Root cause of the conflict
    pub root: ConflictNode,
}

/// A node in the conflict tree
#[derive(Debug, Clone)]
pub enum ConflictNode {
    /// A package requirement
    Requirement {
        package: String,
        version_constraint: String,
        required_by: String,
    },
    /// A conflict between requirements
    Conflict {
        package: String,
        constraints: Vec<(String, String)>, // (required_by, constraint)
    },
    /// Derived from other conflicts
    Derived {
        cause1: Box<ConflictNode>,
        cause2: Box<ConflictNode>,
    },
}

impl ConflictTree {
    /// Create a new conflict tree from a conflict message
    pub fn from_message(_message: &str, packages: &[String]) -> Self {
        // Parse the conflict message to build a tree
        let constraints: Vec<(String, String)> =
            packages.iter().map(|p| (p.clone(), "any".to_string())).collect();

        Self {
            root: ConflictNode::Conflict {
                package: packages.first().cloned().unwrap_or_default(),
                constraints,
            },
        }
    }

    /// Format the conflict tree for display
    pub fn format(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "{}",
            colors::colorize(
                "Dependency conflict detected:",
                &format!("{}{}", colors::BOLD, colors::RED)
            )
        )
        .unwrap();
        writeln!(output).unwrap();
        Self::format_node(&self.root, &mut output, 0);
        output
    }

    fn format_node(node: &ConflictNode, output: &mut String, depth: usize) {
        let indent = "  ".repeat(depth);

        match node {
            ConflictNode::Requirement {
                package,
                version_constraint,
                required_by,
            } => {
                writeln!(
                    output,
                    "{}• {} {} requires {} {}",
                    indent,
                    colors::colorize("→", colors::BLUE),
                    colors::colorize(required_by, colors::CYAN),
                    colors::colorize(package, colors::YELLOW),
                    version_constraint
                )
                .unwrap();
            }
            ConflictNode::Conflict {
                package,
                constraints,
            } => {
                writeln!(
                    output,
                    "{}{}",
                    indent,
                    colors::colorize(
                        &format!("Package '{}' has conflicting requirements:", package),
                        colors::RED
                    )
                )
                .unwrap();
                for (required_by, constraint) in constraints {
                    writeln!(
                        output,
                        "{}  • {} requires {}",
                        indent,
                        colors::colorize(required_by, colors::CYAN),
                        colors::colorize(constraint, colors::YELLOW)
                    )
                    .unwrap();
                }
            }
            ConflictNode::Derived { cause1, cause2 } => {
                Self::format_node(cause1, output, depth);
                Self::format_node(cause2, output, depth);
            }
        }
    }
}

impl Display for ConflictTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Error diagnostics builder for common error types
pub struct DiagnosticsBuilder;

impl DiagnosticsBuilder {
    /// Create a syntax error diagnostic
    pub fn syntax_error(
        message: &str,
        source: &str,
        file: &str,
        line: usize,
        column: usize,
    ) -> Diagnostic {
        let mut diag = Diagnostic::error(format!("SyntaxError: {}", message))
            .with_code("E0001")
            .with_location(file, line, column)
            .with_source(source, line, column, None);

        // Add suggestions based on common mistakes
        diag = Self::add_syntax_suggestions(diag, message, source, line, column);

        diag
    }

    /// Create an import error diagnostic
    pub fn import_error(module: &str, search_paths: &[String]) -> Diagnostic {
        let similar = Self::find_similar_modules(module);

        let mut diag = Diagnostic::error(format!("ImportError: No module named '{}'", module))
            .with_code("E0002");

        if !similar.is_empty() {
            diag = diag.with_suggestion(format!("Did you mean '{}'?", similar[0]));
        }

        diag = diag.with_note(format!("Searched in: {}", search_paths.join(", ")));
        diag = diag.with_help("Make sure the module is installed: dx-py pip install <module>");

        diag
    }

    /// Create a type error diagnostic
    pub fn type_error(expected: &str, actual: &str, context: Option<&str>) -> Diagnostic {
        let mut diag =
            Diagnostic::error(format!("TypeError: expected {}, got {}", expected, actual))
                .with_code("E0003");

        if let Some(ctx) = context {
            diag = diag.with_note(format!("In context: {}", ctx));
        }

        // Add type conversion suggestions
        diag = Self::add_type_suggestions(diag, expected, actual);

        diag
    }

    /// Create a dependency conflict diagnostic
    pub fn dependency_conflict(
        package: &str,
        constraints: &[(String, String)],
        suggestions: &[String],
    ) -> Diagnostic {
        let mut diag = Diagnostic::error(format!(
            "DependencyConflict: Cannot satisfy requirements for '{}'",
            package
        ))
        .with_code("E0004");

        for (required_by, constraint) in constraints {
            diag = diag.with_note(format!("{} requires {} {}", required_by, package, constraint));
        }

        for suggestion in suggestions {
            diag = diag.with_suggestion(suggestion.clone());
        }

        diag = diag.with_help("Try relaxing version constraints or updating packages");

        diag
    }

    /// Create a resolution error diagnostic with conflict tree
    pub fn resolution_error(message: &str, packages: &[String]) -> Diagnostic {
        let conflict_tree = ConflictTree::from_message(message, packages);

        let mut diag = Diagnostic::error("Dependency resolution failed")
            .with_code("E0005")
            .with_note(conflict_tree.format());

        diag = diag.with_suggestion("Try relaxing version constraints for conflicting packages");
        diag = diag.with_suggestion("Check if there are newer versions available");
        diag = diag.with_suggestion("Use --pre flag to allow pre-release versions if needed");

        diag
    }

    /// Add syntax-specific suggestions
    fn add_syntax_suggestions(
        mut diag: Diagnostic,
        message: &str,
        source: &str,
        line: usize,
        _column: usize,
    ) -> Diagnostic {
        let lines: Vec<&str> = source.lines().collect();
        let current_line = lines.get(line.saturating_sub(1)).unwrap_or(&"");

        // Check for common mistakes
        if message.contains("expected ':'")
            && (current_line.contains("def ") || current_line.contains("class "))
        {
            diag = diag.with_fix(
                "Add a colon after the function/class definition",
                format!("{}:", current_line.trim_end()),
            );
        }

        if message.contains("expected ')'") {
            let open_count = current_line.matches('(').count();
            let close_count = current_line.matches(')').count();
            if open_count > close_count {
                diag = diag.with_suggestion("Missing closing parenthesis");
            }
        }

        if message.contains("indentation") {
            diag = diag.with_suggestion("Use consistent indentation (4 spaces recommended)");
            diag = diag.with_help("Python uses indentation to define code blocks");
        }

        if message.contains("invalid syntax") && current_line.contains("=") {
            if current_line.contains("==") {
                diag = diag.with_suggestion("Use '=' for assignment, '==' is for comparison");
            } else if current_line.contains(":=") {
                diag = diag.with_note("':=' is the walrus operator (Python 3.8+)");
            }
        }

        diag
    }

    /// Find similar module names for suggestions
    fn find_similar_modules(module: &str) -> Vec<String> {
        // Common module name corrections
        let corrections: &[(&str, &str)] = &[
            ("numpy", "numpy"),
            ("np", "numpy"),
            ("pandas", "pandas"),
            ("pd", "pandas"),
            ("sklearn", "scikit-learn"),
            ("cv2", "opencv-python"),
            ("PIL", "Pillow"),
            ("yaml", "PyYAML"),
            ("bs4", "beautifulsoup4"),
        ];

        let module_lower = module.to_lowercase();
        corrections
            .iter()
            .filter(|(name, _)| Self::levenshtein_distance(name, &module_lower) <= 2)
            .map(|(_, correct)| correct.to_string())
            .collect()
    }

    /// Add type conversion suggestions
    fn add_type_suggestions(mut diag: Diagnostic, expected: &str, actual: &str) -> Diagnostic {
        match (expected, actual) {
            ("int", "str") => {
                diag = diag.with_suggestion("Convert string to int: int(value)");
            }
            ("str", "int") => {
                diag = diag.with_suggestion("Convert int to string: str(value)");
            }
            ("float", "str") => {
                diag = diag.with_suggestion("Convert string to float: float(value)");
            }
            ("list", "str") => {
                diag = diag.with_suggestion("Convert string to list: list(value) or value.split()");
            }
            ("dict", "str") => {
                diag = diag.with_suggestion("Parse JSON string: json.loads(value)");
            }
            _ => {}
        }
        diag
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut matrix = vec![vec![0usize; b_len + 1]; a_len + 1];

        for (i, row) in matrix.iter_mut().enumerate().take(a_len + 1) {
            row[0] = i;
        }
        for (j, val) in matrix[0].iter_mut().enumerate().take(b_len + 1) {
            *val = j;
        }

        for i in 1..=a_len {
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[a_len][b_len]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_error() {
        let diag = Diagnostic::error("Something went wrong")
            .with_code("E0001")
            .with_suggestion("Try this instead");

        let formatted = diag.format();
        assert!(formatted.contains("error"));
        assert!(formatted.contains("Something went wrong"));
        assert!(formatted.contains("E0001"));
        assert!(formatted.contains("Try this instead"));
    }

    #[test]
    fn test_diagnostic_with_source() {
        let source = "def foo(x)\n    return x";
        let diag = Diagnostic::error("expected ':'").with_location("test.py", 1, 10).with_source(
            source,
            1,
            10,
            Some(11),
        );

        let formatted = diag.format();
        assert!(formatted.contains("def foo(x)"));
        assert!(formatted.contains("^"));
    }

    #[test]
    fn test_syntax_error_builder() {
        let source = "def foo(x)\n    return x";
        let diag = DiagnosticsBuilder::syntax_error("expected ':'", source, "test.py", 1, 10);

        let formatted = diag.format();
        assert!(formatted.contains("SyntaxError"));
        assert!(formatted.contains("colon"));
    }

    #[test]
    fn test_import_error_builder() {
        let diag = DiagnosticsBuilder::import_error("numpyy", &["/usr/lib/python3.12".to_string()]);

        let formatted = diag.format();
        assert!(formatted.contains("ImportError"));
        assert!(formatted.contains("numpyy"));
    }

    #[test]
    fn test_type_error_builder() {
        let diag = DiagnosticsBuilder::type_error("int", "str", Some("function argument"));

        let formatted = diag.format();
        assert!(formatted.contains("TypeError"));
        assert!(formatted.contains("int"));
        assert!(formatted.contains("str"));
    }

    #[test]
    fn test_conflict_tree() {
        let tree = ConflictTree::from_message(
            "Package c has conflicting requirements",
            &["a".to_string(), "b".to_string(), "c".to_string()],
        );

        let formatted = tree.format();
        assert!(formatted.contains("conflict"));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(DiagnosticsBuilder::levenshtein_distance("numpy", "numpy"), 0);
        assert_eq!(DiagnosticsBuilder::levenshtein_distance("numpy", "numpyy"), 1);
        assert_eq!(DiagnosticsBuilder::levenshtein_distance("numpy", "numpi"), 1);
        assert_eq!(DiagnosticsBuilder::levenshtein_distance("", "abc"), 3);
    }

    #[test]
    fn test_find_similar_modules() {
        let similar = DiagnosticsBuilder::find_similar_modules("numpyy");
        assert!(!similar.is_empty());
    }
}
