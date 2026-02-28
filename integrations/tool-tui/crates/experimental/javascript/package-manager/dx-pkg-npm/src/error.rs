//! Error types for npm client with helpful messages

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "Package not found: {0}\n\n  💡 Suggestions:\n     - Check the package name spelling\n     - Verify the package exists on npmjs.com\n     - For scoped packages, use @scope/package format"
    )]
    PackageNotFound(String),

    #[error(
        "Network error: {0}\n\n  💡 Suggestions:\n     - Check your internet connection\n     - Verify npm registry is accessible (registry.npmjs.org)\n     - Check if you're behind a proxy"
    )]
    NetworkError(String),

    #[error(
        "Failed to parse response: {0}\n\n  💡 This might be a temporary registry issue. Try again in a few moments."
    )]
    ParseError(String),

    #[error(
        "Download failed: {0}\n\n  💡 Suggestions:\n     - Check your internet connection\n     - The package tarball might be temporarily unavailable\n     - Try running the command again"
    )]
    DownloadFailed(String),

    #[error(
        "Invalid version constraint: {0}\n\n  💡 Valid formats:\n     - Exact: 1.2.3\n     - Caret: ^1.2.3 (compatible with 1.x.x)\n     - Tilde: ~1.2.3 (compatible with 1.2.x)\n     - Range: >=1.0.0 <2.0.0"
    )]
    InvalidConstraint(String),

    #[error(
        "No matching version found for the specified constraint\n\n  💡 Suggestions:\n     - Check available versions with `dx info <package>`\n     - Try a less restrictive version constraint\n     - Use 'latest' to get the most recent version"
    )]
    NoVersionFound,

    #[error(
        "IO error: {0}\n\n  💡 Suggestions:\n     - Check file/directory permissions\n     - Ensure sufficient disk space\n     - Verify the path exists"
    )]
    IoError(String),

    #[error(
        "Version conflict: {package} requires {required} but {conflicting} is already installed\n\n  💡 Suggestions:\n     - Try `dx update {package}` to resolve\n     - Check for peer dependency requirements\n     - Consider using `--force` to override"
    )]
    VersionConflict {
        package: String,
        required: String,
        conflicting: String,
    },

    #[error(
        "Checksum mismatch for {package}\n\n  ⚠️  The downloaded package doesn't match the expected checksum.\n     This could indicate a corrupted download or a security issue.\n\n  💡 Suggestions:\n     - Clear the cache and try again\n     - Check your network for interference\n     - Report this issue if it persists"
    )]
    ChecksumMismatch { package: String },

    #[error(
        "Registry authentication required\n\n  💡 Suggestions:\n     - Run `npm login` to authenticate\n     - Check your .npmrc configuration\n     - Verify your npm token is valid"
    )]
    AuthenticationRequired,

    #[error(
        "Rate limited by npm registry\n\n  💡 The npm registry has temporarily limited your requests.\n     Please wait a few minutes before trying again."
    )]
    RateLimited,
}

impl Error {
    /// Create a package not found error with the package name
    pub fn package_not_found(name: impl Into<String>) -> Self {
        Error::PackageNotFound(name.into())
    }

    /// Create a network error with details
    pub fn network(msg: impl Into<String>) -> Self {
        Error::NetworkError(msg.into())
    }

    /// Create a download failed error
    pub fn download_failed(url: impl Into<String>) -> Self {
        Error::DownloadFailed(url.into())
    }

    /// Create a version conflict error
    pub fn version_conflict(
        package: impl Into<String>,
        required: impl Into<String>,
        conflicting: impl Into<String>,
    ) -> Self {
        Error::VersionConflict {
            package: package.into(),
            required: required.into(),
            conflicting: conflicting.into(),
        }
    }

    /// Create a checksum mismatch error
    pub fn checksum_mismatch(package: impl Into<String>) -> Self {
        Error::ChecksumMismatch {
            package: package.into(),
        }
    }
}
