//! PEP 508 Environment Marker Evaluation
//!
//! Parses and evaluates environment markers for conditional dependencies.
//!
//! # Examples
//! ```
//! use dx_py_compat::markers::{MarkerEnvironment, MarkerEvaluator};
//!
//! let env = MarkerEnvironment::current();
//! let result = MarkerEvaluator::evaluate("python_version >= '3.8'", &env, &[]);
//! ```

use std::fmt;

/// Environment context for marker evaluation
#[derive(Debug, Clone)]
pub struct MarkerEnvironment {
    /// Python version (e.g., "3.12")
    pub python_version: String,
    /// Full Python version (e.g., "3.12.0")
    pub python_full_version: String,
    /// OS name (e.g., "posix", "nt", "java")
    pub os_name: String,
    /// sys.platform (e.g., "linux", "win32", "darwin")
    pub sys_platform: String,
    /// platform.system() (e.g., "Linux", "Windows", "Darwin")
    pub platform_system: String,
    /// platform.machine() (e.g., "x86_64", "aarch64")
    pub platform_machine: String,
    /// platform.release()
    pub platform_release: String,
    /// platform.version()
    pub platform_version: String,
    /// Implementation name (e.g., "cpython", "pypy")
    pub implementation_name: String,
    /// Implementation version
    pub implementation_version: String,
}

impl MarkerEnvironment {
    /// Detect current environment
    pub fn current() -> Self {
        Self {
            python_version: "3.12".to_string(),
            python_full_version: "3.12.0".to_string(),
            os_name: Self::detect_os_name(),
            sys_platform: Self::detect_sys_platform(),
            platform_system: Self::detect_platform_system(),
            platform_machine: Self::detect_platform_machine(),
            platform_release: String::new(),
            platform_version: String::new(),
            implementation_name: "cpython".to_string(),
            implementation_version: "3.12.0".to_string(),
        }
    }

    /// Create with specific Python version
    pub fn with_python(mut self, version: &str) -> Self {
        self.python_version = version.to_string();
        if !version.contains('.') || version.matches('.').count() == 1 {
            self.python_full_version = format!("{}.0", version);
        } else {
            self.python_full_version = version.to_string();
        }
        self.implementation_version = self.python_full_version.clone();
        self
    }

    fn detect_os_name() -> String {
        #[cfg(unix)]
        return "posix".to_string();
        #[cfg(windows)]
        return "nt".to_string();
        #[cfg(not(any(unix, windows)))]
        return "unknown".to_string();
    }

    fn detect_sys_platform() -> String {
        #[cfg(target_os = "linux")]
        return "linux".to_string();
        #[cfg(target_os = "windows")]
        return "win32".to_string();
        #[cfg(target_os = "macos")]
        return "darwin".to_string();
        #[cfg(target_os = "freebsd")]
        return "freebsd".to_string();
        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
            target_os = "freebsd"
        )))]
        return "unknown".to_string();
    }

    fn detect_platform_system() -> String {
        #[cfg(target_os = "linux")]
        return "Linux".to_string();
        #[cfg(target_os = "windows")]
        return "Windows".to_string();
        #[cfg(target_os = "macos")]
        return "Darwin".to_string();
        #[cfg(target_os = "freebsd")]
        return "FreeBSD".to_string();
        #[cfg(not(any(
            target_os = "linux",
            target_os = "windows",
            target_os = "macos",
            target_os = "freebsd"
        )))]
        return "Unknown".to_string();
    }

    fn detect_platform_machine() -> String {
        #[cfg(target_arch = "x86_64")]
        return "x86_64".to_string();
        #[cfg(target_arch = "x86")]
        return "i686".to_string();
        #[cfg(target_arch = "aarch64")]
        return "aarch64".to_string();
        #[cfg(target_arch = "arm")]
        return "armv7l".to_string();
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        return "unknown".to_string();
    }

    /// Get value for a marker variable
    pub fn get(&self, var: &MarkerVar) -> String {
        match var {
            MarkerVar::PythonVersion => self.python_version.clone(),
            MarkerVar::PythonFullVersion => self.python_full_version.clone(),
            MarkerVar::OsName => self.os_name.clone(),
            MarkerVar::SysPlatform => self.sys_platform.clone(),
            MarkerVar::PlatformSystem => self.platform_system.clone(),
            MarkerVar::PlatformMachine => self.platform_machine.clone(),
            MarkerVar::PlatformRelease => self.platform_release.clone(),
            MarkerVar::PlatformVersion => self.platform_version.clone(),
            MarkerVar::ImplementationName => self.implementation_name.clone(),
            MarkerVar::ImplementationVersion => self.implementation_version.clone(),
            MarkerVar::Extra => String::new(), // Handled separately
        }
    }
}

impl Default for MarkerEnvironment {
    fn default() -> Self {
        Self::current()
    }
}

/// Marker variable names
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerVar {
    PythonVersion,
    PythonFullVersion,
    OsName,
    SysPlatform,
    PlatformSystem,
    PlatformMachine,
    PlatformRelease,
    PlatformVersion,
    ImplementationName,
    ImplementationVersion,
    Extra,
}

impl MarkerVar {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "python_version" => Some(MarkerVar::PythonVersion),
            "python_full_version" => Some(MarkerVar::PythonFullVersion),
            "os_name" | "os.name" => Some(MarkerVar::OsName),
            "sys_platform" | "sys.platform" => Some(MarkerVar::SysPlatform),
            "platform_system" | "platform.system" => Some(MarkerVar::PlatformSystem),
            "platform_machine" | "platform.machine" => Some(MarkerVar::PlatformMachine),
            "platform_release" | "platform.release" => Some(MarkerVar::PlatformRelease),
            "platform_version" | "platform.version" => Some(MarkerVar::PlatformVersion),
            "implementation_name" => Some(MarkerVar::ImplementationName),
            "implementation_version" => Some(MarkerVar::ImplementationVersion),
            "extra" => Some(MarkerVar::Extra),
            _ => None,
        }
    }
}

impl fmt::Display for MarkerVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarkerVar::PythonVersion => write!(f, "python_version"),
            MarkerVar::PythonFullVersion => write!(f, "python_full_version"),
            MarkerVar::OsName => write!(f, "os_name"),
            MarkerVar::SysPlatform => write!(f, "sys_platform"),
            MarkerVar::PlatformSystem => write!(f, "platform_system"),
            MarkerVar::PlatformMachine => write!(f, "platform_machine"),
            MarkerVar::PlatformRelease => write!(f, "platform_release"),
            MarkerVar::PlatformVersion => write!(f, "platform_version"),
            MarkerVar::ImplementationName => write!(f, "implementation_name"),
            MarkerVar::ImplementationVersion => write!(f, "implementation_version"),
            MarkerVar::Extra => write!(f, "extra"),
        }
    }
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Equal,        // ==
    NotEqual,     // !=
    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=
    Compatible,   // ~=
    Arbitrary,    // ===
    In,           // in
    NotIn,        // not in
}

impl CompareOp {
    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "==" => Some(CompareOp::Equal),
            "!=" => Some(CompareOp::NotEqual),
            "<" => Some(CompareOp::Less),
            "<=" => Some(CompareOp::LessEqual),
            ">" => Some(CompareOp::Greater),
            ">=" => Some(CompareOp::GreaterEqual),
            "~=" => Some(CompareOp::Compatible),
            "===" => Some(CompareOp::Arbitrary),
            "in" => Some(CompareOp::In),
            "not in" => Some(CompareOp::NotIn),
            _ => None,
        }
    }
}

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompareOp::Equal => write!(f, "=="),
            CompareOp::NotEqual => write!(f, "!="),
            CompareOp::Less => write!(f, "<"),
            CompareOp::LessEqual => write!(f, "<="),
            CompareOp::Greater => write!(f, ">"),
            CompareOp::GreaterEqual => write!(f, ">="),
            CompareOp::Compatible => write!(f, "~="),
            CompareOp::Arbitrary => write!(f, "==="),
            CompareOp::In => write!(f, "in"),
            CompareOp::NotIn => write!(f, "not in"),
        }
    }
}

/// Marker value (string literal or variable)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerValue {
    String(String),
    Variable(MarkerVar),
}

impl fmt::Display for MarkerValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarkerValue::String(s) => write!(f, "'{}'", s),
            MarkerValue::Variable(v) => write!(f, "{}", v),
        }
    }
}

/// Marker expression AST
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerExpr {
    /// Comparison: left op right
    Compare {
        left: MarkerValue,
        op: CompareOp,
        right: MarkerValue,
    },
    /// Boolean AND
    And(Box<MarkerExpr>, Box<MarkerExpr>),
    /// Boolean OR
    Or(Box<MarkerExpr>, Box<MarkerExpr>),
    /// Boolean NOT (rarely used)
    Not(Box<MarkerExpr>),
    /// Always true
    True,
    /// Always false
    False,
}

impl fmt::Display for MarkerExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarkerExpr::Compare { left, op, right } => {
                write!(f, "{} {} {}", left, op, right)
            }
            MarkerExpr::And(a, b) => write!(f, "({} and {})", a, b),
            MarkerExpr::Or(a, b) => write!(f, "({} or {})", a, b),
            MarkerExpr::Not(e) => write!(f, "(not {})", e),
            MarkerExpr::True => write!(f, "true"),
            MarkerExpr::False => write!(f, "false"),
        }
    }
}

/// Marker parse error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerParseError {
    Empty,
    InvalidVariable(String),
    InvalidOperator(String),
    InvalidSyntax(String),
    UnmatchedParenthesis,
    UnterminatedString,
}

impl fmt::Display for MarkerParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarkerParseError::Empty => write!(f, "empty marker expression"),
            MarkerParseError::InvalidVariable(s) => write!(f, "invalid variable: {}", s),
            MarkerParseError::InvalidOperator(s) => write!(f, "invalid operator: {}", s),
            MarkerParseError::InvalidSyntax(s) => write!(f, "invalid syntax: {}", s),
            MarkerParseError::UnmatchedParenthesis => write!(f, "unmatched parenthesis"),
            MarkerParseError::UnterminatedString => write!(f, "unterminated string"),
        }
    }
}

impl std::error::Error for MarkerParseError {}

/// Marker expression evaluator
pub struct MarkerEvaluator;

impl MarkerEvaluator {
    /// Parse a marker expression string
    pub fn parse(s: &str) -> Result<MarkerExpr, MarkerParseError> {
        let s = s.trim();
        if s.is_empty() {
            return Err(MarkerParseError::Empty);
        }

        Parser::new(s).parse_expr()
    }

    /// Evaluate a marker expression string
    pub fn evaluate(marker: &str, env: &MarkerEnvironment, extras: &[String]) -> bool {
        match Self::parse(marker) {
            Ok(expr) => Self::eval_expr(&expr, env, extras),
            Err(_) => false,
        }
    }

    /// Evaluate a parsed marker expression
    pub fn eval_expr(expr: &MarkerExpr, env: &MarkerEnvironment, extras: &[String]) -> bool {
        match expr {
            MarkerExpr::Compare { left, op, right } => {
                // Special handling for extra comparisons
                if Self::is_extra_comparison(left, right) {
                    return Self::eval_extra_comparison(left, op, right, extras);
                }
                let left_val = Self::resolve_value(left, env);
                let right_val = Self::resolve_value(right, env);
                Self::compare(&left_val, *op, &right_val)
            }
            MarkerExpr::And(a, b) => {
                Self::eval_expr(a, env, extras) && Self::eval_expr(b, env, extras)
            }
            MarkerExpr::Or(a, b) => {
                Self::eval_expr(a, env, extras) || Self::eval_expr(b, env, extras)
            }
            MarkerExpr::Not(e) => !Self::eval_expr(e, env, extras),
            MarkerExpr::True => true,
            MarkerExpr::False => false,
        }
    }

    /// Check if this is an extra comparison
    fn is_extra_comparison(left: &MarkerValue, right: &MarkerValue) -> bool {
        matches!(left, MarkerValue::Variable(MarkerVar::Extra))
            || matches!(right, MarkerValue::Variable(MarkerVar::Extra))
    }

    /// Evaluate an extra comparison
    fn eval_extra_comparison(
        left: &MarkerValue,
        op: &CompareOp,
        right: &MarkerValue,
        extras: &[String],
    ) -> bool {
        // Get the extra name from the string side
        let extra_name = match (left, right) {
            (MarkerValue::Variable(MarkerVar::Extra), MarkerValue::String(s)) => s,
            (MarkerValue::String(s), MarkerValue::Variable(MarkerVar::Extra)) => s,
            _ => return false,
        };

        let has_extra = Self::check_extra(extras, extra_name);

        match op {
            CompareOp::Equal => has_extra,
            CompareOp::NotEqual => !has_extra,
            _ => false, // Other operators don't make sense for extras
        }
    }

    /// Resolve a marker value to a string
    fn resolve_value(value: &MarkerValue, env: &MarkerEnvironment) -> String {
        match value {
            MarkerValue::String(s) => s.clone(),
            MarkerValue::Variable(var) => env.get(var),
        }
    }

    /// Compare two values with an operator
    fn compare(left: &str, op: CompareOp, right: &str) -> bool {
        match op {
            CompareOp::Equal => left == right,
            CompareOp::NotEqual => left != right,
            CompareOp::Less => compare_versions(left, right) == std::cmp::Ordering::Less,
            CompareOp::LessEqual => compare_versions(left, right) != std::cmp::Ordering::Greater,
            CompareOp::Greater => compare_versions(left, right) == std::cmp::Ordering::Greater,
            CompareOp::GreaterEqual => compare_versions(left, right) != std::cmp::Ordering::Less,
            CompareOp::Compatible => is_compatible(left, right),
            CompareOp::Arbitrary => left == right,
            CompareOp::In => right.contains(left),
            CompareOp::NotIn => !right.contains(left),
        }
    }

    /// Check if an extra matches
    fn check_extra(extras: &[String], expected: &str) -> bool {
        extras.iter().any(|e| e.eq_ignore_ascii_case(expected))
    }
}

/// Compare version strings
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts: Vec<u32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for (ap, bp) in a_parts.iter().zip(b_parts.iter()) {
        match ap.cmp(bp) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

/// Check version compatibility (~=)
fn is_compatible(version: &str, constraint: &str) -> bool {
    let v_parts: Vec<&str> = version.split('.').collect();
    let c_parts: Vec<&str> = constraint.split('.').collect();

    if c_parts.is_empty() {
        return false;
    }

    // Must match all but last segment of constraint
    let prefix_len = c_parts.len().saturating_sub(1).max(1);

    for i in 0..prefix_len {
        if v_parts.get(i) != c_parts.get(i) {
            return false;
        }
    }

    // Version must be >= constraint
    compare_versions(version, constraint) != std::cmp::Ordering::Less
}

/// Simple recursive descent parser for marker expressions
struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_expr(&mut self) -> Result<MarkerExpr, MarkerParseError> {
        self.skip_whitespace();
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<MarkerExpr, MarkerParseError> {
        let mut left = self.parse_and()?;

        loop {
            self.skip_whitespace();
            if self.consume_keyword("or") {
                let right = self.parse_and()?;
                left = MarkerExpr::Or(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<MarkerExpr, MarkerParseError> {
        let mut left = self.parse_not()?;

        loop {
            self.skip_whitespace();
            if self.consume_keyword("and") {
                let right = self.parse_not()?;
                left = MarkerExpr::And(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<MarkerExpr, MarkerParseError> {
        self.skip_whitespace();
        if self.consume_keyword("not") {
            let expr = self.parse_primary()?;
            Ok(MarkerExpr::Not(Box::new(expr)))
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<MarkerExpr, MarkerParseError> {
        self.skip_whitespace();

        // Check for parenthesized expression
        if self.peek() == Some('(') {
            self.advance();
            let expr = self.parse_or()?;
            self.skip_whitespace();
            if self.peek() != Some(')') {
                return Err(MarkerParseError::UnmatchedParenthesis);
            }
            self.advance();
            return Ok(expr);
        }

        // Parse comparison
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<MarkerExpr, MarkerParseError> {
        self.skip_whitespace();
        let left = self.parse_value()?;

        self.skip_whitespace();
        let op = self.parse_operator()?;

        self.skip_whitespace();
        let right = self.parse_value()?;

        Ok(MarkerExpr::Compare { left, op, right })
    }

    fn parse_value(&mut self) -> Result<MarkerValue, MarkerParseError> {
        self.skip_whitespace();

        // Check for string literal
        if let Some(c) = self.peek() {
            if c == '\'' || c == '"' {
                return self.parse_string();
            }
        }

        // Parse variable
        self.parse_variable()
    }

    fn parse_string(&mut self) -> Result<MarkerValue, MarkerParseError> {
        let quote = self.peek().ok_or(MarkerParseError::UnterminatedString)?;
        self.advance();

        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == quote {
                let s = self.input[start..self.pos].to_string();
                self.advance();
                return Ok(MarkerValue::String(s));
            }
            self.advance();
        }

        Err(MarkerParseError::UnterminatedString)
    }

    fn parse_variable(&mut self) -> Result<MarkerValue, MarkerParseError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                self.advance();
            } else {
                break;
            }
        }

        let name = &self.input[start..self.pos];
        if name.is_empty() {
            return Err(MarkerParseError::InvalidSyntax("expected variable".to_string()));
        }

        MarkerVar::parse(name)
            .map(MarkerValue::Variable)
            .ok_or_else(|| MarkerParseError::InvalidVariable(name.to_string()))
    }

    fn parse_operator(&mut self) -> Result<CompareOp, MarkerParseError> {
        self.skip_whitespace();

        // Check for multi-character operators first
        let remaining = &self.input[self.pos..];

        // "not in"
        if let Some(after_not) = remaining.strip_prefix("not") {
            let after_not = after_not.trim_start();
            if after_not.starts_with("in") {
                self.pos += 3;
                self.skip_whitespace();
                self.pos += 2;
                return Ok(CompareOp::NotIn);
            }
        }

        // "in"
        if remaining.starts_with("in") && !remaining[2..].starts_with(|c: char| c.is_alphanumeric())
        {
            self.pos += 2;
            return Ok(CompareOp::In);
        }

        // Symbol operators
        let ops = ["===", "~=", "==", "!=", "<=", ">=", "<", ">"];
        for op_str in ops {
            if remaining.starts_with(op_str) {
                self.pos += op_str.len();
                return CompareOp::parse(op_str)
                    .ok_or_else(|| MarkerParseError::InvalidOperator(op_str.to_string()));
            }
        }

        Err(MarkerParseError::InvalidOperator(remaining.chars().take(5).collect()))
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        let remaining = &self.input[self.pos..];
        if let Some(after) = remaining.strip_prefix(keyword) {
            // Make sure it's not part of a larger word
            if after.is_empty() || !after.chars().next().unwrap().is_alphanumeric() {
                self.pos += keyword.len();
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_comparison() {
        let expr = MarkerEvaluator::parse("python_version >= '3.8'").unwrap();
        match expr {
            MarkerExpr::Compare { left, op, right } => {
                assert_eq!(left, MarkerValue::Variable(MarkerVar::PythonVersion));
                assert_eq!(op, CompareOp::GreaterEqual);
                assert_eq!(right, MarkerValue::String("3.8".to_string()));
            }
            _ => panic!("Expected Compare"),
        }
    }

    #[test]
    fn test_parse_and_expression() {
        let expr =
            MarkerEvaluator::parse("python_version >= '3.8' and sys_platform == 'linux'").unwrap();
        match expr {
            MarkerExpr::And(_, _) => {}
            _ => panic!("Expected And"),
        }
    }

    #[test]
    fn test_parse_or_expression() {
        let expr =
            MarkerEvaluator::parse("sys_platform == 'win32' or sys_platform == 'linux'").unwrap();
        match expr {
            MarkerExpr::Or(_, _) => {}
            _ => panic!("Expected Or"),
        }
    }

    #[test]
    fn test_parse_parentheses() {
        let expr = MarkerEvaluator::parse("(python_version >= '3.8')").unwrap();
        match expr {
            MarkerExpr::Compare { .. } => {}
            _ => panic!("Expected Compare"),
        }
    }

    #[test]
    fn test_evaluate_python_version() {
        let env = MarkerEnvironment::current().with_python("3.12");

        assert!(MarkerEvaluator::evaluate("python_version >= '3.8'", &env, &[]));
        assert!(MarkerEvaluator::evaluate("python_version >= '3.12'", &env, &[]));
        assert!(!MarkerEvaluator::evaluate("python_version >= '3.13'", &env, &[]));
        assert!(MarkerEvaluator::evaluate("python_version < '4.0'", &env, &[]));
    }

    #[test]
    fn test_evaluate_sys_platform() {
        let env = MarkerEnvironment::current();

        #[cfg(target_os = "windows")]
        {
            assert!(MarkerEvaluator::evaluate("sys_platform == 'win32'", &env, &[]));
            assert!(!MarkerEvaluator::evaluate("sys_platform == 'linux'", &env, &[]));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(MarkerEvaluator::evaluate("sys_platform == 'linux'", &env, &[]));
            assert!(!MarkerEvaluator::evaluate("sys_platform == 'win32'", &env, &[]));
        }

        #[cfg(target_os = "macos")]
        {
            assert!(MarkerEvaluator::evaluate("sys_platform == 'darwin'", &env, &[]));
            assert!(!MarkerEvaluator::evaluate("sys_platform == 'win32'", &env, &[]));
        }
    }

    #[test]
    fn test_evaluate_complex() {
        let env = MarkerEnvironment::current().with_python("3.12");

        // Complex expression
        let marker = "python_version >= '3.8' and (sys_platform == 'win32' or sys_platform == 'linux' or sys_platform == 'darwin')";
        assert!(MarkerEvaluator::evaluate(marker, &env, &[]));
    }

    #[test]
    fn test_evaluate_extra() {
        let env = MarkerEnvironment::current();

        // Without extras
        assert!(!MarkerEvaluator::evaluate("extra == 'dev'", &env, &[]));

        // With extras
        assert!(MarkerEvaluator::evaluate("extra == 'dev'", &env, &["dev".to_string()]));
    }

    #[test]
    fn test_evaluate_in_operator() {
        let _env = MarkerEnvironment::current();

        #[cfg(target_os = "linux")]
        {
            let env = MarkerEnvironment::current();
            assert!(MarkerEvaluator::evaluate("'linux' in sys_platform", &env, &[]));
        }
    }

    #[test]
    fn test_compatible_operator() {
        assert!(is_compatible("3.12", "3.8"));
        assert!(is_compatible("3.12.1", "3.12"));
        assert!(!is_compatible("3.7", "3.8"));
        assert!(!is_compatible("4.0", "3.8"));
    }

    #[test]
    fn test_marker_environment_detection() {
        let env = MarkerEnvironment::current();

        // Should have reasonable defaults
        assert!(!env.python_version.is_empty());
        assert!(!env.sys_platform.is_empty());
        assert!(!env.platform_system.is_empty());
    }
}
