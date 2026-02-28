//! Integration tests for dx-js-compatibility crate.
//!
//! These tests verify:
//! - Cross-module compatibility
//! - Feature flag combinations
//! - Error propagation across modules
//! - API consistency

// ============================================================================
// Core Library Tests (always available)
// ============================================================================

mod core_tests {
    use dx_js_compatibility::*;

    #[test]
    fn test_version_info() {
        let version = version();
        assert!(!version.is_empty());
        // Version should be semver format
        let parts: Vec<&str> = version.split('.').collect();
        assert!(parts.len() >= 2, "Version should be semver format");
    }

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        let name = platform.name();
        assert!(!name.is_empty());

        // Platform should be one of the known values
        assert!(
            matches!(
                platform,
                Platform::Linux | Platform::MacOS | Platform::Windows | Platform::Unknown
            ),
            "Platform should be a known variant"
        );
    }

    #[test]
    fn test_architecture_detection() {
        let arch = Architecture::current();
        let name = arch.name();
        assert!(!name.is_empty());

        // Architecture should be one of the known values
        assert!(
            matches!(arch, Architecture::X64 | Architecture::Arm64 | Architecture::Unknown),
            "Architecture should be a known variant"
        );
    }

    #[test]
    fn test_system_info() {
        let info = system_info();

        assert!(!info.version.is_empty());
        assert!(matches!(
            info.platform,
            Platform::Linux | Platform::MacOS | Platform::Windows | Platform::Unknown
        ));
        assert!(matches!(
            info.arch,
            Architecture::X64 | Architecture::Arm64 | Architecture::Unknown
        ));

        // Display should work
        let display = format!("{}", info);
        assert!(display.contains("dx-js-compatibility"));
    }

    #[test]
    fn test_enabled_features() {
        let features = enabled_features();
        // Features list should be valid (may be empty if no features enabled)
        for feature in &features {
            assert!(!feature.is_empty());
        }
    }

    #[test]
    fn test_path_separator() {
        let sep = path_separator();
        #[cfg(windows)]
        assert_eq!(sep, '\\');
        #[cfg(not(windows))]
        assert_eq!(sep, '/');
    }

    #[test]
    fn test_line_ending() {
        let ending = line_ending();
        #[cfg(windows)]
        assert_eq!(ending, "\r\n");
        #[cfg(not(windows))]
        assert_eq!(ending, "\n");
    }

    #[test]
    fn test_error_code_values() {
        // Verify error codes have correct numeric values
        assert_eq!(ErrorCode::ENOENT as u32, 2);
        assert_eq!(ErrorCode::EACCES as u32, 13);
        assert_eq!(ErrorCode::EEXIST as u32, 17);
        assert_eq!(ErrorCode::EISDIR as u32, 21);
        assert_eq!(ErrorCode::ENOTDIR as u32, 20);
        assert_eq!(ErrorCode::ENOTEMPTY as u32, 39);
        assert_eq!(ErrorCode::ETIMEDOUT as u32, 110);
        assert_eq!(ErrorCode::ECONNREFUSED as u32, 111);
    }

    #[test]
    fn test_error_code_strings() {
        assert_eq!(ErrorCode::ENOENT.as_str(), "ENOENT");
        assert_eq!(ErrorCode::EACCES.as_str(), "EACCES");
        assert_eq!(ErrorCode::EEXIST.as_str(), "EEXIST");
        assert_eq!(ErrorCode::EISDIR.as_str(), "EISDIR");
        assert_eq!(ErrorCode::ENOTDIR.as_str(), "ENOTDIR");
        assert_eq!(ErrorCode::ENOTEMPTY.as_str(), "ENOTEMPTY");
        assert_eq!(ErrorCode::ETIMEDOUT.as_str(), "ETIMEDOUT");
        assert_eq!(ErrorCode::ECONNREFUSED.as_str(), "ECONNREFUSED");
    }

    #[test]
    fn test_error_code_from_io_error() {
        use std::io::{Error, ErrorKind};

        let not_found = Error::new(ErrorKind::NotFound, "test");
        assert_eq!(ErrorCode::from_io_error(&not_found), Some(ErrorCode::ENOENT));

        let permission = Error::new(ErrorKind::PermissionDenied, "test");
        assert_eq!(ErrorCode::from_io_error(&permission), Some(ErrorCode::EACCES));

        let exists = Error::new(ErrorKind::AlreadyExists, "test");
        assert_eq!(ErrorCode::from_io_error(&exists), Some(ErrorCode::EEXIST));

        let timeout = Error::new(ErrorKind::TimedOut, "test");
        assert_eq!(ErrorCode::from_io_error(&timeout), Some(ErrorCode::ETIMEDOUT));

        let refused = Error::new(ErrorKind::ConnectionRefused, "test");
        assert_eq!(ErrorCode::from_io_error(&refused), Some(ErrorCode::ECONNREFUSED));

        // Unknown error kind should return None
        let other = Error::other("test");
        assert_eq!(ErrorCode::from_io_error(&other), None);
    }

    #[test]
    fn test_compat_error_display() {
        let errors = vec![
            CompatError::Fs("test fs error".to_string()),
            CompatError::Network("test network error".to_string()),
            CompatError::Sqlite("test sqlite error".to_string()),
            CompatError::S3("test s3 error".to_string()),
            CompatError::Ffi("test ffi error".to_string()),
            CompatError::Shell("test shell error".to_string()),
            CompatError::Compile("test compile error".to_string()),
            CompatError::Hmr("test hmr error".to_string()),
            CompatError::Plugin("test plugin error".to_string()),
            CompatError::Macro("test macro error".to_string()),
            CompatError::Html("test html error".to_string()),
            CompatError::InvalidArgument("test invalid arg".to_string()),
            CompatError::NotFound("test not found".to_string()),
            CompatError::PermissionDenied("test permission".to_string()),
            CompatError::Timeout("test timeout".to_string()),
        ];

        for error in errors {
            let display = format!("{}", error);
            assert!(!display.is_empty(), "Error display should not be empty");
        }
    }

    #[test]
    fn test_compat_error_from_io() {
        use std::io::{Error, ErrorKind};

        let io_error = Error::new(ErrorKind::NotFound, "file not found");
        let compat_error: CompatError = io_error.into();

        let display = format!("{}", compat_error);
        assert!(display.contains("file not found") || display.contains("IO error"));
    }
}
