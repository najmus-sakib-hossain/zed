//! Token types for Python lexer

use crate::error::Location;

/// Python token types
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    FString(String),
    Bytes(Vec<u8>),
    True,
    False,
    None,

    // Identifiers
    Identifier(String),

    // Keywords
    And,
    As,
    Assert,
    Async,
    Await,
    Break,
    Class,
    Continue,
    Def,
    Del,
    Elif,
    Else,
    Except,
    Finally,
    For,
    From,
    Global,
    If,
    Import,
    In,
    Is,
    Lambda,
    Nonlocal,
    Not,
    Or,
    Pass,
    Raise,
    Return,
    Try,
    While,
    With,
    Yield,
    Match,
    Case,
    Type,

    // Operators
    Plus,        // +
    Minus,       // -
    Star,        // *
    DoubleStar,  // **
    Slash,       // /
    DoubleSlash, // //
    Percent,     // %
    At,          // @
    AtEqual,     // @=
    Ampersand,   // &
    Pipe,        // |
    Caret,       // ^
    Tilde,       // ~
    LeftShift,   // <<
    RightShift,  // >>

    // Comparison
    Less,         // <
    Greater,      // >
    LessEqual,    // <=
    GreaterEqual, // >=
    Equal,        // ==
    NotEqual,     // !=

    // Assignment
    Assign,           // =
    PlusEqual,        // +=
    MinusEqual,       // -=
    StarEqual,        // *=
    SlashEqual,       // /=
    DoubleSlashEqual, // //=
    PercentEqual,     // %=
    DoubleStarEqual,  // **=
    AmpersandEqual,   // &=
    PipeEqual,        // |=
    CaretEqual,       // ^=
    LeftShiftEqual,   // <<=
    RightShiftEqual,  // >>=
    ColonEqual,       // :=

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Comma,        // ,
    Colon,        // :
    Semicolon,    // ;
    Dot,          // .
    Arrow,        // ->
    Ellipsis,     // ...

    // Special
    Newline,
    Indent,
    Dedent,
    Eof,
}

impl TokenKind {
    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::And
                | TokenKind::As
                | TokenKind::Assert
                | TokenKind::Async
                | TokenKind::Await
                | TokenKind::Break
                | TokenKind::Class
                | TokenKind::Continue
                | TokenKind::Def
                | TokenKind::Del
                | TokenKind::Elif
                | TokenKind::Else
                | TokenKind::Except
                | TokenKind::Finally
                | TokenKind::For
                | TokenKind::From
                | TokenKind::Global
                | TokenKind::If
                | TokenKind::Import
                | TokenKind::In
                | TokenKind::Is
                | TokenKind::Lambda
                | TokenKind::Nonlocal
                | TokenKind::Not
                | TokenKind::Or
                | TokenKind::Pass
                | TokenKind::Raise
                | TokenKind::Return
                | TokenKind::Try
                | TokenKind::While
                | TokenKind::With
                | TokenKind::Yield
                | TokenKind::Match
                | TokenKind::Case
                | TokenKind::Type
        )
    }

    /// Get keyword from string
    pub fn keyword_from_str(s: &str) -> Option<TokenKind> {
        match s {
            "and" => Some(TokenKind::And),
            "as" => Some(TokenKind::As),
            "assert" => Some(TokenKind::Assert),
            "async" => Some(TokenKind::Async),
            "await" => Some(TokenKind::Await),
            "break" => Some(TokenKind::Break),
            "class" => Some(TokenKind::Class),
            "continue" => Some(TokenKind::Continue),
            "def" => Some(TokenKind::Def),
            "del" => Some(TokenKind::Del),
            "elif" => Some(TokenKind::Elif),
            "else" => Some(TokenKind::Else),
            "except" => Some(TokenKind::Except),
            "finally" => Some(TokenKind::Finally),
            "for" => Some(TokenKind::For),
            "from" => Some(TokenKind::From),
            "global" => Some(TokenKind::Global),
            "if" => Some(TokenKind::If),
            "import" => Some(TokenKind::Import),
            "in" => Some(TokenKind::In),
            "is" => Some(TokenKind::Is),
            "lambda" => Some(TokenKind::Lambda),
            "nonlocal" => Some(TokenKind::Nonlocal),
            "not" => Some(TokenKind::Not),
            "or" => Some(TokenKind::Or),
            "pass" => Some(TokenKind::Pass),
            "raise" => Some(TokenKind::Raise),
            "return" => Some(TokenKind::Return),
            "try" => Some(TokenKind::Try),
            "while" => Some(TokenKind::While),
            "with" => Some(TokenKind::With),
            "yield" => Some(TokenKind::Yield),
            "match" => Some(TokenKind::Match),
            "case" => Some(TokenKind::Case),
            "type" => Some(TokenKind::Type),
            "True" => Some(TokenKind::True),
            "False" => Some(TokenKind::False),
            "None" => Some(TokenKind::None),
            _ => None,
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Integer(n) => write!(f, "{}", n),
            TokenKind::Float(n) => write!(f, "{}", n),
            TokenKind::String(s) => write!(f, "\"{}\"", s),
            TokenKind::FString(s) => write!(f, "f\"{}\"", s),
            TokenKind::Bytes(b) => write!(f, "b{:?}", b),
            TokenKind::True => write!(f, "True"),
            TokenKind::False => write!(f, "False"),
            TokenKind::None => write!(f, "None"),
            TokenKind::Identifier(s) => write!(f, "{}", s),
            TokenKind::And => write!(f, "and"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Assert => write!(f, "assert"),
            TokenKind::Async => write!(f, "async"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Class => write!(f, "class"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Def => write!(f, "def"),
            TokenKind::Del => write!(f, "del"),
            TokenKind::Elif => write!(f, "elif"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Except => write!(f, "except"),
            TokenKind::Finally => write!(f, "finally"),
            TokenKind::For => write!(f, "for"),
            TokenKind::From => write!(f, "from"),
            TokenKind::Global => write!(f, "global"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Is => write!(f, "is"),
            TokenKind::Lambda => write!(f, "lambda"),
            TokenKind::Nonlocal => write!(f, "nonlocal"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Pass => write!(f, "pass"),
            TokenKind::Raise => write!(f, "raise"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Try => write!(f, "try"),
            TokenKind::While => write!(f, "while"),
            TokenKind::With => write!(f, "with"),
            TokenKind::Yield => write!(f, "yield"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Case => write!(f, "case"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::DoubleStar => write!(f, "**"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::DoubleSlash => write!(f, "//"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::At => write!(f, "@"),
            TokenKind::AtEqual => write!(f, "@="),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::LeftShift => write!(f, "<<"),
            TokenKind::RightShift => write!(f, ">>"),
            TokenKind::Less => write!(f, "<"),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::Equal => write!(f, "=="),
            TokenKind::NotEqual => write!(f, "!="),
            TokenKind::Assign => write!(f, "="),
            TokenKind::PlusEqual => write!(f, "+="),
            TokenKind::MinusEqual => write!(f, "-="),
            TokenKind::StarEqual => write!(f, "*="),
            TokenKind::SlashEqual => write!(f, "/="),
            TokenKind::DoubleSlashEqual => write!(f, "//="),
            TokenKind::PercentEqual => write!(f, "%="),
            TokenKind::DoubleStarEqual => write!(f, "**="),
            TokenKind::AmpersandEqual => write!(f, "&="),
            TokenKind::PipeEqual => write!(f, "|="),
            TokenKind::CaretEqual => write!(f, "^="),
            TokenKind::LeftShiftEqual => write!(f, "<<="),
            TokenKind::RightShiftEqual => write!(f, ">>="),
            TokenKind::ColonEqual => write!(f, ":="),
            TokenKind::LeftParen => write!(f, "("),
            TokenKind::RightParen => write!(f, ")"),
            TokenKind::LeftBracket => write!(f, "["),
            TokenKind::RightBracket => write!(f, "]"),
            TokenKind::LeftBrace => write!(f, "{{"),
            TokenKind::RightBrace => write!(f, "}}"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::Ellipsis => write!(f, "..."),
            TokenKind::Newline => write!(f, "NEWLINE"),
            TokenKind::Indent => write!(f, "INDENT"),
            TokenKind::Dedent => write!(f, "DEDENT"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// A token with location information
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub location: Location,
}

impl Token {
    pub fn new(kind: TokenKind, location: Location) -> Self {
        Self { kind, location }
    }
}
