//! Analysis pass for the DX Markdown Context Compiler.
//!
//! This module implements the first pass of the compiler, which analyzes
//! the input Markdown to gather statistics and identify optimization opportunities.

use crate::error::CompileError;
use crate::tokenizer::Tokenizer;
use crate::types::{AnalysisResult, BadgeInfo, CodeBlockInfo, TableInfo, TokenizerType, UrlInfo};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;

/// Minimum word length to consider for frequency analysis.
const MIN_WORD_LENGTH: usize = 4;

/// Minimum occurrences to consider for dictionary hoisting.
const MIN_OCCURRENCES: usize = 2;

/// Analyze markdown input for optimization opportunities.
pub fn analyze(input: &str, tokenizer_type: TokenizerType) -> Result<AnalysisResult, CompileError> {
    let tokenizer = Tokenizer::new(tokenizer_type)?;
    let mut result = AnalysisResult::default();

    result.token_count = tokenizer.count(input);

    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let parser = Parser::new_ext(input, options);

    let mut in_table = false;
    let mut in_code_block = false;
    let mut current_table: Option<TableInfo> = None;
    let mut current_code_block: Option<CodeBlockInfo> = None;
    let mut current_link_text = String::new();
    let mut current_link_url = String::new();
    let mut in_link = false;
    let mut in_image = false;
    let mut current_image_alt = String::new();
    let mut line_number = 1;
    let mut table_row: Vec<String> = Vec::new();
    let mut in_table_head = false;
    let mut current_cell = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                current_table = Some(TableInfo {
                    start_line: line_number,
                    ..Default::default()
                });
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                if let Some(mut table) = current_table.take() {
                    table.end_line = line_number;
                    result.tables.push(table);
                }
            }
            Event::Start(Tag::TableHead) => {
                in_table_head = true;
                table_row.clear();
            }
            Event::End(TagEnd::TableHead) => {
                in_table_head = false;
                if let Some(ref mut table) = current_table {
                    table.headers = table_row.clone();
                }
                table_row.clear();
            }
            Event::Start(Tag::TableRow) => {
                table_row.clear();
            }
            Event::End(TagEnd::TableRow) => {
                if !in_table_head && let Some(ref mut table) = current_table {
                    table.rows.push(table_row.clone());
                }
                table_row.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
            }
            Event::End(TagEnd::TableCell) => {
                table_row.push(current_cell.clone());
                current_cell.clear();
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                let language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.to_string();
                        if lang_str.is_empty() {
                            None
                        } else {
                            Some(lang_str)
                        }
                    }
                    pulldown_cmark::CodeBlockKind::Indented => None,
                };
                current_code_block = Some(CodeBlockInfo {
                    language,
                    start_line: line_number,
                    ..Default::default()
                });
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                if let Some(mut cb) = current_code_block.take() {
                    cb.end_line = line_number;
                    result.code_blocks.push(cb);
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                current_link_url = dest_url.to_string();
                current_link_text.clear();
            }
            Event::End(TagEnd::Link) => {
                in_link = false;
                result.urls.push(UrlInfo {
                    text: current_link_text.clone(),
                    url: current_link_url.clone(),
                    line: line_number,
                });
                current_link_text.clear();
            }
            Event::Start(Tag::Image {
                dest_url, title, ..
            }) => {
                in_image = true;
                current_image_alt.clear();
                let url = dest_url.to_string();
                let is_badge = url.contains("badge")
                    || url.contains("shields.io")
                    || url.contains("img.shields.io");
                result.badges.push(BadgeInfo {
                    alt: title.to_string(),
                    url,
                    is_badge,
                    line: line_number,
                });
            }
            Event::End(TagEnd::Image) => {
                in_image = false;
                if let Some(badge) = result.badges.last_mut()
                    && badge.alt.is_empty()
                {
                    badge.alt = current_image_alt.clone();
                }
                current_image_alt.clear();
            }
            Event::Text(text) => {
                let text_str = text.to_string();
                line_number += text_str.matches('\n').count();

                if in_link {
                    current_link_text.push_str(&text_str);
                }
                if in_image {
                    current_image_alt.push_str(&text_str);
                }
                if in_table {
                    current_cell.push_str(&text_str);
                }
                if let Some(ref mut cb) = current_code_block {
                    cb.content.push_str(&text_str);
                }
                if !in_code_block {
                    analyze_text_frequencies(&text_str, &mut result.frequencies);
                }
            }
            Event::Code(code) => {
                let code_str = code.to_string();
                if in_link {
                    current_link_text.push_str(&code_str);
                }
                if in_table {
                    current_cell.push_str(&code_str);
                }
                // Also analyze inline code for frequency (e.g., repeated format names like `json-compact`)
                if !in_code_block && code_str.len() >= MIN_WORD_LENGTH {
                    *result.frequencies.entry(code_str.clone()).or_insert(0) += 1;
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                line_number += 1;
            }
            _ => {}
        }
    }

    Ok(result)
}

fn analyze_text_frequencies(text: &str, frequencies: &mut HashMap<String, usize>) {
    let words: Vec<&str> = text.split_whitespace().collect();

    // Single words
    for word in &words {
        let cleaned = word.trim_matches(|c: char| !c.is_alphanumeric());
        if cleaned.len() >= MIN_WORD_LENGTH {
            *frequencies.entry(cleaned.to_string()).or_insert(0) += 1;
        }
    }

    // 2-word phrases (bigrams)
    for window in words.windows(2) {
        let phrase = format!("{} {}", window[0], window[1]);
        let cleaned = phrase.trim_matches(|c: char| !c.is_alphanumeric() && c != ' ');
        if cleaned.len() >= MIN_WORD_LENGTH && cleaned.contains(' ') {
            *frequencies.entry(cleaned.to_string()).or_insert(0) += 1;
        }
    }

    // 3-word phrases (trigrams) for longer repeated phrases
    for window in words.windows(3) {
        let phrase = format!("{} {} {}", window[0], window[1], window[2]);
        let cleaned = phrase.trim_matches(|c: char| !c.is_alphanumeric() && c != ' ');
        if cleaned.len() >= 10 && cleaned.contains(' ') {
            *frequencies.entry(cleaned.to_string()).or_insert(0) += 1;
        }
    }
}

/// Get candidates for dictionary hoisting.
pub fn get_dictionary_candidates(
    result: &AnalysisResult,
    tokenizer: &Tokenizer,
) -> Vec<(String, usize)> {
    let mut candidates: Vec<(String, usize)> = result
        .frequencies
        .iter()
        .filter(|&(phrase, &count)| {
            count >= MIN_OCCURRENCES && tokenizer.should_replace(phrase, count, "$A")
        })
        .map(|(phrase, &count)| (phrase.clone(), count))
        .collect();

    candidates.sort_by(|a, b| {
        let savings_a = a.1 * tokenizer.count(&a.0);
        let savings_b = b.1 * tokenizer.count(&b.0);
        savings_b.cmp(&savings_a)
    });

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_empty() {
        let result = analyze("", TokenizerType::Cl100k).unwrap();
        assert_eq!(result.token_count, 0);
        assert!(result.tables.is_empty());
        assert!(result.code_blocks.is_empty());
    }

    #[test]
    fn test_analyze_simple_text() {
        let input = "Hello world, this is a test.";
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        assert!(result.token_count > 0);
    }

    #[test]
    fn test_analyze_table() {
        let input = r#"
| Name | Age |
|------|-----|
| Alice | 30 |
| Bob | 25 |
"#;
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.tables[0].headers.len(), 2);
        assert_eq!(result.tables[0].rows.len(), 2);
    }

    #[test]
    fn test_analyze_code_block() {
        let input = r#"
```rust
fn main() {
    println!("Hello");
}
```
"#;
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        assert_eq!(result.code_blocks.len(), 1);
        assert_eq!(result.code_blocks[0].language, Some("rust".to_string()));
        assert!(result.code_blocks[0].content.contains("println"));
    }

    #[test]
    fn test_analyze_links() {
        let input = "Check the [documentation](https://example.com/docs) for more info.";
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        assert_eq!(result.urls.len(), 1);
        assert_eq!(result.urls[0].text, "documentation");
        assert_eq!(result.urls[0].url, "https://example.com/docs");
    }

    #[test]
    fn test_analyze_images() {
        let input = "![Logo](https://example.com/logo.png)";
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        assert_eq!(result.badges.len(), 1);
        assert_eq!(result.badges[0].url, "https://example.com/logo.png");
    }

    #[test]
    fn test_analyze_badges() {
        let input = "![Build](https://img.shields.io/badge/build-passing-green)";
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        assert!(!result.badges.is_empty());
        assert!(result.badges[0].is_badge);
    }

    #[test]
    fn test_analyze_frequencies() {
        let input = "The authentication system handles authentication. Authentication is important. We use authentication everywhere.";
        let result = analyze(input, TokenizerType::Cl100k).unwrap();
        // Should find "authentication" multiple times (case-insensitive)
        let auth_count = result.frequencies.get("authentication").unwrap_or(&0);
        assert!(
            auth_count >= &3,
            "Expected at least 3 occurrences of 'authentication', found {}",
            auth_count
        );
    }
}
