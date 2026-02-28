//! Property-based tests for wheel tag parsing and selection
//!
//! **Property 5: Wheel Tag Parsing**
//! **Validates: Requirements 4.4**
//!
//! **Property 6: Wheel Selection Priority**
//! **Validates: Requirements 4.5, 4.6**

use dx_py_core::wheel::{
    select_best_wheel, Arch, ManylinuxVersion, Os, PlatformEnvironment, PythonImpl, WheelTag,
};
use proptest::prelude::*;

/// Generate valid package names (PEP 503 normalized)
fn arb_package_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,20}".prop_map(|s| s.to_lowercase())
}

/// Generate valid version strings
fn arb_version() -> impl Strategy<Value = String> {
    (1u32..100, 0u32..100, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}

/// Generate Python tags
fn arb_python_tag() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("py3".to_string()),
        Just("py2.py3".to_string()),
        (3u32..4, 8u32..15).prop_map(|(major, minor)| format!("cp{}{}", major, minor)),
        (3u32..4, 8u32..15).prop_map(|(major, minor)| format!("py{}{}", major, minor)),
    ]
}

/// Generate ABI tags
fn arb_abi_tag() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("none".to_string()),
        Just("abi3".to_string()),
        (3u32..4, 8u32..15).prop_map(|(major, minor)| format!("cp{}{}", major, minor)),
    ]
}

/// Generate platform tags
fn arb_platform_tag() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("any".to_string()),
        Just("win_amd64".to_string()),
        Just("win32".to_string()),
        Just("win_arm64".to_string()),
        Just("macosx_10_9_x86_64".to_string()),
        Just("macosx_11_0_arm64".to_string()),
        Just("macosx_10_9_universal2".to_string()),
        Just("manylinux_2_17_x86_64".to_string()),
        Just("manylinux_2_17_aarch64".to_string()),
        Just("manylinux2014_x86_64".to_string()),
        Just("manylinux1_x86_64".to_string()),
        Just("linux_x86_64".to_string()),
    ]
}

/// Generate valid wheel filenames
fn arb_wheel_filename() -> impl Strategy<Value = String> {
    (
        arb_package_name(),
        arb_version(),
        arb_python_tag(),
        arb_abi_tag(),
        arb_platform_tag(),
    )
        .prop_map(|(name, version, python, abi, platform)| {
            format!("{}-{}-{}-{}-{}.whl", name, version, python, abi, platform)
        })
}

/// Generate platform environments
fn arb_platform_env() -> impl Strategy<Value = PlatformEnvironment> {
    (
        prop_oneof![Just(Os::Windows), Just(Os::Linux), Just(Os::MacOs),],
        prop_oneof![Just(Arch::X86_64), Just(Arch::Aarch64), Just(Arch::X86),],
        8u32..15,
    )
        .prop_map(|(os, arch, minor)| {
            let manylinux = if os == Os::Linux {
                Some(ManylinuxVersion::MANYLINUX2014)
            } else {
                None
            };
            let macos_version = if os == Os::MacOs { Some((10, 9)) } else { None };
            PlatformEnvironment {
                os,
                arch,
                python_impl: PythonImpl::CPython,
                python_version: (3, minor),
                abi: format!("cp3{}", minor),
                manylinux,
                macos_version,
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 5: Wheel Tag Parsing**
    /// For any valid wheel filename, parsing SHALL extract the correct name, version, and tags.
    /// **Validates: Requirements 4.4**
    #[test]
    fn prop_wheel_tag_roundtrip(filename in arb_wheel_filename()) {
        let wheel = WheelTag::parse(&filename).expect("should parse valid wheel filename");

        // Verify components are non-empty
        prop_assert!(!wheel.name.is_empty(), "name should not be empty");
        prop_assert!(!wheel.version.is_empty(), "version should not be empty");
        prop_assert!(!wheel.python_tags.is_empty(), "python_tags should not be empty");
        prop_assert!(!wheel.abi_tags.is_empty(), "abi_tags should not be empty");
        prop_assert!(!wheel.platform_tags.is_empty(), "platform_tags should not be empty");

        // Verify roundtrip produces valid filename
        let regenerated = wheel.to_filename();
        prop_assert!(regenerated.ends_with(".whl"), "should end with .whl");

        // Parse again and verify consistency
        let reparsed = WheelTag::parse(&regenerated).expect("should parse regenerated filename");
        prop_assert_eq!(wheel.name, reparsed.name);
        prop_assert_eq!(wheel.version, reparsed.version);
    }

    /// **Property 5: Wheel Tag Parsing - Name Normalization**
    /// Package names should be normalized according to PEP 503.
    /// **Validates: Requirements 4.4**
    #[test]
    fn prop_wheel_name_normalized(filename in arb_wheel_filename()) {
        let wheel = WheelTag::parse(&filename).expect("should parse");

        // Name should be lowercase
        let name_lower = wheel.name.to_lowercase();
        prop_assert_eq!(&wheel.name, &name_lower);

        // Name should not contain hyphens or dots (normalized to underscores)
        prop_assert!(!wheel.name.contains('-'), "name should not contain hyphens");
        prop_assert!(!wheel.name.contains('.'), "name should not contain dots");
    }

    /// **Property 6: Wheel Selection Priority - Platform Specific Preferred**
    /// Platform-specific wheels should have higher specificity than universal wheels.
    /// **Validates: Requirements 4.5, 4.6**
    #[test]
    fn prop_platform_specific_preferred(env in arb_platform_env()) {
        let pure_python = WheelTag {
            name: "test_pkg".to_string(),
            version: "1.0.0".to_string(),
            build: None,
            python_tags: vec!["py3".to_string()],
            abi_tags: vec!["none".to_string()],
            platform_tags: vec!["any".to_string()],
        };

        let platform_specific = WheelTag {
            name: "test_pkg".to_string(),
            version: "1.0.0".to_string(),
            build: None,
            python_tags: vec![format!("cp3{}", env.python_version.1)],
            abi_tags: vec![format!("cp3{}", env.python_version.1)],
            platform_tags: env.platform_tags().into_iter().take(1).collect(),
        };

        let pure_score = pure_python.specificity_score(&env);
        let specific_score = platform_specific.specificity_score(&env);

        prop_assert!(
            specific_score >= pure_score,
            "platform-specific ({}) should score >= pure python ({})",
            specific_score,
            pure_score
        );
    }

    /// **Property 6: Wheel Selection Priority - Manylinux Ordering**
    /// Newer manylinux versions should be preferred over older ones.
    /// **Validates: Requirements 4.5, 4.6**
    #[test]
    fn prop_manylinux_ordering(minor in 8u32..15) {
        let env = PlatformEnvironment {
            os: Os::Linux,
            arch: Arch::X86_64,
            python_impl: PythonImpl::CPython,
            python_version: (3, minor),
            abi: format!("cp3{}", minor),
            manylinux: Some(ManylinuxVersion::MANYLINUX_2_28),
            macos_version: None,
        };

        let older = WheelTag {
            name: "test_pkg".to_string(),
            version: "1.0.0".to_string(),
            build: None,
            python_tags: vec![format!("cp3{}", minor)],
            abi_tags: vec![format!("cp3{}", minor)],
            platform_tags: vec!["manylinux_2_17_x86_64".to_string()],
        };

        let newer = WheelTag {
            name: "test_pkg".to_string(),
            version: "1.0.0".to_string(),
            build: None,
            python_tags: vec![format!("cp3{}", minor)],
            abi_tags: vec![format!("cp3{}", minor)],
            platform_tags: vec!["manylinux_2_28_x86_64".to_string()],
        };

        // Both should be compatible
        prop_assert!(older.is_compatible(&env), "older manylinux should be compatible");
        prop_assert!(newer.is_compatible(&env), "newer manylinux should be compatible");

        // Newer should have higher score
        let older_score = older.specificity_score(&env);
        let newer_score = newer.specificity_score(&env);

        prop_assert!(
            newer_score >= older_score,
            "newer manylinux ({}) should score >= older ({})",
            newer_score,
            older_score
        );
    }

    /// **Property 6: Wheel Selection - Best Wheel Selection**
    /// select_best_wheel should return the most specific compatible wheel.
    /// **Validates: Requirements 4.5, 4.6**
    #[test]
    fn prop_best_wheel_selection(env in arb_platform_env()) {
        let wheels = vec![
            WheelTag {
                name: "pkg".to_string(),
                version: "1.0.0".to_string(),
                build: None,
                python_tags: vec!["py3".to_string()],
                abi_tags: vec!["none".to_string()],
                platform_tags: vec!["any".to_string()],
            },
            WheelTag {
                name: "pkg".to_string(),
                version: "1.0.0".to_string(),
                build: None,
                python_tags: vec![format!("py3{}", env.python_version.1)],
                abi_tags: vec!["none".to_string()],
                platform_tags: vec!["any".to_string()],
            },
        ];

        let best = select_best_wheel(&wheels, &env);
        prop_assert!(best.is_some(), "should find a compatible wheel");

        // The selected wheel should be compatible
        let selected = best.unwrap();
        prop_assert!(selected.is_compatible(&env), "selected wheel should be compatible");

        // The selected wheel should have the highest score among compatible wheels
        let selected_score = selected.specificity_score(&env);
        for wheel in &wheels {
            if wheel.is_compatible(&env) {
                prop_assert!(
                    selected_score >= wheel.specificity_score(&env),
                    "selected wheel should have highest score"
                );
            }
        }
    }
}

/// **Property 5: Wheel Tag Parsing - Specific Cases**
#[test]
fn test_wheel_parsing_specific_cases() {
    // Test various real-world wheel filenames
    let cases = vec![
        ("requests-2.31.0-py3-none-any.whl", "requests", "2.31.0"),
        ("numpy-1.26.0-cp312-cp312-manylinux_2_17_x86_64.whl", "numpy", "1.26.0"),
        ("six-1.16.0-py2.py3-none-any.whl", "six", "1.16.0"),
        (
            "cryptography-41.0.0-cp312-abi3-manylinux_2_28_x86_64.whl",
            "cryptography",
            "41.0.0",
        ),
        ("pywin32-306-cp312-cp312-win_amd64.whl", "pywin32", "306"),
    ];

    for (filename, expected_name, expected_version) in cases {
        let wheel =
            WheelTag::parse(filename).unwrap_or_else(|_| panic!("should parse {}", filename));
        assert_eq!(wheel.name, expected_name, "name mismatch for {}", filename);
        assert_eq!(wheel.version, expected_version, "version mismatch for {}", filename);
    }
}

/// **Property 6: Wheel Selection - Incompatible Wheels Filtered**
#[test]
fn test_incompatible_wheels_filtered() {
    let env = PlatformEnvironment {
        os: Os::Linux,
        arch: Arch::X86_64,
        python_impl: PythonImpl::CPython,
        python_version: (3, 12),
        abi: "cp312".to_string(),
        manylinux: Some(ManylinuxVersion::MANYLINUX2014),
        macos_version: None,
    };

    // Windows-only wheel should not be selected on Linux
    let wheels = vec![WheelTag::parse("pkg-1.0.0-cp312-cp312-win_amd64.whl").unwrap()];

    let best = select_best_wheel(&wheels, &env);
    assert!(best.is_none(), "Windows wheel should not be compatible on Linux");
}

/// **Property 6: Manylinux Compatibility**
#[test]
fn test_manylinux_compatibility_chain() {
    // System with glibc 2.17 (manylinux2014)
    let system = ManylinuxVersion::MANYLINUX2014;

    // Should be compatible with older
    assert!(system.is_compatible_with(&ManylinuxVersion::MANYLINUX1));
    assert!(system.is_compatible_with(&ManylinuxVersion::MANYLINUX2010));
    assert!(system.is_compatible_with(&ManylinuxVersion::MANYLINUX2014));

    // Should NOT be compatible with newer
    assert!(!system.is_compatible_with(&ManylinuxVersion::MANYLINUX_2_24));
    assert!(!system.is_compatible_with(&ManylinuxVersion::MANYLINUX_2_28));
}
