//! Markdown beautifier for human-readable format.
//!
//! This module provides auto-formatting and linting for markdown files,
//! converting them into beautiful, human-readable format with:
//! - Plus-sign ASCII tables (responsive, properly aligned)
//! - Proper spacing and indentation
//! - Consistent heading styles
//! - Clean list formatting

use crate::error::CompileError;
use crate::table_renderer::{TableRenderer, TableRendererConfig};
use crate::types::{DxmDocument, DxmNode, TableNode};
use std::path::Path;

/// Markdown beautifier configuration
#[derive(Debug, Clone)]
pub struct BeautifierConfig {
    /// Use plus-sign ASCII tables (responsive)
    pub use_ascii_tables: bool,
    /// Maximum line width for wrapping
    pub max_line_width: usize,
    /// Indent size for nested lists
    pub indent_size: usize,
    /// Add blank lines between sections
    pub section_spacing: bool,
}

impl Default for BeautifierConfig {
    fn default() -> Self {
        Self {
            use_ascii_tables: true,
            max_line_width: 100,
            indent_size: 2,
            section_spacing: true,
        }
    }
}

/// Markdown beautifier
pub struct MarkdownBeautifier {
    config: BeautifierConfig,
}

impl Default for MarkdownBeautifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownBeautifier {
    /// Create a new beautifier with default configuration
    pub fn new() -> Self {
        Self {
            config: BeautifierConfig::default(),
        }
    }

    /// Create a beautifier with custom configuration
    pub fn with_config(config: BeautifierConfig) -> Self {
        Self { config }
    }

    /// Beautify markdown content
    pub fn beautify(&self, content: &str) -> Result<String, CompileError> {
        // Step 1: Auto-fix common issues (trailing whitespace, multiple blanks)
        let fixed = autofix_markdown(content);

        let lines: Vec<String> = fixed.lines().map(|s| s.to_string()).collect();

        // Step 2: Expand compact tables
        let expanded = expand_compact_tables(&lines.join("\n"));

        // Step 3: Convert tables to ASCII format
        let with_tables = convert_tables_to_ascii(&expanded);

        // Step 4: Fix links (add anchors to TOC, fix broken links)
        let with_links = fix_links(&with_tables);

        // Step 5: Process lines to add spacing around headers
        let table_lines: Vec<String> = with_links.lines().map(|s| s.to_string()).collect();

        let mut result: Vec<String> = Vec::new();
        let mut first_header = true;

        for i in 0..table_lines.len() {
            let line = &table_lines[i];

            // If this is a header (not the first one), add blank line BEFORE it
            if line.starts_with('#') {
                if !first_header && !result.is_empty() {
                    // Check if previous line is already blank
                    if let Some(last) = result.last()
                        && !last.is_empty()
                    {
                        result.push(String::new());
                    }
                }
                first_header = false;
            }

            // Always add the current line
            result.push(line.clone());

            // If this is a header, ensure exactly ONE blank line follows (if there's content after)
            if line.starts_with('#') && i + 1 < table_lines.len() {
                let next = &table_lines[i + 1];

                // If next line is NOT blank and NOT a header, add ONE blank
                if !next.is_empty() && !next.starts_with('#') {
                    result.push(String::new());
                }
            }
        }

        // Final pass: collapse any remaining multiple consecutive blanks to one
        let mut final_result = Vec::new();
        let mut prev_blank = false;

        for line in result {
            if line.is_empty() {
                if !prev_blank {
                    final_result.push(line);
                    prev_blank = true;
                }
            } else {
                final_result.push(line);
                prev_blank = false;
            }
        }

        Ok(final_result.join("\n") + "\n")
    }

    /// Beautify a markdown file and save to .human format
    pub fn beautify_file(&self, input_path: &Path, output_dir: &Path) -> Result<(), CompileError> {
        // Read input file
        let content =
            std::fs::read_to_string(input_path).map_err(|e| CompileError::io(e.to_string()))?;

        // Beautify content
        let beautified = self.beautify(&content)?;

        // Create output path with .human extension
        let relative_path = input_path.file_name().ok_or_else(|| {
            CompileError::io(format!("Invalid file path: {}", input_path.display()))
        })?;

        let output_path = output_dir.join(relative_path).with_extension("human");

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CompileError::io(e.to_string()))?;
        }

        // Write beautified content
        std::fs::write(&output_path, beautified).map_err(|e| CompileError::io(e.to_string()))?;

        Ok(())
    }

    /// Format a document to beautiful markdown
    fn format_document(&self, doc: &DxmDocument) -> Result<String, CompileError> {
        let mut output = String::new();

        for node in doc.nodes.iter() {
            output.push_str(&self.format_node(node)?);
        }

        Ok(output.trim().to_string())
    }

    /// Format a single node
    fn format_node(&self, node: &DxmNode) -> Result<String, CompileError> {
        match node {
            DxmNode::Header(header) => {
                let mut output = String::new();
                output.push_str(&"#".repeat(header.level as usize));
                output.push(' ');

                // Format inline content
                for inline in &header.content {
                    output.push_str(&self.format_inline(inline));
                }

                output.push('\n');
                Ok(output)
            }
            DxmNode::Paragraph(content) => {
                let mut output = String::new();
                for inline in content {
                    output.push_str(&self.format_inline(inline));
                }
                output.push('\n');
                Ok(output)
            }
            DxmNode::Table(table) => self.format_table(table),
            DxmNode::CodeBlock(code) => {
                let mut output = String::new();
                output.push_str("```");
                if let Some(lang) = &code.language {
                    output.push_str(lang);
                }
                output.push('\n');
                output.push_str(&code.content);
                if !code.content.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("```\n");
                Ok(output)
            }
            DxmNode::List(list) => {
                let mut output = String::new();
                for (i, item) in list.items.iter().enumerate() {
                    if list.ordered {
                        output.push_str(&format!("{}. ", i + 1));
                    } else {
                        output.push_str("- ");
                    }

                    for inline in &item.content {
                        output.push_str(&self.format_inline(inline));
                    }
                    output.push('\n');
                }
                Ok(output)
            }
            DxmNode::HorizontalRule => Ok("---\n".to_string()),
            DxmNode::SemanticBlock(block) => {
                let mut output = String::new();
                output.push_str(&format!("{:?}: ", block.block_type));
                for inline in &block.content {
                    output.push_str(&self.format_inline(inline));
                }
                output.push('\n');
                Ok(output)
            }
        }
    }

    /// Format inline content
    fn format_inline(&self, inline: &crate::types::InlineNode) -> String {
        use crate::types::InlineNode;

        match inline {
            InlineNode::Text(text) => text.clone(),
            InlineNode::Bold(content) => {
                let inner: String = content.iter().map(|n| self.format_inline(n)).collect();
                format!("**{}**", inner)
            }
            InlineNode::Italic(content) => {
                let inner: String = content.iter().map(|n| self.format_inline(n)).collect();
                format!("*{}*", inner)
            }
            InlineNode::Strikethrough(content) => {
                let inner: String = content.iter().map(|n| self.format_inline(n)).collect();
                format!("~~{}~~", inner)
            }
            InlineNode::Code(code) => format!("`{}`", code),
            InlineNode::Reference(key) => format!("^{}", key),
            InlineNode::Link { text, url, .. } => {
                let text_str: String = text.iter().map(|n| self.format_inline(n)).collect();
                if url.is_empty() {
                    text_str
                } else {
                    format!("[{}]({})", text_str, url)
                }
            }
            InlineNode::Image { alt, url, .. } => {
                format!("![{}]({})", alt, url)
            }
        }
    }

    /// Format a table with beautiful plus-sign ASCII borders
    fn format_table(&self, table: &TableNode) -> Result<String, CompileError> {
        let renderer = TableRenderer::with_config(TableRendererConfig {
            unicode: !self.config.use_ascii_tables,
            padding: 1,
            align_numbers_right: true,
        });

        let mut output = renderer.render(table);
        output.push('\n');
        Ok(output)
    }
}

/// Lint markdown content and return issues
pub fn lint_markdown(content: &str) -> Vec<String> {
    let mut issues = Vec::new();

    // Check for trailing whitespace
    for (i, line) in content.lines().enumerate() {
        if line.ends_with(' ') || line.ends_with('\t') {
            issues.push(format!("Line {}: Trailing whitespace", i + 1));
        }
    }

    // Check for multiple consecutive blank lines
    let lines: Vec<&str> = content.lines().collect();
    for i in 0..lines.len().saturating_sub(2) {
        if lines[i].trim().is_empty()
            && lines[i + 1].trim().is_empty()
            && lines[i + 2].trim().is_empty()
        {
            issues.push(format!("Line {}: More than 2 consecutive blank lines", i + 1));
        }
    }

    // Check for inconsistent heading styles
    let mut heading_style: Option<bool> = None; // true = ATX (#), false = Setext (===)
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('#') {
            if heading_style.is_none() {
                heading_style = Some(true);
            } else if heading_style == Some(false) {
                issues.push(format!(
                    "Line {}: Inconsistent heading style (mixing ATX and Setext)",
                    i + 1
                ));
            }
        }
    }

    issues
}

/// Add single blank lines between sections for readability
fn add_section_spacing(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();

    for i in 0..lines.len() {
        let line = lines[i];
        result.push(line.to_string());

        // Add blank line after headers if next line exists and is content (not blank, not header)
        if line.starts_with('#') && i + 1 < lines.len() {
            let next = lines[i + 1];
            // Only add blank if next line is NOT already blank and NOT a header
            if !next.trim().is_empty() && !next.starts_with('#') {
                result.push(String::new()); // Add blank line
            }
        }
    }

    // Join lines and ensure single trailing newline
    if result.is_empty() {
        String::new()
    } else {
        result.join("\n") + "\n"
    }
}

/// Auto-fix common markdown issues
pub fn autofix_markdown(content: &str) -> String {
    // Remove trailing whitespace from each line and split
    let lines: Vec<String> = content.lines().map(|line| line.trim_end().to_string()).collect();

    if lines.is_empty() {
        return String::new();
    }

    // Collapse multiple consecutive blank lines to exactly 1
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        if lines[i].trim().is_empty() {
            // Add one blank line
            result.push(String::new());
            // Skip all consecutive blank lines
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
        } else {
            // Add non-blank line
            result.push(lines[i].clone());
            i += 1;
        }
    }

    // Join and ensure single trailing newline
    if result.is_empty() {
        "\n".to_string()
    } else {
        result.join("\n") + "\n"
    }
}

/// Expand compact table format (t:N(...)[...]) to pipe tables
fn expand_compact_tables(content: &str) -> String {
    let mut result = String::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if line contains compact table start: t:N(...)[
        if line.contains("t:") && line.contains('(') && line.contains('[') {
            // Collect all lines until we find the closing ]
            let mut table_lines = vec![line];
            let mut j = i;

            // If the line doesn't contain ], keep collecting
            if !line.contains(']') {
                j = i + 1;
                while j < lines.len() {
                    table_lines.push(lines[j]);
                    if lines[j].contains(']') {
                        break;
                    }
                    j += 1;
                }
            }

            // Join the table lines and parse
            let table_str = table_lines.join("\n");
            if let Some(table) = parse_compact_table(&table_str) {
                result.push_str(&table);
                result.push('\n');
                // Move to the next line after the table
                i = j + 1;
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

/// Parse a compact table format like: t:3(Header_1 Header_2 Header_3)[\nRow1_Cell1 Row1_Cell2 Row1_Cell3\nRow2_Cell1 Row2_Cell2 Row2_Cell3]
/// Also handles comma-separated format: t:4(Header1,Header2,Header3,Header4)[Cell1,Cell2,Cell3,Cell4]
fn parse_compact_table(line: &str) -> Option<String> {
    // Find t:N(
    let t_pos = line.find("t:")?;
    let paren_pos = line[t_pos..].find('(')?;
    let row_count_str = &line[t_pos + 2..t_pos + paren_pos];
    let row_count: usize = row_count_str.parse().ok()?;

    // Extract headers between ( and )
    let headers_start = t_pos + paren_pos + 1;
    let headers_end = line[headers_start..].find(')')?;
    let headers_str = &line[headers_start..headers_start + headers_end];

    // Check if headers are comma-separated or space-separated
    let headers: Vec<String> = if headers_str.contains(',') {
        headers_str.split(',').map(|h| h.trim().to_string()).collect()
    } else {
        headers_str.split(' ').map(|h| h.replace('_', " ")).collect()
    };

    // Extract rows between [ and ]
    let rows_start = line[headers_start + headers_end..].find('[')?;
    let rows_start = headers_start + headers_end + rows_start + 1;
    let rows_end = line[rows_start..].find(']')?;
    let rows_str = &line[rows_start..rows_start + rows_end];

    // Parse rows - handle both comma-separated (single row) and newline-separated (multiple rows)
    let rows: Vec<Vec<String>> = if rows_str.contains('\n') {
        // Multiple rows separated by newlines, cells separated by spaces
        rows_str
            .lines()
            .map(|row_line| row_line.trim())
            .filter(|row_line| !row_line.is_empty())
            .map(|row_line| row_line.split(' ').map(|cell| cell.replace('_', " ")).collect())
            .collect()
    } else {
        // Single row with comma-separated cells
        let cells: Vec<String> = rows_str.split(',').map(|cell| cell.trim().to_string()).collect();

        // Check if we have exactly the right number of cells for one row
        if cells.len() == headers.len() {
            vec![cells]
        } else {
            // Try to split into multiple rows
            let mut result_rows = Vec::new();
            let mut current_row = Vec::new();
            for cell in cells {
                current_row.push(cell);
                if current_row.len() == headers.len() {
                    result_rows.push(current_row.clone());
                    current_row.clear();
                }
            }
            result_rows
        }
    };

    // Verify we got the expected number of rows (silently handle mismatches)
    // Empty tables or mismatched row counts are valid - just use what we got
    if rows.is_empty() && row_count > 0 {
        // Return None for empty tables - they'll be skipped
        return None;
    }

    // Build pipe table
    let mut table = String::new();

    // Header row
    table.push_str("| ");
    table.push_str(&headers.join(" | "));
    table.push_str(" |\n");

    // Separator row
    table.push('|');
    for _ in &headers {
        table.push_str(" --- |");
    }
    table.push('\n');

    // Data rows
    for row in rows {
        table.push_str("| ");
        table.push_str(&row.join(" | "));
        table.push_str(" |\n");
    }

    Some(table)
}

/// Convert pipe tables to plus-sign ASCII tables
fn convert_tables_to_ascii(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if this is already an ASCII table (starts with +---+ or +===+)
        if line.trim().starts_with('+') && (line.contains("---") || line.contains("===")) {
            // This is an ASCII table! Collect all table lines
            let mut table_lines = vec![];
            let mut j = i;
            while j < lines.len() {
                let tline = lines[j];
                if tline.trim().starts_with('+') || tline.trim().starts_with('|') {
                    table_lines.push(tline);
                    j += 1;
                } else {
                    break;
                }
            }

            // Convert ASCII table - just replace = with -
            for tline in table_lines {
                let fixed = tline.replace('=', "-");
                result.push_str(&fixed);
                result.push('\n');
            }

            i = j;
            continue;
        }

        // Check if this is a pipe table header (contains pipes)
        if line.contains('|') && line.trim().starts_with('|') {
            // Check if next line is separator (|---|---|)
            if i + 1 < lines.len() {
                let next_line = lines[i + 1];
                if next_line.contains("---") && next_line.contains('|') {
                    // This is a table! Collect all table rows
                    let mut table_lines = vec![line];
                    table_lines.push(next_line);

                    // Collect remaining table rows
                    let mut j = i + 2;
                    while j < lines.len() {
                        let row = lines[j];
                        if row.contains('|') && row.trim().starts_with('|') {
                            table_lines.push(row);
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    // Convert to plus-sign ASCII table
                    result.push_str(&convert_pipe_table_to_ascii(&table_lines));
                    result.push('\n');

                    i = j;
                    continue;
                }
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

/// Convert a pipe table to plus-sign ASCII format
fn convert_pipe_table_to_ascii(table_lines: &[&str]) -> String {
    if table_lines.len() < 2 {
        return table_lines.join("\n");
    }

    // Parse header - check if cells contain commas (indicating merged columns)
    let header = table_lines[0];
    let raw_headers: Vec<&str> =
        header.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    // Expand comma-separated headers into individual columns
    let mut headers: Vec<&str> = Vec::new();
    for h in &raw_headers {
        if h.contains(',') {
            // Split by comma and add each as separate column
            for part in h.split(',') {
                headers.push(part.trim());
            }
        } else {
            headers.push(h);
        }
    }

    // Parse data rows (skip separator line at index 1)
    let mut rows: Vec<Vec<&str>> = Vec::new();
    for line in table_lines.iter().skip(2) {
        let raw_cells: Vec<&str> =
            line.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

        // Expand comma-separated cells into individual columns
        let mut cells: Vec<&str> = Vec::new();
        for cell in &raw_cells {
            if cell.contains(',') {
                // Split by comma and add each as separate column
                for part in cell.split(',') {
                    cells.push(part.trim());
                }
            } else {
                cells.push(cell);
            }
        }

        if !cells.is_empty() {
            rows.push(cells);
        }
    }

    // Calculate column widths based on display width
    let mut col_widths: Vec<usize> = headers.iter().map(|h| display_width(h)).collect();
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                col_widths[i] = col_widths[i].max(display_width(cell));
            }
        }
    }

    // Build ASCII table
    let mut result = String::new();

    // Top border
    result.push('+');
    for width in &col_widths {
        result.push_str(&"-".repeat(width + 2));
        result.push('+');
    }
    result.push('\n');

    // Header row
    result.push('|');
    for (i, header) in headers.iter().enumerate() {
        result.push(' ');
        result.push_str(header);
        // Calculate padding: target width - actual display width
        let cell_display_width = display_width(header);
        let padding = col_widths[i].saturating_sub(cell_display_width);
        result.push_str(&" ".repeat(padding));
        result.push_str(" |");
    }
    result.push('\n');

    // Header separator
    result.push('+');
    for width in &col_widths {
        result.push_str(&"-".repeat(width + 2));
        result.push('+');
    }
    result.push('\n');

    // Data rows
    for row in &rows {
        result.push('|');
        for (i, cell) in row.iter().enumerate() {
            if i < col_widths.len() {
                result.push(' ');
                result.push_str(cell);
                // Calculate padding: target width - actual display width
                let cell_display_width = display_width(cell);
                let padding = col_widths[i].saturating_sub(cell_display_width);
                result.push_str(&" ".repeat(padding));
                result.push_str(" |");
            }
        }
        result.push('\n');
    }

    // Bottom border
    result.push('+');
    for width in &col_widths {
        result.push_str(&"-".repeat(width + 2));
        result.push('+');
    }

    result
}

/// Calculate display width of a string (accounting for emoji and wide characters)
fn display_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            // East Asian Width and emoji handling
            // Most terminals render these emojis as 2 columns wide
            match c {
            // Emoji presentation characters (always 2 wide in terminals)
            '✅' | '❌' | '✓' | '✗' | '☑' | '☒' => 2,
            // Full-width characters
            '\u{1100}'..='\u{115F}' | // Hangul Jamo
            '\u{2329}'..='\u{232A}' | // Angle brackets
            '\u{2E80}'..='\u{303E}' | // CJK Radicals
            '\u{3040}'..='\u{A4CF}' | // Hiragana, Katakana, etc
            '\u{AC00}'..='\u{D7A3}' | // Hangul Syllables
            '\u{F900}'..='\u{FAFF}' | // CJK Compatibility
            '\u{FE10}'..='\u{FE19}' | // Vertical forms
            '\u{FE30}'..='\u{FE6F}' | // CJK Compatibility Forms
            '\u{FF00}'..='\u{FF60}' | // Fullwidth Forms
            '\u{FFE0}'..='\u{FFE6}' | // Fullwidth Forms
            '\u{1F300}'..='\u{1F9FF}' | // Emoji
            '\u{20000}'..='\u{2FFFD}' | // CJK Extension
            '\u{30000}'..='\u{3FFFD}' => 2,
            // Zero-width characters
            '\u{0300}'..='\u{036F}' | // Combining marks
            '\u{1AB0}'..='\u{1AFF}' | // Combining marks
            '\u{1DC0}'..='\u{1DFF}' | // Combining marks
            '\u{20D0}'..='\u{20FF}' | // Combining marks
            '\u{FE20}'..='\u{FE2F}' => 0, // Combining half marks
            // Regular width
            _ => 1,
        }
        })
        .sum()
}

/// Fix links in markdown content
/// - Add anchor links to table of contents entries
/// - Fix broken/incomplete links
/// - Remove empty link targets
fn fix_links(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut headers: Vec<(String, String)> = Vec::new(); // (title, anchor)

    // First pass: collect all headers to build anchor map
    for line in &lines {
        if line.starts_with('#') {
            let level = line.chars().take_while(|&c| c == '#').count();
            let title = line[level..].trim().to_string();
            let anchor = title_to_anchor(&title);
            headers.push((title.clone(), anchor));
        }
    }

    // Second pass: fix links
    for line in lines {
        let mut fixed_line = line.to_string();

        // Fix table of contents links: [Text] -> [Text](#anchor)
        if fixed_line.trim().starts_with("- [")
            && fixed_line.contains(']')
            && !fixed_line.contains("](")
        {
            // Extract link text
            if let Some(start) = fixed_line.find('[')
                && let Some(end) = fixed_line.find(']')
                && end > start
            {
                let link_text = &fixed_line[start + 1..end];
                // Find matching header
                for (title, anchor) in &headers {
                    if title == link_text || title.to_lowercase() == link_text.to_lowercase() {
                        fixed_line = fixed_line.replace(
                            &format!("[{}]", link_text),
                            &format!("[{}](#{})", link_text, anchor),
                        );
                        break;
                    }
                }
            }
        }

        // Remove empty link targets: [](url) -> just remove them
        if fixed_line.contains("[](") {
            // Skip lines with empty link text
            continue;
        }

        result.push(fixed_line);
    }

    result.join("\n")
}

/// Convert a header title to an anchor ID
/// Example: "Text Formatting" -> "text-formatting"
fn title_to_anchor(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                // Skip special characters
                '\0'
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beautify_simple_markdown() {
        let beautifier = MarkdownBeautifier::new();
        let input = "# Hello\n\nThis is a test.";
        let result = beautifier.beautify(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lint_trailing_whitespace() {
        let content = "Hello world  \nGoodbye";
        let issues = lint_markdown(content);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("Trailing whitespace"));
    }

    #[test]
    fn test_lint_multiple_blank_lines() {
        let content = "Line 1\n\n\n\nLine 2";
        let issues = lint_markdown(content);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_autofix_trailing_whitespace() {
        let content = "Hello world  \nGoodbye  ";
        let fixed = autofix_markdown(content);
        assert!(!fixed.contains("  \n"));
    }

    #[test]
    fn test_autofix_multiple_blank_lines() {
        let content = "Line 1\n\n\n\n\nLine 2";
        let fixed = autofix_markdown(content);
        let blank_count = fixed.matches("\n\n\n").count();
        assert_eq!(blank_count, 0);
    }
}
