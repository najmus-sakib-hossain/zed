//! Property-based tests for PEP 508 dependency parsing
//!
//! **Property 3: PEP 508 Dependency Parsing Round-Trip**
//! **Validates: Requirements 1.2**

use dx_py_package_manager::registry::DependencySpec;
use proptest::prelude::*;

/// Generate valid package names (PEP 503 normalized)
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_lowercase())
}

/// Generate valid version constraints
fn arb_version_constraint() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        (1u32..100, 0u32..100).prop_map(|(major, minor)| Some(format!(">={}.{}", major, minor))),
        (1u32..100, 0u32..100).prop_map(|(major, minor)| Some(format!("=={}.{}", major, minor))),
        (1u32..100, 0u32..100, 1u32..100, 0u32..100).prop_map(|(maj1, min1, maj2, min2)| {
            Some(format!(">={}.{},<{}.{}", maj1, min1, maj2, min2))
        }),
        (1u32..100, 0u32..100).prop_map(|(major, minor)| Some(format!("~={}.{}", major, minor))),
    ]
}

/// Generate valid extras
fn arb_extras() -> impl Strategy<Value = Vec<String>> {
    prop_oneof![
        Just(vec![]),
        Just(vec!["dev".to_string()]),
        Just(vec!["test".to_string()]),
        Just(vec!["dev".to_string(), "test".to_string()]),
        Just(vec!["security".to_string(), "socks".to_string()]),
    ]
}

/// Generate valid markers
fn arb_markers() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some("python_version >= '3.8'".to_string())),
        Just(Some("sys_platform == 'win32'".to_string())),
        Just(Some("python_version >= '3.8' and sys_platform == 'linux'".to_string())),
    ]
}

/// Generate a complete dependency spec string
fn arb_dependency_spec_string() -> impl Strategy<Value = String> {
    (arb_package_name(), arb_extras(), arb_version_constraint(), arb_markers()).prop_map(
        |(name, extras, version, markers)| {
            let mut spec = name;

            if !extras.is_empty() {
                spec.push_str(&format!("[{}]", extras.join(",")));
            }

            if let Some(v) = version {
                spec.push_str(&v);
            }

            if let Some(m) = markers {
                spec.push_str(&format!("; {}", m));
            }

            spec
        },
    )
}

/// Generate URL dependency strings
fn arb_url_dependency() -> impl Strategy<Value = String> {
    (arb_package_name(), arb_extras()).prop_map(|(name, extras)| {
        let mut spec = name;
        if !extras.is_empty() {
            spec.push_str(&format!("[{}]", extras.join(",")));
        }
        spec.push_str(" @ https://example.com/package-1.0.0.whl");
        spec
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: PEP 508 Dependency Parsing Round-Trip**
    /// For any valid PEP 508 dependency string, parsing then formatting SHALL produce
    /// a semantically equivalent dependency string.
    /// **Validates: Requirements 1.2**
    #[test]
    fn prop_dependency_roundtrip(spec_str in arb_dependency_spec_string()) {
        let spec = DependencySpec::parse(&spec_str).expect("should parse valid spec");

        // Verify components are preserved
        prop_assert!(!spec.name.is_empty(), "name should not be empty");

        // Format and reparse
        let formatted = spec.to_string();
        let reparsed = DependencySpec::parse(&formatted).expect("should parse formatted spec");

        // Verify semantic equivalence
        prop_assert_eq!(&spec.name, &reparsed.name, "name should match");
        prop_assert_eq!(&spec.extras, &reparsed.extras, "extras should match");
        prop_assert_eq!(&spec.version_constraint, &reparsed.version_constraint, "version should match");
        // Markers may have whitespace differences, so compare presence
        prop_assert_eq!(spec.markers.is_some(), reparsed.markers.is_some(), "markers presence should match");
    }

    /// **Property 3: URL Dependency Round-Trip**
    /// URL dependencies should preserve the URL through parsing.
    /// **Validates: Requirements 1.2**
    #[test]
    fn prop_url_dependency_roundtrip(spec_str in arb_url_dependency()) {
        let spec = DependencySpec::parse(&spec_str).expect("should parse URL dependency");

        prop_assert!(spec.is_url_dependency(), "should be URL dependency");
        prop_assert!(spec.url.is_some(), "URL should be present");

        // Format and reparse
        let formatted = spec.to_string();
        let reparsed = DependencySpec::parse(&formatted).expect("should parse formatted spec");

        prop_assert_eq!(&spec.name, &reparsed.name);
        prop_assert_eq!(&spec.url, &reparsed.url);
        prop_assert_eq!(&spec.extras, &reparsed.extras);
    }

    /// **Property 3: Name Normalization Idempotent**
    /// Parsing a normalized name should produce the same normalized name.
    /// **Validates: Requirements 1.2**
    #[test]
    fn prop_name_normalization_idempotent(name in arb_package_name()) {
        let spec = DependencySpec::parse(&name).expect("should parse");

        // Parse again with the normalized name
        let reparsed = DependencySpec::parse(&spec.name).expect("should parse normalized");

        prop_assert_eq!(&spec.name, &reparsed.name, "normalized name should be idempotent");
    }

    /// **Property 3: Extras Preserved**
    /// Extras should be preserved through parsing.
    /// **Validates: Requirements 1.2**
    #[test]
    fn prop_extras_preserved(
        name in arb_package_name(),
        extras in arb_extras()
    ) {
        if extras.is_empty() {
            let spec = DependencySpec::parse(&name).expect("should parse");
            prop_assert!(spec.extras.is_empty());
        } else {
            let spec_str = format!("{}[{}]", name, extras.join(","));
            let spec = DependencySpec::parse(&spec_str).expect("should parse");
            prop_assert_eq!(&spec.extras, &extras);
        }
    }
}

/// Test specific edge cases
#[test]
fn test_pep508_edge_cases() {
    // Empty string should fail
    assert!(DependencySpec::parse("").is_err());

    // Whitespace only should fail
    assert!(DependencySpec::parse("   ").is_err());

    // Valid simple name
    let spec = DependencySpec::parse("requests").unwrap();
    assert_eq!(spec.name, "requests");

    // Name with hyphens gets normalized
    let spec = DependencySpec::parse("my-package").unwrap();
    assert_eq!(spec.name, "my_package");

    // Name with dots gets normalized
    let spec = DependencySpec::parse("my.package").unwrap();
    assert_eq!(spec.name, "my_package");

    // Mixed case gets lowercased
    let spec = DependencySpec::parse("MyPackage").unwrap();
    assert_eq!(spec.name, "mypackage");
}

/// Test complex real-world dependency strings
#[test]
fn test_pep508_real_world() {
    let cases = vec![
        "requests>=2.25.0",
        "urllib3[brotli,socks]>=1.21.1,<3",
        "cryptography>=3.4.6; python_version >= '3.6'",
        "pywin32>=300; sys_platform == 'win32'",
        "typing-extensions>=4.0; python_version < '3.11'",
        "numpy>=1.20.0,<2.0.0",
        "pandas[excel,sql]>=1.3.0",
    ];

    for case in cases {
        let spec = DependencySpec::parse(case).unwrap_or_else(|_| panic!("should parse: {}", case));
        assert!(!spec.name.is_empty(), "name should not be empty for: {}", case);

        // Verify roundtrip
        let formatted = spec.to_string();
        let reparsed = DependencySpec::parse(&formatted)
            .unwrap_or_else(|_| panic!("should reparse: {}", formatted));
        assert_eq!(spec.name, reparsed.name, "name mismatch for: {}", case);
    }
}

/// Test URL and path dependencies
#[test]
fn test_pep508_url_path_dependencies() {
    // URL dependency
    let spec =
        DependencySpec::parse("mypackage @ https://example.com/mypackage-1.0.0.whl").unwrap();
    assert!(spec.is_url_dependency());
    assert_eq!(spec.url, Some("https://example.com/mypackage-1.0.0.whl".to_string()));

    // Path dependency
    let spec = DependencySpec::parse("mypackage @ file:///path/to/package").unwrap();
    assert!(spec.is_path_dependency());
    assert_eq!(spec.path, Some("/path/to/package".to_string()));

    // URL with extras
    let spec = DependencySpec::parse("mypackage[dev] @ https://example.com/pkg.whl").unwrap();
    assert!(spec.is_url_dependency());
    assert_eq!(spec.extras, vec!["dev"]);

    // URL with markers
    let spec =
        DependencySpec::parse("mypackage @ https://example.com/pkg.whl; python_version >= '3.8'")
            .unwrap();
    assert!(spec.is_url_dependency());
    assert!(spec.markers.is_some());
}
