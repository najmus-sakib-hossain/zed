//! Error types for dx-generator.
//!
//! Provides comprehensive error handling for all generator operations
//! including template compilation, rendering, caching, and security.

use thiserror::Error;

/// Result type alias for generator operations.
pub type Result<T> = std::result::Result<T, GeneratorError>;

/// Comprehensive error type for dx-generator operations.
#[derive(Error, Debug)]
pub enum GeneratorError {
    // ========================================================================
    // Template Errors
    // ========================================================================
    /// Template file not found
    #[error("Template not found: {path}")]
    TemplateNotFound {
        /// Path to the missing template
        path: String,
    },

    /// Invalid template format
    #[error("Invalid template format: {reason}")]
    InvalidTemplate {
        /// Reason for invalidity
        reason: String,
    },

    /// Template magic number mismatch
    #[error("Invalid DXT magic number: expected {expected:?}, got {actual:?}")]
    InvalidMagic {
        /// Expected magic bytes
        expected: [u8; 4],
        /// Actual magic bytes found
        actual: [u8; 4],
    },

    /// Template version mismatch
    #[error("Unsupported DXT version: {version} (max supported: {max_supported})")]
    UnsupportedVersion {
        /// Version found in template
        version: u16,
        /// Maximum supported version
        max_supported: u16,
    },

    /// Template checksum mismatch
    #[error("Template checksum mismatch: file may be corrupted")]
    ChecksumMismatch,

    // ========================================================================
    // Parameter Errors
    // ========================================================================
    /// Required parameter missing
    #[error("Required parameter missing: {name}")]
    MissingParameter {
        /// Name of the missing parameter
        name: String,
    },

    /// Parameter type mismatch
    #[error("Parameter type mismatch for '{name}': expected {expected}, got {actual}")]
    ParameterTypeMismatch {
        /// Parameter name
        name: String,
        /// Expected type
        expected: String,
        /// Actual type provided
        actual: String,
    },

    /// Invalid parameter value
    #[error("Invalid parameter value for '{name}': {reason}")]
    InvalidParameter {
        /// Parameter name
        name: String,
        /// Reason for invalidity
        reason: String,
    },

    // ========================================================================
    // Rendering Errors
    // ========================================================================
    /// Render operation failed
    #[error("Render failed: {reason}")]
    RenderFailed {
        /// Reason for failure
        reason: String,
    },

    /// Output buffer overflow
    #[error("Output buffer overflow: {size} bytes exceeds maximum of {max_size}")]
    OutputOverflow {
        /// Attempted output size
        size: usize,
        /// Maximum allowed size
        max_size: usize,
    },

    /// Too many placeholders
    #[error("Too many placeholders: {count} exceeds maximum of {max_count}")]
    TooManyPlaceholders {
        /// Number of placeholders found
        count: usize,
        /// Maximum allowed
        max_count: usize,
    },

    /// Invalid bytecode instruction
    #[error("Invalid bytecode instruction at offset {offset}: opcode {opcode}")]
    InvalidBytecode {
        /// Offset in instruction stream
        offset: usize,
        /// Invalid opcode value
        opcode: u8,
    },

    // ========================================================================
    // Cache Errors
    // ========================================================================
    /// Cache entry not found
    #[error("Cache miss for template: {template_id}")]
    CacheMiss {
        /// Template identifier
        template_id: u32,
    },

    /// Cache is full
    #[error("Template cache full: {count} entries (max: {max_count})")]
    CacheFull {
        /// Current entry count
        count: usize,
        /// Maximum capacity
        max_count: usize,
    },

    // ========================================================================
    // Security Errors
    // ========================================================================
    /// Signature verification failed
    #[error("Template signature verification failed")]
    SignatureInvalid,

    /// Capability violation
    #[error("Capability violation: template lacks '{capability}' permission")]
    CapabilityViolation {
        /// Required capability
        capability: String,
    },

    /// Untrusted template
    #[error("Template is not signed and untrusted templates are disabled")]
    UntrustedTemplate,

    // ========================================================================
    // Session Errors
    // ========================================================================
    /// Session not found
    #[error("Session not found: {session_id}")]
    SessionNotFound {
        /// Session identifier
        session_id: String,
    },

    /// Session corrupted
    #[error("Session snapshot corrupted: {reason}")]
    SessionCorrupted {
        /// Reason for corruption
        reason: String,
    },

    // ========================================================================
    // I/O Errors
    // ========================================================================
    /// File I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Memory mapping failed
    #[error("Memory mapping failed: {reason}")]
    MmapFailed {
        /// Reason for failure
        reason: String,
    },

    // ========================================================================
    // Compilation Errors
    // ========================================================================
    /// Template syntax error
    #[error("Template syntax error at line {line}, column {column}: {message}")]
    SyntaxError {
        /// Line number (1-indexed)
        line: usize,
        /// Column number (1-indexed)
        column: usize,
        /// Error message
        message: String,
    },

    /// Invalid control flow
    #[error("Invalid control flow: {message}")]
    ControlFlowError {
        /// Error message
        message: String,
    },

    // ========================================================================
    // Fusion Errors
    // ========================================================================
    /// Fusion bundle error
    #[error("Fusion bundle error: {reason}")]
    FusionError {
        /// Reason for error
        reason: String,
    },

    /// Circular template dependency
    #[error("Circular template dependency detected: {cycle}")]
    CircularDependency {
        /// Description of the cycle
        cycle: String,
    },
}

impl GeneratorError {
    /// Create a template not found error.
    #[must_use]
    pub fn template_not_found(path: impl Into<String>) -> Self {
        Self::TemplateNotFound { path: path.into() }
    }

    /// Create an invalid template error.
    #[must_use]
    pub fn invalid_template(reason: impl Into<String>) -> Self {
        Self::InvalidTemplate {
            reason: reason.into(),
        }
    }

    /// Create a missing parameter error.
    #[must_use]
    pub fn missing_parameter(name: impl Into<String>) -> Self {
        Self::MissingParameter { name: name.into() }
    }

    /// Create a render failed error.
    #[must_use]
    pub fn render_failed(reason: impl Into<String>) -> Self {
        Self::RenderFailed {
            reason: reason.into(),
        }
    }

    /// Create a capability violation error.
    #[must_use]
    pub fn capability_violation(capability: impl Into<String>) -> Self {
        Self::CapabilityViolation {
            capability: capability.into(),
        }
    }

    /// Create a syntax error.
    #[must_use]
    pub fn syntax_error(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::SyntaxError {
            line,
            column,
            message: message.into(),
        }
    }

    /// Check if this error is recoverable.
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        matches!(self, Self::CacheMiss { .. } | Self::SessionNotFound { .. })
    }

    /// Check if this error is a security violation.
    #[must_use]
    pub fn is_security_error(&self) -> bool {
        matches!(
            self,
            Self::SignatureInvalid | Self::CapabilityViolation { .. } | Self::UntrustedTemplate
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = GeneratorError::template_not_found("test.dxt");
        assert!(err.to_string().contains("test.dxt"));

        let err = GeneratorError::missing_parameter("name");
        assert!(err.to_string().contains("name"));
    }

    #[test]
    fn test_is_recoverable() {
        let err = GeneratorError::CacheMiss { template_id: 1 };
        assert!(err.is_recoverable());

        let err = GeneratorError::SignatureInvalid;
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_is_security_error() {
        let err = GeneratorError::SignatureInvalid;
        assert!(err.is_security_error());

        let err = GeneratorError::capability_violation("can_create_files");
        assert!(err.is_security_error());

        let err = GeneratorError::CacheMiss { template_id: 1 };
        assert!(!err.is_security_error());
    }
}
