//! Protocol layer for DCP.

pub mod parser;
pub mod schema;

pub use parser::MessageParser;
pub use schema::{FieldDef, InputSchema, ToolSchema};
