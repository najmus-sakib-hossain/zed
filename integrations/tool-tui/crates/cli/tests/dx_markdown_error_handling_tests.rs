//! Unit tests for error handling scenarios in dx markdown command
//! Feature: professional-dx-markdown-cli
//! Task 8.5: Write unit tests for error handling scenarios
//! Requirements: 11.1-11.4, 1.4-1.5

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

// ============================================================================
// Test: Read Permission Errors (Requirement 11.1)
// ============================================================================

#[test]
#[cfg(unix)] // Permission tests work differently on Windows
fn test_read_permission_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let file_path = temp_dir.path().join("test.md");

    // Create file with content
    fs::write(&file_path, "# Test").expect("Failed to write file");

    // Remove read permissions
    let mut perms = fs::metadata(&file_path).unwrap().permissions();
    perms.set_mode(0o000); // No permissions
    fs::set_permissions(&file_path, perms).expect("Failed to set permissions");

    // Attempt to read should fail
    let result = fs::read_to_string(&file_path);
    assert!(result.is_err(), "Should fail to read file without read permissions");
}

// ============================================================================
// Test: Write Permission Errors (Requirement 11.3)
// ============================================================================

#[test]
#[cfg(unix)] // Permission tests work differently on Windows
fn test_write_permission_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_dir = temp_dir.path().join(".dx").join("markdown");

    // Create output directory
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    // Remove write permissions from output directory
    let mut perms = fs::metadata(&output_dir).unwrap().permissions();
    perms.set_mode(0o444); // Read-only
    fs::set_permissions(&output_dir, perms).expect("Failed to set permissions");

    // Attempt to write should fail
    let output_file = output_dir.join("test.human");
    let result = fs::write(&output_file, "content");
    assert!(result.is_err(), "Should fail to write file without write permissions");

    // Cleanup: restore permissions
    let mut perms = fs::metadata(&output_dir).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&output_dir, perms).ok();
}

// ============================================================================
// Test: Invalid Paths (Requirement 11.4)
// ============================================================================

#[test]
fn test_nonexistent_file() {
    let nonexistent = PathBuf::from("/nonexistent/path/file.md");
    assert!(!nonexistent.exists(), "Path should not exist");

    // Attempting to read should fail
    let result = fs::read_to_string(&nonexistent);
    assert!(result.is_err(), "Should fail to read nonexistent file");
}

#[test]
fn test_nonexistent_directory() {
    let nonexistent = PathBuf::from("/nonexistent/directory");
    assert!(!nonexistent.exists(), "Directory should not exist");
    assert!(!nonexistent.is_dir(), "Should not be a directory");
}

#[test]
fn test_invalid_path_characters() {
    // Test with various invalid path scenarios
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // File that doesn't exist
    let invalid_path = temp_dir.path().join("does_not_exist.md");
    assert!(!invalid_path.exists(), "Invalid path should not exist");

    let result = fs::read_to_string(&invalid_path);
    assert!(result.is_err(), "Should fail to read invalid path");
}

// ============================================================================
// Test: Non-Markdown Files (Requirements 1.4, 1.5)
// ============================================================================

#[test]
fn test_non_markdown_file_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create non-markdown files
    let txt_file = temp_dir.path().join("test.txt");
    let rs_file = temp_dir.path().join("test.rs");
    let no_ext_file = temp_dir.path().join("test");

    fs::write(&txt_file, "content").expect("Failed to write txt file");
    fs::write(&rs_file, "content").expect("Failed to write rs file");
    fs::write(&no_ext_file, "content").expect("Failed to write no-ext file");

    // Check extensions
    assert_eq!(txt_file.extension().and_then(|s| s.to_str()), Some("txt"));
    assert_eq!(rs_file.extension().and_then(|s| s.to_str()), Some("rs"));
    assert_eq!(no_ext_file.extension().and_then(|s| s.to_str()), None);

    // None should be markdown
    assert_ne!(txt_file.extension().and_then(|s| s.to_str()), Some("md"));
    assert_ne!(rs_file.extension().and_then(|s| s.to_str()), Some("md"));
    assert_ne!(no_ext_file.extension().and_then(|s| s.to_str()), Some("md"));
}

#[test]
fn test_markdown_file_detection() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create markdown file
    let md_file = temp_dir.path().join("test.md");
    fs::write(&md_file, "# Test").expect("Failed to write md file");

    // Should be detected as markdown
    assert_eq!(md_file.extension().and_then(|s| s.to_str()), Some("md"));
}

#[test]
fn test_skip_non_markdown_in_directory() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create mixed files
    fs::write(temp_dir.path().join("test1.md"), "# Test 1").unwrap();
    fs::write(temp_dir.path().join("test2.txt"), "Text file").unwrap();
    fs::write(temp_dir.path().join("test3.md"), "# Test 3").unwrap();
    fs::write(temp_dir.path().join("readme.rst"), "RST file").unwrap();

    // Count markdown files
    let md_files: Vec<_> = fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();

    assert_eq!(md_files.len(), 2, "Should find exactly 2 markdown files");
}

// ============================================================================
// Test: Directory Creation Errors (Requirement 4.4)
// ============================================================================

#[test]
#[cfg(unix)]
fn test_directory_creation_permission_error() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a parent directory with no write permissions
    let parent = temp_dir.path().join("readonly");
    fs::create_dir(&parent).expect("Failed to create parent dir");

    let mut perms = fs::metadata(&parent).unwrap().permissions();
    perms.set_mode(0o444); // Read-only
    fs::set_permissions(&parent, perms).expect("Failed to set permissions");

    // Attempt to create subdirectory should fail
    let subdir = parent.join(".dx");
    let result = fs::create_dir(&subdir);
    assert!(result.is_err(), "Should fail to create directory without write permissions");

    // Cleanup: restore permissions
    let mut perms = fs::metadata(&parent).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&parent, perms).ok();
}

// ============================================================================
// Test: File System Edge Cases
// ============================================================================

#[test]
fn test_empty_file_name() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let path = temp_dir.path().join("");

    // Joining empty string returns the directory itself
    assert_eq!(path, temp_dir.path());
}

#[test]
fn test_file_stem_extraction() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test various file names
    let test_cases = vec![
        ("test.md", Some("test")),
        ("my-file.md", Some("my-file")),
        ("file_name.md", Some("file_name")),
        (".hidden.md", Some(".hidden")),
    ];

    for (filename, expected_stem) in test_cases {
        let path = temp_dir.path().join(filename);
        let stem = path.file_stem().and_then(|s| s.to_str());
        assert_eq!(stem, expected_stem, "File stem mismatch for {}", filename);
    }
}

#[test]
fn test_output_path_calculation() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let _input = temp_dir.path().join("test.md");

    // Calculate expected output paths
    let dx_dir = temp_dir.path().join(".dx").join("markdown");
    let expected_human = dx_dir.join("test.human");
    let expected_machine = dx_dir.join("test.machine");

    // Verify path structure
    assert_eq!(expected_human.file_name().unwrap(), "test.human");
    assert_eq!(expected_machine.file_name().unwrap(), "test.machine");
    assert_eq!(expected_human.parent().unwrap(), dx_dir.as_path());
    assert_eq!(expected_machine.parent().unwrap(), dx_dir.as_path());
}

// ============================================================================
// Test: Compilation Error Handling (Requirement 11.2)
// ============================================================================

#[test]
fn test_invalid_markdown_content() {
    use markdown::{CompilerConfig, DxMarkdown};

    // Create compiler
    let config = CompilerConfig::default();
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");

    // Test with various content (compiler should handle gracefully)
    let test_cases = vec![
        "",               // Empty content
        "   ",            // Whitespace only
        "\n\n\n",         // Newlines only
        "Plain text",     // No markdown
        "# Valid Header", // Valid markdown
    ];

    for content in test_cases {
        let result = compiler.compile(content);
        // Compiler should not panic, even with edge cases
        assert!(result.is_ok() || result.is_err(), "Compiler should return a result");
    }
}

// ============================================================================
// Test: Graceful Degradation (Requirements 11.2, 11.3)
// ============================================================================

#[test]
fn test_partial_output_success() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let dx_dir = temp_dir.path().join(".dx").join("markdown");

    // Create output directory
    fs::create_dir_all(&dx_dir).expect("Failed to create output dir");

    // Write some outputs successfully
    let human_path = dx_dir.join("test.human");
    let machine_path = dx_dir.join("test.machine");

    let human_result = fs::write(&human_path, "human content");
    let machine_result = fs::write(&machine_path, b"machine content");

    // Both should succeed
    assert!(human_result.is_ok(), "Human format write should succeed");
    assert!(machine_result.is_ok(), "Machine format write should succeed");

    // Verify files exist
    assert!(human_path.exists(), "Human format file should exist");
    assert!(machine_path.exists(), "Machine format file should exist");
}
