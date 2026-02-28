//! Connection multiplexing for DCP protocol.
//!
//! Provides multiplexed connections supporting concurrent streams over a single
//! TCP connection, with stream ID routing and error isolation.

mod connection;
mod header;
mod pipeline;

pub use connection::{
    MultiplexError, MultiplexedConnection, StreamState, StreamStatus, MAX_STREAMS,
};
pub use header::{StreamFlags, StreamHeader, STREAM_HEADER_SIZE};
pub use pipeline::{PipelinedClient, RequestPipeline};
