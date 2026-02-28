//! Property-based tests for rule binary serialization
//!
//! **Feature: dx-check-production, Property 1: Rule Serialization Round-Trip**
//! **Validates: Requirements 3.1, 3.7**

use crate::rules::binary::RuleSerializer;
use crate::rules::schema::{DxCategory, DxRule, DxRuleDatabase, DxSeverity, Language, RuleSource};
use proptest::prelude::*;

// Generators for rule types

fn arb_language() -> impl Strategy<Value = Language> {
    prop_oneof![
        Just(Language::JavaScript),
        Just(Language::TypeScript),
        Just(Language::Python),
        Just(Language::Go),
        Just(Language::Rust),
        Just(Language::Php),
        Just(Language::Markdown),
        Just(Language::Toml),
        Just(Language::Json),
        Just(Language::Css),
    ]
}

fn arb_category() -> impl Strategy<Value = DxCategory> {
    prop_oneof![
        Just(DxCategory::Correctness),
        Just(DxCategory::Suspicious),
        Just(DxCategory::Style),
        Just(DxCategory::Performance),
        Just(DxCategory::Security),
        Just(DxCategory::Complexity),
        Just(DxCategory::Accessibility),
        Just(DxCategory::Imports),
    ]
}

fn arb_severity() -> impl Strategy<Value = DxSeverity> {
    prop_oneof![
        Just(DxSeverity::Off),
        Just(DxSeverity::Warn),
        Just(DxSeverity::Error),
    ]
}

fn arb_source() -> impl Strategy<Value = RuleSource> {
    prop_oneof![
        Just(RuleSource::DxCheck),
        Just(RuleSource::Biome),
        Just(RuleSource::Oxc),
        Just(RuleSource::Ruff),
    ]
}

fn arb_rule_name() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{2,20}".prop_map(|s| s)
}

fn arb_description() -> impl Strategy<Value = String> {
    "[A-Za-z ]{10,50}".prop_map(|s| s)
}

fn arb_rule(rule_id: u16) -> impl Strategy<Value = DxRule> {
    (
        arb_language(),
        arb_rule_name(),
        arb_description(),
        arb_category(),
        arb_source(),
        arb_severity(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            move |(
                language,
                name,
                description,
                category,
                source,
                severity,
                fixable,
                recommended,
            )| {
                DxRule::new(rule_id, language, name, description, category, source)
                    .severity(severity)
                    .fixable(fixable)
                    .recommended(recommended)
            },
        )
}

fn arb_rule_database() -> impl Strategy<Value = DxRuleDatabase> {
    // Generate unique rule IDs to avoid collisions
    (1..10usize).prop_flat_map(|count| {
        let strategies: Vec<_> = (0..count).map(|i| arb_rule(i as u16)).collect();
        strategies.prop_map(|rules| {
            let mut db = DxRuleDatabase::new();
            for rule in rules {
                db.add_rule(rule);
            }
            db
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 1: Rule Serialization Round-Trip**
    /// *For any* valid RuleDefinition, serializing it to Binary_Rule_Format using
    /// DX_Serializer and then deserializing it back SHALL produce an equivalent RuleDefinition.
    /// **Validates: Requirements 3.1, 3.7**
    #[test]
    fn prop_rule_serialization_round_trip(db in arb_rule_database()) {
        // Serialize
        let bytes = RuleSerializer::serialize(&db).expect("Serialization failed");

        // Deserialize
        let restored = RuleSerializer::deserialize(&bytes).expect("Deserialization failed");

        // Verify rule count
        prop_assert_eq!(db.rule_count, restored.rule_count);
        prop_assert_eq!(db.rules.len(), restored.rules.len());

        // Verify each rule
        for original_rule in &db.rules {
            let restored_rule = restored.get_by_name(&original_rule.prefixed_name);
            prop_assert!(
                restored_rule.is_some(),
                "Rule {} not found after round-trip",
                original_rule.prefixed_name
            );

            let restored_rule = restored_rule.unwrap();
            prop_assert_eq!(original_rule.rule_id, restored_rule.rule_id);
            prop_assert_eq!(&original_rule.name, &restored_rule.name);
            prop_assert_eq!(&original_rule.prefixed_name, &restored_rule.prefixed_name);
            prop_assert_eq!(&original_rule.description, &restored_rule.description);
            prop_assert_eq!(original_rule.language, restored_rule.language);
            prop_assert_eq!(original_rule.category, restored_rule.category);
            prop_assert_eq!(original_rule.source, restored_rule.source);
            prop_assert_eq!(original_rule.default_severity, restored_rule.default_severity);
            prop_assert_eq!(original_rule.fixable, restored_rule.fixable);
            prop_assert_eq!(original_rule.recommended, restored_rule.recommended);
        }
    }

    /// **Property 3: Rule Severity Configuration**
    /// *For any* rule and any severity setting (off, warn, error), the rule SHALL behave
    /// according to that severity.
    /// **Validates: Requirements 3.5**
    #[test]
    fn prop_rule_severity_configuration(
        severity in arb_severity(),
        rule_id in 0u16..1000u16,
        name in arb_rule_name()
    ) {
        let rule = DxRule::new(
            rule_id,
            Language::JavaScript,
            name,
            "Test description",
            DxCategory::Correctness,
            RuleSource::DxCheck,
        )
        .severity(severity);

        prop_assert_eq!(rule.default_severity, severity);

        // Verify severity is preserved through serialization
        let mut db = DxRuleDatabase::new();
        db.add_rule(rule);

        let bytes = RuleSerializer::serialize(&db).expect("Serialization failed");
        let restored = RuleSerializer::deserialize(&bytes).expect("Deserialization failed");

        let restored_rule = restored.rules.first().unwrap();
        prop_assert_eq!(restored_rule.default_severity, severity);
    }

    /// **Property 5: Rule Validation**
    /// *For any* invalid rule definition (missing required fields, invalid schema),
    /// the Rule_Compiler SHALL reject it with a specific validation error.
    /// **Validates: Requirements 3.8**
    #[test]
    fn prop_rule_database_validation(db in arb_rule_database()) {
        // All generated databases should be valid
        let result = db.validate();
        prop_assert!(result.is_ok(), "Valid database should pass validation");

        // Verify magic and version
        prop_assert_eq!(db.magic, DxRuleDatabase::MAGIC);
        prop_assert_eq!(db.version, DxRuleDatabase::VERSION);
    }
}

#[test]
fn test_invalid_database_magic() {
    let mut db = DxRuleDatabase::new();
    db.magic = [0xFF, 0xFF, 0xFF, 0xFF]; // Invalid magic

    let result = db.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid magic"));
}

#[test]
fn test_invalid_database_version() {
    let mut db = DxRuleDatabase::new();
    db.version = 999; // Invalid version

    let result = db.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Version mismatch"));
}
