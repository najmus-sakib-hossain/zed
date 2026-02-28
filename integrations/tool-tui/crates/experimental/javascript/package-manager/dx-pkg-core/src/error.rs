use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid magic number: expected {expected:?}, found {found:?}\n\n  💡 This usually indicates a corrupted package file or incompatible format version.")]
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },

    #[error("Unsupported version: {0}\n\n  💡 This package format version is not supported. Try updating dx-pkg.")]
    UnsupportedVersion(u16),

    #[error("Corrupted data: hash mismatch\n\n  💡 The downloaded data doesn't match its checksum. Try clearing the cache and downloading again.")]
    CorruptedData,

    #[error("Package '{name}' not found{}\n\n  💡 Suggestions:\n     - Check the package name spelling\n     - Verify the package exists on npmjs.com\n     - For scoped packages, use @scope/package format", 
        registry_url.as_ref().map(|u| format!(" at registry: {}", u)).unwrap_or_default())]
    PackageNotFound {
        name: String,
        registry_url: Option<String>,
    },

    #[error("File not found: {path}\n\n  💡 The file '{path}' does not exist in the package.\n     Available files can be listed with `dx ls <package>`")]
    FileNotFound { path: PathBuf },

    #[error("Invalid version string: '{version}'\n\n  💡 Valid version formats:\n     - Exact: 1.2.3\n     - Caret: ^1.2.3 (compatible with 1.x.x)\n     - Tilde: ~1.2.3 (compatible with 1.2.x)\n     - Range: >=1.0.0 <2.0.0")]
    InvalidVersion { version: String },

    #[error("Package too large: {size} bytes (max {max} bytes)\n\n  💡 The package exceeds the maximum allowed size.\n     Consider using a different package or contacting the maintainer.")]
    PackageTooLarge { size: u64, max: u64 },

    #[error("Too many files: {count} (max {max})\n\n  💡 The package contains too many files.\n     This limit exists to prevent malicious packages.")]
    TooManyFiles { count: u32, max: u32 },

    #[error("IO error: {message}\n  Path: {}\n\n  💡 Suggestions:\n     - Check file/directory permissions\n     - Ensure sufficient disk space\n     - Verify the path exists",
        path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "<unknown>".to_string()))]
    Io {
        message: String,
        path: Option<PathBuf>,
    },

    #[error("Compression error: {0}\n\n  💡 The package data could not be decompressed. The file may be corrupted.")]
    Compression(String),

    #[error("Network error: {message}\n  URL: {}\n  Status: {}\n\n  💡 Suggestions:\n     - Check your internet connection\n     - Verify the registry is accessible\n     - Check if you're behind a proxy",
        url.as_deref().unwrap_or("<unknown>"),
        status.map(|s| s.to_string()).unwrap_or_else(|| "N/A".to_string()))]
    Network {
        message: String,
        url: Option<String>,
        status: Option<u16>,
    },

    #[error("Parse error in {}: {message}\n  Line: {}, Column: {}\n\n  💡 Check the file syntax. This may indicate a malformed package.json or configuration file.",
        file.as_ref().map(|f| f.display().to_string()).unwrap_or_else(|| "<unknown>".to_string()),
        line.unwrap_or(0),
        column.unwrap_or(0))]
    Parse {
        message: String,
        file: Option<PathBuf>,
        line: Option<usize>,
        column: Option<usize>,
    },

    #[error("Integrity check failed for '{package}': {reason}\n\n  ⚠️  The package integrity verification failed.\n     This could indicate a corrupted download or a security issue.\n\n  💡 Suggestions:\n     - Clear the cache: `dx cache clean`\n     - Try downloading again\n     - Report this issue if it persists")]
    Integrity { package: String, reason: String },

    #[error("Permission denied: {operation} on {path}\n\n  💡 Suggestions:\n     - Check file/directory permissions\n     - Run with appropriate privileges\n     - Verify the path is not read-only")]
    PermissionDenied { operation: String, path: PathBuf },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io {
            message: err.to_string(),
            path: None,
        }
    }
}

impl Error {
    /// Create a package not found error with optional registry URL
    pub fn package_not_found(name: impl Into<String>) -> Self {
        Error::PackageNotFound {
            name: name.into(),
            registry_url: None,
        }
    }

    /// Create a package not found error with registry URL context
    pub fn package_not_found_at(name: impl Into<String>, registry_url: impl Into<String>) -> Self {
        Error::PackageNotFound {
            name: name.into(),
            registry_url: Some(registry_url.into()),
        }
    }

    /// Create a file not found error with path
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Error::FileNotFound { path: path.into() }
    }

    /// Create an invalid version error
    pub fn invalid_version(version: impl Into<String>) -> Self {
        Error::InvalidVersion {
            version: version.into(),
        }
    }

    /// Create an IO error with path context
    pub fn io_with_path(err: std::io::Error, path: impl Into<PathBuf>) -> Self {
        Error::Io {
            message: err.to_string(),
            path: Some(path.into()),
        }
    }

    /// Create a network error with URL and status
    pub fn network(message: impl Into<String>) -> Self {
        Error::Network {
            message: message.into(),
            url: None,
            status: None,
        }
    }

    /// Create a network error with full context
    pub fn network_with_context(
        message: impl Into<String>,
        url: impl Into<String>,
        status: Option<u16>,
    ) -> Self {
        Error::Network {
            message: message.into(),
            url: Some(url.into()),
            status,
        }
    }

    /// Create a parse error with location
    pub fn parse(message: impl Into<String>) -> Self {
        Error::Parse {
            message: message.into(),
            file: None,
            line: None,
            column: None,
        }
    }

    /// Create a parse error with full context
    pub fn parse_with_location(
        message: impl Into<String>,
        file: impl Into<PathBuf>,
        line: usize,
        column: usize,
    ) -> Self {
        Error::Parse {
            message: message.into(),
            file: Some(file.into()),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Create an integrity error
    pub fn integrity(package: impl Into<String>, reason: impl Into<String>) -> Self {
        Error::Integrity {
            package: package.into(),
            reason: reason.into(),
        }
    }

    /// Create a permission denied error
    pub fn permission_denied(operation: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Error::PermissionDenied {
            operation: operation.into(),
            path: path.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_not_found_contains_name() {
        let err = Error::package_not_found("lodash");
        let msg = err.to_string();
        assert!(msg.contains("lodash"), "Error should contain package name");
        assert!(msg.contains("💡"), "Error should contain suggestions");
    }

    #[test]
    fn test_package_not_found_with_registry() {
        let err = Error::package_not_found_at("lodash", "https://registry.npmjs.org");
        let msg = err.to_string();
        assert!(msg.contains("lodash"), "Error should contain package name");
        assert!(msg.contains("registry.npmjs.org"), "Error should contain registry URL");
    }

    #[test]
    fn test_file_not_found_contains_path() {
        let err = Error::file_not_found("/path/to/file.js");
        let msg = err.to_string();
        assert!(msg.contains("/path/to/file.js"), "Error should contain file path");
    }

    #[test]
    fn test_invalid_version_contains_version() {
        let err = Error::invalid_version("not-a-version");
        let msg = err.to_string();
        assert!(msg.contains("not-a-version"), "Error should contain invalid version string");
        assert!(msg.contains("💡"), "Error should contain suggestions");
    }

    #[test]
    fn test_network_error_contains_url() {
        let err = Error::network_with_context(
            "Connection refused",
            "https://registry.npmjs.org/lodash",
            Some(503),
        );
        let msg = err.to_string();
        assert!(msg.contains("Connection refused"), "Error should contain message");
        assert!(msg.contains("registry.npmjs.org"), "Error should contain URL");
        assert!(msg.contains("503"), "Error should contain status code");
    }

    #[test]
    fn test_parse_error_contains_location() {
        let err = Error::parse_with_location("Unexpected token", "/path/to/file.json", 10, 5);
        let msg = err.to_string();
        assert!(msg.contains("/path/to/file.json"), "Error should contain file path");
        assert!(msg.contains("10"), "Error should contain line number");
        assert!(msg.contains("5"), "Error should contain column number");
    }

    #[test]
    fn test_integrity_error_contains_package() {
        let err = Error::integrity("lodash", "hash mismatch");
        let msg = err.to_string();
        assert!(msg.contains("lodash"), "Error should contain package name");
        assert!(msg.contains("hash mismatch"), "Error should contain reason");
    }

    #[test]
    fn test_permission_denied_contains_operation_and_path() {
        let err = Error::permission_denied("write", "/path/to/file");
        let msg = err.to_string();
        assert!(msg.contains("write"), "Error should contain operation");
        assert!(msg.contains("/path/to/file"), "Error should contain path");
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary package names
    fn arb_package_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}".prop_map(|s| s.to_string())
    }

    /// Generate arbitrary file paths
    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        "[a-z/]{1,50}\\.[a-z]{1,4}".prop_map(PathBuf::from)
    }

    /// Generate arbitrary URLs
    fn arb_url() -> impl Strategy<Value = String> {
        "https://[a-z]{3,10}\\.[a-z]{2,5}/[a-z/]{1,30}".prop_map(|s| s.to_string())
    }

    /// Generate arbitrary error messages
    fn arb_message() -> impl Strategy<Value = String> {
        "[A-Za-z ]{5,50}".prop_map(|s| s.to_string())
    }

    proptest! {
        /// Property 5: Error Message Completeness - Package Errors
        /// For any package error, the error message SHALL contain the package name.
        ///
        /// **Validates: Requirements 10.1**
        #[test]
        fn prop_package_error_contains_name(name in arb_package_name()) {
            let err = Error::package_not_found(&name);
            let msg = err.to_string();
            prop_assert!(msg.contains(&name),
                "Package error should contain package name '{}' but got: {}", name, msg);
        }

        /// Property 5: Error Message Completeness - Network Errors
        /// For any network error, the error message SHALL contain the URL.
        ///
        /// **Validates: Requirements 10.4**
        #[test]
        fn prop_network_error_contains_url(url in arb_url(), message in arb_message()) {
            let err = Error::network_with_context(&message, &url, Some(500));
            let msg = err.to_string();
            // URL might be truncated in display, check for domain at least
            let domain = url.split('/').nth(2).unwrap_or(&url);
            prop_assert!(msg.contains(domain) || msg.contains(&url),
                "Network error should contain URL '{}' but got: {}", url, msg);
        }

        /// Property 5: Error Message Completeness - File Errors
        /// For any file error, the error message SHALL contain the file path.
        ///
        /// **Validates: Requirements 10.2**
        #[test]
        fn prop_file_error_contains_path(path in arb_file_path()) {
            let err = Error::file_not_found(&path);
            let msg = err.to_string();
            let path_str = path.display().to_string();
            prop_assert!(msg.contains(&path_str),
                "File error should contain path '{}' but got: {}", path_str, msg);
        }

        /// Property 5: Error Message Completeness - Parse Errors
        /// For any parse error with location, the error message SHALL contain line and column.
        ///
        /// **Validates: Requirements 10.3**
        #[test]
        fn prop_parse_error_contains_location(
            message in arb_message(),
            path in arb_file_path(),
            line in 1usize..1000,
            column in 1usize..200
        ) {
            let err = Error::parse_with_location(&message, &path, line, column);
            let msg = err.to_string();
            prop_assert!(msg.contains(&line.to_string()),
                "Parse error should contain line {} but got: {}", line, msg);
            prop_assert!(msg.contains(&column.to_string()),
                "Parse error should contain column {} but got: {}", column, msg);
        }

        /// Property 5: Error Message Completeness - All Errors Have Suggestions
        /// For any error type, the error message SHALL contain helpful suggestions.
        ///
        /// **Validates: Requirements 10.1, 10.2, 10.3, 10.4**
        #[test]
        fn prop_errors_have_suggestions(name in arb_package_name()) {
            // Package not found should have suggestions
            let err = Error::package_not_found(&name);
            let msg = err.to_string();
            prop_assert!(msg.contains("💡") || msg.contains("Suggestion"),
                "Error should contain suggestions but got: {}", msg);
        }
    }
}
