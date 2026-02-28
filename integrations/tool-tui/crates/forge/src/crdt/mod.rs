//! CRDT (Conflict-free Replicated Data Types) module
//!
//! This module provides CRDT-based document editing capabilities using automerge.
//! It is only available when the "crdt" feature is enabled.

pub mod anchor;
#[cfg(feature = "crdt")]
pub mod document;
pub mod operations;

pub use anchor::Anchor;
#[cfg(feature = "crdt")]
pub use document::CrdtDocument;
pub use operations::{Operation, OperationType, Position};
