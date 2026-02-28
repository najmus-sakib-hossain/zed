//! Python Abstract Syntax Tree types

use crate::error::Location;

/// A Python module (top-level)
#[derive(Debug, Clone)]
pub struct Module {
    pub body: Vec<Statement>,
}

/// Python statements
#[derive(Debug, Clone)]
pub enum Statement {
    /// Function definition
    FunctionDef {
        name: String,
        args: Arguments,
        body: Vec<Statement>,
        decorators: Vec<Expression>,
        returns: Option<Box<Expression>>,
        is_async: bool,
        location: Location,
    },

    /// Class definition
    ClassDef {
        name: String,
        bases: Vec<Expression>,
        keywords: Vec<Keyword>,
        body: Vec<Statement>,
        decorators: Vec<Expression>,
        location: Location,
    },

    /// Return statement
    Return {
        value: Option<Expression>,
        location: Location,
    },

    /// Delete statement
    Delete {
        targets: Vec<Expression>,
        location: Location,
    },

    /// Assignment
    Assign {
        targets: Vec<Expression>,
        value: Expression,
        location: Location,
    },

    /// Augmented assignment (+=, -=, etc.)
    AugAssign {
        target: Expression,
        op: BinOp,
        value: Expression,
        location: Location,
    },

    /// Annotated assignment
    AnnAssign {
        target: Expression,
        annotation: Expression,
        value: Option<Expression>,
        simple: bool,
        location: Location,
    },

    /// For loop
    For {
        target: Expression,
        iter: Expression,
        body: Vec<Statement>,
        orelse: Vec<Statement>,
        is_async: bool,
        location: Location,
    },

    /// While loop
    While {
        test: Expression,
        body: Vec<Statement>,
        orelse: Vec<Statement>,
        location: Location,
    },

    /// If statement
    If {
        test: Expression,
        body: Vec<Statement>,
        orelse: Vec<Statement>,
        location: Location,
    },

    /// With statement
    With {
        items: Vec<WithItem>,
        body: Vec<Statement>,
        is_async: bool,
        location: Location,
    },

    /// Match statement (Python 3.10+)
    Match {
        subject: Expression,
        cases: Vec<MatchCase>,
        location: Location,
    },

    /// Raise statement
    Raise {
        exc: Option<Expression>,
        cause: Option<Expression>,
        location: Location,
    },

    /// Try statement
    Try {
        body: Vec<Statement>,
        handlers: Vec<ExceptHandler>,
        orelse: Vec<Statement>,
        finalbody: Vec<Statement>,
        location: Location,
    },

    /// Assert statement
    Assert {
        test: Expression,
        msg: Option<Expression>,
        location: Location,
    },

    /// Import statement
    Import {
        names: Vec<Alias>,
        location: Location,
    },

    /// From import statement
    ImportFrom {
        module: Option<String>,
        names: Vec<Alias>,
        level: usize,
        location: Location,
    },

    /// Global statement
    Global {
        names: Vec<String>,
        location: Location,
    },

    /// Nonlocal statement
    Nonlocal {
        names: Vec<String>,
        location: Location,
    },

    /// Expression statement
    Expr {
        value: Expression,
        location: Location,
    },

    /// Pass statement
    Pass { location: Location },

    /// Break statement
    Break { location: Location },

    /// Continue statement
    Continue { location: Location },
}

/// Python expressions
#[derive(Debug, Clone)]
pub enum Expression {
    /// Boolean operation (and, or)
    BoolOp {
        op: BoolOp,
        values: Vec<Expression>,
        location: Location,
    },

    /// Named expression (walrus operator :=)
    NamedExpr {
        target: Box<Expression>,
        value: Box<Expression>,
        location: Location,
    },

    /// Binary operation
    BinOp {
        left: Box<Expression>,
        op: BinOp,
        right: Box<Expression>,
        location: Location,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expression>,
        location: Location,
    },

    /// Lambda expression
    Lambda {
        args: Arguments,
        body: Box<Expression>,
        location: Location,
    },

    /// Conditional expression (ternary)
    IfExp {
        test: Box<Expression>,
        body: Box<Expression>,
        orelse: Box<Expression>,
        location: Location,
    },

    /// Dictionary literal
    Dict {
        keys: Vec<Option<Expression>>,
        values: Vec<Expression>,
        location: Location,
    },

    /// Set literal
    Set {
        elts: Vec<Expression>,
        location: Location,
    },

    /// List comprehension
    ListComp {
        elt: Box<Expression>,
        generators: Vec<Comprehension>,
        location: Location,
    },

    /// Set comprehension
    SetComp {
        elt: Box<Expression>,
        generators: Vec<Comprehension>,
        location: Location,
    },

    /// Dict comprehension
    DictComp {
        key: Box<Expression>,
        value: Box<Expression>,
        generators: Vec<Comprehension>,
        location: Location,
    },

    /// Generator expression
    GeneratorExp {
        elt: Box<Expression>,
        generators: Vec<Comprehension>,
        location: Location,
    },

    /// Await expression
    Await {
        value: Box<Expression>,
        location: Location,
    },

    /// Yield expression
    Yield {
        value: Option<Box<Expression>>,
        location: Location,
    },

    /// Yield from expression
    YieldFrom {
        value: Box<Expression>,
        location: Location,
    },

    /// Comparison
    Compare {
        left: Box<Expression>,
        ops: Vec<CmpOp>,
        comparators: Vec<Expression>,
        location: Location,
    },

    /// Function call
    Call {
        func: Box<Expression>,
        args: Vec<Expression>,
        keywords: Vec<Keyword>,
        location: Location,
    },

    /// Formatted value (in f-string)
    FormattedValue {
        value: Box<Expression>,
        conversion: Option<char>,
        format_spec: Option<Box<Expression>>,
        location: Location,
    },

    /// Joined string (f-string)
    JoinedStr {
        values: Vec<Expression>,
        location: Location,
    },

    /// Constant value
    Constant { value: Constant, location: Location },

    /// Attribute access
    Attribute {
        value: Box<Expression>,
        attr: String,
        location: Location,
    },

    /// Subscript
    Subscript {
        value: Box<Expression>,
        slice: Box<Expression>,
        location: Location,
    },

    /// Starred expression
    Starred {
        value: Box<Expression>,
        location: Location,
    },

    /// Name (identifier)
    Name { id: String, location: Location },

    /// List literal
    List {
        elts: Vec<Expression>,
        location: Location,
    },

    /// Tuple literal
    Tuple {
        elts: Vec<Expression>,
        location: Location,
    },

    /// Slice
    Slice {
        lower: Option<Box<Expression>>,
        upper: Option<Box<Expression>>,
        step: Option<Box<Expression>>,
        location: Location,
    },
}

/// Boolean operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    And,
    Or,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mult,
    MatMult,
    Div,
    Mod,
    Pow,
    LShift,
    RShift,
    BitOr,
    BitXor,
    BitAnd,
    FloorDiv,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Invert,
    Not,
    UAdd,
    USub,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Eq,
    NotEq,
    Lt,
    LtE,
    Gt,
    GtE,
    Is,
    IsNot,
    In,
    NotIn,
}

/// Constant values
#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Complex { real: f64, imag: f64 },
    Str(String),
    Bytes(Vec<u8>),
    Ellipsis,
}

/// Function arguments
#[derive(Debug, Clone, Default)]
pub struct Arguments {
    pub posonlyargs: Vec<Arg>,
    pub args: Vec<Arg>,
    pub vararg: Option<Box<Arg>>,
    pub kwonlyargs: Vec<Arg>,
    pub kw_defaults: Vec<Option<Expression>>,
    pub kwarg: Option<Box<Arg>>,
    pub defaults: Vec<Expression>,
}

/// Single argument
#[derive(Debug, Clone)]
pub struct Arg {
    pub arg: String,
    pub annotation: Option<Box<Expression>>,
    pub location: Location,
}

/// Keyword argument
#[derive(Debug, Clone)]
pub struct Keyword {
    pub arg: Option<String>,
    pub value: Expression,
    pub location: Location,
}

/// Comprehension clause
#[derive(Debug, Clone)]
pub struct Comprehension {
    pub target: Expression,
    pub iter: Expression,
    pub ifs: Vec<Expression>,
    pub is_async: bool,
}

/// Exception handler
#[derive(Debug, Clone)]
pub struct ExceptHandler {
    pub typ: Option<Expression>,
    pub name: Option<String>,
    pub body: Vec<Statement>,
    pub location: Location,
}

/// With item
#[derive(Debug, Clone)]
pub struct WithItem {
    pub context_expr: Expression,
    pub optional_vars: Option<Expression>,
}

/// Match case
#[derive(Debug, Clone)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub body: Vec<Statement>,
}

/// Pattern for match statement
#[derive(Debug, Clone)]
pub enum Pattern {
    MatchValue {
        value: Expression,
        location: Location,
    },
    MatchSingleton {
        value: Constant,
        location: Location,
    },
    MatchSequence {
        patterns: Vec<Pattern>,
        location: Location,
    },
    MatchMapping {
        keys: Vec<Expression>,
        patterns: Vec<Pattern>,
        rest: Option<String>,
        location: Location,
    },
    MatchClass {
        cls: Expression,
        patterns: Vec<Pattern>,
        kwd_attrs: Vec<String>,
        kwd_patterns: Vec<Pattern>,
        location: Location,
    },
    MatchStar {
        name: Option<String>,
        location: Location,
    },
    MatchAs {
        pattern: Option<Box<Pattern>>,
        name: Option<String>,
        location: Location,
    },
    MatchOr {
        patterns: Vec<Pattern>,
        location: Location,
    },
}

/// Import alias
#[derive(Debug, Clone)]
pub struct Alias {
    pub name: String,
    pub asname: Option<String>,
    pub location: Location,
}
