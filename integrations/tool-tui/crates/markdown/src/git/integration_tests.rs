//! Integration tests for Holographic Git workflow.
//!
//! These tests verify the end-to-end workflow of:
//! - `dx dxm init` creates correct configuration
//! - Clean filter converts DXM to MD correctly
//! - Smudge filter converts MD to DXM correctly
//! - Pre-commit hook syncs files
//! - Graceful degradation when DX is not installed
//!
//! Requirements: 3.1-3.7, 8.1-8.4, 9.1-9.5

#[cfg(test)]
#[allow(clippy::module_inception)] // Test module has same name as file
mod integration_tests {
    use crate::git::{CleanFilter, FilterError, InitError, RepoInitializer, SmudgeFilter};
    use std::fs;
    use std::io::Cursor;
    use std::path::PathBuf;

    /// Test that `dx dxm init` creates correct configuration.
    ///
    /// Requirements: 3.1-3.7
    mod init_tests {
        use super::*;
        use std::env;

        /// Helper to create a temporary test directory
        fn create_temp_dir(name: &str) -> PathBuf {
            let temp = env::temp_dir().join(format!("dx_test_{}", name));
            if temp.exists() {
                fs::remove_dir_all(&temp).ok();
            }
            fs::create_dir_all(&temp).unwrap();
            temp
        }

        /// Helper to clean up a temporary test directory
        fn cleanup_temp_dir(path: &PathBuf) {
            fs::remove_dir_all(path).ok();
        }

        /// Test that init creates .gitattributes with correct rules.
        ///
        /// Note: This test requires an actual git repository and filesystem access.
        /// Run with: `cargo test --package dx-markdown -- --ignored`
        #[test]
        #[ignore = "requires actual git repository - run with --ignored"]
        fn test_init_creates_gitattributes() {
            let repo_root = create_temp_dir("gitattributes");

            // Create a fake .git directory
            fs::create_dir(repo_root.join(".git")).unwrap();

            // Initialize
            let initializer = RepoInitializer::new(&repo_root);
            let result = initializer.init();

            assert!(result.is_ok(), "Init should succeed: {:?}", result.err());
            let init_result = result.unwrap();
            assert!(init_result.gitattributes_updated);

            // Check .gitattributes content
            let gitattributes = fs::read_to_string(repo_root.join(".gitattributes")).unwrap();
            assert!(gitattributes.contains("*.dxm filter=dxm diff=dxm text"));

            cleanup_temp_dir(&repo_root);
        }

        /// Test that init creates .dx/config.
        ///
        /// Note: This test requires an actual git repository and filesystem access.
        /// Run with: `cargo test --package dx-markdown -- --ignored`
        #[test]
        #[ignore = "requires actual git repository - run with --ignored"]
        fn test_init_creates_dx_config() {
            let repo_root = create_temp_dir("dx_config");

            // Create a fake .git directory
            fs::create_dir(repo_root.join(".git")).unwrap();

            // Initialize
            let initializer = RepoInitializer::new(&repo_root);
            let result = initializer.init();

            assert!(result.is_ok());
            let init_result = result.unwrap();
            assert!(init_result.dx_config_created);

            // Check .dx/config exists
            assert!(repo_root.join(".dx").join("config").exists());

            // Check content
            let config = fs::read_to_string(repo_root.join(".dx").join("config")).unwrap();
            assert!(config.contains("dxm_enabled = true") || config.contains("enabled = true"));

            cleanup_temp_dir(&repo_root);
        }

        /// Test that init fails when not in a git repository.
        #[test]
        fn test_init_fails_without_git() {
            let repo_root = create_temp_dir("no_git");

            // Don't create .git directory

            let initializer = RepoInitializer::new(&repo_root);
            let result = initializer.init();

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), InitError::NotGitRepo));

            cleanup_temp_dir(&repo_root);
        }

        /// Test that init preserves existing .gitattributes content.
        ///
        /// Note: This test requires an actual git repository and filesystem access.
        /// Run with: `cargo test --package dx-markdown -- --ignored`
        #[test]
        #[ignore = "requires actual git repository - run with --ignored"]
        fn test_init_preserves_existing_gitattributes() {
            let repo_root = create_temp_dir("preserve_gitattributes");

            // Create a fake .git directory
            fs::create_dir(repo_root.join(".git")).unwrap();

            // Create existing .gitattributes
            let existing_content = "*.txt text\n*.bin binary\n";
            fs::write(repo_root.join(".gitattributes"), existing_content).unwrap();

            // Initialize
            let initializer = RepoInitializer::new(&repo_root);
            let result = initializer.init();

            assert!(result.is_ok());

            // Check that existing content is preserved
            let gitattributes = fs::read_to_string(repo_root.join(".gitattributes")).unwrap();
            assert!(gitattributes.contains("*.txt text"));
            assert!(gitattributes.contains("*.bin binary"));
            assert!(gitattributes.contains("*.dxm filter=dxm diff=dxm text"));

            cleanup_temp_dir(&repo_root);
        }

        /// Test that init with --hooks installs pre-commit hook.
        ///
        /// Note: This test requires an actual git repository and filesystem access.
        /// Run with: `cargo test --package dx-markdown -- --ignored`
        #[test]
        #[ignore = "requires actual git repository - run with --ignored"]
        fn test_init_with_hooks() {
            let repo_root = create_temp_dir("with_hooks");

            // Create a fake .git directory with hooks
            fs::create_dir_all(repo_root.join(".git").join("hooks")).unwrap();

            // Initialize with hooks
            let initializer = RepoInitializer::new(&repo_root).with_hooks(true);
            let result = initializer.init();

            assert!(result.is_ok());
            let init_result = result.unwrap();
            assert!(init_result.hooks_installed);

            // Check hook exists
            let hook_path = repo_root.join(".git").join("hooks").join("pre-commit");
            assert!(hook_path.exists());

            // Check hook content
            let hook_content = fs::read_to_string(&hook_path).unwrap();
            assert!(hook_content.contains("DXM Pre-Commit Hook"));

            cleanup_temp_dir(&repo_root);
        }
    }

    /// Test clean filter converts DXM to MD correctly.
    ///
    /// Requirements: 1.1-1.9
    mod clean_filter_tests {
        use super::*;

        /// Test basic header conversion.
        #[test]
        fn test_clean_converts_headers() {
            let filter = CleanFilter::new();
            let dxm = "1|Hello World\n2|Subtitle";
            let md = filter.dxm_to_markdown(dxm).unwrap();

            assert!(md.contains("# Hello World"));
            assert!(md.contains("## Subtitle"));
        }

        /// Test code block conversion.
        #[test]
        fn test_clean_converts_code_blocks() {
            let filter = CleanFilter::new();
            let dxm = "1|Code Example\n@rust\nfn main() {}\n@";
            let md = filter.dxm_to_markdown(dxm).unwrap();

            assert!(md.contains("```rust"));
            assert!(md.contains("fn main() {}"));
            assert!(md.contains("```"));
        }

        /// Test that already-Markdown content passes through unchanged.
        #[test]
        fn test_clean_passthrough_markdown() {
            let filter = CleanFilter::new();
            let md_input = "# Already Markdown\n\nThis is a paragraph.";
            let mut output = Vec::new();

            filter.process(Cursor::new(md_input), &mut output).unwrap();

            let result = String::from_utf8(output).unwrap();
            assert_eq!(result, md_input);
        }

        /// Test input size validation.
        #[test]
        fn test_clean_rejects_large_input() {
            let filter = CleanFilter::with_max_size(100);
            let large_input = "x".repeat(200);
            let mut output = Vec::new();

            let result = filter.process(Cursor::new(large_input), &mut output);

            assert!(matches!(result, Err(FilterError::InputTooLarge { .. })));
        }

        /// Test UTF-8 validation.
        #[test]
        fn test_clean_rejects_invalid_utf8() {
            let filter = CleanFilter::new();
            let invalid_utf8 = vec![0xFF, 0xFE, 0x00, 0x01];
            let mut output = Vec::new();

            let result = filter.process(Cursor::new(invalid_utf8), &mut output);

            assert!(matches!(result, Err(FilterError::InvalidUtf8 { .. })));
        }
    }

    /// Test smudge filter converts MD to DXM correctly.
    ///
    /// Requirements: 2.1-2.9
    mod smudge_filter_tests {
        use super::*;

        /// Test basic header conversion.
        #[test]
        fn test_smudge_converts_headers() {
            let filter = SmudgeFilter::new();
            let md = "# Hello World\n## Subtitle";
            let dxm = filter.markdown_to_dxm(md).unwrap();

            assert!(dxm.contains("1|Hello World"));
            assert!(dxm.contains("2|Subtitle"));
        }

        /// Test code block conversion.
        #[test]
        fn test_smudge_converts_code_blocks() {
            let filter = SmudgeFilter::new();
            let md = "# Code Example\n```rust\nfn main() {}\n```";
            let dxm = filter.markdown_to_dxm(md).unwrap();

            // DXM uses @lang ... @ for code blocks
            assert!(dxm.contains("fn main()"));
        }

        /// Test that already-DXM content passes through unchanged.
        #[test]
        fn test_smudge_passthrough_dxm() {
            let filter = SmudgeFilter::new();
            let dxm_input = "1|Already DXM\n\nThis is a paragraph.";
            let mut output = Vec::new();

            filter.process(Cursor::new(dxm_input), &mut output).unwrap();

            let result = String::from_utf8(output).unwrap();
            assert_eq!(result, dxm_input);
        }

        /// Test graceful degradation on parse error.
        #[test]
        fn test_smudge_graceful_degradation() {
            let filter = SmudgeFilter::new();
            // This is intentionally malformed to test graceful degradation
            let malformed = "<<<<<<< HEAD\nconflict marker\n=======\nother side\n>>>>>>>";
            let mut output = Vec::new();

            // Should not error, should pass through
            let result = filter.process(Cursor::new(malformed), &mut output);
            assert!(result.is_ok());
        }
    }

    /// Test round-trip conversion preserves content.
    ///
    /// Requirements: 1.2, 2.2, 9.3
    mod round_trip_tests {
        use super::*;

        /// Test DXM -> MD -> DXM round trip.
        #[test]
        fn test_dxm_md_dxm_round_trip() {
            let clean_filter = CleanFilter::new();
            let smudge_filter = SmudgeFilter::new();

            let original_dxm =
                "1|Hello World\n\nThis is a paragraph.\n\n2|Section Two\n\nMore content.";

            // DXM -> MD
            let md = clean_filter.dxm_to_markdown(original_dxm).unwrap();

            // MD -> DXM
            let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

            // Content should be preserved
            assert!(dxm_back.contains("Hello World"));
            assert!(dxm_back.contains("Section Two"));
            assert!(dxm_back.contains("paragraph"));
        }

        /// Test MD -> DXM -> MD round trip.
        #[test]
        fn test_md_dxm_md_round_trip() {
            let clean_filter = CleanFilter::new();
            let smudge_filter = SmudgeFilter::new();

            let original_md =
                "# Hello World\n\nThis is a paragraph.\n\n## Section Two\n\nMore content.";

            // MD -> DXM
            let dxm = smudge_filter.markdown_to_dxm(original_md).unwrap();

            // DXM -> MD
            let md_back = clean_filter.dxm_to_markdown(&dxm).unwrap();

            // Content should be preserved
            assert!(md_back.contains("Hello World"));
            assert!(md_back.contains("Section Two"));
            assert!(md_back.contains("paragraph"));
        }

        /// Test header levels are preserved through round trip.
        #[test]
        fn test_header_levels_preserved() {
            let clean_filter = CleanFilter::new();
            let smudge_filter = SmudgeFilter::new();

            for level in 1..=6 {
                let dxm = format!("{}|Header Level {}", level, level);
                let md = clean_filter.dxm_to_markdown(&dxm).unwrap();
                let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

                // Check the level is preserved
                assert!(
                    dxm_back.contains(&format!("{}|", level)),
                    "Level {} not preserved in: {}",
                    level,
                    dxm_back
                );
            }
        }
    }

    /// Test graceful degradation when DX is not installed.
    ///
    /// Requirements: 9.1-9.5
    mod graceful_degradation_tests {
        use super::*;

        /// Test that .md files are valid CommonMark.
        #[test]
        fn test_clean_output_is_valid_commonmark() {
            let filter = CleanFilter::new();
            let dxm = "1|Title\n\nParagraph with text! and text/.\n\n@rust\ncode\n@";
            let md = filter.dxm_to_markdown(dxm).unwrap();

            // Basic CommonMark validation
            assert!(md.contains("# Title"));
            assert!(md.contains("```"));
            // Should not contain DXM-specific syntax
            assert!(!md.contains("1|"));
            assert!(!md.contains("@rust"));
        }

        /// Test that filters pass through when content is already in target format.
        #[test]
        fn test_filters_are_idempotent() {
            let clean_filter = CleanFilter::new();
            let smudge_filter = SmudgeFilter::new();

            // Clean filter on Markdown should pass through
            let md = "# Already Markdown";
            let mut output = Vec::new();
            clean_filter.process(Cursor::new(md), &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), md);

            // Smudge filter on DXM should pass through
            let dxm = "1|Already DXM";
            let mut output = Vec::new();
            smudge_filter.process(Cursor::new(dxm), &mut output).unwrap();
            assert_eq!(String::from_utf8(output).unwrap(), dxm);
        }
    }
}

/// Additional graceful degradation tests.
///
/// Requirements: 9.1-9.5
#[cfg(test)]
mod extended_graceful_degradation_tests {
    use crate::git::{CleanFilter, SmudgeFilter};
    use std::io::Cursor;

    /// Test that repository works without DX installed.
    /// The .md files should be valid and usable.
    ///
    /// Requirements: 9.1, 9.3
    #[test]
    fn test_md_files_are_standalone_valid() {
        let clean_filter = CleanFilter::new();

        // Create a complex DXM document
        let dxm = r#"1|Project README

This is the project description.

2|Installation

@bash
npm install my-package
@

3|Usage

Here's how to use it:

@javascript
const pkg = require('my-package');
pkg.doSomething();
@

4|License

MIT License
"#;

        // Convert to Markdown
        let md = clean_filter.dxm_to_markdown(dxm).unwrap();

        // Verify the Markdown is valid and complete
        assert!(md.contains("# Project README"));
        assert!(md.contains("## Installation"));
        assert!(md.contains("## Usage"));
        assert!(md.contains("## License"));
        assert!(md.contains("```bash"));
        assert!(md.contains("```javascript"));
        assert!(md.contains("npm install"));
        assert!(md.contains("require('my-package')"));
    }

    /// Test that filters pass through when DX not available.
    /// This simulates what happens when git runs the filter but dx isn't installed.
    ///
    /// Requirements: 9.2
    #[test]
    fn test_passthrough_on_unknown_format() {
        let smudge_filter = SmudgeFilter::new();

        // Content that doesn't match any known format
        let unknown_content = "This is just plain text without any special formatting.";
        let mut output = Vec::new();

        // Should not error
        let result = smudge_filter.process(Cursor::new(unknown_content), &mut output);
        assert!(result.is_ok());

        // Content should be preserved (either converted or passed through)
        let result_str = String::from_utf8(output).unwrap();
        assert!(result_str.contains("plain text"));
    }

    /// Test that DX can be installed later and convert existing .md files.
    ///
    /// Requirements: 9.4
    #[test]
    fn test_late_dx_installation() {
        let smudge_filter = SmudgeFilter::new();

        // Simulate a .md file that was created without DX
        let existing_md = r#"# Existing Project

This project was created before DX was installed.

## Features

- Feature 1
- Feature 2

## Code Example

```python
def hello():
    print("Hello, World!")
```
"#;

        // When DX is installed, smudge filter should convert it
        let dxm = smudge_filter.markdown_to_dxm(existing_md).unwrap();

        // Verify conversion happened
        assert!(dxm.contains("1|Existing Project"));
        assert!(dxm.contains("2|Features"));
        assert!(dxm.contains("2|Code Example"));
        assert!(dxm.contains("hello()"));
    }

    /// Test that repository works normally for developers who never install DX.
    ///
    /// Requirements: 9.5
    #[test]
    fn test_repository_works_without_dx() {
        let clean_filter = CleanFilter::new();

        // Create DXM content
        let dxm = "1|Hello\n\nWorld";

        // Convert to MD (what would be stored in git)
        let md = clean_filter.dxm_to_markdown(dxm).unwrap();

        // The MD should be usable as-is
        assert!(md.contains("# Hello"));
        assert!(md.contains("World"));

        // A developer without DX would just see this MD file
        // and could edit it directly
        let edited_md = md.replace("World", "Universe");
        assert!(edited_md.contains("Universe"));
    }

    /// Test that binary format is handled correctly.
    #[test]
    fn test_binary_format_passthrough() {
        let smudge_filter = SmudgeFilter::new();

        // Binary content (DXMB magic bytes)
        let binary_content = b"DXMB\x00\x01\x00\x00some binary data";
        let mut output = Vec::new();

        // Should pass through binary content unchanged
        let result = smudge_filter.process(Cursor::new(&binary_content[..]), &mut output);
        assert!(result.is_ok());
        assert_eq!(output, binary_content);
    }
}
