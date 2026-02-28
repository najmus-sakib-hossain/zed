//! Python Source Parser for DX-Py Runtime
//!
//! This crate provides a Python 3.12+ compatible parser that generates
//! an AST suitable for compilation to DPB bytecode.
//!
//! ## Features
//!
//! - Full Python 3.12+ syntax support
//! - Indentation-aware lexer
//! - Comprehensive error messages with line/column information
//! - AST compatible with DPB compiler
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_py_parser::{Parser, parse_module};
//!
//! let source = r#"
//! def hello(name):
//!     print(f"Hello, {name}!")
//!
//! hello("World")
//! "#;
//!
//! let module = parse_module(source)?;
//! ```

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod printer;
pub mod token;

pub use ast::*;
pub use error::{ParseError, ParseResult};
pub use lexer::Lexer;
pub use parser::Parser;
pub use printer::{print_expression, print_module, print_statement, Printer};

/// Parse a Python source string into a module AST.
pub fn parse_module(source: &str) -> ParseResult<Module> {
    let mut parser = Parser::new(source);
    parser.parse_module()
}

/// Parse a single Python expression.
pub fn parse_expression(source: &str) -> ParseResult<Expression> {
    let mut parser = Parser::new(source);
    parser.parse_expression()
}

/// Parse a single Python statement.
pub fn parse_statement(source: &str) -> ParseResult<Statement> {
    let mut parser = Parser::new(source);
    parser.parse_statement()
}
