//! Context Compressor
//!
//! Compresses LLM context through summarization and redundancy elimination.

use super::{TokenConfig, TokenError};
use std::collections::HashSet;

/// Context compressor for reducing token usage
pub struct ContextCompressor {
    /// Configuration
    config: TokenConfig,
}

impl ContextCompressor {
    /// Create a new context compressor
    pub fn new(config: TokenConfig) -> Self {
        Self { config }
    }

    /// Compress content through various strategies
    pub async fn compress(&self, content: &str) -> Result<String, TokenError> {
        let mut result = content.to_string();

        // Step 1: Remove redundant whitespace
        result = self.normalize_whitespace(&result);

        // Step 2: Remove duplicate lines
        result = self.remove_duplicates(&result);

        // Step 3: Compress repeated patterns
        result = self.compress_patterns(&result);

        // Step 4: Summarize long sections if needed
        if result.len() > self.config.summarization_threshold {
            result = self.summarize_sections(&result).await?;
        }

        // Step 5: Apply DX-specific optimizations
        result = self.apply_dx_optimizations(&result);

        Ok(result)
    }

    /// Normalize whitespace
    fn normalize_whitespace(&self, content: &str) -> String {
        let mut result = String::with_capacity(content.len());
        let mut prev_whitespace = false;
        let mut prev_newline = false;

        for c in content.chars() {
            if c == '\n' {
                if !prev_newline {
                    result.push('\n');
                    prev_newline = true;
                }
                prev_whitespace = false;
            } else if c.is_whitespace() {
                if !prev_whitespace {
                    result.push(' ');
                    prev_whitespace = true;
                }
                prev_newline = false;
            } else {
                result.push(c);
                prev_whitespace = false;
                prev_newline = false;
            }
        }

        result.trim().to_string()
    }

    /// Remove duplicate lines
    fn remove_duplicates(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut result = Vec::new();

        for line in lines {
            let trimmed = line.trim();
            // Keep blank lines and unique content
            if trimmed.is_empty() || !seen.contains(trimmed) {
                result.push(line);
                if !trimmed.is_empty() {
                    seen.insert(trimmed);
                }
            }
        }

        result.join("\n")
    }

    /// Compress repeated patterns
    fn compress_patterns(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Compress repeated symbols (e.g., "=====" → "=…")
        let patterns = [
            ("=====", "=…"),
            ("-----", "-…"),
            ("*****", "*…"),
            ("_____", "_…"),
            ("#####", "#…"),
            ("     ", " "),
        ];

        for (pattern, replacement) in patterns {
            while result.contains(pattern) {
                result = result.replace(pattern, replacement);
            }
        }

        result
    }

    /// Summarize long sections
    async fn summarize_sections(&self, content: &str) -> Result<String, TokenError> {
        let sections = self.split_into_sections(content);
        let mut result = Vec::new();

        for section in sections {
            if section.len() > self.config.summarization_threshold / 4 {
                // For very long sections, add a summary marker
                let summary = self.generate_section_summary(&section);
                result.push(summary);
            } else {
                result.push(section);
            }
        }

        Ok(result.join("\n\n"))
    }

    /// Split content into logical sections
    fn split_into_sections(&self, content: &str) -> Vec<String> {
        let mut sections = Vec::new();
        let mut current_section = String::new();

        for line in content.lines() {
            // Section breaks: headers, blank lines, etc.
            if line.starts_with('#') || line.starts_with("##") || line.trim().is_empty() {
                if !current_section.is_empty() {
                    sections.push(current_section);
                    current_section = String::new();
                }
            }
            current_section.push_str(line);
            current_section.push('\n');
        }

        if !current_section.is_empty() {
            sections.push(current_section);
        }

        sections
    }

    /// Generate a summary for a section
    fn generate_section_summary(&self, section: &str) -> String {
        let lines: Vec<&str> = section.lines().collect();
        let line_count = lines.len();

        // Extract key information
        let _first_line = lines.first().map(|s| s.trim()).unwrap_or("");
        let word_count = section.split_whitespace().count();

        // For now, truncate with summary info
        // Real implementation would use an LLM for summarization
        let preview = if section.len() > 200 {
            format!("{}...", &section[..200])
        } else {
            section.to_string()
        };

        format!("[Section: {} lines, ~{} words]\n{}", line_count, word_count, preview)
    }

    /// Apply DX-specific optimizations
    fn apply_dx_optimizations(&self, content: &str) -> String {
        let mut result = content.to_string();

        // Compress common programming patterns
        let optimizations = [
            // JSON to compact form
            ("{\n  ", "{"),
            ("\n  }", "}"),
            ("\",\n  \"", "\",\""),
            // Common phrases
            ("function ", "fn "),
            ("return ", "ret "),
            ("const ", "c "),
            // Verbose comments
            ("// TODO:", "TODO:"),
            ("// FIXME:", "FIX:"),
            ("// NOTE:", "NOTE:"),
        ];

        for (pattern, replacement) in optimizations {
            result = result.replace(pattern, replacement);
        }

        result
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self, original: &str, compressed: &str) -> f32 {
        if original.is_empty() {
            return 1.0;
        }
        1.0 - (compressed.len() as f32 / original.len() as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> TokenConfig {
        TokenConfig {
            summarization_threshold: 100,
            ..Default::default()
        }
    }

    #[test]
    fn test_normalize_whitespace() {
        let compressor = ContextCompressor::new(test_config());

        let input = "Hello    world\n\n\n\nTest";
        let result = compressor.normalize_whitespace(input);
        assert_eq!(result, "Hello world\n\nTest");
    }

    #[test]
    fn test_remove_duplicates() {
        let compressor = ContextCompressor::new(test_config());

        let input = "line 1\nline 2\nline 1\nline 3";
        let result = compressor.remove_duplicates(input);
        assert_eq!(result, "line 1\nline 2\nline 3");
    }

    #[test]
    fn test_compress_patterns() {
        let compressor = ContextCompressor::new(test_config());

        let input = "Header\n==========\nContent";
        let result = compressor.compress_patterns(input);
        assert!(result.contains("=…"));
        assert!(!result.contains("=========="));
    }

    #[test]
    fn test_dx_optimizations() {
        let compressor = ContextCompressor::new(test_config());

        let input = "function test() { return value; }";
        let result = compressor.apply_dx_optimizations(input);
        assert!(result.contains("fn "));
        assert!(result.contains("ret "));
    }

    #[test]
    fn test_compression_ratio() {
        let compressor = ContextCompressor::new(test_config());

        let original = "Hello world this is a test";
        let compressed = "Hello world";

        let ratio = compressor.compression_ratio(original, compressed);
        assert!(ratio > 0.0);
        assert!(ratio < 1.0);
    }

    #[tokio::test]
    async fn test_compress() {
        let compressor = ContextCompressor::new(test_config());

        let input = "Hello    world\n\n\nTest\n=====\nContent";
        let result = compressor.compress(input).await.unwrap();

        assert!(result.len() <= input.len());
    }
}
