//! Zero-copy tokenizer for DX Machine format
//!
//! Uses SIMD-accelerated scanning via memchr for maximum performance.
//! Operates directly on byte slices without allocations.

use crate::error::{DxError, Result};
use crate::utf8::validate_string_input;
use memchr::memchr;

/// Token types in DX format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    /// End of input
    Eof,
    /// Newline
    Newline,
    /// Colon (:)
    Colon,
    /// Equals (=) - Schema definition
    Equals,
    /// Caret (^) - Prefix inheritance
    Caret,
    /// Pipe (|) - Array delimiter
    Pipe,
    /// Greater than (>) - Stream operator
    Stream,
    /// Underscore (_) - Ditto
    Ditto,
    /// Tilde (~) - Null
    Null,
    /// Plus (+) - True
    True,
    /// Minus (-) - False
    False,
    /// Bang (!) - Implicit true flag
    Bang,
    /// Question (?) - Implicit null
    Void,
    /// Dollar ($) - Alias definition
    Dollar,
    /// At (@) - Anchor reference
    At,
    /// Percent (%) - Type hint
    Percent,
    /// Dot (.) - Path separator
    Dot,
    /// Identifier/String
    Ident(&'a [u8]),
    /// Integer
    Int(i64),
    /// Float
    Float(f64),
}

/// Zero-copy tokenizer
pub struct Tokenizer<'a> {
    pub(crate) input: &'a [u8],
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    /// Create new tokenizer from bytes
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }

    /// Get current position
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Reset to a saved position
    pub fn reset_to(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Check if at end
    pub fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Peek current byte without advancing
    pub fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    /// Peek n bytes ahead
    pub fn peek_n(&self, n: usize) -> Option<u8> {
        self.input.get(self.pos + n).copied()
    }

    /// Advance by n bytes
    pub fn advance(&mut self, n: usize) {
        self.pos = (self.pos + n).min(self.input.len());
    }

    /// Skip whitespace (space and tab only, not newlines)
    pub fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Skip to end of line (for comments)
    pub fn skip_line(&mut self) {
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'\n' {
                break;
            }
        }
    }

    /// Read until delimiter using SIMD search
    pub fn read_until(&mut self, delim: u8) -> &'a [u8] {
        let start = self.pos;
        let remaining = &self.input[self.pos..];

        match memchr(delim, remaining) {
            Some(offset) => {
                self.pos += offset;
                &self.input[start..self.pos]
            }
            None => {
                self.pos = self.input.len();
                &self.input[start..]
            }
        }
    }

    /// Read until any of the delimiters
    pub fn read_until_any(&mut self, delims: &[u8]) -> &'a [u8] {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if delims.contains(&b) {
                break;
            }
            self.pos += 1;
        }
        &self.input[start..self.pos]
    }

    /// Read identifier (alphanumeric + underscore)
    pub fn read_ident(&mut self) -> &'a [u8] {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-' {
                self.pos += 1;
            } else {
                break;
            }
        }
        &self.input[start..self.pos]
    }

    /// Read number (int or float)
    pub fn read_number(&mut self) -> Result<Token<'a>> {
        let start = self.pos;
        let mut has_dot = false;
        let mut has_exp = false;

        // Handle negative sign
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }

        // Read digits, dots, and exponents
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'.' if !has_dot && !has_exp => {
                    has_dot = true;
                    self.pos += 1;
                }
                b'e' | b'E' if !has_exp => {
                    has_exp = true;
                    self.pos += 1;
                    // Handle optional +/- after exponent
                    if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }

        let num_bytes = &self.input[start..self.pos];
        let num_str = std::str::from_utf8(num_bytes)
            .map_err(|_| DxError::InvalidNumber("Invalid UTF-8 in number".to_string()))?;

        if has_dot || has_exp {
            let val = num_str
                .parse::<f64>()
                .map_err(|_| DxError::InvalidNumber(num_str.to_string()))?;
            Ok(Token::Float(val))
        } else {
            let val = num_str
                .parse::<i64>()
                .map_err(|_| DxError::InvalidNumber(num_str.to_string()))?;
            Ok(Token::Int(val))
        }
    }

    /// Read string value using vacuum parsing
    /// Reads until the next expected type based on schema
    pub fn read_string_vacuum(&mut self, next_is_number: bool) -> &'a [u8] {
        let start = self.pos;

        if next_is_number {
            // Read until we hit a number or delimiter
            while let Some(b) = self.peek() {
                if b.is_ascii_digit() || b == b'-' || b == b'|' || b == b'\n' {
                    break;
                }
                self.pos += 1;
            }
        } else {
            // Read until delimiter or boolean marker
            // Stop at + or - (boolean markers) when preceded by whitespace
            while let Some(b) = self.peek() {
                if b == b'|' || b == b'\n' || b == b'#' {
                    break;
                }
                // Check for boolean markers preceded by whitespace
                // Only stop if we've read at least one character and the previous char is whitespace
                if (b == b'+' || b == b'-') && self.pos > start {
                    let prev = self.input[self.pos - 1];
                    if prev.is_ascii_whitespace() {
                        break;
                    }
                }
                self.pos += 1;
            }
        }

        // Trim trailing whitespace
        let mut end = self.pos;
        while end > start && self.input[end - 1].is_ascii_whitespace() {
            end -= 1;
        }

        &self.input[start..end]
    }

    /// Read string value with UTF-8 validation
    /// Returns an error if the string contains invalid UTF-8
    pub fn read_string_validated(&mut self, next_is_number: bool) -> Result<&'a str> {
        let start = self.pos;
        let bytes = self.read_string_vacuum(next_is_number);
        validate_string_input(bytes, start)
    }

    /// Read identifier with UTF-8 validation
    pub fn read_ident_validated(&mut self) -> Result<&'a str> {
        let start = self.pos;
        let bytes = self.read_ident();
        validate_string_input(bytes, start)
    }

    /// Get next token
    pub fn next_token(&mut self) -> Result<Token<'a>> {
        self.skip_whitespace();

        let b = match self.peek() {
            Some(b) => b,
            None => return Ok(Token::Eof),
        };

        // Check for single-char tokens
        let token = match b {
            b'\n' | b'\r' => {
                self.pos += 1;
                if b == b'\r' && self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
                return Ok(Token::Newline);
            }
            b'#' => {
                self.skip_line();
                return self.next_token(); // Skip comment and get next
            }
            b':' => {
                self.pos += 1;
                Token::Colon
            }
            b'=' => {
                self.pos += 1;
                Token::Equals
            }
            b'^' => {
                self.pos += 1;
                Token::Caret
            }
            b'|' => {
                self.pos += 1;
                Token::Pipe
            }
            b'>' => {
                self.pos += 1;
                Token::Stream
            }
            b'_' => {
                self.pos += 1;
                Token::Ditto
            }
            b'~' => {
                self.pos += 1;
                Token::Null
            }
            b'+' => {
                self.pos += 1;
                Token::True
            }
            b'-' if !matches!(self.peek_n(1), Some(b'0'..=b'9')) => {
                self.pos += 1;
                Token::False
            }
            b'!' => {
                self.pos += 1;
                Token::Bang
            }
            b'?' => {
                self.pos += 1;
                Token::Void
            }
            b'$' => {
                self.pos += 1;
                Token::Dollar
            }
            b'@' => {
                self.pos += 1;
                Token::At
            }
            b'%' => {
                self.pos += 1;
                Token::Percent
            }
            b'.' => {
                self.pos += 1;
                Token::Dot
            }
            b'0'..=b'9' | b'-' => {
                return self.read_number();
            }
            _ => {
                let ident = self.read_ident();
                Token::Ident(ident)
            }
        };

        Ok(token)
    }

    /// Peek next token without consuming
    pub fn peek_token(&self) -> Result<Token<'a>> {
        let mut temp = Self {
            input: self.input,
            pos: self.pos,
        };
        temp.next_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_basic() {
        let input = b"id:123 name:Test active:+ score:9.5";
        let mut tok = Tokenizer::new(input);

        assert_eq!(tok.next_token().unwrap(), Token::Ident(b"id"));
        assert_eq!(tok.next_token().unwrap(), Token::Colon);
        assert_eq!(tok.next_token().unwrap(), Token::Int(123));
        assert_eq!(tok.next_token().unwrap(), Token::Ident(b"name"));
        assert_eq!(tok.next_token().unwrap(), Token::Colon);
    }

    #[test]
    fn test_special_tokens() {
        let input = b"_ ~ + - ^ | > = $ @ % !";
        let mut tok = Tokenizer::new(input);

        assert_eq!(tok.next_token().unwrap(), Token::Ditto);
        assert_eq!(tok.next_token().unwrap(), Token::Null);
        assert_eq!(tok.next_token().unwrap(), Token::True);
        assert_eq!(tok.next_token().unwrap(), Token::False);
        assert_eq!(tok.next_token().unwrap(), Token::Caret);
        assert_eq!(tok.next_token().unwrap(), Token::Pipe);
        assert_eq!(tok.next_token().unwrap(), Token::Stream);
        assert_eq!(tok.next_token().unwrap(), Token::Equals);
    }

    #[test]
    #[allow(clippy::approx_constant)] // Using 3.14 intentionally for test data
    fn test_numbers() {
        let input = b"123 -456 3.14 -2.5 1e10";
        let mut tok = Tokenizer::new(input);

        assert_eq!(tok.next_token().unwrap(), Token::Int(123));
        assert_eq!(tok.next_token().unwrap(), Token::Int(-456));
        assert_eq!(tok.next_token().unwrap(), Token::Float(3.14));
        assert_eq!(tok.next_token().unwrap(), Token::Float(-2.5));
    }
}
