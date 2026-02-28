//! Binary protocol for zero-copy IPC between Rust orchestrator and Python workers
//!
//! This crate defines the binary message formats and shared memory communication
//! for high-performance test execution.

mod messages;
mod ring_buffer;

pub use messages::*;
pub use ring_buffer::*;

#[cfg(test)]
mod tests;
