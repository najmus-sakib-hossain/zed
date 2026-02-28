//! Pattern definitions for SIMD scanning

// Common JavaScript/TypeScript patterns
#[allow(dead_code)]
/// Import statement
pub const IMPORT: &[u8] = b"import ";
/// Export statement
pub const EXPORT: &[u8] = b"export ";
/// Export default
pub const EXPORT_DEFAULT: &[u8] = b"export default ";
/// Interface declaration
pub const INTERFACE: &[u8] = b"interface ";
/// Type declaration
pub const TYPE: &[u8] = b"type ";
/// Require call
pub const REQUIRE: &[u8] = b"require(";
/// Async keyword
pub const ASYNC: &[u8] = b"async ";
/// Await keyword
pub const AWAIT: &[u8] = b"await ";
/// Function keyword
pub const FUNCTION: &[u8] = b"function ";
/// Const declaration
pub const CONST: &[u8] = b"const ";
/// Let declaration
pub const LET: &[u8] = b"let ";
/// Var declaration
pub const VAR: &[u8] = b"var ";
/// Class declaration
pub const CLASS: &[u8] = b"class ";
/// Enum declaration
pub const ENUM: &[u8] = b"enum ";

/// JSX-specific patterns
pub mod jsx {
    /// Fragment short syntax
    pub const FRAGMENT: &[u8] = b"<>";
    /// Fragment close
    pub const FRAGMENT_CLOSE: &[u8] = b"</>";
}

/// TypeScript-specific patterns for stripping
pub mod typescript {
    /// Type assertion (as keyword)
    pub const AS: &[u8] = b" as ";
    /// Non-null assertion
    pub const NON_NULL: &[u8] = b"!.";
    /// Optional chaining with type
    pub const OPTIONAL: &[u8] = b"?.";
    /// Readonly modifier
    pub const READONLY: &[u8] = b"readonly ";
    /// Private modifier
    pub const PRIVATE: &[u8] = b"private ";
    /// Protected modifier
    pub const PROTECTED: &[u8] = b"protected ";
    /// Public modifier
    pub const PUBLIC: &[u8] = b"public ";
    /// Abstract modifier
    pub const ABSTRACT: &[u8] = b"abstract ";
    /// Implements keyword
    pub const IMPLEMENTS: &[u8] = b" implements ";
    /// Extends keyword
    pub const EXTENDS: &[u8] = b" extends ";
    /// Declare keyword
    pub const DECLARE: &[u8] = b"declare ";
}
