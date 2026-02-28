//! Tree-sitter Python parser wrapper

use dx_py_core::DiscoveryError;
use tree_sitter::{Parser, Tree};

/// Wrapper around tree-sitter Python parser
pub struct PythonParser {
    parser: Parser,
}

impl PythonParser {
    /// Create a new Python parser
    pub fn new() -> Result<Self, DiscoveryError> {
        let mut parser = Parser::new();
        let language = tree_sitter_python::language();
        parser
            .set_language(language)
            .map_err(|e| DiscoveryError::ParseError(e.to_string()))?;
        Ok(Self { parser })
    }

    /// Parse Python source code into an AST
    pub fn parse(&mut self, source: &str) -> Result<Tree, DiscoveryError> {
        self.parser
            .parse(source, None)
            .ok_or_else(|| DiscoveryError::ParseError("Failed to parse source".into()))
    }
}

impl Default for PythonParser {
    fn default() -> Self {
        Self::new().expect("Failed to create Python parser")
    }
}
