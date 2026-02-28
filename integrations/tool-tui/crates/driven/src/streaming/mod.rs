//! Streaming Protocol - HTIP-Inspired Delivery
//!
//! Binary rule streaming with XOR delta patching for efficient sync.

mod chunk_streamer;
mod etag_negotiator;
mod htip_delivery;
mod xor_patcher;

pub use chunk_streamer::{ChunkStreamer, StreamChunk};
pub use etag_negotiator::{ETagNegotiator, NegotiationResult};
pub use htip_delivery::{HtipDelivery, RuleOperation};
pub use xor_patcher::{XorPatch, XorPatcher};

/// Streaming protocol version
pub const STREAM_VERSION: u8 = 1;

/// Maximum chunk size (64KB)
pub const MAX_CHUNK_SIZE: usize = 64 * 1024;
