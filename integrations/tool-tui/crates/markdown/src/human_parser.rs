//! Human format parser for DXM documents.
//!
//! Parses the human-readable DXM format (with TOML-like sections,
//! Markdown-style headers, and Unicode tables) into a `DxmDocument` AST.

use crate::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::*;

/// Maximum input size (100MB)
const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Human format parser.
///
/// Parses the human-readable DXM format which includes:
/// - TOML-like sections ([meta], [refs])
/// - Markdown-style headers (# Title)
/// - Unicode box-drawing tables
/// - Semantic blocks (> [!WARNING])
pub struct HumanParser<'a> {
    /// Input string
    input: &'a str,
    /// Current position in input (byte offset)
    pos: usize,
    /// Current line number (1-indexed)
    line: usize,
    /// Current column number (1-indexed)
    col: usize,
    /// Collected errors
    errors: Vec<ParseError>,
}

impl<'a> HumanParser<'a> {
    /// Create a new human format parser.
    pub fn new(input: &'a str) -> ParseResult<Self> {
        if input.len() > MAX_INPUT_SIZE {
            return Err(ParseErrorKind::InputTooLarge {
                size: input.len(),
                max: MAX_INPUT_SIZE,
            }
            .into_parse_error());
        }

        Ok(Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
            errors: Vec::new(),
        })
    }

    /// Parse the input into a DxmDocument.
    pub fn parse(input: &'a str) -> ParseResult<DxmDocument> {
        let mut parser = Self::new(input)?;
        parser.parse_document()
    }

    /// Parse the entire document.
    fn parse_document(&mut self) -> ParseResult<DxmDocument> {
        let mut doc = DxmDocument::default();
        doc.meta.version = "1.0".to_string();

        self.skip_whitespace();

        // Parse sections and content
        let mut iterations = 0;
        while !self.is_eof() {
            iterations += 1;
            if iterations > 100_000 {
                return Err(ParseError::new(
                    "Parser exceeded iteration limit".to_string(),
                    self.line,
                    self.col,
                ));
            }

            let prev_pos = self.pos;

            self.skip_whitespace();

            if self.is_eof() {
                break;
            }

            // Check for section headers
            if self.peek_str("[meta]") {
                self.parse_meta_section(&mut doc)?;
            } else if self.peek_str("[refs]") {
                self.parse_refs_section(&mut doc)?;
            } else if let Some(node) = self.parse_block()? {
                doc.nodes.push(node);
            }

            // Safety: ensure we always advance to prevent infinite loops
            if self.pos == prev_pos && !self.is_eof() {
                self.advance();
            }
        }

        if !self.errors.is_empty() {
            return Err(self.errors.remove(0));
        }

        Ok(doc)
    }

    /// Parse the [meta] section.
    fn parse_meta_section(&mut self, doc: &mut DxmDocument) -> ParseResult<()> {
        self.skip_line(); // Skip [meta]

        while !self.is_eof() && !self.peek_str("[") && !self.is_content_start() {
            self.skip_whitespace_inline();

            if self.is_eof()
                || self.peek_str("[")
                || self.is_content_start()
                || self.peek() == Some('\n')
            {
                if self.peek() == Some('\n') {
                    self.advance();
                }
                continue;
            }

            // Check if this line looks like a key-value pair (contains =)
            let line = self.peek_line();
            if !line.contains('=') {
                // Not a key-value pair, stop parsing meta section
                break;
            }

            // Parse key = value
            let key = self.read_until_any(&['=', '\n']);
            let key = key.trim();

            if self.peek() == Some('=') {
                self.advance(); // skip =
                self.skip_whitespace_inline();
                let value = self.read_line();
                let value = value.trim();

                match key {
                    "version" => doc.meta.version = value.to_string(),
                    "tokens" => {
                        if let Ok(n) = value.parse() {
                            doc.meta.token_count = n;
                        }
                    }
                    _ => {} // Ignore unknown keys
                }
            } else {
                self.skip_line();
            }
        }

        Ok(())
    }

    /// Parse the [refs] section.
    fn parse_refs_section(&mut self, doc: &mut DxmDocument) -> ParseResult<()> {
        self.skip_line(); // Skip [refs]

        while !self.is_eof() && !self.peek_str("[") && !self.is_content_start() {
            self.skip_whitespace_inline();

            if self.is_eof() || self.peek_str("[") || self.peek() == Some('\n') {
                if self.peek() == Some('\n') {
                    self.advance();
                }
                continue;
            }

            // Parse key = value
            let key = self.read_until_any(&['=', '\n']);
            let key = key.trim().to_string();

            if self.peek() == Some('=') {
                self.advance(); // skip =
                self.skip_whitespace_inline();
                let value = self.read_line();
                let value = value.trim().to_string();

                if !key.is_empty() && !value.is_empty() {
                    doc.refs.insert(key, value);
                }
            } else {
                self.skip_line();
            }
        }

        Ok(())
    }

    /// Check if we're at the start of content (header, paragraph, etc.)
    fn is_content_start(&self) -> bool {
        self.peek() == Some('#') || self.peek() == Some('>') || self.peek() == Some('`')
            || self.peek() == Some('-') || self.peek() == Some('*')
            || self.peek() == Some('┌') // Unicode table start
            || self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false)
    }

    /// Parse a single block element.
    fn parse_block(&mut self) -> ParseResult<Option<DxmNode>> {
        self.skip_empty_lines();

        if self.is_eof() {
            return Ok(None);
        }

        // Markdown-style header (# Title)
        if self.peek() == Some('#') && !self.peek_str("#t(") {
            return self.parse_header().map(Some);
        }

        // Semantic block (> [!TYPE])
        if self.peek() == Some('>') {
            return self.parse_semantic_block().map(Some);
        }

        // Code block (```)
        if self.peek_str("```") {
            return self.parse_code_block().map(Some);
        }

        // Unicode table
        if self.peek() == Some('┌') {
            return self.parse_unicode_table().map(Some);
        }

        // ASCII table (| header |)
        if self.peek() == Some('|') {
            return self.parse_ascii_table().map(Some);
        }

        // Horizontal rule
        if self.peek_str("---") {
            self.skip_line();
            return Ok(Some(DxmNode::HorizontalRule));
        }

        // Unordered list (- item or * item)
        if self.peek() == Some('-') && self.peek_n(1) == Some(' ') {
            return self.parse_list(false).map(Some);
        }
        if self.peek() == Some('*') && self.peek_n(1) == Some(' ') {
            return self.parse_list(false).map(Some);
        }

        // Ordered list (1. item)
        if self.is_ordered_list_start() {
            return self.parse_list(true).map(Some);
        }

        // Default: paragraph
        self.parse_paragraph().map(Some)
    }

    /// Parse a Markdown-style header (# Title [!!!]).
    fn parse_header(&mut self) -> ParseResult<DxmNode> {
        let mut level = 0u8;

        // Count # characters
        while self.peek() == Some('#') && level < 6 {
            self.advance();
            level += 1;
        }

        // Skip space after #
        self.skip_whitespace_inline();

        // Read content
        let content_str = self.read_line();

        // Extract priority from [!!!], [!!], [!] suffix
        let (content_str, priority) = self.extract_priority_bracket(&content_str);

        let content = self.parse_inline_content(&content_str)?;

        Ok(DxmNode::Header(HeaderNode {
            level,
            content,
            priority,
        }))
    }

    /// Extract priority from bracket notation [!!!], [!!], [!].
    fn extract_priority_bracket(&self, s: &str) -> (String, Option<Priority>) {
        let s = s.trim_end();

        if s.ends_with("[!!!]") {
            (s[..s.len() - 5].trim_end().to_string(), Some(Priority::Critical))
        } else if s.ends_with("[!!]") {
            (s[..s.len() - 4].trim_end().to_string(), Some(Priority::Important))
        } else if s.ends_with("[!]") {
            (s[..s.len() - 3].trim_end().to_string(), Some(Priority::Low))
        } else {
            (s.to_string(), None)
        }
    }

    /// Parse a semantic block (> [!TYPE] content).
    fn parse_semantic_block(&mut self) -> ParseResult<DxmNode> {
        self.advance(); // skip >
        self.skip_whitespace_inline();

        // Check for type marker [!TYPE]
        let block_type = if self.peek_str("[!WARNING]") {
            self.advance_n(10);
            SemanticBlockType::Warning
        } else if self.peek_str("[!FAQ]") {
            self.advance_n(6);
            SemanticBlockType::FAQ
        } else if self.peek_str("[?FAQ]") {
            self.advance_n(6);
            SemanticBlockType::FAQ
        } else if self.peek_str("[!QUOTE]") {
            self.advance_n(8);
            SemanticBlockType::Quote
        } else if self.peek_str("[!INFO]") {
            self.advance_n(7);
            SemanticBlockType::Info
        } else if self.peek_str("[!EXAMPLE]") {
            self.advance_n(10);
            SemanticBlockType::Example
        } else {
            // Default to quote if no type marker
            SemanticBlockType::Quote
        };

        self.skip_line(); // Skip the type line

        // Read content lines (lines starting with >) but stop at new type markers
        let mut content_lines = Vec::new();
        while !self.is_eof() && self.peek() == Some('>') {
            // Peek ahead to check if this is a new semantic block type marker
            let line = self.peek_line();
            let trimmed = line.trim_start_matches('>').trim_start();
            if trimmed.starts_with("[!WARNING]")
                || trimmed.starts_with("[!FAQ]")
                || trimmed.starts_with("[?FAQ]")
                || trimmed.starts_with("[!QUOTE]")
                || trimmed.starts_with("[!INFO]")
                || trimmed.starts_with("[!EXAMPLE]")
            {
                // This is a new semantic block, stop here
                break;
            }

            self.advance(); // skip >
            self.skip_whitespace_inline();
            let line = self.read_line();
            content_lines.push(line);
        }

        let content_str = content_lines.join(" ");
        let content = self.parse_inline_content(&content_str)?;

        Ok(DxmNode::SemanticBlock(SemanticBlockNode {
            block_type,
            content,
            priority: None,
        }))
    }

    /// Parse a code block (```lang ... ```).
    fn parse_code_block(&mut self) -> ParseResult<DxmNode> {
        self.advance_n(3); // skip ```

        // Read language
        let language = self.read_line();
        let language = language.trim();
        let language = if language.is_empty() {
            None
        } else {
            Some(language.to_string())
        };

        // Read content until closing ```
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

    /// Parse a Unicode box-drawing table.
    fn parse_unicode_table(&mut self) -> ParseResult<DxmNode> {
        // Skip top border (┌───┬───┐)
        self.skip_line();

        // Parse header row (│ col1 │ col2 │)
        let header_line = self.read_line();
        let schema = self.parse_table_row(&header_line);

        // Skip header separator (├───┼───┤)
        self.skip_line();

        // Parse data rows
        let mut rows = Vec::new();
        while !self.is_eof() {
            let line = self.peek_line();

            // Check for bottom border (└───┴───┘)
            if line.starts_with('└') {
                self.skip_line();
                break;
            }

            // Check for row separator (├───┼───┤)
            if line.starts_with('├') {
                self.skip_line();
                continue;
            }

            // Parse data row
            if line.starts_with('│') {
                let row_line = self.read_line();
                let cells: Vec<CellValue> = self
                    .parse_table_row(&row_line)
                    .into_iter()
                    .map(|s| self.parse_cell_value(&s))
                    .collect();
                rows.push(cells);
            } else {
                break;
            }
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

    /// Parse a row from a Unicode table (│ cell │ cell │).
    fn parse_table_row(&self, line: &str) -> Vec<String> {
        line.split('│')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Parse a cell value, attempting to detect type.
    fn parse_cell_value(&self, s: &str) -> CellValue {
        let s = s.trim();

        if s.is_empty() {
            return CellValue::Null;
        }

        // Try integer
        if let Ok(i) = s.parse::<i64>() {
            return CellValue::Integer(i);
        }

        // Try float
        if let Ok(f) = s.parse::<f64>() {
            return CellValue::Float(f);
        }

        // Try boolean
        match s.to_lowercase().as_str() {
            "true" => return CellValue::Boolean(true),
            "false" => return CellValue::Boolean(false),
            _ => {}
        }

        CellValue::Text(s.to_string())
    }

    /// Parse an ASCII table (| header | header |).
    fn parse_ascii_table(&mut self) -> ParseResult<DxmNode> {
        // Parse header row
        let header_line = self.read_line();
        let schema: Vec<String> = header_line
            .split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect();

        // Skip separator (|---|---|)
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
            let cells: Vec<CellValue> = row_line
                .split('|')
                .filter(|s| !s.trim().is_empty())
                .map(|s| self.parse_cell_value(s))
                .collect();
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

    /// Parse a list (ordered or unordered).
    fn parse_list(&mut self, ordered: bool) -> ParseResult<DxmNode> {
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
                self.skip_whitespace_inline();
            } else {
                if self.peek() != Some('-') && self.peek() != Some('*') {
                    break;
                }
                self.advance(); // skip - or *
                self.skip_whitespace_inline();
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

    /// Check if we're at an ordered list start (N. ).
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

    /// Parse a paragraph.
    fn parse_paragraph(&mut self) -> ParseResult<DxmNode> {
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

    /// Check if we're at a block boundary.
    fn is_block_boundary(&self) -> bool {
        if self.is_eof() {
            return true;
        }

        let c = self.peek().unwrap_or('\0');

        // Section headers
        if c == '[' {
            return true;
        }

        // Markdown headers
        if c == '#' {
            return true;
        }

        // Semantic blocks
        if c == '>' {
            return true;
        }

        // Code blocks
        if self.peek_str("```") {
            return true;
        }

        // Tables
        if c == '┌' || c == '|' {
            return true;
        }

        // Horizontal rule
        if self.peek_str("---") {
            return true;
        }

        // Lists
        if (c == '-' || c == '*') && self.peek_n(1) == Some(' ') {
            return true;
        }

        if self.is_ordered_list_start() {
            return true;
        }

        false
    }

    /// Parse inline content from a string.
    fn parse_inline_content(&mut self, s: &str) -> ParseResult<Vec<InlineNode>> {
        let mut nodes = Vec::new();
        let mut current_text = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Check for reference usage [^key]
            if c == '[' && i + 1 < chars.len() && chars[i + 1] == '^' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 2; // skip [^
                let key: String = chars[i..].iter().take_while(|c| **c != ']').collect();
                i += key.len();
                if i < chars.len() && chars[i] == ']' {
                    i += 1; // skip ]
                }
                nodes.push(InlineNode::Reference(key));
                continue;
            }

            // Check for bold (**text**)
            if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 2; // skip **
                // Find closing **
                let mut end = i;
                while end + 1 < chars.len() {
                    if chars[end] == '*' && chars[end + 1] == '*' {
                        break;
                    }
                    end += 1;
                }
                let inner: String = chars[i..end].iter().collect();
                i = end + 2; // skip closing **
                nodes.push(InlineNode::Bold(vec![InlineNode::Text(inner)]));
                continue;
            }

            // Check for italic (*text*)
            if c == '*' && i + 1 < chars.len() && chars[i + 1] != '*' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 1; // skip *
                let mut end = i;
                while end < chars.len() && chars[end] != '*' {
                    end += 1;
                }
                let inner: String = chars[i..end].iter().collect();
                i = end + 1; // skip closing *
                nodes.push(InlineNode::Italic(vec![InlineNode::Text(inner)]));
                continue;
            }

            // Check for strikethrough (~~text~~)
            if c == '~' && i + 1 < chars.len() && chars[i + 1] == '~' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 2; // skip ~~
                let mut end = i;
                while end + 1 < chars.len() {
                    if chars[end] == '~' && chars[end + 1] == '~' {
                        break;
                    }
                    end += 1;
                }
                let inner: String = chars[i..end].iter().collect();
                i = end + 2; // skip closing ~~
                nodes.push(InlineNode::Strikethrough(vec![InlineNode::Text(inner)]));
                continue;
            }

            // Check for inline code (`code`)
            if c == '`' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 1; // skip `
                let mut end = i;
                while end < chars.len() && chars[end] != '`' {
                    end += 1;
                }
                let code: String = chars[i..end].iter().collect();
                i = end + 1; // skip closing `
                nodes.push(InlineNode::Code(code));
                continue;
            }

            // Check for link [text](url)
            if c == '[' && !self.peek_str_at(&chars, i, "[^") {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 1; // skip [
                let mut end = i;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }
                let text: String = chars[i..end].iter().collect();
                i = end + 1; // skip ]

                if i < chars.len() && chars[i] == '(' {
                    i += 1; // skip (
                    let mut url_end = i;
                    while url_end < chars.len() && chars[url_end] != ')' {
                        url_end += 1;
                    }
                    let url: String = chars[i..url_end].iter().collect();
                    i = url_end + 1; // skip )

                    nodes.push(InlineNode::Link {
                        text: vec![InlineNode::Text(text)],
                        url,
                        title: None,
                    });
                } else {
                    // Not a link, just text
                    current_text.push('[');
                    current_text.push_str(&text);
                    current_text.push(']');
                }
                continue;
            }

            // Check for image ![alt](url)
            if c == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
                if !current_text.is_empty() {
                    nodes.push(InlineNode::Text(std::mem::take(&mut current_text)));
                }
                i += 2; // skip ![
                let mut end = i;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }
                let alt: String = chars[i..end].iter().collect();
                i = end + 1; // skip ]

                if i < chars.len() && chars[i] == '(' {
                    i += 1; // skip (
                    let mut url_end = i;
                    while url_end < chars.len() && chars[url_end] != ')' {
                        url_end += 1;
                    }
                    let url: String = chars[i..url_end].iter().collect();
                    i = url_end + 1; // skip )

                    nodes.push(InlineNode::Image {
                        alt,
                        url,
                        title: None,
                    });
                } else {
                    current_text.push_str("![");
                    current_text.push_str(&alt);
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

    /// Check if chars at position start with the given string.
    fn peek_str_at(&self, chars: &[char], pos: usize, s: &str) -> bool {
        let s_chars: Vec<char> = s.chars().collect();
        if pos + s_chars.len() > chars.len() {
            return false;
        }
        chars[pos..pos + s_chars.len()] == s_chars[..]
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
    fn peek_line(&self) -> String {
        self.input[self.pos..].lines().next().unwrap_or("").to_string()
    }

    /// Advance by one character.
    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    /// Advance by n characters.
    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    /// Read until any of the given characters is found.
    fn read_until_any(&mut self, delimiters: &[char]) -> String {
        let start = self.pos;
        while !self.is_eof() {
            if let Some(c) = self.peek()
                && delimiters.contains(&c)
            {
                break;
            }
            self.advance();
        }
        self.input[start..self.pos].to_string()
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

    /// Skip whitespace (spaces, tabs, newlines).
    fn skip_whitespace(&mut self) {
        while !self.is_eof() {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') | Some('\n') => self.advance(),
                _ => break,
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_document() {
        let doc = HumanParser::parse("").unwrap();
        assert!(doc.nodes.is_empty());
    }

    #[test]
    fn test_parse_meta_section() {
        let input = "[meta]\nversion = 2.0\ntokens = 1234\n";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.meta.version, "2.0");
        assert_eq!(doc.meta.token_count, 1234);
    }

    #[test]
    fn test_parse_refs_section() {
        let input = "[refs]\ndoc = https://docs.example.com\nrepo = https://github.com/example\n";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.refs.get("doc"), Some(&"https://docs.example.com".to_string()));
        assert_eq!(doc.refs.get("repo"), Some(&"https://github.com/example".to_string()));
    }

    #[test]
    fn test_parse_markdown_header() {
        let input = "# Main Title\n## Section\n### Subsection";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.nodes.len(), 3);

        if let DxmNode::Header(h) = &doc.nodes[0] {
            assert_eq!(h.level, 1);
        } else {
            panic!("Expected header");
        }

        if let DxmNode::Header(h) = &doc.nodes[1] {
            assert_eq!(h.level, 2);
        } else {
            panic!("Expected header");
        }

        if let DxmNode::Header(h) = &doc.nodes[2] {
            assert_eq!(h.level, 3);
        } else {
            panic!("Expected header");
        }
    }

    #[test]
    fn test_parse_header_with_priority() {
        let input = "# Critical Section [!!!]\n## Important Section [!!]\n### Low Priority [!]";
        let doc = HumanParser::parse(input).unwrap();

        if let DxmNode::Header(h) = &doc.nodes[0] {
            assert_eq!(h.priority, Some(Priority::Critical));
        } else {
            panic!("Expected header");
        }

        if let DxmNode::Header(h) = &doc.nodes[1] {
            assert_eq!(h.priority, Some(Priority::Important));
        } else {
            panic!("Expected header");
        }

        if let DxmNode::Header(h) = &doc.nodes[2] {
            assert_eq!(h.priority, Some(Priority::Low));
        } else {
            panic!("Expected header");
        }
    }

    #[test]
    fn test_parse_code_block() {
        let input = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::CodeBlock(cb) = &doc.nodes[0] {
            assert_eq!(cb.language, Some("rust".to_string()));
            assert!(cb.content.contains("fn main()"));
        } else {
            panic!("Expected code block");
        }
    }

    #[test]
    fn test_parse_semantic_block() {
        let input = "> [!WARNING]\n> Be careful!";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::SemanticBlock(sb) = &doc.nodes[0] {
            assert_eq!(sb.block_type, SemanticBlockType::Warning);
        } else {
            panic!("Expected semantic block");
        }
    }

    #[test]
    fn test_parse_unicode_table() {
        let input =
            "┌─────┬───────┐\n│ id  │ name  │\n├─────┼───────┤\n│ 1   │ Alice │\n└─────┴───────┘";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::Table(t) = &doc.nodes[0] {
            assert_eq!(t.schema.len(), 2);
            assert_eq!(t.schema[0].name, "id");
            assert_eq!(t.schema[1].name, "name");
            assert_eq!(t.rows.len(), 1);
        } else {
            panic!("Expected table");
        }
    }

    #[test]
    fn test_parse_unordered_list() {
        let input = "- First item\n- Second item\n- Third item";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::List(list) = &doc.nodes[0] {
            assert!(!list.ordered);
            assert_eq!(list.items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_ordered_list() {
        let input = "1. First item\n2. Second item\n3. Third item";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.nodes.len(), 1);
        if let DxmNode::List(list) = &doc.nodes[0] {
            assert!(list.ordered);
            assert_eq!(list.items.len(), 3);
        } else {
            panic!("Expected list");
        }
    }

    #[test]
    fn test_parse_inline_bold() {
        let input = "This is **bold** text";
        let doc = HumanParser::parse(input).unwrap();

        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_bold = inlines.iter().any(|n| matches!(n, InlineNode::Bold(_)));
            assert!(has_bold, "Should contain bold text");
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_inline_code() {
        let input = "Use `code` here";
        let doc = HumanParser::parse(input).unwrap();

        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_code = inlines.iter().any(|n| matches!(n, InlineNode::Code(_)));
            assert!(has_code, "Should contain inline code");
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_link() {
        let input = "See [docs](https://example.com) for details";
        let doc = HumanParser::parse(input).unwrap();

        if let DxmNode::Paragraph(inlines) = &doc.nodes[0] {
            let has_link = inlines.iter().any(|n| matches!(n, InlineNode::Link { .. }));
            assert!(has_link, "Should contain link");
        } else {
            panic!("Expected paragraph");
        }
    }

    #[test]
    fn test_parse_full_document() {
        let input = r#"[meta]
version = 1.0
tokens = 500

[refs]
doc = https://docs.example.com

# Main Title [!!!]

This is a paragraph with **bold** and *italic* text.

```rust
fn main() {}
```

> [!WARNING]
> Be careful!
"#;
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.meta.version, "1.0");
        assert_eq!(doc.meta.token_count, 500);
        assert!(doc.refs.contains_key("doc"));
        assert!(doc.nodes.len() >= 4);
    }

    #[test]
    fn test_parse_meta_then_header() {
        // This is the exact format produced by HumanFormatter
        let input = "[meta]\nversion = 1.0\n\n# A\n";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.meta.version, "1.0");
        assert_eq!(doc.nodes.len(), 1, "Expected 1 node, got {:?}", doc.nodes);
        if let DxmNode::Header(h) = &doc.nodes[0] {
            assert_eq!(h.level, 1);
        } else {
            panic!("Expected header, got {:?}", doc.nodes[0]);
        }
    }

    #[test]
    fn test_parse_meta_only_then_header() {
        // Simpler case - meta section followed by header
        let input = "[meta]\nversion = 1.0\n# Test\n";
        let doc = HumanParser::parse(input).unwrap();

        assert_eq!(doc.meta.version, "1.0");
        assert_eq!(doc.nodes.len(), 1, "Expected 1 node, got {:?}", doc.nodes);
    }
}
