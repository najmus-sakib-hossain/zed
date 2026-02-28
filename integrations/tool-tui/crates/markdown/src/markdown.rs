//! Markdown parser and converter for DXM.
//!
//! Provides CommonMark and GFM parsing, converting Markdown to DXM format.

use crate::error::ConvertResult;
use crate::refs::ReferenceGraph;
use crate::types::*;

/// Markdown parser that converts CommonMark/GFM to DXM.
pub struct MarkdownParser<'a> {
    /// Input string
    input: &'a str,
    /// Current position
    pos: usize,
    /// Current line number
    line: usize,
    /// Reference graph for URL hoisting
    refs: ReferenceGraph,
    /// URL usage counts for auto-reference generation
    url_counts: std::collections::HashMap<String, usize>,
}

impl<'a> MarkdownParser<'a> {
    /// Create a new Markdown parser.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            refs: ReferenceGraph::new(),
            url_counts: std::collections::HashMap::new(),
        }
    }

    /// Parse Markdown into a DXM document.
    pub fn parse(input: &'a str) -> ConvertResult<DxmDocument> {
        let mut parser = Self::new(input);
        parser.parse_document()
    }

    /// Parse the entire document.
    fn parse_document(&mut self) -> ConvertResult<DxmDocument> {
        // First pass: count URL occurrences for auto-reference
        self.count_urls();

        // Reset position
        self.pos = 0;
        self.line = 1;

        let mut doc = DxmDocument::default();
        doc.meta.version = "1.0".to_string();

        // Preprocess input to split inline tables onto separate lines
        let preprocessed = self.preprocess_inline_tables();
        let mut parser = MarkdownParser::new(&preprocessed);
        parser.url_counts = self.url_counts.clone();

        // Parse content
        let mut last_pos = parser.pos;
        let mut stuck_count = 0;

        while !parser.is_eof() {
            parser.skip_empty_lines();
            if parser.is_eof() {
                break;
            }

            if let Some(node) = parser.parse_block()? {
                doc.nodes.push(node);

                // Detect infinite loop: position not advancing
                if parser.pos == last_pos {
                    stuck_count += 1;
                    if stuck_count > 10 {
                        return Err(crate::error::ConvertError::InvalidFormat(format!(
                            "Parser stuck at position {} (line {})",
                            parser.pos, parser.line
                        )));
                    }
                } else {
                    stuck_count = 0;
                    last_pos = parser.pos;
                }

                // Sanity check: prevent runaway parsing
                if doc.nodes.len() > 100_000 {
                    return Err(crate::error::ConvertError::InvalidFormat(format!(
                        "Too many nodes: {} (max 100K)",
                        doc.nodes.len()
                    )));
                }
            }
        }

        // Generate references for repeated URLs
        parser.generate_auto_refs(&mut doc);

        Ok(doc)
    }

    /// Preprocess input to split inline tables onto separate lines.
    /// Converts "text t:N(...)[ ... ]" into "text\nt:N(...)[ ... ]"
    fn preprocess_inline_tables(&self) -> String {
        let mut result = String::new();
        let input = self.input;
        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Look for table pattern: t:N where N is a digit
            if i + 2 < chars.len()
                && chars[i] == 't'
                && chars[i + 1] == ':'
                && chars[i + 2].is_ascii_digit()
            {
                // Check if there's non-whitespace text before this on the same line
                let mut line_start = i;
                while line_start > 0 && chars[line_start - 1] != '\n' {
                    line_start -= 1;
                }

                let before_table: String = chars[line_start..i].iter().collect();

                // If there's text before the table (not just whitespace), add a newline
                if !before_table.trim().is_empty() {
                    result.push('\n');
                }
            }

            // Add the current character
            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Count URL occurrences for auto-reference generation.
    fn count_urls(&mut self) {
        let input = self.input;
        let mut pos = 0;

        while pos < input.len() {
            // Look for [text](url) pattern
            if let Some(start) = input[pos..].find("](") {
                let url_start = pos + start + 2;
                if let Some(end) = input[url_start..].find(')') {
                    let url = &input[url_start..url_start + end];
                    *self.url_counts.entry(url.to_string()).or_insert(0) += 1;
                    pos = url_start + end + 1;
                    continue;
                }
            }
            // Move to next character boundary (not byte)
            if let Some(ch) = input[pos..].chars().next() {
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
    }

    /// Generate auto-references for repeated URLs.
    fn generate_auto_refs(&mut self, doc: &mut DxmDocument) {
        let mut key_index = 0usize;
        for (url, count) in &self.url_counts {
            if *count >= 2 {
                // Generate key: A, B, C, ..., Z, AA, AB, ...
                let key = Self::index_to_key(key_index);
                key_index += 1;
                self.refs.define(key.clone(), url.clone());
                doc.refs.insert(key, url.clone());
            }
        }
    }

    /// Convert an index to a key (A, B, ..., Z, AA, AB, ...).
    fn index_to_key(index: usize) -> String {
        let mut result = String::new();
        let mut n = index;
        loop {
            result.insert(0, (b'A' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        result
    }

    /// Parse a single block element.
    fn parse_block(&mut self) -> ConvertResult<Option<DxmNode>> {
        let start_pos = self.pos;
        self.skip_empty_lines();
        if self.is_eof() {
            return Ok(None);
        }

        // ATX headers (# Title)
        if self.peek() == Some('#') {
            return self.parse_header().map(Some);
        }

        // Code block (```)
        if self.peek_str("```") {
            return self.parse_code_block().map(Some);
        }

        // Single backtick is not a block element, treat as paragraph
        if self.peek() == Some('`') && !self.peek_str("```") {
            return self.parse_paragraph().map(Some);
        }

        // Horizontal rule (---, ***, ___)
        if self.is_horizontal_rule() {
            self.skip_line();
            return Ok(Some(DxmNode::HorizontalRule));
        }

        // Unordered list (-, *, +)
        if self.is_unordered_list_start() {
            return self.parse_list(false).map(Some);
        }

        // Ordered list (1. )
        if self.is_ordered_list_start() {
            return self.parse_list(true).map(Some);
        }

        // Blockquote (>)
        if self.peek() == Some('>') {
            return self.parse_blockquote().map(Some);
        }

        // GFM table (| header |)
        if self.peek() == Some('|') {
            return self.parse_table().map(Some);
        }

        // DX Serializer table (t:N(...)[...])
        if self.peek() == Some('t') && self.peek_n(1) == Some(':') {
            if let Some(table) = self.try_parse_dx_table()? {
                return Ok(Some(table));
            }
            // If table parsing failed, skip the line to avoid getting stuck
            self.skip_line();
            return Ok(None);
        }

        // Default: paragraph
        let result = self.parse_paragraph().map(Some);

        // SAFETY: Ensure we always advance position
        if self.pos == start_pos && !self.is_eof() {
            eprintln!(
                "WARNING: parse_block did not advance from pos {} (line {}), char: {:?}",
                start_pos,
                self.line,
                self.peek()
            );
            // Force advance to prevent infinite loop
            self.advance();
        }

        result
    }

    /// Parse an ATX header (# Title).
    fn parse_header(&mut self) -> ConvertResult<DxmNode> {
        let mut level = 0u8;
        while self.peek() == Some('#') && level < 6 {
            self.advance();
            level += 1;
        }

        // Skip space after #
        self.skip_whitespace_inline();

        let content_str = self.read_line();
        // Remove trailing # characters
        let content_str = content_str.trim_end_matches('#').trim();
        let content = self.parse_inline_content(content_str)?;

        Ok(DxmNode::Header(HeaderNode {
            level,
            content,
            priority: None,
        }))
    }

    /// Parse a fenced code block (```lang ... ```).
    fn parse_code_block(&mut self) -> ConvertResult<DxmNode> {
        self.advance_n(3); // skip ```

        let language = self.read_line();
        let language = language.trim();
        let language = if language.is_empty() {
            None
        } else {
            Some(language.to_string())
        };

        let mut content = String::new();
        while !self.is_eof() {
            if self.peek_str("```") {
                self.advance_n(3);
                self.skip_line();
                break;
            }
            if let Some(c) = self.peek() {
                content.push(c);
                self.advance();
            }
        }

        // Remove trailing newline
        if content.ends_with('\n') {
            content.pop();
        }

        Ok(DxmNode::CodeBlock(CodeBlockNode {
            language,
            content,
            priority: None,
        }))
    }

    /// Check if current line is a horizontal rule.
    fn is_horizontal_rule(&self) -> bool {
        let line = self.peek_line();
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            return false;
        }
        let first = trimmed.chars().next().unwrap_or(' ');
        if !matches!(first, '-' | '*' | '_') {
            return false;
        }
        trimmed.chars().all(|c| c == first || c.is_whitespace())
    }

    /// Check if at unordered list start.
    fn is_unordered_list_start(&self) -> bool {
        let c = self.peek().unwrap_or(' ');
        if !matches!(c, '-' | '*' | '+') {
            return false;
        }

        // Accept both "- text" (standard) and "-text" (DX LLM format)
        let next = self.peek_n(1);
        next == Some(' ') || next.map(|c| c.is_alphabetic()).unwrap_or(false)
    }

    /// Check if at ordered list start.
    fn is_ordered_list_start(&self) -> bool {
        let mut i = self.pos;
        let bytes = self.input.as_bytes();
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        i > self.pos
            && i < bytes.len()
            && bytes[i] == b'.'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b' '
    }

    /// Parse a list.
    fn parse_list(&mut self, ordered: bool) -> ConvertResult<DxmNode> {
        let mut items = Vec::new();

        while !self.is_eof() {
            if ordered {
                if !self.is_ordered_list_start() {
                    break;
                }
                // Skip number and dot
                while self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    self.advance();
                }
                self.advance(); // skip .
                if self.peek() == Some(' ') {
                    self.advance(); // skip space if present
                }
            } else {
                if !self.is_unordered_list_start() {
                    break;
                }
                self.advance(); // skip marker (-, *, +)
                if self.peek() == Some(' ') {
                    self.advance(); // skip space if present (for standard format)
                }
                // If no space, the text starts immediately (DX LLM format)
            }

            let content_str = self.read_line();
            let content = self.parse_inline_content(&content_str)?;

            items.push(ListItem {
                content,
                nested: None,
            });
        }

        Ok(DxmNode::List(ListNode { ordered, items }))
    }

    /// Parse a blockquote.
    fn parse_blockquote(&mut self) -> ConvertResult<DxmNode> {
        let mut lines = Vec::new();

        while !self.is_eof() && self.peek() == Some('>') {
            self.advance(); // skip >
            self.skip_whitespace_inline();
            let line = self.read_line();
            lines.push(line);
        }

        let content_str = lines.join(" ");
        let content = self.parse_inline_content(&content_str)?;

        Ok(DxmNode::SemanticBlock(SemanticBlockNode {
            block_type: SemanticBlockType::Quote,
            content,
            priority: None,
        }))
    }

    /// Parse a GFM table.
    fn parse_table(&mut self) -> ConvertResult<DxmNode> {
        // Parse header row
        let header_line = self.read_line();
        let schema: Vec<String> = header_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        let expected_cols = schema.len();

        // Skip separator row (|---|---|)
        if self.peek() == Some('|') {
            let sep_line = self.peek_line();
            if sep_line.contains('-') {
                self.skip_line();
            }
        }

        // Parse data rows
        let mut rows = Vec::new();
        while !self.is_eof() && self.peek() == Some('|') {
            let row_line = self.read_line();
            let mut cells: Vec<CellValue> = row_line
                .split('|')
                .filter(|s| !s.trim().is_empty())
                .map(|s| CellValue::Text(s.trim().to_string()))
                .collect();

            // Ensure row has exactly expected_cols columns
            while cells.len() < expected_cols {
                cells.push(CellValue::Text(String::new()));
            }
            if cells.len() > expected_cols {
                cells.truncate(expected_cols);
            }

            rows.push(cells);
        }

        let schema: Vec<ColumnDef> = schema
            .into_iter()
            .map(|name| ColumnDef {
                name,
                type_hint: None,
            })
            .collect();

        Ok(DxmNode::Table(TableNode { schema, rows }))
    }

    /// Try to parse a DX Serializer table format: t:N(col1,col2)[val1,val2;val3,val4]
    fn try_parse_dx_table(&mut self) -> ConvertResult<Option<DxmNode>> {
        let start_pos = self.pos;

        // Check for t:
        if self.peek() != Some('t') || self.peek_n(1) != Some(':') {
            return Ok(None);
        }

        self.advance_n(2); // skip t:

        // Parse column count
        let mut count_str = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                count_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if count_str.is_empty() || self.peek() != Some('(') {
            self.pos = start_pos;
            return Ok(None);
        }

        self.advance(); // skip (

        // Parse schema (column names)
        let mut schema_str = String::new();
        let mut paren_depth = 1;
        while let Some(c) = self.peek() {
            if c == '(' {
                paren_depth += 1;
            } else if c == ')' {
                paren_depth -= 1;
                if paren_depth == 0 {
                    self.advance();
                    break;
                }
            }
            schema_str.push(c);
            self.advance();
        }

        if self.peek() != Some('[') {
            self.pos = start_pos;
            return Ok(None);
        }

        self.advance(); // skip [

        // Parse data rows
        let mut data_str = String::new();
        let mut bracket_depth = 1;
        while let Some(c) = self.peek() {
            if c == '[' {
                bracket_depth += 1;
            } else if c == ']' {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    self.advance();
                    break;
                }
            }
            data_str.push(c);
            self.advance();
        }

        // Parse schema
        let schema: Vec<String> = schema_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if schema.is_empty() {
            self.pos = start_pos;
            return Ok(None);
        }

        // Parse rows - convert to CellValue
        let rows: Vec<Vec<CellValue>> = data_str
            .split(';')
            .map(|row| {
                row.split(',').map(|cell| CellValue::Text(cell.trim().to_string())).collect()
            })
            .filter(|row: &Vec<CellValue>| !row.is_empty())
            .collect();

        let schema_defs: Vec<ColumnDef> = schema
            .into_iter()
            .map(|name| ColumnDef {
                name,
                type_hint: None,
            })
            .collect();

        Ok(Some(DxmNode::Table(TableNode {
            schema: schema_defs,
            rows,
        })))
    }

    /// Parse a paragraph.
    fn parse_paragraph(&mut self) -> ConvertResult<DxmNode> {
        let mut lines = Vec::new();

        while !self.is_eof() && !self.is_block_boundary() {
            let line = self.read_line();
            if line.trim().is_empty() {
                break;
            }
            lines.push(line);
        }

        let content_str = lines.join(" ");
        let content = self.parse_inline_content(&content_str)?;

        Ok(DxmNode::Paragraph(content))
    }

    /// Check if at a block boundary.
    fn is_block_boundary(&self) -> bool {
        if self.is_eof() {
            return true;
        }
        let c = self.peek().unwrap_or(' ');

        // Check specific block markers
        if matches!(c, '#' | '>' | '|') {
            return true;
        }

        // Code block (```) is a boundary, but single ` is not
        if c == '`' && self.peek_str("```") {
            return true;
        }

        // DX Serializer table (t:N) is a block boundary
        if c == 't'
            && self.peek_n(1) == Some(':')
            && let Some(next_char) = self.peek_n(2)
            && next_char.is_ascii_digit()
        {
            return true;
        }

        // For -, *, + check if they're actually list markers or horizontal rules
        if matches!(c, '-' | '*' | '+') {
            return self.is_unordered_list_start() || self.is_horizontal_rule();
        }

        // Check for ordered lists
        self.is_ordered_list_start()
    }

    /// Parse inline content from a string.
    fn parse_inline_content(&self, s: &str) -> ConvertResult<Vec<InlineNode>> {
        let mut nodes = Vec::new();
        let mut current_text = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Bold (**text** or __text__)
            if (c == '*' || c == '_') && i + 1 < chars.len() && chars[i + 1] == c {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                let marker = c;
                i += 2;
                let mut end = i;
                while end + 1 < chars.len() {
                    if chars[end] == marker && chars[end + 1] == marker {
                        break;
                    }
                    end += 1;
                }
                let inner: String = chars[i..end].iter().collect();
                i = end + 2;
                nodes.push(InlineNode::Bold(vec![InlineNode::Text(inner)]));
                continue;
            }

            // Italic (*text* or _text_)
            if (c == '*' || c == '_') && i + 1 < chars.len() && chars[i + 1] != c {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                let marker = c;
                i += 1;
                let mut end = i;
                while end < chars.len() && chars[end] != marker {
                    end += 1;
                }
                let inner: String = chars[i..end].iter().collect();
                i = end + 1;
                nodes.push(InlineNode::Italic(vec![InlineNode::Text(inner)]));
                continue;
            }

            // Strikethrough (~~text~~)
            if c == '~' && i + 1 < chars.len() && chars[i + 1] == '~' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 2;
                let mut end = i;
                while end + 1 < chars.len() {
                    if chars[end] == '~' && chars[end + 1] == '~' {
                        break;
                    }
                    end += 1;
                }
                let inner: String = chars[i..end].iter().collect();
                i = end + 2;
                nodes.push(InlineNode::Strikethrough(vec![InlineNode::Text(inner)]));
                continue;
            }

            // Inline code (`code`)
            if c == '`' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 1;
                let mut end = i;
                while end < chars.len() && chars[end] != '`' {
                    end += 1;
                }
                let code: String = chars[i..end].iter().collect();
                i = end + 1;
                nodes.push(InlineNode::Code(code));
                continue;
            }

            // Image (![alt](url))
            if c == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 2;
                let mut end = i;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }
                let alt: String = chars[i..end].iter().collect();
                i = end + 1;

                if i < chars.len() && chars[i] == '(' {
                    i += 1;
                    let mut url_end = i;
                    while url_end < chars.len() && chars[url_end] != ')' {
                        url_end += 1;
                    }
                    let url: String = chars[i..url_end].iter().collect();
                    i = url_end + 1;
                    nodes.push(InlineNode::Image {
                        alt,
                        url,
                        title: None,
                    });
                }
                continue;
            }

            // Link ([text](url))
            if c == '[' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 1;
                let mut end = i;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }
                let text: String = chars[i..end].iter().collect();
                i = end + 1;

                if i < chars.len() && chars[i] == '(' {
                    i += 1;
                    let mut url_end = i;
                    while url_end < chars.len() && chars[url_end] != ')' {
                        url_end += 1;
                    }
                    let url: String = chars[i..url_end].iter().collect();
                    i = url_end + 1;
                    nodes.push(InlineNode::Link {
                        text: vec![InlineNode::Text(text)],
                        url,
                        title: None,
                    });
                } else {
                    current_text.push('[');
                    current_text.push_str(&text);
                    current_text.push(']');
                }
                continue;
            }

            current_text.push(c);
            i += 1;
        }

        if !current_text.is_empty() {
            nodes.push(InlineNode::Text(current_text));
        }

        Ok(nodes)
    }

    // ========================================================================
    // Low-level Scanner Methods
    // ========================================================================

    /// Check if we've reached the end of input.
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Peek at the current character without advancing.
    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Peek at the character n positions ahead.
    fn peek_n(&self, n: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(n)
    }

    /// Check if the input starts with the given string at current position.
    fn peek_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }

    /// Peek at the current line without advancing.
    fn peek_line(&self) -> &str {
        self.input[self.pos..].lines().next().unwrap_or("")
    }

    /// Advance by one character.
    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.line += 1;
            }
        }
    }

    /// Advance by n characters.
    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    /// Read the rest of the current line.
    fn read_line(&mut self) -> String {
        let start = self.pos;
        while !self.is_eof() && self.peek() != Some('\n') {
            self.advance();
        }
        let result = self.input[start..self.pos].to_string();
        if self.peek() == Some('\n') {
            self.advance();
        }
        result
    }

    /// Skip the current line entirely.
    fn skip_line(&mut self) {
        while !self.is_eof() && self.peek() != Some('\n') {
            self.advance();
        }
        if self.peek() == Some('\n') {
            self.advance();
        }
    }

    /// Skip whitespace on the current line only (spaces, tabs).
    fn skip_whitespace_inline(&mut self) {
        while !self.is_eof() {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => self.advance(),
                _ => break,
            }
        }
    }

    /// Skip empty lines.
    fn skip_empty_lines(&mut self) {
        while !self.is_eof() {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => self.advance(),
                _ => break,
            }
        }
    }
}

// ============================================================================
// DXM to Markdown Conversion
// ============================================================================

/// Convert a DXM document to CommonMark/GFM Markdown.
pub fn to_markdown(doc: &DxmDocument) -> String {
    let mut output = String::new();

    for node in &doc.nodes {
        output.push_str(&node_to_markdown(node, doc));
        output.push('\n');
    }

    output
}

/// Convert a single node to Markdown.
fn node_to_markdown(node: &DxmNode, doc: &DxmDocument) -> String {
    match node {
        DxmNode::Header(h) => {
            let prefix = "#".repeat(h.level as usize);
            let content = inlines_to_markdown(&h.content, doc);
            format!("{} {}", prefix, content)
        }
        DxmNode::Paragraph(inlines) => inlines_to_markdown(inlines, doc),
        DxmNode::CodeBlock(cb) => {
            let lang = cb.language.as_deref().unwrap_or("");
            format!("```{}\n{}\n```", lang, cb.content)
        }
        DxmNode::Table(t) => table_to_markdown(t),
        DxmNode::List(l) => list_to_markdown(l, doc),
        DxmNode::SemanticBlock(sb) => {
            let content = inlines_to_markdown(&sb.content, doc);
            match sb.block_type {
                SemanticBlockType::Warning => format!("> **Warning:** {}", content),
                SemanticBlockType::Info => format!("> **Note:** {}", content),
                SemanticBlockType::FAQ => format!("> **FAQ:** {}", content),
                SemanticBlockType::Quote => format!("> {}", content),
                SemanticBlockType::Example => format!("> **Example:** {}", content),
            }
        }
        DxmNode::HorizontalRule => "---".to_string(),
    }
}

/// Convert inline nodes to Markdown.
fn inlines_to_markdown(inlines: &[InlineNode], doc: &DxmDocument) -> String {
    let mut output = String::new();
    for inline in inlines {
        output.push_str(&inline_to_markdown(inline, doc));
    }
    output
}

/// Convert a single inline node to Markdown.
fn inline_to_markdown(inline: &InlineNode, doc: &DxmDocument) -> String {
    match inline {
        InlineNode::Text(t) => t.clone(),
        InlineNode::Bold(inner) => format!("**{}**", inlines_to_markdown(inner, doc)),
        InlineNode::Italic(inner) => format!("*{}*", inlines_to_markdown(inner, doc)),
        InlineNode::Strikethrough(inner) => format!("~~{}~~", inlines_to_markdown(inner, doc)),
        InlineNode::Code(c) => format!("`{}`", c),
        InlineNode::Link { text, url, title } => {
            let text_str = inlines_to_markdown(text, doc);
            if let Some(t) = title {
                format!("[{}]({} \"{}\")", text_str, url, t)
            } else {
                format!("[{}]({})", text_str, url)
            }
        }
        InlineNode::Image { alt, url, title } => {
            if let Some(t) = title {
                format!("![{}]({} \"{}\")", alt, url, t)
            } else {
                format!("![{}]({})", alt, url)
            }
        }
        InlineNode::Reference(key) => {
            // Expand reference
            doc.refs.get(key).cloned().unwrap_or_else(|| format!("[^{}]", key))
        }
    }
}

/// Convert a table to GFM Markdown.
fn table_to_markdown(table: &TableNode) -> String {
    let mut output = String::new();

    // Header row
    output.push('|');
    for col in &table.schema {
        output.push_str(&format!(" {} |", col.name));
    }
    output.push('\n');

    // Separator row
    output.push('|');
    for _ in &table.schema {
        output.push_str(" --- |");
    }
    output.push('\n');

    // Data rows
    for row in &table.rows {
        output.push('|');
        for cell in row {
            let cell_str = match cell {
                CellValue::Text(t) => t.clone(),
                CellValue::Integer(i) => i.to_string(),
                CellValue::Float(f) => f.to_string(),
                CellValue::Boolean(b) => b.to_string(),
                CellValue::Null => String::new(),
            };
            output.push_str(&format!(" {} |", cell_str));
        }
        output.push('\n');
    }

    output.trim_end().to_string()
}

/// Convert a list to Markdown.
fn list_to_markdown(list: &ListNode, doc: &DxmDocument) -> String {
    let mut output = String::new();
    for (i, item) in list.items.iter().enumerate() {
        let prefix = if list.ordered {
            format!("{}. ", i + 1)
        } else {
            "- ".to_string()
        };
        let content = inlines_to_markdown(&item.content, doc);
        output.push_str(&format!("{}{}\n", prefix, content));
    }
    output.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let doc = MarkdownParser::parse("").unwrap();
        assert!(doc.nodes.is_empty());
    }

    #[test]
    fn test_parse_header() {
        let doc = MarkdownParser::parse("# Hello World").unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::Header(h) = &doc.nodes[0] {
            assert_eq!(h.level, 1);
        } else {
            panic!("Expected header");
        }
    }

    #[test]
    fn test_parse_multiple_headers() {
        let doc = MarkdownParser::parse("# H1\n## H2\n### H3").unwrap();
        assert_eq!(doc.nodes.len(), 3);
    }

    #[test]
    fn test_parse_code_block() {
        let doc = MarkdownParser::parse("```rust\nfn main() {}\n```").unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::CodeBlock(cb) = &doc.nodes[0] {
            assert_eq!(cb.language, Some("rust".to_string()));
            assert!(cb.content.contains("fn main()"));
        } else {
            panic!("Expected code block");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let doc = MarkdownParser::parse("- Item 1\n- Item 2\n- Item 3").unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::List(l) = &doc.nodes[0] {
            assert!(!l.ordered);
            assert_eq!(l.items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = MarkdownParser::parse("1. First\n2. Second\n3. Third").unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::List(l) = &doc.nodes[0] {
            assert!(l.ordered);
            assert_eq!(l.items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_blockquote() {
        let doc = MarkdownParser::parse("> This is a quote").unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::SemanticBlock(sb) = &doc.nodes[0] {
            assert_eq!(sb.block_type, SemanticBlockType::Quote);
        } else {
            panic!("Expected semantic block");
        }
    }

    #[test]
    fn test_parse_table() {
        let input = "| Name | Age |\n|---|---|\n| Alice | 30 |";
        let doc = MarkdownParser::parse(input).unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::Table(t) = &doc.nodes[0] {
            assert_eq!(t.schema.len(), 2);
            assert_eq!(t.rows.len(), 1);
        } else {
            panic!("Expected table");
        }
    }

    #[test]
    fn test_parse_inline_bold() {
        let doc = MarkdownParser::parse("This is **bold** text").unwrap();
        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_bold = inlines.iter().any(|n| matches!(n, InlineNode::Bold(_)));
            assert!(has_bold);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_inline_italic() {
        let doc = MarkdownParser::parse("This is *italic* text").unwrap();
        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_italic = inlines.iter().any(|n| matches!(n, InlineNode::Italic(_)));
            assert!(has_italic);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_inline_code() {
        let doc = MarkdownParser::parse("Use `code` here").unwrap();
        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_code = inlines.iter().any(|n| matches!(n, InlineNode::Code(_)));
            assert!(has_code);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_link() {
        let doc = MarkdownParser::parse("See [docs](https://example.com)").unwrap();
        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_link = inlines.iter().any(|n| matches!(n, InlineNode::Link { .. }));
            assert!(has_link);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_image() {
        let doc = MarkdownParser::parse("![alt](https://example.com/img.png)").unwrap();
        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_image = inlines.iter().any(|n| matches!(n, InlineNode::Image { .. }));
            assert!(has_image);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_strikethrough() {
        let doc = MarkdownParser::parse("This is ~~deleted~~ text").unwrap();
        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_strike = inlines.iter().any(|n| matches!(n, InlineNode::Strikethrough(_)));
            assert!(has_strike);
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_to_markdown_header() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 2,
            content: vec![InlineNode::Text("Test".to_string())],
            priority: None,
        }));
        let md = to_markdown(&doc);
        assert!(md.contains("## Test"));
    }

    #[test]
    fn test_to_markdown_code_block() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::CodeBlock(CodeBlockNode {
            language: Some("rust".to_string()),
            content: "fn main() {}".to_string(),
            priority: None,
        }));
        let md = to_markdown(&doc);
        assert!(md.contains("```rust"));
        assert!(md.contains("fn main() {}"));
    }

    #[test]
    fn test_to_markdown_table() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Table(TableNode {
            schema: vec![
                ColumnDef {
                    name: "Name".to_string(),
                    type_hint: None,
                },
                ColumnDef {
                    name: "Age".to_string(),
                    type_hint: None,
                },
            ],
            rows: vec![vec![
                CellValue::Text("Alice".to_string()),
                CellValue::Integer(30),
            ]],
        }));
        let md = to_markdown(&doc);
        assert!(md.contains("| Name | Age |"));
        assert!(md.contains("| Alice | 30 |"));
    }

    #[test]
    fn test_round_trip_simple() {
        let input = "# Hello\n\nThis is a paragraph.";
        let doc = MarkdownParser::parse(input).unwrap();
        let output = to_markdown(&doc);
        assert!(output.contains("# Hello"));
        assert!(output.contains("This is a paragraph"));
    }

    #[test]
    fn test_task_list_parsing() {
        // GFM task list
        let doc = MarkdownParser::parse("- [ ] Todo\n- [x] Done").unwrap();
        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::List(l) = &doc.nodes[0] {
            assert_eq!(l.items.len(), 2);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_url_hoisting() {
        // URLs used multiple times should be auto-referenced
        let input = "See [link1](https://example.com) and [link2](https://example.com)";
        let doc = MarkdownParser::parse(input).unwrap();
        // Should have auto-generated reference for repeated URL
        assert!(doc.refs.values().any(|v| v == "https://example.com"));
    }
}
