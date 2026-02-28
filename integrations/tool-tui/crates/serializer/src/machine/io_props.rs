//! Property tests for async I/O operations
//!
//! These tests validate the correctness properties defined in the design document
//! for batch file operations.

#[cfg(test)]
#[cfg(feature = "async-io")]
mod property_tests {
    use crate::io::{blocking::BlockingIO, create_async_io, AsyncFileIO};
    use proptest::prelude::*;
    use std::path::Path;
    use tempfile::TempDir;

    // ========================================================================
    // Property 11: Batch File Operations Correctness
    // Validates: Requirements 5.6
    // For any set of files processed in batch, the batch operation SHALL
    // produce the same results as processing each file individually.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Feature: dx-serializer-quantum-entanglement, Property 11: Batch File Operations
        /// Validates: Requirements 5.6
        #[test]
        fn prop_batch_read_equals_individual_reads(
            file_contents in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..500), 1..5)
        ) {
            let temp_dir = TempDir::new().unwrap();
            let io = BlockingIO;

            // Create test files
            let mut paths = Vec::new();
            for (i, content) in file_contents.iter().enumerate() {
                let path = temp_dir.path().join(format!("test_{}.bin", i));
                std::fs::write(&path, content).unwrap();
                paths.push(path);
            }

            // Read individually
            let individual_results: Vec<Vec<u8>> = paths.iter()
                .map(|p| io.read_sync(p).unwrap())
                .collect();

            // Read in batch
            let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
            let batch_results = io.read_batch_sync(&path_refs).unwrap();
            let batch_data: Vec<Vec<u8>> = batch_results.into_iter()
                .map(|r| r.unwrap())
                .collect();

            // Verify equivalence
            prop_assert_eq!(individual_results.len(), batch_data.len(),
                "Batch should return same number of results as individual reads");

            for (i, (individual, batch)) in individual_results.iter().zip(batch_data.iter()).enumerate() {
                prop_assert_eq!(individual, batch,
                    "File {} content should match between individual and batch read", i);
            }
        }

        /// Test batch write equals individual writes
        #[test]
        fn prop_batch_write_equals_individual_writes(
            file_contents in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..500), 1..5)
        ) {
            let temp_dir = TempDir::new().unwrap();
            let io = BlockingIO;

            // Write individually
            let mut individual_paths = Vec::new();
            for (i, content) in file_contents.iter().enumerate() {
                let path = temp_dir.path().join(format!("individual_{}.bin", i));
                io.write_sync(&path, content).unwrap();
                individual_paths.push(path);
            }

            // Write in batch
            let mut batch_paths = Vec::new();
            let _batch_files: Vec<(&Path, &[u8])> = Vec::new();
            for (i, _content) in file_contents.iter().enumerate() {
                let path = temp_dir.path().join(format!("batch_{}.bin", i));
                batch_paths.push(path);
            }

            // Need to create the batch_files after batch_paths is complete
            let batch_files: Vec<(&Path, &[u8])> = batch_paths.iter()
                .zip(file_contents.iter())
                .map(|(p, c)| (p.as_path(), c.as_slice()))
                .collect();

            io.write_batch_sync(&batch_files).unwrap();

            // Verify both methods produce identical files
            for (individual_path, batch_path) in individual_paths.iter().zip(batch_paths.iter()) {
                let individual_content = std::fs::read(individual_path).unwrap();
                let batch_content = std::fs::read(batch_path).unwrap();

                prop_assert_eq!(individual_content, batch_content,
                    "Individual and batch write should produce identical files");
            }
        }

        /// Test batch operations preserve file order
        #[test]
        fn prop_batch_preserves_order(
            file_count in 2usize..6
        ) {
            let temp_dir = TempDir::new().unwrap();
            let io = BlockingIO;

            // Create files with unique content based on index
            let mut paths = Vec::new();
            for i in 0..file_count {
                let path = temp_dir.path().join(format!("order_{}.bin", i));
                let content = format!("FILE_INDEX_{}", i);
                std::fs::write(&path, content.as_bytes()).unwrap();
                paths.push(path);
            }

            // Read in batch
            let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
            let batch_results = io.read_batch_sync(&path_refs).unwrap();

            // Verify order is preserved
            for (i, result) in batch_results.into_iter().enumerate() {
                let content = String::from_utf8(result.unwrap()).unwrap();
                let expected = format!("FILE_INDEX_{}", i);
                prop_assert_eq!(content, expected,
                    "Batch read should preserve file order");
            }
        }
    }

    // ========================================================================
    // Additional unit tests
    // ========================================================================

    #[test]
    fn test_batch_read_empty_list() {
        let io = BlockingIO;
        let paths: Vec<&Path> = vec![];
        let results = io.read_batch_sync(&paths).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_batch_write_empty_list() {
        let io = BlockingIO;
        let files: Vec<(&Path, &[u8])> = vec![];
        let results = io.write_batch_sync(&files).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_batch_read_nonexistent_file() {
        let io = BlockingIO;
        let temp_dir = TempDir::new().unwrap();

        // Create one valid file
        let valid_path = temp_dir.path().join("valid.bin");
        std::fs::write(&valid_path, b"valid content").unwrap();

        // Reference a nonexistent file
        let invalid_path = temp_dir.path().join("nonexistent.bin");

        let paths: Vec<&Path> = vec![valid_path.as_path(), invalid_path.as_path()];
        let results = io.read_batch_sync(&paths).unwrap();

        // First should succeed, second should fail
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
    }

    #[test]
    fn test_create_async_io_returns_valid_backend() {
        let io = create_async_io();
        assert!(!io.backend_name().is_empty());

        // Backend should report availability correctly
        // (it was created, so it should be available)
        println!("Backend: {}, Available: {}", io.backend_name(), io.is_available());
    }

    #[test]
    fn test_round_trip_through_batch_operations() {
        let temp_dir = TempDir::new().unwrap();
        let io = BlockingIO;

        // Original data
        let files_data = vec![
            (temp_dir.path().join("a.bin"), b"content A".to_vec()),
            (temp_dir.path().join("b.bin"), b"content B".to_vec()),
            (temp_dir.path().join("c.bin"), b"content C".to_vec()),
        ];

        // Write in batch
        let write_files: Vec<(&Path, &[u8])> =
            files_data.iter().map(|(p, d)| (p.as_path(), d.as_slice())).collect();
        io.write_batch_sync(&write_files).unwrap();

        // Read in batch
        let read_paths: Vec<&Path> = files_data.iter().map(|(p, _)| p.as_path()).collect();
        let read_results = io.read_batch_sync(&read_paths).unwrap();

        // Verify round-trip
        for ((_, original), read_result) in files_data.iter().zip(read_results.into_iter()) {
            assert_eq!(read_result.unwrap(), *original);
        }
    }
}
