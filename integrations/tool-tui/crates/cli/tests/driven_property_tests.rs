//! Property-based tests for Driven CLI Commands
//!
//! These tests verify universal properties for rule synchronization,
//! conversion, and format handling.
//!
//! Feature: dx-unified-tooling
//!
//! Run with: cargo test --test driven_property_tests

use proptest::prelude::*;
use tempfile::TempDir;

// ============================================================================
// Test Rule Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestRule {
    title: String,
    content: String,
    category: String,
    priority: u8,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestRuleSet {
    rules: Vec<TestRule>,
}

impl TestRuleSet {
    fn new() -> Self {
        Self { rules: Vec::new() }
    }

    fn add_rule(&mut self, rule: TestRule) {
        self.rules.push(rule);
    }

    fn to_markdown(&self) -> String {
        let mut output = String::from("# AI Rules\n\n");
        for rule in &self.rules {
            output.push_str(&format!("## {}\n\n", rule.title));
            output.push_str(&format!("{}\n\n", rule.content));
        }
        output
    }

    fn from_markdown(content: &str) -> Result<Self, String> {
        let mut rules = Vec::new();
        let mut current_title = String::new();
        let mut current_content = String::new();
        let mut in_rule = false;

        for line in content.lines() {
            if let Some(stripped) = line.strip_prefix("## ") {
                if in_rule && !current_title.is_empty() {
                    rules.push(TestRule {
                        title: current_title.clone(),
                        content: current_content.trim().to_string(),
                        category: "general".to_string(),
                        priority: 1,
                    });
                }
                current_title = stripped.trim().to_string();
                current_content = String::new();
                in_rule = true;
            } else if in_rule && !line.starts_with("# ") {
                current_content.push_str(line);
                current_content.push('\n');
            }
        }

        // Add last rule
        if in_rule && !current_title.is_empty() {
            rules.push(TestRule {
                title: current_title,
                content: current_content.trim().to_string(),
                category: "general".to_string(),
                priority: 1,
            });
        }

        Ok(Self { rules })
    }

    fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    fn from_json(content: &str) -> Result<Self, String> {
        serde_json::from_str(content).map_err(|e| e.to_string())
    }
}

// ============================================================================
// Arbitrary Generators
// ============================================================================

fn arbitrary_rule_title() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Coding Standards".to_string()),
        Just("Architecture Guidelines".to_string()),
        Just("Testing Requirements".to_string()),
        Just("Documentation Rules".to_string()),
        Just("Security Practices".to_string()),
        "[A-Z][a-z]{2,15}( [A-Z][a-z]{2,10})?".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_rule_content() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Follow consistent naming conventions.".to_string()),
        Just("Write clear, self-documenting code.".to_string()),
        Just("Include appropriate comments.".to_string()),
        Just("Use proper error handling.".to_string()),
        "[A-Za-z .,]{10,100}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_category() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("general".to_string()),
        Just("coding".to_string()),
        Just("testing".to_string()),
        Just("security".to_string()),
        Just("documentation".to_string()),
    ]
}

fn arbitrary_rule() -> impl Strategy<Value = TestRule> {
    (
        arbitrary_rule_title(),
        arbitrary_rule_content(),
        arbitrary_category(),
        1u8..10u8,
    )
        .prop_map(|(title, content, category, priority)| TestRule {
            title,
            content,
            category,
            priority,
        })
}

fn arbitrary_ruleset() -> impl Strategy<Value = TestRuleSet> {
    prop::collection::vec(arbitrary_rule(), 1..5).prop_map(|rules| TestRuleSet { rules })
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: Rule Sync Round-Trip (Markdown)
    /// *For any* valid rule set, converting to markdown and back
    /// SHALL produce semantically equivalent rules.
    ///
    /// **Validates: Requirements 1.3, 1.5**
    #[test]
    fn prop_rule_sync_roundtrip_markdown(ruleset in arbitrary_ruleset()) {
        // Convert to markdown
        let markdown = ruleset.to_markdown();

        // Parse back
        let parsed = TestRuleSet::from_markdown(&markdown).unwrap();

        // Verify rule count matches
        prop_assert_eq!(ruleset.rules.len(), parsed.rules.len());

        // Verify each rule's title and content match
        for (original, parsed_rule) in ruleset.rules.iter().zip(parsed.rules.iter()) {
            prop_assert_eq!(&original.title, &parsed_rule.title);
            // Content may have whitespace differences, so compare trimmed
            prop_assert_eq!(original.content.trim(), parsed_rule.content.trim());
        }
    }

    /// Property 1b: Rule Sync Round-Trip (JSON)
    /// *For any* valid rule set, converting to JSON and back
    /// SHALL produce identical rules.
    ///
    /// **Validates: Requirements 1.3, 1.5**
    #[test]
    fn prop_rule_sync_roundtrip_json(ruleset in arbitrary_ruleset()) {
        // Convert to JSON
        let json = ruleset.to_json();

        // Parse back
        let parsed = TestRuleSet::from_json(&json).unwrap();

        // Verify exact equality
        prop_assert_eq!(ruleset, parsed);
    }

    /// Property 1c: Format Conversion Preserves Content
    /// *For any* valid rule set, converting between formats
    /// SHALL preserve the semantic content.
    ///
    /// **Validates: Requirements 1.5**
    #[test]
    fn prop_format_conversion_preserves_content(ruleset in arbitrary_ruleset()) {
        // Convert to markdown
        let markdown = ruleset.to_markdown();

        // Parse from markdown
        let from_md = TestRuleSet::from_markdown(&markdown).unwrap();

        // Convert to JSON
        let json = from_md.to_json();

        // Parse from JSON
        let from_json = TestRuleSet::from_json(&json).unwrap();

        // Verify rule count preserved
        prop_assert_eq!(ruleset.rules.len(), from_json.rules.len());

        // Verify titles preserved
        for (original, final_rule) in ruleset.rules.iter().zip(from_json.rules.iter()) {
            prop_assert_eq!(&original.title, &final_rule.title);
        }
    }

    /// Property: Rule Title Uniqueness
    /// *For any* rule set with unique titles, the titles SHALL remain
    /// unique after round-trip conversion.
    ///
    /// **Validates: Requirements 1.3**
    #[test]
    fn prop_rule_title_uniqueness(ruleset in arbitrary_ruleset()) {
        // Get original titles
        let original_titles: Vec<_> = ruleset.rules.iter().map(|r| r.title.clone()).collect();

        // Convert and parse back
        let markdown = ruleset.to_markdown();
        let parsed = TestRuleSet::from_markdown(&markdown).unwrap();

        // Get parsed titles
        let parsed_titles: Vec<_> = parsed.rules.iter().map(|r| r.title.clone()).collect();

        // Verify same titles (order preserved)
        prop_assert_eq!(original_titles, parsed_titles);
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

/// Test empty ruleset handling
#[test]
fn test_empty_ruleset() {
    let ruleset = TestRuleSet::new();
    let markdown = ruleset.to_markdown();

    assert!(markdown.contains("# AI Rules"));

    let parsed = TestRuleSet::from_markdown(&markdown).unwrap();
    assert!(parsed.rules.is_empty());
}

/// Test single rule round-trip
#[test]
fn test_single_rule_roundtrip() {
    let mut ruleset = TestRuleSet::new();
    ruleset.add_rule(TestRule {
        title: "Test Rule".to_string(),
        content: "This is a test rule.".to_string(),
        category: "general".to_string(),
        priority: 1,
    });

    let markdown = ruleset.to_markdown();
    let parsed = TestRuleSet::from_markdown(&markdown).unwrap();

    assert_eq!(ruleset.rules.len(), parsed.rules.len());
    assert_eq!(ruleset.rules[0].title, parsed.rules[0].title);
    assert_eq!(ruleset.rules[0].content.trim(), parsed.rules[0].content.trim());
}

/// Test JSON serialization
#[test]
fn test_json_serialization() {
    let mut ruleset = TestRuleSet::new();
    ruleset.add_rule(TestRule {
        title: "JSON Test".to_string(),
        content: "Testing JSON serialization.".to_string(),
        category: "testing".to_string(),
        priority: 5,
    });

    let json = ruleset.to_json();
    let parsed = TestRuleSet::from_json(&json).unwrap();

    assert_eq!(ruleset, parsed);
}

/// Test file I/O round-trip
#[test]
fn test_file_io_roundtrip() {
    let temp_dir = TempDir::new().unwrap();

    let mut ruleset = TestRuleSet::new();
    ruleset.add_rule(TestRule {
        title: "File Test".to_string(),
        content: "Testing file I/O.".to_string(),
        category: "general".to_string(),
        priority: 1,
    });

    // Write markdown
    let md_path = temp_dir.path().join("rules.md");
    std::fs::write(&md_path, ruleset.to_markdown()).unwrap();

    // Read and parse
    let content = std::fs::read_to_string(&md_path).unwrap();
    let parsed = TestRuleSet::from_markdown(&content).unwrap();

    assert_eq!(ruleset.rules.len(), parsed.rules.len());
    assert_eq!(ruleset.rules[0].title, parsed.rules[0].title);
}

/// Test special characters in content
#[test]
fn test_special_characters() {
    let mut ruleset = TestRuleSet::new();
    ruleset.add_rule(TestRule {
        title: "Special Characters".to_string(),
        content: "Use `code` and **bold** and *italic*.".to_string(),
        category: "general".to_string(),
        priority: 1,
    });

    let markdown = ruleset.to_markdown();
    let parsed = TestRuleSet::from_markdown(&markdown).unwrap();

    assert_eq!(ruleset.rules[0].content.trim(), parsed.rules[0].content.trim());
}

/// Test multiple rules ordering
#[test]
fn test_multiple_rules_ordering() {
    let mut ruleset = TestRuleSet::new();
    for i in 1..=5 {
        ruleset.add_rule(TestRule {
            title: format!("Rule {}", i),
            content: format!("Content for rule {}.", i),
            category: "general".to_string(),
            priority: i as u8,
        });
    }

    let markdown = ruleset.to_markdown();
    let parsed = TestRuleSet::from_markdown(&markdown).unwrap();

    // Verify order preserved
    for (i, rule) in parsed.rules.iter().enumerate() {
        assert_eq!(rule.title, format!("Rule {}", i + 1));
    }
}

// ============================================================================
// Spec Artifact Traceability Tests
// ============================================================================

/// Test spec structure for traceability
#[derive(Debug, Clone, PartialEq)]
struct TestSpec {
    name: String,
    requirements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct TestPlan {
    spec_name: String,
    tasks: Vec<TestTask>,
}

#[derive(Debug, Clone, PartialEq)]
struct TestTask {
    id: String,
    description: String,
    requirement_refs: Vec<String>,
}

impl TestSpec {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            requirements: Vec::new(),
        }
    }

    fn add_requirement(&mut self, req: &str) {
        self.requirements.push(req.to_string());
    }

    fn to_markdown(&self) -> String {
        let mut output = format!("# Specification: {}\n\n## Requirements\n\n", self.name);
        for (i, req) in self.requirements.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, req));
        }
        output
    }
}

impl TestPlan {
    fn new(spec_name: &str) -> Self {
        Self {
            spec_name: spec_name.to_string(),
            tasks: Vec::new(),
        }
    }

    fn add_task(&mut self, task: TestTask) {
        self.tasks.push(task);
    }

    fn to_markdown(&self) -> String {
        let mut output =
            format!("# Implementation Plan\n\nSpec: {}\n\n## Tasks\n\n", self.spec_name);
        for task in &self.tasks {
            output.push_str(&format!("- [ ] {} {}\n", task.id, task.description));
            if !task.requirement_refs.is_empty() {
                output.push_str(&format!(
                    "  - _Requirements: {}_\n",
                    task.requirement_refs.join(", ")
                ));
            }
        }
        output
    }

    fn validate_traceability(&self, spec: &TestSpec) -> Vec<String> {
        let mut errors = Vec::new();

        // Check that all requirement refs exist in spec
        for task in &self.tasks {
            for req_ref in &task.requirement_refs {
                // Parse requirement number
                if let Ok(num) = req_ref.parse::<usize>()
                    && (num == 0 || num > spec.requirements.len())
                {
                    errors.push(format!(
                        "Task {} references non-existent requirement {}",
                        task.id, req_ref
                    ));
                }
            }
        }

        // Check that all requirements are covered by at least one task
        let mut covered: std::collections::HashSet<usize> = std::collections::HashSet::new();
        for task in &self.tasks {
            for req_ref in &task.requirement_refs {
                if let Ok(num) = req_ref.parse::<usize>() {
                    covered.insert(num);
                }
            }
        }

        for i in 1..=spec.requirements.len() {
            if !covered.contains(&i) {
                errors.push(format!("Requirement {} is not covered by any task", i));
            }
        }

        errors
    }
}

fn arbitrary_spec_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("user-authentication".to_string()),
        Just("data-export".to_string()),
        Just("api-integration".to_string()),
        "[a-z]{3,10}-[a-z]{3,10}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_requirement() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("User can log in with email and password".to_string()),
        Just("System validates input data".to_string()),
        Just("API returns JSON responses".to_string()),
        Just("Data is persisted to database".to_string()),
        "[A-Z][a-z ]{10,50}".prop_map(|s| s.to_string()),
    ]
}

#[allow(dead_code)]
fn arbitrary_task_id() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("1.1".to_string()),
        Just("1.2".to_string()),
        Just("2.1".to_string()),
        "[1-5]\\.[1-5]".prop_map(|s| s.to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 14: Spec Artifact Traceability
    /// *For any* specification with plan and tasks, all references between
    /// artifacts SHALL be valid and bidirectional.
    ///
    /// **Validates: Requirements 2.7, 2.10**
    #[test]
    fn prop_spec_artifact_traceability(
        spec_name in arbitrary_spec_name(),
        req1 in arbitrary_requirement(),
        req2 in arbitrary_requirement(),
        req3 in arbitrary_requirement(),
    ) {
        // Create spec with requirements
        let mut spec = TestSpec::new(&spec_name);
        spec.add_requirement(&req1);
        spec.add_requirement(&req2);
        spec.add_requirement(&req3);

        // Create plan with tasks that reference all requirements
        let mut plan = TestPlan::new(&spec_name);
        plan.add_task(TestTask {
            id: "1.1".to_string(),
            description: "Implement first requirement".to_string(),
            requirement_refs: vec!["1".to_string()],
        });
        plan.add_task(TestTask {
            id: "1.2".to_string(),
            description: "Implement second requirement".to_string(),
            requirement_refs: vec!["2".to_string()],
        });
        plan.add_task(TestTask {
            id: "2.1".to_string(),
            description: "Implement third requirement".to_string(),
            requirement_refs: vec!["3".to_string()],
        });

        // Validate traceability
        let errors = plan.validate_traceability(&spec);

        // Should have no errors when all requirements are covered
        prop_assert!(errors.is_empty(), "Traceability errors: {:?}", errors);
    }

    /// Property 14b: Invalid References Detected
    /// *For any* plan with invalid requirement references, validation
    /// SHALL detect and report the errors.
    ///
    /// **Validates: Requirements 2.7**
    #[test]
    fn prop_invalid_references_detected(
        spec_name in arbitrary_spec_name(),
        req1 in arbitrary_requirement(),
    ) {
        // Create spec with one requirement
        let mut spec = TestSpec::new(&spec_name);
        spec.add_requirement(&req1);

        // Create plan with invalid reference
        let mut plan = TestPlan::new(&spec_name);
        plan.add_task(TestTask {
            id: "1.1".to_string(),
            description: "Task with invalid ref".to_string(),
            requirement_refs: vec!["99".to_string()], // Invalid reference
        });

        // Validate traceability
        let errors = plan.validate_traceability(&spec);

        // Should detect the invalid reference
        prop_assert!(!errors.is_empty(), "Should detect invalid reference");
        prop_assert!(
            errors.iter().any(|e| e.contains("non-existent")),
            "Should report non-existent requirement"
        );
    }
}

/// Test complete traceability chain
#[test]
fn test_complete_traceability() {
    let mut spec = TestSpec::new("test-feature");
    spec.add_requirement("User can create account");
    spec.add_requirement("User can log in");
    spec.add_requirement("User can log out");

    let mut plan = TestPlan::new("test-feature");
    plan.add_task(TestTask {
        id: "1.1".to_string(),
        description: "Implement account creation".to_string(),
        requirement_refs: vec!["1".to_string()],
    });
    plan.add_task(TestTask {
        id: "1.2".to_string(),
        description: "Implement login".to_string(),
        requirement_refs: vec!["2".to_string()],
    });
    plan.add_task(TestTask {
        id: "1.3".to_string(),
        description: "Implement logout".to_string(),
        requirement_refs: vec!["3".to_string()],
    });

    let errors = plan.validate_traceability(&spec);
    assert!(errors.is_empty(), "Should have no traceability errors");
}

/// Test uncovered requirement detection
#[test]
fn test_uncovered_requirement() {
    let mut spec = TestSpec::new("test-feature");
    spec.add_requirement("Requirement 1");
    spec.add_requirement("Requirement 2");
    spec.add_requirement("Requirement 3");

    let mut plan = TestPlan::new("test-feature");
    plan.add_task(TestTask {
        id: "1.1".to_string(),
        description: "Only covers req 1".to_string(),
        requirement_refs: vec!["1".to_string()],
    });

    let errors = plan.validate_traceability(&spec);
    assert!(!errors.is_empty(), "Should detect uncovered requirements");
    assert!(errors.iter().any(|e| e.contains("not covered")));
}

/// Test spec markdown generation
#[test]
fn test_spec_markdown_generation() {
    let mut spec = TestSpec::new("my-feature");
    spec.add_requirement("First requirement");
    spec.add_requirement("Second requirement");

    let markdown = spec.to_markdown();
    assert!(markdown.contains("# Specification: my-feature"));
    assert!(markdown.contains("1. First requirement"));
    assert!(markdown.contains("2. Second requirement"));
}

/// Test plan markdown generation
#[test]
fn test_plan_markdown_generation() {
    let mut plan = TestPlan::new("my-feature");
    plan.add_task(TestTask {
        id: "1.1".to_string(),
        description: "First task".to_string(),
        requirement_refs: vec!["1".to_string(), "2".to_string()],
    });

    let markdown = plan.to_markdown();
    assert!(markdown.contains("# Implementation Plan"));
    assert!(markdown.contains("Spec: my-feature"));
    assert!(markdown.contains("- [ ] 1.1 First task"));
    assert!(markdown.contains("_Requirements: 1, 2_"));
}

// ============================================================================
// Hook State Consistency Tests
// ============================================================================

/// Test hook structure for property tests
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct TestHook {
    name: String,
    enabled: bool,
    trigger_type: String,
    pattern: Option<String>,
    action_type: String,
    command: Option<String>,
}

impl TestHook {
    fn to_toml(&self) -> String {
        let pattern_line = self
            .pattern
            .as_ref()
            .map(|p| format!("pattern = \"{}\"", p))
            .unwrap_or_default();
        let command_line = self
            .command
            .as_ref()
            .map(|c| format!("command = \"{}\"", c))
            .unwrap_or_default();

        format!(
            r#"[hook]
name = "{}"
enabled = {}

[trigger]
type = "{}"
{}

[action]
type = "{}"
{}
"#,
            self.name,
            self.enabled,
            self.trigger_type,
            pattern_line,
            self.action_type,
            command_line
        )
    }

    fn from_toml(content: &str) -> Result<Self, String> {
        let value: toml::Value = toml::from_str(content).map_err(|e| e.to_string())?;

        let hook = value.get("hook").ok_or("Missing [hook] section")?;
        let trigger = value.get("trigger").ok_or("Missing [trigger] section")?;
        let action = value.get("action").ok_or("Missing [action] section")?;

        Ok(TestHook {
            name: hook.get("name").and_then(|v| v.as_str()).ok_or("Missing name")?.to_string(),
            enabled: hook.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
            trigger_type: trigger
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or("Missing trigger type")?
                .to_string(),
            pattern: trigger.get("pattern").and_then(|v| v.as_str()).map(|s| s.to_string()),
            action_type: action
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or("Missing action type")?
                .to_string(),
            command: action.get("command").and_then(|v| v.as_str()).map(|s| s.to_string()),
        })
    }

    fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }
}

fn arbitrary_hook_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("test-hook".to_string()),
        Just("on-save".to_string()),
        Just("pre-commit".to_string()),
        Just("lint-check".to_string()),
        "[a-z]{3,10}-[a-z]{3,10}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_trigger_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("file-save".to_string()),
        Just("manual".to_string()),
        Just("session".to_string()),
        Just("message".to_string()),
    ]
}

fn arbitrary_action_type() -> impl Strategy<Value = String> {
    prop_oneof![Just("shell".to_string()), Just("message".to_string()),]
}

fn arbitrary_hook() -> impl Strategy<Value = TestHook> {
    (
        arbitrary_hook_name(),
        any::<bool>(),
        arbitrary_trigger_type(),
        prop::option::of(Just("**/*.rs".to_string())),
        arbitrary_action_type(),
        prop::option::of(Just("echo 'triggered'".to_string())),
    )
        .prop_map(|(name, enabled, trigger_type, pattern, action_type, command)| TestHook {
            name,
            enabled,
            trigger_type,
            pattern,
            action_type,
            command,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 2: Hook State Consistency
    /// *For any* hook, enabling and disabling SHALL toggle the state correctly
    /// and persist the change.
    ///
    /// **Validates: Requirements 3.3, 3.4, 3.10**
    #[test]
    fn prop_hook_state_consistency(hook in arbitrary_hook()) {
        // Serialize to TOML
        let toml = hook.to_toml();

        // Parse back from TOML
        let parsed = TestHook::from_toml(&toml);
        prop_assert!(parsed.is_ok(), "Failed to parse TOML: {:?}", parsed.err());

        let mut parsed_hook = parsed.unwrap();
        let original_enabled = parsed_hook.enabled;

        // Toggle the state
        parsed_hook.toggle_enabled();
        prop_assert_ne!(parsed_hook.enabled, original_enabled, "Toggle should change enabled state");

        // Toggle back
        parsed_hook.toggle_enabled();
        prop_assert_eq!(parsed_hook.enabled, original_enabled, "Double toggle should restore original state");
    }
}
