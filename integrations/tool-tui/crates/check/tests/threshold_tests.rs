//! Tests for score threshold checking functionality

use dx_check::config::{CheckerConfig, ThresholdConfig};
use dx_check::scoring::{
    Category, ProjectScore, Severity, ThresholdChecker, ThresholdResult, Violation,
};
use std::path::PathBuf;

#[test]
fn test_threshold_config_to_checker() {
    let config = ThresholdConfig {
        min_total_score: Some(400),
        min_formatting: Some(90),
        min_linting: Some(85),
        min_security: Some(95),
        min_design_patterns: Some(80),
        min_structure_docs: Some(85),
    };

    let checker = config.to_checker();

    // Test with passing score
    let mut score = ProjectScore::new(10);
    score.total_score = 450;

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(failures) => {
            panic!("Should pass with score 450, but failed: {:?}", failures);
        }
    }

    // Test with failing total score
    score.total_score = 350;
    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail with score 350"),
        ThresholdResult::Fail(failures) => {
            assert!(!failures.is_empty());
            assert!(failures[0].contains("Total score"));
        }
    }
}

#[test]
fn test_threshold_config_category_thresholds() {
    let config = ThresholdConfig {
        min_total_score: None,
        min_formatting: None,
        min_linting: None,
        min_security: Some(95),
        min_design_patterns: None,
        min_structure_docs: None,
    };

    let checker = config.to_checker();

    let mut score = ProjectScore::new(10);

    // Add violations to bring security score below threshold
    for _ in 0..2 {
        score.add_violation(Violation {
            category: Category::Security,
            severity: Severity::High,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 1,
            rule_id: "no-unsafe".to_string(),
            message: "Unsafe code".to_string(),
            points: 5,
        });
    }

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail security threshold"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].contains("security"));
        }
    }
}

#[test]
fn test_threshold_config_exit_codes() {
    let config = ThresholdConfig {
        min_total_score: Some(400),
        min_formatting: None,
        min_linting: None,
        min_security: None,
        min_design_patterns: None,
        min_structure_docs: None,
    };

    let checker = config.to_checker();

    // Passing score should return exit code 0
    let mut score = ProjectScore::new(10);
    score.total_score = 450;
    assert_eq!(checker.exit_code(&score), 0);

    // Failing score should return exit code 1
    score.total_score = 350;
    assert_eq!(checker.exit_code(&score), 1);
}

#[test]
fn test_threshold_config_no_thresholds() {
    let config = ThresholdConfig::default();
    let checker = config.to_checker();

    // With no thresholds, any score should pass
    let mut score = ProjectScore::new(10);
    score.total_score = 0;

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Should pass with no thresholds configured"),
    }
}

#[test]
fn test_threshold_config_multiple_category_failures() {
    let config = ThresholdConfig {
        min_total_score: Some(450),
        min_formatting: None,
        min_linting: Some(90),
        min_security: Some(95),
        min_design_patterns: None,
        min_structure_docs: None,
    };

    let checker = config.to_checker();

    let mut score = ProjectScore::new(10);

    // Add violations to bring linting below threshold (need 11 points to go from 100 to 89)
    for _ in 0..3 {
        score.add_violation(Violation {
            category: Category::Linting,
            severity: Severity::High,
            file: PathBuf::from("test.js"),
            line: 1,
            column: 1,
            rule_id: "no-unused-vars".to_string(),
            message: "Unused variable".to_string(),
            points: 5,
        });
    }

    // Add violations to bring security below threshold (need 6 points to go from 100 to 94)
    for _ in 0..2 {
        score.add_violation(Violation {
            category: Category::Security,
            severity: Severity::High,
            file: PathBuf::from("test.js"),
            line: 1,
            column: 1,
            rule_id: "no-eval".to_string(),
            message: "Eval usage".to_string(),
            points: 5,
        });
    }

    // Total score should now be 500 - 15 - 10 = 475, which is above 450
    // But linting (100-15=85) and security (100-10=90) are below their thresholds

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail category thresholds"),
        ThresholdResult::Fail(failures) => {
            // Should have 2 failures: linting and security (total passes)
            assert!(
                failures.len() >= 2,
                "Expected at least 2 failures, got {}: {:?}",
                failures.len(),
                failures
            );
            assert!(failures.iter().any(|f| f.contains("linting")));
            assert!(failures.iter().any(|f| f.contains("security")));
        }
    }
}

#[test]
fn test_threshold_config_serialization() {
    let config = ThresholdConfig {
        min_total_score: Some(400),
        min_formatting: Some(90),
        min_linting: Some(85),
        min_security: Some(95),
        min_design_patterns: Some(80),
        min_structure_docs: Some(85),
    };

    // Serialize to TOML
    let toml_str = toml::to_string(&config).unwrap();

    // Deserialize back
    let deserialized: ThresholdConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(config, deserialized);
}

#[test]
fn test_threshold_config_in_checker_config() {
    let mut config = CheckerConfig::default();
    config.thresholds = Some(ThresholdConfig {
        min_total_score: Some(400),
        min_security: Some(95),
        ..Default::default()
    });

    // Serialize and deserialize
    let toml_str = config.to_toml().unwrap();
    let parsed: CheckerConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(config.thresholds, parsed.thresholds);
}

#[test]
fn test_threshold_config_boundary_values() {
    // Test with boundary values
    let config = ThresholdConfig {
        min_total_score: Some(500),    // Maximum possible
        min_formatting: Some(100),     // Maximum per category
        min_linting: Some(0),          // Minimum
        min_security: None,            // Not set
        min_design_patterns: Some(50), // Mid-range
        min_structure_docs: None,
    };

    let checker = config.to_checker();

    // Perfect score should pass
    let score = ProjectScore::new(10);
    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Perfect score should pass"),
    }

    // Score with any violation should fail the total threshold
    let mut score = ProjectScore::new(10);
    score.add_violation(Violation {
        category: Category::Formatting,
        severity: Severity::Low,
        file: PathBuf::from("test.js"),
        line: 1,
        column: 1,
        rule_id: "indent".to_string(),
        message: "Bad indent".to_string(),
        points: 1,
    });

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail with min_total_score=500"),
        ThresholdResult::Fail(failures) => {
            assert!(failures.iter().any(|f| f.contains("Total score")));
        }
    }
}

// ============================================================================
// Unit Tests for Threshold Checking - Pass/Fail Scenarios
// ============================================================================

#[test]
fn test_pass_scenario_no_thresholds_set() {
    let checker = ThresholdChecker::new();
    let mut score = ProjectScore::new(10);
    score.total_score = 0; // Even zero score should pass with no thresholds

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Should pass when no thresholds are set"),
    }
}

#[test]
fn test_pass_scenario_exact_threshold_match() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 400; // Exact match should pass

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Should pass when score equals threshold"),
    }
}

#[test]
fn test_pass_scenario_above_threshold() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 450;

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Should pass when score is above threshold"),
    }
}

#[test]
fn test_pass_scenario_all_categories_meet_threshold() {
    let checker = ThresholdChecker::new()
        .with_category_threshold(Category::Formatting, 90)
        .with_category_threshold(Category::Linting, 85)
        .with_category_threshold(Category::Security, 95)
        .with_category_threshold(Category::DesignPatterns, 80)
        .with_category_threshold(Category::StructureAndDocs, 85);

    let score = ProjectScore::new(10); // Perfect score

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Should pass when all categories meet thresholds"),
    }
}

#[test]
fn test_fail_scenario_total_below_threshold() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 399; // Just below threshold

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail when total score is below threshold"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].contains("Total score"));
            assert!(failures[0].contains("399"));
            assert!(failures[0].contains("400"));
        }
    }
}

#[test]
fn test_fail_scenario_single_category_below_threshold() {
    let checker = ThresholdChecker::new().with_category_threshold(Category::Security, 95);

    let mut score = ProjectScore::new(10);

    // Add violation to bring security score to 94
    score.add_violation(Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "unsafe-code".to_string(),
        message: "Unsafe code detected".to_string(),
        points: 6,
    });

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail when security score is below threshold"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].contains("security"));
            assert!(failures[0].contains("94"));
            assert!(failures[0].contains("95"));
        }
    }
}

#[test]
fn test_fail_scenario_multiple_categories_below_threshold() {
    let checker = ThresholdChecker::new()
        .with_category_threshold(Category::Formatting, 90)
        .with_category_threshold(Category::Linting, 90)
        .with_category_threshold(Category::Security, 90);

    let mut score = ProjectScore::new(10);

    // Bring formatting to 85
    score.add_violation(Violation {
        category: Category::Formatting,
        severity: Severity::Critical,
        file: PathBuf::from("test.js"),
        line: 1,
        column: 1,
        rule_id: "indent".to_string(),
        message: "Bad indent".to_string(),
        points: 15,
    });

    // Bring linting to 80
    score.add_violation(Violation {
        category: Category::Linting,
        severity: Severity::Critical,
        file: PathBuf::from("test.js"),
        line: 2,
        column: 1,
        rule_id: "no-unused-vars".to_string(),
        message: "Unused variable".to_string(),
        points: 20,
    });

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail when multiple categories are below threshold"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 2);
            assert!(failures.iter().any(|f| f.contains("formatting")));
            assert!(failures.iter().any(|f| f.contains("linting")));
        }
    }
}

#[test]
fn test_fail_scenario_total_and_category_both_fail() {
    let checker = ThresholdChecker::new()
        .with_total_threshold(450)
        .with_category_threshold(Category::Security, 95);

    let mut score = ProjectScore::new(10);

    // Add violations to bring total to 440 and security to 90
    score.add_violation(Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "unsafe".to_string(),
        message: "Unsafe".to_string(),
        points: 10,
    });

    score.add_violation(Violation {
        category: Category::Formatting,
        severity: Severity::Critical,
        file: PathBuf::from("test.js"),
        line: 1,
        column: 1,
        rule_id: "indent".to_string(),
        message: "Bad indent".to_string(),
        points: 50,
    });

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail both total and category thresholds"),
        ThresholdResult::Fail(failures) => {
            assert!(failures.len() >= 2);
            assert!(failures.iter().any(|f| f.contains("Total score")));
            assert!(failures.iter().any(|f| f.contains("security")));
        }
    }
}

#[test]
fn test_fail_scenario_all_categories_below_threshold() {
    let checker = ThresholdChecker::new()
        .with_category_threshold(Category::Formatting, 95)
        .with_category_threshold(Category::Linting, 95)
        .with_category_threshold(Category::Security, 95)
        .with_category_threshold(Category::DesignPatterns, 95)
        .with_category_threshold(Category::StructureAndDocs, 95);

    let mut score = ProjectScore::new(10);

    // Add violations to each category
    for category in [
        Category::Formatting,
        Category::Linting,
        Category::Security,
        Category::DesignPatterns,
        Category::StructureAndDocs,
    ] {
        score.add_violation(Violation {
            category,
            severity: Severity::Critical,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 1,
            rule_id: "test".to_string(),
            message: "Test violation".to_string(),
            points: 10,
        });
    }

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail all category thresholds"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 5);
        }
    }
}

// ============================================================================
// Unit Tests for Exit Code Generation
// ============================================================================

#[test]
fn test_exit_code_zero_on_pass() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 450;

    assert_eq!(checker.exit_code(&score), 0);
}

#[test]
fn test_exit_code_one_on_fail() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 350;

    assert_eq!(checker.exit_code(&score), 1);
}

#[test]
fn test_exit_code_zero_with_no_thresholds() {
    let checker = ThresholdChecker::new();

    let mut score = ProjectScore::new(10);
    score.total_score = 0; // Even zero score

    assert_eq!(checker.exit_code(&score), 0);
}

#[test]
fn test_exit_code_one_on_category_failure() {
    let checker = ThresholdChecker::new().with_category_threshold(Category::Security, 95);

    let mut score = ProjectScore::new(10);
    score.add_violation(Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "unsafe".to_string(),
        message: "Unsafe".to_string(),
        points: 10,
    });

    assert_eq!(checker.exit_code(&score), 1);
}

#[test]
fn test_exit_code_one_on_multiple_failures() {
    let checker = ThresholdChecker::new()
        .with_total_threshold(450)
        .with_category_threshold(Category::Security, 95)
        .with_category_threshold(Category::Linting, 90);

    let mut score = ProjectScore::new(10);
    score.total_score = 400;

    // Add violations to multiple categories
    score.add_violation(Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "unsafe".to_string(),
        message: "Unsafe".to_string(),
        points: 10,
    });

    score.add_violation(Violation {
        category: Category::Linting,
        severity: Severity::Critical,
        file: PathBuf::from("test.js"),
        line: 1,
        column: 1,
        rule_id: "unused".to_string(),
        message: "Unused".to_string(),
        points: 15,
    });

    assert_eq!(checker.exit_code(&score), 1);
}

#[test]
fn test_exit_code_zero_at_exact_threshold() {
    let checker = ThresholdChecker::new()
        .with_total_threshold(400)
        .with_category_threshold(Category::Formatting, 90);

    let mut score = ProjectScore::new(10);
    score.total_score = 400;

    // Bring formatting to exactly 90
    score.add_violation(Violation {
        category: Category::Formatting,
        severity: Severity::Critical,
        file: PathBuf::from("test.js"),
        line: 1,
        column: 1,
        rule_id: "indent".to_string(),
        message: "Bad indent".to_string(),
        points: 10,
    });

    assert_eq!(checker.exit_code(&score), 0);
}

#[test]
fn test_exit_code_consistency_with_check_result() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    // Test passing score
    let mut score = ProjectScore::new(10);
    score.total_score = 450;

    let check_result = checker.check(&score);
    let exit_code = checker.exit_code(&score);

    match check_result {
        ThresholdResult::Pass => assert_eq!(exit_code, 0),
        ThresholdResult::Fail(_) => assert_eq!(exit_code, 1),
    }

    // Test failing score
    score.total_score = 350;

    let check_result = checker.check(&score);
    let exit_code = checker.exit_code(&score);

    match check_result {
        ThresholdResult::Pass => assert_eq!(exit_code, 0),
        ThresholdResult::Fail(_) => assert_eq!(exit_code, 1),
    }
}
