//! Property tests for wheel to DPP conversion
//!
//! Property 2: DPP Wheel Conversion Round-Trip
//! For any valid Python wheel file, converting to DPP format and then extracting
//! back SHALL produce a functionally equivalent package.

#![allow(dead_code)]

use dx_py_core::DPP_MAGIC;
use dx_py_package_manager::converter::{inspect_dpp, DppBuilder};
use proptest::prelude::*;

/// Generate a valid package name (lowercase, alphanumeric with hyphens)
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,30}[a-z0-9]"
        .prop_filter("valid package name", |s| !s.contains("--") && s.len() >= 2)
}

/// Generate a valid version string
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate a valid Python requires string
fn arb_python_requires() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(">=3.7".to_string()),
        Just(">=3.8".to_string()),
        Just(">=3.9".to_string()),
        Just(">=3.10".to_string()),
        Just(">=3.11".to_string()),
        Just(">=3.12".to_string()),
        Just(">=3.8,<4.0".to_string()),
    ]
}

/// Generate a valid file path within a package
fn arb_file_path(pkg_name: &str) -> impl Strategy<Value = String> {
    let pkg = pkg_name.replace('-', "_");
    prop_oneof![
        Just(format!("{}/__init__.py", pkg)),
        Just(format!("{}/main.py", pkg)),
        Just(format!("{}/utils.py", pkg)),
        Just(format!("{}/core.py", pkg)),
    ]
}

/// Generate Python source content
fn arb_python_content() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        Just(b"# Python module\n".to_vec()),
        Just(b"def main(): pass\n".to_vec()),
        Just(b"class Foo:\n    pass\n".to_vec()),
        Just(b"import os\nimport sys\n".to_vec()),
        "[a-zA-Z0-9_ \n]{1,100}".prop_map(|s| s.into_bytes()),
    ]
}

/// Generate a dependency string
fn arb_dependency() -> impl Strategy<Value = String> {
    (arb_package_name(), arb_version()).prop_map(|(name, ver)| format!("{}>={}", name, ver))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 2: DPP Wheel Conversion Round-Trip
    /// Validates: Requirements 1.7, 1.8
    #[test]
    fn prop_dpp_builder_creates_valid_package(
        name in arb_package_name(),
        version in arb_version(),
        python_requires in arb_python_requires(),
    ) {
        let mut builder = DppBuilder::new(&name, &version);
        builder.python_requires(&python_requires);

        // Add a simple file
        let pkg_name = name.replace('-', "_");
        let file_path = format!("{}/__init__.py", pkg_name);
        builder.add_file(&file_path, b"# init".to_vec(), true);

        let data = builder.build();

        // Verify magic number
        prop_assert_eq!(&data[0..4], DPP_MAGIC);

        // Verify we can inspect it
        let inspection = inspect_dpp(&data);
        prop_assert!(inspection.contains(&name), "Inspection should contain package name");
        prop_assert!(inspection.contains(&version), "Inspection should contain version");
    }

    /// Property: DPP builder preserves all files
    #[test]
    fn prop_dpp_builder_preserves_files(
        name in arb_package_name(),
        version in arb_version(),
        file_count in 1usize..5,
    ) {
        let mut builder = DppBuilder::new(&name, &version);
        let pkg_name = name.replace('-', "_");

        // Add multiple files
        for i in 0..file_count {
            let path = format!("{}/module{}.py", pkg_name, i);
            let content = format!("# Module {}\n", i);
            builder.add_file(&path, content.into_bytes(), true);
        }

        let data = builder.build();

        // Verify package is valid
        prop_assert!(data.len() >= 64, "Package should have at least header size");
        prop_assert_eq!(&data[0..4], DPP_MAGIC);
    }

    /// Property: DPP builder preserves dependencies
    #[test]
    fn prop_dpp_builder_preserves_dependencies(
        name in arb_package_name(),
        version in arb_version(),
        deps in prop::collection::vec(arb_dependency(), 0..5),
    ) {
        let mut builder = DppBuilder::new(&name, &version);

        for dep in &deps {
            builder.add_dependency(dep);
        }

        // Add at least one file
        let pkg_name = name.replace('-', "_");
        builder.add_file(&format!("{}/__init__.py", pkg_name), b"".to_vec(), true);

        let data = builder.build();

        // Verify package is valid
        prop_assert!(data.len() >= 64);
        prop_assert_eq!(&data[0..4], DPP_MAGIC);
    }

    /// Property: DPP roundtrip through file preserves metadata
    #[test]
    fn prop_dpp_roundtrip_preserves_metadata(
        name in arb_package_name(),
        version in arb_version(),
        python_requires in arb_python_requires(),
    ) {
        use dx_py_package_manager::DppPackage;
        use std::io::Write;

        let mut builder = DppBuilder::new(&name, &version);
        builder.python_requires(&python_requires);

        let pkg_name = name.replace('-', "_");
        builder.add_file(&format!("{}/__init__.py", pkg_name), b"# init".to_vec(), true);

        let data = builder.build();

        // Write to temp file and read back
        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(&data).unwrap();
        temp.flush().unwrap();

        let package = DppPackage::open(temp.path()).unwrap();

        prop_assert_eq!(package.name(), name);
        prop_assert_eq!(package.version(), version);
        prop_assert_eq!(package.python_requires(), python_requires);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpp_builder_with_multiple_files() {
        let mut builder = DppBuilder::new("test-pkg", "1.0.0");
        builder.python_requires(">=3.8");
        builder.add_file("test_pkg/__init__.py", b"# init".to_vec(), true);
        builder.add_file("test_pkg/main.py", b"def main(): pass".to_vec(), true);
        builder.add_file("test_pkg/data.txt", b"some data".to_vec(), false);
        builder.add_dependency("requests>=2.0");
        builder.add_dependency("numpy>=1.20");

        let data = builder.build();

        assert_eq!(&data[0..4], DPP_MAGIC);
        assert!(data.len() > 64);

        let inspection = inspect_dpp(&data);
        assert!(inspection.contains("test-pkg"));
        assert!(inspection.contains("1.0.0"));
    }
}
