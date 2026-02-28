//! Comprehensive tests for the 500-point scoring system

use dx_check::scoring::*;
use std::path::PathBuf;

#[test]
fn test_category_enum() {
    let categories = Category::all();
    assert_eq!(categories.len(), 5);
    assert!(categories.contains(&Category::Formatting));
    assert!(categories.contains(&Category::Linting));
    assert!(categories.contains(&Category::Security));
    assert!(categories.contains(&Category::DesignPatterns));
    assert!(categories.contains(&Category::StructureAndDocs));
}

#[test]
fn test_category_as_str() {
    assert_eq!(Category::Formatting.as_str(), "formatting");
    assert_eq!(Category::Linting.as_str(), "linting");
    assert_eq!(Category::Security.as_str(), "security");
    assert_eq!(Category::DesignPatterns.as_str(), "design_patterns");
    assert_eq!(Category::StructureAndDocs.as_str(), "structure_and_docs");
}

#[test]
fn test_severity_points() {
    assert_eq!(Severity::Critical.points(), 10);
    assert_eq!(Severity::High.points(), 5);
    assert_eq!(Severity::Medium.points(), 2);
    assert_eq!(Severity::Low.points(), 1);
}

#[test]
fn test_violation_creation() {
    let violation = Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("src/main.rs"),
        line: 42,
        column: 10,
        rule_id: "no-unsafe".to_string(),
        message: "Unsafe block without SAFETY comment".to_string(),
        points: Severity::Critical.points(),
    };

    assert_eq!(violation.category, Category::Security);
    assert_eq!(violation.severity, Severity::Critical);
    assert_eq!(violation.points, 10);
    assert_eq!(violation.line, 42);
}

#[test]
fn test_deduction_rule() {
    let rule = DeductionRule {
        rule_id: "no-eval".to_string(),
        category: Category::Security,
        default_severity: Severity::Critical,
        description: "Avoid using eval()".to_string(),
    };

    assert_eq!(rule.rule_id, "no-eval");
    assert_eq!(rule.category, Category::Security);
    assert_eq!(rule.default_severity, Severity::Critical);
}

#[test]
fn test_project_score_initialization() {
    let score = ProjectScore::new(100);

    assert_eq!(score.total_score, MAX_TOTAL_SCORE);
    assert_eq!(score.files_analyzed, 100);
    assert_eq!(score.categories.len(), 5);

    for category in Category::all() {
        assert_eq!(score.get_category_score(*category), MAX_CATEGORY_SCORE);
    }
}

#[test]
fn test_category_score_add_violation() {
    let mut cat_score = CategoryScore::new(Category::Formatting);

    let violation = Violation {
        category: Category::Formatting,
        severity: Severity::High,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "indent".to_string(),
        message: "Incorrect indentation".to_string(),
        points: 5,
    };

    cat_score.add_violation(violation);

    assert_eq!(cat_score.score, 95);
    assert_eq!(cat_score.deductions, 5);
    assert_eq!(cat_score.violation_count(), 1);
}

#[test]
fn test_project_score_add_violation() {
    let mut score = ProjectScore::new(10);

    let violation = Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("auth.rs"),
        line: 50,
        column: 5,
        rule_id: "no-unsafe".to_string(),
        message: "Unsafe code detected".to_string(),
        points: 10,
    };

    score.add_violation(violation);

    assert_eq!(score.get_category_score(Category::Security), 90);
    assert_eq!(score.total_score, 490);
    assert_eq!(score.total_violations(), 1);
}

#[test]
fn test_multiple_violations_same_category() {
    let mut score = ProjectScore::new(5);

    for i in 0..3 {
        let violation = Violation {
            category: Category::Linting,
            severity: Severity::Medium,
            file: PathBuf::from(format!("file{}.rs", i)),
            line: i as u32,
            column: 1,
            rule_id: "no-unused-vars".to_string(),
            message: "Unused variable".to_string(),
            points: 2,
        };
        score.add_violation(violation);
    }

    assert_eq!(score.get_category_score(Category::Linting), 94);
    assert_eq!(score.total_score, 494);
    assert_eq!(score.total_violations(), 3);
}

#[test]
fn test_violations_across_categories() {
    let mut score = ProjectScore::new(10);

    let violations = vec![
        (Category::Formatting, Severity::Low, 1),
        (Category::Linting, Severity::Medium, 2),
        (Category::Security, Severity::High, 5),
        (Category::DesignPatterns, Severity::Critical, 10),
        (Category::StructureAndDocs, Severity::Low, 1),
    ];

    for (category, severity, points) in violations {
        let violation = Violation {
            category,
            severity,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 1,
            rule_id: "test-rule".to_string(),
            message: "Test violation".to_string(),
            points,
        };
        score.add_violation(violation);
    }

    assert_eq!(score.get_category_score(Category::Formatting), 99);
    assert_eq!(score.get_category_score(Category::Linting), 98);
    assert_eq!(score.get_category_score(Category::Security), 95);
    assert_eq!(score.get_category_score(Category::DesignPatterns), 90);
    assert_eq!(score.get_category_score(Category::StructureAndDocs), 99);
    assert_eq!(score.total_score, 481);
}

#[test]
fn test_score_cannot_go_negative() {
    let mut cat_score = CategoryScore::new(Category::Formatting);

    // Add violations totaling more than 100 points
    for _ in 0..15 {
        let violation = Violation {
            category: Category::Formatting,
            severity: Severity::Critical,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 1,
            rule_id: "test".to_string(),
            message: "Test".to_string(),
            points: 10,
        };
        cat_score.add_violation(violation);
    }

    assert_eq!(cat_score.score, 0);
    assert!(cat_score.deductions >= 100);
}

#[test]
fn test_grade_calculation() {
    let test_cases = vec![
        (500, "A+"),
        (475, "A+"),
        (450, "A+"),
        (449, "A"),
        (425, "A"),
        (400, "A"),
        (399, "B+"),
        (375, "B+"),
        (350, "B+"),
        (349, "B"),
        (325, "B"),
        (300, "B"),
        (299, "C+"),
        (275, "C+"),
        (250, "C+"),
        (249, "C"),
        (225, "C"),
        (200, "C"),
        (199, "D"),
        (175, "D"),
        (150, "D"),
        (149, "F"),
        (100, "F"),
        (0, "F"),
    ];

    for (score_value, expected_grade) in test_cases {
        let mut score = ProjectScore::new(10);
        score.total_score = score_value;
        assert_eq!(
            score.grade(),
            expected_grade,
            "Score {} should be grade {}",
            score_value,
            expected_grade
        );
    }
}

#[test]
fn test_score_calculator_default_rules() {
    let calculator = ScoreCalculator::new();

    // Test that default rules are registered
    let diagnostics = vec![];
    let score = calculator.calculate(&diagnostics, 10);

    assert_eq!(score.total_score, MAX_TOTAL_SCORE);
    assert_eq!(score.files_analyzed, 10);
}

#[test]
fn test_score_calculator_register_rule() {
    let mut calculator = ScoreCalculator::new();

    calculator.register_rule("custom-rule".to_string(), Category::Security);

    // The rule should now be registered (we can't directly test the internal map,
    // but we can verify it doesn't panic)
    let diagnostics = vec![];
    let score = calculator.calculate(&diagnostics, 5);
    assert_eq!(score.total_score, MAX_TOTAL_SCORE);
}

#[test]
fn test_threshold_checker_pass() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 450;

    match checker.check(&score) {
        ThresholdResult::Pass => {}
        ThresholdResult::Fail(_) => panic!("Should pass threshold check"),
    }

    assert_eq!(checker.exit_code(&score), 0);
}

#[test]
fn test_threshold_checker_fail_total() {
    let checker = ThresholdChecker::new().with_total_threshold(400);

    let mut score = ProjectScore::new(10);
    score.total_score = 350;

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail threshold check"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].contains("Total score"));
        }
    }

    assert_eq!(checker.exit_code(&score), 1);
}

#[test]
fn test_threshold_checker_fail_category() {
    let checker = ThresholdChecker::new().with_category_threshold(Category::Security, 95);

    let mut score = ProjectScore::new(10);

    let violation = Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "test".to_string(),
        message: "Test".to_string(),
        points: 10,
    };
    score.add_violation(violation);

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail category threshold check"),
        ThresholdResult::Fail(failures) => {
            assert_eq!(failures.len(), 1);
            assert!(failures[0].contains("security"));
        }
    }
}

#[test]
fn test_threshold_checker_multiple_failures() {
    let checker = ThresholdChecker::new()
        .with_total_threshold(450)
        .with_category_threshold(Category::Security, 95)
        .with_category_threshold(Category::Linting, 90);

    let mut score = ProjectScore::new(10);
    score.total_score = 400;

    // Add violations to bring categories below thresholds
    for _ in 0..2 {
        score.add_violation(Violation {
            category: Category::Security,
            severity: Severity::High,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 1,
            rule_id: "test".to_string(),
            message: "Test".to_string(),
            points: 5,
        });

        score.add_violation(Violation {
            category: Category::Linting,
            severity: Severity::Critical,
            file: PathBuf::from("test.rs"),
            line: 1,
            column: 1,
            rule_id: "test".to_string(),
            message: "Test".to_string(),
            points: 10,
        });
    }

    match checker.check(&score) {
        ThresholdResult::Pass => panic!("Should fail multiple thresholds"),
        ThresholdResult::Fail(failures) => {
            assert!(failures.len() >= 2);
        }
    }
}

#[test]
fn test_constants() {
    assert_eq!(MAX_TOTAL_SCORE, 500);
    assert_eq!(MAX_CATEGORY_SCORE, 100);
    assert_eq!(MAX_TOTAL_SCORE, MAX_CATEGORY_SCORE * 5);
}

#[test]
fn test_serialization() {
    let violation = Violation {
        category: Category::Security,
        severity: Severity::High,
        file: PathBuf::from("test.rs"),
        line: 10,
        column: 5,
        rule_id: "no-eval".to_string(),
        message: "Avoid eval".to_string(),
        points: 5,
    };

    let json = serde_json::to_string(&violation).unwrap();
    let deserialized: Violation = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.category, violation.category);
    assert_eq!(deserialized.severity, violation.severity);
    assert_eq!(deserialized.points, violation.points);
}

#[test]
fn test_deduction_rule_serialization() {
    let rule = DeductionRule {
        rule_id: "no-unsafe".to_string(),
        category: Category::Security,
        default_severity: Severity::Critical,
        description: "Avoid unsafe blocks".to_string(),
    };

    let json = serde_json::to_string(&rule).unwrap();
    let deserialized: DeductionRule = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.rule_id, rule.rule_id);
    assert_eq!(deserialized.category, rule.category);
    assert_eq!(deserialized.default_severity, rule.default_severity);
    assert_eq!(deserialized.description, rule.description);
}

#[test]
fn test_analysis_mode_default() {
    use dx_check::scoring::AnalysisMode;
    assert_eq!(AnalysisMode::default(), AnalysisMode::Quick);
}

#[test]
fn test_score_calculator_quick_mode() {
    use dx_check::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
    use dx_check::scoring::AnalysisMode;

    let calculator = ScoreCalculator::with_mode(AnalysisMode::Quick);
    assert_eq!(calculator.mode(), AnalysisMode::Quick);

    let diagnostics = vec![Diagnostic {
        severity: DiagnosticSeverity::Error,
        message: "Unused variable".to_string(),
        file: PathBuf::from("src/main.rs"),
        span: Span { start: 10, end: 15 },
        rule_id: "no-unused-vars".to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    }];

    let score = calculator.calculate(&diagnostics, 1);
    assert_eq!(score.files_analyzed, 1);
    assert!(score.total_score < MAX_TOTAL_SCORE);
}

#[test]
fn test_score_calculator_detailed_mode() {
    use dx_check::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
    use dx_check::scoring::AnalysisMode;

    let calculator = ScoreCalculator::with_mode(AnalysisMode::Detailed);
    assert_eq!(calculator.mode(), AnalysisMode::Detailed);

    let diagnostics = vec![
        Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: "Unused variable".to_string(),
            file: PathBuf::from("src/main.rs"),
            span: Span { start: 10, end: 15 },
            rule_id: "no-unused-vars".to_string(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        },
        Diagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "Missing docs".to_string(),
            file: PathBuf::from("src/lib.rs"),
            span: Span { start: 1, end: 5 },
            rule_id: "missing-docs".to_string(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        },
    ];

    let (project_score, file_scores) = calculator.calculate_detailed(&diagnostics, 2);

    assert_eq!(project_score.files_analyzed, 2);
    assert_eq!(file_scores.len(), 2);
    assert!(file_scores.contains_key(&PathBuf::from("src/main.rs")));
    assert!(file_scores.contains_key(&PathBuf::from("src/lib.rs")));
}

#[test]
fn test_file_score_initialization() {
    use dx_check::scoring::FileScore;

    let file_score = FileScore::new(PathBuf::from("test.rs"));

    assert_eq!(file_score.file, PathBuf::from("test.rs"));
    assert_eq!(file_score.total_score, MAX_TOTAL_SCORE);
    assert_eq!(file_score.categories.len(), 5);

    for category in Category::all() {
        assert_eq!(file_score.get_category_score(*category), MAX_CATEGORY_SCORE);
    }
}

#[test]
fn test_file_score_add_violation() {
    use dx_check::scoring::FileScore;

    let mut file_score = FileScore::new(PathBuf::from("test.rs"));

    let violation = Violation {
        category: Category::Security,
        severity: Severity::Critical,
        file: PathBuf::from("test.rs"),
        line: 10,
        column: 5,
        rule_id: "no-eval".to_string(),
        message: "Avoid eval".to_string(),
        points: 10,
    };

    file_score.add_violation(violation);

    assert_eq!(file_score.get_category_score(Category::Security), 90);
    assert_eq!(file_score.total_score, 490);
}

#[test]
fn test_file_score_multiple_violations() {
    use dx_check::scoring::FileScore;

    let mut file_score = FileScore::new(PathBuf::from("test.rs"));

    // Add violations across different categories
    file_score.add_violation(Violation {
        category: Category::Formatting,
        severity: Severity::Low,
        file: PathBuf::from("test.rs"),
        line: 1,
        column: 1,
        rule_id: "indent".to_string(),
        message: "Bad indent".to_string(),
        points: 1,
    });

    file_score.add_violation(Violation {
        category: Category::Linting,
        severity: Severity::Medium,
        file: PathBuf::from("test.rs"),
        line: 5,
        column: 1,
        rule_id: "no-unused-vars".to_string(),
        message: "Unused var".to_string(),
        points: 2,
    });

    assert_eq!(file_score.get_category_score(Category::Formatting), 99);
    assert_eq!(file_score.get_category_score(Category::Linting), 98);
    assert_eq!(file_score.total_score, 497);
}

#[test]
fn test_detailed_mode_aggregation() {
    use dx_check::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
    use dx_check::scoring::AnalysisMode;

    let calculator = ScoreCalculator::with_mode(AnalysisMode::Detailed);

    let diagnostics = vec![
        Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: "Error 1".to_string(),
            file: PathBuf::from("file1.rs"),
            span: Span { start: 1, end: 5 },
            rule_id: "no-unused-vars".to_string(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        },
        Diagnostic {
            severity: DiagnosticSeverity::Error,
            message: "Error 2".to_string(),
            file: PathBuf::from("file1.rs"),
            span: Span { start: 10, end: 15 },
            rule_id: "no-debugger".to_string(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        },
        Diagnostic {
            severity: DiagnosticSeverity::Warning,
            message: "Warning 1".to_string(),
            file: PathBuf::from("file2.rs"),
            span: Span { start: 5, end: 10 },
            rule_id: "indent".to_string(),
            suggestion: None,
            related: Vec::new(),
            fix: None,
        },
    ];

    let (project_score, file_scores) = calculator.calculate_detailed(&diagnostics, 2);

    // Check file scores
    assert_eq!(file_scores.len(), 2);

    let file1_score = file_scores.get(&PathBuf::from("file1.rs")).unwrap();
    assert!(file1_score.total_score < MAX_TOTAL_SCORE);

    let file2_score = file_scores.get(&PathBuf::from("file2.rs")).unwrap();
    assert!(file2_score.total_score < MAX_TOTAL_SCORE);

    // Check project score aggregation
    assert_eq!(project_score.total_violations(), 3);
    assert!(project_score.total_score < MAX_TOTAL_SCORE);
}

#[test]
fn test_mode_switching() {
    use dx_check::scoring::AnalysisMode;

    let mut calculator = ScoreCalculator::new();
    assert_eq!(calculator.mode(), AnalysisMode::Quick);

    calculator.set_mode(AnalysisMode::Detailed);
    assert_eq!(calculator.mode(), AnalysisMode::Detailed);

    calculator.set_mode(AnalysisMode::Quick);
    assert_eq!(calculator.mode(), AnalysisMode::Quick);
}

#[test]
fn test_quick_vs_detailed_consistency() {
    use dx_check::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
    use dx_check::scoring::AnalysisMode;

    let diagnostics = vec![Diagnostic {
        severity: DiagnosticSeverity::Error,
        message: "Error".to_string(),
        file: PathBuf::from("test.rs"),
        span: Span { start: 1, end: 5 },
        rule_id: "no-unused-vars".to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    }];

    let quick_calc = ScoreCalculator::with_mode(AnalysisMode::Quick);
    let quick_score = quick_calc.calculate(&diagnostics, 1);

    let detailed_calc = ScoreCalculator::with_mode(AnalysisMode::Detailed);
    let (detailed_score, _) = detailed_calc.calculate_detailed(&diagnostics, 1);

    // Both modes should produce the same project-level score
    assert_eq!(quick_score.total_score, detailed_score.total_score);
    assert_eq!(quick_score.total_violations(), detailed_score.total_violations());
}

#[test]
fn test_file_score_serialization() {
    use dx_check::scoring::FileScore;

    let mut file_score = FileScore::new(PathBuf::from("test.rs"));
    file_score.add_violation(Violation {
        category: Category::Security,
        severity: Severity::High,
        file: PathBuf::from("test.rs"),
        line: 10,
        column: 5,
        rule_id: "no-eval".to_string(),
        message: "Avoid eval".to_string(),
        points: 5,
    });

    let json = serde_json::to_string(&file_score).unwrap();
    let deserialized: FileScore = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.file, file_score.file);
    assert_eq!(deserialized.total_score, file_score.total_score);
    assert_eq!(deserialized.categories.len(), file_score.categories.len());
}
