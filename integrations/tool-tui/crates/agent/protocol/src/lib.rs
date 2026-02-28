//! # DX Agent Protocol
//!
//! Binary protocol definition for DX Agent gateway communication.
//! Supports WebSocket message framing, authentication flows,
//! presence/typing indicators, and session state sync.

pub mod auth;
pub mod framing;
pub mod messages;

pub use auth::{AuthChallenge, AuthRequest, AuthResponse, AuthToken};
pub use framing::{Frame, FrameType};
pub use messages::{
    ChannelEvent, GatewayEvent, GatewayMessage, GatewayRequest, GatewayResponse, PresenceUpdate,
    SessionSync, TypingIndicator,
};

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;

/// Default gateway port
pub const DEFAULT_PORT: u16 = 31337;

/// Maximum message size (1MB)
pub const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// Heartbeat interval in seconds
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;
