//! Property tests for CSS file discovery
//!
//! Feature: dx-www-production-ready, Property 3: CSS File Discovery
//!
//! This property test verifies that the StyleProcessor correctly discovers and processes
//! all CSS files in the www/styles/ directory. The property tests that:
//! 1. For any set of CSS files in a directory, all files are discovered
//! 2. CSS files in nested subdirectories are discovered
//! 3. Non-CSS files are not discovered
//! 4. Empty directories return no files

use build::{StyleConfig, StyleProcessor};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Strategy for generating arbitrary CSS content
fn arbitrary_css_content() -> impl Strategy<Value = String> {
    prop::string::string_regex("(body|div|span) \\{ [a-z-]+: [a-z0-9]+; \\}").unwrap()
}

/// Strategy for generating arbitrary CSS file names
fn arbitrary_css_filename() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]{3,10}\\.css").unwrap()
}

/// Strategy for generating arbitrary non-CSS file names
fn arbitrary_non_css_filename() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]{3,10}\\.(js|txt|md|json)").unwrap()
}

/// Strategy for generating arbitrary subdirectory names
fn arbitrary_subdir_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]{3,10}").unwrap()
}

/// Strategy for generating a set of CSS files (1-10 files)
fn arbitrary_css_files() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arbitrary_css_filename(), 1..=10)
}

/// Strategy for generating a set of non-CSS files (0-5 files)
fn arbitrary_non_css_files() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arbitrary_non_css_filename(), 0..=5)
}

proptest! {
    /// Property 3a: All CSS files in flat directory are discovered
    ///
    /// For any set of CSS files in a flat directory structure, the StyleProcessor
    /// should discover all of them.
    ///
    /// **Validates: Requirements 1.2**
    #[test]
    fn discovers_all_css_files_in_flat_directory(
        css_files in arbitrary_css_files(),
        css_content in arbitrary_css_content(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        fs::create_dir(&styles_dir).unwrap();

        // Create CSS files
        let mut expected_files = HashSet::new();
        for filename in &css_files {
            let file_path = styles_dir.join(filename);
            fs::write(&file_path, &css_content).unwrap();
            expected_files.insert(file_path);
        }

        // Create StyleProcessor
        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };
        let processor = StyleProcessor::new(config);

        // Discover CSS files
        let discovered = processor.discover_css_files().unwrap();
        let discovered_set: HashSet<PathBuf> = discovered.into_iter().collect();

        // Verify all expected files were discovered
        prop_assert_eq!(
            discovered_set.len(),
            expected_files.len(),
            "Should discover exactly {} CSS files",
            expected_files.len()
        );

        for expected_file in &expected_files {
            prop_assert!(
                discovered_set.contains(expected_file),
                "Should discover file: {:?}",
                expected_file
            );
        }
    }

    /// Property 3b: CSS files in nested subdirectories are discovered
    ///
    /// For any set of CSS files in nested subdirectories, the StyleProcessor
    /// should discover all of them.
    ///
    /// **Validates: Requirements 1.2**
    #[test]
    fn discovers_css_files_in_nested_directories(
        subdir_name in arbitrary_subdir_name(),
        css_files in arbitrary_css_files(),
        css_content in arbitrary_css_content(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        fs::create_dir(&styles_dir).unwrap();

        // Create nested subdirectory
        let nested_dir = styles_dir.join(&subdir_name);
        fs::create_dir(&nested_dir).unwrap();

        // Create CSS files in nested directory
        let mut expected_files = HashSet::new();
        for filename in &css_files {
            let file_path = nested_dir.join(filename);
            fs::write(&file_path, &css_content).unwrap();
            expected_files.insert(file_path);
        }

        // Create StyleProcessor
        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };
        let processor = StyleProcessor::new(config);

        // Discover CSS files
        let discovered = processor.discover_css_files().unwrap();
        let discovered_set: HashSet<PathBuf> = discovered.into_iter().collect();

        // Verify all expected files were discovered
        prop_assert_eq!(
            discovered_set.len(),
            expected_files.len(),
            "Should discover exactly {} CSS files in nested directory",
            expected_files.len()
        );

        for expected_file in &expected_files {
            prop_assert!(
                discovered_set.contains(expected_file),
                "Should discover nested file: {:?}",
                expected_file
            );
        }
    }

    /// Property 3c: Non-CSS files are not discovered
    ///
    /// For any set of non-CSS files in the directory, the StyleProcessor
    /// should not discover them.
    ///
    /// **Validates: Requirements 1.2**
    #[test]
    fn does_not_discover_non_css_files(
        css_files in arbitrary_css_files(),
        non_css_files in arbitrary_non_css_files(),
        css_content in arbitrary_css_content(),
        non_css_content in prop::string::string_regex(".{0,100}").unwrap(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        fs::create_dir(&styles_dir).unwrap();

        // Create CSS files
        let mut expected_css_files = HashSet::new();
        for filename in &css_files {
            let file_path = styles_dir.join(filename);
            fs::write(&file_path, &css_content).unwrap();
            expected_css_files.insert(file_path);
        }

        // Create non-CSS files
        for filename in &non_css_files {
            let file_path = styles_dir.join(filename);
            fs::write(&file_path, &non_css_content).unwrap();
        }

        // Create StyleProcessor
        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };
        let processor = StyleProcessor::new(config);

        // Discover CSS files
        let discovered = processor.discover_css_files().unwrap();
        let discovered_set: HashSet<PathBuf> = discovered.into_iter().collect();

        // Verify only CSS files were discovered
        prop_assert_eq!(
            discovered_set.len(),
            expected_css_files.len(),
            "Should discover only CSS files, not non-CSS files"
        );

        for expected_file in &expected_css_files {
            prop_assert!(
                discovered_set.contains(expected_file),
                "Should discover CSS file: {:?}",
                expected_file
            );
        }

        // Verify no non-CSS files were discovered
        for discovered_file in &discovered_set {
            prop_assert!(
                discovered_file.extension().and_then(|s| s.to_str()) == Some("css"),
                "Discovered file should have .css extension: {:?}",
                discovered_file
            );
        }
    }

    /// Property 3d: Empty directory returns no files
    ///
    /// For an empty directory, the StyleProcessor should return an empty list.
    ///
    /// **Validates: Requirements 1.2**
    #[test]
    fn empty_directory_returns_no_files(_dummy in 0..1u8) {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        fs::create_dir(&styles_dir).unwrap();

        // Create StyleProcessor
        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };
        let processor = StyleProcessor::new(config);

        // Discover CSS files
        let discovered = processor.discover_css_files().unwrap();

        // Verify no files were discovered
        prop_assert_eq!(
            discovered.len(),
            0,
            "Empty directory should return no CSS files"
        );
    }

    /// Property 3e: Mixed flat and nested CSS files are all discovered
    ///
    /// For any combination of CSS files in both the root directory and nested
    /// subdirectories, all files should be discovered.
    ///
    /// **Validates: Requirements 1.2**
    #[test]
    fn discovers_mixed_flat_and_nested_css_files(
        root_css_files in prop::collection::vec(arbitrary_css_filename(), 1..=5),
        subdir_name in arbitrary_subdir_name(),
        nested_css_files in prop::collection::vec(arbitrary_css_filename(), 1..=5),
        css_content in arbitrary_css_content(),
    ) {
        let temp_dir = TempDir::new().unwrap();
        let styles_dir = temp_dir.path().join("styles");
        fs::create_dir(&styles_dir).unwrap();

        // Create nested subdirectory
        let nested_dir = styles_dir.join(&subdir_name);
        fs::create_dir(&nested_dir).unwrap();

        // Create CSS files in root directory
        let mut expected_files = HashSet::new();
        for filename in &root_css_files {
            let file_path = styles_dir.join(filename);
            fs::write(&file_path, &css_content).unwrap();
            expected_files.insert(file_path);
        }

        // Create CSS files in nested directory
        for filename in &nested_css_files {
            let file_path = nested_dir.join(filename);
            fs::write(&file_path, &css_content).unwrap();
            expected_files.insert(file_path);
        }

        // Create StyleProcessor
        let config = StyleConfig {
            input_dir: styles_dir,
            output_dir: temp_dir.path().join("dist"),
            ..Default::default()
        };
        let processor = StyleProcessor::new(config);

        // Discover CSS files
        let discovered = processor.discover_css_files().unwrap();
        let discovered_set: HashSet<PathBuf> = discovered.into_iter().collect();

        // Verify all expected files were discovered
        prop_assert_eq!(
            discovered_set.len(),
            expected_files.len(),
            "Should discover all CSS files in both root and nested directories"
        );

        for expected_file in &expected_files {
            prop_assert!(
                discovered_set.contains(expected_file),
                "Should discover file: {:?}",
                expected_file
            );
        }
    }
}
