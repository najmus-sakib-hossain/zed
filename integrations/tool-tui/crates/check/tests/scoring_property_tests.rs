//! Property-based tests for the scoring system

use dx_check::scoring::*;
use proptest::prelude::*;
use std::path::PathBuf;

// Strategy for generating Category
fn category_strategy() -> impl Strategy<Value = Category> {
    prop_oneof![
        Just(Category::Formatting),
        Just(Category::Linting),
        Just(Category::Security),
        Just(Category::DesignPatterns),
        Just(Category::StructureAndDocs),
    ]
}

// Strategy for generating Severity
fn severity_strategy() -> impl Strategy<Value = Severity> {
    prop_oneof![
        Just(Severity::Critical),
        Just(Severity::High),
        Just(Severity::Medium),
        Just(Severity::Low),
    ]
}

// Strategy for generating Violation
fn violation_strategy() -> impl Strategy<Value = Violation> {
    (
        category_strategy(),
        severity_strategy(),
        "[a-z]+\\.rs",
        1u32..1000,
        1u32..100,
        "[a-z-]+",
        ".+",
    )
        .prop_map(|(category, severity, file, line, column, rule_id, message)| Violation {
            category,
            severity,
            file: PathBuf::from(file),
            line,
            column,
            rule_id,
            message,
            points: severity.points(),
        })
}

proptest! {
    #[test]
    fn prop_severity_points_are_positive(severity in severity_strategy()) {
        assert!(severity.points() > 0);
        assert!(severity.points() <= 10);
    }

    #[test]
    fn prop_severity_points_ordered(
        s1 in severity_strategy(),
        s2 in severity_strategy()
    ) {
        // Critical should always have highest points
        if matches!(s1, Severity::Critical) {
            assert!(s1.points() >= s2.points());
        }
        // Low should always have lowest points
        if matches!(s2, Severity::Low) {
            assert!(s1.points() >= s2.points());
        }
    }

    #[test]
    fn prop_category_score_never_negative(violations in prop::collection::vec(violation_strategy(), 0..100)) {
        let mut cat_score = CategoryScore::new(Category::Formatting);

        for mut violation in violations {
            violation.category = Category::Formatting;
            cat_score.add_violation(violation);
        }

        assert!(cat_score.score <= MAX_CATEGORY_SCORE);
        assert_eq!(cat_score.score, MAX_CATEGORY_SCORE.saturating_sub(cat_score.deductions));
    }

    #[test]
    fn prop_project_score_never_exceeds_max(violations in prop::collection::vec(violation_strategy(), 0..50)) {
        let mut score = ProjectScore::new(10);

        for violation in violations {
            score.add_violation(violation);
        }

        assert!(score.total_score <= MAX_TOTAL_SCORE);
        for category in Category::all() {
            assert!(score.get_category_score(*category) <= MAX_CATEGORY_SCORE);
        }
    }

    #[test]
    fn prop_total_score_equals_sum_of_categories(violations in prop::collection::vec(violation_strategy(), 0..30)) {
        let mut score = ProjectScore::new(5);

        for violation in violations {
            score.add_violation(violation);
        }

        let sum: u16 = Category::all()
            .iter()
            .map(|c| score.get_category_score(*c))
            .sum();

        assert_eq!(score.total_score, sum);
    }

    #[test]
    fn prop_violation_count_matches_added(violations in prop::collection::vec(violation_strategy(), 0..50)) {
        let mut score = ProjectScore::new(10);
        let count = violations.len();

        for violation in violations {
            score.add_violation(violation);
        }

        assert_eq!(score.total_violations(), count);
    }

    #[test]
    fn prop_grade_monotonic(score1 in 0u16..=500, score2 in 0u16..=500) {
        let mut s1 = ProjectScore::new(10);
        let mut s2 = ProjectScore::new(10);
        s1.total_score = score1;
        s2.total_score = score2;

        let grade1 = s1.grade();
        let grade2 = s2.grade();

        // Higher scores should have better or equal grades
        if score1 > score2 {
            let grade_order = ["F", "D", "C", "C+", "B", "B+", "A", "A+"];
            let idx1 = grade_order.iter().position(|&g| g == grade1).unwrap();
            let idx2 = grade_order.iter().position(|&g| g == grade2).unwrap();
            assert!(idx1 >= idx2, "Score {} (grade {}) should be >= score {} (grade {})", score1, grade1, score2, grade2);
        }
    }

    #[test]
    fn prop_threshold_checker_consistent(
        threshold in 0u16..=500,
        score_value in 0u16..=500
    ) {
        let checker = ThresholdChecker::new().with_total_threshold(threshold);
        let mut score = ProjectScore::new(10);
        score.total_score = score_value;

        let result = checker.check(&score);
        let exit_code = checker.exit_code(&score);

        match result {
            ThresholdResult::Pass => {
                assert_eq!(exit_code, 0);
                assert!(score_value >= threshold);
            }
            ThresholdResult::Fail(_) => {
                assert_eq!(exit_code, 1);
                assert!(score_value < threshold);
            }
        }
    }

    #[test]
    fn prop_deduction_rule_roundtrip(
        rule_id in "[a-z-]+",
        category in category_strategy(),
        severity in severity_strategy(),
        description in ".+"
    ) {
        let rule = DeductionRule {
            rule_id: rule_id.clone(),
            category,
            default_severity: severity,
            description: description.clone(),
        };

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: DeductionRule = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.rule_id, rule_id);
        assert_eq!(deserialized.category, category);
        assert_eq!(deserialized.default_severity, severity);
        assert_eq!(deserialized.description, description);
    }

    #[test]
    fn prop_violation_points_match_severity(violation in violation_strategy()) {
        assert_eq!(violation.points, violation.severity.points());
    }

    #[test]
    fn prop_category_all_contains_all_variants(category in category_strategy()) {
        assert!(Category::all().contains(&category));
    }

    #[test]
    fn prop_adding_violations_decreases_or_maintains_score(
        initial_violations in prop::collection::vec(violation_strategy(), 0..10),
        new_violation in violation_strategy()
    ) {
        let mut score1 = ProjectScore::new(10);
        let mut score2 = ProjectScore::new(10);

        for v in initial_violations.clone() {
            score1.add_violation(v.clone());
            score2.add_violation(v);
        }

        let before = score2.total_score;
        score2.add_violation(new_violation);
        let after = score2.total_score;

        assert!(after <= before, "Score should not increase when adding violations");
    }

    /// **Property 2: Score bounds**
    /// **Validates: Requirements 2.1, 2.2**
    ///
    /// This property test verifies that for any set of violations:
    /// - Total score is always between 0 and 500
    /// - Each category score is always between 0 and 100
    /// - Deductions never produce negative scores
    #[test]
    fn prop_score_bounds_always_valid(
        violations in prop::collection::vec(violation_strategy(), 0..200)
    ) {
        let mut score = ProjectScore::new(10);

        // Add all violations
        for violation in violations {
            score.add_violation(violation);
        }

        // Property 1: Total score must be between 0 and 500
        assert!(
            score.total_score <= MAX_TOTAL_SCORE,
            "Total score {} exceeds maximum {}",
            score.total_score,
            MAX_TOTAL_SCORE
        );
        assert!(
            score.total_score >= 0,
            "Total score {} is negative (impossible with u16, but checking bounds)",
            score.total_score
        );

        // Property 2: Each category score must be between 0 and 100
        for category in Category::all() {
            let cat_score = score.get_category_score(*category);
            assert!(
                cat_score <= MAX_CATEGORY_SCORE,
                "Category {:?} score {} exceeds maximum {}",
                category,
                cat_score,
                MAX_CATEGORY_SCORE
            );
            assert!(
                cat_score >= 0,
                "Category {:?} score {} is negative (impossible with u16, but checking bounds)",
                category,
                cat_score
            );
        }

        // Property 3: Deductions never produce negative scores
        // This is verified by checking that score = max - deductions, with saturation
        for category in Category::all() {
            if let Some(cat_score_obj) = score.categories.get(category) {
                let expected_score = MAX_CATEGORY_SCORE.saturating_sub(cat_score_obj.deductions);
                assert_eq!(
                    cat_score_obj.score,
                    expected_score,
                    "Category {:?} score {} does not match expected saturating subtraction {}",
                    category,
                    cat_score_obj.score,
                    expected_score
                );

                // Verify deductions are tracked correctly
                let actual_deductions: u16 = cat_score_obj.violations
                    .iter()
                    .map(|v| v.points)
                    .sum();
                assert_eq!(
                    cat_score_obj.deductions,
                    actual_deductions,
                    "Category {:?} deductions {} do not match sum of violation points {}",
                    category,
                    cat_score_obj.deductions,
                    actual_deductions
                );
            }
        }

        // Property 4: Total score equals sum of all category scores
        let sum_of_categories: u16 = Category::all()
            .iter()
            .map(|c| score.get_category_score(*c))
            .sum();
        assert_eq!(
            score.total_score,
            sum_of_categories,
            "Total score {} does not equal sum of category scores {}",
            score.total_score,
            sum_of_categories
        );
    }

    /// **Property 2: Score bounds (extreme case)**
    /// **Validates: Requirements 2.1, 2.2**
    ///
    /// Test with extreme number of violations to ensure bounds hold even under stress
    #[test]
    fn prop_score_bounds_extreme_violations(
        violations in prop::collection::vec(violation_strategy(), 100..500)
    ) {
        let mut score = ProjectScore::new(100);

        for violation in violations {
            score.add_violation(violation);
        }

        // Even with many violations, bounds must hold
        assert!(score.total_score <= MAX_TOTAL_SCORE);
        assert!(score.total_score >= 0);

        for category in Category::all() {
            let cat_score = score.get_category_score(*category);
            assert!(cat_score <= MAX_CATEGORY_SCORE);
            assert!(cat_score >= 0);
        }
    }

    /// **Property 2: Score bounds (all critical violations)**
    /// **Validates: Requirements 2.1, 2.2**
    ///
    /// Test with all critical violations (maximum deduction per violation)
    #[test]
    fn prop_score_bounds_all_critical(
        num_violations in 0usize..100,
        category in category_strategy()
    ) {
        let mut score = ProjectScore::new(10);

        // Add critical violations to a single category
        for i in 0..num_violations {
            let violation = Violation {
                category,
                severity: Severity::Critical,
                file: PathBuf::from(format!("test{}.rs", i)),
                line: i as u32,
                column: 1,
                rule_id: "test-rule".to_string(),
                message: "Test violation".to_string(),
                points: Severity::Critical.points(),
            };
            score.add_violation(violation);
        }

        // Category score should floor at 0, not go negative
        let cat_score = score.get_category_score(category);
        assert!(cat_score <= MAX_CATEGORY_SCORE);
        assert!(cat_score >= 0);

        // If we have enough critical violations (10 points each), score should be 0
        if num_violations >= 10 {
            assert_eq!(cat_score, 0, "Category score should be 0 with {} critical violations", num_violations);
        }

        // Total score should still be valid
        assert!(score.total_score <= MAX_TOTAL_SCORE);
        assert!(score.total_score >= 0);
    }
}
