//! Property tests for Node.js compatibility modules.
//!
//! This file contains property-based tests for the dx-compat-node crate,
//! validating correctness properties defined in the design document.

#[cfg(feature = "node-core")]
mod tests {
    use dx_js_compatibility::node::{buffer, events, fs, path};
    use proptest::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    // =========================================================================
    // Property 1: File System Read/Write Round-Trip
    // Validates: Requirements 2.1, 2.2
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 1: For any byte sequence, write then read SHALL produce the original data.
        #[test]
        fn fs_read_write_round_trip(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let dir = tempdir().unwrap();
                let file_path = dir.path().join("test_file.bin");

                fs::write_file(&file_path, &data).await.unwrap();
                let read_data = fs::read_file(&file_path).await.unwrap();

                prop_assert_eq!(&data[..], &read_data[..], "Read data should match written data");
                Ok(())
            });
            result?;
        }

        /// Property 1: Sync variant - write then read SHALL produce the original data.
        #[test]
        fn fs_read_write_round_trip_sync(data in prop::collection::vec(any::<u8>(), 0..10000)) {
            let dir = tempdir().unwrap();
            let file_path = dir.path().join("test_file_sync.bin");

            fs::sync::write_file_sync(&file_path, &data).unwrap();
            let read_data = fs::sync::read_file_sync(&file_path).unwrap();

            prop_assert_eq!(&data[..], &read_data[..], "Sync read data should match written data");
        }
    }

    // =========================================================================
    // Property 4: Buffer Encoding Round-Trip
    // Validates: Requirements 4.2, 4.7
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 4: UTF-8 round-trip should preserve string content.
        #[test]
        fn buffer_utf8_round_trip(s in "\\PC{0,1000}") {
            let buf = buffer::Buffer::from_string(&s, buffer::Encoding::Utf8);
            let result = buf.to_string(buffer::Encoding::Utf8);
            prop_assert_eq!(s, result, "UTF-8 round-trip should preserve string content");
        }

        /// Property 4: Hex round-trip should preserve byte content.
        #[test]
        fn buffer_hex_round_trip(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let buf = buffer::Buffer::from_vec(data.clone());
            let hex_str = buf.to_string(buffer::Encoding::Hex);
            let buf2 = buffer::Buffer::from_string(&hex_str, buffer::Encoding::Hex);
            prop_assert_eq!(&data[..], buf2.as_bytes(), "Hex round-trip should preserve byte content");
        }

        /// Property 4: Base64 round-trip should preserve byte content.
        #[test]
        fn buffer_base64_round_trip(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            let buf = buffer::Buffer::from_vec(data.clone());
            let base64_str = buf.to_string(buffer::Encoding::Base64);
            let buf2 = buffer::Buffer::from_string(&base64_str, buffer::Encoding::Base64);
            prop_assert_eq!(&data[..], buf2.as_bytes(), "Base64 round-trip should preserve byte content");
        }
    }

    // =========================================================================
    // Property 10: Event Emitter Listener Invocation
    // Validates: Requirements 6.1, 6.2, 6.3
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 10: For N listeners, emit SHALL invoke all N listeners exactly once.
        #[test]
        fn event_emitter_invokes_all_listeners(n_listeners in 1usize..20) {
            let emitter = events::EventEmitter::new();
            let counters: Vec<Arc<AtomicUsize>> = (0..n_listeners)
                .map(|_| Arc::new(AtomicUsize::new(0)))
                .collect();

            for counter in &counters {
                let counter_clone = Arc::clone(counter);
                emitter.on("test", Box::new(move |_| {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }));
            }

            let result = emitter.emit("test", &[]);
            prop_assert!(result, "emit should return true when listeners exist");

            for (i, counter) in counters.iter().enumerate() {
                prop_assert_eq!(
                    counter.load(Ordering::SeqCst),
                    1,
                    "Listener {} should be called exactly once",
                    i
                );
            }
        }

        /// Property 10: once() listeners SHALL be invoked exactly once, then removed.
        #[test]
        fn event_emitter_once_invoked_once(n_emits in 1usize..10) {
            let emitter = events::EventEmitter::new();
            let counter = Arc::new(AtomicUsize::new(0));
            let counter_clone = Arc::clone(&counter);

            emitter.once("test", Box::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }));

            for _ in 0..n_emits {
                emitter.emit("test", &[]);
            }

            prop_assert_eq!(
                counter.load(Ordering::SeqCst),
                1,
                "once listener should be called exactly once regardless of emit count"
            );
        }
    }

    // =========================================================================
    // Property 11: Path Operations Correctness
    // Validates: Requirements 3.1, 3.2, 3.6, 3.7
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 11: path.resolve() SHALL produce an absolute path.
        #[test]
        fn path_resolve_produces_absolute(
            segments in prop::collection::vec("[a-zA-Z0-9_]{1,20}", 1..5)
        ) {
            let segment_refs: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();
            let resolved = path::resolve(&segment_refs);
            let resolved_str = resolved.to_string_lossy();

            prop_assert!(
                path::is_absolute(&resolved_str),
                "path.resolve() should produce an absolute path, got: {}",
                resolved_str
            );
        }

        /// Property 11: normalize is idempotent.
        #[test]
        fn path_normalize_is_idempotent(
            segments in prop::collection::vec("[a-zA-Z0-9_]{1,10}", 1..5)
        ) {
            let segment_refs: Vec<&str> = segments.iter().map(|s| s.as_str()).collect();
            let joined = path::join(&segment_refs);
            let joined_str = joined.to_string_lossy();

            let normalized_once = path::normalize(&joined_str);
            let normalized_twice = path::normalize(&normalized_once.to_string_lossy());

            prop_assert_eq!(normalized_once, normalized_twice, "normalize should be idempotent");
        }
    }

    // =========================================================================
    // Property 19: Error Code Correctness
    // Validates: Requirements 29.1, 29.4, 29.5
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 19: ENOENT for non-existent paths.
        #[test]
        fn error_code_enoent_for_nonexistent_path(
            filename in "[a-zA-Z0-9_]{1,20}\\.(txt|json|rs|md)"
        ) {
            use dx_js_compatibility::node::error::ErrorCode;

            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let nonexistent_path = format!("/nonexistent_dir_12345/{}", filename);
                let read_result = fs::read_file(&nonexistent_path).await;

                prop_assert!(read_result.is_err(), "Expected error for non-existent path");
                let err = read_result.unwrap_err();
                prop_assert_eq!(
                    err.code,
                    ErrorCode::ENOENT,
                    "Expected ENOENT error code for non-existent path, got {:?}",
                    err.code
                );
                Ok(())
            });
            result?;
        }
    }
}
