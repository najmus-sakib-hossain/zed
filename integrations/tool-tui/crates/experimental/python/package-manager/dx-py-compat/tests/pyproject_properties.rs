//! Property-based tests for pyproject.toml handling
//!
//! Property 8: pyproject.toml Round-Trip Conversion
//! For any valid pyproject.toml file, converting to binary pyproject.dx format
//! and back to TOML SHALL preserve all project metadata, dependencies, and configuration.

use proptest::prelude::*;

use dx_py_compat::{
    convert_from_binary, convert_to_binary, BuildSystem, ProjectSection, PyProjectToml,
};

/// Generate arbitrary package names
fn arb_package_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9_-]{2,20}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// Generate arbitrary version strings
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate arbitrary dependency strings
fn arb_dependency() -> impl Strategy<Value = String> {
    (arb_package_name(), prop::option::of(arb_version())).prop_map(|(name, version)| {
        if let Some(v) = version {
            format!("{}>={}", name, v)
        } else {
            name
        }
    })
}

/// Generate arbitrary project section
fn arb_project_section() -> impl Strategy<Value = ProjectSection> {
    (
        arb_package_name(),
        prop::option::of(arb_version()),
        prop::option::of(prop::string::string_regex("[A-Za-z0-9 ]{0,50}").unwrap()),
        prop::option::of(prop::collection::vec(arb_dependency(), 0..5)),
    )
        .prop_map(|(name, version, description, dependencies)| ProjectSection {
            name,
            version,
            description,
            dependencies,
            ..Default::default()
        })
}

/// Generate arbitrary build system
fn arb_build_system() -> impl Strategy<Value = BuildSystem> {
    (
        prop::collection::vec(arb_dependency(), 0..3),
        prop::option::of(prop::string::string_regex("[a-z_]+\\.[a-z_]+").unwrap()),
    )
        .prop_map(|(requires, build_backend)| BuildSystem {
            requires,
            build_backend,
            backend_path: None,
        })
}

/// Generate arbitrary pyproject.toml
fn arb_pyproject() -> impl Strategy<Value = PyProjectToml> {
    (prop::option::of(arb_project_section()), prop::option::of(arb_build_system())).prop_map(
        |(project, build_system)| PyProjectToml {
            project,
            tool: None, // Tool section has complex nested structure
            build_system,
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 8: pyproject.toml Round-Trip Conversion
    /// Validates: Requirements 12.4, 12.5
    ///
    /// For any valid pyproject.toml, converting to binary and back
    /// SHALL preserve all project metadata.
    #[test]
    fn prop_pyproject_binary_roundtrip(pyproject in arb_pyproject()) {
        // Convert to binary
        let binary = convert_to_binary(&pyproject).unwrap();

        // Convert back
        let restored = convert_from_binary(&binary).unwrap();

        // Compare project section
        prop_assert_eq!(
            pyproject.project.as_ref().map(|p| &p.name),
            restored.project.as_ref().map(|p| &p.name)
        );
        prop_assert_eq!(
            pyproject.project.as_ref().and_then(|p| p.version.as_ref()),
            restored.project.as_ref().and_then(|p| p.version.as_ref())
        );
        prop_assert_eq!(
            pyproject.project.as_ref().and_then(|p| p.description.as_ref()),
            restored.project.as_ref().and_then(|p| p.description.as_ref())
        );
        prop_assert_eq!(
            pyproject.project.as_ref().and_then(|p| p.dependencies.as_ref()),
            restored.project.as_ref().and_then(|p| p.dependencies.as_ref())
        );

        // Compare build system
        prop_assert_eq!(
            pyproject.build_system.as_ref().map(|b| &b.requires),
            restored.build_system.as_ref().map(|b| &b.requires)
        );
        prop_assert_eq!(
            pyproject.build_system.as_ref().and_then(|b| b.build_backend.as_ref()),
            restored.build_system.as_ref().and_then(|b| b.build_backend.as_ref())
        );
    }

    /// Property: Binary format has valid magic bytes
    #[test]
    fn prop_binary_has_valid_magic(pyproject in arb_pyproject()) {
        let binary = convert_to_binary(&pyproject).unwrap();

        prop_assert!(binary.len() >= 4);
        prop_assert_eq!(&binary[0..4], b"DXPY");
    }

    /// Property: Empty pyproject converts correctly
    #[test]
    fn prop_empty_pyproject_roundtrip(_seed in any::<u64>()) {
        let empty = PyProjectToml::default();
        let binary = convert_to_binary(&empty).unwrap();
        let restored = convert_from_binary(&binary).unwrap();

        prop_assert!(restored.project.is_none());
        prop_assert!(restored.build_system.is_none());
    }

    /// Property: Dependencies are preserved exactly
    #[test]
    fn prop_dependencies_preserved(
        name in arb_package_name(),
        version in arb_version(),
        deps in prop::collection::vec(arb_dependency(), 1..10),
    ) {
        let pyproject = PyProjectToml {
            project: Some(ProjectSection {
                name,
                version: Some(version),
                dependencies: Some(deps.clone()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let binary = convert_to_binary(&pyproject).unwrap();
        let restored = convert_from_binary(&binary).unwrap();

        let restored_deps = restored.project.as_ref()
            .and_then(|p| p.dependencies.as_ref())
            .cloned()
            .unwrap_or_default();

        prop_assert_eq!(deps, restored_deps);
    }

    /// Property: TOML serialization is valid
    #[test]
    fn prop_toml_serialization_valid(pyproject in arb_pyproject()) {
        // Should be able to serialize to TOML
        let toml_str = pyproject.to_toml();
        prop_assert!(toml_str.is_ok());

        // Should be able to parse it back
        if let Ok(toml_str) = toml_str {
            let parsed = PyProjectToml::parse(&toml_str);
            prop_assert!(parsed.is_ok());
        }
    }
}

#[test]
fn test_complex_pyproject_roundtrip() {
    let toml = r#"
[project]
name = "complex-package"
version = "2.0.0"
description = "A complex test package"
requires-python = ">=3.8"
dependencies = [
    "requests>=2.28",
    "flask>=2.0,<3.0",
    "numpy",
]

[project.optional-dependencies]
dev = ["pytest>=7.0", "black", "mypy"]
docs = ["sphinx", "sphinx-rtd-theme"]

[project.scripts]
my-cli = "my_package.cli:main"

[build-system]
requires = ["setuptools>=61.0", "wheel"]
build-backend = "setuptools.build_meta"
"#;

    let original = PyProjectToml::parse(toml).unwrap();

    // Convert to binary and back
    let binary = convert_to_binary(&original).unwrap();
    let restored = convert_from_binary(&binary).unwrap();

    // Verify key fields
    assert_eq!(original.name(), restored.name());
    assert_eq!(original.version(), restored.version());
    assert_eq!(original.dependencies(), restored.dependencies());

    // Verify optional dependencies
    assert_eq!(original.optional_dependencies("dev"), restored.optional_dependencies("dev"));

    // Verify build system
    assert_eq!(
        original.build_system.as_ref().map(|b| &b.requires),
        restored.build_system.as_ref().map(|b| &b.requires)
    );
}

#[test]
fn test_invalid_binary_rejected() {
    // Too small
    let result = convert_from_binary(&[0u8; 10]);
    assert!(result.is_err());

    // Wrong magic
    let mut bad_magic = vec![0u8; 64];
    bad_magic[0..4].copy_from_slice(b"XXXX");
    let result = convert_from_binary(&bad_magic);
    assert!(result.is_err());
}
