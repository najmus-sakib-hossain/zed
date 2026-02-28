//! Error types for Node.js compatibility layer.

use std::io;
use thiserror::Error;

/// Node.js compatible error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ErrorCode {
    /// No such file or directory
    ENOENT = -2,
    /// Permission denied
    EACCES = -13,
    /// File exists
    EEXIST = -17,
    /// Is a directory
    EISDIR = -21,
    /// Not a directory
    ENOTDIR = -20,
    /// Directory not empty
    ENOTEMPTY = -39,
    /// Operation timed out
    ETIMEDOUT = -110,
    /// Connection refused
    ECONNREFUSED = -111,
    /// Invalid argument
    EINVAL = -22,
    /// Bad file descriptor
    EBADF = -9,
    /// Resource busy
    EBUSY = -16,
    /// Operation canceled
    ECANCELED = -125,
    /// No child processes
    ECHILD = -10,
    /// Connection aborted
    ECONNABORTED = -103,
    /// Connection reset
    ECONNRESET = -104,
    /// Address already in use
    EADDRINUSE = -98,
    /// Cross-device link
    EXDEV = -18,
    /// Unknown error
    UNKNOWN = -1,
}

impl ErrorCode {
    /// Get the error code name as a string.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ENOENT => "ENOENT",
            ErrorCode::EACCES => "EACCES",
            ErrorCode::EEXIST => "EEXIST",
            ErrorCode::EISDIR => "EISDIR",
            ErrorCode::ENOTDIR => "ENOTDIR",
            ErrorCode::ENOTEMPTY => "ENOTEMPTY",
            ErrorCode::ETIMEDOUT => "ETIMEDOUT",
            ErrorCode::ECONNREFUSED => "ECONNREFUSED",
            ErrorCode::EINVAL => "EINVAL",
            ErrorCode::EBADF => "EBADF",
            ErrorCode::EBUSY => "EBUSY",
            ErrorCode::ECANCELED => "ECANCELED",
            ErrorCode::ECHILD => "ECHILD",
            ErrorCode::ECONNABORTED => "ECONNABORTED",
            ErrorCode::ECONNRESET => "ECONNRESET",
            ErrorCode::EADDRINUSE => "EADDRINUSE",
            ErrorCode::EXDEV => "EXDEV",
            ErrorCode::UNKNOWN => "UNKNOWN",
        }
    }

    /// Get the error message for this code.
    pub fn message(&self) -> &'static str {
        match self {
            ErrorCode::ENOENT => "no such file or directory",
            ErrorCode::EACCES => "permission denied",
            ErrorCode::EEXIST => "file already exists",
            ErrorCode::EISDIR => "illegal operation on a directory",
            ErrorCode::ENOTDIR => "not a directory",
            ErrorCode::ENOTEMPTY => "directory not empty",
            ErrorCode::ETIMEDOUT => "operation timed out",
            ErrorCode::ECONNREFUSED => "connection refused",
            ErrorCode::EINVAL => "invalid argument",
            ErrorCode::EBADF => "bad file descriptor",
            ErrorCode::EBUSY => "resource busy or locked",
            ErrorCode::ECANCELED => "operation canceled",
            ErrorCode::ECHILD => "no child processes",
            ErrorCode::ECONNABORTED => "connection aborted",
            ErrorCode::ECONNRESET => "connection reset by peer",
            ErrorCode::EADDRINUSE => "address already in use",
            ErrorCode::EXDEV => "cross-device link not permitted",
            ErrorCode::UNKNOWN => "unknown error",
        }
    }

    /// Convert from an IO error kind.
    pub fn from_io_error_kind(kind: io::ErrorKind) -> Self {
        match kind {
            io::ErrorKind::NotFound => ErrorCode::ENOENT,
            io::ErrorKind::PermissionDenied => ErrorCode::EACCES,
            io::ErrorKind::AlreadyExists => ErrorCode::EEXIST,
            io::ErrorKind::TimedOut => ErrorCode::ETIMEDOUT,
            io::ErrorKind::ConnectionRefused => ErrorCode::ECONNREFUSED,
            io::ErrorKind::ConnectionReset => ErrorCode::ECONNRESET,
            io::ErrorKind::ConnectionAborted => ErrorCode::ECONNABORTED,
            io::ErrorKind::InvalidInput => ErrorCode::EINVAL,
            io::ErrorKind::InvalidData => ErrorCode::EINVAL,
            _ => ErrorCode::UNKNOWN,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unified error type for Node.js compatibility operations.
#[derive(Debug, Error)]
pub struct NodeError {
    /// The error code
    pub code: ErrorCode,
    /// The error message
    pub message: String,
    /// The syscall that failed (if applicable)
    pub syscall: Option<String>,
    /// The path involved (if applicable)
    pub path: Option<String>,
}

impl NodeError {
    /// Create a new NodeError with the given code and message.
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            syscall: None,
            path: None,
        }
    }

    /// Create a new NodeError with syscall information.
    pub fn with_syscall(mut self, syscall: impl Into<String>) -> Self {
        self.syscall = Some(syscall.into());
        self
    }

    /// Create a new NodeError with path information.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Create an ENOENT error for a missing file.
    pub fn enoent(path: impl Into<String>) -> Self {
        let path_str = path.into();
        Self::new(ErrorCode::ENOENT, format!("ENOENT: no such file or directory, '{}'", path_str))
            .with_path(path_str)
    }

    /// Create an EACCES error for permission denied.
    pub fn eacces(path: impl Into<String>) -> Self {
        let path_str = path.into();
        Self::new(ErrorCode::EACCES, format!("EACCES: permission denied, '{}'", path_str))
            .with_path(path_str)
    }

    /// Create an EEXIST error for file already exists.
    pub fn eexist(path: impl Into<String>) -> Self {
        let path_str = path.into();
        Self::new(ErrorCode::EEXIST, format!("EEXIST: file already exists, '{}'", path_str))
            .with_path(path_str)
    }

    /// Create an EISDIR error for illegal operation on directory.
    pub fn eisdir(path: impl Into<String>) -> Self {
        let path_str = path.into();
        Self::new(
            ErrorCode::EISDIR,
            format!("EISDIR: illegal operation on a directory, '{}'", path_str),
        )
        .with_path(path_str)
    }

    /// Create an ENOTDIR error for not a directory.
    pub fn enotdir(path: impl Into<String>) -> Self {
        let path_str = path.into();
        Self::new(ErrorCode::ENOTDIR, format!("ENOTDIR: not a directory, '{}'", path_str))
            .with_path(path_str)
    }
}

impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<io::Error> for NodeError {
    fn from(err: io::Error) -> Self {
        let code = ErrorCode::from_io_error_kind(err.kind());
        Self::new(code, err.to_string())
    }
}

/// Result type for Node.js compatibility operations.
pub type NodeResult<T> = Result<T, NodeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::ENOENT.as_str(), "ENOENT");
        assert_eq!(ErrorCode::EACCES.as_str(), "EACCES");
        assert_eq!(ErrorCode::EEXIST.as_str(), "EEXIST");
    }

    #[test]
    fn test_error_code_from_io() {
        assert_eq!(ErrorCode::from_io_error_kind(io::ErrorKind::NotFound), ErrorCode::ENOENT);
        assert_eq!(
            ErrorCode::from_io_error_kind(io::ErrorKind::PermissionDenied),
            ErrorCode::EACCES
        );
    }

    #[test]
    fn test_node_error_enoent() {
        let err = NodeError::enoent("/path/to/file");
        assert_eq!(err.code, ErrorCode::ENOENT);
        assert!(err.message.contains("ENOENT"));
        assert_eq!(err.path, Some("/path/to/file".to_string()));
    }

    #[test]
    fn test_node_error_eacces() {
        let err = NodeError::eacces("/protected/file");
        assert_eq!(err.code, ErrorCode::EACCES);
        assert!(err.message.contains("EACCES"));
    }
}
