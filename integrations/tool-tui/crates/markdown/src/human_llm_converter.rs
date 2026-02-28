//! Human ↔ LLM Format Converter with Section Filtering
//!
//! Flow:
//! 1. README.md → README.human (with FIGlet headers via WASM, full content)
//! 2. README.human → README.llm (filtered, no FIGlet, optimized for tokens)
//! 3. When editing README.human, regenerate README.llm

use crate::error::CompileError;
use crate::section_filter::{SectionFilterConfig, filter_sections};
use std::fs;
use std::path::Path;

/// Convert markdown to human format (preserves all content, expands compact lists, formats tables)
pub fn md_to_human_format(md_content: &str) -> Result<String, CompileError> {
    // Step 1: Convert compact table notation to proper markdown tables
    let with_tables = expand_compact_tables(md_content);

    // Step 2: Clean up variable references and shell-like syntax
    let cleaned = clean_variable_references(&with_tables);

    // Step 3: Add proper spacing between sections
    let spaced = add_section_spacing(&cleaned);

    Ok(spaced)
}

/// Expand compact table notation: t:N(col1,col2)[row1,val1 row2,val2] → proper markdown table
fn expand_compact_tables(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if line contains compact table notation: t:N(headers)[rows]
        if let Some(table_md) = parse_compact_table(line) {
            result.push(table_md);
            i += 1;
        } else if let Some(table_md) = parse_dx_serializer_table(line) {
            // Also handle DX Serializer format with + separators
            result.push(table_md);
            i += 1;
        } else {
            result.push(line.to_string());
            i += 1;
        }
    }

    result.join("\n")
}

/// Parse DX Serializer table format with + separators
/// Format: :N header1+header2+header3 [ row1val1+row1val2+row1val3 row2val1+row2val2+row2val3 ]
fn parse_dx_serializer_table(line: &str) -> Option<String> {
    // Look for pattern: :N header1+header2 [ ... ]
    if !line.contains(':') || !line.contains('[') {
        return None;
    }

    // Find the colon and bracket positions
    let colon_pos = line.find(':')?;
    let bracket_start = line.find('[')?;
    let bracket_end = line.rfind(']')?;

    // Validate positions
    if colon_pos >= bracket_start || bracket_start >= bracket_end || bracket_end > line.len() {
        return None;
    }

    // Extract column count
    let header_section = &line[colon_pos + 1..bracket_start];
    let col_count_str = header_section.split_whitespace().next()?;
    let col_count: usize = col_count_str.trim().parse().ok()?;

    if col_count == 0 {
        return None;
    }

    // Extract headers - they come after the column count, before the bracket
    let headers_str = header_section.trim();
    let headers_parts: Vec<&str> = headers_str.split_whitespace().collect();

    if headers_parts.len() < 2 {
        return None;
    }

    // First part is the column count, rest is headers
    let headers_combined = headers_parts[1..].join(" ");
    let headers: Vec<&str> = headers_combined.split('+').map(|s| s.trim()).collect();

    if headers.len() != col_count {
        return None;
    }

    // Extract rows
    if bracket_start + 1 >= bracket_end {
        return None;
    }
    let rows_str = &line[bracket_start + 1..bracket_end];

    // Parse rows - they are separated by spaces, cells by +
    let mut rows = Vec::new();

    for row_str in rows_str.split_whitespace() {
        let cells: Vec<&str> = row_str.split('+').map(|s| s.trim()).collect();
        if cells.len() == col_count {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        return None;
    }

    // Build ASCII art table with + and - characters
    let mut table = String::new();

    // Calculate column widths (minimum 10 characters per column)
    let mut col_widths = vec![10; col_count];
    for (i, header) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(header.len() + 2);
    }
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len() + 2);
        }
    }

    // Top border
    table.push('+');
    for width in &col_widths {
        table.push_str(&"-".repeat(*width));
        table.push('+');
    }
    table.push('\n');

    // Header row
    table.push('|');
    for (i, header) in headers.iter().enumerate() {
        let padding = col_widths[i] - header.len();
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        table.push_str(&" ".repeat(left_pad));
        table.push_str(header);
        table.push_str(&" ".repeat(right_pad));
        table.push('|');
    }
    table.push('\n');

    // Middle border
    table.push('+');
    for width in &col_widths {
        table.push_str(&"-".repeat(*width));
        table.push('+');
    }
    table.push('\n');

    // Data rows
    for row in rows {
        table.push('|');
        for (i, cell) in row.iter().enumerate() {
            let padding = col_widths[i] - cell.len();
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            table.push_str(&" ".repeat(left_pad));
            table.push_str(cell);
            table.push_str(&" ".repeat(right_pad));
            table.push('|');
        }
        table.push('\n');
    }

    // Bottom border
    table.push('+');
    for width in &col_widths {
        table.push_str(&"-".repeat(*width));
        table.push('+');
    }
    table.push('\n');

    Some(table)
}

/// Parse compact table notation and convert to ASCII art table
/// Format: t:N(col1,col2,col3)[row1,val1,val2 row2,val3,val4]
fn parse_compact_table(line: &str) -> Option<String> {
    // Look for pattern: t:N(...)[ ... ]
    if !line.contains("t:") {
        return None;
    }

    // Find the table notation pattern
    let table_start = line.find("t:")?;
    let remaining = &line[table_start..];

    let _paren_start = remaining.find('(')?;
    let _paren_end = remaining.find(')')?;
    let _bracket_start = remaining.find('[')?;
    let _bracket_end = remaining.rfind(']')?;

    let paren_start = remaining.find('(')?;
    let paren_end = remaining.find(')')?;
    let bracket_start = remaining.find('[')?;
    let bracket_end = remaining.rfind(']')?;

    // Validate positions
    if paren_start >= paren_end || bracket_start >= bracket_end || paren_end > bracket_start {
        return None;
    }

    // Extract column count (between "t:" and "(")
    if table_start + 2 > table_start + paren_start {
        return None;
    }
    let col_count_str = &line[table_start + 2..table_start + paren_start];
    let col_count: usize = col_count_str.trim().parse().ok()?;

    if col_count == 0 {
        return None;
    }

    // Extract headers (between "(" and ")")
    if table_start + paren_start + 1 > table_start + paren_end {
        return None;
    }
    let headers_str = &line[table_start + paren_start + 1..table_start + paren_end];
    let headers: Vec<&str> = headers_str.split(',').map(|s| s.trim()).collect();

    if headers.len() != col_count {
        return None;
    }

    // Extract rows (between "[" and "]")
    if table_start + bracket_start + 1 > table_start + bracket_end {
        return None;
    }
    let rows_str = &line[table_start + bracket_start + 1..table_start + bracket_end];

    // Parse rows - cells are comma-separated
    // The entire bracket content is ONE row with comma-separated cells
    let mut rows = Vec::new();

    let cells: Vec<&str> =
        rows_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    // Check if we have exactly the right number of cells for one row
    if cells.len() == col_count {
        rows.push(cells);
    } else {
        // If not exact match, it might be multiple rows
        // Try to split into rows of col_count cells each
        let mut current_row = Vec::new();
        for cell in cells {
            current_row.push(cell);
            if current_row.len() == col_count {
                rows.push(current_row.clone());
                current_row.clear();
            }
        }
    }

    if rows.is_empty() {
        return None;
    }

    // Build ASCII art table with + and - characters
    let mut table = String::new();

    // Calculate column widths (minimum 10 characters per column)
    let mut col_widths = vec![10; col_count];
    for (i, header) in headers.iter().enumerate() {
        col_widths[i] = col_widths[i].max(header.len() + 2);
    }
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            col_widths[i] = col_widths[i].max(cell.len() + 2);
        }
    }

    // Top border
    table.push('+');
    for width in &col_widths {
        table.push_str(&"-".repeat(*width));
        table.push('+');
    }
    table.push('\n');

    // Header row
    table.push('|');
    for (i, header) in headers.iter().enumerate() {
        let padding = col_widths[i] - header.len();
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        table.push_str(&" ".repeat(left_pad));
        table.push_str(header);
        table.push_str(&" ".repeat(right_pad));
        table.push('|');
    }
    table.push('\n');

    // Middle border
    table.push('+');
    for width in &col_widths {
        table.push_str(&"-".repeat(*width));
        table.push('+');
    }
    table.push('\n');

    // Data rows
    for row in rows {
        table.push('|');
        for (i, cell) in row.iter().enumerate() {
            let padding = col_widths[i] - cell.len();
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            table.push_str(&" ".repeat(left_pad));
            table.push_str(cell);
            table.push_str(&" ".repeat(right_pad));
            table.push('|');
        }
        table.push('\n');
    }

    // Bottom border
    table.push('+');
    for width in &col_widths {
        table.push_str(&"-".repeat(*width));
        table.push('+');
    }
    table.push('\n');

    Some(table)
}

/// Clean up variable references like $A, $B, etc. and expand them to readable text
fn clean_variable_references(content: &str) -> String {
    let mut result = content.to_string();

    // Common variable patterns to expand
    let replacements = vec![
        ("$A", "faster than"),
        ("$B", "Achieved 10x"),
        ("$C", "crates/serializer/README.md"),
        ("$D", "dx-style README"),
        ("$E", "dx-form, dx-guard, dx-a11y"),
        ("$F", "RPS HTTP"),
        ("$M", "SharedArrayBuffer"),
        ("$N", "lines"),
        ("$O", "dx-reactor README"),
        ("$P", "10.59x"),
        ("$Q", "faster than Bun"),
        ("$R", "JavaScript/TypeScript"),
        ("$U", "smaller than"),
        ("$V", "Benchmarks"),
        ("$Y", "7.5KB"),
        ("$Z", "HTTP, 5M+ RPS"),
    ];

    for (var, replacement) in replacements {
        result = result.replace(var, replacement);
    }

    result
}

/// Add proper spacing between sections for readability
fn add_section_spacing(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let is_empty = line.trim().is_empty();
        let is_header = line.starts_with('#');

        // Add the current line
        result.push(line.to_string());

        // Add spacing after headers (unless next line is empty)
        if is_header && i + 1 < lines.len() && !lines[i + 1].trim().is_empty() {
            result.push(String::new());
        }

        // Add spacing before headers (unless previous line is empty)
        if i + 1 < lines.len() && lines[i + 1].starts_with('#') && !is_empty {
            result.push(String::new());
        }
    }

    result.join("\n")
}

/// Convert human format to LLM format (DX Markdown LLM v1.0)
/// Yellow List: Remove blank lines + convert tables/ASCII to DX Serializer
pub fn human_format_to_llm(
    human_content: &str,
    config: &SectionFilterConfig,
) -> Result<String, CompileError> {
    // Step 1: Remove FIGlet headers (they waste tokens)
    let without_figlet = remove_figlet_headers(human_content);

    // Step 2: Remove blank lines (keep single newlines only)
    let no_blank_lines = remove_blank_lines(&without_figlet);

    // Step 3: Convert markdown tables to DX Serializer LLM format
    let with_dx_tables = convert_tables_to_dx_serializer(&no_blank_lines);

    // Step 4: Convert ASCII art to DX Serializer LLM format
    let with_dx_ascii = convert_ascii_art_to_dx_serializer(&with_dx_tables);

    // Step 5: Apply section filtering (Red List - user controlled)
    let filtered = filter_sections(&with_dx_ascii, config);

    Ok(filtered)
}

/// Remove blank lines: keep only single newlines
fn remove_blank_lines(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();

    for line in lines {
        // Skip empty lines
        if !line.trim().is_empty() {
            result.push(line.to_string());
        }
    }

    result.join("\n")
}

/// Convert markdown tables to DX Serializer LLM format
fn convert_tables_to_dx_serializer(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        // Check if this is a markdown table
        if i + 2 < lines.len()
            && lines[i].contains('|')
            && lines[i + 1].contains('|')
            && lines[i + 1].contains('-')
        {
            // Parse table
            let header_line = lines[i];
            let headers: Vec<&str> =
                header_line.split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

            if !headers.is_empty() {
                // Collect data rows
                let mut rows = Vec::new();
                let mut j = i + 2; // Skip header and separator

                while j < lines.len() && lines[j].contains('|') {
                    let cells: Vec<&str> =
                        lines[j].split('|').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

                    if cells.len() == headers.len() {
                        rows.push(cells);
                    }
                    j += 1;
                }

                if !rows.is_empty() {
                    // Convert to DX Serializer LLM format
                    let header_str = headers.join(" ").replace(' ', "_");
                    result.push(format!("t:{}({})[", rows.len(), header_str));

                    for row in rows {
                        let row_str = row.join(" ").replace(' ', "_");
                        result.push(row_str);
                    }
                    result.push("]".to_string());

                    i = j;
                    continue;
                }
            }
        }

        result.push(lines[i].to_string());
        i += 1;
    }

    result.join("\n")
}

/// Convert ASCII art to DX Serializer LLM format
fn convert_ascii_art_to_dx_serializer(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if this is ASCII box art (starts with + or |)
        if line.trim().starts_with('+') || line.trim().starts_with('|') {
            let mut ascii_lines = Vec::new();
            let mut j = i;

            // Collect consecutive ASCII art lines
            while j < lines.len() {
                let l = lines[j].trim();
                if l.starts_with('+')
                    || l.starts_with('|')
                    || l.starts_with('─')
                    || l.starts_with('╔')
                {
                    ascii_lines.push(lines[j]);
                    j += 1;
                } else {
                    break;
                }
            }

            // If we found ASCII art (at least 3 lines), convert it
            if ascii_lines.len() >= 3 {
                // Extract content from ASCII boxes
                let mut content_items = Vec::new();
                for ascii_line in &ascii_lines {
                    // Extract text between | or box characters
                    let text: String = ascii_line
                        .chars()
                        .filter(|c| {
                            !matches!(
                                c,
                                '+' | '-'
                                    | '|'
                                    | '─'
                                    | '│'
                                    | '┌'
                                    | '┐'
                                    | '└'
                                    | '┘'
                                    | '╔'
                                    | '╗'
                                    | '╚'
                                    | '╝'
                                    | '║'
                                    | '═'
                            )
                        })
                        .collect();

                    let trimmed = text.trim();
                    if !trimmed.is_empty() && !trimmed.chars().all(|c| c == ' ') {
                        content_items.push(trimmed.replace(' ', "_"));
                    }
                }

                // Convert to DX Serializer format
                if !content_items.is_empty() {
                    result.push(format!("t:{}(ASCII_Art)[", content_items.len()));
                    for item in content_items {
                        result.push(item);
                    }
                    result.push("]".to_string());
                }

                i = j;
                continue;
            }
        }

        result.push(line.to_string());
        i += 1;
    }

    result.join("\n")
}

/// Remove FIGlet ASCII art headers
fn remove_figlet_headers(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if is_figlet_line(line) {
            // Skip FIGlet lines
            while i < lines.len() && (is_figlet_line(lines[i]) || lines[i].trim().is_empty()) {
                i += 1;
            }
            // Keep the header that follows
            if i < lines.len() && lines[i].starts_with('#') {
                result.push(lines[i]);
                i += 1;
            }
        } else {
            result.push(line);
            i += 1;
        }
    }

    result.join("\n")
}

/// Check if line is FIGlet ASCII art
fn is_figlet_line(line: &str) -> bool {
    if line.trim().is_empty() {
        return false;
    }

    let figlet_chars = [
        '█', '▀', '▄', '▌', '▐', '░', '▒', '▓', '│', '─', '┌', '┐', '└', '┘',
    ];
    let char_count = line.chars().filter(|c| figlet_chars.contains(c)).count();
    let non_whitespace = line.chars().filter(|c| !c.is_whitespace()).count();

    non_whitespace > 0 && (char_count as f64 / non_whitespace as f64) > 0.3
}

/// Convert LLM format back to human format (lossy)
pub fn llm_format_to_human(llm_content: &str) -> Result<String, CompileError> {
    Ok(llm_content.to_string())
}

/// Generate .human and .llm from .md
pub fn generate_human_llm_from_md(
    md_path: &Path,
    config: &SectionFilterConfig,
) -> Result<(), CompileError> {
    let md_content = fs::read_to_string(md_path).map_err(|e| CompileError::Io {
        message: format!("Failed to read {}: {}", md_path.display(), e),
    })?;

    // Generate .human
    let human_content = md_to_human_format(&md_content)?;
    let human_path = md_path.with_extension("human");
    fs::write(&human_path, human_content).map_err(|e| CompileError::Io {
        message: format!("Failed to write {}: {}", human_path.display(), e),
    })?;

    // Generate .llm
    let llm_content = human_format_to_llm(&md_content, config)?;
    let llm_path = md_path.with_extension("llm");
    fs::write(&llm_path, llm_content).map_err(|e| CompileError::Io {
        message: format!("Failed to write {}: {}", llm_path.display(), e),
    })?;

    Ok(())
}

/// Regenerate .llm from .human
pub fn regenerate_llm_from_human_file(
    human_path: &Path,
    config: &SectionFilterConfig,
) -> Result<(), CompileError> {
    let human_content = fs::read_to_string(human_path).map_err(|e| CompileError::Io {
        message: format!("Failed to read {}: {}", human_path.display(), e),
    })?;

    let llm_content = human_format_to_llm(&human_content, config)?;
    let llm_path = human_path.with_extension("llm");

    fs::write(&llm_path, llm_content).map_err(|e| CompileError::Io {
        message: format!("Failed to write {}: {}", llm_path.display(), e),
    })?;

    Ok(())
}

/// Watcher for .human file changes
pub struct HumanFileWatcher {
    config: SectionFilterConfig,
}

impl HumanFileWatcher {
    pub fn new(config: SectionFilterConfig) -> Self {
        Self { config }
    }

    pub fn on_human_file_saved(&self, human_path: &Path) -> Result<(), CompileError> {
        regenerate_llm_from_human_file(human_path, &self.config)
    }

    pub fn update_config(&mut self, config: SectionFilterConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md_to_human_preserves_all() {
        let md = "# Title\n\n## Section 1\nContent";
        let human = md_to_human_format(md).unwrap();
        assert!(human.contains("# Title"));
    }

    #[test]
    fn test_remove_blank_lines() {
        let content = "# Title\n\n\nContent\n\n## Section\n\nMore content";
        let result = remove_blank_lines(content);
        assert!(!result.contains("\n\n"));
        assert_eq!(result, "# Title\nContent\n## Section\nMore content");
    }

    #[test]
    fn test_convert_tables_to_dx_serializer() {
        let table = "| Name | Age |\n|------|-----|\n| Alice | 30 |\n| Bob | 25 |";
        let converted = convert_tables_to_dx_serializer(table);
        assert!(converted.contains("t:2("));
        assert!(converted.contains("Alice"));
    }

    #[test]
    fn test_convert_ascii_art_to_dx_serializer() {
        let ascii = "+--------+\n|  Box1  |\n+--------+";
        let converted = convert_ascii_art_to_dx_serializer(ascii);
        assert!(converted.contains("t:"));
        assert!(converted.contains("Box1"));
    }

    #[test]
    fn test_remove_figlet_headers() {
        let content = "█████╗ ██╗\n# DX\n\nContent";
        let result = remove_figlet_headers(content);
        assert!(result.contains("# DX"));
        assert!(!result.contains("█████"));
    }

    #[test]
    fn test_human_to_llm_removes_figlet_and_filters() {
        let human = "█████╗\n# Title\n\n## Section 1\nContent\n\n## Acknowledgments\nThanks";
        let config = SectionFilterConfig::conservative();
        let llm = human_format_to_llm(human, &config).unwrap();

        assert!(llm.contains("## Section 1"));
        assert!(!llm.contains("## Acknowledgments"));
        assert!(!llm.contains("█████"));
        assert!(!llm.contains("\n\n")); // No blank lines
    }

    #[test]
    fn test_human_to_llm_minimal_changes() {
        let human = "## Features\n\n- Perf: 10ms\n- SIMD: AVX2\n\n| Name | Age |\n|------|-----|\n| Alice | 30 |";
        let config = SectionFilterConfig::default();
        let llm = human_format_to_llm(human, &config).unwrap();

        // Should remove blank lines
        assert!(!llm.contains("\n\n"));

        // Should convert tables
        assert!(llm.contains("t:1("));

        // Should preserve list syntax exactly
        assert!(llm.contains("- Perf: 10ms"));
        assert!(llm.contains("- SIMD: AVX2"));
    }

    #[test]
    fn test_is_figlet_line() {
        assert!(is_figlet_line("█████╗ ██╗  ██╗"));
        assert!(!is_figlet_line("# Regular Header"));
    }

    #[test]
    fn test_expand_compact_tables() {
        let compact = "t:3(Name,Age,City)[Alice,30,NYC Bob,25,LA]";
        let expanded = expand_compact_tables(compact);
        assert!(expanded.contains("+"));
        assert!(expanded.contains("Name"));
        assert!(expanded.contains("Alice"));
        assert!(expanded.contains("Bob"));
    }

    #[test]
    fn test_parse_compact_table_simple() {
        let line = "t:4(Metric,Target,Comparison,Status)[HTTP,Mode,2,500]";
        let result = parse_compact_table(line);
        assert!(result.is_some(), "parse_compact_table should return Some for valid table");
        let table = result.unwrap();
        assert!(table.contains("+"), "Table should contain + characters");
        assert!(table.contains("Metric"), "Table should contain header 'Metric'");
        assert!(table.contains("HTTP"), "Table should contain data 'HTTP'");
    }

    #[test]
    fn test_md_to_human_format_with_table() {
        let md = "# Title\n\nt:4(Metric,Target,Comparison,Status)[HTTP,Mode,2,500]\n\nSome text";
        let result = md_to_human_format(md).unwrap();
        println!("Result:\n{}", result);
        assert!(result.contains("+"), "Result should contain ASCII table with + characters");
        assert!(result.contains("Metric"), "Result should contain header 'Metric'");
        assert!(result.contains("HTTP"), "Result should contain data 'HTTP'");
    }

    #[test]
    fn test_expand_dx_serializer_tables() {
        let compact = ":3 Name+Age+City [ Alice+30+NYC Bob+25+LA ]";
        let expanded = expand_compact_tables(compact);
        assert!(expanded.contains("+"));
        assert!(expanded.contains("Name"));
        assert!(expanded.contains("Alice"));
        assert!(expanded.contains("Bob"));
    }

    #[test]
    fn test_clean_variable_references() {
        let content = "Performance: $P $Q with $M support";
        let cleaned = clean_variable_references(content);
        assert_eq!(cleaned, "Performance: 10.59x faster than Bun with SharedArrayBuffer support");
    }

    #[test]
    fn test_md_to_human_format_full() {
        let md = "t:2(Feature,Status)[Fast,✅ Complete Secure,✅ Ready]\n-Perf: 10ms -SIMD: AVX2";
        let human = md_to_human_format(md).unwrap();

        // Should expand tables to ASCII art
        assert!(human.contains("+"));
        assert!(human.contains("Feature"));

        // Should expand bullets
        assert!(human.contains("- Perf: 10ms"));
        assert!(human.contains("- SIMD: AVX2"));
    }
}
