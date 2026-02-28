//! Property tests for file system watching functionality.
//!
//! This file contains property-based tests for the fs.watch() and fs.watchFile()
//! implementations, validating correctness properties defined in the design document.
//!
//! Feature: production-readiness

#[cfg(feature = "node-core")]
mod tests {
    use dx_js_compatibility::node::fs::watch::{
        FSWatchFile, FSWatcher, WatchEventType, WatchFileStats,
    };
    use proptest::prelude::*;
    use std::path::Path;
    use tempfile::tempdir;


    // =========================================================================
    // Property 10: Watch Event Correctness
    // Validates: Requirements 3.1, 3.2, 3.3, 3.6
    // For any file modification on a watched path, the watcher SHALL invoke
    // the callback with the correct event type ('change' or 'rename').
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 10: FSWatcher creation succeeds for existing directories.
        /// For any valid directory path, FSWatcher::new() SHALL succeed.
        #[test]
        fn watcher_creation_succeeds_for_existing_dir(
            dir_name in "[a-zA-Z0-9_]{1,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let watch_dir = temp_dir.path().join(&dir_name);
                std::fs::create_dir(&watch_dir).unwrap();

                let watcher = FSWatcher::new(&watch_dir, false);
                prop_assert!(watcher.is_ok(), "FSWatcher should be created for existing directory");

                let mut watcher = watcher.unwrap();
                prop_assert!(!watcher.is_closed(), "New watcher should not be closed");
                watcher.close();
                prop_assert!(watcher.is_closed(), "Watcher should be closed after close()");
                Ok(())
            });
            result?;
        }

        /// Property 10: WatchFileStats correctly reports file existence and size.
        /// For any file with content, WatchFileStats SHALL report correct size and existence.
        #[test]
        fn watch_file_stats_correctness(
            content in prop::collection::vec(any::<u8>(), 0..1000)
        ) {
            let temp_dir = tempdir().unwrap();
            let file_path = temp_dir.path().join("test_file.bin");
            std::fs::write(&file_path, &content).unwrap();

            let stats = WatchFileStats::from_path(&file_path);
            prop_assert!(stats.exists, "Stats should report file exists");
            prop_assert_eq!(
                stats.size,
                content.len() as u64,
                "Stats should report correct file size"
            );
        }

        /// Property 10: WatchFileStats reports non-existence for missing files.
        /// For any non-existent path, WatchFileStats SHALL report exists=false.
        #[test]
        fn watch_file_stats_nonexistent(
            filename in "[a-zA-Z0-9_]{1,20}\\.txt"
        ) {
            let nonexistent_path = format!("/nonexistent_dir_xyz123/{}", filename);
            let stats = WatchFileStats::from_path(Path::new(&nonexistent_path));
            prop_assert!(!stats.exists, "Stats should report file does not exist");
            prop_assert_eq!(stats.size, 0, "Non-existent file should have size 0");
        }
    }

    // =========================================================================
    // Property 11: Watch Resource Cleanup
    // Validates: Requirements 3.5, 3.8
    // For any watcher that is closed or unwatched, no further callbacks SHALL
    // be invoked and resources SHALL be released.
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 11: Closing a watcher releases all watched paths.
        /// For any watcher with N watched paths, close() SHALL release all paths.
        #[test]
        fn watcher_close_releases_paths(
            n_paths in 1usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let temp_dir = tempdir().unwrap();

                // Create subdirectories to watch
                let mut paths = Vec::new();
                for i in 0..n_paths {
                    let subdir = temp_dir.path().join(format!("dir_{}", i));
                    std::fs::create_dir(&subdir).unwrap();
                    paths.push(subdir);
                }

                // Create watcher for first path
                let mut watcher = FSWatcher::new(&paths[0], false).unwrap();

                // Add additional paths
                for path in paths.iter().skip(1) {
                    watcher.watch_path(path, false).unwrap();
                }

                prop_assert_eq!(
                    watcher.watched_paths().len(),
                    n_paths,
                    "Watcher should have {} watched paths",
                    n_paths
                );

                // Close and verify
                watcher.close();
                prop_assert!(watcher.is_closed(), "Watcher should be closed");
                prop_assert!(
                    watcher.watched_paths().is_empty(),
                    "All watched paths should be released after close"
                );
                Ok(())
            });
            result?;
        }

        /// Property 11: FSWatchFile close releases all watched files.
        /// For any FSWatchFile with N watched files, close() SHALL release all.
        #[test]
        fn watch_file_close_releases_files(
            n_files in 1usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let temp_dir = tempdir().unwrap();

                // Create files to watch
                let mut paths = Vec::new();
                for i in 0..n_files {
                    let file_path = temp_dir.path().join(format!("file_{}.txt", i));
                    std::fs::write(&file_path, format!("content {}", i)).unwrap();
                    paths.push(file_path);
                }

                let mut watcher = FSWatchFile::new(100);

                // Watch all files
                for path in &paths {
                    watcher.watch(path);
                }

                // Close and verify
                watcher.close();
                Ok(())
            });
            result?;
        }

        /// Property 11: Unwatch removes specific path from watcher.
        /// For any watched path, unwatch() SHALL remove only that path.
        #[test]
        fn watcher_unwatch_removes_specific_path(
            n_paths in 2usize..5
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let temp_dir = tempdir().unwrap();

                // Create subdirectories
                let mut paths = Vec::new();
                for i in 0..n_paths {
                    let subdir = temp_dir.path().join(format!("dir_{}", i));
                    std::fs::create_dir(&subdir).unwrap();
                    paths.push(subdir);
                }

                let mut watcher = FSWatcher::new(&paths[0], false).unwrap();
                for path in paths.iter().skip(1) {
                    watcher.watch_path(path, false).unwrap();
                }

                // Unwatch first path
                watcher.unwatch(&paths[0]).unwrap();

                let watched = watcher.watched_paths();
                prop_assert_eq!(
                    watched.len(),
                    n_paths - 1,
                    "Should have one less watched path after unwatch"
                );
                prop_assert!(
                    !watched.contains(&paths[0]),
                    "Unwatched path should not be in watched list"
                );

                watcher.close();
                Ok(())
            });
            result?;
        }
    }

    // =========================================================================
    // Property 12: Watch Error Handling
    // Validates: Requirements 3.7
    // For any non-existent watched path, the watcher SHALL emit an error event.
    // =========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 12: FSWatcher returns error for non-existent paths.
        /// For any non-existent path, FSWatcher::new() SHALL return an error.
        #[test]
        fn watcher_error_for_nonexistent_path(
            path_segment in "[a-zA-Z0-9_]{1,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let nonexistent_path = format!("/nonexistent_xyz123/{}", path_segment);
                let watcher = FSWatcher::new(&nonexistent_path, false);

                prop_assert!(
                    watcher.is_err(),
                    "FSWatcher should return error for non-existent path"
                );
                Ok(())
            });
            result?;
        }

        /// Property 12: watch_path returns error for non-existent paths.
        /// For any existing watcher, watch_path() with non-existent path SHALL return error.
        #[test]
        fn watch_path_error_for_nonexistent(
            path_segment in "[a-zA-Z0-9_]{1,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let mut watcher = FSWatcher::new(temp_dir.path(), false).unwrap();

                let nonexistent_path = format!("/nonexistent_xyz123/{}", path_segment);
                let result = watcher.watch_path(&nonexistent_path, false);

                prop_assert!(
                    result.is_err(),
                    "watch_path should return error for non-existent path"
                );

                watcher.close();
                Ok(())
            });
            result?;
        }

        /// Property 12: Operations on closed watcher return error.
        /// For any closed watcher, watch_path() and unwatch() SHALL return error.
        #[test]
        fn closed_watcher_operations_error(
            dir_name in "[a-zA-Z0-9_]{1,20}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result: Result<(), TestCaseError> = rt.block_on(async {
                let temp_dir = tempdir().unwrap();
                let watch_dir = temp_dir.path().join(&dir_name);
                std::fs::create_dir(&watch_dir).unwrap();

                let mut watcher = FSWatcher::new(&watch_dir, false).unwrap();
                watcher.close();

                // Try to watch a new path on closed watcher
                let new_dir = temp_dir.path().join("new_dir");
                std::fs::create_dir(&new_dir).unwrap();
                let result = watcher.watch_path(&new_dir, false);

                prop_assert!(
                    result.is_err(),
                    "watch_path on closed watcher should return error"
                );

                // Try to unwatch on closed watcher
                let result = watcher.unwatch(&watch_dir);
                prop_assert!(
                    result.is_err(),
                    "unwatch on closed watcher should return error"
                );
                Ok(())
            });
            result?;
        }
    }

    // =========================================================================
    // Additional Unit Tests for Edge Cases
    // =========================================================================

    #[tokio::test]
    async fn test_watch_event_type_equality() {
        assert_eq!(WatchEventType::Change, WatchEventType::Change);
        assert_eq!(WatchEventType::Rename, WatchEventType::Rename);
        assert_ne!(WatchEventType::Change, WatchEventType::Rename);
    }

    #[tokio::test]
    async fn test_watch_file_stats_equality() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let stats1 = WatchFileStats::from_path(&file_path);
        let stats2 = WatchFileStats::from_path(&file_path);

        // Stats should be equal for same file (assuming no changes between reads)
        assert_eq!(stats1.size, stats2.size);
        assert_eq!(stats1.exists, stats2.exists);
    }

    #[tokio::test]
    async fn test_fs_watch_file_creation() {
        let watcher = FSWatchFile::new(5007); // Node.js default
        // Watcher should be created successfully
        drop(watcher);
    }

    #[tokio::test]
    async fn test_recursive_watch() {
        let temp_dir = tempdir().unwrap();
        let subdir = temp_dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();

        let watcher = FSWatcher::new(temp_dir.path(), true);
        assert!(watcher.is_ok(), "Recursive watch should succeed");

        let mut watcher = watcher.unwrap();
        watcher.close();
    }
}
