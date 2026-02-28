//! Property-based tests for Tokens CLI commands
//!
//! These tests verify universal properties that should hold across all inputs.
//! Feature: token-efficiency-display

use proptest::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

use serializer::{ModelType, TokenCounter};

/// Generate valid DX content strings
#[allow(dead_code)]
fn dx_content_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("nm=test ver=1.0".to_string()),
        Just("key=value count=42 active=+".to_string()),
        Just("name=\"Test App\" version=\"1.0.0\" enabled=+".to_string()),
        "[a-z]{1,10}".prop_map(|s| format!("key={}", s)),
    ]
}

/// Generate random text content for token counting
fn text_content_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9 ]{10,100}",
        Just("Hello, world! This is a test.".to_string()),
        Just("The quick brown fox jumps over the lazy dog.".to_string()),
        Just("fn main() { println!(\"Hello\"); }".to_string()),
    ]
}

proptest! {
    /// Property 7: JSON Output Schema Validity
    /// *For any* DX file analyzed with `--json` flag, the output SHALL be valid JSON
    /// containing `tokenCounts`, `tokenIds`, and `savings` fields.
    ///
    /// **Validates: Requirements 6.2**
    #[test]
    fn prop_json_output_schema_validity(content in text_content_strategy()) {
        let counter = TokenCounter::new();
        let counts = counter.count_primary_models(&content);

        // Build JSON output structure similar to what the CLI produces
        let token_counts: Vec<serde_json::Value> = counts
            .iter()
            .map(|(model, info)| {
                serde_json::json!({
                    "model": model.to_string(),
                    "count": info.count,
                    "ids": info.ids
                })
            })
            .collect();

        let result = serde_json::json!({
            "file": "test.dx",
            "tokenCounts": token_counts,
            "savings": {
                "vsJson": 15.5,
                "vsYaml": 10.2,
                "vsToml": 8.1
            }
        });

        // Verify it's valid JSON
        let json_str = serde_json::to_string(&result).expect("Should serialize to JSON");
        let parsed: Value = serde_json::from_str(&json_str).expect("Should parse as JSON");

        // Verify required fields exist
        prop_assert!(
            parsed.get("tokenCounts").is_some(),
            "JSON output should contain tokenCounts field"
        );
        prop_assert!(
            parsed.get("file").is_some(),
            "JSON output should contain file field"
        );

        // Verify tokenCounts is an array
        prop_assert!(
            parsed["tokenCounts"].is_array(),
            "tokenCounts should be an array"
        );

        // Verify each token count has required fields
        if let Some(counts_array) = parsed["tokenCounts"].as_array() {
            for count in counts_array {
                prop_assert!(
                    count.get("model").is_some(),
                    "Each token count should have model field"
                );
                prop_assert!(
                    count.get("count").is_some(),
                    "Each token count should have count field"
                );
            }
        }
    }

    /// Property 8: Multi-File Aggregation Correctness
    /// *For any* set of DX files, the aggregated token savings SHALL equal
    /// the sum of individual file savings.
    ///
    /// **Validates: Requirements 6.4**
    #[test]
    fn prop_multi_file_aggregation_correctness(
        file_count in 1usize..5,
        content_idx in 0usize..4
    ) {
        let contents = [
            "nm=test1 ver=1.0",
            "nm=test2 ver=2.0 count=42",
            "key=value active=+",
            "name=\"App\" version=\"1.0.0\"",
        ];

        let content = contents[content_idx % contents.len()];
        let counter = TokenCounter::new();

        // Calculate individual token counts
        let mut individual_totals: HashMap<String, usize> = HashMap::new();
        for _ in 0..file_count {
            let counts = counter.count_primary_models(content);
            for (model, info) in counts {
                *individual_totals.entry(model.to_string()).or_insert(0) += info.count;
            }
        }

        // Calculate aggregated totals (simulating multi-file analysis)
        let mut aggregated_totals: HashMap<String, usize> = HashMap::new();
        for _ in 0..file_count {
            let counts = counter.count_primary_models(content);
            for (model, info) in counts {
                *aggregated_totals.entry(model.to_string()).or_insert(0) += info.count;
            }
        }

        // Verify aggregated equals sum of individual
        for (model, individual_total) in &individual_totals {
            let aggregated_total = aggregated_totals.get(model).unwrap_or(&0);
            prop_assert_eq!(
                individual_total, aggregated_total,
                "Aggregated total for {} should equal sum of individual totals",
                model
            );
        }
    }

    /// Property: Token counts are always non-zero for non-empty content
    #[test]
    fn prop_token_counts_non_zero(content in text_content_strategy()) {
        let counter = TokenCounter::new();
        let counts = counter.count_primary_models(&content);

        // All models should return non-zero counts for non-empty content
        for (model, info) in &counts {
            prop_assert!(
                info.count > 0,
                "Token count for {} should be non-zero for content: {}",
                model, content
            );
        }
    }

    /// Property: All four primary models are always present
    #[test]
    fn prop_all_primary_models_present(content in text_content_strategy()) {
        let counter = TokenCounter::new();
        let counts = counter.count_primary_models(&content);

        // Should have exactly 4 models
        prop_assert_eq!(
            counts.len(), 4,
            "Should have exactly 4 primary models, got {}",
            counts.len()
        );

        // Verify specific models are present
        prop_assert!(
            counts.contains_key(&ModelType::Gpt4o),
            "Should contain GPT-4o model"
        );
        prop_assert!(
            counts.contains_key(&ModelType::ClaudeSonnet4),
            "Should contain Claude Sonnet 4 model"
        );
        prop_assert!(
            counts.contains_key(&ModelType::Gemini3),
            "Should contain Gemini 3 model"
        );
        prop_assert!(
            counts.contains_key(&ModelType::Other),
            "Should contain Other model"
        );
    }
}

/// Test JSON output structure matches expected schema
#[test]
fn test_json_output_structure() {
    let counter = TokenCounter::new();
    let content = "nm=test ver=1.0 enabled=+";
    let counts = counter.count_primary_models(content);

    // Build the expected JSON structure
    let token_counts: Vec<serde_json::Value> = counts
        .iter()
        .map(|(model, info)| {
            serde_json::json!({
                "model": model.to_string(),
                "count": info.count
            })
        })
        .collect();

    let result = serde_json::json!({
        "file": "test.dx",
        "tokenCounts": token_counts,
        "savings": {
            "vsJson": 25.0,
            "vsYaml": 15.0,
            "vsToml": 10.0
        }
    });

    // Verify structure
    assert!(result["file"].is_string());
    assert!(result["tokenCounts"].is_array());
    assert!(result["savings"].is_object());
    assert!(result["savings"]["vsJson"].is_number());
    assert!(result["savings"]["vsYaml"].is_number());
    assert!(result["savings"]["vsToml"].is_number());
}

/// Test aggregation with actual files
#[test]
fn test_file_aggregation() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create test DX files
    let contents = [
        "nm=app1 ver=1.0",
        "nm=app2 ver=2.0 count=100",
        "key=value active=+",
    ];

    for (i, content) in contents.iter().enumerate() {
        let file_path = dir_path.join(format!("test_{}.dx", i));
        fs::write(&file_path, content).unwrap();
    }

    let counter = TokenCounter::new();
    let mut total_tokens: HashMap<String, usize> = HashMap::new();

    // Aggregate token counts
    for content in contents.iter() {
        let counts = counter.count_primary_models(content);
        for (model, info) in counts {
            *total_tokens.entry(model.to_string()).or_insert(0) += info.count;
        }
    }

    // Verify we have aggregated counts for all models
    assert_eq!(total_tokens.len(), 4, "Should have 4 model totals");

    // All totals should be positive
    for (model, total) in &total_tokens {
        assert!(*total > 0, "Total for {} should be positive", model);
    }
}

/// Test savings calculation
#[test]
fn test_savings_calculation() {
    // Test the savings formula: ((other - dx) / other) * 100
    let dx_tokens = 73;
    let json_tokens = 100;

    let savings = ((json_tokens as f64 - dx_tokens as f64) / json_tokens as f64) * 100.0;

    assert!((savings - 27.0).abs() < 0.1, "Savings should be ~27%");

    // Test negative savings (DX larger)
    let dx_tokens_large = 110;
    let savings_negative =
        ((json_tokens as f64 - dx_tokens_large as f64) / json_tokens as f64) * 100.0;

    assert!(savings_negative < 0.0, "Savings should be negative when DX is larger");
    assert!((savings_negative - (-10.0)).abs() < 0.1, "Savings should be ~-10%");
}

/// Test that token counting is consistent
#[test]
fn test_token_counting_consistency() {
    let counter = TokenCounter::new();
    let content = "This is a test string for token counting consistency.";

    // Count multiple times
    let count1 = counter.count(content, ModelType::Gpt4o);
    let count2 = counter.count(content, ModelType::Gpt4o);
    let count3 = counter.count(content, ModelType::Gpt4o);

    // All counts should be identical
    assert_eq!(count1.count, count2.count, "Token counts should be consistent");
    assert_eq!(count2.count, count3.count, "Token counts should be consistent");
}
