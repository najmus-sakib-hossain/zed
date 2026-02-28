//! Streaming support for DCP protocol.

pub mod dcp_stream;
pub mod ring_buffer;

pub use dcp_stream::DcpStream;
pub use ring_buffer::{Backpressure, StreamRingBuffer};
