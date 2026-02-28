//! Error hint generation

use super::types::DxError;

impl DxError {
    /// Returns a context-specific hint for the error
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            // Configuration errors
            DxError::ConfigNotFound { .. } => {
                Some("Run `dx init` to create a new project with dx.toml")
            }
            DxError::ConfigInvalid { .. } => {
                Some("Check the TOML syntax and ensure all required fields are present")
            }
            DxError::ConfigMissingField { .. } => {
                Some("Add the missing field to your dx.toml configuration")
            }

            // File system errors
            DxError::FileNotFound { .. } => {
                Some("Check that the file path is correct and the file exists")
            }
            DxError::DirectoryNotFound { .. } => {
                Some("Check that the directory path is correct and exists")
            }
            DxError::PermissionDenied { path } => {
                let path_str = path.to_string_lossy();
                if path_str.contains(".dx") || path_str.contains("dx.toml") {
                    Some(
                        "Check file ownership with `ls -la`. Try `chmod 644` for files or `chmod 755` for directories. On Windows, right-click → Properties → Security",
                    )
                } else {
                    Some(
                        "Try running with elevated permissions (sudo on Unix, Run as Administrator on Windows), or check file ownership with `ls -la`",
                    )
                }
            }
            DxError::FileExists { .. } => Some("Use --force to overwrite existing files"),
            DxError::Io { message } => {
                if message.contains("disk full") || message.contains("no space") {
                    Some(
                        "Free up disk space and try again. Check available space with `df -h` (Unix) or `dir` (Windows)",
                    )
                } else if message.contains("too many open files") {
                    Some(
                        "Close some applications or increase the file descriptor limit with `ulimit -n`",
                    )
                } else {
                    Some("Check disk space and file system permissions")
                }
            }
            DxError::SymlinkLoop { .. } => {
                Some("Check for circular symbolic links. Use `ls -la` to inspect symlink targets")
            }

            // Network errors
            DxError::Network { message } => {
                if message.contains("dns")
                    || message.contains("DNS")
                    || message.contains("resolve")
                    || message.contains("getaddrinfo")
                {
                    Some(
                        "DNS resolution failed. Check your DNS settings, try `nslookup` to test, or use a different DNS server (e.g., 8.8.8.8)",
                    )
                } else if message.contains("connection refused") {
                    Some(
                        "Connection refused. Check if the server is running and the port is correct",
                    )
                } else if message.contains("connection reset") {
                    Some(
                        "Connection was reset. This may be a firewall issue or server problem. Try again later",
                    )
                } else {
                    Some(
                        "Check your internet connection. Try `ping google.com` to test connectivity",
                    )
                }
            }
            DxError::Timeout { .. } => Some(
                "Request timed out. Check your network connection, try again, or increase timeout with --timeout",
            ),

            // TLS/SSL errors
            DxError::Tls { message } => {
                if message.contains("certificate") || message.contains("cert") {
                    Some(
                        "SSL certificate error. Update your CA certificates: `update-ca-certificates` (Linux), or check system date/time is correct",
                    )
                } else if message.contains("handshake") {
                    Some(
                        "TLS handshake failed. The server may not support your TLS version. Check firewall/proxy settings",
                    )
                } else if message.contains("expired") {
                    Some(
                        "Certificate has expired. Check your system date/time, or the server's certificate needs renewal",
                    )
                } else {
                    Some(
                        "TLS error. Update CA certificates, check system date/time, or try with --insecure (not recommended)",
                    )
                }
            }

            // HTTP errors
            DxError::Http { status, .. } if *status == 401 => {
                Some("Authentication required. Check your credentials or API token")
            }
            DxError::Http { status, .. } if *status == 403 => {
                Some("Access forbidden. Check your permissions or API token scope")
            }
            DxError::Http { status, .. } if *status == 404 => {
                Some("Resource not found. Check the URL or resource name")
            }
            DxError::Http { status, .. } if *status == 429 => {
                Some("Rate limited. Wait a moment and try again, or check API rate limits")
            }
            DxError::Http { status, .. } if *status >= 500 && *status < 600 => {
                Some("Server error. The service may be experiencing issues. Try again later")
            }
            DxError::Http { .. } => Some("HTTP request failed. Check the URL and try again"),

            // Tool errors
            DxError::ToolNotInstalled { .. } => {
                Some("Install the tool with `dx forge install <tool>`")
            }
            DxError::ToolVersionMismatch { .. } => Some(
                "Update the tool with `dx forge update <tool>` or specify a compatible version",
            ),
            DxError::ToolExecutionFailed { .. } => Some(
                "Tool execution failed. Check the tool's logs or run with --verbose for details",
            ),

            // Build errors
            DxError::BuildFailed { .. } => {
                Some("Build failed. Check the error message above and fix the issues")
            }
            DxError::CompilationError { .. } => {
                Some("Compilation error. Fix the syntax error at the indicated location")
            }

            // Update errors
            DxError::SignatureInvalid => Some(
                "Signature verification failed. The download may be corrupted or tampered with. Try downloading again from the official source",
            ),
            DxError::ChecksumMismatch { .. } => {
                Some("Checksum mismatch. The download may be corrupted. Try downloading again")
            }
            DxError::UpdateDownloadFailed { .. } => {
                Some("Update download failed. Check your internet connection and try again")
            }
            DxError::DeltaPatchFailed { .. } => {
                Some("Delta patch failed. Try a full download with --full-download")
            }

            // Shell errors
            DxError::ShellNotDetected => Some(
                "Could not detect your shell. Specify it with --shell (bash, zsh, fish, powershell)",
            ),
            DxError::ShellIntegrationExists { .. } => {
                Some("Shell integration already installed. Use --force to reinstall")
            }

            // General errors
            DxError::InvalidArgument { .. } => {
                Some("Invalid argument provided. Check the command help with --help")
            }
            DxError::Cancelled => Some("Operation was cancelled"),
            DxError::Internal { .. } => {
                Some("An internal error occurred. Please report this issue with the error details")
            }
            DxError::LockTimeout { .. } => Some(
                "Another process may be holding the lock. Wait and try again, or check for stale lock files",
            ),
        }
    }

    /// Returns whether the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, DxError::Network { .. } | DxError::Timeout { .. } | DxError::Tls { .. })
    }
}
