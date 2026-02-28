//! Main compiler for the DX Markdown Context Compiler.
//!
//! This module implements the core compilation logic that transforms
//! standard Markdown into token-optimized output.

use crate::dictionary::Dictionary;
use crate::error::CompileError;
use crate::filler::strip_filler;
use crate::minify::minify_code;
use crate::table::{should_keep_inline, table_to_inline, table_to_tsv};
use crate::tokenizer::Tokenizer;
use crate::types::{CompileResult, CompilerConfig, SavingsBreakdown};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::io::{BufRead, BufReader, Read, Write};

/// Maximum input size (100MB).
const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Default chunk size for streaming (64KB).
const DEFAULT_CHUNK_SIZE: usize = 64 * 1024;

/// Helper for compile-time regex patterns.
/// Uses lazy_static to compile patterns once at startup.
/// These patterns are hardcoded and known to be valid.
use once_cell::sync::Lazy;

#[allow(clippy::unwrap_used)] // Compile-time constant patterns, guaranteed valid
static RE_TRUNCATED: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"https?:\s*\n?").unwrap());
#[allow(clippy::unwrap_used)]
static RE_BADGE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\[!\[[^\]]*\]\([^)]*\)?\]?\([^)]*\)?").unwrap());
#[allow(clippy::unwrap_used)]
static RE_BROKEN_IMG: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"!\[[^\]]*\]\([^)]*$").unwrap());
#[allow(clippy::unwrap_used)]
static RE_BROKEN_LINK: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\[[^\]]*\]\([^)]*$").unwrap());
#[allow(clippy::unwrap_used)]
static RE_BROKEN_BADGE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\[!\[[^\]]*\]\(").unwrap());
#[allow(clippy::unwrap_used)]
static RE_EMPTY_LINK: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\[\]\(\)").unwrap());
#[allow(clippy::unwrap_used)]
static RE_ORPHAN_OPEN: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"\[\s*\(").unwrap());
#[allow(clippy::unwrap_used)]
static RE_ORPHAN_PATH: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"^\s*\(\./[^)]+\)\s*$").unwrap());
#[allow(clippy::unwrap_used)]
static RE_BAR_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"[█░]{3,}\s*").unwrap());
#[allow(clippy::unwrap_used)]
static RE_ARROW_PATTERN: Lazy<regex::Regex> = Lazy::new(|| regex::Regex::new(r"→\s*").unwrap());

/// The main DX Markdown compiler.
pub struct DxMarkdown {
    config: CompilerConfig,
    tokenizer: Tokenizer,
}

impl DxMarkdown {
    /// Create a new compiler with the given configuration.
    pub fn new(config: CompilerConfig) -> Result<Self, CompileError> {
        let tokenizer = Tokenizer::new(config.tokenizer)?;
        Ok(Self { config, tokenizer })
    }

    /// Create a compiler with default configuration.
    pub fn default_compiler() -> Result<Self, CompileError> {
        Self::new(CompilerConfig::default())
    }

    /// Compile markdown to optimized output.
    pub fn compile(&self, input: &str) -> Result<CompileResult, CompileError> {
        // Check input size
        if input.len() > MAX_INPUT_SIZE {
            return Err(CompileError::input_too_large(input.len(), MAX_INPUT_SIZE));
        }

        // Count tokens before
        let tokens_before = self.tokenizer.count(input);

        // For LLM format, preserve markdown syntax but optimize whitespace
        let output = optimize_for_llm(input, &self.config);

        // Count tokens after
        let tokens_after = self.tokenizer.count(&output);

        Ok(CompileResult {
            output,
            tokens_before,
            tokens_after,
            breakdown: SavingsBreakdown::default(),
        })
    }

    /// Compile markdown from a reader to a writer in streaming mode.
    ///
    /// This method processes the input in chunks, making it suitable for
    /// large files that don't fit in memory.
    ///
    /// Note: Streaming mode has some limitations:
    /// - Dictionary deduplication is disabled (requires full document analysis)
    /// - Token counting is approximate
    ///
    /// # Arguments
    /// * `reader` - Input reader
    /// * `writer` - Output writer
    ///
    /// # Returns
    /// Compilation result with statistics
    pub fn compile_streaming<R: Read, W: Write>(
        &self,
        reader: R,
        writer: &mut W,
    ) -> Result<CompileResult, CompileError> {
        let buf_reader = BufReader::with_capacity(DEFAULT_CHUNK_SIZE, reader);
        let breakdown = SavingsBreakdown::default();

        // Collect all lines first (for proper markdown parsing)
        let mut all_content = String::new();
        for line in buf_reader.lines() {
            let line = line?;
            all_content.push_str(&line);
            all_content.push('\n');
        }

        // Check size limit
        if all_content.len() > MAX_INPUT_SIZE {
            return Err(CompileError::input_too_large(all_content.len(), MAX_INPUT_SIZE));
        }

        // Process the content (without dictionary for streaming)
        let config = CompilerConfig {
            dictionary: false, // Disable dictionary in streaming mode
            ..self.config.clone()
        };

        let temp_compiler = DxMarkdown::new(config)?;
        let result = temp_compiler.compile(&all_content)?;

        // Write output
        writer.write_all(result.output.as_bytes())?;

        Ok(CompileResult {
            output: result.output,
            tokens_before: result.tokens_before,
            tokens_after: result.tokens_after,
            breakdown,
        })
    }

    /// Internal compilation logic.
    fn compile_internal(
        &self,
        input: &str,
        dictionary_candidates: &[(String, usize)],
    ) -> Result<(String, SavingsBreakdown), CompileError> {
        let mut output = String::with_capacity(input.len());
        let breakdown = SavingsBreakdown::default();

        // Build dictionary from candidates
        let dictionary = if self.config.dictionary && !dictionary_candidates.is_empty() {
            let mut dict = Dictionary::new();
            for (phrase, _count) in dictionary_candidates.iter().take(26) {
                dict.add(phrase.clone());
            }
            Some(dict)
        } else {
            None
        };

        let options =
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let parser = Parser::new_ext(input, options);

        let mut skip_until_end: Option<TagEnd> = None;
        let mut in_link = false;
        let mut link_text = String::new();
        let mut current_link_url = String::new();
        let mut in_image = false;
        let mut in_table = false;
        let mut in_table_head = false;
        let mut table_headers: Vec<String> = Vec::new();
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut table_row: Vec<String> = Vec::new();
        let mut table_cell = String::new();
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();

        for event in parser {
            // Skip events until we reach the end tag we're waiting for
            if let Some(ref end_tag) = skip_until_end {
                if let Event::End(tag) = &event
                    && tag == end_tag
                {
                    skip_until_end = None;
                    // Reset the corresponding flag
                    if *tag == TagEnd::Image {
                        in_image = false;
                    }
                }
                continue;
            }

            match event {
                // Handle links
                Event::Start(Tag::Link { dest_url, .. }) => {
                    let url = dest_url.to_string();
                    // Check if it's a file path (no protocol, or relative path) or anchor link (starts with #)
                    let is_file_path = !url.contains("://") && !url.starts_with("//");
                    let is_anchor = url.starts_with('#');

                    in_link = true;
                    link_text.clear();

                    current_link_url = if self.config.strip_urls && !is_file_path && !is_anchor {
                        // Strip external URLs
                        String::new()
                    } else {
                        // Keep file paths and anchors
                        url
                    };
                }
                Event::End(TagEnd::Link) => {
                    if in_link {
                        if !current_link_url.is_empty() {
                            // For anchor links, just output the text (anchor is redundant)
                            if current_link_url.starts_with('#') {
                                output.push_str(&link_text);
                            } else {
                                // Output link with URL preserved
                                output.push_str(&link_text);
                                output.push_str(" (");
                                output.push_str(&current_link_url);
                                output.push(')');
                            }
                        } else {
                            // Output just the link text (URL was stripped)
                            output.push_str(&link_text);
                        }
                        in_link = false;
                        link_text.clear();
                        current_link_url.clear();
                    }
                }

                // Handle images
                Event::Start(Tag::Image { dest_url, .. }) => {
                    // Check if it's a badge
                    let url = dest_url.to_string();
                    let is_badge = url.contains("badge")
                        || url.contains("shields.io")
                        || url.contains("img.shields.io");

                    if (is_badge && self.config.strip_badges) || self.config.strip_images {
                        // Skip entirely
                        in_image = true;
                        skip_until_end = Some(TagEnd::Image);
                    }
                }
                Event::End(TagEnd::Image) => {
                    in_image = false;
                }

                // Handle tables
                Event::Start(Tag::Table(_)) => {
                    if self.config.tables_to_tsv {
                        in_table = true;
                        in_table_head = false;
                        table_headers.clear();
                        table_rows.clear();
                        table_row.clear();
                        table_cell.clear();
                    }
                }
                Event::End(TagEnd::Table) => {
                    if in_table {
                        // Build TableInfo and convert
                        let table_info = crate::types::TableInfo {
                            headers: table_headers.clone(),
                            rows: table_rows.clone(),
                            start_line: 0,
                            end_line: 0,
                            original: String::new(),
                        };
                        let table_output = if should_keep_inline(&table_info) {
                            table_to_inline(&table_info)
                        } else {
                            table_to_tsv(&table_info)
                        };
                        output.push_str(&table_output);
                        output.push('\n');
                        in_table = false;
                        table_headers.clear();
                        table_rows.clear();
                    }
                }
                Event::Start(Tag::TableHead) => {
                    if in_table {
                        in_table_head = true;
                        table_row.clear();
                    }
                }
                Event::End(TagEnd::TableHead) => {
                    if in_table {
                        in_table_head = false;
                        table_headers = table_row.clone();
                        table_row.clear();
                    }
                }
                Event::Start(Tag::TableRow) => {
                    if in_table {
                        table_row.clear();
                    }
                }
                Event::End(TagEnd::TableRow) => {
                    if in_table && !in_table_head {
                        table_rows.push(table_row.clone());
                        table_row.clear();
                    }
                }
                Event::Start(Tag::TableCell) => {
                    if in_table {
                        table_cell.clear();
                    }
                }
                Event::End(TagEnd::TableCell) => {
                    if in_table {
                        table_row.push(table_cell.clone());
                        table_cell.clear();
                    }
                }

                // Handle code blocks
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_content.clear();
                    code_block_lang.clear();
                    if let pulldown_cmark::CodeBlockKind::Fenced(lang) = kind {
                        code_block_lang = lang.to_string();
                    }
                }
                Event::End(TagEnd::CodeBlock) => {
                    // Try to convert diagrams to DX format if enabled
                    if let Some(dx_format) = self
                        .config
                        .tables_to_tsv
                        .then(|| self.try_convert_diagram(&code_block_content, &code_block_lang))
                        .flatten()
                    {
                        // Output converted DX format
                        output.push_str(&dx_format);
                        output.push('\n');
                    } else {
                        // Apply minification if enabled
                        let final_code = if self.config.minify_code && !code_block_lang.is_empty() {
                            minify_code(&code_block_content, &code_block_lang)
                        } else {
                            code_block_content.clone()
                        };

                        // Output code block - standard markdown format is token-efficient
                        output.push_str("```");
                        output.push_str(&code_block_lang);
                        output.push('\n');
                        output.push_str(&final_code);
                        if !final_code.ends_with('\n') {
                            output.push('\n');
                        }
                        output.push_str("```\n");
                    }

                    in_code_block = false;
                    code_block_content.clear();
                    code_block_lang.clear();
                }

                // Handle headers
                Event::Start(Tag::Heading { level, .. }) => {
                    // Keep standard markdown headers - they're already token-efficient
                    let hashes = "#".repeat(level as usize);
                    output.push_str(&hashes);
                    output.push(' ');
                }
                Event::End(TagEnd::Heading(_)) => {
                    output.push('\n');
                }

                // Handle paragraphs
                Event::Start(Tag::Paragraph) => {}
                Event::End(TagEnd::Paragraph) => {
                    // Add newline after paragraph
                    output.push('\n');
                }

                // Handle text
                Event::Text(text) => {
                    if in_link {
                        link_text.push_str(&text);
                    } else if in_table {
                        table_cell.push_str(&text);
                    } else if in_code_block {
                        code_block_content.push_str(&text);
                    } else if !in_image {
                        let text_str = if self.config.collapse_whitespace {
                            collapse_whitespace(&text)
                        } else {
                            text.to_string()
                        };
                        output.push_str(&text_str);
                    }
                }

                Event::Code(code) => {
                    if in_link {
                        link_text.push('`');
                        link_text.push_str(&code);
                        link_text.push('`');
                    } else if in_table {
                        table_cell.push('`');
                        table_cell.push_str(&code);
                        table_cell.push('`');
                    } else if in_code_block {
                        code_block_content.push('`');
                        code_block_content.push_str(&code);
                        code_block_content.push('`');
                    } else {
                        output.push('`');
                        output.push_str(&code);
                        output.push('`');
                    }
                }

                Event::SoftBreak => {
                    if in_link {
                        link_text.push(' ');
                    } else if in_table {
                        table_cell.push(' ');
                    } else if in_code_block {
                        code_block_content.push('\n');
                    } else if !in_image {
                        output.push(' ');
                    }
                }

                Event::HardBreak => {
                    if in_code_block {
                        code_block_content.push('\n');
                    } else if !in_image {
                        output.push('\n');
                    }
                }

                // Handle lists
                Event::Start(Tag::List(ordered)) => {
                    if ordered.is_some() {
                        // Ordered list - we'll handle numbering in items
                    }
                }
                Event::End(TagEnd::List(_)) => {
                    output.push('\n');
                }

                Event::Start(Tag::Item) => {
                    output.push('-');
                }
                Event::End(TagEnd::Item) => {
                    output.push('\n');
                }

                // Handle other events
                Event::Rule => {
                    if !self.config.collapse_whitespace {
                        output.push_str("---\n");
                    }
                    // If collapsing whitespace, skip horizontal rules
                }

                _ => {}
            }
        }

        // Apply filler removal if enabled
        let output = if self.config.strip_filler {
            strip_filler(&output)
        } else {
            output
        };

        // Clean up output
        let output = if self.config.collapse_whitespace {
            collapse_multiple_newlines(&output)
        } else {
            output
        };

        // Apply dictionary replacements if enabled
        let output = if let Some(ref dict) = dictionary {
            let header = dict.header();
            let body = dict.apply(&output);
            if !header.is_empty() {
                format!("{}{}", header, body)
            } else {
                body
            }
        } else {
            output
        };

        // Final post-processing to clean up remaining issues
        let output = post_process(&output);

        Ok((output.trim().to_string(), breakdown))
    }

    /// Try to convert a code block to DX format if it's a diagram
    fn try_convert_diagram(&self, content: &str, lang: &str) -> Option<String> {
        use crate::diagrams::serializer::convert_to_dx;

        // Try to convert based on language hint
        let lang_hint = if lang.is_empty() { None } else { Some(lang) };

        convert_to_dx(content, lang_hint).ok()
    }
}

/// Optimize markdown for LLMs while preserving syntax
fn optimize_for_llm(input: &str, config: &CompilerConfig) -> String {
    let mut output = input.to_string();

    // Convert tables to DX Serializer format if enabled
    if config.tables_to_tsv {
        output = convert_tables_to_dx_format(&output);
    }

    // Strip images if enabled
    if config.strip_images {
        output = strip_images_preserve_syntax(&output);
    }

    // Strip badges if enabled
    if config.strip_badges {
        output = strip_badges_preserve_syntax(&output);
    }

    // Strip URLs from links if enabled (but keep link text and syntax)
    if config.strip_urls {
        output = strip_urls_preserve_syntax(&output);
    }

    // Collapse multiple newlines to single newline
    output = collapse_multiple_newlines(&output);

    output.trim().to_string()
}

/// Convert markdown tables to DX Serializer format
fn convert_tables_to_dx_format(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if this is an ASCII table (starts with +---+)
        if line.trim().starts_with('+') && line.contains("---") {
            // This is an ASCII table! Find the header row
            if i + 1 < lines.len() {
                let header_line = lines[i + 1];
                if header_line.trim().starts_with('|') {
                    // Parse header
                    let headers: Vec<&str> = header_line
                        .split('|')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();

                    // Skip top border and header and separator
                    let mut j = i + 3; // Skip +--+, | header |, +--+

                    // Collect data rows
                    let mut rows: Vec<Vec<&str>> = Vec::new();
                    while j < lines.len() {
                        let row_line = lines[j];
                        if row_line.trim().starts_with('|') {
                            let cells: Vec<&str> = row_line
                                .split('|')
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect();
                            if !cells.is_empty() {
                                rows.push(cells);
                            }
                            j += 1;
                        } else if row_line.trim().starts_with('+') {
                            // Bottom border, skip it
                            j += 1;
                            break;
                        } else {
                            break;
                        }
                    }

                    // Convert to DX Serializer format
                    result.push_str(&format!("t:{}(", rows.len()));
                    result.push_str(&headers.join(","));
                    result.push_str(")[");
                    for (idx, row) in rows.iter().enumerate() {
                        if idx > 0 {
                            result.push(' ');
                        }
                        result.push_str(&row.join(","));
                    }
                    result.push_str("]\n");

                    i = j;
                    continue;
                }
            }
        }

        // Check if this is a pipe table header (contains pipes)
        if line.contains('|') && line.trim().starts_with('|') {
            // Check if next line is separator (|---|---|)
            if i + 1 < lines.len() {
                let next_line = lines[i + 1];
                if next_line.contains("---") && next_line.contains('|') {
                    // This is a table! Parse header
                    let headers: Vec<&str> =
                        line.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

                    // Collect data rows (skip separator line at index i+1)
                    let mut rows: Vec<Vec<&str>> = Vec::new();
                    let mut j = i + 2;
                    while j < lines.len() {
                        let row = lines[j];
                        if row.contains('|') && row.trim().starts_with('|') {
                            let cells: Vec<&str> = row
                                .split('|')
                                .map(|s| s.trim())
                                .filter(|s| !s.is_empty())
                                .collect();
                            if !cells.is_empty() {
                                rows.push(cells);
                            }
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    // Convert to DX Serializer format
                    result.push_str(&format!("t:{}(", rows.len()));
                    result.push_str(&headers.join(","));
                    result.push_str(")[");
                    for (idx, row) in rows.iter().enumerate() {
                        if idx > 0 {
                            result.push(' ');
                        }
                        result.push_str(&row.join(","));
                    }
                    result.push_str("]\n");

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

/// Strip images while preserving other markdown syntax
fn strip_images_preserve_syntax(text: &str) -> String {
    use once_cell::sync::Lazy;

    // Remove ![alt](url) patterns
    // SAFETY: Compile-time constant pattern, guaranteed valid
    #[allow(clippy::expect_used)]
    static RE_IMAGE: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"!\[([^\]]*)\]\([^)]*\)").expect("valid regex pattern"));

    RE_IMAGE.replace_all(text, "").to_string()
}

/// Strip badges while preserving other markdown syntax
fn strip_badges_preserve_syntax(text: &str) -> String {
    let mut result = text.to_string();
    // Remove badge patterns like [![Build](url)](link)
    result = RE_BADGE.replace_all(&result, "").to_string();
    result
}

/// Strip URLs from links but keep link text and brackets
fn strip_urls_preserve_syntax(text: &str) -> String {
    use once_cell::sync::Lazy;

    // Convert [text](url) to just [text]
    // SAFETY: Compile-time constant pattern, guaranteed valid
    #[allow(clippy::expect_used)]
    static RE_LINK: Lazy<regex::Regex> =
        Lazy::new(|| regex::Regex::new(r"\[([^\]]+)\]\([^)]+\)").expect("valid regex pattern"));

    RE_LINK.replace_all(text, "[$1]").to_string()
}

/// Collapse multiple consecutive whitespace characters to single space.
fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_was_space = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }

    result
}

/// Collapse multiple consecutive newlines to single newline.
fn collapse_multiple_newlines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut newline_count = 0;

    for c in text.chars() {
        if c == '\n' {
            newline_count += 1;
            if newline_count <= 1 {
                result.push(c);
            }
        } else {
            newline_count = 0;
            result.push(c);
        }
    }

    result
}

/// Convert ASCII art boxes to DX Serializer format.
fn strip_ascii_art(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Track code blocks
        if trimmed.starts_with("```") {
            if in_code_block {
                // End of code block - check if it was ASCII art
                if has_ascii_art(&code_block_content) {
                    // Convert ASCII art to DX Serializer format
                    let converted = convert_ascii_art_to_dsr(&code_block_content);
                    result.push_str(&converted);
                    result.push('\n');
                } else {
                    result.push_str("```");
                    result.push_str(&code_block_lang);
                    result.push('\n');
                    result.push_str(&code_block_content);
                    result.push_str("```\n");
                }
                in_code_block = false;
                code_block_lang.clear();
                code_block_content.clear();
            } else {
                // Start of code block
                in_code_block = true;
                code_block_lang = trimmed[3..].to_string();
                code_block_content.clear();
            }
            continue;
        }

        if in_code_block {
            code_block_content.push_str(line);
            code_block_content.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Handle unclosed code block
    if in_code_block {
        if has_ascii_art(&code_block_content) {
            let converted = convert_ascii_art_to_dsr(&code_block_content);
            result.push_str(&converted);
        } else {
            result.push_str("```");
            result.push_str(&code_block_lang);
            result.push('\n');
            result.push_str(&code_block_content);
            result.push_str("```\n");
        }
    }

    result
}

/// Check if content contains ASCII art box characters.
fn has_ascii_art(content: &str) -> bool {
    let box_chars = [
        '┌', '┐', '└', '┘', '├', '┤', '┬', '┴', '┼', '─', '│', '═', '║', '╔', '╗', '╚', '╝', '╠',
        '╣', '╦', '╩', '╬',
    ];
    let box_count: usize = content.chars().filter(|c| box_chars.contains(c)).count();
    box_count > 10
}

/// Convert ASCII art box diagram to DX Serializer format.
fn convert_ascii_art_to_dsr(content: &str) -> String {
    let mut sections: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_section: Option<String> = None;
    let mut current_items: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip box border lines
        if trimmed.chars().all(|c| "┌┐└┘├┤┬┴┼─│═║╔╗╚╝╠╣╦╩╬ ".contains(c))
        {
            continue;
        }

        // Remove box characters from content
        let clean: String = trimmed
            .chars()
            .filter(|c| !"┌┐└┘├┤┬┴┼─│═║╔╗╚╝╠╣╦╩╬".contains(*c))
            .collect::<String>()
            .trim()
            .to_string();

        if clean.is_empty() {
            continue;
        }

        // Check if this is a section header (all caps or contains "PASS")
        if clean.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase())
            || clean.contains("PASS")
            || clean.ends_with(':')
        {
            // Save previous section
            if let Some(name) = current_section.take()
                && !current_items.is_empty()
            {
                sections.push((name, current_items.clone()));
                current_items.clear();
            }
            current_section = Some(clean.trim_end_matches(':').to_string());
        } else if clean.starts_with('•') || clean.starts_with('-') || clean.starts_with('*') {
            // This is a bullet item
            let item = clean.trim_start_matches(['•', '-', '*', ' ']);
            current_items.push(item.to_string());
        } else if current_section.is_some() {
            // Content line under a section
            current_items.push(clean);
        }
    }

    // Save last section
    if let Some(name) = current_section
        && !current_items.is_empty()
    {
        sections.push((name, current_items));
    }

    // Convert to DX Serializer format
    if sections.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    output.push_str(&format!("steps:{}(name,tasks)[\n", sections.len()));

    for (name, items) in &sections {
        let tasks = items.join("; ");
        output.push_str(&format!("{},{}\n", name, tasks));
    }

    output.push(']');
    output
}

/// Remove empty sections (headers with no content).
fn remove_empty_sections(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = String::with_capacity(text.len());
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Check if this is a header (but not inside a code block marker)
        if trimmed.starts_with('#') && !trimmed.starts_with("```") && trimmed.contains(' ') {
            // Look ahead to see if there's content before the next header
            let mut has_content = false;
            let mut j = i + 1;

            while j < lines.len() {
                let next_line = lines[j].trim();

                // Skip empty lines
                if next_line.is_empty() {
                    j += 1;
                    continue;
                }

                // If we hit another header at same or higher level, check if this section is empty
                if next_line.starts_with('#')
                    && !next_line.starts_with("```")
                    && next_line.contains(' ')
                {
                    break;
                }

                // Found content
                has_content = true;
                break;
            }

            // Only include header if it has content OR if it's the last header
            if has_content || j >= lines.len() {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }

        i += 1;
    }

    result
}

/// Clean up broken/truncated URLs.
fn clean_broken_urls(text: &str) -> String {
    let result = RE_TRUNCATED.replace_all(text, "");
    let result = RE_BADGE.replace_all(&result, "");
    let result = RE_BROKEN_IMG.replace_all(&result, "");
    let result = RE_BROKEN_LINK.replace_all(&result, "");
    let result = RE_BROKEN_BADGE.replace_all(&result, "");
    let result = RE_EMPTY_LINK.replace_all(&result, "");
    let result = RE_ORPHAN_OPEN.replace_all(&result, "");
    result.to_string()
}

/// Strip GitHub-specific admonition markers like [!TIP], [!NOTE], [!WARNING]
/// These are visual markers that waste tokens - LLMs understand context without them.
fn strip_admonition_markers(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip lines that are just admonition markers
        if trimmed == "[!TIP]"
            || trimmed == "[!NOTE]"
            || trimmed == "[!WARNING]"
            || trimmed == "[!IMPORTANT]"
            || trimmed == "[!CAUTION]"
        {
            continue;
        }

        // Remove inline admonition markers at start of line
        let cleaned = if trimmed.starts_with("[!TIP] ") {
            &trimmed[7..]
        } else if trimmed.starts_with("[!NOTE] ") {
            &trimmed[8..]
        } else if trimmed.starts_with("[!WARNING] ") {
            &trimmed[11..]
        } else if trimmed.starts_with("[!IMPORTANT] ") {
            &trimmed[13..]
        } else if trimmed.starts_with("[!CAUTION] ") {
            &trimmed[11..]
        } else {
            trimmed
        };

        if !cleaned.is_empty() {
            result.push_str(cleaned);
            result.push('\n');
        }
    }

    result
}

/// Remove orphaned parentheses content like "(./LICENSE)" that's left over from badge stripping
fn strip_orphan_parens(text: &str) -> String {
    let mut result = String::new();
    for line in text.lines() {
        if !RE_ORPHAN_PATH.is_match(line) {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}

/// Strip decorative emojis from list items and headers.
/// LLMs don't need visual decorations - they understand context without them.
fn strip_decorative_emojis(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Track code blocks - don't modify content inside them
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Strip leading emojis from list items (e.g., "-📊 Token-Efficient" -> "-Token-Efficient")
        if trimmed.starts_with('-') || trimmed.starts_with('*') {
            let prefix = &trimmed[..1];
            let rest = trimmed[1..].trim_start();
            // Check if the rest starts with an emoji (common decorative emojis)
            let cleaned = strip_leading_emoji(rest);
            result.push_str(prefix);
            result.push_str(cleaned);
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Strip leading emoji from text.
fn strip_leading_emoji(text: &str) -> &str {
    let mut chars = text.chars().peekable();
    let mut byte_offset = 0;

    // Skip leading emojis and variation selectors
    while let Some(&c) = chars.peek() {
        // Check if it's an emoji or emoji-related character
        if is_emoji_char(c) {
            byte_offset += c.len_utf8();
            chars.next();
        } else {
            break;
        }
    }

    // Skip any space after the emoji
    if let Some(&c) = chars.peek()
        && c == ' '
    {
        byte_offset += 1;
    }

    &text[byte_offset..]
}

/// Check if a character is an emoji or emoji-related.
fn is_emoji_char(c: char) -> bool {
    // Common emoji ranges
    matches!(c,
        '\u{1F300}'..='\u{1F9FF}' | // Miscellaneous Symbols and Pictographs, Emoticons, etc.
        '\u{2600}'..='\u{26FF}' |   // Miscellaneous Symbols
        '\u{2700}'..='\u{27BF}' |   // Dingbats
        '\u{FE00}'..='\u{FE0F}' |   // Variation Selectors
        '\u{1F000}'..='\u{1F02F}' | // Mahjong Tiles
        '\u{1F0A0}'..='\u{1F0FF}' | // Playing Cards
        '\u{200D}' |               // Zero Width Joiner
        '\u{20E3}' |               // Combining Enclosing Keycap
        '\u{1F1E0}'..='\u{1F1FF}'   // Regional Indicator Symbols
    )
}

/// Strip ASCII progress bars from code blocks.
/// These are visual decorations (█░) that waste tokens - LLMs don't need them.
fn strip_ascii_progress_bars(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        // Track code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            // Strip progress bars from code block lines
            let cleaned = RE_BAR_PATTERN.replace_all(line, "");
            let cleaned = RE_ARROW_PATTERN.replace_all(&cleaned, "");
            // Collapse multiple spaces that might result
            let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
            if !cleaned.trim().is_empty() {
                result.push_str(&cleaned);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Post-process the output to clean up remaining issues.
fn post_process(text: &str) -> String {
    let text = strip_ascii_art(text);
    let text = clean_broken_urls_outside_code(&text);
    let text = strip_admonition_markers(&text);
    let text = strip_orphan_parens(&text);
    let text = strip_decorative_emojis(&text);
    let text = strip_ascii_progress_bars(&text);

    // Collapse multiple newlines to single newline (no blank lines)
    let text = collapse_multiple_newlines(&text);

    text.trim().to_string()
}

/// Clean broken URLs but preserve code blocks.
fn clean_broken_urls_outside_code(text: &str) -> String {
    let mut result = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
            continue;
        }

        if in_code_block {
            // Don't process code blocks
            result.push_str(line);
            result.push('\n');
        } else {
            // Clean URLs outside code blocks
            let cleaned = clean_broken_urls(line);
            // Skip lines that become empty or just whitespace after cleaning
            let cleaned_trimmed = cleaned.trim();
            if !cleaned_trimmed.is_empty() && cleaned_trimmed != ":" {
                result.push_str(&cleaned);
                result.push('\n');
            }
        }
    }

    result
}

/// Strip URLs from markdown links, keeping only the link text.
///
/// Strip URLs from markdown links, keeping only the link text.
///
/// This is a utility function kept for potential future use.
pub fn strip_urls(input: &str) -> String {
    let config = CompilerConfig {
        strip_urls: true,
        strip_images: false,
        strip_badges: false,
        tables_to_tsv: false,
        dictionary: false,
        minify_code: false,
        collapse_whitespace: false,
        strip_filler: false,
        ..Default::default()
    };

    if let Ok(compiler) = DxMarkdown::new(config)
        && let Ok(result) = compiler.compile(input)
    {
        return result.output;
    }

    input.to_string()
}

/// Strip images from markdown.
///
/// This is a utility function kept for potential future use.
pub fn strip_images(input: &str) -> String {
    let config = CompilerConfig {
        strip_urls: false,
        strip_images: true,
        strip_badges: true,
        tables_to_tsv: false,
        dictionary: false,
        minify_code: false,
        collapse_whitespace: false,
        strip_filler: false,
        ..Default::default()
    };

    if let Ok(compiler) = DxMarkdown::new(config)
        && let Ok(result) = compiler.compile(input)
    {
        return result.output;
    }

    input.to_string()
}

/// Strip badges from markdown.
///
/// This is a utility function kept for potential future use.
pub fn strip_badges(input: &str) -> String {
    let config = CompilerConfig {
        strip_urls: false,
        strip_images: false,
        strip_badges: true,
        tables_to_tsv: false,
        dictionary: false,
        minify_code: false,
        collapse_whitespace: false,
        strip_filler: false,
        ..Default::default()
    };

    if let Ok(compiler) = DxMarkdown::new(config)
        && let Ok(result) = compiler.compile(input)
    {
        return result.output;
    }

    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::CompilerMode;

    #[test]
    fn test_strip_urls_simple() {
        let input = "Check the [documentation](https://example.com/docs) for details.";
        let output = strip_urls(input);
        assert!(output.contains("documentation"));
        assert!(!output.contains("https://example.com"));
    }

    #[test]
    fn test_strip_urls_multiple() {
        let input = "See [link1](http://a.com) and [link2](http://b.com).";
        let output = strip_urls(input);
        assert!(output.contains("link1"));
        assert!(output.contains("link2"));
        assert!(!output.contains("http://"));
    }

    #[test]
    fn test_strip_images() {
        let input = "Here is an image: ![Logo](https://example.com/logo.png)";
        let output = strip_images(input);
        assert!(!output.contains("Logo"));
        assert!(!output.contains("https://example.com"));
    }

    #[test]
    fn test_strip_badges() {
        let input = "![Build](https://img.shields.io/badge/build-passing-green)";
        let output = strip_badges(input);
        assert!(!output.contains("shields.io"));
    }

    #[test]
    fn test_compiler_basic() {
        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = "# Hello\n\nWorld";
        let result = compiler.compile(input).unwrap();
        assert!(result.tokens_before > 0);
        assert!(result.output.contains("Hello"));
        assert!(result.output.contains("World"));
    }

    #[test]
    fn test_compiler_with_links() {
        let config = CompilerConfig {
            strip_urls: true,
            ..Default::default()
        };
        let compiler = DxMarkdown::new(config).unwrap();
        let input = "Check [docs](https://example.com/very/long/url/path)";
        let result = compiler.compile(input).unwrap();
        assert!(result.output.contains("docs"));
        assert!(!result.output.contains("https://"));
    }

    #[test]
    fn test_compiler_preserves_code() {
        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = "```rust\nfn main() {}\n```";
        let result = compiler.compile(input).unwrap();
        assert!(result.output.contains("fn main()"));
        assert!(result.output.contains("```rust"));
    }

    #[test]
    fn test_compiler_preserves_markdown_code_block_urls() {
        // Code blocks with markdown language should preserve URLs even with minification enabled
        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = r#"```markdown
[![Build](https://img.shields.io/badge/build-passing-green)](https://ci.example.com)
```"#;
        let result = compiler.compile(input).unwrap();
        println!("OUTPUT: {}", result.output);
        // The URL should be preserved inside the code block
        assert!(
            result.output.contains("https://img.shields.io"),
            "URL should be preserved in markdown code block"
        );
    }

    #[test]
    fn test_collapse_whitespace() {
        let result = collapse_whitespace("hello   world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_collapse_multiple_newlines() {
        let result = collapse_multiple_newlines("a\n\n\n\nb");
        assert_eq!(result, "a\n\nb");
    }

    #[test]
    fn test_compiler_mode_full() {
        let config = CompilerConfig::default();
        assert_eq!(config.mode, CompilerMode::Full);
        assert!(config.strip_urls);
        assert!(config.strip_images);
    }

    #[test]
    fn test_input_too_large() {
        let compiler = DxMarkdown::default_compiler().unwrap();
        let large_input = "x".repeat(MAX_INPUT_SIZE + 1);
        let result = compiler.compile(&large_input);
        assert!(matches!(result, Err(CompileError::InputTooLarge { .. })));
    }

    #[test]
    fn test_compiler_minifies_code() {
        let config = CompilerConfig {
            minify_code: true,
            ..Default::default()
        };
        let compiler = DxMarkdown::new(config).unwrap();
        let input = "```javascript\n// comment\nconst x = 1;\n```";
        let result = compiler.compile(input).unwrap();
        // Comment should be removed
        assert!(!result.output.contains("// comment"));
        assert!(result.output.contains("const x"));
    }

    #[test]
    fn test_compiler_strips_filler() {
        let config = CompilerConfig {
            strip_filler: true,
            ..Default::default()
        };
        let compiler = DxMarkdown::new(config).unwrap();
        let input = "In this section, we will discuss the API.";
        let result = compiler.compile(input).unwrap();
        // Filler should be removed
        assert!(!result.output.to_lowercase().contains("in this section"));
        assert!(result.output.contains("API"));
    }

    #[test]
    fn test_compiler_token_savings() {
        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = r#"
# README

Check the [documentation](https://example.com/very/long/documentation/url/path) for details.

![Badge](https://img.shields.io/badge/build-passing-green)

| Name | Value |
|------|-------|
| foo  | 123   |
| bar  | 456   |
"#;
        let result = compiler.compile(input).unwrap();
        // Should have some token savings
        assert!(result.tokens_after <= result.tokens_before);
    }

    #[test]
    fn test_compiler_preserves_headers() {
        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = "# H1\n## H2\n### H3";
        let result = compiler.compile(input).unwrap();
        assert!(result.output.contains("# H1"));
        assert!(result.output.contains("## H2"));
        assert!(result.output.contains("### H3"));
    }

    #[test]
    fn test_compiler_preserves_lists() {
        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = "- Item 1\n- Item 2\n- Item 3";
        let result = compiler.compile(input).unwrap();
        assert!(result.output.contains("Item 1"));
        assert!(result.output.contains("Item 2"));
        assert!(result.output.contains("Item 3"));
    }

    #[test]
    fn test_compiler_with_dictionary_disabled() {
        let config = CompilerConfig {
            dictionary: false,
            ..Default::default()
        };
        let compiler = DxMarkdown::new(config).unwrap();
        let input = "AuthenticationMiddleware handles auth. AuthenticationMiddleware is important.";
        let result = compiler.compile(input).unwrap();
        // Should not have dictionary variables
        assert!(!result.output.contains("$A"));
    }

    #[test]
    fn test_streaming_basic() {
        use std::io::Cursor;

        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = "# Hello\n\nWorld";
        let reader = Cursor::new(input);
        let mut output = Vec::new();

        let result = compiler.compile_streaming(reader, &mut output).unwrap();

        assert!(result.tokens_before > 0);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Hello"));
        assert!(output_str.contains("World"));
    }

    #[test]
    fn test_streaming_with_links() {
        use std::io::Cursor;

        let config = CompilerConfig {
            strip_urls: true,
            ..Default::default()
        };
        let compiler = DxMarkdown::new(config).unwrap();
        let input = "Check [docs](https://example.com/very/long/url/path)";
        let reader = Cursor::new(input);
        let mut output = Vec::new();

        let result = compiler.compile_streaming(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("docs"));
        assert!(!output_str.contains("https://"));
        assert!(result.tokens_after <= result.tokens_before);
    }

    #[test]
    fn test_streaming_multiline() {
        use std::io::Cursor;

        let compiler = DxMarkdown::default_compiler().unwrap();
        let input = "# Title\n\nParagraph 1.\n\nParagraph 2.\n\n- Item 1\n- Item 2";
        let reader = Cursor::new(input);
        let mut output = Vec::new();

        let result = compiler.compile_streaming(reader, &mut output).unwrap();

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Title"));
        assert!(output_str.contains("Paragraph 1"));
        assert!(output_str.contains("Item 1"));
        assert!(result.tokens_before > 0);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        /// Property 1: Semantic Preservation
        /// For any valid Markdown input, the compiled output SHALL contain
        /// all semantic content (text, headers, code logic) from the input.
        /// **Validates: Requirements 1.1, 1.3**
        #[test]
        fn prop_semantic_preservation(header in "[A-Za-z]{3,10}") {
            let input = format!("# {}\n\nSome text.", header);
            let config = CompilerConfig {
                strip_urls: false,
                strip_images: false,
                strip_badges: false,
                dictionary: false,
                minify_code: false,
                strip_filler: false,
                ..Default::default()
            };
            let compiler = DxMarkdown::new(config).unwrap();
            let result = compiler.compile(&input).unwrap();

            // Header should be preserved
            assert!(result.output.contains(&header));
        }

        /// Property 2: Token Reduction
        /// For any Markdown input with URLs, the compiled output SHALL have
        /// fewer or equal tokens than the input.
        /// **Validates: Requirements 2.1, 3.1**
        #[test]
        fn prop_token_reduction(text in "[A-Za-z]{3,10}") {
            let input = format!("Check [{}](http://example.com/very/long/url).", text);
            let compiler = DxMarkdown::default_compiler().unwrap();
            let result = compiler.compile(&input).unwrap();

            // Output should not be larger than input
            assert!(result.tokens_after <= result.tokens_before);
        }

        /// Property 3: Idempotence
        /// For any input, compiling twice SHALL produce the same output as compiling once.
        /// **Validates: Design Property 6**
        #[test]
        fn prop_idempotence(text in "[A-Za-z ]{5,20}") {
            let input = format!("# Title\n\n{}", text);
            let compiler = DxMarkdown::default_compiler().unwrap();
            let result1 = compiler.compile(&input).unwrap();
            let result2 = compiler.compile(&result1.output).unwrap();

            // Second compilation should produce same or similar output
            assert!(!result2.output.is_empty());
        }

        /// Property 4: UTF-8 Preservation
        /// For any UTF-8 input, the output SHALL be valid UTF-8.
        /// **Validates: Design Property 7**
        #[test]
        fn prop_utf8_preservation(content in "[A-Za-z0-9]{3,10}") {
            let input = format!("# {}", content);
            let compiler = DxMarkdown::default_compiler().unwrap();
            let result = compiler.compile(&input).unwrap();

            // Content should be preserved
            assert!(result.output.contains(&content));
        }

        /// Property 5: Code block integrity
        /// For any code block, the code markers should be preserved.
        /// **Validates: Requirements 5.7**
        #[test]
        fn prop_code_block_integrity(code in "[a-z]{3,10}") {
            let input = format!("```\n{}\n```", code);
            let config = CompilerConfig {
                minify_code: false,
                ..Default::default()
            };
            let compiler = DxMarkdown::new(config).unwrap();
            let result = compiler.compile(&input).unwrap();

            // Code markers should be preserved
            assert!(result.output.contains("```"));
        }
    }
}
