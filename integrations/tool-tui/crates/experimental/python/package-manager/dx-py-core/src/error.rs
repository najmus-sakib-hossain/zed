//! Error types for DX-Py operations

use thiserror::Error;

/// Main error type for DX-Py operations
#[derive(Error, Debug)]
pub enum Error {
    // Format errors
    #[error("Invalid magic number: expected {expected:?}, found {found:?}")]
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },

    #[error("Corrupted data: integrity check failed")]
    IntegrityError,

    #[error("Unsupported format version: {0}")]
    UnsupportedVersion(u16),

    #[error("Invalid section offset: {section} at {offset}")]
    InvalidOffset { section: &'static str, offset: u32 },

    #[error("Package size exceeds limit: {size} > {limit}")]
    PackageTooLarge { size: u64, limit: u64 },

    #[error("File count exceeds limit: {count} > {limit}")]
    TooManyFiles { count: u32, limit: u32 },

    // Resolution errors
    #[error("No matching version found for {package} with constraint {constraint}")]
    NoMatchingVersion { package: String, constraint: String },

    #[error("Dependency conflict: {0}")]
    DependencyConflict(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Resolution error: {0}")]
    Resolution(String),

    // I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    // Python errors
    #[error("Python version not found: {0}")]
    PythonNotFound(String),

    #[error("Virtual environment error: {0}")]
    VenvError(String),

    // Package errors
    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Invalid package name: {0}")]
    InvalidPackageName(String),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    // Network errors
    #[error("Network error: {0}")]
    Network(String),

    // Extras errors
    #[error("Invalid extra: {0}")]
    InvalidExtra(String),

    // Pre-release errors
    #[error("Pre-release version not allowed: {0}")]
    PreReleaseNotAllowed(String),

    // Environment marker errors
    #[error("Invalid environment marker: {0}")]
    InvalidMarker(String),
}

/// Result type alias for DX-Py operations
pub type Result<T> = std::result::Result<T, Error>;
