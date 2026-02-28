//! DX-Py IPC - Binary Protocol IPC (HBTP for Python)
//!
//! High-performance binary protocol for inter-process communication
//! with zero-copy array transfer via shared memory.

pub mod channel;
pub mod protocol;
pub mod shared_memory;

pub use channel::HbtpChannel;
pub use protocol::{HbtpFlags, HbtpHeader, MessageType};
pub use shared_memory::{SharedArrayHandle, SharedMemoryArena};
