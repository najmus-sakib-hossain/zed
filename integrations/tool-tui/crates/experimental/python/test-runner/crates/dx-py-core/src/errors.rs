//! Error types for dx-py-test-runner

use thiserror::Error;

/// Errors that can occur during test discovery
#[derive(Error, Debug)]
pub enum DiscoveryError {
    #[error("Failed to parse Python file: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Index file corrupted: {0}")]
    IndexCorrupted(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Errors that can occur in the daemon pool
#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Worker crashed: {0}")]
    WorkerCrash(String),

    #[error("Test execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("Failed to start daemon: {0}")]
    StartupFailure(String),

    #[error("Worker not available")]
    NoWorkerAvailable,

    #[error("Shutdown error: {0}")]
    ShutdownError(String),
}

/// Errors that can occur in the binary protocol
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid magic bytes: expected 0xDEADBEEF, got {0:#X}")]
    InvalidMagic(u32),

    #[error("Invalid message type: {0}")]
    InvalidMessageType(u8),

    #[error("Payload too large: {0} bytes (max: {1})")]
    PayloadTooLarge(usize, usize),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Shared memory error: {0}")]
    SharedMemoryError(String),
}

/// Errors that can occur in the dependency graph
#[derive(Error, Debug)]
pub enum GraphError {
    #[error("Failed to parse imports: {0}")]
    ParseError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Cache corrupted: {0}")]
    CacheCorrupted(String),

    #[error("Cycle detected in import graph")]
    CycleDetected,
}

/// Errors that can occur during test execution
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Worker panicked: {0}")]
    WorkerPanic(String),

    #[error("Queue full, cannot accept more tests")]
    QueueFull,

    #[error("Test not found: {0}")]
    TestNotFound(String),

    #[error("Daemon error: {0}")]
    DaemonError(#[from] DaemonError),
}

/// Errors that can occur in fixture caching
#[derive(Error, Debug)]
pub enum FixtureError {
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),

    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Fixture not found: {0}")]
    NotFound(String),
}

/// Errors that can occur in snapshot testing
#[derive(Error, Debug)]
pub enum SnapshotError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Snapshot not found: {0}")]
    NotFound(String),

    #[error("Index corrupted: {0}")]
    IndexCorrupted(String),
}
