//! Transport layer for DCP protocol.
//!
//! Provides TCP, TLS, and other network transports for DCP servers.

pub mod framing;
pub mod tcp;

pub use framing::{FrameCodec, FrameError, FrameHeader, MAX_MESSAGE_SIZE, PROTOCOL_VERSION};
pub use tcp::{Connection, ProtocolMode, TcpConfig, TcpServer, TlsConfig, TlsVersion};
