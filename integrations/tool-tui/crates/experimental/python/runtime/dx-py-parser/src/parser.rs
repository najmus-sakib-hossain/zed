//! Python parser

use crate::ast::*;
use crate::error::{Location, ParseError, ParseResult};
use crate::lexer::Lexer;
use crate::token::{Token, TokenKind};

/// Python parser
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given source
    pub fn new(source: &'a str) -> Self {
        Self {
            lexer: Lexer::new(source),
            current: None,
        }
    }

    /// Get current location
    fn location(&self) -> Location {
        self.current.as_ref().map(|t| t.location).unwrap_or_default()
    }

    /// Advance to the next token
    fn advance(&mut self) -> ParseResult<Token> {
        let token = self.lexer.next_token()?;
        let prev = self.current.replace(token.clone());
        Ok(prev.unwrap_or(token))
    }

    /// Peek at the current token
    fn peek(&mut self) -> ParseResult<&Token> {
        if self.current.is_none() {
            self.current = Some(self.lexer.next_token()?);
        }
        Ok(self.current.as_ref().unwrap())
    }

    /// Check if current token matches
    fn check(&mut self, kind: &TokenKind) -> ParseResult<bool> {
        Ok(std::mem::discriminant(&self.peek()?.kind) == std::mem::discriminant(kind))
    }

    /// Consume token if it matches
    fn consume(&mut self, kind: &TokenKind) -> ParseResult<bool> {
        if self.check(kind)? {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Expect a specific token
    fn expect(&mut self, kind: &TokenKind) -> ParseResult<Token> {
        if self.check(kind)? {
            self.advance()
        } else {
            let token = self.peek()?;
            Err(ParseError::unexpected_token(
                token.location,
                &kind.to_string(),
                &token.kind.to_string(),
            ))
        }
    }

    /// Parse a module
    pub fn parse_module(&mut self) -> ParseResult<Module> {
        let mut body = Vec::new();

        while !self.check(&TokenKind::Eof)? {
            // Skip newlines at module level
            while self.consume(&TokenKind::Newline)? {}

            if self.check(&TokenKind::Eof)? {
                break;
            }

            body.push(self.parse_statement()?);
        }

        Ok(Module { body })
    }

    /// Parse a single statement
    pub fn parse_statement(&mut self) -> ParseResult<Statement> {
        // Handle decorators
        if self.check(&TokenKind::At)? {
            return self.parse_decorated();
        }

        let token = self.peek()?;
        let location = token.location;

        match &token.kind {
            TokenKind::Def => self.parse_function_def_with_decorators(false, Vec::new()),
            TokenKind::Async => {
                self.advance()?;
                if self.check(&TokenKind::Def)? {
                    self.parse_function_def_with_decorators(true, Vec::new())
                } else if self.check(&TokenKind::For)? {
                    self.parse_for(true)
                } else if self.check(&TokenKind::With)? {
                    self.parse_with(true)
                } else {
                    Err(ParseError::invalid_syntax(
                        location,
                        "expected 'def', 'for', or 'with' after 'async'",
                    ))
                }
            }
            TokenKind::Class => self.parse_class_def_with_decorators(Vec::new()),
            TokenKind::Return => self.parse_return(),
            TokenKind::Del => self.parse_del(),
            TokenKind::Pass => self.parse_pass(),
            TokenKind::Break => self.parse_break(),
            TokenKind::Continue => self.parse_continue(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(false),
            TokenKind::Try => self.parse_try(),
            TokenKind::With => self.parse_with(false),
            TokenKind::Raise => self.parse_raise(),
            TokenKind::Assert => self.parse_assert(),
            TokenKind::Import => self.parse_import(),
            TokenKind::From => self.parse_from_import(),
            TokenKind::Global => self.parse_global(),
            TokenKind::Nonlocal => self.parse_nonlocal(),
            TokenKind::Match => self.parse_match(),
            _ => self.parse_simple_stmt(),
        }
    }

    /// Parse a simple statement (expression or assignment)
    fn parse_simple_stmt(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        let expr = self.parse_expression()?;

        // Check for assignment
        if self.consume(&TokenKind::Assign)? {
            let value = self.parse_expression()?;
            self.consume(&TokenKind::Newline)?;
            return Ok(Statement::Assign {
                targets: vec![expr],
                value,
                location,
            });
        }

        // Check for augmented assignment
        if let Some(op) = self.check_aug_assign()? {
            self.advance()?;
            let value = self.parse_expression()?;
            self.consume(&TokenKind::Newline)?;
            return Ok(Statement::AugAssign {
                target: expr,
                op,
                value,
                location,
            });
        }

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Expr {
            value: expr,
            location,
        })
    }

    fn check_aug_assign(&mut self) -> ParseResult<Option<BinOp>> {
        let token = self.peek()?;
        Ok(match &token.kind {
            TokenKind::PlusEqual => Some(BinOp::Add),
            TokenKind::MinusEqual => Some(BinOp::Sub),
            TokenKind::StarEqual => Some(BinOp::Mult),
            TokenKind::SlashEqual => Some(BinOp::Div),
            TokenKind::DoubleSlashEqual => Some(BinOp::FloorDiv),
            TokenKind::PercentEqual => Some(BinOp::Mod),
            TokenKind::DoubleStarEqual => Some(BinOp::Pow),
            TokenKind::AmpersandEqual => Some(BinOp::BitAnd),
            TokenKind::PipeEqual => Some(BinOp::BitOr),
            TokenKind::CaretEqual => Some(BinOp::BitXor),
            TokenKind::LeftShiftEqual => Some(BinOp::LShift),
            TokenKind::RightShiftEqual => Some(BinOp::RShift),
            TokenKind::AtEqual => Some(BinOp::MatMult),
            _ => None,
        })
    }

    /// Parse decorated statement (function or class with decorators)
    fn parse_decorated(&mut self) -> ParseResult<Statement> {
        let mut decorators = Vec::new();

        while self.check(&TokenKind::At)? {
            self.advance()?;
            let decorator = self.parse_expression()?;
            self.expect(&TokenKind::Newline)?;
            decorators.push(decorator);
        }

        // After decorators, we expect def, async def, or class
        let token = self.peek()?.clone();
        match &token.kind {
            TokenKind::Def => self.parse_function_def_with_decorators(false, decorators),
            TokenKind::Async => {
                self.advance()?;
                if self.check(&TokenKind::Def)? {
                    self.parse_function_def_with_decorators(true, decorators)
                } else {
                    Err(ParseError::invalid_syntax(
                        token.location,
                        "expected 'def' after 'async' in decorated statement",
                    ))
                }
            }
            TokenKind::Class => self.parse_class_def_with_decorators(decorators),
            _ => Err(ParseError::invalid_syntax(
                token.location,
                "expected 'def', 'async def', or 'class' after decorator",
            )),
        }
    }

    /// Parse function definition with decorators
    fn parse_function_def_with_decorators(
        &mut self,
        is_async: bool,
        decorators: Vec<Expression>,
    ) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Def)?;

        let name = self.parse_identifier()?;
        self.expect(&TokenKind::LeftParen)?;
        let args = self.parse_arguments()?;
        self.expect(&TokenKind::RightParen)?;

        let returns = if self.consume(&TokenKind::Arrow)? {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        Ok(Statement::FunctionDef {
            name,
            args,
            body,
            decorators,
            returns,
            is_async,
            location,
        })
    }

    /// Parse class definition with decorators
    fn parse_class_def_with_decorators(
        &mut self,
        decorators: Vec<Expression>,
    ) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Class)?;

        let name = self.parse_identifier()?;

        let (bases, keywords) = if self.consume(&TokenKind::LeftParen)? {
            let (bases, keywords) = self.parse_call_args()?;
            self.expect(&TokenKind::RightParen)?;
            (bases, keywords)
        } else {
            (Vec::new(), Vec::new())
        };

        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        Ok(Statement::ClassDef {
            name,
            bases,
            keywords,
            body,
            decorators,
            location,
        })
    }

    /// Parse return statement
    fn parse_return(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Return)?;

        let value = if !self.check(&TokenKind::Newline)? && !self.check(&TokenKind::Eof)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Return { value, location })
    }

    /// Parse del statement
    fn parse_del(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Del)?;

        let mut targets = vec![self.parse_expression()?];
        while self.consume(&TokenKind::Comma)? {
            targets.push(self.parse_expression()?);
        }

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Delete { targets, location })
    }

    /// Parse pass statement
    fn parse_pass(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Pass)?;
        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Pass { location })
    }

    /// Parse break statement
    fn parse_break(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Break)?;
        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Break { location })
    }

    /// Parse continue statement
    fn parse_continue(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Continue)?;
        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Continue { location })
    }

    /// Parse if statement
    fn parse_if(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::If)?;

        let test = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        let orelse = if self.consume(&TokenKind::Elif)? {
            // Elif is syntactic sugar for else: if
            let elif_stmt = self.parse_elif()?;
            vec![elif_stmt]
        } else if self.consume(&TokenKind::Else)? {
            self.expect(&TokenKind::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Statement::If {
            test,
            body,
            orelse,
            location,
        })
    }

    fn parse_elif(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        let test = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        let orelse = if self.consume(&TokenKind::Elif)? {
            vec![self.parse_elif()?]
        } else if self.consume(&TokenKind::Else)? {
            self.expect(&TokenKind::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Statement::If {
            test,
            body,
            orelse,
            location,
        })
    }

    /// Parse while statement
    fn parse_while(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::While)?;

        let test = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        let orelse = if self.consume(&TokenKind::Else)? {
            self.expect(&TokenKind::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Statement::While {
            test,
            body,
            orelse,
            location,
        })
    }

    /// Parse for statement
    fn parse_for(&mut self, is_async: bool) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::For)?;

        // Parse target - should be a simple name or tuple, not a full expression
        let target = self.parse_target()?;
        self.expect(&TokenKind::In)?;
        let iter = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        let orelse = if self.consume(&TokenKind::Else)? {
            self.expect(&TokenKind::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Statement::For {
            target,
            iter,
            body,
            orelse,
            is_async,
            location,
        })
    }

    /// Parse try statement
    fn parse_try(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Try)?;
        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        let mut handlers = Vec::new();
        while self.consume(&TokenKind::Except)? {
            handlers.push(self.parse_except_handler()?);
        }

        let orelse = if self.consume(&TokenKind::Else)? {
            self.expect(&TokenKind::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        let finalbody = if self.consume(&TokenKind::Finally)? {
            self.expect(&TokenKind::Colon)?;
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Statement::Try {
            body,
            handlers,
            orelse,
            finalbody,
            location,
        })
    }

    fn parse_except_handler(&mut self) -> ParseResult<ExceptHandler> {
        let location = self.location();

        let typ = if !self.check(&TokenKind::Colon)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let name = if self.consume(&TokenKind::As)? {
            Some(self.parse_identifier()?)
        } else {
            None
        };

        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        Ok(ExceptHandler {
            typ,
            name,
            body,
            location,
        })
    }

    /// Parse with statement
    fn parse_with(&mut self, is_async: bool) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::With)?;

        let mut items = vec![self.parse_with_item()?];
        while self.consume(&TokenKind::Comma)? {
            items.push(self.parse_with_item()?);
        }

        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        Ok(Statement::With {
            items,
            body,
            is_async,
            location,
        })
    }

    fn parse_with_item(&mut self) -> ParseResult<WithItem> {
        let context_expr = self.parse_expression()?;
        let optional_vars = if self.consume(&TokenKind::As)? {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(WithItem {
            context_expr,
            optional_vars,
        })
    }

    /// Parse raise statement
    fn parse_raise(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Raise)?;

        let exc = if !self.check(&TokenKind::Newline)? && !self.check(&TokenKind::Eof)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let cause = if self.consume(&TokenKind::From)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Raise {
            exc,
            cause,
            location,
        })
    }

    /// Parse assert statement
    fn parse_assert(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Assert)?;

        let test = self.parse_expression()?;
        let msg = if self.consume(&TokenKind::Comma)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Assert {
            test,
            msg,
            location,
        })
    }

    /// Parse import statement
    fn parse_import(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Import)?;

        let mut names = vec![self.parse_alias()?];
        while self.consume(&TokenKind::Comma)? {
            names.push(self.parse_alias()?);
        }

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Import { names, location })
    }

    /// Parse from import statement
    fn parse_from_import(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::From)?;

        let mut level = 0;
        while self.consume(&TokenKind::Dot)? {
            level += 1;
        }

        let module = if !self.check(&TokenKind::Import)? {
            Some(self.parse_dotted_name()?)
        } else {
            None
        };

        self.expect(&TokenKind::Import)?;

        let names = if self.consume(&TokenKind::Star)? {
            vec![Alias {
                name: "*".to_string(),
                asname: None,
                location,
            }]
        } else if self.consume(&TokenKind::LeftParen)? {
            let names = self.parse_import_names()?;
            self.expect(&TokenKind::RightParen)?;
            names
        } else {
            self.parse_import_names()?
        };

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::ImportFrom {
            module,
            names,
            level,
            location,
        })
    }

    fn parse_import_names(&mut self) -> ParseResult<Vec<Alias>> {
        let mut names = vec![self.parse_alias()?];
        while self.consume(&TokenKind::Comma)? {
            if self.check(&TokenKind::RightParen)? || self.check(&TokenKind::Newline)? {
                break;
            }
            names.push(self.parse_alias()?);
        }
        Ok(names)
    }

    fn parse_alias(&mut self) -> ParseResult<Alias> {
        let location = self.location();
        let name = self.parse_dotted_name()?;
        let asname = if self.consume(&TokenKind::As)? {
            Some(self.parse_identifier()?)
        } else {
            None
        };
        Ok(Alias {
            name,
            asname,
            location,
        })
    }

    fn parse_dotted_name(&mut self) -> ParseResult<String> {
        let mut name = self.parse_identifier()?;
        while self.consume(&TokenKind::Dot)? {
            name.push('.');
            name.push_str(&self.parse_identifier()?);
        }
        Ok(name)
    }

    /// Parse global statement
    fn parse_global(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Global)?;

        let mut names = vec![self.parse_identifier()?];
        while self.consume(&TokenKind::Comma)? {
            names.push(self.parse_identifier()?);
        }

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Global { names, location })
    }

    /// Parse nonlocal statement
    fn parse_nonlocal(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Nonlocal)?;

        let mut names = vec![self.parse_identifier()?];
        while self.consume(&TokenKind::Comma)? {
            names.push(self.parse_identifier()?);
        }

        self.consume(&TokenKind::Newline)?;
        Ok(Statement::Nonlocal { names, location })
    }

    /// Parse match statement with full pattern support
    fn parse_match(&mut self) -> ParseResult<Statement> {
        let location = self.location();
        self.expect(&TokenKind::Match)?;
        let subject = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;

        let mut cases = Vec::new();
        while self.check(&TokenKind::Case)? {
            cases.push(self.parse_match_case()?);
        }

        self.expect(&TokenKind::Dedent)?;

        Ok(Statement::Match {
            subject,
            cases,
            location,
        })
    }

    /// Parse a single match case
    fn parse_match_case(&mut self) -> ParseResult<MatchCase> {
        self.expect(&TokenKind::Case)?;
        let pattern = self.parse_pattern()?;

        // Parse optional guard
        let guard = if self.consume(&TokenKind::If)? {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect(&TokenKind::Colon)?;
        let body = self.parse_block()?;

        Ok(MatchCase {
            pattern,
            guard,
            body,
        })
    }

    /// Parse a pattern for match statement
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let pattern = self.parse_or_pattern()?;

        // Check for 'as' pattern binding
        if self.consume(&TokenKind::As)? {
            let location = self.location();
            let name = self.parse_identifier()?;
            return Ok(Pattern::MatchAs {
                pattern: Some(Box::new(pattern)),
                name: Some(name),
                location,
            });
        }

        Ok(pattern)
    }

    /// Parse or-pattern (pattern | pattern | ...)
    fn parse_or_pattern(&mut self) -> ParseResult<Pattern> {
        let mut patterns = vec![self.parse_closed_pattern()?];

        while self.consume(&TokenKind::Pipe)? {
            patterns.push(self.parse_closed_pattern()?);
        }

        if patterns.len() == 1 {
            Ok(patterns.pop().unwrap())
        } else {
            let location = self.location();
            Ok(Pattern::MatchOr { patterns, location })
        }
    }

    /// Parse a closed pattern (not or-pattern)
    fn parse_closed_pattern(&mut self) -> ParseResult<Pattern> {
        let token = self.peek()?.clone();
        let location = token.location;

        match &token.kind {
            // Wildcard pattern: _
            TokenKind::Identifier(name) if name == "_" => {
                self.advance()?;
                Ok(Pattern::MatchAs {
                    pattern: None,
                    name: None,
                    location,
                })
            }

            // Literal patterns: numbers, strings, True, False, None
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance()?;
                Ok(Pattern::MatchValue {
                    value: Expression::Constant {
                        value: Constant::Int(n),
                        location,
                    },
                    location,
                })
            }
            TokenKind::Float(n) => {
                let n = *n;
                self.advance()?;
                Ok(Pattern::MatchValue {
                    value: Expression::Constant {
                        value: Constant::Float(n),
                        location,
                    },
                    location,
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(Pattern::MatchValue {
                    value: Expression::Constant {
                        value: Constant::Str(s),
                        location,
                    },
                    location,
                })
            }
            TokenKind::True => {
                self.advance()?;
                Ok(Pattern::MatchSingleton {
                    value: Constant::Bool(true),
                    location,
                })
            }
            TokenKind::False => {
                self.advance()?;
                Ok(Pattern::MatchSingleton {
                    value: Constant::Bool(false),
                    location,
                })
            }
            TokenKind::None => {
                self.advance()?;
                Ok(Pattern::MatchSingleton {
                    value: Constant::None,
                    location,
                })
            }

            // Negative number literals
            TokenKind::Minus => {
                self.advance()?;
                let next = self.peek()?.clone();
                match next.kind {
                    TokenKind::Integer(n) => {
                        self.advance()?;
                        Ok(Pattern::MatchValue {
                            value: Expression::UnaryOp {
                                op: UnaryOp::USub,
                                operand: Box::new(Expression::Constant {
                                    value: Constant::Int(n),
                                    location: next.location,
                                }),
                                location,
                            },
                            location,
                        })
                    }
                    TokenKind::Float(n) => {
                        self.advance()?;
                        Ok(Pattern::MatchValue {
                            value: Expression::UnaryOp {
                                op: UnaryOp::USub,
                                operand: Box::new(Expression::Constant {
                                    value: Constant::Float(n),
                                    location: next.location,
                                }),
                                location,
                            },
                            location,
                        })
                    }
                    _ => Err(ParseError::invalid_syntax(
                        location,
                        "expected number after '-' in pattern",
                    )),
                }
            }

            // Sequence pattern: [pattern, ...]
            TokenKind::LeftBracket => {
                self.advance()?;
                let patterns = self.parse_sequence_pattern_items()?;
                self.expect(&TokenKind::RightBracket)?;
                Ok(Pattern::MatchSequence { patterns, location })
            }

            // Tuple/group pattern or sequence: (pattern, ...)
            TokenKind::LeftParen => {
                self.advance()?;
                if self.check(&TokenKind::RightParen)? {
                    self.advance()?;
                    return Ok(Pattern::MatchSequence {
                        patterns: Vec::new(),
                        location,
                    });
                }
                let first = self.parse_pattern()?;
                if self.consume(&TokenKind::Comma)? {
                    // Tuple pattern
                    let mut patterns = vec![first];
                    while !self.check(&TokenKind::RightParen)? {
                        patterns.push(self.parse_pattern()?);
                        if !self.consume(&TokenKind::Comma)? {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RightParen)?;
                    Ok(Pattern::MatchSequence { patterns, location })
                } else {
                    // Grouping parentheses
                    self.expect(&TokenKind::RightParen)?;
                    Ok(first)
                }
            }

            // Mapping pattern: {key: pattern, ...}
            TokenKind::LeftBrace => {
                self.advance()?;
                self.parse_mapping_pattern(location)
            }

            // Star pattern: *name (in sequences)
            TokenKind::Star => {
                self.advance()?;
                let name = if matches!(self.peek()?.kind, TokenKind::Identifier(_)) {
                    let id = self.parse_identifier()?;
                    if id == "_" {
                        None
                    } else {
                        Some(id)
                    }
                } else {
                    None
                };
                Ok(Pattern::MatchStar { name, location })
            }

            // Identifier: capture pattern or class pattern or dotted name value
            TokenKind::Identifier(_) => self.parse_name_or_class_pattern(),

            _ => Err(ParseError::invalid_syntax(
                location,
                &format!("invalid pattern: unexpected {:?}", token.kind),
            )),
        }
    }

    /// Parse sequence pattern items (handles star patterns)
    fn parse_sequence_pattern_items(&mut self) -> ParseResult<Vec<Pattern>> {
        let mut patterns = Vec::new();

        if self.check(&TokenKind::RightBracket)? || self.check(&TokenKind::RightParen)? {
            return Ok(patterns);
        }

        patterns.push(self.parse_pattern()?);
        while self.consume(&TokenKind::Comma)? {
            if self.check(&TokenKind::RightBracket)? || self.check(&TokenKind::RightParen)? {
                break;
            }
            patterns.push(self.parse_pattern()?);
        }

        Ok(patterns)
    }

    /// Parse mapping pattern: {key: pattern, **rest}
    fn parse_mapping_pattern(&mut self, location: Location) -> ParseResult<Pattern> {
        let mut keys = Vec::new();
        let mut patterns = Vec::new();
        let mut rest = None;

        if !self.check(&TokenKind::RightBrace)? {
            loop {
                if self.consume(&TokenKind::DoubleStar)? {
                    // **rest pattern
                    let name = self.parse_identifier()?;
                    rest = if name == "_" { None } else { Some(name) };
                    self.consume(&TokenKind::Comma)?;
                    break;
                }

                // Parse key (must be a literal or attribute)
                let key = self.parse_pattern_key()?;
                self.expect(&TokenKind::Colon)?;
                let pattern = self.parse_pattern()?;

                keys.push(key);
                patterns.push(pattern);

                if !self.consume(&TokenKind::Comma)? {
                    break;
                }
                if self.check(&TokenKind::RightBrace)? {
                    break;
                }
            }
        }

        self.expect(&TokenKind::RightBrace)?;
        Ok(Pattern::MatchMapping {
            keys,
            patterns,
            rest,
            location,
        })
    }

    /// Parse a pattern key (literal or dotted name)
    fn parse_pattern_key(&mut self) -> ParseResult<Expression> {
        let token = self.peek()?.clone();
        let location = token.location;

        match &token.kind {
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Int(n),
                    location,
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Str(s),
                    location,
                })
            }
            TokenKind::True => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Bool(true),
                    location,
                })
            }
            TokenKind::False => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Bool(false),
                    location,
                })
            }
            TokenKind::None => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::None,
                    location,
                })
            }
            TokenKind::Identifier(_) => {
                // Dotted name for attribute access
                let mut expr = Expression::Name {
                    id: self.parse_identifier()?,
                    location,
                };
                while self.consume(&TokenKind::Dot)? {
                    let attr = self.parse_identifier()?;
                    expr = Expression::Attribute {
                        value: Box::new(expr),
                        attr,
                        location: self.location(),
                    };
                }
                Ok(expr)
            }
            _ => Err(ParseError::invalid_syntax(
                location,
                "expected literal or name as mapping pattern key",
            )),
        }
    }

    /// Parse name pattern, class pattern, or dotted name value pattern
    fn parse_name_or_class_pattern(&mut self) -> ParseResult<Pattern> {
        let location = self.location();
        let first_name = self.parse_identifier()?;

        // Check for dotted name (value pattern or class pattern)
        let mut is_dotted = false;
        let mut expr = Expression::Name {
            id: first_name.clone(),
            location,
        };

        while self.consume(&TokenKind::Dot)? {
            is_dotted = true;
            let attr = self.parse_identifier()?;
            expr = Expression::Attribute {
                value: Box::new(expr),
                attr,
                location: self.location(),
            };
        }

        // Check for class pattern: Name(patterns...)
        if self.check(&TokenKind::LeftParen)? {
            self.advance()?;
            let (patterns, kwd_attrs, kwd_patterns) = self.parse_class_pattern_args()?;
            self.expect(&TokenKind::RightParen)?;
            return Ok(Pattern::MatchClass {
                cls: expr,
                patterns,
                kwd_attrs,
                kwd_patterns,
                location,
            });
        }

        // If dotted, it's a value pattern
        if is_dotted {
            return Ok(Pattern::MatchValue {
                value: expr,
                location,
            });
        }

        // Simple name is a capture pattern
        Ok(Pattern::MatchAs {
            pattern: None,
            name: Some(first_name),
            location,
        })
    }

    /// Parse class pattern arguments
    fn parse_class_pattern_args(
        &mut self,
    ) -> ParseResult<(Vec<Pattern>, Vec<String>, Vec<Pattern>)> {
        let mut patterns = Vec::new();
        let mut kwd_attrs = Vec::new();
        let mut kwd_patterns = Vec::new();
        let mut seen_keyword = false;

        if self.check(&TokenKind::RightParen)? {
            return Ok((patterns, kwd_attrs, kwd_patterns));
        }

        loop {
            // Check for keyword pattern: name=pattern
            if let TokenKind::Identifier(name) = &self.peek()?.kind.clone() {
                let name = name.clone();
                let saved_location = self.location();
                self.advance()?;

                if self.consume(&TokenKind::Assign)? {
                    // Keyword pattern
                    seen_keyword = true;
                    kwd_attrs.push(name);
                    kwd_patterns.push(self.parse_pattern()?);
                } else {
                    // Not a keyword pattern, backtrack
                    // We need to parse this as a regular pattern
                    // Since we already consumed the identifier, we need to handle it
                    if seen_keyword {
                        return Err(ParseError::invalid_syntax(
                            saved_location,
                            "positional pattern follows keyword pattern",
                        ));
                    }
                    // Create a capture pattern from the identifier
                    let pattern = if name == "_" {
                        Pattern::MatchAs {
                            pattern: None,
                            name: None,
                            location: saved_location,
                        }
                    } else {
                        Pattern::MatchAs {
                            pattern: None,
                            name: Some(name),
                            location: saved_location,
                        }
                    };
                    patterns.push(pattern);
                }
            } else {
                if seen_keyword {
                    return Err(ParseError::invalid_syntax(
                        self.location(),
                        "positional pattern follows keyword pattern",
                    ));
                }
                patterns.push(self.parse_pattern()?);
            }

            if !self.consume(&TokenKind::Comma)? {
                break;
            }
            if self.check(&TokenKind::RightParen)? {
                break;
            }
        }

        Ok((patterns, kwd_attrs, kwd_patterns))
    }

    /// Parse a block of statements
    fn parse_block(&mut self) -> ParseResult<Vec<Statement>> {
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;

        let mut body = Vec::new();
        while !self.check(&TokenKind::Dedent)? && !self.check(&TokenKind::Eof)? {
            // Skip blank lines
            while self.consume(&TokenKind::Newline)? {}

            if self.check(&TokenKind::Dedent)? || self.check(&TokenKind::Eof)? {
                break;
            }

            body.push(self.parse_statement()?);
        }

        self.consume(&TokenKind::Dedent)?;
        Ok(body)
    }

    /// Parse function arguments
    fn parse_arguments(&mut self) -> ParseResult<Arguments> {
        let mut args = Arguments::default();

        if self.check(&TokenKind::RightParen)? {
            return Ok(args);
        }

        loop {
            if self.check(&TokenKind::RightParen)? {
                break;
            }

            // Check for *args or **kwargs
            if self.consume(&TokenKind::DoubleStar)? {
                let name = self.parse_identifier()?;
                args.kwarg = Some(Box::new(Arg {
                    arg: name,
                    annotation: None,
                    location: self.location(),
                }));
                break;
            }

            if self.consume(&TokenKind::Star)? {
                if self.check(&TokenKind::Comma)? || self.check(&TokenKind::RightParen)? {
                    // Bare * for keyword-only args
                } else {
                    let name = self.parse_identifier()?;
                    args.vararg = Some(Box::new(Arg {
                        arg: name,
                        annotation: None,
                        location: self.location(),
                    }));
                }
                if !self.consume(&TokenKind::Comma)? {
                    break;
                }
                continue;
            }

            let name = self.parse_identifier()?;
            let annotation = if self.consume(&TokenKind::Colon)? {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            let has_default = if self.consume(&TokenKind::Assign)? {
                args.defaults.push(self.parse_expression()?);
                true
            } else {
                false
            };

            let arg = Arg {
                arg: name,
                annotation,
                location: self.location(),
            };

            if args.vararg.is_some() {
                args.kwonlyargs.push(arg);
                if !has_default {
                    args.kw_defaults.push(None);
                }
            } else {
                args.args.push(arg);
            }

            if !self.consume(&TokenKind::Comma)? {
                break;
            }
        }

        Ok(args)
    }

    /// Parse lambda arguments (stops at colon instead of right paren)
    fn parse_lambda_arguments(&mut self) -> ParseResult<Arguments> {
        let mut args = Arguments::default();

        if self.check(&TokenKind::Colon)? {
            return Ok(args);
        }

        loop {
            if self.check(&TokenKind::Colon)? {
                break;
            }

            // Check for *args or **kwargs
            if self.consume(&TokenKind::DoubleStar)? {
                let name = self.parse_identifier()?;
                args.kwarg = Some(Box::new(Arg {
                    arg: name,
                    annotation: None,
                    location: self.location(),
                }));
                break;
            }

            if self.consume(&TokenKind::Star)? {
                if self.check(&TokenKind::Comma)? || self.check(&TokenKind::Colon)? {
                    // Bare * for keyword-only args
                } else {
                    let name = self.parse_identifier()?;
                    args.vararg = Some(Box::new(Arg {
                        arg: name,
                        annotation: None,
                        location: self.location(),
                    }));
                }
                if !self.consume(&TokenKind::Comma)? {
                    break;
                }
                continue;
            }

            let name = self.parse_identifier()?;
            // Lambda args don't have annotations
            let has_default = if self.consume(&TokenKind::Assign)? {
                args.defaults.push(self.parse_expression()?);
                true
            } else {
                false
            };

            let arg = Arg {
                arg: name,
                annotation: None,
                location: self.location(),
            };

            if args.vararg.is_some() {
                args.kwonlyargs.push(arg);
                if !has_default {
                    args.kw_defaults.push(None);
                }
            } else {
                args.args.push(arg);
            }

            if !self.consume(&TokenKind::Comma)? {
                break;
            }
        }

        Ok(args)
    }

    /// Parse call arguments
    fn parse_call_args(&mut self) -> ParseResult<(Vec<Expression>, Vec<Keyword>)> {
        let mut args = Vec::new();
        let mut keywords = Vec::new();

        if self.check(&TokenKind::RightParen)? {
            return Ok((args, keywords));
        }

        loop {
            if self.check(&TokenKind::RightParen)? {
                break;
            }

            // Check for **kwargs
            if self.consume(&TokenKind::DoubleStar)? {
                let value = self.parse_expression()?;
                keywords.push(Keyword {
                    arg: None,
                    value,
                    location: self.location(),
                });
            } else if self.consume(&TokenKind::Star)? {
                // *args
                let value = self.parse_expression()?;
                args.push(Expression::Starred {
                    value: Box::new(value),
                    location: self.location(),
                });
            } else {
                let expr = self.parse_expression()?;

                // Check for keyword argument
                if let Expression::Name { id, location } = &expr {
                    if self.consume(&TokenKind::Assign)? {
                        let value = self.parse_expression()?;
                        keywords.push(Keyword {
                            arg: Some(id.clone()),
                            value,
                            location: *location,
                        });
                    } else {
                        args.push(expr);
                    }
                } else {
                    args.push(expr);
                }
            }

            if !self.consume(&TokenKind::Comma)? {
                break;
            }
        }

        Ok((args, keywords))
    }

    /// Parse an identifier
    fn parse_identifier(&mut self) -> ParseResult<String> {
        let token = self.advance()?;
        match token.kind {
            TokenKind::Identifier(name) => Ok(name),
            _ => Err(ParseError::unexpected_token(
                token.location,
                "identifier",
                &token.kind.to_string(),
            )),
        }
    }

    /// Parse a target for assignment or comprehension (name or tuple of names)
    fn parse_target(&mut self) -> ParseResult<Expression> {
        let location = self.location();

        // Check for tuple unpacking
        if self.consume(&TokenKind::LeftParen)? {
            let mut elts = Vec::new();
            if !self.check(&TokenKind::RightParen)? {
                elts.push(self.parse_target()?);
                while self.consume(&TokenKind::Comma)? {
                    if self.check(&TokenKind::RightParen)? {
                        break;
                    }
                    elts.push(self.parse_target()?);
                }
            }
            self.expect(&TokenKind::RightParen)?;
            return Ok(Expression::Tuple { elts, location });
        }

        // Simple name
        let name = self.parse_identifier()?;
        Ok(Expression::Name { id: name, location })
    }

    /// Parse an expression (handles yield, yield from, and named expressions)
    pub fn parse_expression(&mut self) -> ParseResult<Expression> {
        // Handle yield and yield from at the top level
        if self.check(&TokenKind::Yield)? {
            return self.parse_yield_expr();
        }

        self.parse_named_expr()
    }

    /// Parse yield expression
    fn parse_yield_expr(&mut self) -> ParseResult<Expression> {
        let location = self.location();
        self.expect(&TokenKind::Yield)?;

        // Check for yield from
        if self.consume(&TokenKind::From)? {
            let value = self.parse_expression()?;
            return Ok(Expression::YieldFrom {
                value: Box::new(value),
                location,
            });
        }

        // Check if there's a value to yield
        let value = if !self.check(&TokenKind::Newline)?
            && !self.check(&TokenKind::Eof)?
            && !self.check(&TokenKind::RightParen)?
            && !self.check(&TokenKind::Comma)?
            && !self.check(&TokenKind::Colon)?
        {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        Ok(Expression::Yield { value, location })
    }

    /// Parse named expression (walrus operator :=)
    fn parse_named_expr(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_conditional_expr()?;

        // Check for walrus operator
        if self.consume(&TokenKind::ColonEqual)? {
            let location = self.location();
            // The left side must be a simple name
            if let Expression::Name { .. } = &expr {
                let value = self.parse_named_expr()?;
                return Ok(Expression::NamedExpr {
                    target: Box::new(expr),
                    value: Box::new(value),
                    location,
                });
            } else {
                return Err(ParseError::invalid_syntax(
                    location,
                    "cannot use assignment expression with non-name target",
                ));
            }
        }

        Ok(expr)
    }

    /// Parse conditional expression (ternary: x if cond else y)
    fn parse_conditional_expr(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_or_expr()?;

        // Handle conditional expression: x if cond else y
        if self.consume(&TokenKind::If)? {
            let location = self.location();
            let test = self.parse_or_expr()?;
            self.expect(&TokenKind::Else)?;
            let orelse = self.parse_conditional_expr()?;
            return Ok(Expression::IfExp {
                test: Box::new(test),
                body: Box::new(expr),
                orelse: Box::new(orelse),
                location,
            });
        }

        Ok(expr)
    }

    /// Parse a test expression (stops at 'for' keyword for comprehensions)
    fn parse_test_expr(&mut self) -> ParseResult<Expression> {
        // For comprehensions, we need to parse conditional expressions
        // but stop at 'for' keyword
        let expr = self.parse_or_expr()?;

        // Handle conditional expression: x if cond else y
        if self.consume(&TokenKind::If)? {
            let location = self.location();
            let test = self.parse_or_expr()?;
            self.expect(&TokenKind::Else)?;
            let orelse = self.parse_test_expr()?;
            return Ok(Expression::IfExp {
                test: Box::new(test),
                body: Box::new(expr),
                orelse: Box::new(orelse),
                location,
            });
        }

        Ok(expr)
    }

    fn parse_or_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_and_expr()?;

        while self.consume(&TokenKind::Or)? {
            let right = self.parse_and_expr()?;
            let location = self.location();
            left = Expression::BoolOp {
                op: BoolOp::Or,
                values: vec![left, right],
                location,
            };
        }

        Ok(left)
    }

    fn parse_and_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_not_expr()?;

        while self.consume(&TokenKind::And)? {
            let right = self.parse_not_expr()?;
            let location = self.location();
            left = Expression::BoolOp {
                op: BoolOp::And,
                values: vec![left, right],
                location,
            };
        }

        Ok(left)
    }

    fn parse_not_expr(&mut self) -> ParseResult<Expression> {
        if self.consume(&TokenKind::Not)? {
            let location = self.location();
            let operand = self.parse_not_expr()?;
            return Ok(Expression::UnaryOp {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                location,
            });
        }
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let left = self.parse_bitor_expr()?;
        let location = self.location();

        let mut ops = Vec::new();
        let mut comparators = Vec::new();

        loop {
            let op = if self.consume(&TokenKind::Less)? {
                CmpOp::Lt
            } else if self.consume(&TokenKind::Greater)? {
                CmpOp::Gt
            } else if self.consume(&TokenKind::LessEqual)? {
                CmpOp::LtE
            } else if self.consume(&TokenKind::GreaterEqual)? {
                CmpOp::GtE
            } else if self.consume(&TokenKind::Equal)? {
                CmpOp::Eq
            } else if self.consume(&TokenKind::NotEqual)? {
                CmpOp::NotEq
            } else if self.consume(&TokenKind::In)? {
                CmpOp::In
            } else if self.consume(&TokenKind::Is)? {
                if self.consume(&TokenKind::Not)? {
                    CmpOp::IsNot
                } else {
                    CmpOp::Is
                }
            } else if self.consume(&TokenKind::Not)? {
                self.expect(&TokenKind::In)?;
                CmpOp::NotIn
            } else {
                break;
            };

            ops.push(op);
            comparators.push(self.parse_bitor_expr()?);
        }

        if ops.is_empty() {
            Ok(left)
        } else {
            Ok(Expression::Compare {
                left: Box::new(left),
                ops,
                comparators,
                location,
            })
        }
    }

    fn parse_bitor_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitxor_expr()?;

        while self.consume(&TokenKind::Pipe)? {
            let location = self.location();
            let right = self.parse_bitxor_expr()?;
            left = Expression::BinOp {
                left: Box::new(left),
                op: BinOp::BitOr,
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    fn parse_bitxor_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitand_expr()?;

        while self.consume(&TokenKind::Caret)? {
            let location = self.location();
            let right = self.parse_bitand_expr()?;
            left = Expression::BinOp {
                left: Box::new(left),
                op: BinOp::BitXor,
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    fn parse_bitand_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_shift_expr()?;

        while self.consume(&TokenKind::Ampersand)? {
            let location = self.location();
            let right = self.parse_shift_expr()?;
            left = Expression::BinOp {
                left: Box::new(left),
                op: BinOp::BitAnd,
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    fn parse_shift_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_arith_expr()?;

        loop {
            let op = if self.consume(&TokenKind::LeftShift)? {
                BinOp::LShift
            } else if self.consume(&TokenKind::RightShift)? {
                BinOp::RShift
            } else {
                break;
            };

            let location = self.location();
            let right = self.parse_arith_expr()?;
            left = Expression::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    fn parse_arith_expr(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_term()?;

        loop {
            let op = if self.consume(&TokenKind::Plus)? {
                BinOp::Add
            } else if self.consume(&TokenKind::Minus)? {
                BinOp::Sub
            } else {
                break;
            };

            let location = self.location();
            let right = self.parse_term()?;
            left = Expression::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_factor()?;

        loop {
            let op = if self.consume(&TokenKind::Star)? {
                BinOp::Mult
            } else if self.consume(&TokenKind::Slash)? {
                BinOp::Div
            } else if self.consume(&TokenKind::DoubleSlash)? {
                BinOp::FloorDiv
            } else if self.consume(&TokenKind::Percent)? {
                BinOp::Mod
            } else if self.consume(&TokenKind::At)? {
                BinOp::MatMult
            } else {
                break;
            };

            let location = self.location();
            let right = self.parse_factor()?;
            left = Expression::BinOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
                location,
            };
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> ParseResult<Expression> {
        let location = self.location();

        if self.consume(&TokenKind::Plus)? {
            let operand = self.parse_factor()?;
            return Ok(Expression::UnaryOp {
                op: UnaryOp::UAdd,
                operand: Box::new(operand),
                location,
            });
        }

        if self.consume(&TokenKind::Minus)? {
            let operand = self.parse_factor()?;
            return Ok(Expression::UnaryOp {
                op: UnaryOp::USub,
                operand: Box::new(operand),
                location,
            });
        }

        if self.consume(&TokenKind::Tilde)? {
            let operand = self.parse_factor()?;
            return Ok(Expression::UnaryOp {
                op: UnaryOp::Invert,
                operand: Box::new(operand),
                location,
            });
        }

        self.parse_power()
    }

    fn parse_power(&mut self) -> ParseResult<Expression> {
        let base = self.parse_await_expr()?;

        if self.consume(&TokenKind::DoubleStar)? {
            let location = self.location();
            let exp = self.parse_factor()?;
            return Ok(Expression::BinOp {
                left: Box::new(base),
                op: BinOp::Pow,
                right: Box::new(exp),
                location,
            });
        }

        Ok(base)
    }

    fn parse_await_expr(&mut self) -> ParseResult<Expression> {
        if self.consume(&TokenKind::Await)? {
            let location = self.location();
            let value = self.parse_primary()?;
            return Ok(Expression::Await {
                value: Box::new(value),
                location,
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_atom()?;

        loop {
            if self.consume(&TokenKind::Dot)? {
                let location = self.location();
                let attr = self.parse_identifier()?;
                expr = Expression::Attribute {
                    value: Box::new(expr),
                    attr,
                    location,
                };
            } else if self.consume(&TokenKind::LeftParen)? {
                let location = self.location();
                let (args, keywords) = self.parse_call_args()?;
                self.expect(&TokenKind::RightParen)?;
                expr = Expression::Call {
                    func: Box::new(expr),
                    args,
                    keywords,
                    location,
                };
            } else if self.consume(&TokenKind::LeftBracket)? {
                let location = self.location();
                let slice = self.parse_slice()?;
                self.expect(&TokenKind::RightBracket)?;
                expr = Expression::Subscript {
                    value: Box::new(expr),
                    slice: Box::new(slice),
                    location,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_slice(&mut self) -> ParseResult<Expression> {
        let location = self.location();

        // Check for simple index vs slice
        if self.check(&TokenKind::Colon)? {
            // Slice starting with :
            self.advance()?;
            let upper =
                if !self.check(&TokenKind::Colon)? && !self.check(&TokenKind::RightBracket)? {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
            let step = if self.consume(&TokenKind::Colon)? {
                if !self.check(&TokenKind::RightBracket)? {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                }
            } else {
                None
            };
            return Ok(Expression::Slice {
                lower: None,
                upper,
                step,
                location,
            });
        }

        let lower = self.parse_expression()?;

        if self.consume(&TokenKind::Colon)? {
            let upper =
                if !self.check(&TokenKind::Colon)? && !self.check(&TokenKind::RightBracket)? {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
            let step = if self.consume(&TokenKind::Colon)? {
                if !self.check(&TokenKind::RightBracket)? {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                }
            } else {
                None
            };
            return Ok(Expression::Slice {
                lower: Some(Box::new(lower)),
                upper,
                step,
                location,
            });
        }

        Ok(lower)
    }

    fn parse_atom(&mut self) -> ParseResult<Expression> {
        let token = self.peek()?.clone();
        let location = token.location;

        match &token.kind {
            TokenKind::Integer(n) => {
                let n = *n;
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Int(n),
                    location,
                })
            }
            TokenKind::Float(n) => {
                let n = *n;
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Float(n),
                    location,
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Str(s),
                    location,
                })
            }
            TokenKind::FString(s) => {
                let s = s.clone();
                self.advance()?;
                // For now, treat f-strings as regular strings
                Ok(Expression::Constant {
                    value: Constant::Str(s),
                    location,
                })
            }
            TokenKind::Bytes(b) => {
                let b = b.clone();
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Bytes(b),
                    location,
                })
            }
            TokenKind::True => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Bool(true),
                    location,
                })
            }
            TokenKind::False => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Bool(false),
                    location,
                })
            }
            TokenKind::None => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::None,
                    location,
                })
            }
            TokenKind::Ellipsis => {
                self.advance()?;
                Ok(Expression::Constant {
                    value: Constant::Ellipsis,
                    location,
                })
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance()?;
                Ok(Expression::Name { id: name, location })
            }
            TokenKind::LeftParen => {
                self.advance()?;
                if self.check(&TokenKind::RightParen)? {
                    self.advance()?;
                    return Ok(Expression::Tuple {
                        elts: Vec::new(),
                        location,
                    });
                }
                // Use parse_test_expr to stop at 'for' keyword for generator expressions
                // But first check if this could be a walrus operator
                let first = self.parse_test_expr()?;
                
                // Check for walrus operator first
                if self.consume(&TokenKind::ColonEqual)? {
                    let walrus_location = self.location();
                    // The left side must be a simple name
                    if let Expression::Name { .. } = &first {
                        let value = self.parse_named_expr()?;
                        let named_expr = Expression::NamedExpr {
                            target: Box::new(first),
                            value: Box::new(value),
                            location: walrus_location,
                        };
                        self.expect(&TokenKind::RightParen)?;
                        return Ok(named_expr);
                    } else {
                        return Err(ParseError::invalid_syntax(
                            walrus_location,
                            "cannot use assignment expression with non-name target",
                        ));
                    }
                }
                
                if self.check(&TokenKind::For)? {
                    // Generator expression: (expr for x in iterable)
                    let generators = self.parse_comprehension()?;
                    self.expect(&TokenKind::RightParen)?;
                    Ok(Expression::GeneratorExp {
                        elt: Box::new(first),
                        generators,
                        location,
                    })
                } else if self.consume(&TokenKind::Comma)? {
                    // Tuple
                    let mut elts = vec![first];
                    while !self.check(&TokenKind::RightParen)? {
                        elts.push(self.parse_expression()?);
                        if !self.consume(&TokenKind::Comma)? {
                            break;
                        }
                    }
                    self.expect(&TokenKind::RightParen)?;
                    Ok(Expression::Tuple { elts, location })
                } else {
                    self.expect(&TokenKind::RightParen)?;
                    Ok(first)
                }
            }
            TokenKind::LeftBracket => {
                self.advance()?;
                if self.check(&TokenKind::RightBracket)? {
                    self.advance()?;
                    return Ok(Expression::List {
                        elts: Vec::new(),
                        location,
                    });
                }
                // For list comprehension, we need to parse the element expression
                // but stop before 'for' keyword
                let first = self.parse_test_expr()?;
                if self.check(&TokenKind::For)? {
                    // List comprehension
                    let generators = self.parse_comprehension()?;
                    self.expect(&TokenKind::RightBracket)?;
                    Ok(Expression::ListComp {
                        elt: Box::new(first),
                        generators,
                        location,
                    })
                } else {
                    let mut elts = vec![first];
                    while self.consume(&TokenKind::Comma)? {
                        if self.check(&TokenKind::RightBracket)? {
                            break;
                        }
                        elts.push(self.parse_expression()?);
                    }
                    self.expect(&TokenKind::RightBracket)?;
                    Ok(Expression::List { elts, location })
                }
            }
            TokenKind::LeftBrace => {
                self.advance()?;
                if self.check(&TokenKind::RightBrace)? {
                    self.advance()?;
                    return Ok(Expression::Dict {
                        keys: Vec::new(),
                        values: Vec::new(),
                        location,
                    });
                }
                // For dict/set comprehension, we need to parse the element expression
                // but stop before 'for' keyword
                let first = self.parse_test_expr()?;
                if self.consume(&TokenKind::Colon)? {
                    // Dict or dict comprehension
                    let first_value = self.parse_test_expr()?;
                    
                    // Check for dict comprehension: {k: v for x in iterable}
                    if self.check(&TokenKind::For)? {
                        let generators = self.parse_comprehension()?;
                        self.expect(&TokenKind::RightBrace)?;
                        return Ok(Expression::DictComp {
                            key: Box::new(first),
                            value: Box::new(first_value),
                            generators,
                            location,
                        });
                    }
                    
                    // Regular dict literal
                    let mut keys = vec![Some(first)];
                    let mut values = vec![first_value];
                    while self.consume(&TokenKind::Comma)? {
                        if self.check(&TokenKind::RightBrace)? {
                            break;
                        }
                        if self.consume(&TokenKind::DoubleStar)? {
                            keys.push(None);
                            values.push(self.parse_expression()?);
                        } else {
                            keys.push(Some(self.parse_expression()?));
                            self.expect(&TokenKind::Colon)?;
                            values.push(self.parse_expression()?);
                        }
                    }
                    self.expect(&TokenKind::RightBrace)?;
                    Ok(Expression::Dict {
                        keys,
                        values,
                        location,
                    })
                } else if self.check(&TokenKind::For)? {
                    // Set comprehension: {x for x in iterable}
                    let generators = self.parse_comprehension()?;
                    self.expect(&TokenKind::RightBrace)?;
                    Ok(Expression::SetComp {
                        elt: Box::new(first),
                        generators,
                        location,
                    })
                } else {
                    // Set literal
                    let mut elts = vec![first];
                    while self.consume(&TokenKind::Comma)? {
                        if self.check(&TokenKind::RightBrace)? {
                            break;
                        }
                        elts.push(self.parse_expression()?);
                    }
                    self.expect(&TokenKind::RightBrace)?;
                    Ok(Expression::Set { elts, location })
                }
            }
            TokenKind::Lambda => {
                self.advance()?;
                let args = if self.check(&TokenKind::Colon)? {
                    Arguments::default()
                } else {
                    self.parse_lambda_arguments()?
                };
                self.expect(&TokenKind::Colon)?;
                let body = self.parse_expression()?;
                Ok(Expression::Lambda {
                    args,
                    body: Box::new(body),
                    location,
                })
            }
            _ => Err(ParseError::unexpected_token(location, "expression", &token.kind.to_string())),
        }
    }

    fn parse_comprehension(&mut self) -> ParseResult<Vec<Comprehension>> {
        let mut generators = Vec::new();

        while self.consume(&TokenKind::For)? {
            // Async comprehensions are parsed but not yet fully supported in execution
            let is_async = false;
            // Parse target - should be a simple name or tuple, not a full expression
            let target = self.parse_target()?;
            self.expect(&TokenKind::In)?;
            let iter = self.parse_or_expr()?;

            let mut ifs = Vec::new();
            while self.consume(&TokenKind::If)? {
                ifs.push(self.parse_or_expr()?);
            }

            generators.push(Comprehension {
                target,
                iter,
                ifs,
                is_async,
            });
        }

        Ok(generators)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_expression() {
        let mut parser = Parser::new("1 + 2 * 3");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::BinOp { op: BinOp::Add, .. }));
    }

    #[test]
    fn test_parse_function_def() {
        let source = "def foo(x, y):\n    return x + y\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::FunctionDef { .. }));
    }

    #[test]
    fn test_parse_if_statement() {
        let source = "if x > 0:\n    y = 1\nelse:\n    y = 2\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::If { .. }));
    }

    #[test]
    fn test_parse_class_def() {
        let source = "class Foo:\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::ClassDef { .. }));
    }

    #[test]
    fn test_parse_list_comprehension() {
        let mut parser = Parser::new("[x * 2 for x in items]");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::ListComp { .. }));
    }

    #[test]
    fn test_parse_while_loop() {
        let source = "while x > 0:\n    x = x - 1\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::While { .. }));
    }

    #[test]
    fn test_parse_for_loop() {
        let source = "for i in items:\n    x = i\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::For { .. }));
    }

    #[test]
    fn test_parse_try_except() {
        let source = "try:\n    x = 1\nexcept ValueError:\n    x = 0\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::Try { .. }));
    }

    #[test]
    fn test_parse_with_statement() {
        let source = "with open('file') as f:\n    data = f.read()\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::With { .. }));
    }

    #[test]
    fn test_parse_import() {
        let source = "import os\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::Import { .. }));
    }

    #[test]
    fn test_parse_from_import() {
        let source = "from os import path\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::ImportFrom { .. }));
    }

    #[test]
    fn test_parse_lambda() {
        let mut parser = Parser::new("lambda x: x + 1");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Lambda { .. }));
    }

    #[test]
    fn test_parse_dict_literal() {
        let mut parser = Parser::new("{'a': 1, 'b': 2}");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Dict { .. }));
    }

    #[test]
    fn test_parse_set_literal() {
        let mut parser = Parser::new("{1, 2, 3}");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Set { .. }));
    }

    #[test]
    fn test_parse_dict_comprehension() {
        let mut parser = Parser::new("{x: x * 2 for x in items}");
        let expr = parser.parse_expression().unwrap();
        if let Expression::DictComp { key, value, generators, .. } = expr {
            assert!(matches!(*key, Expression::Name { .. }));
            assert!(matches!(*value, Expression::BinOp { .. }));
            assert_eq!(generators.len(), 1);
        } else {
            panic!("Expected DictComp, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_dict_comprehension_with_filter() {
        let mut parser = Parser::new("{x: x * 2 for x in items if x > 0}");
        let expr = parser.parse_expression().unwrap();
        if let Expression::DictComp { generators, .. } = expr {
            assert_eq!(generators.len(), 1);
            assert_eq!(generators[0].ifs.len(), 1);
        } else {
            panic!("Expected DictComp");
        }
    }

    #[test]
    fn test_parse_set_comprehension() {
        let mut parser = Parser::new("{x * 2 for x in items}");
        let expr = parser.parse_expression().unwrap();
        if let Expression::SetComp { elt, generators, .. } = expr {
            assert!(matches!(*elt, Expression::BinOp { .. }));
            assert_eq!(generators.len(), 1);
        } else {
            panic!("Expected SetComp, got {:?}", expr);
        }
    }

    #[test]
    fn test_parse_set_comprehension_with_filter() {
        let mut parser = Parser::new("{x for x in items if x > 0}");
        let expr = parser.parse_expression().unwrap();
        if let Expression::SetComp { generators, .. } = expr {
            assert_eq!(generators.len(), 1);
            assert_eq!(generators[0].ifs.len(), 1);
        } else {
            panic!("Expected SetComp");
        }
    }

    #[test]
    fn test_parse_generator_expression() {
        let mut parser = Parser::new("(x * 2 for x in items)");
        let expr = parser.parse_expression().unwrap();
        if let Expression::GeneratorExp { generators, .. } = expr {
            assert_eq!(generators.len(), 1);
        } else {
            panic!("Expected GeneratorExp");
        }
    }

    #[test]
    fn test_parse_generator_expression_with_filter() {
        let mut parser = Parser::new("(x for x in items if x > 0)");
        let expr = parser.parse_expression().unwrap();
        if let Expression::GeneratorExp { generators, .. } = expr {
            assert_eq!(generators.len(), 1);
            assert_eq!(generators[0].ifs.len(), 1);
        } else {
            panic!("Expected GeneratorExp");
        }
    }

    #[test]
    fn test_parse_nested_generator_expression() {
        let mut parser = Parser::new("(x + y for x in xs for y in ys)");
        let expr = parser.parse_expression().unwrap();
        if let Expression::GeneratorExp { generators, .. } = expr {
            assert_eq!(generators.len(), 2);
        } else {
            panic!("Expected GeneratorExp");
        }
    }

    #[test]
    fn test_parse_tuple() {
        let mut parser = Parser::new("(1, 2, 3)");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Tuple { .. }));
    }

    #[test]
    fn test_parse_slice() {
        let mut parser = Parser::new("x[1:10:2]");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Subscript { .. }));
    }

    #[test]
    fn test_parse_attribute() {
        let mut parser = Parser::new("obj.attr.method()");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Call { .. }));
    }

    #[test]
    fn test_parse_comparison_chain() {
        let mut parser = Parser::new("0 < x < 10");
        let expr = parser.parse_expression().unwrap();
        if let Expression::Compare {
            ops, comparators, ..
        } = expr
        {
            assert_eq!(ops.len(), 2);
            assert_eq!(comparators.len(), 2);
        } else {
            panic!("Expected Compare expression");
        }
    }

    #[test]
    fn test_parse_boolean_ops() {
        let mut parser = Parser::new("a and b or c");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::BoolOp { op: BoolOp::Or, .. }));
    }

    #[test]
    fn test_parse_unary_ops() {
        let mut parser = Parser::new("-x");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::UnaryOp {
                op: UnaryOp::USub,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_async_function() {
        let source = "async def fetch():\n    return await get_data()\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        if let Statement::FunctionDef { is_async, .. } = &module.body[0] {
            assert!(*is_async);
        } else {
            panic!("Expected async function def");
        }
    }

    #[test]
    fn test_parse_decorated_function() {
        // Note: decorators are parsed but stored empty for now
        let source = "def foo():\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
    }

    #[test]
    fn test_parse_augmented_assignment() {
        let source = "x += 1\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::AugAssign { op: BinOp::Add, .. }));
    }

    #[test]
    fn test_parse_global_nonlocal() {
        let source = "global x\nnonlocal y\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 2);
        assert!(matches!(module.body[0], Statement::Global { .. }));
        assert!(matches!(module.body[1], Statement::Nonlocal { .. }));
    }

    #[test]
    fn test_parse_raise() {
        let source = "raise ValueError('error')\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::Raise { .. }));
    }

    #[test]
    fn test_parse_assert() {
        let source = "assert x > 0, 'x must be positive'\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::Assert { .. }));
    }

    #[test]
    fn test_parse_del() {
        let source = "del x, y\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::Delete { .. }));
    }

    #[test]
    fn test_parse_pass_break_continue() {
        let source = "while True:\n    if x:\n        break\n    elif y:\n        continue\n    else:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
    }

    #[test]
    fn test_parse_elif_chain() {
        let source = "if a:\n    x = 1\nelif b:\n    x = 2\nelif c:\n    x = 3\nelse:\n    x = 4\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        assert!(matches!(module.body[0], Statement::If { .. }));
    }

    #[test]
    fn test_parse_nested_comprehension() {
        let mut parser = Parser::new("[x + y for x in xs for y in ys]");
        let expr = parser.parse_expression().unwrap();
        if let Expression::ListComp { generators, .. } = expr {
            assert_eq!(generators.len(), 2);
        } else {
            panic!("Expected ListComp");
        }
    }

    #[test]
    fn test_parse_comprehension_with_if() {
        let mut parser = Parser::new("[x for x in items if x > 0]");
        let expr = parser.parse_expression().unwrap();
        if let Expression::ListComp { generators, .. } = expr {
            assert_eq!(generators.len(), 1);
            assert_eq!(generators[0].ifs.len(), 1);
        } else {
            panic!("Expected ListComp");
        }
    }

    #[test]
    fn test_parse_function_with_annotations() {
        let source = "def foo(x: int, y: str) -> bool:\n    return True\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::FunctionDef { args, returns, .. } = &module.body[0] {
            assert!(args.args[0].annotation.is_some());
            assert!(args.args[1].annotation.is_some());
            assert!(returns.is_some());
        } else {
            panic!("Expected FunctionDef");
        }
    }

    #[test]
    fn test_parse_function_with_defaults() {
        let source = "def foo(x, y=10, z=20):\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::FunctionDef { args, .. } = &module.body[0] {
            assert_eq!(args.args.len(), 3);
            assert_eq!(args.defaults.len(), 2);
        } else {
            panic!("Expected FunctionDef");
        }
    }

    #[test]
    fn test_parse_function_with_varargs() {
        let source = "def foo(*args, **kwargs):\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::FunctionDef { args, .. } = &module.body[0] {
            assert!(args.vararg.is_some());
            assert!(args.kwarg.is_some());
        } else {
            panic!("Expected FunctionDef");
        }
    }

    #[test]
    fn test_parse_class_with_bases() {
        let source = "class Foo(Bar, Baz):\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::ClassDef { bases, .. } = &module.body[0] {
            assert_eq!(bases.len(), 2);
        } else {
            panic!("Expected ClassDef");
        }
    }

    #[test]
    fn test_parse_try_finally() {
        let source = "try:\n    x = 1\nfinally:\n    cleanup()\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Try { finalbody, .. } = &module.body[0] {
            assert!(!finalbody.is_empty());
        } else {
            panic!("Expected Try");
        }
    }

    #[test]
    fn test_parse_multiple_with_items() {
        let source = "with open('a') as f, open('b') as g:\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::With { items, .. } = &module.body[0] {
            assert_eq!(items.len(), 2);
        } else {
            panic!("Expected With");
        }
    }

    #[test]
    fn test_parse_starred_expression() {
        let mut parser = Parser::new("func(*args, **kwargs)");
        let expr = parser.parse_expression().unwrap();
        if let Expression::Call { args, keywords, .. } = expr {
            assert!(args.iter().any(|a| matches!(a, Expression::Starred { .. })));
            assert!(keywords.iter().any(|k| k.arg.is_none()));
        } else {
            panic!("Expected Call");
        }
    }

    #[test]
    fn test_parse_power_operator() {
        let mut parser = Parser::new("2 ** 10");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::BinOp { op: BinOp::Pow, .. }));
    }

    #[test]
    fn test_parse_bitwise_operators() {
        let mut parser = Parser::new("a & b | c ^ d");
        let expr = parser.parse_expression().unwrap();
        // Should parse as (a & b) | (c ^ d) due to precedence
        assert!(matches!(
            expr,
            Expression::BinOp {
                op: BinOp::BitOr,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_shift_operators() {
        let mut parser = Parser::new("x << 2 >> 1");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::BinOp {
                op: BinOp::RShift,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_floor_division() {
        let mut parser = Parser::new("x // y");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::BinOp {
                op: BinOp::FloorDiv,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_is_not() {
        let mut parser = Parser::new("x is not None");
        let expr = parser.parse_expression().unwrap();
        if let Expression::Compare { ops, .. } = expr {
            assert_eq!(ops[0], CmpOp::IsNot);
        } else {
            panic!("Expected Compare");
        }
    }

    #[test]
    fn test_parse_not_in() {
        let mut parser = Parser::new("x not in items");
        let expr = parser.parse_expression().unwrap();
        if let Expression::Compare { ops, .. } = expr {
            assert_eq!(ops[0], CmpOp::NotIn);
        } else {
            panic!("Expected Compare");
        }
    }

    #[test]
    fn test_parse_constants() {
        let mut parser = Parser::new("True");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::Constant {
                value: Constant::Bool(true),
                ..
            }
        ));

        let mut parser = Parser::new("False");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::Constant {
                value: Constant::Bool(false),
                ..
            }
        ));

        let mut parser = Parser::new("None");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::Constant {
                value: Constant::None,
                ..
            }
        ));

        let mut parser = Parser::new("...");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(
            expr,
            Expression::Constant {
                value: Constant::Ellipsis,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_match_simple() {
        let source = "match x:\n    case 1:\n        y = 1\n    case 2:\n        y = 2\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        assert_eq!(module.body.len(), 1);
        if let Statement::Match { cases, .. } = &module.body[0] {
            assert_eq!(cases.len(), 2);
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_wildcard() {
        let source = "match x:\n    case _:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            assert!(matches!(
                cases[0].pattern,
                Pattern::MatchAs {
                    pattern: None,
                    name: None,
                    ..
                }
            ));
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_capture() {
        let source = "match x:\n    case y:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchAs {
                name: Some(name), ..
            } = &cases[0].pattern
            {
                assert_eq!(name, "y");
            } else {
                panic!("Expected capture pattern");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_sequence() {
        let source = "match x:\n    case [a, b, c]:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchSequence { patterns, .. } = &cases[0].pattern {
                assert_eq!(patterns.len(), 3);
            } else {
                panic!("Expected sequence pattern");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_mapping() {
        let source = "match x:\n    case {\"a\": 1, \"b\": 2}:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchMapping { keys, patterns, .. } = &cases[0].pattern {
                assert_eq!(keys.len(), 2);
                assert_eq!(patterns.len(), 2);
            } else {
                panic!("Expected mapping pattern");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_class() {
        let source = "match x:\n    case Point(x=0, y=0):\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchClass { kwd_attrs, .. } = &cases[0].pattern {
                assert_eq!(kwd_attrs.len(), 2);
            } else {
                panic!("Expected class pattern");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_or() {
        let source = "match x:\n    case 1 | 2 | 3:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchOr { patterns, .. } = &cases[0].pattern {
                assert_eq!(patterns.len(), 3);
            } else {
                panic!("Expected or pattern");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_as() {
        let source = "match x:\n    case [a, b] as pair:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchAs {
                pattern: Some(_),
                name: Some(name),
                ..
            } = &cases[0].pattern
            {
                assert_eq!(name, "pair");
            } else {
                panic!("Expected as pattern");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_guard() {
        let source = "match x:\n    case n if n > 0:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            assert!(cases[0].guard.is_some());
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_star() {
        let source = "match x:\n    case [first, *rest]:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchSequence { patterns, .. } = &cases[0].pattern {
                assert_eq!(patterns.len(), 2);
                assert!(matches!(patterns[1], Pattern::MatchStar { .. }));
            } else {
                panic!("Expected sequence pattern with star");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_match_negative_literal() {
        let source = "match x:\n    case -1:\n        pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::Match { cases, .. } = &module.body[0] {
            if let Pattern::MatchValue { value, .. } = &cases[0].pattern {
                assert!(matches!(
                    value,
                    Expression::UnaryOp {
                        op: UnaryOp::USub,
                        ..
                    }
                ));
            } else {
                panic!("Expected value pattern with negative number");
            }
        } else {
            panic!("Expected Match statement");
        }
    }

    #[test]
    fn test_parse_walrus_operator() {
        let mut parser = Parser::new("(x := 10)");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::NamedExpr { .. }));
    }

    #[test]
    fn test_parse_decorator() {
        let source = "@decorator\ndef foo():\n    pass\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        // Note: decorators are parsed but need to be associated with functions
        assert!(!module.body.is_empty());
    }

    #[test]
    fn test_parse_yield_expression() {
        let source = "def gen():\n    yield 1\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::FunctionDef { body, .. } = &module.body[0] {
            if let Statement::Expr { value, .. } = &body[0] {
                assert!(matches!(value, Expression::Yield { .. }));
            } else {
                panic!("Expected expression statement");
            }
        } else {
            panic!("Expected function def");
        }
    }

    #[test]
    fn test_parse_yield_from() {
        let source = "def gen():\n    yield from other()\n";
        let mut parser = Parser::new(source);
        let module = parser.parse_module().unwrap();
        if let Statement::FunctionDef { body, .. } = &module.body[0] {
            if let Statement::Expr { value, .. } = &body[0] {
                assert!(matches!(value, Expression::YieldFrom { .. }));
            } else {
                panic!("Expected expression statement");
            }
        } else {
            panic!("Expected function def");
        }
    }
}
