//! SIMD-Accelerated Tokenizer
//!
//! Uses SIMD instructions for fast pattern matching in rule parsing.

use crate::Result;

/// Token types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TokenType {
    /// Heading (## ...)
    Heading = 0,
    /// Bullet point (- ...)
    Bullet = 1,
    /// Code block
    CodeBlock = 2,
    /// Plain text
    Text = 3,
    /// Whitespace/newline
    Whitespace = 4,
    /// Comment
    Comment = 5,
    /// Key-value pair
    KeyValue = 6,
    /// End of input
    End = 255,
}

/// A token with position information
#[derive(Debug, Clone, Copy)]
pub struct Token {
    /// Token type
    pub ty: TokenType,
    /// Start offset in input
    pub start: u32,
    /// Length in bytes
    pub len: u32,
}

impl Token {
    /// Create a new token
    pub fn new(ty: TokenType, start: u32, len: u32) -> Self {
        Self { ty, start, len }
    }

    /// Get end offset
    pub fn end(&self) -> u32 {
        self.start + self.len
    }

    /// Extract text from input
    pub fn text<'a>(&self, input: &'a [u8]) -> Option<&'a str> {
        let start = self.start as usize;
        let end = self.end() as usize;
        if end > input.len() {
            return None;
        }
        std::str::from_utf8(&input[start..end]).ok()
    }
}

/// SIMD-accelerated tokenizer
#[derive(Debug)]
pub struct SimdTokenizer<'a> {
    /// Input data
    input: &'a [u8],
    /// Current position
    pos: usize,
    /// Line number
    line: u32,
    /// Column number
    col: u32,
}

impl<'a> SimdTokenizer<'a> {
    /// Create a new tokenizer
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Get next token
    pub fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace();

        if self.pos >= self.input.len() {
            return Ok(Token::new(TokenType::End, self.pos as u32, 0));
        }

        let start = self.pos as u32;

        // Check for heading
        if self.starts_with(b"#") {
            return Ok(self.tokenize_heading(start));
        }

        // Check for bullet
        if self.starts_with(b"- ") || self.starts_with(b"* ") || self.starts_with(b"\xE2\x80\xA2 ")
        {
            return Ok(self.tokenize_bullet(start));
        }

        // Check for code block
        if self.starts_with(b"```") {
            return Ok(self.tokenize_code_block(start));
        }

        // Check for key-value
        if let Some(token) = self.try_tokenize_key_value(start) {
            return Ok(token);
        }

        // Default: text until newline
        Ok(self.tokenize_text(start))
    }

    /// Tokenize all input
    pub fn tokenize_all(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_end = token.ty == TokenType::End;
            tokens.push(token);
            if is_end {
                break;
            }
        }

        Ok(tokens)
    }

    /// Find all occurrences of a pattern (SIMD-accelerated when available)
    pub fn find_all(&self, pattern: &[u8]) -> Vec<usize> {
        // Use memchr for SIMD-accelerated single-byte search
        if pattern.len() == 1 {
            return memchr::memchr_iter(pattern[0], self.input).collect();
        }

        // Multi-byte pattern search
        let mut positions = Vec::new();
        let mut pos = 0;

        while pos + pattern.len() <= self.input.len() {
            if &self.input[pos..pos + pattern.len()] == pattern {
                positions.push(pos);
                pos += pattern.len();
            } else {
                pos += 1;
            }
        }

        positions
    }

    /// Count lines efficiently
    pub fn count_lines(&self) -> usize {
        memchr::memchr_iter(b'\n', self.input).count() + 1
    }

    // Private helpers

    fn starts_with(&self, pattern: &[u8]) -> bool {
        self.input[self.pos..].starts_with(pattern)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' => {
                    self.pos += 1;
                    self.col += 1;
                }
                b'\n' => {
                    self.pos += 1;
                    self.line += 1;
                    self.col = 1;
                }
                b'\r' => {
                    self.pos += 1;
                    // Handle CRLF
                    if self.pos < self.input.len() && self.input[self.pos] == b'\n' {
                        self.pos += 1;
                    }
                    self.line += 1;
                    self.col = 1;
                }
                _ => break,
            }
        }
    }

    fn advance_to_newline(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos] != b'\n' {
            self.pos += 1;
            self.col += 1;
        }
    }

    fn tokenize_heading(&mut self, start: u32) -> Token {
        self.advance_to_newline();
        Token::new(TokenType::Heading, start, (self.pos as u32) - start)
    }

    fn tokenize_bullet(&mut self, start: u32) -> Token {
        self.advance_to_newline();
        Token::new(TokenType::Bullet, start, (self.pos as u32) - start)
    }

    fn tokenize_code_block(&mut self, start: u32) -> Token {
        // Skip opening ```
        self.pos += 3;

        // Skip to end of opening line
        self.advance_to_newline();
        if self.pos < self.input.len() {
            self.pos += 1; // Skip newline
        }

        // Find closing ```
        while self.pos + 3 <= self.input.len() {
            if &self.input[self.pos..self.pos + 3] == b"```" {
                self.pos += 3;
                self.advance_to_newline();
                break;
            }
            self.pos += 1;
        }

        Token::new(TokenType::CodeBlock, start, (self.pos as u32) - start)
    }

    fn try_tokenize_key_value(&mut self, start: u32) -> Option<Token> {
        // Look for : in the current line
        let line_end = self.input[self.pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|p| self.pos + p)
            .unwrap_or(self.input.len());

        let line = &self.input[self.pos..line_end];

        if line.contains(&b':') {
            self.pos = line_end;
            return Some(Token::new(TokenType::KeyValue, start, (self.pos as u32) - start));
        }

        None
    }

    fn tokenize_text(&mut self, start: u32) -> Token {
        self.advance_to_newline();
        Token::new(TokenType::Text, start, (self.pos as u32) - start)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_heading() {
        let input = b"## Test Heading\nsome text";
        let mut tokenizer = SimdTokenizer::new(input);

        let token = tokenizer.next_token().unwrap();
        assert_eq!(token.ty, TokenType::Heading);
        assert_eq!(token.text(input), Some("## Test Heading"));
    }

    #[test]
    fn test_tokenize_bullet() {
        let input = b"- Item one\n- Item two";
        let mut tokenizer = SimdTokenizer::new(input);

        let token = tokenizer.next_token().unwrap();
        assert_eq!(token.ty, TokenType::Bullet);
        assert_eq!(token.text(input), Some("- Item one"));
    }

    #[test]
    fn test_find_all() {
        let input = b"hello world hello rust hello";
        let tokenizer = SimdTokenizer::new(input);

        let positions = tokenizer.find_all(b"hello");
        assert_eq!(positions.len(), 3);
    }

    #[test]
    fn test_count_lines() {
        let input = b"line 1\nline 2\nline 3";
        let tokenizer = SimdTokenizer::new(input);

        assert_eq!(tokenizer.count_lines(), 3);
    }
}
