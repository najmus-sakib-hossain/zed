//! Property-based tests for dx-www-a11y
//!
//! These tests verify the correctness of accessibility rule detection
//! and error report completeness using property-based testing.

use dx_www_a11y::{A11yReport, A11ySeverity, ASTAnalyzer};
use proptest::prelude::*;

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate a valid img tag with optional alt attribute
fn gen_img_tag(has_alt: bool, alt_text: &str) -> String {
    if has_alt {
        format!("<img src=\"test.jpg\" alt=\"{}\">", alt_text)
    } else {
        "<img src=\"test.jpg\">".to_string()
    }
}

/// Generate a valid button tag with optional aria-label
fn gen_button_tag(has_label: bool, label: &str, self_closing: bool) -> String {
    if self_closing {
        if has_label {
            format!("<button aria-label=\"{}\"/>", label)
        } else {
            "<button/>".to_string()
        }
    } else if has_label {
        format!("<button aria-label=\"{}\">{}</button>", label, label)
    } else {
        "<button>Click me</button>".to_string()
    }
}

/// Generate a valid input tag with optional label association
fn gen_input_tag(has_id: bool, has_aria_label: bool, id: &str) -> String {
    let mut attrs = vec!["type=\"text\"".to_string()];
    if has_id {
        attrs.push(format!("id=\"{}\"", id));
    }
    if has_aria_label {
        attrs.push(format!("aria-label=\"{}\"", id));
    }
    format!("<input {}>", attrs.join(" "))
}

/// Generate heading tags with specified levels
fn gen_headings(levels: &[u8]) -> String {
    levels
        .iter()
        .map(|&level| format!("<h{}>Heading {}</h{}>", level, level, level))
        .collect::<Vec<_>>()
        .join("\n")
}

// ============================================================================
// Property Test 12: A11y Rule Detection
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 12: A11y Rule Detection
    ///
    /// For any JSX with known violations:
    /// 1. All violations are detected
    /// 2. Each violation has correct rule ID
    /// 3. Each violation has correct severity
    #[test]
    fn prop_a11y_rule_detection(
        num_imgs_without_alt in 0usize..5,
        num_imgs_with_alt in 0usize..5,
        num_buttons_without_label in 0usize..3,
        num_inputs_without_label in 0usize..3,
    ) {
        let mut source = String::new();

        // Add images without alt (should trigger errors)
        for _ in 0..num_imgs_without_alt {
            source.push_str(&gen_img_tag(false, ""));
            source.push('\n');
        }

        // Add images with alt (should not trigger errors)
        for i in 0..num_imgs_with_alt {
            source.push_str(&gen_img_tag(true, &format!("Image {}", i)));
            source.push('\n');
        }

        // Add self-closing buttons without label (should trigger errors)
        for _ in 0..num_buttons_without_label {
            source.push_str(&gen_button_tag(false, "", true));
            source.push('\n');
        }

        // Add inputs without label (should trigger errors)
        for _ in 0..num_inputs_without_label {
            source.push_str(&gen_input_tag(false, false, ""));
            source.push('\n');
        }

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        // Property 1: All img-alt violations detected
        let img_alt_issues: Vec<_> = analyzer.issues().iter()
            .filter(|i| i.rule == "img-alt")
            .collect();
        prop_assert_eq!(
            img_alt_issues.len(),
            num_imgs_without_alt,
            "Expected {} img-alt issues, found {}",
            num_imgs_without_alt,
            img_alt_issues.len()
        );

        // Property 2: All button-label violations detected
        let button_issues: Vec<_> = analyzer.issues().iter()
            .filter(|i| i.rule == "button-label")
            .collect();
        prop_assert_eq!(
            button_issues.len(),
            num_buttons_without_label,
            "Expected {} button-label issues, found {}",
            num_buttons_without_label,
            button_issues.len()
        );

        // Property 3: All form-label violations detected
        let form_issues: Vec<_> = analyzer.issues().iter()
            .filter(|i| i.rule == "form-label")
            .collect();
        prop_assert_eq!(
            form_issues.len(),
            num_inputs_without_label,
            "Expected {} form-label issues, found {}",
            num_inputs_without_label,
            form_issues.len()
        );

        // Property 4: All img-alt issues are errors
        for issue in &img_alt_issues {
            prop_assert_eq!(issue.severity, A11ySeverity::Error);
        }
    }

    /// Property 12b: Heading level skip detection
    #[test]
    fn prop_heading_level_skip_detection(
        skip_from in 1u8..5,
        skip_to in 2u8..6,
    ) {
        // Only test when there's actually a skip
        prop_assume!(skip_to > skip_from + 1);

        let source = gen_headings(&[skip_from, skip_to]);

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        // Property: Heading skip should be detected
        let heading_issues: Vec<_> = analyzer.issues().iter()
            .filter(|i| i.rule == "heading-order")
            .collect();

        prop_assert!(
            !heading_issues.is_empty(),
            "Should detect heading skip from h{} to h{}",
            skip_from,
            skip_to
        );

        // Property: Issue message should mention the skip
        let issue = &heading_issues[0];
        prop_assert!(
            issue.message.contains(&format!("h{}", skip_to)),
            "Message should mention h{}: {}",
            skip_to,
            issue.message
        );
    }

    /// Property 12c: Valid headings don't trigger warnings
    #[test]
    fn prop_valid_headings_no_warnings(
        num_headings in 1usize..4,
    ) {
        // Generate sequential headings (h1, h2, h3, ...)
        let levels: Vec<u8> = (1..=(num_headings as u8)).collect();
        let source = gen_headings(&levels);

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        // Property: No heading-order warnings for sequential headings
        let heading_issues: Vec<_> = analyzer.issues().iter()
            .filter(|i| i.rule == "heading-order")
            .collect();

        prop_assert!(
            heading_issues.is_empty(),
            "Sequential headings should not trigger warnings: {:?}",
            heading_issues
        );
    }
}

// ============================================================================
// Property Test 13: Error Report Completeness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 13: A11y Error Report Completeness
    ///
    /// For any detected violation:
    /// 1. Report contains rule ID
    /// 2. Report contains severity
    /// 3. Report contains message
    /// 4. Report optionally contains span and suggestion
    #[test]
    fn prop_error_report_completeness(
        num_violations in 1usize..10,
    ) {
        let mut source = String::new();

        // Generate various violations
        for i in 0..num_violations {
            match i % 3 {
                0 => source.push_str(&gen_img_tag(false, "")),
                1 => source.push_str(&gen_button_tag(false, "", true)),
                _ => source.push_str(&gen_input_tag(false, false, "")),
            }
            source.push('\n');
        }

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        // Property 1: All issues have non-empty rule ID
        for issue in analyzer.issues() {
            prop_assert!(!issue.rule.is_empty(), "Issue should have rule ID");
        }

        // Property 2: All issues have valid severity
        for issue in analyzer.issues() {
            prop_assert!(
                matches!(issue.severity, A11ySeverity::Error | A11ySeverity::Warning | A11ySeverity::Info),
                "Issue should have valid severity"
            );
        }

        // Property 3: All issues have non-empty message
        for issue in analyzer.issues() {
            prop_assert!(!issue.message.is_empty(), "Issue should have message");
        }

        // Property 4: Report counts match issue counts
        let report = A11yReport::from_analyzer(&analyzer);
        prop_assert_eq!(
            report.total_issues,
            analyzer.issues().len(),
            "Report total should match issue count"
        );
        prop_assert_eq!(
            report.errors,
            analyzer.error_count(),
            "Report errors should match error count"
        );
        prop_assert_eq!(
            report.warnings,
            analyzer.warning_count(),
            "Report warnings should match warning count"
        );
    }

    /// Property 13b: Issue spans are valid when present
    #[test]
    fn prop_issue_spans_valid(
        num_imgs in 1usize..5,
    ) {
        let mut source = String::new();
        for _ in 0..num_imgs {
            source.push_str(&gen_img_tag(false, ""));
            source.push('\n');
        }

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        // Property: All spans are within source bounds
        for issue in analyzer.issues() {
            if let Some((start, end)) = issue.span {
                prop_assert!(
                    start <= end,
                    "Span start {} should be <= end {}",
                    start,
                    end
                );
                prop_assert!(
                    end <= source.len(),
                    "Span end {} should be <= source length {}",
                    end,
                    source.len()
                );
            }
        }
    }

    /// Property 13c: Suggestions are helpful
    #[test]
    fn prop_suggestions_helpful(
        num_violations in 1usize..5,
    ) {
        let mut source = String::new();
        for _ in 0..num_violations {
            source.push_str(&gen_img_tag(false, ""));
            source.push('\n');
        }

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        // Property: img-alt issues have suggestions
        for issue in analyzer.issues() {
            if issue.rule == "img-alt" {
                prop_assert!(
                    issue.suggestion.is_some(),
                    "img-alt issues should have suggestions"
                );
                let suggestion = issue.suggestion.as_ref().unwrap();
                prop_assert!(
                    suggestion.contains("alt"),
                    "Suggestion should mention 'alt': {}",
                    suggestion
                );
            }
        }
    }
}

// ============================================================================
// Additional Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Empty aria-label is detected
    #[test]
    fn prop_empty_aria_label_detected(
        num_empty in 1usize..5,
    ) {
        let mut source = String::new();
        for _ in 0..num_empty {
            source.push_str("<div aria-label=\"\">Content</div>\n");
        }

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        let empty_label_issues: Vec<_> = analyzer.issues().iter()
            .filter(|i| i.rule == "aria-label-empty")
            .collect();

        prop_assert_eq!(
            empty_label_issues.len(),
            num_empty,
            "Should detect {} empty aria-labels",
            num_empty
        );
    }

    /// Property: Report passed() is correct
    #[test]
    fn prop_report_passed_correct(
        has_errors in proptest::bool::ANY,
    ) {
        let source = if has_errors {
            gen_img_tag(false, "") // Will trigger error
        } else {
            gen_img_tag(true, "Valid alt text") // No error
        };

        let mut analyzer = ASTAnalyzer::new();
        analyzer.analyze(&source);

        let report = A11yReport::from_analyzer(&analyzer);

        if has_errors {
            prop_assert!(!report.passed(), "Report should not pass with errors");
        } else {
            prop_assert!(report.passed(), "Report should pass without errors");
        }
    }

    /// Property: Analyzer can be cleared and reused
    #[test]
    fn prop_analyzer_clear_reuse(
        first_violations in 1usize..5,
        second_violations in 1usize..5,
    ) {
        let mut analyzer = ASTAnalyzer::new();

        // First analysis
        let mut source1 = String::new();
        for _ in 0..first_violations {
            source1.push_str(&gen_img_tag(false, ""));
        }
        analyzer.analyze(&source1);
        let _first_count = analyzer.issues().len();

        // Clear and reuse
        analyzer.clear();
        prop_assert!(analyzer.issues().is_empty(), "Issues should be cleared");

        // Second analysis
        let mut source2 = String::new();
        for _ in 0..second_violations {
            source2.push_str(&gen_img_tag(false, ""));
        }
        analyzer.analyze(&source2);

        // Property: Second analysis is independent
        prop_assert_eq!(
            analyzer.issues().len(),
            second_violations,
            "Second analysis should be independent"
        );
    }
}
