// Markdown table parser

use super::*;
use std::borrow::Cow;

/// Parses standard Markdown tables
pub struct TableParser;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("table has too few rows")]
    TooFewRows,
    #[error("empty row in table")]
    EmptyRow,
    #[error("invalid structure")]
    InvalidStructure,
    #[error("parse error: {0}")]
    Other(String),
}

impl TableParser {
    /// Parse a markdown table into structured data
    pub fn parse(input: &str) -> Result<ParsedStructure<'_>, ParseError> {
        let lines: Vec<&str> = input.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();

        if lines.len() < 2 {
            return Err(ParseError::TooFewRows);
        }

        // Parse header row
        let header_line = lines[0];
        let columns = Self::parse_row(header_line)?;

        // Parse separator row (for alignment)
        let alignments = if lines.len() > 1 && Self::is_separator(lines[1]) {
            Some(Self::parse_alignments(lines[1])?)
        } else {
            None
        };

        // Parse data rows
        let data_start = if alignments.is_some() { 2 } else { 1 };
        let mut rows = Vec::with_capacity(lines.len() - data_start);

        for line in &lines[data_start..] {
            if !Self::is_separator(line) {
                rows.push(Self::parse_row(line)?);
            }
        }

        // Detect structure subtype
        let structure_type = Self::detect_subtype(&columns, &rows);

        Ok(ParsedStructure {
            structure_type,
            name: None,
            data: StructureData::Table {
                columns,
                rows,
                alignments,
            },
            original_len: input.len(),
        })
    }

    /// Parse a single row into cells
    fn parse_row(line: &str) -> Result<Vec<Cow<'_, str>>, ParseError> {
        let line = line.trim_start_matches('|').trim_end_matches('|');

        let cells: Vec<Cow<'_, str>> = line
            .split('|')
            .map(|cell| {
                let trimmed = cell.trim();
                if trimmed.contains(' ') && !trimmed.contains("http") {
                    Cow::Owned(trimmed.replace(' ', "_"))
                } else {
                    Cow::Borrowed(trimmed)
                }
            })
            .collect();

        if cells.is_empty() {
            return Err(ParseError::EmptyRow);
        }

        Ok(cells)
    }

    /// Check if line is a separator row
    fn is_separator(line: &str) -> bool {
        let cleaned = line.replace(['|', ':', ' '], "");
        cleaned.chars().all(|c| c == '-')
    }

    /// Parse alignments from separator row
    fn parse_alignments(line: &str) -> Result<Vec<Alignment>, ParseError> {
        let line = line.trim_start_matches('|').trim_end_matches('|');

        let alignments: Vec<Alignment> = line
            .split('|')
            .map(|cell| {
                let cell = cell.trim();
                let left_colon = cell.starts_with(':');
                let right_colon = cell.ends_with(':');

                match (left_colon, right_colon) {
                    (true, true) => Alignment::Center,
                    (false, true) => Alignment::Right,
                    _ => Alignment::Left,
                }
            })
            .collect();

        Ok(alignments)
    }

    /// Detect table subtype based on content
    fn detect_subtype(_columns: &[Cow<'_, str>], rows: &[Vec<Cow<'_, str>>]) -> StructureType {
        let has_urls = rows.iter().any(|row| {
            row.iter().any(|cell| cell.contains("http://") || cell.contains("https://"))
        });
        if has_urls {
            return StructureType::TableWithLinks;
        }

        let has_checks = rows.iter().any(|row| {
            row.iter().any(|cell| {
                cell.contains('✅')
                    || cell.contains('❌')
                    || cell.contains('✓')
                    || cell.contains('✗')
                    || cell == "y"
                    || cell == "n"
                    || cell == "yes"
                    || cell == "no"
            })
        });
        if has_checks {
            return StructureType::TableFeatureMatrix;
        }

        StructureType::Table
    }
}
