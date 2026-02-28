//! PEP 508 environment marker evaluation
//!
//! Provides functionality for parsing and evaluating PEP 508 environment markers.

mod cache;
mod evaluator;
mod parser;

pub use cache::MarkerCache;
pub use evaluator::{MarkerEnvironment, MarkerEvaluator};
pub use parser::{MarkerExpr, MarkerOp, MarkerParser, MarkerValue};

/// Marker evaluation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum MarkerError {
    /// Invalid marker syntax
    #[error("Invalid marker syntax at position {position}: {message}")]
    ParseError { position: usize, message: String },
    /// Unknown marker variable
    #[error("Unknown marker variable: {0}")]
    UnknownVariable(String),
    /// Invalid operator for marker comparison
    #[error("Invalid operator for marker comparison")]
    InvalidOperator,
}
