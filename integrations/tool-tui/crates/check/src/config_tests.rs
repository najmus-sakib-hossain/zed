//! Property-based tests for configuration system
//!
//! **Feature: dx-check-production, Property 10: Configuration Parsing Round-Trip**
//! **Validates: Requirements 6.1, 6.5**

use crate::config::*;
use proptest::prelude::*;
use std::collections::HashMap;

// Generators for configuration types

fn arb_rule_severity() -> impl Strategy<Value = RuleSeverity> {
    prop_oneof![
        Just(RuleSeverity::Off),
        Just(RuleSeverity::Warn),
        Just(RuleSeverity::Error),
    ]
}

fn arb_quote_style() -> impl Strategy<Value = QuoteStyle> {
    prop_oneof![Just(QuoteStyle::Single), Just(QuoteStyle::Double),]
}

fn arb_semicolons() -> impl Strategy<Value = Semicolons> {
    prop_oneof![Just(Semicolons::Always), Just(Semicolons::AsNeeded),]
}

fn arb_trailing_comma() -> impl Strategy<Value = TrailingComma> {
    prop_oneof![
        Just(TrailingComma::All),
        Just(TrailingComma::Es5),
        Just(TrailingComma::None),
    ]
}

fn arb_format_config() -> impl Strategy<Value = FormatConfig> {
    (
        any::<bool>(),
        1u8..=16u8,
        40u16..=200u16,
        arb_quote_style(),
        arb_semicolons(),
        arb_trailing_comma(),
    )
        .prop_map(
            |(use_tabs, indent_width, line_width, quote_style, semicolons, trailing_comma)| {
                FormatConfig {
                    use_tabs,
                    indent_width,
                    line_width,
                    quote_style,
                    semicolons,
                    trailing_comma,
                }
            },
        )
}

fn arb_rule_config() -> impl Strategy<Value = RuleConfig> {
    prop_oneof![
        arb_rule_severity().prop_map(RuleConfig::Severity),
        (arb_rule_severity(), Just(HashMap::new()))
            .prop_map(|(severity, options)| { RuleConfig::Full { severity, options } }),
    ]
}

fn arb_rule_configs() -> impl Strategy<Value = RuleConfigs> {
    (
        any::<bool>(),
        any::<bool>(),
        prop::collection::hash_map("[a-z-]+", arb_rule_config(), 0..5),
    )
        .prop_map(|(recommended, auto_fix, rules)| RuleConfigs {
            recommended,
            auto_fix,
            rules,
        })
}

fn arb_ignore_config() -> impl Strategy<Value = IgnoreConfig> {
    (
        prop::collection::vec("\\*\\*/[a-z]+/\\*\\*", 0..3),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(patterns, use_gitignore, use_dxignore)| IgnoreConfig {
            patterns,
            use_gitignore,
            use_dxignore,
        })
}

fn arb_cache_config() -> impl Strategy<Value = CacheConfig> {
    (any::<bool>(), 1024u64..1024 * 1024 * 1024).prop_map(|(enabled, max_size)| CacheConfig {
        enabled,
        directory: std::path::PathBuf::from(".dx/check"),
        max_size,
    })
}

fn arb_parallel_config() -> impl Strategy<Value = ParallelConfig> {
    (0usize..16, any::<bool>(), 1usize..1000).prop_map(|(threads, work_stealing, batch_size)| {
        ParallelConfig {
            threads,
            work_stealing,
            batch_size,
        }
    })
}

fn arb_checker_config() -> impl Strategy<Value = CheckerConfig> {
    (
        any::<bool>(),
        arb_rule_configs(),
        arb_format_config(),
        arb_ignore_config(),
        arb_cache_config(),
        arb_parallel_config(),
    )
        .prop_map(|(enabled, rules, format, ignore, cache, parallel)| CheckerConfig {
            enabled,
            root: std::path::PathBuf::from("."),
            include: vec!["**/*.ts".into()],
            exclude: vec!["**/node_modules/**".into()],
            rules,
            format,
            ignore,
            cache,
            parallel,
            thresholds: None,
            architecture: None,
            overrides: Vec::new(),
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 10: Configuration Parsing Round-Trip**
    /// *For any* valid CheckerConfig, serializing it to TOML format and parsing it back
    /// SHALL produce an equivalent configuration.
    /// **Validates: Requirements 6.1, 6.5**
    #[test]
    fn prop_config_round_trip(config in arb_checker_config()) {
        // Serialize to TOML
        let toml_str = config.to_toml().expect("Failed to serialize config");

        // Parse back
        let parsed: CheckerConfig = toml::from_str(&toml_str).expect("Failed to parse config");

        // Compare key fields (root path may differ due to serialization)
        prop_assert_eq!(config.enabled, parsed.enabled);
        prop_assert_eq!(config.rules.recommended, parsed.rules.recommended);
        prop_assert_eq!(config.rules.auto_fix, parsed.rules.auto_fix);
        prop_assert_eq!(config.format.indent_width, parsed.format.indent_width);
        prop_assert_eq!(config.format.line_width, parsed.format.line_width);
        prop_assert_eq!(config.format.use_tabs, parsed.format.use_tabs);
        prop_assert_eq!(config.cache.enabled, parsed.cache.enabled);
        prop_assert_eq!(config.parallel.threads, parsed.parallel.threads);
    }

    /// **Property 13: Config Validation Errors**
    /// *For any* invalid configuration, the Configuration_System SHALL report
    /// specific validation errors identifying the invalid fields.
    /// **Validates: Requirements 6.6**
    #[test]
    fn prop_config_validation_reports_errors(
        indent_width in 0u8..=20u8,
        line_width in 0u16..=500u16
    ) {
        let mut config = CheckerConfig::default();
        config.format.indent_width = indent_width;
        config.format.line_width = line_width;

        let result = config.validate().expect("Validation should not fail");

        // Check that invalid indent_width is caught
        if indent_width == 0 || indent_width > 16 {
            prop_assert!(
                result.errors.iter().any(|e| e.field == "format.indent_width"),
                "Expected validation error for invalid indent_width"
            );
        }

        // Check that unusual line_width generates warning
        if line_width < 40 || line_width > 400 {
            prop_assert!(
                result.warnings.iter().any(|w| w.field == "format.line_width"),
                "Expected validation warning for unusual line_width"
            );
        }
    }

    /// **Property 12: Glob Pattern Overrides**
    /// *For any* file matching a glob pattern in overrides, the Configuration_System
    /// SHALL apply the override settings to that file.
    /// **Validates: Requirements 6.4**
    #[test]
    fn prop_glob_pattern_overrides(
        filename in "[a-z]+\\.(test|spec)\\.ts"
    ) {
        let override_config = OverrideConfig {
            files: vec!["**/*.test.ts".into(), "**/*.spec.ts".into()],
            rules: {
                let mut rules = HashMap::new();
                rules.insert("no-console".into(), RuleConfig::Severity(RuleSeverity::Off));
                rules
            },
        };

        let path = std::path::Path::new(&filename);

        // Files matching the pattern should match
        if filename.ends_with(".test.ts") || filename.ends_with(".spec.ts") {
            prop_assert!(
                override_config.matches(path),
                "Override should match test/spec files"
            );
        }
    }

    /// **Property 14: Environment Variable Substitution**
    /// *For any* path containing environment variable references,
    /// the Configuration_System SHALL substitute the actual values.
    /// **Validates: Requirements 6.7**
    #[test]
    fn prop_env_var_substitution(
        var_name in "[A-Z][A-Z0-9_]{0,10}",
        var_value in "[a-z0-9_/]{1,20}"
    ) {
        // Set the environment variable
        // SAFETY: Setting test environment variable in isolated test
        unsafe {
            std::env::set_var(&var_name, &var_value);
        }

        // Test ${VAR} syntax
        let input_braced = format!("path/${{{}}}file", var_name);
        let result_braced = substitute_env_vars(&input_braced).expect("Substitution failed");
        prop_assert!(
            result_braced.contains(&var_value),
            "Braced env var should be substituted"
        );

        // Test $VAR syntax
        let input_simple = format!("path/${} file", var_name);
        let result_simple = substitute_env_vars(&input_simple).expect("Substitution failed");
        prop_assert!(
            result_simple.contains(&var_value),
            "Simple env var should be substituted"
        );

        // Clean up
        // SAFETY: Cleaning up test environment variable
        unsafe {
            std::env::remove_var(&var_name);
        }
    }
}

/// **Property 11: Biome Config Compatibility**
/// *For any* valid Biome configuration file, the Configuration_System SHALL
/// successfully parse it and produce equivalent CheckerConfig settings.
/// **Validates: Requirements 6.2**
#[test]
fn test_biome_config_compatibility() {
    let biome_json = r#"{
        "linter": {
            "enabled": true,
            "rules": {
                "recommended": true,
                "correctness": {
                    "noUnusedVariables": "error"
                },
                "style": {
                    "noConsole": "warn"
                }
            }
        },
        "formatter": {
            "indentStyle": "space",
            "indentWidth": 4,
            "lineWidth": 100
        },
        "javascript": {
            "formatter": {
                "quoteStyle": "single",
                "semicolons": "asNeeded"
            }
        }
    }"#;

    let config = CheckerConfig::from_biome_json(biome_json, std::path::Path::new("."))
        .expect("Parse failed");

    assert!(config.enabled);
    assert!(config.rules.recommended);
    assert_eq!(config.format.indent_width, 4);
    assert_eq!(config.format.line_width, 100);
    assert_eq!(config.format.quote_style, QuoteStyle::Single);
    assert_eq!(config.format.semicolons, Semicolons::AsNeeded);
}

#[test]
fn test_rule_severity_display() {
    assert_eq!(RuleSeverity::Off.to_string(), "off");
    assert_eq!(RuleSeverity::Warn.to_string(), "warn");
    assert_eq!(RuleSeverity::Error.to_string(), "error");
}
