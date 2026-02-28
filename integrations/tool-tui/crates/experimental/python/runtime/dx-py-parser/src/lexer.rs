//! Python lexer with indentation handling

use crate::error::{Location, ParseError, ParseResult};
use crate::token::{Token, TokenKind};

/// Python lexer that handles indentation-based syntax
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    line: usize,
    column: usize,
    offset: usize,
    indent_stack: Vec<usize>,
    pending_tokens: Vec<Token>,
    at_line_start: bool,
    paren_depth: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given source
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
            offset: 0,
            indent_stack: vec![0],
            pending_tokens: Vec::new(),
            at_line_start: true,
            paren_depth: 0,
        }
    }

    /// Get current location
    fn location(&self) -> Location {
        Location::new(self.line, self.column, self.offset)
    }

    /// Peek at the next character
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    /// Peek at the second character
    fn peek2(&self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().map(|(_, c)| c)
    }

    /// Advance to the next character
    fn advance(&mut self) -> Option<char> {
        if let Some((offset, c)) = self.chars.next() {
            self.offset = offset + c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    /// Check if at end of file
    pub fn is_eof(&mut self) -> bool {
        self.pending_tokens.is_empty() && self.peek().is_none()
    }

    /// Get the next token
    pub fn next_token(&mut self) -> ParseResult<Token> {
        // Return pending tokens first (INDENT/DEDENT)
        if let Some(token) = self.pending_tokens.pop() {
            return Ok(token);
        }

        // Handle indentation at line start
        if self.at_line_start {
            self.handle_indentation()?;
            if let Some(token) = self.pending_tokens.pop() {
                return Ok(token);
            }
        }

        self.skip_whitespace_and_comments();

        let location = self.location();

        match self.peek() {
            None => {
                // Generate DEDENT tokens for remaining indentation
                while self.indent_stack.len() > 1 {
                    self.indent_stack.pop();
                    self.pending_tokens.push(Token::new(TokenKind::Dedent, location));
                }
                if let Some(token) = self.pending_tokens.pop() {
                    return Ok(token);
                }
                Ok(Token::new(TokenKind::Eof, location))
            }
            Some(c) => {
                if c == '\n' {
                    self.advance();
                    self.at_line_start = true;
                    // Only emit NEWLINE if not inside parentheses
                    if self.paren_depth == 0 {
                        return Ok(Token::new(TokenKind::Newline, location));
                    } else {
                        return self.next_token();
                    }
                }

                if c.is_ascii_digit() {
                    return self.scan_number();
                }

                if c == '"' || c == '\'' {
                    return self.scan_string();
                }

                if c == 'f' || c == 'F' || c == 'r' || c == 'R' || c == 'b' || c == 'B' {
                    if let Some(next) = self.peek2() {
                        if next == '"' || next == '\'' {
                            return self.scan_string();
                        }
                    }
                }

                if is_identifier_start(c) {
                    return self.scan_identifier();
                }

                self.scan_operator()
            }
        }
    }

    /// Handle indentation at the start of a line
    fn handle_indentation(&mut self) -> ParseResult<()> {
        self.at_line_start = false;
        let location = self.location();

        // Count leading spaces/tabs
        let mut indent = 0;
        while let Some(c) = self.peek() {
            match c {
                ' ' => {
                    indent += 1;
                    self.advance();
                }
                '\t' => {
                    // Tab = 8 spaces (Python default)
                    indent = (indent / 8 + 1) * 8;
                    self.advance();
                }
                '\n' | '#' => {
                    // Blank line or comment-only line, skip
                    return Ok(());
                }
                _ => break,
            }
        }

        // Skip blank lines
        if self.peek().is_none() {
            return Ok(());
        }

        let current_indent = *self.indent_stack.last().unwrap();

        if indent > current_indent {
            self.indent_stack.push(indent);
            self.pending_tokens.push(Token::new(TokenKind::Indent, location));
        } else if indent < current_indent {
            // Generate DEDENT tokens
            while let Some(&top) = self.indent_stack.last() {
                if top <= indent {
                    break;
                }
                self.indent_stack.pop();
                self.pending_tokens.push(Token::new(TokenKind::Dedent, location));
            }

            // Check for inconsistent indentation
            if *self.indent_stack.last().unwrap() != indent {
                return Err(ParseError::indentation_error(
                    location,
                    "unindent does not match any outer indentation level",
                ));
            }
        }

        Ok(())
    }

    /// Skip whitespace and comments
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('#') => {
                    // Skip comment until end of line
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('\\') => {
                    // Line continuation
                    self.advance();
                    if self.peek() == Some('\n') {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    /// Scan a number literal
    fn scan_number(&mut self) -> ParseResult<Token> {
        let location = self.location();
        let start = self.offset;

        // Check for hex, octal, binary
        if self.peek() == Some('0') {
            self.advance();
            match self.peek() {
                Some('x') | Some('X') => {
                    self.advance();
                    return self.scan_hex_number(location);
                }
                Some('o') | Some('O') => {
                    self.advance();
                    return self.scan_octal_number(location);
                }
                Some('b') | Some('B') => {
                    self.advance();
                    return self.scan_binary_number(location);
                }
                _ => {}
            }
        }

        // Scan integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        // Check for float
        let is_float = if self.peek() == Some('.') {
            if let Some(next) = self.peek2() {
                if next.is_ascii_digit() {
                    self.advance(); // consume '.'
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() || c == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        // Check for exponent
        let has_exponent = if let Some('e') | Some('E') = self.peek() {
            self.advance();
            if let Some('+') | Some('-') = self.peek() {
                self.advance();
            }
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            true
        } else {
            false
        };

        let text = &self.source[start..self.offset];
        let clean_text: String = text.chars().filter(|&c| c != '_').collect();

        if is_float || has_exponent {
            let value: f64 = clean_text.parse().map_err(|_| ParseError::InvalidNumber {
                location,
                message: format!("invalid float literal: {}", text),
            })?;
            Ok(Token::new(TokenKind::Float(value), location))
        } else {
            let value: i64 = clean_text.parse().map_err(|_| ParseError::InvalidNumber {
                location,
                message: format!("invalid integer literal: {}", text),
            })?;
            Ok(Token::new(TokenKind::Integer(value), location))
        }
    }

    fn scan_hex_number(&mut self, location: Location) -> ParseResult<Token> {
        let start = self.offset;
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let text: String = self.source[start..self.offset].chars().filter(|&c| c != '_').collect();
        let value = i64::from_str_radix(&text, 16).map_err(|_| ParseError::InvalidNumber {
            location,
            message: format!("invalid hex literal: 0x{}", text),
        })?;
        Ok(Token::new(TokenKind::Integer(value), location))
    }

    fn scan_octal_number(&mut self, location: Location) -> ParseResult<Token> {
        let start = self.offset;
        while let Some(c) = self.peek() {
            if ('0'..='7').contains(&c) || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let text: String = self.source[start..self.offset].chars().filter(|&c| c != '_').collect();
        let value = i64::from_str_radix(&text, 8).map_err(|_| ParseError::InvalidNumber {
            location,
            message: format!("invalid octal literal: 0o{}", text),
        })?;
        Ok(Token::new(TokenKind::Integer(value), location))
    }

    fn scan_binary_number(&mut self, location: Location) -> ParseResult<Token> {
        let start = self.offset;
        while let Some(c) = self.peek() {
            if c == '0' || c == '1' || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        let text: String = self.source[start..self.offset].chars().filter(|&c| c != '_').collect();
        let value = i64::from_str_radix(&text, 2).map_err(|_| ParseError::InvalidNumber {
            location,
            message: format!("invalid binary literal: 0b{}", text),
        })?;
        Ok(Token::new(TokenKind::Integer(value), location))
    }

    /// Scan a string literal
    fn scan_string(&mut self) -> ParseResult<Token> {
        let location = self.location();
        let mut is_fstring = false;
        let mut is_raw = false;
        let mut is_bytes = false;

        // Check for prefix
        while let Some(c) = self.peek() {
            match c {
                'f' | 'F' => {
                    is_fstring = true;
                    self.advance();
                }
                'r' | 'R' => {
                    is_raw = true;
                    self.advance();
                }
                'b' | 'B' => {
                    is_bytes = true;
                    self.advance();
                }
                '"' | '\'' => break,
                _ => break,
            }
        }

        let quote = self
            .advance()
            .ok_or_else(|| ParseError::unexpected_eof(location, "expected string quote"))?;

        // Check for triple-quoted string
        let triple = if self.peek() == Some(quote) {
            self.advance();
            if self.peek() == Some(quote) {
                self.advance();
                true
            } else {
                // Empty string ""
                return if is_fstring {
                    Ok(Token::new(TokenKind::FString(String::new()), location))
                } else if is_bytes {
                    Ok(Token::new(TokenKind::Bytes(Vec::new()), location))
                } else {
                    Ok(Token::new(TokenKind::String(String::new()), location))
                };
            }
        } else {
            false
        };

        let mut value = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(ParseError::InvalidString {
                        location,
                        message: "unterminated string literal".to_string(),
                    });
                }
                Some(c) if c == quote => {
                    self.advance();
                    if triple {
                        if self.peek() == Some(quote) {
                            self.advance();
                            if self.peek() == Some(quote) {
                                self.advance();
                                break;
                            } else {
                                value.push(quote);
                                value.push(quote);
                            }
                        } else {
                            value.push(quote);
                        }
                    } else {
                        break;
                    }
                }
                Some('\n') if !triple => {
                    return Err(ParseError::InvalidString {
                        location,
                        message: "EOL while scanning string literal".to_string(),
                    });
                }
                Some('\\') if !is_raw => {
                    self.advance();
                    match self.advance() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('r') => value.push('\r'),
                        Some('\\') => value.push('\\'),
                        Some('\'') => value.push('\''),
                        Some('"') => value.push('"'),
                        Some('0') => value.push('\0'),
                        Some(c) => {
                            value.push('\\');
                            value.push(c);
                        }
                        None => {
                            return Err(ParseError::InvalidString {
                                location,
                                message: "unterminated escape sequence".to_string(),
                            });
                        }
                    }
                }
                Some(c) => {
                    self.advance();
                    value.push(c);
                }
            }
        }

        if is_fstring {
            Ok(Token::new(TokenKind::FString(value), location))
        } else if is_bytes {
            Ok(Token::new(TokenKind::Bytes(value.into_bytes()), location))
        } else {
            Ok(Token::new(TokenKind::String(value), location))
        }
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self) -> ParseResult<Token> {
        let location = self.location();
        let start = self.offset;

        while let Some(c) = self.peek() {
            if is_identifier_continue(c) {
                self.advance();
            } else {
                break;
            }
        }

        let text = &self.source[start..self.offset];

        // Check for keyword
        if let Some(keyword) = TokenKind::keyword_from_str(text) {
            return Ok(Token::new(keyword, location));
        }

        Ok(Token::new(TokenKind::Identifier(text.to_string()), location))
    }

    /// Scan an operator or delimiter
    fn scan_operator(&mut self) -> ParseResult<Token> {
        let location = self.location();
        let c = self.advance().unwrap();

        let kind = match c {
            '(' => {
                self.paren_depth += 1;
                TokenKind::LeftParen
            }
            ')' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                TokenKind::RightParen
            }
            '[' => {
                self.paren_depth += 1;
                TokenKind::LeftBracket
            }
            ']' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                TokenKind::RightBracket
            }
            '{' => {
                self.paren_depth += 1;
                TokenKind::LeftBrace
            }
            '}' => {
                self.paren_depth = self.paren_depth.saturating_sub(1);
                TokenKind::RightBrace
            }
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '~' => TokenKind::Tilde,
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                }
            }
            '-' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::MinusEqual
                } else if self.peek() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '*' => {
                if self.peek() == Some('*') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::DoubleStarEqual
                    } else {
                        TokenKind::DoubleStar
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                }
            }
            '/' => {
                if self.peek() == Some('/') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::DoubleSlashEqual
                    } else {
                        TokenKind::DoubleSlash
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                }
            }
            '@' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::AtEqual
                } else {
                    TokenKind::At
                }
            }
            '&' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::AmpersandEqual
                } else {
                    TokenKind::Ampersand
                }
            }
            '|' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::PipeEqual
                } else {
                    TokenKind::Pipe
                }
            }
            '^' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::CaretEqual
                } else {
                    TokenKind::Caret
                }
            }
            '<' => {
                if self.peek() == Some('<') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LeftShiftEqual
                    } else {
                        TokenKind::LeftShift
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if self.peek() == Some('>') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::RightShiftEqual
                    } else {
                        TokenKind::RightShift
                    }
                } else if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::Equal
                } else {
                    TokenKind::Assign
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    return Err(ParseError::invalid_syntax(location, "unexpected character '!'"));
                }
            }
            ':' => {
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::ColonEqual
                } else {
                    TokenKind::Colon
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        TokenKind::Ellipsis
                    } else {
                        return Err(ParseError::invalid_syntax(location, "unexpected '..'"));
                    }
                } else {
                    TokenKind::Dot
                }
            }
            _ => {
                return Err(ParseError::invalid_syntax(
                    location,
                    &format!("unexpected character '{}'", c),
                ));
            }
        };

        Ok(Token::new(kind, location))
    }

    /// Peek at the next token without consuming it
    pub fn peek_token(&mut self) -> ParseResult<Token> {
        let token = self.next_token()?;
        self.pending_tokens.push(token.clone());
        Ok(token)
    }
}

/// Check if a character can start an identifier
fn is_identifier_start(c: char) -> bool {
    c == '_' || unicode_xid::UnicodeXID::is_xid_start(c)
}

/// Check if a character can continue an identifier
fn is_identifier_continue(c: char) -> bool {
    c == '_' || unicode_xid::UnicodeXID::is_xid_continue(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokens() {
        let mut lexer = Lexer::new("x = 1 + 2");

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::Identifier(ref s) if s == "x"));

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::Assign));

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::Integer(1)));

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::Plus));

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::Integer(2)));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("def if else while for");

        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Def));
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::If));
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Else));
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::While));
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::For));
    }

    #[test]
    fn test_string_literals() {
        let mut lexer = Lexer::new(r#""hello" 'world'"#);

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::String(ref s) if s == "hello"));

        let tok = lexer.next_token().unwrap();
        assert!(matches!(tok.kind, TokenKind::String(ref s) if s == "world"));
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14 0xff 0o77 0b1010");

        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Integer(42)));
        #[allow(clippy::approx_constant)]
        {
            assert!(
                matches!(lexer.next_token().unwrap().kind, TokenKind::Float(f) if (f - 3.14).abs() < 0.001)
            );
        }
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Integer(255)));
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Integer(63)));
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Integer(10)));
    }

    #[test]
    fn test_indentation() {
        let source = "if x:\n    y = 1\n    z = 2\n";
        let mut lexer = Lexer::new(source);

        // if
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::If));
        // x
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Identifier(_)));
        // :
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Colon));
        // NEWLINE
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Newline));
        // INDENT
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Indent));
        // y
        assert!(matches!(lexer.next_token().unwrap().kind, TokenKind::Identifier(_)));
    }
}
