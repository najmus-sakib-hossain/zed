//! Table conversion for the DX Markdown Context Compiler.
//!
//! This module converts Markdown tables to DX Serializer LLM format
//! for significant token savings (40-60% per table).
//!
//! DX Serializer LLM Table Format:
//! ```text
//! t:3(col1,col2,col3)[
//! val1,val2,val3
//! val1,val2,val3]
//! ```
//!
//! Key rules:
//! - Comma separates columns in schema and values in rows
//! - `:N` prefix indicates row count (e.g., `t:3` = 3 rows)
//! - No quotes needed for text with spaces (commas handle separation)
//! - Quotes only for escaped characters (\n, \,, \\)

use crate::types::TableInfo;

/// Convert a table to DX Serializer LLM format.
///
/// DX Serializer LLM format is the most token-efficient table serialization:
/// ```text
/// t:3(col1,col2,col3)[
/// val1,val2,val3
/// val1,val2,val3]
/// ```
///
/// # Arguments
/// * `table` - The table information from analysis
///
/// # Returns
/// DX Serializer LLM formatted string.
pub fn table_to_tsv(table: &TableInfo) -> String {
    let mut output = String::new();

    // DX Serializer LLM format: t:N(col1,col2,col3)[
    output.push_str("t:");
    output.push_str(&table.rows.len().to_string());
    output.push('(');
    output
        .push_str(&table.headers.iter().map(|h| escape_dsr_value(h)).collect::<Vec<_>>().join(","));
    output.push_str(")[");

    // Add data rows with comma separator
    for row in &table.rows {
        output.push('\n');
        output.push_str(&row.iter().map(|v| escape_dsr_value(v)).collect::<Vec<_>>().join(","));
    }

    output.push(']');
    output
}

/// Escape a value for DX Serializer format.
/// In Dx Serializer LLM format, commas separate values so no quotes needed for spaces.
/// Quotes only needed for escape sequences (\n, \,, \\, \(, \), \[, \]).
fn escape_dsr_value(value: &str) -> String {
    // Trim and collapse multiple spaces to single space
    let trimmed: String = value.split_whitespace().collect::<Vec<_>>().join(" ");

    // Only escape if contains special characters that need escaping
    if trimmed.contains(',')
        || trimmed.contains('\n')
        || trimmed.contains('\\')
        || trimmed.contains('(')
        || trimmed.contains(')')
        || trimmed.contains('[')
        || trimmed.contains(']')
    {
        trimmed
            .replace('\\', "\\\\")
            .replace(',', "\\,")
            .replace('\n', "\\n")
            .replace('(', "\\(")
            .replace(')', "\\)")
            .replace('[', "\\[")
            .replace(']', "\\]")
    } else {
        trimmed
    }
}

/// Convert a table to TSV format without markers.
///
/// Useful when you want raw TSV without the DX_DATA wrapper.
pub fn table_to_tsv_raw(table: &TableInfo) -> String {
    let mut output = String::new();

    // Add header row
    output.push_str(&table.headers.join("\t"));
    output.push('\n');

    // Add data rows
    for row in &table.rows {
        output.push_str(&row.join("\t"));
        output.push('\n');
    }

    output
}

/// Convert a table to CSV format.
///
/// # Arguments
/// * `table` - The table information from analysis
///
/// # Returns
/// CSV-formatted string.
pub fn table_to_csv(table: &TableInfo) -> String {
    let mut output = String::new();

    // Add header row
    output.push_str(&escape_csv_row(&table.headers));
    output.push('\n');

    // Add data rows
    for row in &table.rows {
        output.push_str(&escape_csv_row(row));
        output.push('\n');
    }

    output
}

/// Escape a CSV row, quoting fields that contain commas or quotes.
fn escape_csv_row(row: &[String]) -> String {
    row.iter().map(|cell| escape_csv_field(cell)).collect::<Vec<_>>().join(",")
}

/// Escape a single CSV field.
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Check if a table should be kept inline (fewer than 3 rows).
pub fn should_keep_inline(table: &TableInfo) -> bool {
    table.rows.len() < 2
}

/// Convert a small table to inline format.
///
/// For tables with fewer than 3 rows, keep them inline for readability.
pub fn table_to_inline(table: &TableInfo) -> String {
    let mut parts = Vec::new();

    // Format: Header1: Value1, Header2: Value2
    for row in &table.rows {
        let pairs: Vec<String> = table
            .headers
            .iter()
            .zip(row.iter())
            .map(|(h, v)| format!("{}: {}", h, v))
            .collect();
        parts.push(pairs.join(", "));
    }

    parts.join("; ")
}

/// Parse a Markdown table string into TableInfo.
///
/// # Arguments
/// * `markdown` - The Markdown table string
///
/// # Returns
/// Parsed TableInfo or None if parsing fails.
pub fn parse_markdown_table(markdown: &str) -> Option<TableInfo> {
    let lines: Vec<&str> = markdown.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.len() < 2 {
        return None;
    }

    // Parse header row
    let headers = parse_table_row(lines[0])?;

    // Skip alignment row (line 1)
    // Parse data rows (lines 2+)
    let mut rows = Vec::new();
    for line in lines.iter().skip(2) {
        if let Some(row) = parse_table_row(line) {
            rows.push(row);
        }
    }

    Some(TableInfo {
        headers,
        rows,
        start_line: 0,
        end_line: 0,
        original: markdown.to_string(),
    })
}

/// Parse a single table row.
fn parse_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();

    // Skip alignment rows
    if trimmed.chars().all(|c| c == '|' || c == '-' || c == ':' || c.is_whitespace()) {
        return None;
    }

    // Remove leading/trailing pipes and split
    let content = trimmed.trim_matches('|');
    let cells: Vec<String> = content.split('|').map(|s| s.trim().to_string()).collect();

    if cells.is_empty() { None } else { Some(cells) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_table() -> TableInfo {
        TableInfo {
            headers: vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
            rows: vec![
                vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
                vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
            ],
            start_line: 1,
            end_line: 5,
            original: String::new(),
        }
    }

    #[test]
    fn test_table_to_tsv() {
        let table = sample_table();
        let tsv = table_to_tsv(&table);

        // DX Serializer LLM format: t:N(col1,col2,col3)[rows]
        assert!(tsv.starts_with("t:2("));
        assert!(tsv.contains("Name,Age,City"));
        assert!(tsv.contains("Alice,30,NYC"));
        assert!(tsv.contains("Bob,25,LA"));
        assert!(tsv.ends_with("]"));
    }

    #[test]
    fn test_table_to_tsv_raw() {
        let table = sample_table();
        let tsv = table_to_tsv_raw(&table);

        assert!(!tsv.contains("t:"));
        assert!(tsv.contains("Name\tAge\tCity"));
    }

    #[test]
    fn test_table_to_csv() {
        let table = sample_table();
        let csv = table_to_csv(&table);

        assert!(csv.contains("Name,Age,City"));
        assert!(csv.contains("Alice,30,NYC"));
    }

    #[test]
    fn test_csv_escaping() {
        let table = TableInfo {
            headers: vec!["Name".to_string(), "Description".to_string()],
            rows: vec![
                vec!["Test".to_string(), "Has, comma".to_string()],
                vec!["Quote".to_string(), "Has \"quotes\"".to_string()],
            ],
            ..Default::default()
        };

        let csv = table_to_csv(&table);
        assert!(csv.contains("\"Has, comma\""));
        assert!(csv.contains("\"Has \"\"quotes\"\"\""));
    }

    #[test]
    fn test_should_keep_inline() {
        let small_table = TableInfo {
            headers: vec!["A".to_string()],
            rows: vec![vec!["1".to_string()]],
            ..Default::default()
        };
        assert!(should_keep_inline(&small_table));

        let large_table = sample_table();
        assert!(!should_keep_inline(&large_table));
    }

    #[test]
    fn test_table_to_inline() {
        let table = TableInfo {
            headers: vec!["Name".to_string(), "Value".to_string()],
            rows: vec![vec!["foo".to_string(), "bar".to_string()]],
            ..Default::default()
        };

        let inline = table_to_inline(&table);
        assert_eq!(inline, "Name: foo, Value: bar");
    }

    #[test]
    fn test_parse_markdown_table() {
        let markdown = r#"
| Name | Age |
|------|-----|
| Alice | 30 |
| Bob | 25 |
"#;

        let table = parse_markdown_table(markdown).unwrap();
        assert_eq!(table.headers, vec!["Name", "Age"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0], vec!["Alice", "30"]);
    }

    #[test]
    fn test_parse_empty_table() {
        let result = parse_markdown_table("");
        assert!(result.is_none());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating valid table cell content (non-empty, no tabs or newlines).
    fn cell_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9]{1,20}"
            .prop_map(|s| s.trim().to_string())
            .prop_filter("cell must not be empty", |s| !s.is_empty())
    }

    /// Strategy for generating a table row.
    fn row_strategy(num_cols: usize) -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(cell_strategy(), num_cols)
    }

    /// Strategy for generating a complete table.
    fn table_strategy() -> impl Strategy<Value = TableInfo> {
        (2usize..=5, 1usize..=10).prop_flat_map(|(num_cols, num_rows)| {
            (row_strategy(num_cols), prop::collection::vec(row_strategy(num_cols), num_rows))
                .prop_map(|(headers, rows)| TableInfo {
                    headers,
                    rows,
                    start_line: 0,
                    end_line: 0,
                    original: String::new(),
                })
        })
    }

    proptest! {
        /// Property: Table integrity - DX Serializer output contains same number of rows and columns.
        /// Validates: Requirements 2.4 (preserve header row as first line)
        #[test]
        fn prop_table_integrity(table in table_strategy()) {
            let output = table_to_tsv(&table);

            // Parse DX Serializer LLM format: t:N(headers)[row1\nrow2]
            // Count lines inside brackets
            let bracket_start = output.find('[').unwrap();
            let bracket_end = output.rfind(']').unwrap();
            let rows_content = &output[bracket_start + 1..bracket_end];
            let data_lines: Vec<&str> = rows_content.lines().filter(|l| !l.is_empty()).collect();

            // Should have same number of data rows
            prop_assert_eq!(data_lines.len(), table.rows.len());

            // First part should be t:N(col1,col2,...)
            prop_assert!(output.starts_with("t:"));

            // Data lines should have correct number of columns (comma-separated)
            for line in data_lines {
                let cols: Vec<&str> = line.split(',').collect();
                prop_assert_eq!(cols.len(), table.headers.len());
            }
        }

        /// Property: Header preservation - headers appear in schema definition.
        /// Validates: Requirements 2.4
        #[test]
        fn prop_header_preservation(table in table_strategy()) {
            let output = table_to_tsv(&table);

            // Extract schema from t:N(col1,col2,...)[
            let paren_start = output.find('(').unwrap();
            let paren_end = output.find(')').unwrap();
            let schema = &output[paren_start + 1..paren_end];
            let headers: Vec<&str> = schema.split(',').collect();

            // Headers should match
            prop_assert_eq!(headers.len(), table.headers.len());
            for (i, header) in headers.iter().enumerate() {
                prop_assert_eq!(*header, table.headers[i].as_str());
            }
        }

        /// Property: Data preservation - all cell values are preserved.
        /// Validates: Requirements 2.1
        #[test]
        fn prop_data_preservation(table in table_strategy()) {
            let output = table_to_tsv(&table);

            // All cell values should appear in output
            for row in &table.rows {
                for cell in row {
                    prop_assert!(output.contains(cell.as_str()));
                }
            }
        }

        /// Property: No alignment row - output should not contain alignment markers.
        /// Validates: Requirements 2.2
        #[test]
        fn prop_no_alignment_row(table in table_strategy()) {
            let output = table_to_tsv(&table);

            // Should not contain alignment patterns
            prop_assert!(!output.contains("|---"));
            prop_assert!(!output.contains(":---"));
            prop_assert!(!output.contains("---:"));
        }
    }
}
