//! Steering CLI Commands
//!
//! CLI commands for managing agent steering rules.

use crate::steering::{AgentContext, SteeringEngine, SteeringInclusion, SteeringRule};
use crate::{DrivenError, Result};
use std::path::{Path, PathBuf};

/// Steering command handler
pub struct SteerCommand;

impl SteerCommand {
    /// List all steering rules
    pub fn list(project_root: &Path) -> Result<Vec<SteeringInfo>> {
        let steering_dir = project_root.join(".driven/steering");
        let mut engine = SteeringEngine::with_steering_dir(&steering_dir);

        if steering_dir.exists() {
            engine.load_steering(&steering_dir)?;
        }

        let rules: Vec<SteeringInfo> = engine.list_rules().iter().map(SteeringInfo::from).collect();

        Ok(rules)
    }

    /// Add a new steering rule
    pub fn add(
        project_root: &Path,
        id: &str,
        name: Option<&str>,
        inclusion_type: &str,
        pattern_or_key: Option<&str>,
        content: &str,
        priority: Option<u8>,
    ) -> Result<()> {
        let steering_dir = project_root.join(".driven/steering");
        std::fs::create_dir_all(&steering_dir).map_err(DrivenError::Io)?;

        // Parse inclusion type
        let inclusion = Self::parse_inclusion(inclusion_type, pattern_or_key)?;

        // Create rule
        let rule = SteeringRule::new(id)
            .with_name(name.unwrap_or(id))
            .with_inclusion(inclusion)
            .with_content(content)
            .with_priority(priority.unwrap_or(100));

        // Save rule to file
        let rule_path = steering_dir.join(format!("{}.md", id));

        let engine = SteeringEngine::new();
        engine.save_rule(&rule, &rule_path)?;

        super::print_success(&format!("Steering rule '{}' added successfully", id));
        Ok(())
    }

    /// Remove a steering rule
    pub fn remove(project_root: &Path, id: &str) -> Result<()> {
        let steering_dir = project_root.join(".driven/steering");

        if !steering_dir.exists() {
            return Err(DrivenError::Config("No steering directory found".to_string()));
        }

        let rule_path = steering_dir.join(format!("{}.md", id));

        if rule_path.exists() {
            std::fs::remove_file(&rule_path).map_err(DrivenError::Io)?;
            super::print_success(&format!("Steering rule '{}' removed successfully", id));
            Ok(())
        } else {
            Err(DrivenError::Config(format!("Steering rule with ID '{}' not found", id)))
        }
    }

    /// Test which rules apply to a file
    pub fn test(
        project_root: &Path,
        file_path: &Path,
        manual_keys: &[String],
    ) -> Result<Vec<SteeringInfo>> {
        let steering_dir = project_root.join(".driven/steering");
        let mut engine = SteeringEngine::with_steering_dir(&steering_dir);

        if steering_dir.exists() {
            engine.load_steering(&steering_dir)?;
        }

        // Build context
        let mut context = AgentContext::new().with_file(file_path);
        for key in manual_keys {
            context = context.with_manual_key(key);
        }

        // Get applicable rules
        let rules = engine.get_rules_for_context(&context);

        Ok(rules.iter().map(|r| SteeringInfo::from(*r)).collect())
    }

    /// Show steering rule details
    pub fn show(project_root: &Path, id: &str) -> Result<SteeringInfo> {
        let steering_dir = project_root.join(".driven/steering");
        let mut engine = SteeringEngine::with_steering_dir(&steering_dir);

        if steering_dir.exists() {
            engine.load_steering(&steering_dir)?;
        }

        let rule = engine.get_rule(id).ok_or_else(|| {
            DrivenError::Config(format!("Steering rule with ID '{}' not found", id))
        })?;

        Ok(SteeringInfo::from(rule))
    }

    /// Get the combined steering content for a context
    pub fn inject(
        project_root: &Path,
        file_path: Option<&Path>,
        manual_keys: &[String],
    ) -> Result<String> {
        let steering_dir = project_root.join(".driven/steering");
        let mut engine = SteeringEngine::with_steering_dir(&steering_dir);

        if steering_dir.exists() {
            engine.load_steering(&steering_dir)?;
            engine.resolve_all_file_references(project_root)?;
        }

        // Build context
        let mut context = AgentContext::new();
        if let Some(path) = file_path {
            context = context.with_file(path);
        }
        for key in manual_keys {
            context = context.with_manual_key(key);
        }

        Ok(engine.inject_into_context(&context))
    }

    /// Parse inclusion type from string
    fn parse_inclusion(
        inclusion_type: &str,
        pattern_or_key: Option<&str>,
    ) -> Result<SteeringInclusion> {
        match inclusion_type.to_lowercase().as_str() {
            "always" => Ok(SteeringInclusion::Always),
            "filematch" | "file_match" | "file" => {
                let pattern = pattern_or_key.ok_or_else(|| {
                    DrivenError::Config("fileMatch inclusion requires a pattern".to_string())
                })?;
                Ok(SteeringInclusion::FileMatch {
                    pattern: pattern.to_string(),
                })
            }
            "manual" => {
                let key = pattern_or_key.ok_or_else(|| {
                    DrivenError::Config("manual inclusion requires a key".to_string())
                })?;
                Ok(SteeringInclusion::Manual {
                    key: key.to_string(),
                })
            }
            _ => Err(DrivenError::Config(format!(
                "Unknown inclusion type: {}. Valid: always, fileMatch, manual",
                inclusion_type
            ))),
        }
    }
}

/// Steering rule information for display
#[derive(Debug, Clone)]
pub struct SteeringInfo {
    pub id: String,
    pub name: String,
    pub inclusion_type: String,
    pub inclusion_value: Option<String>,
    pub priority: u8,
    pub content_preview: String,
    pub file_reference_count: usize,
    pub source_path: Option<PathBuf>,
}

impl From<&SteeringRule> for SteeringInfo {
    fn from(rule: &SteeringRule) -> Self {
        let (inclusion_type, inclusion_value) = match &rule.inclusion {
            SteeringInclusion::Always => ("always".to_string(), None),
            SteeringInclusion::FileMatch { pattern } => {
                ("fileMatch".to_string(), Some(pattern.clone()))
            }
            SteeringInclusion::Manual { key } => ("manual".to_string(), Some(key.clone())),
        };

        // Create content preview (first 50 chars)
        let content_preview = if rule.content.len() > 50 {
            format!("{}...", &rule.content[..50].replace('\n', " "))
        } else {
            rule.content.replace('\n', " ")
        };

        Self {
            id: rule.id.clone(),
            name: rule.name.clone(),
            inclusion_type,
            inclusion_value,
            priority: rule.priority,
            content_preview,
            file_reference_count: rule.file_references.len(),
            source_path: rule.source_path.clone(),
        }
    }
}

/// Print steering rules in a formatted table
pub fn print_steering_table(rules: &[SteeringInfo]) {
    use console::style;

    if rules.is_empty() {
        println!("No steering rules configured.");
        return;
    }

    // Print header
    println!(
        "{:<20} {:<15} {:<25} {:<8}",
        style("ID").bold(),
        style("Inclusion").bold(),
        style("Pattern/Key").bold(),
        style("Priority").bold(),
    );
    println!("{}", "-".repeat(70));

    // Print rules
    for rule in rules {
        let inclusion_value = rule.inclusion_value.as_deref().unwrap_or("-");

        println!(
            "{:<20} {:<15} {:<25} {:<8}",
            rule.id, rule.inclusion_type, inclusion_value, rule.priority,
        );
    }
}

/// Print detailed steering rule information
pub fn print_steering_details(rule: &SteeringInfo) {
    use console::style;

    println!("{}", style("Steering Rule Details").bold().underlined());
    println!();
    println!("  {}: {}", style("ID").bold(), rule.id);
    println!("  {}: {}", style("Name").bold(), rule.name);
    println!("  {}: {}", style("Inclusion Type").bold(), rule.inclusion_type);
    if let Some(value) = &rule.inclusion_value {
        println!("  {}: {}", style("Pattern/Key").bold(), value);
    }
    println!("  {}: {}", style("Priority").bold(), rule.priority);
    println!("  {}: {}", style("File References").bold(), rule.file_reference_count);
    if let Some(path) = &rule.source_path {
        println!("  {}: {}", style("Source").bold(), path.display());
    }
    println!();
    println!("  {}: {}", style("Content Preview").bold(), rule.content_preview);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_inclusion_always() {
        let inclusion = SteerCommand::parse_inclusion("always", None).unwrap();
        assert_eq!(inclusion, SteeringInclusion::Always);
    }

    #[test]
    fn test_parse_inclusion_file_match() {
        let inclusion = SteerCommand::parse_inclusion("fileMatch", Some("**/*.rs")).unwrap();
        match inclusion {
            SteeringInclusion::FileMatch { pattern } => {
                assert_eq!(pattern, "**/*.rs");
            }
            _ => panic!("Wrong inclusion type"),
        }
    }

    #[test]
    fn test_parse_inclusion_manual() {
        let inclusion = SteerCommand::parse_inclusion("manual", Some("rust-style")).unwrap();
        match inclusion {
            SteeringInclusion::Manual { key } => {
                assert_eq!(key, "rust-style");
            }
            _ => panic!("Wrong inclusion type"),
        }
    }

    #[test]
    fn test_parse_inclusion_file_match_requires_pattern() {
        let result = SteerCommand::parse_inclusion("fileMatch", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_inclusion_manual_requires_key() {
        let result = SteerCommand::parse_inclusion("manual", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_add_and_list_steering() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a steering rule
        SteerCommand::add(
            project_root,
            "rust-standards",
            Some("Rust Standards"),
            "always",
            None,
            "# Rust Standards\n\nUse these standards.",
            Some(50),
        )
        .unwrap();

        // List steering rules
        let rules = SteerCommand::list(project_root).unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "rust-standards");
        assert_eq!(rules[0].name, "Rust Standards");
        assert_eq!(rules[0].inclusion_type, "always");
        assert_eq!(rules[0].priority, 50);
    }

    #[test]
    fn test_remove_steering() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a steering rule
        SteerCommand::add(project_root, "test-rule", None, "always", None, "# Test", None).unwrap();

        // Remove the rule
        SteerCommand::remove(project_root, "test-rule").unwrap();

        // List should be empty
        let rules = SteerCommand::list(project_root).unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_test_steering() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add an always rule
        SteerCommand::add(project_root, "always-rule", None, "always", None, "# Always", None)
            .unwrap();

        // Add a file match rule
        SteerCommand::add(
            project_root,
            "rust-rule",
            None,
            "fileMatch",
            Some("**/*.rs"),
            "# Rust",
            None,
        )
        .unwrap();

        // Test with Rust file
        let rules = SteerCommand::test(project_root, Path::new("src/main.rs"), &[]).unwrap();
        assert_eq!(rules.len(), 2); // always + rust

        // Test with Python file
        let rules = SteerCommand::test(project_root, Path::new("src/main.py"), &[]).unwrap();
        assert_eq!(rules.len(), 1); // only always
    }

    #[test]
    fn test_inject_steering() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a steering rule
        SteerCommand::add(project_root, "test-rule", None, "always", None, "# Test Content", None)
            .unwrap();

        // Inject steering
        let content = SteerCommand::inject(project_root, None, &[]).unwrap();
        assert!(content.contains("# Test Content"));
    }
}
