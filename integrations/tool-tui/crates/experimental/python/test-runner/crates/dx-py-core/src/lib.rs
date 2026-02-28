//! Core types for dx-py-test-runner
//!
//! This crate defines the fundamental data structures used throughout
//! the test runner: TestCase, TestResult, TestId, and related types.

mod assertion;
mod errors;
mod types;

pub use assertion::*;
pub use errors::*;
pub use types::*;

#[cfg(test)]
mod tests;
