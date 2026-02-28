//! Property tests for DPP bytecode compilation and loading
//!
//! **Property 7: DPP Bytecode Validity**
//! For any Python source file, the bytecode stored in a DPP package SHALL be valid
//! and produce the same execution results as compiling the source at runtime.
//!
//! **Validates: Requirements 8.1, 8.3**

use dx_py_core::DPP_MAGIC;
use dx_py_package_manager::converter::{
    BytecodeCompiler, BytecodeLoader, DppBuilder, MultiVersionStore, PythonVersion, VersionSelector,
};
use proptest::prelude::*;

/// Generate a valid package name
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9]{1,20}".prop_filter("valid package name", |s| s.len() >= 2 && s.len() <= 21)
}

/// Generate a valid version string
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate a valid Python version
fn arb_python_version() -> impl Strategy<Value = PythonVersion> {
    prop_oneof![
        Just(PythonVersion::new(3, 10, 0)),
        Just(PythonVersion::new(3, 11, 0)),
        Just(PythonVersion::new(3, 12, 0)),
    ]
}

/// Generate valid Python source code
fn arb_python_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("# Empty module\n".to_string()),
        Just("def hello(): pass\n".to_string()),
        Just("class Foo:\n    pass\n".to_string()),
        Just("import os\nimport sys\n".to_string()),
        Just("def add(a, b):\n    return a + b\n".to_string()),
        Just("async def fetch(): pass\n".to_string()),
        Just("x = 42\ny = 'hello'\n".to_string()),
        "[a-zA-Z_][a-zA-Z0-9_]{0,10} = [0-9]{1,5}\n".prop_map(|s| s),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: dx-py-production-ready, Property 7: DPP Bytecode Validity**
    /// For any Python source file, compiling to bytecode and loading back
    /// SHALL produce valid bytecode that can be validated against the source hash.
    ///
    /// **Validates: Requirements 8.1, 8.3**
    #[test]
    fn prop_bytecode_compilation_roundtrip(
        source in arb_python_source(),
        python_version in arb_python_version(),
    ) {
        let source_bytes = source.as_bytes();

        // Compile source to bytecode
        let mut compiler = BytecodeCompiler::new(python_version);
        let compiled = compiler.compile("test.py", source_bytes);

        // Verify source hash matches
        let expected_hash = *blake3::hash(source_bytes).as_bytes();
        prop_assert_eq!(compiled.source_hash, expected_hash,
            "Source hash should match BLAKE3 hash of source content");

        // Verify bytecode is not empty
        prop_assert!(!compiled.bytecode.is_empty(),
            "Compiled bytecode should not be empty");

        // Verify bytecode starts with DPB magic
        prop_assert!(compiled.bytecode.len() >= 4,
            "Bytecode should be at least 4 bytes");
        prop_assert_eq!(&compiled.bytecode[0..4], b"DPB\x01",
            "Bytecode should start with DPB magic");

        // Verify validation works
        let mut loaded = dx_py_package_manager::converter::LoadedBytecode::new(
            compiled.source_path.clone(),
            compiled.source_hash,
            compiled.python_version,
            compiled.bytecode.clone(),
        );

        prop_assert!(loaded.validate(source_bytes),
            "Bytecode should validate against original source");
    }

    /// **Feature: dx-py-production-ready, Property 7: DPP Bytecode Validity**
    /// For any DPP package with Python files, the bytecode section SHALL contain
    /// valid bytecode entries that can be loaded and validated.
    ///
    /// **Validates: Requirements 8.1, 8.3**
    #[test]
    fn prop_dpp_bytecode_section_validity(
        name in arb_package_name(),
        version in arb_version(),
        source in arb_python_source(),
    ) {
        let pkg_name = name.replace('-', "_");
        let file_path = format!("{}/__init__.py", pkg_name);

        // Build DPP package with bytecode compilation enabled
        let mut builder = DppBuilder::new(&name, &version);
        builder.python_requires(">=3.10");
        builder.add_file(&file_path, source.as_bytes().to_vec(), true);

        let data = builder.build();

        // Verify package is valid
        prop_assert!(data.len() >= 64, "Package should have at least header size");
        prop_assert_eq!(&data[0..4], DPP_MAGIC, "Package should have DPP magic");

        // Open package and verify bytecode section exists
        use dx_py_package_manager::DppPackage;
        use std::io::Write;

        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(&data).unwrap();
        temp.flush().unwrap();

        let package = DppPackage::open(temp.path()).unwrap();

        // Verify bytecode section is not empty (has at least count field)
        let bytecode_section = package.bytecode();
        prop_assert!(bytecode_section.len() >= 4,
            "Bytecode section should have at least count field");

        // Parse bytecode count
        let count = u32::from_le_bytes(bytecode_section[0..4].try_into().unwrap());
        prop_assert!(count >= 1,
            "Bytecode section should have at least one entry for the Python file");
    }

    /// **Feature: dx-py-production-ready, Property 7: DPP Bytecode Validity**
    /// For any bytecode loader, loading bytecode from a DPP section and validating
    /// against the original source SHALL succeed.
    ///
    /// **Validates: Requirements 8.1, 8.3**
    #[test]
    fn prop_bytecode_loader_validation(
        source in arb_python_source(),
        python_version in arb_python_version(),
    ) {
        let source_bytes = source.as_bytes();

        // Compile source
        let mut compiler = BytecodeCompiler::new(python_version);
        let compiled = compiler.compile("test.py", source_bytes);

        // Serialize to bytecode section format
        let mut section = Vec::new();
        section.extend_from_slice(&1u32.to_le_bytes()); // count = 1

        let path_bytes = compiled.source_path.as_bytes();
        section.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
        section.extend_from_slice(path_bytes);
        section.extend_from_slice(&compiled.source_hash);
        section.extend_from_slice(&compiled.python_version.to_u32().to_le_bytes());
        section.extend_from_slice(&(compiled.bytecode.len() as u64).to_le_bytes());
        section.extend_from_slice(&compiled.bytecode);

        // Load bytecode
        let mut loader = BytecodeLoader::new(python_version);
        let loaded_count = loader.load_from_section(&section).unwrap();

        prop_assert_eq!(loaded_count, 1, "Should load exactly one bytecode entry");

        // Validate against source
        let is_valid = loader.validate("test.py", source_bytes).unwrap();
        prop_assert!(is_valid, "Loaded bytecode should validate against source");
    }

    /// **Feature: dx-py-production-ready, Property 7: DPP Bytecode Validity**
    /// For any multi-version bytecode store, selecting bytecode for a compatible
    /// version SHALL return valid bytecode.
    ///
    /// **Validates: Requirements 8.1, 8.3**
    #[test]
    fn prop_multi_version_selection(
        source in arb_python_source(),
    ) {
        let source_bytes = source.as_bytes();
        let source_hash = *blake3::hash(source_bytes).as_bytes();

        // Create bytecode for multiple versions
        let mut store = MultiVersionStore::new();

        for minor in [10u8, 11, 12] {
            let version = PythonVersion::new(3, minor, 0);
            let mut compiler = BytecodeCompiler::new(version);
            let compiled = compiler.compile("test.py", source_bytes);

            let loaded = dx_py_package_manager::converter::LoadedBytecode::new(
                compiled.source_path,
                compiled.source_hash,
                compiled.python_version,
                compiled.bytecode,
            );
            store.add(loaded);
        }

        // Verify store has entries
        prop_assert_eq!(store.len(), 1, "Store should have one source file");
        prop_assert_eq!(store.get_all("test.py").unwrap().len(), 3,
            "Store should have 3 version entries");

        // Test version selection
        let selector = VersionSelector::new(PythonVersion::new(3, 12, 0));
        let result = selector.select(&store, "test.py");

        prop_assert!(result.is_found(), "Should find compatible bytecode");

        if let Some(bytecode) = result.bytecode() {
            // Verify selected bytecode has correct hash
            prop_assert_eq!(bytecode.source_hash, source_hash,
                "Selected bytecode should have correct source hash");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unit test for bytecode compilation with specific content
    #[test]
    fn test_bytecode_compilation_specific() {
        let source = b"def hello():\n    print('Hello, World!')\n";
        let version = PythonVersion::new(3, 12, 0);

        let mut compiler = BytecodeCompiler::new(version);
        let compiled = compiler.compile("hello.py", source);

        assert_eq!(compiled.source_path, "hello.py");
        assert_eq!(compiled.python_version, version);
        assert!(!compiled.bytecode.is_empty());

        // Verify hash
        let expected_hash = *blake3::hash(source).as_bytes();
        assert_eq!(compiled.source_hash, expected_hash);
    }

    /// Unit test for DPP package with multiple Python files
    #[test]
    fn test_dpp_multiple_python_files() {
        let mut builder = DppBuilder::new("mypackage", "1.0.0");
        builder.python_requires(">=3.10");
        builder.add_file("mypackage/__init__.py", b"# init".to_vec(), true);
        builder.add_file("mypackage/main.py", b"def main(): pass".to_vec(), true);
        builder.add_file("mypackage/utils.py", b"def helper(): pass".to_vec(), true);

        let data = builder.build();

        assert!(data.len() >= 64);
        assert_eq!(&data[0..4], DPP_MAGIC);

        // Open and verify
        use dx_py_package_manager::DppPackage;
        use std::io::Write;

        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(&data).unwrap();
        temp.flush().unwrap();

        let package = DppPackage::open(temp.path()).unwrap();
        assert_eq!(package.name(), "mypackage");

        // Verify bytecode section has entries
        let bytecode_section = package.bytecode();
        assert!(bytecode_section.len() >= 4);

        let count = u32::from_le_bytes(bytecode_section[0..4].try_into().unwrap());
        assert_eq!(count, 3, "Should have 3 bytecode entries");
    }

    /// Unit test for bytecode validation failure
    #[test]
    fn test_bytecode_validation_failure() {
        let source = b"def hello(): pass";
        let version = PythonVersion::new(3, 12, 0);

        let mut compiler = BytecodeCompiler::new(version);
        let compiled = compiler.compile("test.py", source);

        let mut loaded = dx_py_package_manager::converter::LoadedBytecode::new(
            compiled.source_path,
            compiled.source_hash,
            compiled.python_version,
            compiled.bytecode,
        );

        // Validation should fail with different content
        assert!(!loaded.validate(b"different content"));
    }
}
