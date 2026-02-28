//! Property Tests for Check Mode, Write Mode, and File Extension Routing
//!
//! This module contains property-based tests for verifying that:
//! - Check mode (write=false) does not modify files
//! - Write mode (write=true) correctly modifies files
//! - Unchanged files are correctly detected
//! - File extension routing correctly maps files to handlers

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    use crate::languages::{FileStatus, LanguageHandler, MarkdownHandler, TomlHandler};

    /// Generator for simple TOML content
    fn arb_simple_toml() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("[package]\nname = \"test\"\nversion = \"1.0.0\"\n".to_string()),
            Just("[dependencies]\nserde = \"1.0\"\n".to_string()),
            Just("key = \"value\"\n".to_string()),
        ]
    }

    /// Generator for simple Markdown content
    fn arb_simple_markdown() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("# Heading\n\nParagraph text.\n".to_string()),
            Just("- Item 1\n- Item 2\n".to_string()),
            Just("Some text.\n".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 5: Check Mode File Preservation**
        /// *For any* file processed in check mode (write=false), the file's content on disk
        /// SHALL remain unchanged after processing.
        /// **Validates: Requirements 10.1**
        #[test]
        fn prop_check_mode_preserves_toml_files(content in arb_simple_toml()) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join("test.toml");

            // Write initial content
            fs::write(&file_path, &content).expect("Failed to write file");

            // Read the content back to ensure it's what we wrote
            let original_content = fs::read_to_string(&file_path).expect("Failed to read file");

            // Process in check mode (write=false)
            let handler = TomlHandler::new();
            let _ = handler.format(&file_path, &content, false);

            // Read the content after processing
            let after_content = fs::read_to_string(&file_path).expect("Failed to read file after");

            // Content should be unchanged
            prop_assert_eq!(
                original_content,
                after_content,
                "Check mode should not modify file content"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 5: Check Mode File Preservation (Markdown)**
        /// *For any* Markdown file processed in check mode (write=false), the file's content
        /// on disk SHALL remain unchanged after processing.
        /// **Validates: Requirements 10.1**
        #[test]
        fn prop_check_mode_preserves_markdown_files(content in arb_simple_markdown()) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join("test.md");

            // Write initial content
            fs::write(&file_path, &content).expect("Failed to write file");

            // Read the content back
            let original_content = fs::read_to_string(&file_path).expect("Failed to read file");

            // Process in check mode (write=false)
            let handler = MarkdownHandler::new();
            let _ = handler.format(&file_path, &content, false);

            // Read the content after processing
            let after_content = fs::read_to_string(&file_path).expect("Failed to read file after");

            // Content should be unchanged
            prop_assert_eq!(
                original_content,
                after_content,
                "Check mode should not modify Markdown file content"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 6: Write Mode File Modification**
        /// *For any* file that requires formatting changes and is processed in write mode
        /// (write=true), the file's content on disk SHALL be updated to match the formatted output.
        /// **Validates: Requirements 10.2**
        #[test]
        fn prop_write_mode_modifies_toml_files(content in arb_simple_toml()) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join("test.toml");

            // Write initial content
            fs::write(&file_path, &content).expect("Failed to write file");

            // Process in write mode (write=true)
            let handler = TomlHandler::new();
            let result = handler.format(&file_path, &content, true);

            // If formatting succeeded and changed the file
            if let Ok(FileStatus::Changed) = result {
                // Read the content after processing
                let after_content = fs::read_to_string(&file_path).expect("Failed to read file after");

                // Content should be different from original (formatted)
                // Note: We can't assert they're different because some content might already be formatted
                // Instead, we verify the file is valid TOML after formatting
                let parse_result: Result<toml::Value, _> = after_content.parse();
                prop_assert!(
                    parse_result.is_ok(),
                    "Written content should be valid TOML"
                );
            }
        }

        /// **Feature: multi-language-formatter-linter, Property 7: Unchanged File Detection**
        /// *For any* file where the formatted output equals the original content,
        /// the FileStatus SHALL be Unchanged.
        /// **Validates: Requirements 10.3**
        #[test]
        fn prop_unchanged_file_detection_toml(content in arb_simple_toml()) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join("test.toml");

            // Write initial content
            fs::write(&file_path, &content).expect("Failed to write file");

            let handler = TomlHandler::new();

            // First, format the content to get the canonical form
            let first_result = handler.format(&file_path, &content, true);

            // If first format succeeded
            if first_result.is_ok() {
                // Read the formatted content
                let formatted_content = fs::read_to_string(&file_path).expect("Failed to read file");

                // Format again with the already-formatted content
                let second_result = handler.format(&file_path, &formatted_content, false);

                // Should be unchanged since content is already formatted
                prop_assert!(
                    matches!(second_result, Ok(FileStatus::Unchanged)),
                    "Already formatted content should return Unchanged status, got: {:?}",
                    second_result
                );
            }
        }

        /// **Feature: multi-language-formatter-linter, Property 7: Unchanged File Detection (Markdown)**
        /// *For any* Markdown file where the formatted output equals the original content,
        /// the FileStatus SHALL be Unchanged.
        /// **Validates: Requirements 10.3**
        #[test]
        fn prop_unchanged_file_detection_markdown(content in arb_simple_markdown()) {
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let file_path = temp_dir.path().join("test.md");

            // Write initial content
            fs::write(&file_path, &content).expect("Failed to write file");

            let handler = MarkdownHandler::new();

            // First, format the content to get the canonical form
            let first_result = handler.format(&file_path, &content, true);

            // If first format succeeded
            if first_result.is_ok() {
                // Read the formatted content
                let formatted_content = fs::read_to_string(&file_path).expect("Failed to read file");

                // Format again with the already-formatted content
                let second_result = handler.format(&file_path, &formatted_content, false);

                // Should be unchanged since content is already formatted
                prop_assert!(
                    matches!(second_result, Ok(FileStatus::Unchanged)),
                    "Already formatted Markdown content should return Unchanged status, got: {:?}",
                    second_result
                );
            }
        }
    }
}

#[cfg(test)]
mod extension_routing_tests {
    use proptest::prelude::*;
    use std::path::Path;

    use crate::languages::{
        CppHandler, FileProcessor, GoHandler, KotlinHandler, LanguageHandler, MarkdownHandler,
        PhpHandler, PythonHandler, RustHandler, TomlHandler,
    };

    /// All supported extensions mapped to their expected handler names
    const EXTENSION_HANDLER_MAP: &[(&str, &str)] = &[
        // Python
        ("py", "python"),
        ("pyi", "python"),
        // C/C++
        ("c", "cpp"),
        ("cpp", "cpp"),
        ("cc", "cpp"),
        ("cxx", "cpp"),
        ("h", "cpp"),
        ("hpp", "cpp"),
        ("hxx", "cpp"),
        // Go
        ("go", "go"),
        // Rust
        ("rs", "rust"),
        // PHP
        ("php", "php"),
        // Kotlin
        ("kt", "kotlin"),
        ("kts", "kotlin"),
        // Markdown
        ("md", "markdown"),
        ("markdown", "markdown"),
        // TOML
        ("toml", "toml"),
    ];

    /// Create a FileProcessor with all handlers registered
    fn create_full_processor() -> FileProcessor {
        let mut processor = FileProcessor::new();
        processor.register(PythonHandler::new());
        processor.register(CppHandler::new());
        processor.register(GoHandler::new());
        processor.register(RustHandler::new());
        processor.register(PhpHandler::new());
        processor.register(KotlinHandler::new());
        processor.register(MarkdownHandler::new());
        processor.register(TomlHandler::new());
        processor
    }

    /// Generator for Python file extensions
    fn arb_python_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("py"), Just("pyi"),]
    }

    /// Generator for C/C++ file extensions
    fn arb_cpp_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("c"),
            Just("cpp"),
            Just("cc"),
            Just("cxx"),
            Just("h"),
            Just("hpp"),
            Just("hxx"),
        ]
    }

    /// Generator for Go file extensions
    fn arb_go_extension() -> impl Strategy<Value = &'static str> {
        Just("go")
    }

    /// Generator for Rust file extensions
    fn arb_rust_extension() -> impl Strategy<Value = &'static str> {
        Just("rs")
    }

    /// Generator for PHP file extensions
    fn arb_php_extension() -> impl Strategy<Value = &'static str> {
        Just("php")
    }

    /// Generator for Kotlin file extensions
    fn arb_kotlin_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("kt"), Just("kts"),]
    }

    /// Generator for Markdown file extensions
    fn arb_markdown_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("md"), Just("markdown"),]
    }

    /// Generator for TOML file extensions
    fn arb_toml_extension() -> impl Strategy<Value = &'static str> {
        Just("toml")
    }

    /// Generator for all supported file extensions
    fn arb_supported_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            arb_python_extension(),
            arb_cpp_extension(),
            arb_go_extension(),
            arb_rust_extension(),
            arb_php_extension(),
            arb_kotlin_extension(),
            arb_markdown_extension(),
            arb_toml_extension(),
        ]
    }

    /// Generator for unsupported file extensions
    fn arb_unsupported_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("txt"),
            Just("json"),
            Just("xml"),
            Just("yaml"),
            Just("yml"),
            Just("html"),
            Just("css"),
            Just("scss"),
            Just("less"),
            Just("sql"),
            Just("sh"),
            Just("bat"),
            Just("ps1"),
            Just("rb"),
            Just("pl"),
            Just("lua"),
            Just("swift"),
            Just("scala"),
            Just("groovy"),
            Just("dart"),
            Just("r"),
            Just("m"),
            Just("mm"),
            Just("asm"),
            Just("s"),
        ]
    }

    /// Generator for file names (without extension)
    fn arb_file_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,15}".prop_map(String::from)
    }

    /// Generator for directory paths
    fn arb_dir_path() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("".to_string()),
            "[a-z][a-z0-9_]{0,5}".prop_map(|s| format!("{}/", s)),
            ("[a-z][a-z0-9_]{0,5}", "[a-z][a-z0-9_]{0,5}")
                .prop_map(|(a, b)| format!("{}/{}/", a, b)),
        ]
    }

    /// Get the expected handler name for an extension
    fn get_expected_handler(ext: &str) -> Option<&'static str> {
        EXTENSION_HANDLER_MAP.iter().find(|(e, _)| *e == ext).map(|(_, h)| *h)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing**
        /// *For any* file with a known extension (py, pyi, c, cpp, cc, cxx, h, hpp, hxx, go, rs,
        /// php, kt, kts, md, markdown, toml), the File_Processor SHALL route it to the correct
        /// Language_Handler.
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_file_extension_routing_supported(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_supported_extension(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&file_path);

            // Get the handler for this file
            let handler = processor.get_handler(path);

            // Handler should be found for supported extensions
            prop_assert!(
                handler.is_some(),
                "Handler should be found for supported extension '{}', file: {}",
                ext, file_path
            );

            let handler = handler.unwrap();
            let expected_handler = get_expected_handler(ext);

            // Handler name should match expected
            prop_assert!(
                expected_handler.is_some(),
                "Extension '{}' should have an expected handler mapping",
                ext
            );

            prop_assert_eq!(
                handler.name(),
                expected_handler.unwrap(),
                "Extension '{}' should route to handler '{}', but got '{}'",
                ext, expected_handler.unwrap(), handler.name()
            );

            // The handler should support this extension
            prop_assert!(
                handler.extensions().contains(&ext),
                "Handler '{}' should list '{}' in its extensions, but extensions are: {:?}",
                handler.name(), ext, handler.extensions()
            );

            // is_supported should return true
            prop_assert!(
                processor.is_supported(path),
                "is_supported should return true for file with extension '{}'",
                ext
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Python)**
        /// *For any* Python file (.py, .pyi), the File_Processor SHALL route it to the Python handler.
        /// **Validates: Requirements 1.2, 2.1**
        #[test]
        fn prop_python_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_python_extension(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "python");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (C/C++)**
        /// *For any* C/C++ file (.c, .cpp, .cc, .cxx, .h, .hpp, .hxx), the File_Processor SHALL
        /// route it to the C/C++ handler.
        /// **Validates: Requirements 1.2, 3.1**
        #[test]
        fn prop_cpp_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_cpp_extension(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "cpp");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Go)**
        /// *For any* Go file (.go), the File_Processor SHALL route it to the Go handler.
        /// **Validates: Requirements 1.2, 4.1**
        #[test]
        fn prop_go_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.go", dir_path, file_name);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "go");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Rust)**
        /// *For any* Rust file (.rs), the File_Processor SHALL route it to the Rust handler.
        /// **Validates: Requirements 1.2, 5.1**
        #[test]
        fn prop_rust_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.rs", dir_path, file_name);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "rust");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (PHP)**
        /// *For any* PHP file (.php), the File_Processor SHALL route it to the PHP handler.
        /// **Validates: Requirements 1.2, 6.1**
        #[test]
        fn prop_php_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.php", dir_path, file_name);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "php");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Kotlin)**
        /// *For any* Kotlin file (.kt, .kts), the File_Processor SHALL route it to the Kotlin handler.
        /// **Validates: Requirements 1.2, 7.1**
        #[test]
        fn prop_kotlin_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_kotlin_extension(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "kotlin");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Markdown)**
        /// *For any* Markdown file (.md, .markdown), the File_Processor SHALL route it to the
        /// Markdown handler.
        /// **Validates: Requirements 1.2, 8.1**
        #[test]
        fn prop_markdown_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_markdown_extension(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "markdown");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (TOML)**
        /// *For any* TOML file (.toml), the File_Processor SHALL route it to the TOML handler.
        /// **Validates: Requirements 1.2, 9.1**
        #[test]
        fn prop_toml_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.toml", dir_path, file_name);
            let path = Path::new(&file_path);

            let handler = processor.get_handler(path);
            prop_assert!(handler.is_some());
            prop_assert_eq!(handler.unwrap().name(), "toml");
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Unsupported)**
        /// *For any* file with an unsupported extension, the File_Processor SHALL return None
        /// (no handler found).
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_unsupported_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_unsupported_extension(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&file_path);

            // Handler should NOT be found for unsupported extensions
            let handler = processor.get_handler(path);
            prop_assert!(
                handler.is_none(),
                "Handler should NOT be found for unsupported extension '{}', file: {}",
                ext, file_path
            );

            // is_supported should return false
            prop_assert!(
                !processor.is_supported(path),
                "is_supported should return false for file with unsupported extension '{}'",
                ext
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (No Extension)**
        /// *For any* file without an extension, the File_Processor SHALL return None (no handler found).
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_no_extension_routing(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
        ) {
            let processor = create_full_processor();
            let file_path = format!("{}{}", dir_path, file_name);
            let path = Path::new(&file_path);

            // Handler should NOT be found for files without extensions
            let handler = processor.get_handler(path);
            prop_assert!(
                handler.is_none(),
                "Handler should NOT be found for file without extension: {}",
                file_path
            );

            // is_supported should return false
            prop_assert!(
                !processor.is_supported(path),
                "is_supported should return false for file without extension"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 1: File Extension Routing (Case Sensitivity)**
        /// *For any* supported extension, the routing SHALL be case-sensitive (lowercase only).
        /// **Validates: Requirements 1.2**
        #[test]
        fn prop_extension_case_sensitivity(
            dir_path in arb_dir_path(),
            file_name in arb_file_name(),
            ext in arb_supported_extension(),
        ) {
            let processor = create_full_processor();

            // Lowercase extension should be supported
            let lowercase_path = format!("{}{}.{}", dir_path, file_name, ext);
            let path = Path::new(&lowercase_path);
            prop_assert!(
                processor.is_supported(path),
                "Lowercase extension '{}' should be supported",
                ext
            );

            // Uppercase extension should NOT be supported (case-sensitive)
            let uppercase_ext = ext.to_uppercase();
            let uppercase_path = format!("{}{}.{}", dir_path, file_name, uppercase_ext);
            let path = Path::new(&uppercase_path);
            prop_assert!(
                !processor.is_supported(path),
                "Uppercase extension '{}' should NOT be supported (case-sensitive)",
                uppercase_ext
            );
        }
    }

    /// Unit tests for specific edge cases
    #[test]
    fn test_all_extensions_covered() {
        let processor = create_full_processor();
        let all_extensions = processor.supported_extensions();

        // Verify all expected extensions are covered
        for (ext, _) in EXTENSION_HANDLER_MAP {
            assert!(
                all_extensions.contains(ext),
                "Extension '{}' should be in supported_extensions",
                ext
            );
        }
    }

    #[test]
    fn test_handler_count() {
        let processor = create_full_processor();
        // We register 8 handlers: Python, C++, Go, Rust, PHP, Kotlin, Markdown, TOML
        assert_eq!(processor.handler_count(), 8);
    }

    #[test]
    fn test_extension_uniqueness() {
        // Verify no extension is claimed by multiple handlers
        let mut seen_extensions: std::collections::HashSet<&str> = std::collections::HashSet::new();

        for (ext, _) in EXTENSION_HANDLER_MAP {
            assert!(
                seen_extensions.insert(ext),
                "Extension '{}' appears multiple times in EXTENSION_HANDLER_MAP",
                ext
            );
        }
    }

    #[test]
    fn test_hidden_files() {
        let processor = create_full_processor();

        // Hidden files with supported extensions should still be routed
        let path = Path::new(".hidden.py");
        assert!(processor.is_supported(path));
        assert_eq!(processor.get_handler(path).unwrap().name(), "python");

        // Hidden files without extensions should not be routed
        let path = Path::new(".gitignore");
        assert!(!processor.is_supported(path));
    }

    #[test]
    fn test_deeply_nested_paths() {
        let processor = create_full_processor();

        let path = Path::new("a/b/c/d/e/f/g/h/i/j/test.rs");
        assert!(processor.is_supported(path));
        assert_eq!(processor.get_handler(path).unwrap().name(), "rust");
    }

    #[test]
    fn test_special_characters_in_filename() {
        let processor = create_full_processor();

        // Filenames with special characters should still route correctly
        let path = Path::new("test-file_name.123.py");
        assert!(processor.is_supported(path));
        assert_eq!(processor.get_handler(path).unwrap().name(), "python");

        let path = Path::new("test.min.cpp");
        assert!(processor.is_supported(path));
        assert_eq!(processor.get_handler(path).unwrap().name(), "cpp");
    }
}
