//! Capability manifest for DCP protocol.
//!
//! Provides pre-computed capability negotiation using bitsets for O(1) operations.

pub mod manifest;

pub use manifest::CapabilityManifest;
