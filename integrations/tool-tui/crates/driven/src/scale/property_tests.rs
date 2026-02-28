//! Property-based tests for Scale-Adaptive Intelligence
//!
//! Tests Property 24 (Scale Detection Consistency) and Property 25 (Scale-Adaptive Workflow Selection)

use super::{ProjectContext, ProjectScale, ScaleDetector};
use proptest::prelude::*;

/// Strategy for generating project scales
fn project_scale_strategy() -> impl Strategy<Value = ProjectScale> {
    prop_oneof![
        Just(ProjectScale::BugFix),
        Just(ProjectScale::Feature),
        Just(ProjectScale::Product),
        Just(ProjectScale::Enterprise),
    ]
}

/// Strategy for generating project contexts
fn project_context_strategy() -> impl Strategy<Value = ProjectContext> {
    (
        proptest::option::of(1usize..500_000), // lines_of_code
        proptest::option::of(1usize..10_000),  // file_count
        proptest::option::of(0usize..500),     // dependency_count
        proptest::option::of(1usize..100),     // contributor_count
        proptest::option::of(1usize..50_000),  // commit_count
        any::<bool>(),                         // has_ci_cd
        any::<bool>(),                         // has_tests
        any::<bool>(),                         // has_docs
    )
        .prop_map(|(loc, files, deps, contributors, commits, ci_cd, tests, docs)| {
            let mut ctx = ProjectContext::new();
            if let Some(l) = loc {
                ctx = ctx.with_lines_of_code(l);
            }
            if let Some(f) = files {
                ctx = ctx.with_file_count(f);
            }
            if let Some(d) = deps {
                ctx = ctx.with_dependency_count(d);
            }
            if let Some(c) = contributors {
                ctx = ctx.with_contributor_count(c);
            }
            if let Some(c) = commits {
                ctx = ctx.with_commit_count(c);
            }
            ctx.with_ci_cd(ci_cd).with_tests(tests).with_docs(docs)
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 24: Scale Detection Consistency
    /// For any project context, scale detection SHALL return a consistent scale
    /// based on project characteristics.
    #[test]
    fn property_24_scale_detection_consistency(context in project_context_strategy()) {
        let detector = ScaleDetector::new();

        // Detection should be deterministic - same input = same output
        let scale1 = detector.detect(&context);
        let scale2 = detector.detect(&context);

        prop_assert_eq!(scale1, scale2, "Scale detection must be deterministic");

        // Scale must be one of the valid variants
        prop_assert!(
            matches!(scale1, ProjectScale::BugFix | ProjectScale::Feature | ProjectScale::Product | ProjectScale::Enterprise),
            "Scale must be a valid variant"
        );
    }

    /// Property 24 (continued): Scale detection respects manual override
    #[test]
    fn property_24_scale_override_respected(
        context in project_context_strategy(),
        override_scale in project_scale_strategy()
    ) {
        let detector = ScaleDetector::new();
        let context_with_override = ProjectContext {
            scale_override: Some(override_scale),
            ..context
        };

        let detected = detector.detect(&context_with_override);

        prop_assert_eq!(
            detected, override_scale,
            "Manual scale override must be respected"
        );
    }

    /// Property 25: Scale-Adaptive Workflow Selection
    /// For any detected scale, the appropriate workflow set SHALL be selected.
    #[test]
    fn property_25_scale_adaptive_workflow_selection(context in project_context_strategy()) {
        let detector = ScaleDetector::new();
        let scale = detector.detect(&context);
        let workflows = detector.select_workflows(&context);

        // Workflows must not be empty
        prop_assert!(!workflows.is_empty(), "Workflows must not be empty");

        // Verify scale-appropriate workflows are selected
        match scale {
            ProjectScale::BugFix => {
                prop_assert!(
                    workflows.iter().any(|w| w.contains("quick") || w.contains("bug")),
                    "BugFix scale should include quick workflows"
                );
            }
            ProjectScale::Feature => {
                prop_assert!(
                    workflows.iter().any(|w| w.contains("feature") || w.contains("tech-spec") || w.contains("dev-story")),
                    "Feature scale should include feature development workflows"
                );
            }
            ProjectScale::Product => {
                prop_assert!(
                    workflows.iter().any(|w| w.contains("prd") || w.contains("architecture") || w.contains("sprint")),
                    "Product scale should include full product workflows"
                );
            }
            ProjectScale::Enterprise => {
                prop_assert!(
                    workflows.len() >= 10,
                    "Enterprise scale should have comprehensive workflow set"
                );
                prop_assert!(
                    workflows.iter().any(|w| w.contains("security") || w.contains("deployment")),
                    "Enterprise scale should include governance workflows"
                );
            }
        }
    }

    /// Property 25 (continued): Scale-adaptive agent selection
    #[test]
    fn property_25_scale_adaptive_agent_selection(context in project_context_strategy()) {
        let detector = ScaleDetector::new();
        let scale = detector.detect(&context);
        let agents = detector.select_agents(&context);

        // Agents must not be empty
        prop_assert!(!agents.is_empty(), "Agents must not be empty");

        // Core agents should always be present
        prop_assert!(
            agents.contains(&"developer"),
            "Developer agent should always be recommended"
        );

        // Verify scale-appropriate agents
        match scale {
            ProjectScale::BugFix => {
                prop_assert!(
                    agents.len() <= 3,
                    "BugFix scale should have minimal agent set"
                );
            }
            ProjectScale::Feature => {
                prop_assert!(
                    agents.contains(&"architect") || agents.contains(&"reviewer"),
                    "Feature scale should include architect or reviewer"
                );
            }
            ProjectScale::Product => {
                prop_assert!(
                    agents.contains(&"pm") && agents.contains(&"architect"),
                    "Product scale should include PM and architect"
                );
            }
            ProjectScale::Enterprise => {
                prop_assert!(
                    agents.contains(&"security") && agents.contains(&"devops"),
                    "Enterprise scale should include security and devops"
                );
            }
        }
    }

    /// Property 25 (continued): Recommendation confidence is valid
    #[test]
    fn property_25_recommendation_confidence_valid(context in project_context_strategy()) {
        let detector = ScaleDetector::new();
        let recommendation = detector.recommend(&context);

        // Confidence must be in valid range
        prop_assert!(
            recommendation.confidence >= 0.0 && recommendation.confidence <= 1.0,
            "Confidence must be between 0.0 and 1.0, got {}",
            recommendation.confidence
        );

        // Suggested workflows must match detected scale
        let expected_workflows = recommendation.detected_scale.recommended_workflows();
        prop_assert_eq!(
            recommendation.suggested_workflows,
            expected_workflows.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "Suggested workflows must match scale recommendations"
        );

        // Suggested agents must match detected scale
        let expected_agents = recommendation.detected_scale.recommended_agents();
        prop_assert_eq!(
            recommendation.suggested_agents,
            expected_agents.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            "Suggested agents must match scale recommendations"
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_scale_detection_with_small_project() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(100)
            .with_file_count(3)
            .with_dependency_count(2);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::BugFix);
    }

    #[test]
    fn test_scale_detection_with_large_project() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(150_000)
            .with_file_count(1500)
            .with_dependency_count(150)
            .with_contributor_count(30)
            .with_ci_cd(true)
            .with_tests(true)
            .with_docs(true);

        let scale = detector.detect(&context);
        assert_eq!(scale, ProjectScale::Enterprise);
    }

    #[test]
    fn test_workflow_selection_bug_fix() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new().with_scale_override(ProjectScale::BugFix);

        let workflows = detector.select_workflows(&context);
        assert!(workflows.contains(&"quick-bug-fix"));
        assert!(workflows.contains(&"quick-refactor"));
    }

    #[test]
    fn test_workflow_selection_enterprise() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new().with_scale_override(ProjectScale::Enterprise);

        let workflows = detector.select_workflows(&context);
        assert!(workflows.contains(&"security-review"));
        assert!(workflows.contains(&"deployment"));
        assert!(workflows.len() > 10);
    }

    #[test]
    fn test_recommendation_has_reasoning() {
        let detector = ScaleDetector::new();
        let context = ProjectContext::new()
            .with_lines_of_code(5000)
            .with_file_count(50)
            .with_dependency_count(20);

        let recommendation = detector.recommend(&context);
        assert!(!recommendation.reasoning.is_empty());
        assert!(recommendation.reasoning.iter().any(|r| r.contains("lines of code")));
    }

    #[test]
    fn test_scale_display() {
        assert_eq!(format!("{}", ProjectScale::BugFix), "Bug Fix");
        assert_eq!(format!("{}", ProjectScale::Feature), "Feature");
        assert_eq!(format!("{}", ProjectScale::Product), "Product");
        assert_eq!(format!("{}", ProjectScale::Enterprise), "Enterprise");
    }
}
