//! Human format formatter for DXM documents.
//!
//! This module provides advanced formatting capabilities for the human-readable
//! DXM format with auto-formatting support, Unicode tables, and semantic blocks.
//!
//! The human format is designed for maximum readability with:
//! - Proper indentation and spacing
//! - Unicode box-drawing tables with aligned columns
//! - Clear section separators
//! - Symbol table with aligned definitions
//! - Beautiful ASCII art with FIGlet fonts (400+ fonts rotating)

use crate::figlet_manager;
use crate::table_renderer::{TableRenderer, TableRendererConfig};
use crate::types::*;

/// Formatter configuration options.
#[derive(Debug, Clone)]
pub struct FormatterConfig {
    /// Indentation width (spaces)
    pub indent_width: usize,
    /// Use Unicode box-drawing for tables
    pub unicode_tables: bool,
    /// Column padding for table alignment
    pub table_padding: usize,
    /// Align numeric columns to the right
    pub align_numbers_right: bool,
    /// Add blank lines between sections
    pub section_spacing: bool,
    /// Maximum line width (0 = no limit)
    pub max_line_width: usize,
    /// Add extra spacing for readability
    pub enhanced_readability: bool,
    /// Use FIGlet fonts for headers (if false, use Markdown # syntax)
    pub use_figlet_headers: bool,
    /// Counter for rotating styles (tables, lists, horizontal rules, etc.)
    pub style_counter: usize,
    /// Table format: true = dx-serializer style (TOML/INI), false = ASCII Plus tables
    pub use_serializer_tables: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_width: 2,
            unicode_tables: true,
            table_padding: 1,
            align_numbers_right: true,
            section_spacing: true,
            max_line_width: 0,
            enhanced_readability: true, // Enabled for beautiful .human files
            use_figlet_headers: false,  // Default to Markdown mode (# headers)
            style_counter: 0,
            use_serializer_tables: true, // Default to dx-serializer style tables
        }
    }
}

/// Human format formatter with auto-formatting support.
#[derive(Debug, Clone)]
pub struct HumanFormatter {
    /// Configuration options
    pub config: FormatterConfig,
    /// Table renderer instance
    table_renderer: TableRenderer,
    /// Header counter for FIGlet font rotation
    header_counter: usize,
    /// Table style counter for rotation
    table_style_counter: usize,
    /// Horizontal rule style counter for rotation
    hr_style_counter: usize,
}

impl Default for HumanFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl HumanFormatter {
    /// Create formatter with default config.
    pub fn new() -> Self {
        Self::with_config(FormatterConfig::default())
    }

    /// Create formatter with custom config.
    pub fn with_config(config: FormatterConfig) -> Self {
        let table_renderer = TableRenderer::with_config(TableRendererConfig {
            unicode: config.unicode_tables,
            padding: config.table_padding,
            align_numbers_right: config.align_numbers_right,
        });

        Self {
            config,
            table_renderer,
            header_counter: 0,
            table_style_counter: 0,
            hr_style_counter: 0,
        }
    }

    /// Format a document to human-readable string.
    pub fn format(&mut self, doc: &DxmDocument) -> String {
        let mut output = String::new();

        // Meta section - simple format
        if !self.config.enhanced_readability {
            output.push_str("[meta]\n");
            output.push_str(&format!("version = {}\n", doc.meta.version));
            if doc.meta.token_count > 0 {
                output.push_str(&format!("tokens = {}\n", doc.meta.token_count));
            }
            output.push('\n');
        }

        // Refs section (symbol table) - simple format
        if !doc.refs.is_empty() && !self.config.enhanced_readability {
            output.push_str("[refs]\n");
            let mut refs: Vec<_> = doc.refs.iter().collect();
            refs.sort_by_key(|(k, _)| *k);
            for (key, value) in refs {
                output.push_str(&format!("{} = {}\n", key, value));
            }
            output.push('\n');
        }

        // Content nodes
        for (i, node) in doc.nodes.iter().enumerate() {
            // Add blank line between all sections (if not already present)
            if i > 0 {
                // Check if output already ends with a blank line
                let trailing_newlines = output.chars().rev().take_while(|&c| c == '\n').count();
                // If we don't have at least 2 newlines (1 blank line), add one
                if trailing_newlines < 2 {
                    output.push('\n');
                }
            }

            output.push_str(&self.format_node(node));
            output.push('\n');
        }

        // Compress multiple blank lines to single blank line
        if self.config.enhanced_readability {
            output = self.compress_whitespace(&output);
        }

        output
    }

    /// Compress multiple blank lines into at most 1 blank line
    /// This ensures clean spacing throughout the document
    fn compress_whitespace(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut result = String::new();
        let mut blank_count = 0;

        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                blank_count += 1;
                // Allow up to 1 consecutive blank line (2 newlines = 1 blank line visually)
                if blank_count <= 1 {
                    result.push('\n');
                }
            } else {
                blank_count = 0;
                result.push_str(line);
                // Add newline after content (unless it's the last line)
                if i < lines.len() - 1 {
                    result.push('\n');
                }
            }
        }

        result
    }

    /// Format a single node.
    pub fn format_node(&mut self, node: &DxmNode) -> String {
        match node {
            DxmNode::Header(h) => self.format_header(h),
            DxmNode::Paragraph(inlines) => {
                // Simple paragraph - no indentation
                self.format_inlines(inlines)
            }
            DxmNode::CodeBlock(cb) => self.format_code_block(cb),
            DxmNode::Table(t) => self.format_table(t),
            DxmNode::List(l) => self.format_list(l),
            DxmNode::SemanticBlock(sb) => self.format_semantic_block(sb),
            DxmNode::HorizontalRule => self.format_horizontal_rule(),
        }
    }

    /// Format a horizontal rule with clean styling.
    fn format_horizontal_rule(&self) -> String {
        if self.config.enhanced_readability {
            // Clean, simple horizontal rule
            "─".repeat(70)
        } else {
            "---".to_string()
        }
    }

    /// Format a header with FIGlet fonts for levels 1-3, simple text for others.
    fn format_header(&mut self, header: &HeaderNode) -> String {
        let content = self.format_inlines(&header.content);

        let priority_suffix = match &header.priority {
            Some(p) => format!(" [{}]", p.to_marker()),
            None => String::new(),
        };

        if self.config.enhanced_readability {
            let full_text = format!("{}{}", content, priority_suffix);

            // Use FIGlet fonts for level 1-3 headers only if enabled
            if self.config.use_figlet_headers
                && header.level <= 3
                && let Some(figlet_text) =
                    figlet_manager::render_header_figlet(&full_text, self.header_counter)
            {
                self.header_counter += 1;
                return figlet_text;
            }

            // Fallback to Markdown # syntax
            let hashes = "#".repeat(header.level as usize);
            format!("{} {}", hashes, full_text)
        } else {
            let hashes = "#".repeat(header.level as usize);
            format!("{} {}{}", hashes, content, priority_suffix)
        }
    }

    /// Format inline nodes with clean, readable styling.
    fn format_inlines(&self, inlines: &[InlineNode]) -> String {
        let mut output = String::new();

        for inline in inlines {
            match inline {
                InlineNode::Text(text) => output.push_str(text),
                InlineNode::Bold(inner) => {
                    if self.config.enhanced_readability {
                        // Clean bold markers
                        output.push_str("**");
                        output.push_str(&self.format_inlines(inner));
                        output.push_str("**");
                    } else {
                        output.push_str("**");
                        output.push_str(&self.format_inlines(inner));
                        output.push_str("**");
                    }
                }
                InlineNode::Italic(inner) => {
                    if self.config.enhanced_readability {
                        // Clean italic markers
                        output.push('*');
                        output.push_str(&self.format_inlines(inner));
                        output.push('*');
                    } else {
                        output.push('*');
                        output.push_str(&self.format_inlines(inner));
                        output.push('*');
                    }
                }
                InlineNode::Strikethrough(inner) => {
                    output.push_str("~~");
                    output.push_str(&self.format_inlines(inner));
                    output.push_str("~~");
                }
                InlineNode::Code(code) => {
                    output.push('`');
                    output.push_str(code);
                    output.push('`');
                }
                InlineNode::Reference(key) => {
                    output.push_str(&format!("[^{}]", key));
                }
                InlineNode::Link { text, url, title } => {
                    output.push('[');
                    output.push_str(&self.format_inlines(text));
                    output.push_str("](");
                    output.push_str(url);
                    if let Some(t) = title {
                        output.push_str(&format!(" \"{}\"", t));
                    }
                    output.push(')');
                }
                InlineNode::Image { alt, url, title } => {
                    output.push_str("![");
                    output.push_str(alt);
                    output.push_str("](");
                    output.push_str(url);
                    if let Some(t) = title {
                        output.push_str(&format!(" \"{}\"", t));
                    }
                    output.push(')');
                }
            }
        }

        output
    }

    /// Format a code block with simple box and language.
    fn format_code_block(&self, cb: &CodeBlockNode) -> String {
        let lang = cb.language.as_deref().unwrap_or("");

        if self.config.enhanced_readability {
            let mut output = String::new();
            let lang_display = if lang.is_empty() { "CODE" } else { lang };

            // Simple header with language
            let header = format!("┌─── {} ", lang_display.to_uppercase());
            let padding = 70usize.saturating_sub(header.len());
            output.push_str(&header);
            output.push_str(&"─".repeat(padding));
            output.push_str("┐\n");

            // Add content with left border - no extra indentation
            for line in cb.content.lines() {
                output.push_str("│  ");
                output.push_str(line);
                output.push('\n');
            }

            // Handle empty code blocks
            if cb.content.is_empty() || !cb.content.contains('\n') && cb.content.trim().is_empty() {
                output.push_str("│  \n");
            }

            // Simple footer
            output.push('└');
            output.push_str(&"─".repeat(69));
            output.push('┘');
            output
        } else {
            format!("```{}\n{}\n```", lang, cb.content)
        }
    }

    /// Format a table using the configured style.
    fn format_table(&self, table: &TableNode) -> String {
        if self.config.enhanced_readability {
            if self.config.use_serializer_tables {
                // dx-serializer style (TOML/INI format)
                self.format_table_serializer(table)
            } else {
                // ASCII Plus Signs style
                self.format_table_ascii(table)
            }
        } else {
            self.table_renderer.render(table)
        }
    }

    /// Format a table in dx-serializer style (TOML/INI format).
    fn format_table_serializer(&self, table: &TableNode) -> String {
        let mut output = String::new();

        // Get column names
        let columns: Vec<&str> = table.schema.iter().map(|c| c.name.as_str()).collect();

        // Format each row as a section
        for (row_idx, row) in table.rows.iter().enumerate() {
            // Section header with row number
            output.push_str(&format!("[row_{}]\n", row_idx + 1));

            // Key-value pairs for each column
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx < columns.len() {
                    let key = columns[col_idx];
                    let value = match cell {
                        crate::types::CellValue::Text(s) => s.clone(),
                        crate::types::CellValue::Integer(i) => i.to_string(),
                        crate::types::CellValue::Float(f) => f.to_string(),
                        crate::types::CellValue::Boolean(b) => b.to_string(),
                        crate::types::CellValue::Null => "null".to_string(),
                    };
                    output.push_str(&format!("{} = {}\n", key, value));
                }
            }

            // Blank line between rows
            if row_idx < table.rows.len() - 1 {
                output.push('\n');
            }
        }

        output
    }

    /// Format a table with Unicode box-drawing.
    pub fn format_table_unicode(&self, table: &TableNode) -> String {
        let renderer = TableRenderer::with_config(TableRendererConfig {
            unicode: true,
            padding: self.config.table_padding,
            align_numbers_right: self.config.align_numbers_right,
        });
        renderer.render(table)
    }

    /// Format a table with ASCII characters.
    pub fn format_table_ascii(&self, table: &TableNode) -> String {
        let renderer = TableRenderer::with_config(TableRendererConfig {
            unicode: false,
            padding: self.config.table_padding,
            align_numbers_right: self.config.align_numbers_right,
        });
        renderer.render(table)
    }

    /// Format a list with clean bullets and proper spacing.
    fn format_list(&mut self, list: &ListNode) -> String {
        self.format_list_with_indent(list, 0)
    }

    /// Format a list with specified indentation level.
    fn format_list_with_indent(&mut self, list: &ListNode, indent_level: usize) -> String {
        let mut output = String::new();
        let base_indent = "  ".repeat(indent_level);

        // Clean, simple bullets
        let bullet = if self.config.enhanced_readability {
            "• "
        } else {
            "- "
        };

        for (i, item) in list.items.iter().enumerate() {
            output.push_str(&base_indent);

            if list.ordered {
                output.push_str(&format!("{}. ", i + 1));
            } else {
                output.push_str(bullet);
            }

            output.push_str(&self.format_inlines(&item.content));
            output.push('\n');

            // Handle nested lists
            if let Some(nested) = &item.nested {
                output.push_str(&self.format_list_with_indent(nested, indent_level + 1));
            }
        }

        // Remove trailing newline
        if output.ends_with('\n') {
            output.pop();
        }

        output
    }

    /// Format a semantic block with clean styling.
    fn format_semantic_block(&self, sb: &SemanticBlockNode) -> String {
        let (label, icon) = match sb.block_type {
            SemanticBlockType::Warning => ("WARNING", "⚠"),
            SemanticBlockType::FAQ => ("FAQ", "❓"),
            SemanticBlockType::Quote => ("QUOTE", "💬"),
            SemanticBlockType::Info => ("INFO", "ℹ"),
            SemanticBlockType::Example => ("EXAMPLE", "📝"),
        };

        let content = self.format_inlines(&sb.content);

        if self.config.enhanced_readability {
            let mut output = String::new();

            // Clean header with icon
            let header = format!("┌─ {} {} ", icon, label);
            let padding = 70usize.saturating_sub(header.len());
            output.push_str(&header);
            output.push_str(&"─".repeat(padding));
            output.push_str("┐\n");

            // Content with clean borders
            for line in content.lines() {
                output.push_str("│  ");
                output.push_str(line);
                output.push('\n');
            }
            if !content.contains('\n') && !content.is_empty() {
                output.push_str("│  ");
                output.push_str(&content);
                output.push('\n');
            }

            // Footer
            output.push('└');
            output.push_str(&"─".repeat(69));
            output.push('┘');
            output
        } else {
            // Markdown blockquote style
            let content_lines: Vec<&str> = content.lines().collect();
            if content_lines.is_empty() {
                format!("> [!{}]", label)
            } else if content_lines.len() == 1 {
                format!("> [!{}]\n> {}", label, content_lines[0])
            } else {
                let mut output = format!("> [!{}]", label);
                for line in content_lines {
                    output.push_str(&format!("\n> {}", line));
                }
                output
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_doc() -> DxmDocument {
        DxmDocument {
            meta: DxmMeta {
                version: "1.0".to_string(),
                token_count: 100,
                ..Default::default()
            },
            refs: {
                let mut refs = HashMap::new();
                refs.insert("doc".to_string(), "https://docs.example.com".to_string());
                refs
            },
            nodes: vec![
                DxmNode::Header(HeaderNode {
                    level: 1,
                    content: vec![InlineNode::Text("Hello World".to_string())],
                    priority: Some(Priority::Critical),
                }),
                DxmNode::Paragraph(vec![
                    InlineNode::Text("This is ".to_string()),
                    InlineNode::Bold(vec![InlineNode::Text("bold".to_string())]),
                    InlineNode::Text(" text.".to_string()),
                ]),
            ],
        }
    }

    /// Create a formatter with enhanced readability disabled for simpler tests
    fn simple_formatter() -> HumanFormatter {
        HumanFormatter::with_config(FormatterConfig {
            enhanced_readability: false,
            ..Default::default()
        })
    }

    #[test]
    fn test_format_header() {
        let mut formatter = simple_formatter();

        let header = HeaderNode {
            level: 2,
            content: vec![InlineNode::Text("Test Header".to_string())],
            priority: None,
        };

        let output = formatter.format_header(&header);
        assert_eq!(output, "## Test Header");
    }

    #[test]
    fn test_format_header_with_priority() {
        let mut formatter = simple_formatter();

        let header = HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Important".to_string())],
            priority: Some(Priority::Critical),
        };

        let output = formatter.format_header(&header);
        assert_eq!(output, "# Important [!!!]");
    }

    #[test]
    fn test_format_inline_styles() {
        let formatter = simple_formatter();

        let inlines = vec![
            InlineNode::Bold(vec![InlineNode::Text("bold".to_string())]),
            InlineNode::Text(" and ".to_string()),
            InlineNode::Italic(vec![InlineNode::Text("italic".to_string())]),
            InlineNode::Text(" and ".to_string()),
            InlineNode::Strikethrough(vec![InlineNode::Text("strike".to_string())]),
            InlineNode::Text(" and ".to_string()),
            InlineNode::Code("code".to_string()),
        ];

        let output = formatter.format_inlines(&inlines);
        assert_eq!(output, "**bold** and *italic* and ~~strike~~ and `code`");
    }

    #[test]
    fn test_format_code_block() {
        let formatter = simple_formatter();

        let cb = CodeBlockNode {
            language: Some("rust".to_string()),
            content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
            priority: None,
        };

        let output = formatter.format_code_block(&cb);
        assert!(output.starts_with("```rust\n"));
        assert!(output.ends_with("\n```"));
        assert!(output.contains("fn main()"));
    }

    #[test]
    fn test_format_code_block_enhanced() {
        // Create formatter with enhanced readability enabled
        let config = FormatterConfig {
            enhanced_readability: true,
            ..Default::default()
        };
        let mut formatter = HumanFormatter::with_config(config);

        let cb = CodeBlockNode {
            language: Some("rust".to_string()),
            content: "fn main() {}".to_string(),
            priority: None,
        };

        let output = formatter.format_code_block(&cb);
        assert!(output.contains("rust"));
        assert!(output.contains("fn main()"));
        assert!(output.contains("┌"));
        assert!(output.contains("└"));
    }

    #[test]
    fn test_format_code_block_preserves_content() {
        let formatter = simple_formatter();

        let content = "fn main() {\n    // Special chars: <>&\"'\n    let x = 42;\n}";
        let cb = CodeBlockNode {
            language: Some("rust".to_string()),
            content: content.to_string(),
            priority: None,
        };

        let output = formatter.format_code_block(&cb);
        // Content should be preserved exactly
        assert!(output.contains(content));
    }

    #[test]
    fn test_format_list_unordered() {
        let mut formatter = simple_formatter();

        let list = ListNode {
            ordered: false,
            items: vec![
                ListItem {
                    content: vec![InlineNode::Text("First".to_string())],
                    nested: None,
                },
                ListItem {
                    content: vec![InlineNode::Text("Second".to_string())],
                    nested: None,
                },
            ],
        };

        let output = formatter.format_list(&list);
        assert!(output.contains("First"));
        assert!(output.contains("Second"));
    }

    #[test]
    fn test_format_list_ordered() {
        let mut formatter = simple_formatter();

        let list = ListNode {
            ordered: true,
            items: vec![
                ListItem {
                    content: vec![InlineNode::Text("First".to_string())],
                    nested: None,
                },
                ListItem {
                    content: vec![InlineNode::Text("Second".to_string())],
                    nested: None,
                },
            ],
        };

        let output = formatter.format_list(&list);
        assert!(output.contains("1."));
        assert!(output.contains("First"));
        assert!(output.contains("2."));
        assert!(output.contains("Second"));
    }

    #[test]
    fn test_format_semantic_blocks() {
        let formatter = simple_formatter();

        let test_cases = vec![
            (SemanticBlockType::Warning, "WARNING"),
            (SemanticBlockType::Info, "INFO"),
            (SemanticBlockType::FAQ, "FAQ"),
            (SemanticBlockType::Quote, "QUOTE"),
            (SemanticBlockType::Example, "EXAMPLE"),
        ];

        for (block_type, expected_label) in test_cases {
            let sb = SemanticBlockNode {
                block_type,
                content: vec![InlineNode::Text("Test content".to_string())],
                priority: None,
            };

            let output = formatter.format_semantic_block(&sb);
            assert!(
                output.contains(&format!("> [!{}]", expected_label)),
                "Expected label {} not found in output: {}",
                expected_label,
                output
            );
            assert!(output.contains("Test content"));
        }
    }

    #[test]
    fn test_format_semantic_blocks_enhanced() {
        // Create formatter with enhanced readability enabled
        let config = FormatterConfig {
            enhanced_readability: true,
            ..Default::default()
        };
        let mut formatter = HumanFormatter::with_config(config);

        let sb = SemanticBlockNode {
            block_type: SemanticBlockType::Warning,
            content: vec![InlineNode::Text("Test warning".to_string())],
            priority: None,
        };

        let output = formatter.format_semantic_block(&sb);
        assert!(output.contains("WARNING"));
        assert!(output.contains("Test warning"));
        assert!(output.contains("┌"));
        assert!(output.contains("└"));
    }

    #[test]
    fn test_format_reference() {
        let formatter = simple_formatter();

        let inlines = vec![
            InlineNode::Text("See ".to_string()),
            InlineNode::Reference("doc".to_string()),
            InlineNode::Text(" for details.".to_string()),
        ];

        let output = formatter.format_inlines(&inlines);
        assert_eq!(output, "See [^doc] for details.");
    }

    #[test]
    fn test_format_document() {
        let mut formatter = HumanFormatter::new();
        let doc = create_test_doc();

        let output = formatter.format(&doc);

        // Check meta section
        assert!(output.contains("[meta]"));
        assert!(output.contains("version"));
        assert!(output.contains("1.0"));

        // Check refs section
        assert!(output.contains("[refs]"));
        assert!(output.contains("doc"));
        assert!(output.contains("https://docs.example.com"));

        // Check content
        assert!(output.contains("Hello World"));
        assert!(output.contains("**bold**"));
    }

    #[test]
    fn test_format_table_unicode() {
        let mut formatter = HumanFormatter::new();

        let table = TableNode {
            schema: vec![
                ColumnDef {
                    name: "id".to_string(),
                    type_hint: None,
                },
                ColumnDef {
                    name: "name".to_string(),
                    type_hint: None,
                },
            ],
            rows: vec![vec![
                CellValue::Integer(1),
                CellValue::Text("Alice".to_string()),
            ]],
        };

        let output = formatter.format_table_unicode(&table);
        assert!(output.contains('┌'));
        assert!(output.contains('│'));
        assert!(output.contains('└'));
    }

    #[test]
    fn test_format_table_ascii() {
        let mut formatter = HumanFormatter::new();

        let table = TableNode {
            schema: vec![
                ColumnDef {
                    name: "id".to_string(),
                    type_hint: None,
                },
                ColumnDef {
                    name: "name".to_string(),
                    type_hint: None,
                },
            ],
            rows: vec![vec![
                CellValue::Integer(1),
                CellValue::Text("Alice".to_string()),
            ]],
        };

        let output = formatter.format_table_ascii(&table);
        assert!(output.contains('|'));
        assert!(output.contains('-'));
        assert!(output.contains('+'));
        assert!(!output.contains('┌'));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Create a formatter with enhanced readability disabled for property tests
    fn simple_formatter() -> HumanFormatter {
        HumanFormatter::with_config(FormatterConfig {
            enhanced_readability: false,
            ..Default::default()
        })
    }

    /// Generate a random inline node
    fn arb_inline_node() -> impl Strategy<Value = InlineNode> {
        let leaf = prop_oneof![
            "[a-zA-Z0-9 ]{1,20}".prop_map(InlineNode::Text),
            "[a-zA-Z0-9_]{1,10}".prop_map(InlineNode::Code),
            "[a-zA-Z][a-zA-Z0-9_]{0,5}".prop_map(InlineNode::Reference),
        ];

        leaf.prop_recursive(2, 8, 3, |inner| {
            prop_oneof![
                inner.clone().prop_map(|n| InlineNode::Bold(vec![n])),
                inner.clone().prop_map(|n| InlineNode::Italic(vec![n])),
                inner.prop_map(|n| InlineNode::Strikethrough(vec![n])),
            ]
        })
    }

    /// Generate a random code block
    fn arb_code_block() -> impl Strategy<Value = CodeBlockNode> {
        (
            prop_oneof![
                Just(None),
                Just(Some("rust".to_string())),
                Just(Some("python".to_string())),
                Just(Some("javascript".to_string())),
            ],
            "[a-zA-Z0-9 \n\t!@#$%^&*()_+=\\[\\]{}|;:',.<>?/-]{0,200}",
        )
            .prop_map(|(language, content)| CodeBlockNode {
                language,
                content,
                priority: None,
            })
    }

    /// Generate a random semantic block
    fn arb_semantic_block() -> impl Strategy<Value = SemanticBlockNode> {
        (
            prop_oneof![
                Just(SemanticBlockType::Warning),
                Just(SemanticBlockType::Info),
                Just(SemanticBlockType::FAQ),
                Just(SemanticBlockType::Quote),
                Just(SemanticBlockType::Example),
            ],
            prop::collection::vec(arb_inline_node(), 1..=3),
        )
            .prop_map(|(block_type, content)| SemanticBlockNode {
                block_type,
                content,
                priority: None,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dxm-human-format, Property 10: Code Block Content Preservation**
        /// *For any* code block, formatting SHALL preserve the content exactly
        /// (byte-for-byte), only normalizing the delimiter syntax (``` markers).
        /// **Validates: Requirements 3.4, 1.4**
        #[test]
        fn prop_code_block_content_preservation(cb in arb_code_block()) {
            let formatter = simple_formatter();
            let output = formatter.format_code_block(&cb);

            // The output should contain the original content exactly
            prop_assert!(output.contains(&cb.content),
                "Code block content not preserved. Original: {:?}, Output: {:?}",
                cb.content, output);

            // Should have proper delimiters
            prop_assert!(output.starts_with("```"),
                "Code block should start with ```");
            prop_assert!(output.ends_with("```"),
                "Code block should end with ```");
        }

        /// **Feature: dxm-human-format, Property 6: Semantic Block Type Preservation**
        /// *For any* semantic block (Warning, Info, FAQ, Quote, Example), formatting
        /// SHALL produce output with the correct type label (> [!WARNING], > [!INFO],
        /// > [!FAQ], > [!QUOTE], > [!EXAMPLE]).
        /// **Validates: Requirements 8.1, 8.2, 8.3, 8.4, 8.5, 1.7**
        #[test]
        fn prop_semantic_block_type_preservation(sb in arb_semantic_block()) {
            let formatter = simple_formatter();
            let output = formatter.format_semantic_block(&sb);

            // Check that the correct label is present
            let expected_label = match sb.block_type {
                SemanticBlockType::Warning => "WARNING",
                SemanticBlockType::Info => "INFO",
                SemanticBlockType::FAQ => "FAQ",
                SemanticBlockType::Quote => "QUOTE",
                SemanticBlockType::Example => "EXAMPLE",
            };

            prop_assert!(output.contains(&format!("> [!{}]", expected_label)),
                "Expected label {} not found in output: {}", expected_label, output);

            // Should start with >
            prop_assert!(output.starts_with(">"),
                "Semantic block should start with >");
        }
    }
}
