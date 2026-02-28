//! Error types for DCP protocol.

use thiserror::Error;

/// DCP error types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[repr(u8)]
pub enum DCPError {
    /// Insufficient data for parsing
    #[error("insufficient data for parsing")]
    InsufficientData = 1,
    /// Invalid magic number
    #[error("invalid magic number")]
    InvalidMagic = 2,
    /// Unknown message type
    #[error("unknown message type")]
    UnknownMessageType = 3,
    /// Tool not found
    #[error("tool not found")]
    ToolNotFound = 4,
    /// Schema validation failed
    #[error("schema validation failed")]
    ValidationFailed = 5,
    /// Hash mismatch in delta sync
    #[error("hash mismatch")]
    HashMismatch = 6,
    /// Signature verification failed
    #[error("signature invalid")]
    SignatureInvalid = 7,
    /// Nonce reused (replay attack)
    #[error("nonce reused")]
    NonceReused = 8,
    /// Timestamp expired
    #[error("timestamp expired")]
    TimestampExpired = 9,
    /// Checksum mismatch
    #[error("checksum mismatch")]
    ChecksumMismatch = 10,
    /// Backpressure - consumer too slow
    #[error("backpressure")]
    Backpressure = 11,
    /// Memory bounds violation
    #[error("out of bounds")]
    OutOfBounds = 12,
    /// Internal error
    #[error("internal error")]
    InternalError = 13,
    /// Resource exhausted
    #[error("resource exhausted")]
    ResourceExhausted = 14,
    /// Session not found
    #[error("session not found")]
    SessionNotFound = 15,
}

/// Security-specific errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[repr(u8)]
pub enum SecurityError {
    #[error("invalid signature")]
    InvalidSignature = 1,
    #[error("expired timestamp")]
    ExpiredTimestamp = 2,
    #[error("replay attack detected")]
    ReplayAttack = 3,
    #[error("insufficient capabilities")]
    InsufficientCapabilities = 4,
}

/// Binary error response
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ErrorResponse {
    /// Error category (1 byte)
    pub category: u8,
    /// Error code within category (1 byte)
    pub code: u8,
    /// Additional context length (2 bytes)
    pub context_len: u16,
}

impl ErrorResponse {
    pub const SIZE: usize = 4;

    pub fn new(category: u8, code: u8) -> Self {
        Self {
            category,
            code,
            context_len: 0,
        }
    }

    pub fn from_dcp_error(err: DCPError) -> Self {
        Self::new(1, err as u8)
    }

    pub fn from_security_error(err: SecurityError) -> Self {
        Self::new(2, err as u8)
    }
}
