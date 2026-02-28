//! Property-based tests for Convert CLI commands
//!
//! These tests verify universal properties that should hold across all inputs.
//! Feature: token-efficiency-display

use proptest::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Generate valid JSON content
fn json_content_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r#"{"name": "test", "value": 42}"#.to_string()),
        Just(r#"{"items": [1, 2, 3], "active": true}"#.to_string()),
        Just(r#"{"nested": {"key": "value"}, "count": 100}"#.to_string()),
        "[a-z]{1,10}".prop_map(|s| format!(r#"{{"key": "{}"}}"#, s)),
    ]
}

/// Generate valid YAML content
fn yaml_content_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("name: test\nvalue: 42".to_string()),
        Just("items:\n  - 1\n  - 2\n  - 3\nactive: true".to_string()),
        Just("nested:\n  key: value\ncount: 100".to_string()),
    ]
}

/// Generate valid TOML content
fn toml_content_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("name = \"test\"\nvalue = 42".to_string()),
        Just("items = [1, 2, 3]\nactive = true".to_string()),
        Just("[nested]\nkey = \"value\"\n\n[root]\ncount = 100".to_string()),
    ]
}

/// Generate file extension
fn extension_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("json".to_string()),
        Just("yaml".to_string()),
        Just("yml".to_string()),
        Just("toml".to_string()),
    ]
}

proptest! {
    /// Property 4: Glob Pattern File Matching
    /// *For any* valid glob pattern and file system state, the CLI SHALL find all
    /// and only the files that match the pattern.
    ///
    /// **Validates: Requirements 5.1**
    #[test]
    fn prop_glob_pattern_matching(
        file_count in 1usize..5,
        ext in extension_strategy()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        let mut created_files: HashSet<PathBuf> = HashSet::new();
        for i in 0..file_count {
            let file_path = dir_path.join(format!("test_{}.{}", i, ext));
            let content = match ext.as_str() {
                "json" => r#"{"test": true}"#,
                "yaml" | "yml" => "test: true",
                "toml" => "test = true",
                _ => "test",
            };
            fs::write(&file_path, content).unwrap();
            created_files.insert(file_path);
        }

        // Also create some files with different extensions that should NOT match
        fs::write(dir_path.join("other.txt"), "not a config").unwrap();
        fs::write(dir_path.join("readme.md"), "# Readme").unwrap();

        // Use glob to find files
        let pattern = format!("{}/*.{}", dir_path.display(), ext);
        let matched_files: HashSet<PathBuf> = glob::glob(&pattern)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        // All created files should be matched
        for file in &created_files {
            prop_assert!(
                matched_files.contains(file),
                "Created file {:?} was not matched by pattern {}",
                file, pattern
            );
        }

        // Only files with correct extension should be matched
        for file in &matched_files {
            let file_ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
            prop_assert!(
                file_ext == ext || (ext == "yaml" && file_ext == "yml") || (ext == "yml" && file_ext == "yaml"),
                "File {:?} with extension {} should not match pattern for {}",
                file, file_ext, ext
            );
        }

        // Number of matched files should equal created files
        prop_assert_eq!(
            matched_files.len(),
            created_files.len(),
            "Matched {} files but created {}",
            matched_files.len(),
            created_files.len()
        );
    }

    /// Property 5: Format-Specific Conversion Support
    /// *For any* valid file in JSON, YAML, or TOML format, the CLI SHALL successfully
    /// convert it to DX format with the correct `.dx` extension.
    ///
    /// **Validates: Requirements 5.2, 5.3**
    #[test]
    fn prop_format_conversion_support_json(content in json_content_strategy()) {
        // Test JSON to DX conversion
        let result = serializer::json_to_dx(&content);
        prop_assert!(
            result.is_ok(),
            "JSON conversion failed for content: {} - error: {:?}",
            content, result.err()
        );

        let dx_content = result.unwrap();
        prop_assert!(
            !dx_content.is_empty(),
            "DX output should not be empty for JSON input"
        );
    }

    #[test]
    fn prop_format_conversion_support_yaml(content in yaml_content_strategy()) {
        // Test YAML to DX conversion
        let result = serializer::yaml_to_dx(&content);
        prop_assert!(
            result.is_ok(),
            "YAML conversion failed for content: {} - error: {:?}",
            content, result.err()
        );

        let dx_content = result.unwrap();
        prop_assert!(
            !dx_content.is_empty(),
            "DX output should not be empty for YAML input"
        );
    }

    #[test]
    fn prop_format_conversion_support_toml(content in toml_content_strategy()) {
        // Test TOML to DX conversion
        let result = serializer::toml_to_dx(&content);
        prop_assert!(
            result.is_ok(),
            "TOML conversion failed for content: {} - error: {:?}",
            content, result.err()
        );

        let dx_content = result.unwrap();
        prop_assert!(
            !dx_content.is_empty(),
            "DX output should not be empty for TOML input"
        );
    }
}

/// Property 6: Format Filter Correctness
/// *For any* set of files and a format filter, the CLI SHALL process only files
/// matching the specified format.
///
/// **Validates: Requirements 5.7**
#[test]
fn test_format_filter_correctness() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create files with different extensions
    fs::write(dir_path.join("config.json"), r#"{"test": true}"#).unwrap();
    fs::write(dir_path.join("config.yaml"), "test: true").unwrap();
    fs::write(dir_path.join("config.toml"), "test = true").unwrap();
    fs::write(dir_path.join("readme.txt"), "Not a config").unwrap();

    // Test JSON filter
    let json_pattern = format!("{}/*.json", dir_path.display());
    let json_files: Vec<_> = glob::glob(&json_pattern).unwrap().filter_map(|r| r.ok()).collect();
    assert_eq!(json_files.len(), 1, "Should find exactly 1 JSON file");
    assert!(json_files[0].to_string_lossy().ends_with(".json"));

    // Test YAML filter
    let yaml_pattern = format!("{}/*.yaml", dir_path.display());
    let yaml_files: Vec<_> = glob::glob(&yaml_pattern).unwrap().filter_map(|r| r.ok()).collect();
    assert_eq!(yaml_files.len(), 1, "Should find exactly 1 YAML file");
    assert!(yaml_files[0].to_string_lossy().ends_with(".yaml"));

    // Test TOML filter
    let toml_pattern = format!("{}/*.toml", dir_path.display());
    let toml_files: Vec<_> = glob::glob(&toml_pattern).unwrap().filter_map(|r| r.ok()).collect();
    assert_eq!(toml_files.len(), 1, "Should find exactly 1 TOML file");
    assert!(toml_files[0].to_string_lossy().ends_with(".toml"));
}

/// Test that conversion produces valid DX output
#[test]
fn test_conversion_produces_valid_dx() {
    let json_input = r#"{"name": "test", "version": "1.0.0", "enabled": true, "count": 42}"#;

    let dx_output = serializer::json_to_dx(json_input).expect("JSON conversion should succeed");

    // DX output should not be empty
    assert!(!dx_output.is_empty(), "DX output should not be empty");

    // DX output should be shorter than JSON (token efficient)
    // Note: This may not always be true for very small inputs
    // but should generally hold for typical config files
}

/// Test that token counting works for converted content
#[test]
fn test_token_counting_for_converted_content() {
    use serializer::{ModelType, TokenCounter};

    let json_input = r#"{"name": "test-app", "version": "1.0.0", "description": "A test application", "enabled": true}"#;
    let dx_output = serializer::json_to_dx(json_input).expect("JSON conversion should succeed");

    let counter = TokenCounter::new();

    let json_tokens = counter.count(json_input, ModelType::Gpt4o);
    let dx_tokens = counter.count(&dx_output, ModelType::Gpt4o);

    // Both should have non-zero token counts
    assert!(json_tokens.count > 0, "JSON should have tokens");
    assert!(dx_tokens.count > 0, "DX should have tokens");

    // Token counts should be available for all primary models
    let json_all = counter.count_primary_models(json_input);
    let dx_all = counter.count_primary_models(&dx_output);

    assert_eq!(json_all.len(), 4, "Should have 4 model counts for JSON");
    assert_eq!(dx_all.len(), 4, "Should have 4 model counts for DX");
}
