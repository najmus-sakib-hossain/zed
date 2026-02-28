//! DX-Py Stack - Stack Allocation Fast Path
//!
//! This crate implements escape analysis and stack allocation for
//! non-escaping objects, providing significant performance improvements.

pub mod escape_analysis;
pub mod stack_list;
pub mod stack_tuple;
pub mod tagged_value;

pub use escape_analysis::EscapeAnalyzer;
pub use stack_list::StackList;
pub use stack_tuple::StackTuple;
pub use tagged_value::TaggedValue;
