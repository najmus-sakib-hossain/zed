//! Marker expression parser
//!
//! Parses PEP 508 marker expressions into an AST.

use super::MarkerError;

/// Marker comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerOp {
    /// ==
    Equal,
    /// !=
    NotEqual,
    /// <
    LessThan,
    /// <=
    LessEqual,
    /// >
    GreaterThan,
    /// >=
    GreaterEqual,
    /// ~= (compatible release)
    Compatible,
    /// in
    In,
    /// not in
    NotIn,
}

impl MarkerOp {
    /// Parse operator from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "==" => Some(MarkerOp::Equal),
            "!=" => Some(MarkerOp::NotEqual),
            "<" => Some(MarkerOp::LessThan),
            "<=" => Some(MarkerOp::LessEqual),
            ">" => Some(MarkerOp::GreaterThan),
            ">=" => Some(MarkerOp::GreaterEqual),
            "~=" => Some(MarkerOp::Compatible),
            "in" => Some(MarkerOp::In),
            "not in" => Some(MarkerOp::NotIn),
            _ => None,
        }
    }
}

/// Marker value (variable or literal)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerValue {
    /// Environment variable
    Variable(String),
    /// String literal
    Literal(String),
}

/// Marker expression AST
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerExpr {
    /// Comparison expression
    Compare {
        left: MarkerValue,
        op: MarkerOp,
        right: MarkerValue,
    },
    /// AND expression
    And(Box<MarkerExpr>, Box<MarkerExpr>),
    /// OR expression
    Or(Box<MarkerExpr>, Box<MarkerExpr>),
}

/// Token types for lexer
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Variable(String),
    Literal(String),
    Op(MarkerOp),
    And,
    Or,
    LParen,
    RParen,
    Eof,
}

/// Marker expression parser
pub struct MarkerParser {
    input: String,
    pos: usize,
    tokens: Vec<Token>,
    token_pos: usize,
}

impl MarkerParser {
    /// Create a new parser
    pub fn new(input: &str) -> Self {
        Self {
            input: input.to_string(),
            pos: 0,
            tokens: Vec::new(),
            token_pos: 0,
        }
    }

    /// Parse the marker expression
    pub fn parse(&mut self) -> Result<MarkerExpr, MarkerError> {
        self.tokenize()?;
        self.parse_or()
    }

    /// Tokenize the input
    fn tokenize(&mut self) -> Result<(), MarkerError> {
        let chars: Vec<char> = self.input.chars().collect();

        while self.pos < chars.len() {
            self.skip_whitespace(&chars);
            if self.pos >= chars.len() {
                break;
            }

            let c = chars[self.pos];

            if c == '(' {
                self.tokens.push(Token::LParen);
                self.pos += 1;
            } else if c == ')' {
                self.tokens.push(Token::RParen);
                self.pos += 1;
            } else if c == '\'' || c == '"' {
                let literal = self.read_string(&chars, c)?;
                self.tokens.push(Token::Literal(literal));
            } else if c.is_alphabetic() || c == '_' {
                let ident = self.read_identifier(&chars);

                // Check for keywords and operators
                if ident == "and" {
                    self.tokens.push(Token::And);
                } else if ident == "or" {
                    self.tokens.push(Token::Or);
                } else if ident == "in" {
                    self.tokens.push(Token::Op(MarkerOp::In));
                } else if ident == "not" {
                    // Check for "not in"
                    self.skip_whitespace(&chars);
                    if self.pos < chars.len() {
                        let next = self.read_identifier(&chars);
                        if next == "in" {
                            self.tokens.push(Token::Op(MarkerOp::NotIn));
                        } else {
                            return Err(MarkerError::ParseError {
                                position: self.pos,
                                message: "Expected 'in' after 'not'".to_string(),
                            });
                        }
                    }
                } else {
                    self.tokens.push(Token::Variable(ident));
                }
            } else if c == '=' || c == '!' || c == '<' || c == '>' || c == '~' {
                let op = self.read_operator(&chars)?;
                self.tokens.push(Token::Op(op));
            } else {
                return Err(MarkerError::ParseError {
                    position: self.pos,
                    message: format!("Unexpected character: {}", c),
                });
            }
        }

        self.tokens.push(Token::Eof);
        Ok(())
    }

    fn skip_whitespace(&mut self, chars: &[char]) {
        while self.pos < chars.len() && chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn read_string(&mut self, chars: &[char], quote: char) -> Result<String, MarkerError> {
        self.pos += 1; // Skip opening quote
        let start = self.pos;

        while self.pos < chars.len() && chars[self.pos] != quote {
            self.pos += 1;
        }

        if self.pos >= chars.len() {
            return Err(MarkerError::ParseError {
                position: start,
                message: "Unterminated string".to_string(),
            });
        }

        let s: String = chars[start..self.pos].iter().collect();
        self.pos += 1; // Skip closing quote
        Ok(s)
    }

    fn read_identifier(&mut self, chars: &[char]) -> String {
        let start = self.pos;
        while self.pos < chars.len()
            && (chars[self.pos].is_alphanumeric()
                || chars[self.pos] == '_'
                || chars[self.pos] == '.')
        {
            self.pos += 1;
        }
        chars[start..self.pos].iter().collect()
    }

    fn read_operator(&mut self, chars: &[char]) -> Result<MarkerOp, MarkerError> {
        let start = self.pos;
        let c = chars[self.pos];
        self.pos += 1;

        let op_str = if self.pos < chars.len() && chars[self.pos] == '=' {
            self.pos += 1;
            format!("{}=", c)
        } else {
            c.to_string()
        };

        MarkerOp::parse(&op_str).ok_or_else(|| MarkerError::ParseError {
            position: start,
            message: format!("Unknown operator: {}", op_str),
        })
    }

    fn current_token(&self) -> &Token {
        self.tokens.get(self.token_pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) {
        if self.token_pos < self.tokens.len() {
            self.token_pos += 1;
        }
    }

    fn parse_or(&mut self) -> Result<MarkerExpr, MarkerError> {
        let mut left = self.parse_and()?;

        while matches!(self.current_token(), Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = MarkerExpr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<MarkerExpr, MarkerError> {
        let mut left = self.parse_comparison()?;

        while matches!(self.current_token(), Token::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = MarkerExpr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<MarkerExpr, MarkerError> {
        if matches!(self.current_token(), Token::LParen) {
            self.advance();
            let expr = self.parse_or()?;
            if !matches!(self.current_token(), Token::RParen) {
                return Err(MarkerError::ParseError {
                    position: self.token_pos,
                    message: "Expected ')'".to_string(),
                });
            }
            self.advance();
            return Ok(expr);
        }

        let left = match self.current_token().clone() {
            Token::Variable(v) => {
                self.advance();
                MarkerValue::Variable(v)
            }
            Token::Literal(l) => {
                self.advance();
                MarkerValue::Literal(l)
            }
            _ => {
                return Err(MarkerError::ParseError {
                    position: self.token_pos,
                    message: "Expected variable or literal".to_string(),
                });
            }
        };

        let op = match self.current_token() {
            Token::Op(op) => {
                let op = *op;
                self.advance();
                op
            }
            _ => {
                return Err(MarkerError::ParseError {
                    position: self.token_pos,
                    message: "Expected operator".to_string(),
                });
            }
        };

        let right = match self.current_token().clone() {
            Token::Variable(v) => {
                self.advance();
                MarkerValue::Variable(v)
            }
            Token::Literal(l) => {
                self.advance();
                MarkerValue::Literal(l)
            }
            _ => {
                return Err(MarkerError::ParseError {
                    position: self.token_pos,
                    message: "Expected variable or literal".to_string(),
                });
            }
        };

        Ok(MarkerExpr::Compare { left, op, right })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_comparison() {
        let mut parser = MarkerParser::new("python_version >= '3.8'");
        let expr = parser.parse().unwrap();

        match expr {
            MarkerExpr::Compare { left, op, right } => {
                assert_eq!(left, MarkerValue::Variable("python_version".to_string()));
                assert_eq!(op, MarkerOp::GreaterEqual);
                assert_eq!(right, MarkerValue::Literal("3.8".to_string()));
            }
            _ => panic!("Expected Compare expression"),
        }
    }

    #[test]
    fn test_parse_and_expression() {
        let mut parser = MarkerParser::new("python_version >= '3.8' and sys_platform == 'linux'");
        let expr = parser.parse().unwrap();

        assert!(matches!(expr, MarkerExpr::And(_, _)));
    }

    #[test]
    fn test_parse_or_expression() {
        let mut parser = MarkerParser::new("sys_platform == 'linux' or sys_platform == 'darwin'");
        let expr = parser.parse().unwrap();

        assert!(matches!(expr, MarkerExpr::Or(_, _)));
    }

    #[test]
    fn test_parse_in_operator() {
        let mut parser = MarkerParser::new("'linux' in sys_platform");
        let expr = parser.parse().unwrap();

        match expr {
            MarkerExpr::Compare { op, .. } => {
                assert_eq!(op, MarkerOp::In);
            }
            _ => panic!("Expected Compare expression"),
        }
    }
}
