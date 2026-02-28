//! Compile-time pre-serialization for static data.
//!
//! This module provides macros and utilities for serializing data at compile time,
//! eliminating runtime serialization overhead for static configuration and data.
//!
//! # Features
//!
//! - **Zero Runtime Cost**: Data is serialized during compilation
//! - **Type Safety**: Full type checking at compile time
//! - **RKYV Compatible**: Generates standard RKYV format
//! - **Const Evaluation**: Works with const contexts
//!
//! # Examples
//!
//! ```rust,ignore
//! use dx_serializer::machine::static_ser::*;
//!
//! #[dx_static_serialize]
//! const CONFIG: &[u8] = {
//!     AppConfig {
//!         host: "localhost",
//!         port: 8080,
//!     }
//! };
//!
//! // Zero runtime cost - data is already serialized
//! let config: &ArchivedAppConfig = unsafe {
//!     rkyv::archived_root::<AppConfig>(CONFIG)
//! };
//! ```

pub mod macros;

// Re-export macros for convenience
pub use macros::*;

/// Marker trait for types that can be statically serialized.
///
/// This trait is automatically implemented for types that implement
/// `rkyv::Archive` and `rkyv::Serialize`.
pub trait StaticSerialize: rkyv::Archive {}

impl<T> StaticSerialize for T where T: rkyv::Archive {}

/// Result type for static serialization operations.
pub type StaticResult<T> = Result<T, StaticError>;

/// Errors that can occur during static serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticError {
    /// Serialization failed during const evaluation
    SerializationFailed,
    /// Buffer size exceeded during const evaluation
    BufferOverflow,
    /// Invalid data format
    InvalidFormat,
}

impl std::fmt::Display for StaticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StaticError::SerializationFailed => write!(f, "Static serialization failed"),
            StaticError::BufferOverflow => write!(f, "Buffer overflow during static serialization"),
            StaticError::InvalidFormat => write!(f, "Invalid static serialization format"),
        }
    }
}

impl std::error::Error for StaticError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_error_display() {
        assert_eq!(StaticError::SerializationFailed.to_string(), "Static serialization failed");
        assert_eq!(
            StaticError::BufferOverflow.to_string(),
            "Buffer overflow during static serialization"
        );
        assert_eq!(StaticError::InvalidFormat.to_string(), "Invalid static serialization format");
    }

    #[test]
    fn test_static_error_equality() {
        assert_eq!(StaticError::SerializationFailed, StaticError::SerializationFailed);
        assert_ne!(StaticError::SerializationFailed, StaticError::BufferOverflow);
    }
}
