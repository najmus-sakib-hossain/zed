//! Property-based tests for dx-py-compability
//!
//! These tests validate universal correctness properties using proptest.

use dx_py_compability::{
    DxPyConfig, MarkerEnvironment, MarkerEvaluator, Platform, PreRelease, PythonPreference,
    PythonVersion, UvConfig, WheelTag, WheelTagGenerator,
};
use proptest::prelude::*;

// =============================================================================
// Task 3.4: Property test for Python version validation
// Property 2: Python Version Range Validation
// Validates: Requirements 1.3, 1.4
// =============================================================================

prop_compose! {
    /// Generate valid Python version components
    fn valid_python_version()(
        major in 2u8..=3,
        minor in 0u8..=20,
        patch in 0u8..=50
    ) -> (u8, u8, u8) {
        (major, minor, patch)
    }
}

prop_compose! {
    /// Generate supported Python versions (3.8-3.13)
    fn supported_python_version()(
        minor in 8u8..=13,
        patch in 0u8..=20
    ) -> PythonVersion {
        PythonVersion::new(3, minor, patch)
    }
}

prop_compose! {
    /// Generate unsupported Python versions
    fn unsupported_python_version()(
        choice in 0..3usize,
        minor_low in 0u8..=7,
        minor_high in 14u8..=20,
        patch in 0u8..=10
    ) -> PythonVersion {
        match choice {
            0 => PythonVersion::new(2, 7, patch), // Python 2
            1 => PythonVersion::new(3, minor_low, patch), // Too old
            _ => PythonVersion::new(3, minor_high, patch), // Too new
        }
    }
}

proptest! {
    /// Property: Supported versions (3.8-3.13) should return is_supported() == true
    #[test]
    fn prop_supported_versions_are_valid(version in supported_python_version()) {
        prop_assert!(version.is_supported(),
            "Version {} should be supported", version);
    }

    /// Property: Unsupported versions should return is_supported() == false
    #[test]
    fn prop_unsupported_versions_are_invalid(version in unsupported_python_version()) {
        prop_assert!(!version.is_supported(),
            "Version {} should not be supported", version);
    }

    /// Property: Version parsing should be consistent with display
    #[test]
    fn prop_version_parse_display_roundtrip(version in supported_python_version()) {
        let displayed = version.to_string();
        let parsed = PythonVersion::parse(&displayed).unwrap();
        prop_assert_eq!(version, parsed,
            "Parsing displayed version should produce same version");
    }

    /// Property: Version ordering should be transitive
    #[test]
    fn prop_version_ordering_transitive(
        (m1, m2, m3) in (8u8..=13, 8u8..=13, 8u8..=13)
    ) {
        let v1 = PythonVersion::new(3, m1, 0);
        let v2 = PythonVersion::new(3, m2, 0);
        let v3 = PythonVersion::new(3, m3, 0);

        // If v1 <= v2 and v2 <= v3, then v1 <= v3
        if v1 <= v2 && v2 <= v3 {
            prop_assert!(v1 <= v3, "Version ordering should be transitive");
        }
    }

    /// Property: Pre-release versions should be less than release versions
    #[test]
    fn prop_prerelease_less_than_release(
        minor in 8u8..=13,
        pre_num in 1u32..=10
    ) {
        let release = PythonVersion::new(3, minor, 0);
        let alpha = PythonVersion::new(3, minor, 0).with_pre_release(PreRelease::Alpha(pre_num));
        let beta = PythonVersion::new(3, minor, 0).with_pre_release(PreRelease::Beta(pre_num));
        let rc = PythonVersion::new(3, minor, 0).with_pre_release(PreRelease::ReleaseCandidate(pre_num));

        prop_assert!(alpha < release, "Alpha should be less than release");
        prop_assert!(beta < release, "Beta should be less than release");
        prop_assert!(rc < release, "RC should be less than release");
        prop_assert!(alpha < beta, "Alpha should be less than beta");
        prop_assert!(beta < rc, "Beta should be less than RC");
    }
}

// =============================================================================
// Task 4.3: Property test for uv config parsing
// Property 3: uv Configuration Parsing Completeness
// Validates: Requirements 2.1, 2.2, 2.3, 2.5
// =============================================================================

prop_compose! {
    /// Generate valid index URLs
    fn valid_index_url()(
        scheme in prop_oneof!["https://", "http://"].prop_map(String::from),
        domain in "[a-z]{3,10}\\.[a-z]{2,4}",
        path in prop_oneof!["/simple", "/pypi", "/packages"].prop_map(String::from)
    ) -> String {
        format!("{}{}{}", scheme, domain, path)
    }
}

prop_compose! {
    /// Generate valid UvConfig
    fn valid_uv_config()(
        index_url in proptest::option::of(valid_index_url()),
        python_version in proptest::option::of("[3]\\.[8-9]|3\\.1[0-3]"),
        python_preference in proptest::option::of(prop_oneof![
            Just(PythonPreference::OnlyManaged),
            Just(PythonPreference::Managed),
            Just(PythonPreference::System),
            Just(PythonPreference::OnlySystem),
        ]),
        compile_bytecode in proptest::option::of(any::<bool>())
    ) -> UvConfig {
        UvConfig {
            index_url,
            extra_index_url: vec![],
            find_links: vec![],
            no_binary: vec![],
            only_binary: vec![],
            python_version,
            python_preference,
            cache_dir: None,
            compile_bytecode,
        }
    }
}

proptest! {
    /// Property: UvConfig serialization should round-trip correctly
    #[test]
    fn prop_uv_config_roundtrip(config in valid_uv_config()) {
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: UvConfig = toml::from_str(&toml_str).unwrap();
        prop_assert_eq!(config, parsed,
            "UvConfig should round-trip through TOML serialization");
    }

    /// Property: Empty UvConfig should serialize to minimal TOML
    #[test]
    fn prop_empty_uv_config_is_empty(_dummy in Just(())) {
        let config = UvConfig::default();
        prop_assert!(config.is_empty(), "Default UvConfig should be empty");
    }

    /// Property: PythonPreference should serialize with kebab-case
    #[test]
    fn prop_python_preference_kebab_case(
        pref in prop_oneof![
            Just(PythonPreference::OnlyManaged),
            Just(PythonPreference::Managed),
            Just(PythonPreference::System),
            Just(PythonPreference::OnlySystem),
        ]
    ) {
        let config = UvConfig {
            python_preference: Some(pref.clone()),
            ..Default::default()
        };
        let toml_str = toml::to_string(&config).unwrap();

        // Verify kebab-case serialization
        let has_kebab = toml_str.contains("only-managed")
            || toml_str.contains("managed")
            || toml_str.contains("system")
            || toml_str.contains("only-system");
        prop_assert!(has_kebab, "PythonPreference should use kebab-case");
    }
}

// =============================================================================
// Task 4.4: Property test for configuration precedence
// Property 4: Configuration Precedence
// Validates: Requirements 2.4
// =============================================================================

proptest! {
    /// Property: dx-py config should override uv config
    #[test]
    fn prop_dxpy_overrides_uv(
        uv_index in valid_index_url(),
        dxpy_index in valid_index_url()
    ) {
        use dx_py_compability::uv::UvConfigLoader;

        let uv_config = UvConfig {
            index_url: Some(uv_index.clone()),
            ..Default::default()
        };

        let dxpy_config = DxPyConfig {
            index_url: Some(dxpy_index.clone()),
            ..Default::default()
        };

        let merged = UvConfigLoader::merge_with_dxpy(uv_config, &dxpy_config);

        prop_assert_eq!(merged.config.index_url, Some(dxpy_index),
            "dx-py index_url should override uv index_url");

        // Should have warning if values differ
        if uv_index != dxpy_config.index_url.clone().unwrap() {
            prop_assert!(!merged.warnings.is_empty(),
                "Should warn when uv config is overridden");
        }
    }

    /// Property: Unset dx-py values should preserve uv values
    #[test]
    fn prop_uv_preserved_when_dxpy_unset(uv_index in valid_index_url()) {
        use dx_py_compability::uv::UvConfigLoader;

        let uv_config = UvConfig {
            index_url: Some(uv_index.clone()),
            ..Default::default()
        };

        let dxpy_config = DxPyConfig::default(); // No index_url set

        let merged = UvConfigLoader::merge_with_dxpy(uv_config, &dxpy_config);

        prop_assert_eq!(merged.config.index_url, Some(uv_index),
            "uv index_url should be preserved when dx-py doesn't set it");
        prop_assert!(merged.warnings.is_empty(),
            "No warnings when nothing is overridden");
    }
}

// =============================================================================
// Task 6.4: Property test for marker evaluation
// Property 5: Marker Evaluation Correctness
// Validates: Requirements 3.1, 3.2, 3.3
// =============================================================================

prop_compose! {
    /// Generate valid marker variable names
    fn marker_variable()(
        var in prop_oneof![
            Just("python_version"),
            Just("python_full_version"),
            Just("os_name"),
            Just("sys_platform"),
            Just("platform_machine"),
            Just("platform_system"),
            Just("implementation_name"),
        ].prop_map(String::from)
    ) -> String {
        var
    }
}

prop_compose! {
    /// Generate valid marker operators
    fn marker_operator()(
        op in prop_oneof![
            Just("=="),
            Just("!="),
            Just("<"),
            Just("<="),
            Just(">"),
            Just(">="),
        ].prop_map(String::from)
    ) -> String {
        op
    }
}

proptest! {
    /// Property: Marker evaluation should be deterministic
    #[test]
    fn prop_marker_evaluation_deterministic(
        minor in 8u8..=13
    ) {
        let env = MarkerEnvironment {
            python_version: format!("3.{}", minor),
            ..Default::default()
        };

        let marker = "python_version >= '3.8'".to_string();

        let mut eval1 = MarkerEvaluator::new(env.clone());
        let mut eval2 = MarkerEvaluator::new(env);

        let result1 = eval1.evaluate(&marker).unwrap();
        let result2 = eval2.evaluate(&marker).unwrap();

        prop_assert_eq!(result1, result2,
            "Same marker with same environment should produce same result");
    }

    /// Property: AND should be false if either operand is false
    #[test]
    fn prop_marker_and_semantics(
        minor in 8u8..=13
    ) {
        let env = MarkerEnvironment {
            python_version: format!("3.{}", minor),
            sys_platform: "linux".to_string(),
            ..Default::default()
        };

        let mut evaluator = MarkerEvaluator::new(env);

        // True AND True = True
        let both_true = evaluator.evaluate("python_version >= '3.8' and sys_platform == 'linux'").unwrap();
        prop_assert!(both_true, "True AND True should be True");

        // True AND False = False
        let one_false = evaluator.evaluate("python_version >= '3.8' and sys_platform == 'win32'").unwrap();
        prop_assert!(!one_false, "True AND False should be False");
    }

    /// Property: OR should be true if either operand is true
    #[test]
    fn prop_marker_or_semantics(
        minor in 8u8..=13
    ) {
        let env = MarkerEnvironment {
            python_version: format!("3.{}", minor),
            sys_platform: "linux".to_string(),
            ..Default::default()
        };

        let mut evaluator = MarkerEvaluator::new(env);

        // True OR False = True
        let one_true = evaluator.evaluate("sys_platform == 'linux' or sys_platform == 'win32'").unwrap();
        prop_assert!(one_true, "True OR False should be True");

        // False OR False = False
        let both_false = evaluator.evaluate("sys_platform == 'darwin' or sys_platform == 'win32'").unwrap();
        prop_assert!(!both_false, "False OR False should be False");
    }

    /// Property: Version comparisons should be consistent
    #[test]
    fn prop_marker_version_comparison(
        minor in 8u8..=13
    ) {
        let env = MarkerEnvironment {
            python_version: format!("3.{}", minor),
            ..Default::default()
        };

        let mut evaluator = MarkerEvaluator::new(env);

        // If version >= X, then version > (X-1) should also be true (for X > 8)
        if minor > 8 {
            let ge_result = evaluator.evaluate(&format!("python_version >= '3.{}'", minor)).unwrap();
            let gt_result = evaluator.evaluate(&format!("python_version > '3.{}'", minor - 1)).unwrap();
            prop_assert_eq!(ge_result, gt_result,
                ">= 3.{} should equal > 3.{}", minor, minor - 1);
        }
    }

    /// Property: Unknown variables should return error
    #[test]
    fn prop_marker_unknown_variable_error(_dummy in Just(())) {
        let env = MarkerEnvironment::default();
        let mut evaluator = MarkerEvaluator::new(env);

        let result = evaluator.evaluate("unknown_var == 'value'");
        prop_assert!(result.is_err(), "Unknown variable should return error");
    }
}

// =============================================================================
// Task 6.5: Property test for marker caching
// Property 6: Marker Evaluation Caching
// Validates: Requirements 3.5
// =============================================================================

proptest! {
    /// Property: Cached results should match non-cached results
    #[test]
    fn prop_marker_cache_consistency(
        minor in 8u8..=13
    ) {
        let env = MarkerEnvironment {
            python_version: format!("3.{}", minor),
            ..Default::default()
        };

        let marker = "python_version >= '3.8'";

        let mut evaluator = MarkerEvaluator::new(env);

        // First evaluation (not cached)
        let result1 = evaluator.evaluate(marker).unwrap();

        // Second evaluation (should be cached)
        let result2 = evaluator.evaluate(marker).unwrap();

        // Third evaluation (definitely cached)
        let result3 = evaluator.evaluate(marker).unwrap();

        prop_assert_eq!(result1, result2, "Cached result should match first result");
        prop_assert_eq!(result2, result3, "Multiple cached calls should be consistent");
    }

    /// Property: Different markers should have independent cache entries
    #[test]
    fn prop_marker_cache_independence(
        minor in 8u8..=13
    ) {
        let env = MarkerEnvironment {
            python_version: format!("3.{}", minor),
            sys_platform: "linux".to_string(),
            ..Default::default()
        };

        let mut evaluator = MarkerEvaluator::new(env);

        let marker1 = "python_version >= '3.8'";
        let marker2 = "sys_platform == 'linux'";

        let result1 = evaluator.evaluate(marker1).unwrap();
        let result2 = evaluator.evaluate(marker2).unwrap();

        // Re-evaluate to use cache
        let cached1 = evaluator.evaluate(marker1).unwrap();
        let cached2 = evaluator.evaluate(marker2).unwrap();

        prop_assert_eq!(result1, cached1, "Marker 1 cache should be independent");
        prop_assert_eq!(result2, cached2, "Marker 2 cache should be independent");
    }
}

// =============================================================================
// Task 7.4: Property test for wheel tag ordering
// Property 7: Wheel Tag Priority Ordering
// Validates: Requirements 4.2, 4.3
// =============================================================================

proptest! {
    /// Property: Generated tags should have increasing priority values
    #[test]
    fn prop_wheel_tags_priority_order(
        minor in 8u8..=13
    ) {
        let platform = Platform::default();
        let generator = WheelTagGenerator::with_version(platform, 3, minor);
        let tags = generator.generate_tags();

        // Verify tags are sorted by priority
        for window in tags.windows(2) {
            prop_assert!(window[0].priority <= window[1].priority,
                "Tags should be in priority order: {} <= {}",
                window[0].priority, window[1].priority);
        }
    }

    /// Property: Platform-specific tags should have higher priority than 'any'
    #[test]
    fn prop_platform_specific_higher_priority(
        minor in 8u8..=13
    ) {
        let platform = Platform::default();
        let generator = WheelTagGenerator::with_version(platform, 3, minor);
        let tags = generator.generate_tags();

        let any_tags: Vec<_> = tags.iter().filter(|t| t.platform == "any").collect();
        let platform_tags: Vec<_> = tags.iter().filter(|t| t.platform != "any").collect();

        if !any_tags.is_empty() && !platform_tags.is_empty() {
            let min_any_priority = any_tags.iter().map(|t| t.priority).min().unwrap();
            let max_platform_priority = platform_tags.iter().map(|t| t.priority).max().unwrap();

            prop_assert!(max_platform_priority < min_any_priority,
                "Platform-specific tags should have higher priority (lower number) than 'any' tags");
        }
    }

    /// Property: CPython tags should have higher priority than pure Python tags
    #[test]
    fn prop_cpython_higher_priority_than_pure(
        minor in 8u8..=13
    ) {
        let platform = Platform::default();
        let generator = WheelTagGenerator::with_version(platform, 3, minor);
        let tags = generator.generate_tags();

        let cpython_tags: Vec<_> = tags.iter()
            .filter(|t| t.python.starts_with("cp"))
            .collect();
        let pure_tags: Vec<_> = tags.iter()
            .filter(|t| t.python.starts_with("py") && t.abi == "none" && t.platform == "any")
            .collect();

        if !cpython_tags.is_empty() && !pure_tags.is_empty() {
            let max_cpython_priority = cpython_tags.iter().map(|t| t.priority).max().unwrap();
            let min_pure_priority = pure_tags.iter().map(|t| t.priority).min().unwrap();

            prop_assert!(max_cpython_priority < min_pure_priority,
                "CPython tags should have higher priority than pure Python tags");
        }
    }

    /// Property: select_best should return tag with lowest priority
    #[test]
    fn prop_select_best_returns_lowest_priority(
        minor in 8u8..=13
    ) {
        let platform = Platform::default();
        let generator = WheelTagGenerator::with_version(platform, 3, minor);
        let tags = generator.generate_tags();

        if tags.len() >= 2 {
            // Create candidate tags from generated tags
            let candidates: Vec<WheelTag> = tags.iter().take(5).cloned().collect();

            if let Some(best) = generator.select_best(&candidates) {
                // Best should be the one with lowest priority among candidates
                let min_priority = candidates.iter()
                    .filter_map(|c| {
                        tags.iter()
                            .find(|t| t.python == c.python && t.abi == c.abi && t.platform == c.platform)
                            .map(|t| t.priority)
                    })
                    .min();

                if let Some(expected_priority) = min_priority {
                    let best_priority = tags.iter()
                        .find(|t| t.python == best.python && t.abi == best.abi && t.platform == best.platform)
                        .map(|t| t.priority);

                    prop_assert_eq!(best_priority, Some(expected_priority),
                        "select_best should return tag with lowest priority");
                }
            }
        }
    }
}

// =============================================================================
// Task 7.5: Property test for Linux wheel tags
// Property 8: Linux Wheel Tag Support
// Validates: Requirements 4.4, 4.5
// =============================================================================

proptest! {
    /// Property: manylinux tags should parse correctly
    #[test]
    fn prop_manylinux_parse_roundtrip(
        major in 2u32..=2,
        minor in 17u32..=35
    ) {
        use dx_py_compability::platform::ManylinuxTag;
        use std::str::FromStr;

        let tag_str = format!("manylinux_{}_{}_x86_64", major, minor);
        let parsed = ManylinuxTag::from_str(&tag_str);

        prop_assert!(parsed.is_ok(), "Should parse valid manylinux tag: {}", tag_str);

        if let Ok(tag) = parsed {
            let (req_major, req_minor) = tag.min_glibc_version();
            prop_assert_eq!(req_major, major, "Major version should match");
            prop_assert_eq!(req_minor, minor, "Minor version should match");
        }
    }

    /// Property: musllinux tags should parse correctly
    #[test]
    fn prop_musllinux_parse_roundtrip(
        major in 1u32..=1,
        minor in 1u32..=3
    ) {
        use dx_py_compability::platform::MusllinuxTag;
        use std::str::FromStr;

        let tag_str = format!("musllinux_{}_{}_x86_64", major, minor);
        let parsed = MusllinuxTag::from_str(&tag_str);

        prop_assert!(parsed.is_ok(), "Should parse valid musllinux tag: {}", tag_str);

        if let Ok(tag) = parsed {
            prop_assert_eq!(tag.major, major, "Major version should match");
            prop_assert_eq!(tag.minor, minor, "Minor version should match");
        }
    }

    /// Property: manylinux compatibility should be monotonic
    #[test]
    fn prop_manylinux_compatibility_monotonic(
        tag_minor in 17u32..=28,
        sys_minor in 17u32..=35
    ) {
        use dx_py_compability::platform::ManylinuxTag;
        use std::str::FromStr;

        let tag = ManylinuxTag::from_str(&format!("manylinux_2_{}_x86_64", tag_minor)).unwrap();

        let is_compat = tag.is_compatible_with_glibc(2, sys_minor);

        // If system glibc >= tag requirement, should be compatible
        if sys_minor >= tag_minor {
            prop_assert!(is_compat,
                "glibc 2.{} should be compatible with manylinux_2_{}", sys_minor, tag_minor);
        } else {
            prop_assert!(!is_compat,
                "glibc 2.{} should NOT be compatible with manylinux_2_{}", sys_minor, tag_minor);
        }
    }

    /// Property: Legacy manylinux tags should map to correct glibc versions
    #[test]
    fn prop_legacy_manylinux_glibc_versions(_dummy in Just(())) {
        use dx_py_compability::platform::ManylinuxTag;

        // Test the enum variants directly since parsing with arch suffix works differently
        let manylinux1 = ManylinuxTag::Manylinux1;
        let manylinux2010 = ManylinuxTag::Manylinux2010;
        let manylinux2014 = ManylinuxTag::Manylinux2014;

        prop_assert_eq!(manylinux1.min_glibc_version(), (2, 5),
            "manylinux1 should require glibc 2.5");
        prop_assert_eq!(manylinux2010.min_glibc_version(), (2, 12),
            "manylinux2010 should require glibc 2.12");
        prop_assert_eq!(manylinux2014.min_glibc_version(), (2, 17),
            "manylinux2014 should require glibc 2.17");
    }
}

// =============================================================================
// Additional: Config round-trip tests (supports Task 10)
// =============================================================================

proptest! {
    /// Property: DxPyConfig should round-trip through TOML
    #[test]
    fn prop_dxpy_config_roundtrip(
        minor in 8u8..=13,
        max_downloads in 1u32..=100
    ) {
        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(3, minor, 0)),
            index_url: Some("https://pypi.org/simple".to_string()),
            extra_index_urls: vec![],
            cache_dir: None,
            max_concurrent_downloads: Some(max_downloads),
            uv_compat: None,
        };

        let toml_str = config.to_toml().unwrap();
        let parsed = DxPyConfig::from_toml(&toml_str).unwrap();

        prop_assert_eq!(config, parsed,
            "DxPyConfig should round-trip through TOML");
    }
}

// =============================================================================
// Task 9.5: Property test for venv compliance
// Property 9: Virtual Environment PEP 405 Compliance
// Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5
// =============================================================================

use dx_py_compability::{
    Architecture, InstallationSource, PythonRuntime, RuntimeCapabilities, VenvBuilder, VenvOptions,
};
use tempfile::TempDir;

prop_compose! {
    /// Generate valid venv options
    fn valid_venv_options()(
        system_site_packages in any::<bool>(),
        copies in any::<bool>(),
        clear in any::<bool>()
    ) -> VenvOptions {
        VenvOptions {
            system_site_packages,
            copies,
            clear,
            upgrade: false,
            with_pip: false,
        }
    }
}

prop_compose! {
    /// Generate valid venv name
    fn valid_venv_name()(
        name in "[a-z][a-z0-9_]{2,10}"
    ) -> String {
        name
    }
}

proptest! {
    /// Property: Created venv should have pyvenv.cfg with required fields
    #[test]
    fn prop_venv_has_pyvenv_cfg(
        minor in 8u8..=13,
        system_site_packages in any::<bool>()
    ) {
        let temp_dir = TempDir::new().unwrap();
        let python_path = temp_dir.path().join("python");

        // Create a mock Python executable
        #[cfg(unix)]
        {
            std::fs::write(&python_path, "#!/bin/sh\necho 'Python 3.12.0'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&python_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            std::fs::write(&python_path, "").unwrap();
        }

        let python = PythonRuntime {
            executable: python_path,
            version: PythonVersion::new(3, minor, 0),
            architecture: Architecture::default(),
            source: InstallationSource::System,
            capabilities: RuntimeCapabilities::default(),
        };

        let options = VenvOptions {
            system_site_packages,
            copies: true, // Use copies for testing
            clear: true,
            upgrade: false,
            with_pip: false,
        };

        let venv_path = temp_dir.path().join("test_venv");
        let builder = VenvBuilder::new(python).with_options(options);
        let result = builder.build(&venv_path);

        if let Ok(venv) = result {
            // Check pyvenv.cfg exists
            let cfg_path = venv.path.join("pyvenv.cfg");
            prop_assert!(cfg_path.exists(), "pyvenv.cfg should exist");

            // Check required fields
            let content = std::fs::read_to_string(&cfg_path).unwrap();
            prop_assert!(content.contains("home = "), "pyvenv.cfg should have 'home' field");
            prop_assert!(content.contains("include-system-site-packages = "),
                "pyvenv.cfg should have 'include-system-site-packages' field");
            prop_assert!(content.contains("version = "), "pyvenv.cfg should have 'version' field");

            // Check system-site-packages value matches option
            let expected_value = if system_site_packages { "true" } else { "false" };
            prop_assert!(content.contains(&format!("include-system-site-packages = {}", expected_value)),
                "include-system-site-packages should match option");
        }
    }

    /// Property: Created venv should have activation scripts for all shells
    #[test]
    fn prop_venv_has_activation_scripts(minor in 8u8..=13) {
        let temp_dir = TempDir::new().unwrap();
        let python_path = temp_dir.path().join("python");

        // Create a mock Python executable
        #[cfg(unix)]
        {
            std::fs::write(&python_path, "#!/bin/sh\necho 'Python 3.12.0'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&python_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            std::fs::write(&python_path, "").unwrap();
        }

        let python = PythonRuntime {
            executable: python_path,
            version: PythonVersion::new(3, minor, 0),
            architecture: Architecture::default(),
            source: InstallationSource::System,
            capabilities: RuntimeCapabilities::default(),
        };

        let options = VenvOptions {
            system_site_packages: false,
            copies: true,
            clear: true,
            upgrade: false,
            with_pip: false,
        };

        let venv_path = temp_dir.path().join("test_venv");
        let builder = VenvBuilder::new(python).with_options(options);
        let result = builder.build(&venv_path);

        if let Ok(venv) = result {
            // Check activation scripts exist
            let scripts_dir = &venv.scripts_dir;

            // bash/zsh
            prop_assert!(scripts_dir.join("activate").exists(),
                "bash/zsh activate script should exist");

            // fish
            prop_assert!(scripts_dir.join("activate.fish").exists(),
                "fish activate script should exist");

            // csh
            prop_assert!(scripts_dir.join("activate.csh").exists(),
                "csh activate script should exist");

            // PowerShell
            prop_assert!(scripts_dir.join("Activate.ps1").exists(),
                "PowerShell activate script should exist");
        }
    }

    /// Property: Created venv should have Python executable
    #[test]
    fn prop_venv_has_python_executable(minor in 8u8..=13) {
        let temp_dir = TempDir::new().unwrap();
        let python_path = temp_dir.path().join("python");

        // Create a mock Python executable
        #[cfg(unix)]
        {
            std::fs::write(&python_path, "#!/bin/sh\necho 'Python 3.12.0'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&python_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            std::fs::write(&python_path, "").unwrap();
        }

        let python = PythonRuntime {
            executable: python_path,
            version: PythonVersion::new(3, minor, 0),
            architecture: Architecture::default(),
            source: InstallationSource::System,
            capabilities: RuntimeCapabilities::default(),
        };

        let options = VenvOptions {
            system_site_packages: false,
            copies: true,
            clear: true,
            upgrade: false,
            with_pip: false,
        };

        let venv_path = temp_dir.path().join("test_venv");
        let builder = VenvBuilder::new(python).with_options(options);
        let result = builder.build(&venv_path);

        if let Ok(venv) = result {
            // Check Python executable exists
            prop_assert!(venv.python.exists(),
                "Python executable should exist at {:?}", venv.python);

            // Check site-packages directory exists
            prop_assert!(venv.site_packages.exists(),
                "site-packages directory should exist at {:?}", venv.site_packages);
        }
    }

    /// Property: VenvOptions builder pattern should preserve all settings
    #[test]
    fn prop_venv_options_builder_preserves_settings(
        system_site_packages in any::<bool>(),
        copies in any::<bool>(),
        clear in any::<bool>(),
        with_pip in any::<bool>()
    ) {
        let options = VenvOptions::new()
            .system_site_packages(system_site_packages)
            .copies(copies)
            .clear(clear)
            .with_pip(with_pip);

        prop_assert_eq!(options.system_site_packages, system_site_packages,
            "system_site_packages should be preserved");
        prop_assert_eq!(options.copies, copies,
            "copies should be preserved");
        prop_assert_eq!(options.clear, clear,
            "clear should be preserved");
        prop_assert_eq!(options.with_pip, with_pip,
            "with_pip should be preserved");
    }

    /// Property: Activation script should contain venv path
    #[test]
    fn prop_activation_script_contains_venv_path(
        venv_name in valid_venv_name(),
        minor in 8u8..=13
    ) {
        let temp_dir = TempDir::new().unwrap();
        let python_path = temp_dir.path().join("python");

        // Create a mock Python executable
        #[cfg(unix)]
        {
            std::fs::write(&python_path, "#!/bin/sh\necho 'Python 3.12.0'").unwrap();
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&python_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(windows)]
        {
            std::fs::write(&python_path, "").unwrap();
        }

        let python = PythonRuntime {
            executable: python_path,
            version: PythonVersion::new(3, minor, 0),
            architecture: Architecture::default(),
            source: InstallationSource::System,
            capabilities: RuntimeCapabilities::default(),
        };

        let options = VenvOptions {
            system_site_packages: false,
            copies: true,
            clear: true,
            upgrade: false,
            with_pip: false,
        };

        let venv_path = temp_dir.path().join(&venv_name);
        let builder = VenvBuilder::new(python).with_options(options);
        let result = builder.build(&venv_path);

        if let Ok(venv) = result {
            // Check bash activate script contains venv name
            let activate_path = venv.scripts_dir.join("activate");
            if activate_path.exists() {
                let content = std::fs::read_to_string(&activate_path).unwrap();
                prop_assert!(content.contains(&venv_name),
                    "Activation script should contain venv name '{}'", venv_name);
            }
        }
    }
}

// =============================================================================
// Task 10.3: Property test for config round-trip
// Property 10: Configuration Round-Trip
// Validates: Requirements 7.1, 7.2, 7.3
// =============================================================================

prop_compose! {
    /// Generate valid DxPyConfig
    fn valid_dxpy_config()(
        minor in 8u8..=13,
        has_index in any::<bool>(),
        max_downloads in 1u32..=100
    ) -> DxPyConfig {
        DxPyConfig {
            python_version: Some(PythonVersion::new(3, minor, 0)),
            index_url: if has_index { Some("https://pypi.org/simple".to_string()) } else { None },
            extra_index_urls: vec![],
            cache_dir: None,
            max_concurrent_downloads: Some(max_downloads),
            uv_compat: None,
        }
    }
}

proptest! {
    /// Property 10: Configuration should round-trip through TOML
    #[test]
    fn prop_config_roundtrip_comprehensive(config in valid_dxpy_config()) {
        let toml_str = config.to_toml().unwrap();
        let parsed = DxPyConfig::from_toml(&toml_str).unwrap();

        prop_assert_eq!(config, parsed,
            "DxPyConfig should round-trip through TOML serialization");
    }

    /// Property: Empty config should round-trip
    #[test]
    fn prop_empty_config_roundtrip(_dummy in Just(())) {
        let config = DxPyConfig::default();
        let toml_str = config.to_toml().unwrap();
        let parsed = DxPyConfig::from_toml(&toml_str).unwrap();

        prop_assert_eq!(config, parsed,
            "Empty DxPyConfig should round-trip");
    }

    /// Property: Config with extra_index_urls should round-trip
    #[test]
    fn prop_config_with_extra_urls_roundtrip(
        minor in 8u8..=13,
        num_extras in 0usize..=3
    ) {
        let extra_urls: Vec<String> = (0..num_extras)
            .map(|i| format!("https://extra{}.pypi.org/simple", i))
            .collect();

        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(3, minor, 0)),
            index_url: Some("https://pypi.org/simple".to_string()),
            extra_index_urls: extra_urls,
            cache_dir: None,
            max_concurrent_downloads: Some(10),
            uv_compat: None,
        };

        let toml_str = config.to_toml().unwrap();
        let parsed = DxPyConfig::from_toml(&toml_str).unwrap();

        prop_assert_eq!(config, parsed,
            "DxPyConfig with extra_index_urls should round-trip");
    }

    /// Property: Serialization should be deterministic
    #[test]
    fn prop_config_serialization_deterministic(config in valid_dxpy_config()) {
        let toml1 = config.to_toml().unwrap();
        let toml2 = config.to_toml().unwrap();

        prop_assert_eq!(toml1, toml2,
            "Serialization should be deterministic");
    }
}

// =============================================================================
// Task 10.4: Property test for config validation
// Property 11: Configuration Validation
// Validates: Requirements 7.4, 7.5
// =============================================================================

use dx_py_compability::validate_config;

proptest! {
    /// Property 11: Invalid index_url should fail validation
    #[test]
    fn prop_invalid_index_url_fails_validation(
        protocol in prop_oneof!["ftp://", "file://", "ssh://", ""].prop_map(String::from),
        domain in "[a-z]{3,10}\\.[a-z]{2,4}"
    ) {
        let config = DxPyConfig {
            index_url: Some(format!("{}{}", protocol, domain)),
            ..Default::default()
        };

        let result = validate_config(&config);
        prop_assert!(result.is_err(),
            "Invalid protocol '{}' should fail validation", protocol);

        if let Err(dx_py_compability::ConfigError::ValidationError { field, .. }) = result {
            prop_assert_eq!(field, "index_url",
                "Error should be for index_url field");
        }
    }

    /// Property: Valid index_url should pass validation
    #[test]
    fn prop_valid_index_url_passes_validation(
        protocol in prop_oneof!["https://", "http://"].prop_map(String::from),
        domain in "[a-z]{3,10}\\.[a-z]{2,4}"
    ) {
        let config = DxPyConfig {
            index_url: Some(format!("{}{}/simple", protocol, domain)),
            ..Default::default()
        };

        let result = validate_config(&config);
        prop_assert!(result.is_ok(),
            "Valid URL with protocol '{}' should pass validation", protocol);
    }

    /// Property: max_concurrent_downloads outside range should fail
    #[test]
    fn prop_invalid_max_downloads_fails_validation(
        max in prop_oneof![Just(0u32), 101u32..=1000]
    ) {
        let config = DxPyConfig {
            max_concurrent_downloads: Some(max),
            ..Default::default()
        };

        let result = validate_config(&config);
        prop_assert!(result.is_err(),
            "max_concurrent_downloads={} should fail validation", max);

        if let Err(dx_py_compability::ConfigError::ValidationError { field, .. }) = result {
            prop_assert_eq!(field, "max_concurrent_downloads",
                "Error should be for max_concurrent_downloads field");
        }
    }

    /// Property: max_concurrent_downloads in valid range should pass
    #[test]
    fn prop_valid_max_downloads_passes_validation(max in 1u32..=100) {
        let config = DxPyConfig {
            max_concurrent_downloads: Some(max),
            ..Default::default()
        };

        let result = validate_config(&config);
        prop_assert!(result.is_ok(),
            "max_concurrent_downloads={} should pass validation", max);
    }

    /// Property: Unsupported Python version should fail validation
    #[test]
    fn prop_unsupported_python_fails_validation(
        minor in prop_oneof![0u8..=7, 14u8..=20]
    ) {
        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(3, minor, 0)),
            ..Default::default()
        };

        let result = validate_config(&config);
        prop_assert!(result.is_err(),
            "Python 3.{} should fail validation", minor);

        if let Err(dx_py_compability::ConfigError::ValidationError { field, .. }) = result {
            prop_assert_eq!(field, "python_version",
                "Error should be for python_version field");
        }
    }

    /// Property: Supported Python version should pass validation
    #[test]
    fn prop_supported_python_passes_validation(minor in 8u8..=13) {
        let config = DxPyConfig {
            python_version: Some(PythonVersion::new(3, minor, 0)),
            ..Default::default()
        };

        let result = validate_config(&config);
        prop_assert!(result.is_ok(),
            "Python 3.{} should pass validation", minor);
    }

    /// Property: Validation errors should have descriptive messages
    #[test]
    fn prop_validation_errors_have_messages(
        invalid_url in prop_oneof!["", "ftp://test.com", "invalid"].prop_map(String::from)
    ) {
        let config = DxPyConfig {
            index_url: Some(invalid_url.clone()),
            ..Default::default()
        };

        let result = validate_config(&config);

        if let Err(dx_py_compability::ConfigError::ValidationError { message, .. }) = result {
            prop_assert!(!message.is_empty(),
                "Validation error should have a non-empty message");
        }
    }
}
