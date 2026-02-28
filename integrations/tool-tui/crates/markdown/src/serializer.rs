//! DXM serializers for LLM and Human formats.
//!
//! This module provides serializers that convert `DxmDocument` AST
//! to various text formats.

use crate::types::*;

/// LLM format serializer (token-optimized).
///
/// Produces compact output optimized for AI consumption with minimal tokens.
#[derive(Debug, Clone, Default)]
pub struct LlmSerializer {
    /// Whether to include brain header
    pub include_brain_header: bool,
    /// Priority filter level (only include content at or above this priority)
    pub priority_filter: Option<Priority>,
    /// Whether to auto-generate references for repeated content
    pub auto_refs: bool,
}

impl LlmSerializer {
    /// Create a new LLM serializer with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable brain header generation.
    pub fn with_brain_header(mut self) -> Self {
        self.include_brain_header = true;
        self
    }

    /// Set priority filter.
    pub fn with_priority_filter(mut self, priority: Priority) -> Self {
        self.priority_filter = Some(priority);
        self
    }

    /// Enable auto-reference generation.
    pub fn with_auto_refs(mut self) -> Self {
        self.auto_refs = true;
        self
    }

    /// Serialize a document to LLM format.
    pub fn serialize(&self, doc: &DxmDocument) -> String {
        let mut output = String::new();

        // Brain header
        if self.include_brain_header {
            output.push_str(&self.serialize_brain_header(doc));
        }

        // Reference definitions
        for (key, value) in &doc.refs {
            output.push_str(&format!("#:{}|{}\n", key, value));
        }

        if !doc.refs.is_empty() {
            output.push('\n');
        }

        // Content nodes with priority filtering
        for node in &doc.nodes {
            if let Some(serialized) = self.serialize_node_with_priority(node) {
                output.push_str(&serialized);
                output.push('\n');
            }
        }

        output
    }

    /// Serialize a node with priority filtering applied.
    ///
    /// If priority filtering is enabled:
    /// - High-priority content (at or above filter level) is included in full
    /// - Low-priority content (below filter level) is collapsed to one-line summaries
    fn serialize_node_with_priority(&self, node: &DxmNode) -> Option<String> {
        let node_priority = self.get_node_priority(node);

        // If no priority filter is set, serialize normally
        let Some(filter_priority) = &self.priority_filter else {
            return self.serialize_node(node);
        };

        // Check if node should be included based on priority
        match node_priority {
            Some(priority) if priority >= *filter_priority => {
                // High priority: include full content
                self.serialize_node(node)
            }
            Some(_) => {
                // Low priority: collapse to summary
                self.serialize_node_summary(node)
            }
            None => {
                // No priority marker: treat as normal priority (include full)
                self.serialize_node(node)
            }
        }
    }

    /// Get the priority of a node, if any.
    fn get_node_priority(&self, node: &DxmNode) -> Option<Priority> {
        match node {
            DxmNode::Header(h) => h.priority,
            DxmNode::CodeBlock(cb) => cb.priority,
            DxmNode::SemanticBlock(sb) => sb.priority,
            _ => None,
        }
    }

    /// Serialize a node as a one-line summary (for low-priority content).
    fn serialize_node_summary(&self, node: &DxmNode) -> Option<String> {
        match node {
            DxmNode::Header(h) => {
                // Include header with [collapsed] marker using standard markdown syntax
                let content = self.serialize_inlines(&h.content);
                let truncated = if content.len() > 50 {
                    format!("{}...", &content[..47])
                } else {
                    content
                };
                let hashes = "#".repeat(h.level as usize);
                Some(format!("{} {} [collapsed]", hashes, truncated))
            }
            DxmNode::Paragraph(inlines) => {
                // Collapse paragraph to first 50 chars
                let content = self.serialize_inlines(inlines);
                if content.len() > 50 {
                    Some(format!("{}...", &content[..47]))
                } else {
                    Some(content)
                }
            }
            DxmNode::CodeBlock(cb) => {
                // Show language and line count only
                let line_count = cb.content.lines().count();
                let lang = cb.language.as_deref().unwrap_or("code");
                Some(format!("@{} [{} lines collapsed]@", lang, line_count))
            }
            DxmNode::Table(t) => {
                // Show schema and row count only
                let schema: Vec<&str> = t.schema.iter().map(|c| c.name.as_str()).collect();
                Some(format!("#t({}) [{} rows collapsed]", schema.join("|"), t.rows.len()))
            }
            DxmNode::List(l) => {
                // Show item count only
                let list_type = if l.ordered { "ordered" } else { "unordered" };
                Some(format!("[{} {} items collapsed]", l.items.len(), list_type))
            }
            DxmNode::SemanticBlock(sb) => {
                // Show type and collapsed marker
                let prefix = match sb.block_type {
                    SemanticBlockType::Warning => "#!",
                    SemanticBlockType::FAQ => "#?",
                    SemanticBlockType::Quote => "#>",
                    SemanticBlockType::Info => "#i",
                    SemanticBlockType::Example => "#x",
                };
                let content = self.serialize_inlines(&sb.content);
                let truncated = if content.len() > 40 {
                    format!("{}...", &content[..37])
                } else {
                    content
                };
                Some(format!("{}[collapsed] {}", prefix, truncated))
            }
            DxmNode::HorizontalRule => Some("---".to_string()),
        }
    }

    /// Serialize with token counting.
    pub fn serialize_with_tokens(&self, doc: &DxmDocument) -> (String, usize) {
        let output = self.serialize(doc);
        let token_count = estimate_tokens(&output);
        (output, token_count)
    }

    /// Serialize the brain header.
    fn serialize_brain_header(&self, doc: &DxmDocument) -> String {
        let mut header = String::new();

        header.push_str(&format!("@dxm|{}\n", doc.meta.version));

        // Token count
        let token_count = if doc.meta.token_count > 0 {
            doc.meta.token_count
        } else {
            // Estimate if not pre-calculated
            estimate_tokens(&self.serialize_content_only(doc))
        };

        // Section count
        let section_count = doc.nodes.iter().filter(|n| matches!(n, DxmNode::Header(_))).count();

        // Ref count
        let ref_count = doc.refs.len();

        header.push_str(&format!(
            "@meta|tokens:{}|sections:{}|refs:{}\n",
            token_count, section_count, ref_count
        ));

        // Priority distribution
        let priorities = self.count_priorities(doc);
        if priorities.critical > 0 || priorities.important > 0 || priorities.low > 0 {
            header.push_str(&format!(
                "@priority|!!!:{}|!!:{}|!:{}\n",
                priorities.critical, priorities.important, priorities.low
            ));
        }

        header.push('\n');
        header
    }

    /// Serialize content only (without brain header).
    fn serialize_content_only(&self, doc: &DxmDocument) -> String {
        let mut output = String::new();
        for node in &doc.nodes {
            if let Some(serialized) = self.serialize_node(node) {
                output.push_str(&serialized);
                output.push('\n');
            }
        }
        output
    }

    /// Count priority distribution in document.
    fn count_priorities(&self, doc: &DxmDocument) -> PriorityDistribution {
        let mut dist = PriorityDistribution::default();

        for node in &doc.nodes {
            match node {
                DxmNode::Header(h) => match h.priority {
                    Some(Priority::Critical) => dist.critical += 1,
                    Some(Priority::Important) => dist.important += 1,
                    Some(Priority::Low) => dist.low += 1,
                    None => {}
                },
                DxmNode::CodeBlock(cb) => match cb.priority {
                    Some(Priority::Critical) => dist.critical += 1,
                    Some(Priority::Important) => dist.important += 1,
                    Some(Priority::Low) => dist.low += 1,
                    None => {}
                },
                DxmNode::SemanticBlock(sb) => match sb.priority {
                    Some(Priority::Critical) => dist.critical += 1,
                    Some(Priority::Important) => dist.important += 1,
                    Some(Priority::Low) => dist.low += 1,
                    None => {}
                },
                _ => {}
            }
        }

        dist
    }

    /// Serialize a single node.
    fn serialize_node(&self, node: &DxmNode) -> Option<String> {
        match node {
            DxmNode::Header(h) => Some(self.serialize_header(h)),
            DxmNode::Paragraph(inlines) => Some(self.serialize_inlines(inlines)),
            DxmNode::CodeBlock(cb) => Some(self.serialize_code_block(cb)),
            DxmNode::Table(t) => Some(self.serialize_table(t)),
            DxmNode::List(l) => Some(self.serialize_list(l)),
            DxmNode::SemanticBlock(sb) => Some(self.serialize_semantic_block(sb)),
            DxmNode::HorizontalRule => Some("---".to_string()),
        }
    }

    /// Serialize a header node.
    fn serialize_header(&self, header: &HeaderNode) -> String {
        // Use standard markdown # syntax instead of numbered format
        let mut output = "#".repeat(header.level as usize);
        output.push(' ');
        output.push_str(&self.serialize_inlines(&header.content));

        if let Some(priority) = &header.priority {
            output.push(' ');
            output.push_str(priority.to_marker());
        }

        output
    }

    /// Serialize inline nodes.
    fn serialize_inlines(&self, inlines: &[InlineNode]) -> String {
        let mut output = String::new();

        for inline in inlines {
            match inline {
                InlineNode::Text(text) => output.push_str(text),
                InlineNode::Bold(inner) => {
                    output.push_str(&self.serialize_inlines(inner));
                    output.push('!');
                }
                InlineNode::Italic(inner) => {
                    output.push_str(&self.serialize_inlines(inner));
                    output.push('/');
                }
                InlineNode::Strikethrough(inner) => {
                    output.push_str(&self.serialize_inlines(inner));
                    output.push('~');
                }
                InlineNode::Code(code) => {
                    output.push_str(code);
                    output.push('@');
                }
                InlineNode::Reference(key) => {
                    output.push('^');
                    output.push_str(key);
                }
                InlineNode::Link { text, url, .. } => {
                    output.push('[');
                    output.push_str(&self.serialize_inlines(text));
                    output.push_str("](");
                    output.push_str(url);
                    output.push(')');
                }
                InlineNode::Image { alt, url, .. } => {
                    output.push_str("![");
                    output.push_str(alt);
                    output.push_str("](");
                    output.push_str(url);
                    output.push(')');
                }
            }
        }

        output
    }

    /// Serialize a code block.
    fn serialize_code_block(&self, cb: &CodeBlockNode) -> String {
        let mut output = String::from("@");

        if let Some(lang) = &cb.language {
            output.push_str(lang);
        }

        output.push('\n');
        output.push_str(&cb.content);
        output.push_str("\n@");

        output
    }

    /// Serialize a table.
    fn serialize_table(&self, table: &TableNode) -> String {
        let mut output = String::from("#t(");

        // Schema
        let schema: Vec<&str> = table.schema.iter().map(|c| c.name.as_str()).collect();
        output.push_str(&schema.join("|"));
        output.push_str(")\n");

        // Rows
        for row in &table.rows {
            let cells: Vec<String> = row
                .iter()
                .map(|cell| match cell {
                    CellValue::Text(t) => t.clone(),
                    CellValue::Integer(i) => i.to_string(),
                    CellValue::Float(f) => f.to_string(),
                    CellValue::Boolean(b) => b.to_string(),
                    CellValue::Null => String::new(),
                })
                .collect();
            output.push_str(&cells.join("|"));
            output.push('\n');
        }

        // Remove trailing newline
        if output.ends_with('\n') {
            output.pop();
        }

        output
    }

    /// Serialize a list.
    fn serialize_list(&self, list: &ListNode) -> String {
        let mut output = String::new();

        // Check if we can use compressed notation
        let can_compress = list.items.iter().all(|item| {
            item.nested.is_none()
                && item.content.len() == 1
                && matches!(&item.content[0], InlineNode::Text(t) if !t.contains(','))
        });

        if can_compress && !list.ordered {
            // Compressed notation: *a,b,c
            output.push('*');
            let items: Vec<String> = list
                .items
                .iter()
                .filter_map(|item| {
                    if let InlineNode::Text(t) = &item.content[0] {
                        Some(t.clone())
                    } else {
                        None
                    }
                })
                .collect();
            output.push_str(&items.join(","));
        } else {
            // Multi-line notation
            for (i, item) in list.items.iter().enumerate() {
                if list.ordered {
                    output.push_str(&format!("{}.", i + 1));
                } else {
                    output.push('*');
                }
                output.push_str(&self.serialize_inlines(&item.content));
                output.push('\n');
            }
            // Remove trailing newline
            if output.ends_with('\n') {
                output.pop();
            }
        }

        output
    }

    /// Serialize a semantic block.
    fn serialize_semantic_block(&self, sb: &SemanticBlockNode) -> String {
        let prefix = match sb.block_type {
            SemanticBlockType::Warning => "#!",
            SemanticBlockType::FAQ => "#?",
            SemanticBlockType::Quote => "#>",
            SemanticBlockType::Info => "#i",
            SemanticBlockType::Example => "#x",
        };

        format!("{}{}", prefix, self.serialize_inlines(&sb.content))
    }
}

/// Estimate token count for a string.
///
/// Uses a simple heuristic: ~4 characters per token on average.
/// This is a rough approximation of tiktoken behavior.
fn estimate_tokens(s: &str) -> usize {
    // Simple heuristic: count words and punctuation
    let words = s.split_whitespace().count();
    let punctuation = s.chars().filter(|c| c.is_ascii_punctuation()).count();

    // Rough estimate: words + punctuation/2
    words + punctuation / 2
}

// ============================================================================
// Human Format Serializer
// ============================================================================

/// Human format serializer (beautiful display).
///
/// Produces readable output with proper indentation and Unicode formatting.
///
/// # Deprecation Notice
///
/// This serializer is deprecated in favor of [`crate::human_formatter::HumanFormatter`]
/// which provides more advanced features including:
/// - Configurable table rendering with the new TableRenderer
/// - Better semantic block formatting
/// - Nested list support
/// - Section spacing options
///
/// Use `HumanFormatter::new().format(&doc)` instead of `HumanSerializer::new().serialize(&doc)`.
#[derive(Debug, Clone)]
#[deprecated(
    since = "0.2.0",
    note = "Use HumanFormatter from human_formatter module instead"
)]
#[allow(deprecated)]
pub struct HumanSerializer {
    /// Indentation width (spaces)
    pub indent: usize,
    /// Whether to use Unicode box drawing for tables
    pub unicode_tables: bool,
}

#[allow(deprecated)]
impl Default for HumanSerializer {
    fn default() -> Self {
        Self {
            indent: 2,
            unicode_tables: true,
        }
    }
}

#[allow(deprecated)]
impl HumanSerializer {
    /// Create a new human serializer with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set indentation width.
    pub fn with_indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    /// Disable Unicode tables.
    pub fn without_unicode_tables(mut self) -> Self {
        self.unicode_tables = false;
        self
    }

    /// Serialize a document to human-readable format.
    pub fn serialize(&self, doc: &DxmDocument) -> String {
        let mut output = String::new();

        // Meta section
        output.push_str("[meta]\n");
        output.push_str(&format!("version = {}\n", doc.meta.version));
        if doc.meta.token_count > 0 {
            output.push_str(&format!("tokens = {}\n", doc.meta.token_count));
        }
        output.push('\n');

        // Refs section
        if !doc.refs.is_empty() {
            output.push_str("[refs]\n");
            for (key, value) in &doc.refs {
                output.push_str(&format!("{} = {}\n", key, value));
            }
            output.push('\n');
        }

        // Content
        for node in &doc.nodes {
            output.push_str(&self.serialize_node(node));
            output.push('\n');
        }

        output
    }

    /// Serialize a single node.
    fn serialize_node(&self, node: &DxmNode) -> String {
        match node {
            DxmNode::Header(h) => self.serialize_header(h),
            DxmNode::Paragraph(inlines) => self.serialize_inlines(inlines),
            DxmNode::CodeBlock(cb) => self.serialize_code_block(cb),
            DxmNode::Table(t) => self.serialize_table(t),
            DxmNode::List(l) => self.serialize_list(l),
            DxmNode::SemanticBlock(sb) => self.serialize_semantic_block(sb),
            DxmNode::HorizontalRule => "---".to_string(),
        }
    }

    /// Serialize a header (Markdown-style).
    fn serialize_header(&self, header: &HeaderNode) -> String {
        let hashes = "#".repeat(header.level as usize);
        let content = self.serialize_inlines(&header.content);

        let priority_suffix = match &header.priority {
            Some(p) => format!(" [{}]", p.to_marker()),
            None => String::new(),
        };

        format!("{} {}{}", hashes, content, priority_suffix)
    }

    /// Serialize inline nodes (Markdown-style).
    fn serialize_inlines(&self, inlines: &[InlineNode]) -> String {
        let mut output = String::new();

        for inline in inlines {
            match inline {
                InlineNode::Text(text) => output.push_str(text),
                InlineNode::Bold(inner) => {
                    output.push_str("**");
                    output.push_str(&self.serialize_inlines(inner));
                    output.push_str("**");
                }
                InlineNode::Italic(inner) => {
                    output.push('*');
                    output.push_str(&self.serialize_inlines(inner));
                    output.push('*');
                }
                InlineNode::Strikethrough(inner) => {
                    output.push_str("~~");
                    output.push_str(&self.serialize_inlines(inner));
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
                InlineNode::Link { text, url, .. } => {
                    output.push('[');
                    output.push_str(&self.serialize_inlines(text));
                    output.push_str("](");
                    output.push_str(url);
                    output.push(')');
                }
                InlineNode::Image { alt, url, .. } => {
                    output.push_str("![");
                    output.push_str(alt);
                    output.push_str("](");
                    output.push_str(url);
                    output.push(')');
                }
            }
        }

        output
    }

    /// Serialize a code block.
    fn serialize_code_block(&self, cb: &CodeBlockNode) -> String {
        let lang = cb.language.as_deref().unwrap_or("");
        format!("```{}\n{}\n```", lang, cb.content)
    }

    /// Serialize a table with Unicode box drawing.
    fn serialize_table(&self, table: &TableNode) -> String {
        if !self.unicode_tables {
            return self.serialize_table_ascii(table);
        }

        // Calculate column widths
        let mut widths: Vec<usize> = table.schema.iter().map(|c| c.name.len()).collect();

        for row in &table.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    let cell_str = self.cell_to_string(cell);
                    widths[i] = widths[i].max(cell_str.len());
                }
            }
        }

        let mut output = String::new();

        // Top border
        output.push('┌');
        for (i, w) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(*w + 2));
            if i < widths.len() - 1 {
                output.push('┬');
            }
        }
        output.push_str("┐\n");

        // Header row
        output.push('│');
        for (i, col) in table.schema.iter().enumerate() {
            output.push_str(&format!(" {:width$} ", col.name, width = widths[i]));
            output.push('│');
        }
        output.push('\n');

        // Header separator
        output.push('├');
        for (i, w) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(*w + 2));
            if i < widths.len() - 1 {
                output.push('┼');
            }
        }
        output.push_str("┤\n");

        // Data rows
        for row in &table.rows {
            output.push('│');
            for (i, cell) in row.iter().enumerate() {
                let cell_str = self.cell_to_string(cell);
                let width = widths.get(i).copied().unwrap_or(0);
                output.push_str(&format!(" {:width$} ", cell_str, width = width));
                output.push('│');
            }
            output.push('\n');
        }

        // Bottom border
        output.push('└');
        for (i, w) in widths.iter().enumerate() {
            output.push_str(&"─".repeat(*w + 2));
            if i < widths.len() - 1 {
                output.push('┴');
            }
        }
        output.push('┘');

        output
    }

    /// Serialize a table with ASCII characters.
    fn serialize_table_ascii(&self, table: &TableNode) -> String {
        let mut output = String::new();

        // Header
        let headers: Vec<&str> = table.schema.iter().map(|c| c.name.as_str()).collect();
        output.push_str("| ");
        output.push_str(&headers.join(" | "));
        output.push_str(" |\n");

        // Separator
        output.push('|');
        for _ in &table.schema {
            output.push_str("---|");
        }
        output.push('\n');

        // Rows
        for row in &table.rows {
            output.push_str("| ");
            let cells: Vec<String> = row.iter().map(|c| self.cell_to_string(c)).collect();
            output.push_str(&cells.join(" | "));
            output.push_str(" |\n");
        }

        // Remove trailing newline
        if output.ends_with('\n') {
            output.pop();
        }

        output
    }

    /// Convert a cell value to string.
    fn cell_to_string(&self, cell: &CellValue) -> String {
        match cell {
            CellValue::Text(t) => t.clone(),
            CellValue::Integer(i) => i.to_string(),
            CellValue::Float(f) => f.to_string(),
            CellValue::Boolean(b) => b.to_string(),
            CellValue::Null => String::new(),
        }
    }

    /// Serialize a list.
    fn serialize_list(&self, list: &ListNode) -> String {
        let mut output = String::new();

        for (i, item) in list.items.iter().enumerate() {
            if list.ordered {
                output.push_str(&format!("{}. ", i + 1));
            } else {
                output.push_str("- ");
            }
            output.push_str(&self.serialize_inlines(&item.content));
            output.push('\n');
        }

        // Remove trailing newline
        if output.ends_with('\n') {
            output.pop();
        }

        output
    }

    /// Serialize a semantic block.
    fn serialize_semantic_block(&self, sb: &SemanticBlockNode) -> String {
        let label = match sb.block_type {
            SemanticBlockType::Warning => "WARNING",
            SemanticBlockType::FAQ => "FAQ",
            SemanticBlockType::Quote => "QUOTE",
            SemanticBlockType::Info => "INFO",
            SemanticBlockType::Example => "EXAMPLE",
        };

        format!("> [!{}]\n> {}", label, self.serialize_inlines(&sb.content))
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

    #[test]
    fn test_llm_serializer_basic() {
        let doc = create_test_doc();
        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("#:doc|https://docs.example.com"));
        assert!(output.contains("1|Hello World !!!"));
        assert!(output.contains("bold!"));
    }

    #[test]
    fn test_llm_serializer_with_brain_header() {
        let doc = create_test_doc();
        let serializer = LlmSerializer::new().with_brain_header();
        let output = serializer.serialize(&doc);

        assert!(output.contains("@dxm|1.0"));
        assert!(output.contains("@meta|"));
    }

    #[test]
    fn test_llm_serializer_code_block() {
        let doc = DxmDocument {
            nodes: vec![DxmNode::CodeBlock(CodeBlockNode {
                language: Some("rust".to_string()),
                content: "fn main() {}".to_string(),
                priority: None,
            })],
            ..Default::default()
        };

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("@rust"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains("\n@"));
    }

    #[test]
    fn test_llm_serializer_table() {
        let doc = DxmDocument {
            nodes: vec![DxmNode::Table(TableNode {
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
                    CellValue::Text("1".to_string()),
                    CellValue::Text("Alice".to_string()),
                ]],
            })],
            ..Default::default()
        };

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("#t(id|name)"));
        assert!(output.contains("1|Alice"));
    }

    #[test]
    fn test_llm_serializer_list_compressed() {
        let doc = DxmDocument {
            nodes: vec![DxmNode::List(ListNode {
                ordered: false,
                items: vec![
                    ListItem {
                        content: vec![InlineNode::Text("a".to_string())],
                        nested: None,
                    },
                    ListItem {
                        content: vec![InlineNode::Text("b".to_string())],
                        nested: None,
                    },
                    ListItem {
                        content: vec![InlineNode::Text("c".to_string())],
                        nested: None,
                    },
                ],
            })],
            ..Default::default()
        };

        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("*a,b,c"));
    }

    #[test]
    #[allow(deprecated)] // Testing deprecated API for backwards compatibility
    fn test_human_serializer_basic() {
        let doc = create_test_doc();
        let serializer = HumanSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("[meta]"));
        assert!(output.contains("version = 1.0"));
        assert!(output.contains("[refs]"));
        assert!(output.contains("# Hello World [!!!]"));
        assert!(output.contains("**bold**"));
    }

    #[test]
    #[allow(deprecated)] // Testing deprecated API for backwards compatibility
    fn test_human_serializer_unicode_table() {
        let doc = DxmDocument {
            nodes: vec![DxmNode::Table(TableNode {
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
                    CellValue::Text("1".to_string()),
                    CellValue::Text("Alice".to_string()),
                ]],
            })],
            ..Default::default()
        };

        let serializer = HumanSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("┌"));
        assert!(output.contains("│"));
        assert!(output.contains("└"));
    }

    #[test]
    #[allow(deprecated)] // Testing deprecated API for backwards compatibility
    fn test_human_serializer_semantic_block() {
        let doc = DxmDocument {
            nodes: vec![DxmNode::SemanticBlock(SemanticBlockNode {
                block_type: SemanticBlockType::Warning,
                content: vec![InlineNode::Text("Be careful!".to_string())],
                priority: None,
            })],
            ..Default::default()
        };

        let serializer = HumanSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("[!WARNING]"));
        assert!(output.contains("Be careful!"));
    }

    #[test]
    fn test_priority_filtering_includes_high_priority() {
        // Create a document with mixed priorities
        let doc = DxmDocument {
            nodes: vec![
                DxmNode::Header(HeaderNode {
                    level: 1,
                    content: vec![InlineNode::Text("Critical Section".to_string())],
                    priority: Some(Priority::Critical),
                }),
                DxmNode::Header(HeaderNode {
                    level: 2,
                    content: vec![InlineNode::Text("Important Section".to_string())],
                    priority: Some(Priority::Important),
                }),
                DxmNode::Header(HeaderNode {
                    level: 3,
                    content: vec![InlineNode::Text("Low Priority Section".to_string())],
                    priority: Some(Priority::Low),
                }),
            ],
            ..Default::default()
        };

        // Filter to Important and above
        let serializer = LlmSerializer::new().with_priority_filter(Priority::Important);
        let output = serializer.serialize(&doc);

        // Critical and Important should be included in full
        assert!(output.contains("1|Critical Section !!!"));
        assert!(output.contains("2|Important Section !!"));
        // Low priority should be collapsed
        assert!(output.contains("[collapsed]"));
    }

    #[test]
    fn test_priority_filtering_collapses_low_priority() {
        let doc = DxmDocument {
            nodes: vec![
                DxmNode::Header(HeaderNode {
                    level: 1,
                    content: vec![InlineNode::Text("Low Priority Header".to_string())],
                    priority: Some(Priority::Low),
                }),
                DxmNode::CodeBlock(CodeBlockNode {
                    language: Some("rust".to_string()),
                    content: "fn main() {\n    println!(\"Hello\");\n}".to_string(),
                    priority: Some(Priority::Low),
                }),
            ],
            ..Default::default()
        };

        // Filter to Critical only
        let serializer = LlmSerializer::new().with_priority_filter(Priority::Critical);
        let output = serializer.serialize(&doc);

        // Both should be collapsed
        assert!(output.contains("[collapsed]"));
        assert!(output.contains("lines collapsed"));
    }

    #[test]
    fn test_priority_filtering_no_filter_includes_all() {
        let doc = DxmDocument {
            nodes: vec![
                DxmNode::Header(HeaderNode {
                    level: 1,
                    content: vec![InlineNode::Text("Critical".to_string())],
                    priority: Some(Priority::Critical),
                }),
                DxmNode::Header(HeaderNode {
                    level: 2,
                    content: vec![InlineNode::Text("Low".to_string())],
                    priority: Some(Priority::Low),
                }),
            ],
            ..Default::default()
        };

        // No filter - should include all in full
        let serializer = LlmSerializer::new();
        let output = serializer.serialize(&doc);

        assert!(output.contains("1|Critical !!!"));
        assert!(output.contains("2|Low !"));
        assert!(!output.contains("[collapsed]"));
    }

    #[test]
    fn test_priority_filtering_no_priority_treated_as_normal() {
        let doc = DxmDocument {
            nodes: vec![
                DxmNode::Header(HeaderNode {
                    level: 1,
                    content: vec![InlineNode::Text("No Priority".to_string())],
                    priority: None,
                }),
                DxmNode::Paragraph(vec![InlineNode::Text("Regular paragraph".to_string())]),
            ],
            ..Default::default()
        };

        // Filter to Important - nodes without priority should be included
        let serializer = LlmSerializer::new().with_priority_filter(Priority::Important);
        let output = serializer.serialize(&doc);

        // No priority = normal priority = included in full
        assert!(output.contains("1|No Priority"));
        assert!(output.contains("Regular paragraph"));
        assert!(!output.contains("[collapsed]"));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating simple documents
    fn simple_doc_strategy() -> impl Strategy<Value = DxmDocument> {
        (
            prop::collection::vec(header_node_strategy(), 0..5),
            prop::collection::vec(paragraph_node_strategy(), 0..5),
        )
            .prop_map(|(headers, paragraphs)| {
                let mut nodes: Vec<DxmNode> = headers.into_iter().map(DxmNode::Header).collect();
                nodes.extend(paragraphs.into_iter().map(DxmNode::Paragraph));
                DxmDocument {
                    meta: DxmMeta {
                        version: "1.0".to_string(),
                        ..Default::default()
                    },
                    refs: std::collections::HashMap::new(),
                    nodes,
                }
            })
    }

    fn header_node_strategy() -> impl Strategy<Value = HeaderNode> {
        (1u8..=6, "[a-zA-Z ]{1,20}", prop::option::of(priority_strategy())).prop_map(
            |(level, text, priority)| HeaderNode {
                level,
                content: vec![InlineNode::Text(text)],
                priority,
            },
        )
    }

    fn paragraph_node_strategy() -> impl Strategy<Value = Vec<InlineNode>> {
        "[a-zA-Z ]{1,50}".prop_map(|text| vec![InlineNode::Text(text)])
    }

    fn priority_strategy() -> impl Strategy<Value = Priority> {
        prop_oneof![
            Just(Priority::Critical),
            Just(Priority::Important),
            Just(Priority::Low),
        ]
    }

    proptest! {
        /// **Feature: dx-markdown, Property 15: Brain Header Completeness**
        /// **Validates: Requirements 10.1, 10.2, 10.3, 10.4**
        ///
        /// *For any* document serialized with brain header enabled, the header SHALL
        /// contain token count, section hierarchy, reference list, and priority distribution.
        #[test]
        fn prop_brain_header_contains_token_count(doc in simple_doc_strategy()) {
            let serializer = LlmSerializer::new().with_brain_header();
            let output = serializer.serialize(&doc);

            prop_assert!(
                output.contains("@meta|tokens:"),
                "Brain header should contain token count"
            );
        }

        #[test]
        fn prop_brain_header_contains_section_count(doc in simple_doc_strategy()) {
            let serializer = LlmSerializer::new().with_brain_header();
            let output = serializer.serialize(&doc);

            prop_assert!(
                output.contains("|sections:"),
                "Brain header should contain section count"
            );
        }

        #[test]
        fn prop_brain_header_contains_ref_count(doc in simple_doc_strategy()) {
            let serializer = LlmSerializer::new().with_brain_header();
            let output = serializer.serialize(&doc);

            prop_assert!(
                output.contains("|refs:"),
                "Brain header should contain reference count"
            );
        }

        #[test]
        fn prop_brain_header_version(doc in simple_doc_strategy()) {
            let serializer = LlmSerializer::new().with_brain_header();
            let output = serializer.serialize(&doc);

            prop_assert!(
                output.contains("@dxm|1.0"),
                "Brain header should contain version"
            );
        }

        /// **Feature: dx-markdown, Property 9: Priority Marker Attachment**
        /// **Validates: Requirements 1.9, 8.1**
        ///
        /// *For any* node with a priority marker (!!!, !!, !), parsing SHALL attach
        /// the correct Priority enum value to that node.
        #[test]
        fn prop_priority_marker_attachment(
            level in 1u8..=6,
            content in "[a-zA-Z]{3,15}",
            priority in priority_strategy()
        ) {
            use crate::parser::DxmParser;

            // Create input with priority marker
            let marker = priority.to_marker();
            let input = format!("{}|{} {}", level, content, marker);

            // Parse the input
            let doc = DxmParser::parse(&input).unwrap();

            // Verify the header has the correct priority attached
            prop_assert!(!doc.nodes.is_empty(), "Document should have at least one node");

            if let DxmNode::Header(header) = &doc.nodes[0] {
                prop_assert!(
                    header.priority.is_some(),
                    "Header should have priority attached"
                );
                prop_assert_eq!(
                    header.priority.unwrap(),
                    priority,
                    "Priority should match: expected {:?}, got {:?}",
                    priority,
                    header.priority
                );
            } else {
                prop_assert!(false, "First node should be a header");
            }
        }

        /// Property: Priority markers are preserved through serialization round trip
        #[test]
        fn prop_priority_marker_roundtrip(
            level in 1u8..=6,
            content in "[a-zA-Z]{3,15}",
            priority in priority_strategy()
        ) {
            use crate::parser::DxmParser;

            // Create document with priority
            let doc = DxmDocument {
                meta: DxmMeta {
                    version: "1.0".to_string(),
                    ..Default::default()
                },
                refs: std::collections::HashMap::new(),
                nodes: vec![DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(content)],
                    priority: Some(priority),
                })],
            };

            // Serialize
            let serializer = LlmSerializer::new();
            let output = serializer.serialize(&doc);

            // Parse back
            let doc2 = DxmParser::parse(&output).unwrap();

            // Verify priority is preserved
            if let DxmNode::Header(h2) = &doc2.nodes[0] {
                prop_assert_eq!(
                    h2.priority,
                    Some(priority),
                    "Priority should be preserved through round trip"
                );
            }
        }

        /// Property: Priority filtering correctly includes high-priority content
        #[test]
        fn prop_priority_filtering_high_priority_included(
            level in 1u8..=6,
            content in "[a-zA-Z]{3,15}"
        ) {
            // Create document with Critical priority
            let doc = DxmDocument {
                meta: DxmMeta::default(),
                refs: std::collections::HashMap::new(),
                nodes: vec![DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(content.clone())],
                    priority: Some(Priority::Critical),
                })],
            };

            // Filter to Important (Critical should still be included)
            let serializer = LlmSerializer::new().with_priority_filter(Priority::Important);
            let output = serializer.serialize(&doc);

            // Critical content should be included in full (not collapsed)
            prop_assert!(
                !output.contains("[collapsed]"),
                "Critical priority content should not be collapsed when filtering to Important"
            );
            prop_assert!(
                output.contains(&content),
                "Critical priority content should be included"
            );
        }

        /// Property: Priority filtering correctly collapses low-priority content
        #[test]
        fn prop_priority_filtering_low_priority_collapsed(
            level in 1u8..=6,
            content in "[a-zA-Z]{3,15}"
        ) {
            // Create document with Low priority
            let doc = DxmDocument {
                meta: DxmMeta::default(),
                refs: std::collections::HashMap::new(),
                nodes: vec![DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(content.clone())],
                    priority: Some(Priority::Low),
                })],
            };

            // Filter to Critical (Low should be collapsed)
            let serializer = LlmSerializer::new().with_priority_filter(Priority::Critical);
            let output = serializer.serialize(&doc);

            // Low priority content should be collapsed
            prop_assert!(
                output.contains("[collapsed]"),
                "Low priority content should be collapsed when filtering to Critical"
            );
        }
    }
}

#[cfg(test)]
mod prop_tests_roundtrip {
    use super::*;
    use crate::parser::DxmParser;
    use proptest::prelude::*;

    // Strategy for generating valid DXM input strings
    fn dxm_input_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple header
            (1u8..=6, "[a-zA-Z ]{1,20}")
                .prop_map(|(level, content)| { format!("{}|{}", level, content.trim()) }),
            // Header with priority
            (1u8..=6, "[a-zA-Z ]{1,20}", priority_marker_strategy()).prop_map(
                |(level, content, priority)| {
                    format!("{}|{} {}", level, content.trim(), priority)
                }
            ),
            // Simple paragraph
            "[a-zA-Z ]{5,50}".prop_map(|s| s.trim().to_string()),
            // Semantic block
            (semantic_prefix_strategy(), "[a-zA-Z ]{1,30}")
                .prop_map(|(prefix, content)| { format!("{}{}", prefix, content.trim()) }),
        ]
    }

    fn priority_marker_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("!!!"), Just("!!"),]
    }

    fn semantic_prefix_strategy() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("#!"), Just("#?"), Just("#>"), Just("#i"), Just("#x"),]
    }

    proptest! {
        /// **Feature: dx-markdown, Property 1: DXM Parse-Print Round Trip**
        /// **Validates: Requirements 2.2**
        ///
        /// *For any* valid DXM text document, parsing it into a DxmDocument AST and then
        /// printing it back to DXM text, then parsing again SHALL produce an equivalent AST.
        #[test]
        fn prop_parse_print_roundtrip(input in dxm_input_strategy()) {
            // Skip empty inputs
            if input.trim().is_empty() {
                return Ok(());
            }

            // First parse
            let doc1 = match DxmParser::parse(&input) {
                Ok(doc) => doc,
                Err(_) => return Ok(()), // Skip invalid inputs
            };

            // Serialize
            let serializer = LlmSerializer::new();
            let output = serializer.serialize(&doc1);

            // Second parse
            let doc2 = match DxmParser::parse(&output) {
                Ok(doc) => doc,
                Err(e) => {
                    prop_assert!(false, "Failed to parse serialized output: {}", e);
                    return Ok(());
                }
            };

            // Compare node counts (structural equivalence)
            prop_assert_eq!(
                doc1.nodes.len(),
                doc2.nodes.len(),
                "Node count should be preserved after round trip"
            );

            // Compare node types
            for (i, (n1, n2)) in doc1.nodes.iter().zip(doc2.nodes.iter()).enumerate() {
                let type1 = std::mem::discriminant(n1);
                let type2 = std::mem::discriminant(n2);
                prop_assert_eq!(
                    type1,
                    type2,
                    "Node {} type should be preserved: {:?} vs {:?}",
                    i,
                    n1,
                    n2
                );
            }
        }

        /// Property: Header level is preserved through round trip
        #[test]
        fn prop_header_level_roundtrip(level in 1u8..=6, content in "[a-zA-Z]{3,15}") {
            let input = format!("{}|{}", level, content);

            let doc1 = DxmParser::parse(&input).unwrap();
            let output = LlmSerializer::new().serialize(&doc1);
            let doc2 = DxmParser::parse(&output).unwrap();

            if let (DxmNode::Header(h1), DxmNode::Header(h2)) = (&doc1.nodes[0], &doc2.nodes[0]) {
                prop_assert_eq!(h1.level, h2.level, "Header level should be preserved");
            }
        }

        /// Property: Semantic block type is preserved through round trip
        #[test]
        fn prop_semantic_block_roundtrip(
            prefix in semantic_prefix_strategy(),
            content in "[a-zA-Z ]{3,20}"
        ) {
            let input = format!("{}{}", prefix, content.trim());

            let doc1 = DxmParser::parse(&input).unwrap();
            let output = LlmSerializer::new().serialize(&doc1);
            let doc2 = DxmParser::parse(&output).unwrap();

            if let (DxmNode::SemanticBlock(sb1), DxmNode::SemanticBlock(sb2)) = (&doc1.nodes[0], &doc2.nodes[0]) {
                prop_assert_eq!(
                    sb1.block_type,
                    sb2.block_type,
                    "Semantic block type should be preserved"
                );
            }
        }
    }
}
