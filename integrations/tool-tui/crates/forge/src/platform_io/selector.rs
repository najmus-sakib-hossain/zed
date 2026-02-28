//! Platform I/O backend selector.
//!
//! This module provides automatic selection of the most performant I/O backend
//! for the current platform. When a native backend fails to initialize, the
//! system gracefully falls back to the portable tokio-based backend with a warning.

use std::sync::Arc;
use tracing::{debug, info, warn};

use super::{FallbackBackend, IoBackend, Platform, PlatformIO, PlatformInfo};

#[cfg(target_os = "linux")]
use super::IoUringBackend;

#[cfg(target_os = "macos")]
use super::KqueueBackend;

#[cfg(target_os = "windows")]
use super::IocpBackend;

/// Create the most performant platform I/O backend for the current system.
///
/// This function automatically detects the platform and selects the best
/// available I/O backend:
/// - Linux: io_uring (if kernel 5.1+ and available)
/// - macOS: kqueue
/// - Windows: IOCP
/// - Fallback: tokio async I/O (always available)
///
/// # Graceful Fallback
///
/// When a native backend fails to initialize, the system logs a warning and
/// falls back to the portable tokio-based backend. This ensures the application
/// continues to operate correctly even when platform-specific features are
/// unavailable.
///
/// # Example
/// ```rust,ignore
/// use dx_forge::platform_io::create_platform_io;
///
/// let io = create_platform_io();
/// println!("Using backend: {}", io.backend_name());
/// ```
pub fn create_platform_io() -> Arc<dyn PlatformIO> {
    create_platform_io_with_fallback_tracking().0
}

/// Create platform I/O backend and return whether fallback was used.
///
/// This is useful for testing and diagnostics to determine if the native
/// backend was successfully initialized or if fallback occurred.
///
/// Returns a tuple of (backend, did_fallback) where did_fallback is true
/// if the native backend failed and we fell back to the portable backend.
pub fn create_platform_io_with_fallback_tracking() -> (Arc<dyn PlatformIO>, bool) {
    let platform = Platform::current();
    debug!("Detected platform: {:?}", platform);

    #[cfg(target_os = "linux")]
    {
        if IoUringBackend::is_available() {
            match IoUringBackend::new(256) {
                Ok(backend) => {
                    info!("Using io_uring backend for high-performance I/O");
                    return (Arc::new(backend), false);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        platform = "linux",
                        native_backend = "io_uring",
                        "Native io_uring backend failed to initialize, falling back to portable backend. \
                         This may result in reduced I/O performance. Error: {}",
                        e
                    );
                }
            }
        } else {
            warn!(
                platform = "linux",
                native_backend = "io_uring",
                "io_uring not available on this system (requires kernel 5.1+), \
                 falling back to portable backend"
            );
        }

        info!("Using fallback (tokio) backend on Linux");
        return (Arc::new(FallbackBackend::new()), true);
    }

    #[cfg(target_os = "macos")]
    {
        if KqueueBackend::is_available() {
            match KqueueBackend::new() {
                Ok(backend) => {
                    info!("Using kqueue backend for efficient event notification");
                    return (Arc::new(backend), false);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        platform = "macos",
                        native_backend = "kqueue",
                        "Native kqueue backend failed to initialize, falling back to portable backend. \
                         This may result in reduced I/O performance. Error: {}",
                        e
                    );
                }
            }
        } else {
            warn!(
                platform = "macos",
                native_backend = "kqueue",
                "kqueue not available on this system, falling back to portable backend"
            );
        }

        info!("Using fallback (tokio) backend on macOS");
        return (Arc::new(FallbackBackend::new()), true);
    }

    #[cfg(target_os = "windows")]
    {
        if IocpBackend::is_available() {
            match IocpBackend::new(num_cpus::get()) {
                Ok(backend) => {
                    info!("Using IOCP backend for high-performance async I/O");
                    return (Arc::new(backend), false);
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        platform = "windows",
                        native_backend = "iocp",
                        "Native IOCP backend failed to initialize, falling back to portable backend. \
                         This may result in reduced I/O performance. Error: {}",
                        e
                    );
                }
            }
        } else {
            warn!(
                platform = "windows",
                native_backend = "iocp",
                "IOCP not available on this system, falling back to portable backend"
            );
        }

        info!("Using fallback (tokio) backend on Windows");
        (Arc::new(FallbackBackend::new()), true)
    }

    // For unknown platforms, use fallback without warning (expected behavior)
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        info!("Using fallback (tokio) backend for unknown platform");
        (Arc::new(FallbackBackend::new()), false)
    }
}

/// Get information about the current platform and I/O backend.
///
/// Reserved for public API - will be exposed in v0.3.0 for platform diagnostics.
#[allow(dead_code)]
pub fn get_platform_info() -> PlatformInfo {
    let backend = detect_active_backend();
    PlatformInfo::current(backend)
}

/// Detect which backend would be active on this platform.
///
/// Reserved for public API - will be exposed in v0.3.0 for platform diagnostics.
#[allow(dead_code)]
fn detect_active_backend() -> IoBackend {
    #[cfg(target_os = "linux")]
    {
        if IoUringBackend::is_available() {
            return IoBackend::IoUring;
        }
    }

    #[cfg(target_os = "macos")]
    {
        if KqueueBackend::is_available() {
            return IoBackend::Kqueue;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if IocpBackend::is_available() {
            return IoBackend::Iocp;
        }
    }

    IoBackend::Fallback
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_platform_io() {
        let io = create_platform_io();
        // Should always succeed and return a valid backend
        let name = io.backend_name();
        assert!(!name.is_empty());
    }

    #[tokio::test]
    async fn test_create_platform_io_with_fallback_tracking() {
        let (io, did_fallback) = create_platform_io_with_fallback_tracking();

        // Should always succeed and return a valid backend
        let name = io.backend_name();
        assert!(!name.is_empty());

        // The backend should be one of the known backends
        assert!(
            name == "io_uring" || name == "kqueue" || name == "iocp" || name == "fallback",
            "Unexpected backend name: {}",
            name
        );

        // If we got fallback backend, did_fallback should be true (on known platforms)
        // If we got a native backend, did_fallback should be false
        if name == "fallback" {
            // On known platforms (Linux, macOS, Windows), fallback means native failed
            #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
            assert!(
                did_fallback,
                "Expected did_fallback=true when using fallback backend on known platform"
            );
        } else {
            assert!(!did_fallback, "Expected did_fallback=false when using native backend");
        }
    }

    #[tokio::test]
    async fn test_fallback_backend_works_correctly() {
        // Verify that the fallback backend can perform basic I/O operations
        let fallback = FallbackBackend::new();
        let temp_dir = tempfile::tempdir().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // Write some data
        let test_data = b"Hello, fallback backend!";
        fallback.write_all(&test_file, test_data).await.unwrap();

        // Read it back
        let read_data = fallback.read_all(&test_file).await.unwrap();
        assert_eq!(read_data, test_data);
    }

    #[test]
    fn test_get_platform_info() {
        let info = get_platform_info();
        assert_eq!(info.platform, Platform::current());
    }

    #[test]
    fn test_detect_active_backend() {
        let backend = detect_active_backend();
        // On any platform, we should get a valid backend
        let name = backend.name();
        assert!(!name.is_empty());
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::tempdir;

    // Feature: platform-native-io-hardening, Property 1: Platform Detection Correctness
    // For any execution of the platform detection logic, the detected platform SHALL
    // match the actual operating system the code is running on.
    // **Validates: Requirements 1.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_platform_detection_correctness(_dummy in 0..100u32) {
            // The platform detection should always return the correct platform
            let detected = Platform::current();

            #[cfg(target_os = "linux")]
            prop_assert_eq!(detected, Platform::Linux);

            #[cfg(target_os = "macos")]
            prop_assert_eq!(detected, Platform::MacOS);

            #[cfg(target_os = "windows")]
            prop_assert_eq!(detected, Platform::Windows);

            // Platform name should be consistent
            let name = detected.name();
            prop_assert!(!name.is_empty());

            #[cfg(target_os = "linux")]
            prop_assert_eq!(name, "linux");

            #[cfg(target_os = "macos")]
            prop_assert_eq!(name, "macos");

            #[cfg(target_os = "windows")]
            prop_assert_eq!(name, "windows");
        }

        #[test]
        fn prop_backend_detection_consistency(_dummy in 0..100u32) {
            // Backend detection should be consistent across calls
            let backend1 = detect_active_backend();
            let backend2 = detect_active_backend();

            prop_assert_eq!(backend1, backend2);

            // Backend name should be non-empty
            prop_assert!(!backend1.name().is_empty());
        }

        #[test]
        fn prop_platform_info_consistency(_dummy in 0..100u32) {
            // Platform info should be consistent
            let info1 = get_platform_info();
            let info2 = get_platform_info();

            prop_assert_eq!(info1.platform, info2.platform);
            prop_assert_eq!(info1.backend, info2.backend);
        }
    }

    // Feature: platform-native-io-hardening, Property 2: Fallback Behavior Guarantee
    // For any platform where the native I/O backend initialization fails or is unavailable,
    // the system SHALL successfully fall back to the tokio-based backend and all I/O
    // operations SHALL complete successfully.
    // **Validates: Requirements 1.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_fallback_always_available(_dummy in 0..100u32) {
            // Fallback backend should always be available
            prop_assert!(FallbackBackend::is_available());
        }

        #[test]
        fn prop_create_platform_io_never_fails(_dummy in 0..100u32) {
            // create_platform_io should never fail - it always falls back to tokio
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let io = create_platform_io();
                // Should always return a valid backend
                let name = io.backend_name();
                prop_assert!(!name.is_empty());

                // The backend should be one of the known backends
                prop_assert!(
                    name == "io_uring" || name == "kqueue" || name == "iocp" || name == "fallback"
                );

                Ok(())
            })?;
        }
    }

    // ========================================================================
    // Feature: forge-production-ready, Property 6: Backend Fallback
    // For any platform where the native I/O backend fails to initialize, the
    // system SHALL fall back to the portable backend and continue operating
    // correctly.
    // **Validates: Requirements 11.4**
    // ========================================================================
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: forge-production-ready, Property 6: Backend Fallback
        /// Verifies that the fallback backend is always available regardless of
        /// platform or native backend status.
        /// **Validates: Requirements 11.4**
        #[test]
        fn prop_fallback_backend_always_available(_dummy in 0..100u32) {
            // The fallback backend must ALWAYS be available on any platform
            prop_assert!(
                FallbackBackend::is_available(),
                "Fallback backend must always be available"
            );

            // Creating a fallback backend should never fail
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let fallback = FallbackBackend::new();
                prop_assert_eq!(
                    fallback.backend_name(),
                    "fallback",
                    "Fallback backend should identify itself correctly"
                );
                Ok(())
            })?;
        }

        /// Feature: forge-production-ready, Property 6: Backend Fallback
        /// Verifies that create_platform_io() never fails and always returns a
        /// valid, operational backend.
        /// **Validates: Requirements 11.4**
        #[test]
        fn prop_create_platform_io_always_succeeds(_dummy in 0..100u32) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // create_platform_io should NEVER fail - it must always return a valid backend
                let io = create_platform_io();

                // Backend name must be non-empty
                let name = io.backend_name();
                prop_assert!(
                    !name.is_empty(),
                    "Backend name must not be empty"
                );

                // Backend must be one of the known types
                prop_assert!(
                    name == "io_uring" || name == "kqueue" || name == "iocp" || name == "fallback",
                    "Backend must be a known type, got: {}",
                    name
                );

                Ok(())
            })?;
        }

        /// Feature: forge-production-ready, Property 6: Backend Fallback
        /// Verifies that the fallback tracking function correctly reports when
        /// fallback was used.
        /// **Validates: Requirements 11.4**
        #[test]
        fn prop_fallback_tracking_correctness(_dummy in 0..100u32) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let (io, did_fallback) = create_platform_io_with_fallback_tracking();
                let name = io.backend_name();

                // If we got the fallback backend, did_fallback should be true on known platforms
                if name == "fallback" {
                    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
                    prop_assert!(
                        did_fallback,
                        "did_fallback should be true when using fallback backend on known platform"
                    );
                } else {
                    // If we got a native backend, did_fallback should be false
                    prop_assert!(
                        !did_fallback,
                        "did_fallback should be false when using native backend ({})",
                        name
                    );
                }

                Ok(())
            })?;
        }

        /// Feature: forge-production-ready, Property 6: Backend Fallback
        /// Verifies that when fallback occurs, all I/O operations still work
        /// correctly with random file content.
        /// **Validates: Requirements 11.4**
        #[test]
        fn prop_fallback_io_operations_work(
            content in prop::collection::vec(any::<u8>(), 0..5000)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let test_file = temp_dir.path().join("fallback_test.bin");

                // Use the fallback backend directly to verify it works
                let fallback = FallbackBackend::new();

                // Write operation should succeed
                fallback.write_all(&test_file, &content).await
                    .expect("Fallback write_all should succeed");

                // Read operation should succeed and return the same content
                let read_content = fallback.read_all(&test_file).await
                    .expect("Fallback read_all should succeed");

                prop_assert_eq!(
                    content,
                    read_content,
                    "Fallback backend should correctly round-trip data"
                );

                Ok(())
            })?;
        }

        /// Feature: forge-production-ready, Property 6: Backend Fallback
        /// Verifies that batch operations work correctly on the fallback backend
        /// with random file contents.
        /// **Validates: Requirements 11.4**
        #[test]
        fn prop_fallback_batch_operations_work(
            file_contents in prop::collection::vec(
                prop::collection::vec(any::<u8>(), 0..1000),
                1..5
            )
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let fallback = FallbackBackend::new();

                // Create write operations
                let ops: Vec<super::super::WriteOp> = file_contents
                    .iter()
                    .enumerate()
                    .map(|(i, content)| {
                        super::super::WriteOp::new(
                            temp_dir.path().join(format!("batch_file_{}.bin", i)),
                            content.clone(),
                        )
                    })
                    .collect();

                // Batch write should succeed
                fallback.batch_write(&ops).await
                    .expect("Fallback batch_write should succeed");

                // Batch read should succeed
                let paths: Vec<std::path::PathBuf> = ops.iter().map(|op| op.path.clone()).collect();
                let read_contents = fallback.batch_read(&paths).await
                    .expect("Fallback batch_read should succeed");

                // Verify all contents match
                prop_assert_eq!(
                    read_contents.len(),
                    file_contents.len(),
                    "Batch read should return same number of files"
                );

                for (original, read) in file_contents.iter().zip(read_contents.iter()) {
                    prop_assert_eq!(
                        original,
                        read,
                        "Batch operation should correctly round-trip data"
                    );
                }

                Ok(())
            })?;
        }

        /// Feature: forge-production-ready, Property 6: Backend Fallback
        /// Verifies that the platform I/O backend (whether native or fallback)
        /// correctly performs I/O operations.
        /// **Validates: Requirements 11.4**
        #[test]
        fn prop_platform_io_operations_work(
            content in prop::collection::vec(any::<u8>(), 0..5000)
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let test_file = temp_dir.path().join("platform_io_test.bin");

                // Get the platform I/O backend (may be native or fallback)
                let io = create_platform_io();

                // Write operation should succeed
                io.write_all(&test_file, &content).await
                    .expect("Platform I/O write_all should succeed");

                // Read operation should succeed and return the same content
                let read_content = io.read_all(&test_file).await
                    .expect("Platform I/O read_all should succeed");

                prop_assert_eq!(
                    content,
                    read_content,
                    "Platform I/O backend should correctly round-trip data"
                );

                Ok(())
            })?;
        }
    }
}
