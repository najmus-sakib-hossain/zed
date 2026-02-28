//! Agent Steering System
//!
//! This module provides a system for guiding agent behavior and responses
//! through configurable steering rules that can be applied based on context.
//!
//! ## Features
//!
//! - Always-included steering rules that apply to all interactions
//! - Conditional steering rules based on file patterns (fileMatch)
//! - Manual steering rules invoked via `#key` syntax
//! - Rule inheritance from parent directories
//! - File reference resolution via `#[[file:<path>]]` syntax
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::steering::{SteeringRule, SteeringInclusion, SteeringEngine};
//!
//! let mut engine = SteeringEngine::new();
//! engine.load_steering(Path::new(".driven/steering"))?;
//!
//! // Get rules for a specific context
//! let context = AgentContext::new()
//!     .with_file(Path::new("src/main.rs"));
//! let rules = engine.get_rules_for_context(&context);
//! ```

use crate::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Steering rule definition
///
/// Represents a rule that guides agent behavior based on context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SteeringRule {
    /// Unique identifier for the rule
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// How this rule should be included
    pub inclusion: SteeringInclusion,
    /// The content of the steering rule (markdown)
    pub content: String,
    /// File references to resolve
    pub file_references: Vec<FileReference>,
    /// Priority (lower = higher priority)
    pub priority: u8,
    /// Source file path
    pub source_path: Option<PathBuf>,
}

impl SteeringRule {
    /// Create a new steering rule with the given ID
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            inclusion: SteeringInclusion::Always,
            content: String::new(),
            file_references: Vec::new(),
            priority: 100,
            source_path: None,
        }
    }

    /// Set the rule name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the inclusion type
    pub fn with_inclusion(mut self, inclusion: SteeringInclusion) -> Self {
        self.inclusion = inclusion;
        self
    }

    /// Set the content
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the source path
    pub fn with_source_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Add a file reference
    pub fn with_file_reference(mut self, reference: FileReference) -> Self {
        self.file_references.push(reference);
        self
    }

    /// Check if this rule applies to the given context
    pub fn applies_to(&self, context: &AgentContext) -> bool {
        match &self.inclusion {
            SteeringInclusion::Always => true,
            SteeringInclusion::FileMatch { pattern } => {
                if let Some(file_path) = &context.file_path {
                    glob_match(pattern, &file_path.to_string_lossy())
                } else {
                    false
                }
            }
            SteeringInclusion::Manual { key } => context.manual_keys.contains(key),
        }
    }

    /// Get the resolved content with file references inlined
    pub fn resolved_content(&self) -> String {
        let mut content = self.content.clone();

        for reference in &self.file_references {
            if let Some(resolved) = &reference.resolved_content {
                content = content.replace(&reference.syntax, resolved);
            }
        }

        content
    }
}

/// How a steering rule should be included
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SteeringInclusion {
    /// Always include this rule
    Always,
    /// Include when file matches pattern
    FileMatch {
        /// Glob pattern to match
        pattern: String,
    },
    /// Include when manually invoked via #key
    Manual {
        /// Key to invoke this rule
        key: String,
    },
}

/// File reference in a steering rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReference {
    /// The original syntax (e.g., "#[[file:path/to/file.md]]")
    pub syntax: String,
    /// The path to the referenced file
    pub path: PathBuf,
    /// The resolved content (if resolved)
    pub resolved_content: Option<String>,
}

impl FileReference {
    /// Create a new file reference
    pub fn new(syntax: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            syntax: syntax.into(),
            path: path.into(),
            resolved_content: None,
        }
    }

    /// Set the resolved content
    pub fn with_resolved_content(mut self, content: impl Into<String>) -> Self {
        self.resolved_content = Some(content.into());
        self
    }
}

/// Context for agent interactions
#[derive(Debug, Clone, Default)]
pub struct AgentContext {
    /// Current file being worked on
    pub file_path: Option<PathBuf>,
    /// Current directory
    pub directory: Option<PathBuf>,
    /// Manually invoked steering keys
    pub manual_keys: Vec<String>,
    /// Additional context variables
    pub variables: HashMap<String, String>,
}

impl AgentContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the current file
    pub fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set the current directory
    pub fn with_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.directory = Some(path.into());
        self
    }

    /// Add a manual key
    pub fn with_manual_key(mut self, key: impl Into<String>) -> Self {
        self.manual_keys.push(key.into());
        self
    }

    /// Set a variable
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }
}

/// Steering engine for managing and applying steering rules
pub struct SteeringEngine {
    /// Registered steering rules
    rules: Vec<SteeringRule>,
    /// Steering directory
    steering_dir: PathBuf,
    /// Inheritance cache (directory -> applicable rules)
    inheritance_cache: HashMap<PathBuf, Vec<String>>,
}

impl SteeringEngine {
    /// Create a new steering engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            steering_dir: PathBuf::from(".driven/steering"),
            inheritance_cache: HashMap::new(),
        }
    }

    /// Create a steering engine with a custom steering directory
    pub fn with_steering_dir(steering_dir: impl Into<PathBuf>) -> Self {
        Self {
            rules: Vec::new(),
            steering_dir: steering_dir.into(),
            inheritance_cache: HashMap::new(),
        }
    }

    /// Register a steering rule
    pub fn register_rule(&mut self, rule: SteeringRule) -> Result<()> {
        // Check for duplicate ID
        if self.rules.iter().any(|r| r.id == rule.id) {
            return Err(DrivenError::Config(format!(
                "Steering rule with ID '{}' already exists",
                rule.id
            )));
        }

        self.rules.push(rule);

        // Sort by priority
        self.rules.sort_by_key(|r| r.priority);

        Ok(())
    }

    /// Unregister a steering rule by ID
    pub fn unregister_rule(&mut self, id: &str) -> Result<SteeringRule> {
        let pos = self.rules.iter().position(|r| r.id == id).ok_or_else(|| {
            DrivenError::Config(format!("Steering rule with ID '{}' not found", id))
        })?;

        // Clear inheritance cache
        self.inheritance_cache.clear();

        Ok(self.rules.remove(pos))
    }

    /// Get a rule by ID
    pub fn get_rule(&self, id: &str) -> Option<&SteeringRule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// Get a mutable reference to a rule by ID
    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut SteeringRule> {
        self.rules.iter_mut().find(|r| r.id == id)
    }

    /// List all rules
    pub fn list_rules(&self) -> &[SteeringRule] {
        &self.rules
    }

    /// Get rules that apply to a given context
    pub fn get_rules_for_context(&self, context: &AgentContext) -> Vec<&SteeringRule> {
        self.rules.iter().filter(|r| r.applies_to(context)).collect()
    }

    /// Load steering rules from a directory
    pub fn load_steering(&mut self, path: &Path) -> Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        let mut count = 0;

        for entry in std::fs::read_dir(path).map_err(DrivenError::Io)? {
            let entry = entry.map_err(DrivenError::Io)?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "md") {
                match self.load_steering_file(&path) {
                    Ok(rule) => {
                        if let Err(e) = self.register_rule(rule) {
                            tracing::warn!(
                                "Failed to register steering rule from {:?}: {}",
                                path,
                                e
                            );
                        } else {
                            count += 1;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load steering rule from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a single steering rule from a markdown file
    fn load_steering_file(&self, path: &Path) -> Result<SteeringRule> {
        let content = std::fs::read_to_string(path).map_err(DrivenError::Io)?;

        // Parse front matter and content
        let (front_matter, body) = Self::parse_front_matter(&content)?;

        // Generate ID from filename
        let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();

        // Parse inclusion type from front matter
        let inclusion = Self::parse_inclusion(&front_matter)?;

        // Parse priority from front matter
        let priority =
            front_matter.get("priority").and_then(|v| v.parse::<u8>().ok()).unwrap_or(100);

        // Parse file references from content
        let file_references = Self::parse_file_references(&body);

        // Get name from front matter or use ID
        let name = front_matter.get("name").cloned().unwrap_or_else(|| id.clone());

        Ok(SteeringRule {
            id,
            name,
            inclusion,
            content: body,
            file_references,
            priority,
            source_path: Some(path.to_path_buf()),
        })
    }

    /// Parse YAML front matter from markdown content
    fn parse_front_matter(content: &str) -> Result<(HashMap<String, String>, String)> {
        let content = content.trim();

        if !content.starts_with("---") {
            return Ok((HashMap::new(), content.to_string()));
        }

        // Find the end of front matter
        let rest = &content[3..];
        let end_pos = rest
            .find("---")
            .ok_or_else(|| DrivenError::Parse("Unclosed front matter".to_string()))?;

        let front_matter_str = &rest[..end_pos].trim();
        let body = rest[end_pos + 3..].trim().to_string();

        // Parse simple YAML key-value pairs
        let mut front_matter = HashMap::new();
        for line in front_matter_str.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').trim_matches('\'').to_string();
                front_matter.insert(key, value);
            }
        }

        Ok((front_matter, body))
    }

    /// Parse inclusion type from front matter
    fn parse_inclusion(front_matter: &HashMap<String, String>) -> Result<SteeringInclusion> {
        let inclusion_type = front_matter.get("inclusion").map(|s| s.as_str()).unwrap_or("always");

        match inclusion_type.to_lowercase().as_str() {
            "always" => Ok(SteeringInclusion::Always),
            "filematch" | "file_match" => {
                let pattern = front_matter
                    .get("fileMatchPattern")
                    .or_else(|| front_matter.get("file_match_pattern"))
                    .or_else(|| front_matter.get("pattern"))
                    .ok_or_else(|| {
                        DrivenError::Parse(
                            "fileMatch inclusion requires fileMatchPattern".to_string(),
                        )
                    })?;
                Ok(SteeringInclusion::FileMatch {
                    pattern: pattern.clone(),
                })
            }
            "manual" => {
                let key =
                    front_matter.get("key").or_else(|| front_matter.get("manualKey")).ok_or_else(
                        || DrivenError::Parse("manual inclusion requires key".to_string()),
                    )?;
                Ok(SteeringInclusion::Manual { key: key.clone() })
            }
            _ => Err(DrivenError::Parse(format!(
                "Unknown inclusion type: {}. Valid: always, fileMatch, manual",
                inclusion_type
            ))),
        }
    }

    /// Parse file references from content
    fn parse_file_references(content: &str) -> Vec<FileReference> {
        let mut references = Vec::new();

        // Pattern: #[[file:<path>]]
        let pattern = "#[[file:";
        let mut pos = 0;

        while let Some(start) = content[pos..].find(pattern) {
            let abs_start = pos + start;
            let ref_start = abs_start + pattern.len();

            if let Some(end) = content[ref_start..].find("]]") {
                let path_str = &content[ref_start..ref_start + end];
                let syntax = format!("#[[file:{}]]", path_str);

                references.push(FileReference::new(syntax, PathBuf::from(path_str)));

                pos = ref_start + end + 2;
            } else {
                break;
            }
        }

        references
    }

    /// Resolve file references in a rule
    pub fn resolve_file_references(&self, rule: &mut SteeringRule, base_path: &Path) -> Result<()> {
        for reference in &mut rule.file_references {
            let full_path = if reference.path.is_absolute() {
                reference.path.clone()
            } else {
                base_path.join(&reference.path)
            };

            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    reference.resolved_content = Some(content);
                }
                Err(e) => {
                    tracing::warn!("Failed to resolve file reference {:?}: {}", full_path, e);
                }
            }
        }

        Ok(())
    }

    /// Resolve all file references in all rules
    pub fn resolve_all_file_references(&mut self, base_path: &Path) -> Result<()> {
        for rule in &mut self.rules {
            for reference in &mut rule.file_references {
                let full_path = if reference.path.is_absolute() {
                    reference.path.clone()
                } else {
                    base_path.join(&reference.path)
                };

                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        reference.resolved_content = Some(content);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to resolve file reference {:?}: {}", full_path, e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Get rules with inheritance from parent directories
    pub fn get_rules_with_inheritance(&self, directory: &Path) -> Vec<&SteeringRule> {
        let mut applicable_rules: Vec<&SteeringRule> = Vec::new();
        let mut seen_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();

        // Walk up the directory tree
        let mut current = Some(directory);
        while let Some(dir) = current {
            // Check for steering directory in this location
            let steering_dir = dir.join(".driven/steering");

            // Find rules that came from this directory
            for rule in &self.rules {
                if let Some(source) = &rule.source_path {
                    if source.starts_with(&steering_dir) && !seen_ids.contains(rule.id.as_str()) {
                        applicable_rules.push(rule);
                        seen_ids.insert(&rule.id);
                    }
                }
            }

            current = dir.parent();
        }

        // Sort by priority (child rules have been added first, so they take precedence)
        applicable_rules.sort_by_key(|r| r.priority);

        applicable_rules
    }

    /// Inject applicable steering rules into an agent context
    pub fn inject_into_context(&self, context: &AgentContext) -> String {
        let rules = self.get_rules_for_context(context);

        let mut output = String::new();

        for rule in rules {
            if !output.is_empty() {
                output.push_str("\n\n---\n\n");
            }
            output.push_str(&rule.resolved_content());
        }

        output
    }

    /// Save a steering rule to a file
    pub fn save_rule(&self, rule: &SteeringRule, path: &Path) -> Result<()> {
        let mut content = String::new();

        // Write front matter
        content.push_str("---\n");
        content.push_str(&format!("name: \"{}\"\n", rule.name));

        match &rule.inclusion {
            SteeringInclusion::Always => {
                content.push_str("inclusion: always\n");
            }
            SteeringInclusion::FileMatch { pattern } => {
                content.push_str("inclusion: fileMatch\n");
                content.push_str(&format!("fileMatchPattern: \"{}\"\n", pattern));
            }
            SteeringInclusion::Manual { key } => {
                content.push_str("inclusion: manual\n");
                content.push_str(&format!("key: \"{}\"\n", key));
            }
        }

        content.push_str(&format!("priority: {}\n", rule.priority));
        content.push_str("---\n\n");

        // Write content
        content.push_str(&rule.content);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DrivenError::Io)?;
        }

        std::fs::write(path, content).map_err(DrivenError::Io)?;

        Ok(())
    }

    /// Get the steering directory
    pub fn steering_dir(&self) -> &Path {
        &self.steering_dir
    }
}

impl Default for SteeringEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Handle ** (match any path)
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');

            if !prefix.is_empty() && !text.starts_with(prefix) {
                return false;
            }
            if !suffix.is_empty() && !glob_match(suffix, text.rsplit('/').next().unwrap_or(text)) {
                return false;
            }
            return true;
        }
    }

    // Handle * (match any characters except /)
    if pattern.contains('*') && !pattern.contains("**") {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                // First part must be at the start
                if !text.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == parts.len() - 1 {
                // Last part must be at the end
                if !text.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts must exist somewhere
                if let Some(found) = text[pos..].find(part) {
                    pos += found + part.len();
                } else {
                    return false;
                }
            }
        }

        return true;
    }

    // Exact match
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_steering_rule_creation() {
        let rule = SteeringRule::new("test-rule")
            .with_name("Test Rule")
            .with_inclusion(SteeringInclusion::Always)
            .with_content("# Test Content")
            .with_priority(50);

        assert_eq!(rule.id, "test-rule");
        assert_eq!(rule.name, "Test Rule");
        assert_eq!(rule.content, "# Test Content");
        assert_eq!(rule.priority, 50);
    }

    #[test]
    fn test_always_inclusion() {
        let rule = SteeringRule::new("always-rule").with_inclusion(SteeringInclusion::Always);

        let context = AgentContext::new();
        assert!(rule.applies_to(&context));

        let context_with_file = AgentContext::new().with_file(Path::new("src/main.rs"));
        assert!(rule.applies_to(&context_with_file));
    }

    #[test]
    fn test_file_match_inclusion() {
        let rule = SteeringRule::new("rust-rule").with_inclusion(SteeringInclusion::FileMatch {
            pattern: "**/*.rs".to_string(),
        });

        let rust_context = AgentContext::new().with_file(Path::new("src/main.rs"));
        assert!(rule.applies_to(&rust_context));

        let python_context = AgentContext::new().with_file(Path::new("src/main.py"));
        assert!(!rule.applies_to(&python_context));

        let no_file_context = AgentContext::new();
        assert!(!rule.applies_to(&no_file_context));
    }

    #[test]
    fn test_manual_inclusion() {
        let rule = SteeringRule::new("manual-rule").with_inclusion(SteeringInclusion::Manual {
            key: "rust-style".to_string(),
        });

        let context_without_key = AgentContext::new();
        assert!(!rule.applies_to(&context_without_key));

        let context_with_key = AgentContext::new().with_manual_key("rust-style");
        assert!(rule.applies_to(&context_with_key));

        let context_with_wrong_key = AgentContext::new().with_manual_key("python-style");
        assert!(!rule.applies_to(&context_with_wrong_key));
    }

    #[test]
    fn test_file_reference_parsing() {
        let content = r#"
# Rust Style Guide

Follow these rules:

#[[file:docs/rust-style.md]]

Also see:

#[[file:Cargo.toml]]
"#;

        let references = SteeringEngine::parse_file_references(content);
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].path, PathBuf::from("docs/rust-style.md"));
        assert_eq!(references[1].path, PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn test_front_matter_parsing() {
        let content = r#"---
name: "Rust Standards"
inclusion: fileMatch
fileMatchPattern: "**/*.rs"
priority: 10
---

# Rust Code Standards

Use these standards when writing Rust code.
"#;

        let (front_matter, body) = SteeringEngine::parse_front_matter(content).unwrap();

        assert_eq!(front_matter.get("name"), Some(&"Rust Standards".to_string()));
        assert_eq!(front_matter.get("inclusion"), Some(&"fileMatch".to_string()));
        assert_eq!(front_matter.get("fileMatchPattern"), Some(&"**/*.rs".to_string()));
        assert_eq!(front_matter.get("priority"), Some(&"10".to_string()));
        assert!(body.contains("# Rust Code Standards"));
    }

    #[test]
    fn test_front_matter_parsing_no_front_matter() {
        let content = "# Just Content\n\nNo front matter here.";

        let (front_matter, body) = SteeringEngine::parse_front_matter(content).unwrap();

        assert!(front_matter.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_steering_engine_register() {
        let mut engine = SteeringEngine::new();

        let rule = SteeringRule::new("test-rule");
        engine.register_rule(rule).unwrap();

        assert_eq!(engine.list_rules().len(), 1);
        assert!(engine.get_rule("test-rule").is_some());
    }

    #[test]
    fn test_steering_engine_duplicate_id() {
        let mut engine = SteeringEngine::new();

        let rule1 = SteeringRule::new("test-rule");
        let rule2 = SteeringRule::new("test-rule");

        engine.register_rule(rule1).unwrap();
        assert!(engine.register_rule(rule2).is_err());
    }

    #[test]
    fn test_steering_engine_unregister() {
        let mut engine = SteeringEngine::new();

        let rule = SteeringRule::new("test-rule");
        engine.register_rule(rule).unwrap();

        let removed = engine.unregister_rule("test-rule").unwrap();
        assert_eq!(removed.id, "test-rule");
        assert!(engine.get_rule("test-rule").is_none());
    }

    #[test]
    fn test_steering_engine_priority_ordering() {
        let mut engine = SteeringEngine::new();

        let low_priority = SteeringRule::new("low").with_priority(200);
        let high_priority = SteeringRule::new("high").with_priority(50);
        let medium_priority = SteeringRule::new("medium").with_priority(100);

        engine.register_rule(low_priority).unwrap();
        engine.register_rule(high_priority).unwrap();
        engine.register_rule(medium_priority).unwrap();

        let rules = engine.list_rules();
        assert_eq!(rules[0].id, "high");
        assert_eq!(rules[1].id, "medium");
        assert_eq!(rules[2].id, "low");
    }

    #[test]
    fn test_steering_engine_get_rules_for_context() {
        let mut engine = SteeringEngine::new();

        let always_rule = SteeringRule::new("always").with_inclusion(SteeringInclusion::Always);

        let rust_rule = SteeringRule::new("rust").with_inclusion(SteeringInclusion::FileMatch {
            pattern: "**/*.rs".to_string(),
        });

        let manual_rule = SteeringRule::new("manual").with_inclusion(SteeringInclusion::Manual {
            key: "special".to_string(),
        });

        engine.register_rule(always_rule).unwrap();
        engine.register_rule(rust_rule).unwrap();
        engine.register_rule(manual_rule).unwrap();

        // Test with Rust file
        let rust_context = AgentContext::new().with_file(Path::new("src/main.rs"));
        let rules = engine.get_rules_for_context(&rust_context);
        assert_eq!(rules.len(), 2); // always + rust

        // Test with Python file
        let python_context = AgentContext::new().with_file(Path::new("src/main.py"));
        let rules = engine.get_rules_for_context(&python_context);
        assert_eq!(rules.len(), 1); // only always

        // Test with manual key
        let manual_context = AgentContext::new().with_manual_key("special");
        let rules = engine.get_rules_for_context(&manual_context);
        assert_eq!(rules.len(), 2); // always + manual
    }

    #[test]
    fn test_resolved_content() {
        let mut rule = SteeringRule::new("test").with_content("See #[[file:test.md]] for details.");

        rule.file_references.push(
            FileReference::new("#[[file:test.md]]", "test.md")
                .with_resolved_content("# Test Content"),
        );

        let resolved = rule.resolved_content();
        assert_eq!(resolved, "See # Test Content for details.");
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.py"));
        assert!(glob_match("test_*", "test_something"));
        assert!(!glob_match("test_*", "something_test"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "src/lib/mod.rs"));
        assert!(glob_match("**/*.rs", "main.rs"));
        assert!(!glob_match("**/*.rs", "main.py"));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("main.rs", "main.rs"));
        assert!(!glob_match("main.rs", "lib.rs"));
    }

    #[test]
    fn test_agent_context() {
        let context = AgentContext::new()
            .with_file(Path::new("src/main.rs"))
            .with_directory(Path::new("src"))
            .with_manual_key("rust-style")
            .with_variable("project", "driven");

        assert_eq!(context.file_path, Some(PathBuf::from("src/main.rs")));
        assert_eq!(context.directory, Some(PathBuf::from("src")));
        assert!(context.manual_keys.contains(&"rust-style".to_string()));
        assert_eq!(context.variables.get("project"), Some(&"driven".to_string()));
    }

    #[test]
    fn test_inject_into_context() {
        let mut engine = SteeringEngine::new();

        let rule1 = SteeringRule::new("rule1")
            .with_inclusion(SteeringInclusion::Always)
            .with_content("# Rule 1 Content")
            .with_priority(10);

        let rule2 = SteeringRule::new("rule2")
            .with_inclusion(SteeringInclusion::Always)
            .with_content("# Rule 2 Content")
            .with_priority(20);

        engine.register_rule(rule1).unwrap();
        engine.register_rule(rule2).unwrap();

        let context = AgentContext::new();
        let output = engine.inject_into_context(&context);

        assert!(output.contains("# Rule 1 Content"));
        assert!(output.contains("# Rule 2 Content"));
        assert!(output.contains("---")); // Separator between rules
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate an arbitrary file extension
    fn arb_extension() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("rs".to_string()),
            Just("py".to_string()),
            Just("js".to_string()),
            Just("ts".to_string()),
            Just("md".to_string()),
            Just("json".to_string()),
            Just("yaml".to_string()),
        ]
    }

    /// Generate an arbitrary file path
    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        (
            prop_oneof![Just("src"), Just("tests"), Just("lib"), Just("bin"),],
            "[a-z_]+",
            arb_extension(),
        )
            .prop_map(|(dir, name, ext)| PathBuf::from(format!("{}/{}.{}", dir, name, ext)))
    }

    /// Generate an arbitrary inclusion type
    fn arb_inclusion() -> impl Strategy<Value = SteeringInclusion> {
        prop_oneof![
            Just(SteeringInclusion::Always),
            arb_extension().prop_map(|ext| SteeringInclusion::FileMatch {
                pattern: format!("**/*.{}", ext),
            }),
            "[a-z_]+".prop_map(|key| SteeringInclusion::Manual { key }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 12: Steering Rule Application
        /// *For any* agent context, the steering engine SHALL inject all applicable rules
        /// based on inclusion type (always, fileMatch, manual).
        /// **Validates: Requirements 4.1, 4.2, 4.3, 4.5**
        #[test]
        fn prop_always_rules_always_apply(
            path in arb_file_path(),
            content in "[a-zA-Z ]+",
        ) {
            let mut engine = SteeringEngine::new();

            let rule = SteeringRule::new("always-rule")
                .with_inclusion(SteeringInclusion::Always)
                .with_content(&content);
            engine.register_rule(rule).unwrap();

            // Test with file context
            let context = AgentContext::new().with_file(&path);
            let rules = engine.get_rules_for_context(&context);

            prop_assert_eq!(rules.len(), 1);
            prop_assert_eq!(&rules[0].id, "always-rule");

            // Test with empty context
            let empty_context = AgentContext::new();
            let rules = engine.get_rules_for_context(&empty_context);

            prop_assert_eq!(rules.len(), 1);
        }

        /// Property 12b: FileMatch rules apply only when pattern matches
        #[test]
        fn prop_file_match_rules_apply_on_match(
            ext in arb_extension(),
            dir in prop_oneof![Just("src"), Just("tests"), Just("lib")],
            name in "[a-z_]+",
        ) {
            let mut engine = SteeringEngine::new();

            let pattern = format!("**/*.{}", ext);
            let rule = SteeringRule::new("file-rule")
                .with_inclusion(SteeringInclusion::FileMatch { pattern });
            engine.register_rule(rule).unwrap();

            // Test with matching file
            let matching_path = PathBuf::from(format!("{}/{}.{}", dir, name, ext));
            let context = AgentContext::new().with_file(&matching_path);
            let rules = engine.get_rules_for_context(&context);

            prop_assert_eq!(rules.len(), 1);

            // Test with non-matching file
            let non_matching_path = PathBuf::from(format!("{}/{}.different", dir, name));
            let context = AgentContext::new().with_file(&non_matching_path);
            let rules = engine.get_rules_for_context(&context);

            prop_assert_eq!(rules.len(), 0);
        }

        /// Property 12c: Manual rules apply only when key is present
        #[test]
        fn prop_manual_rules_apply_on_key(
            key in "[a-z_]+",
            other_key in "[a-z_]+",
        ) {
            let mut engine = SteeringEngine::new();

            let rule = SteeringRule::new("manual-rule")
                .with_inclusion(SteeringInclusion::Manual { key: key.clone() });
            engine.register_rule(rule).unwrap();

            // Test with matching key
            let context = AgentContext::new().with_manual_key(&key);
            let rules = engine.get_rules_for_context(&context);

            prop_assert_eq!(rules.len(), 1);

            // Test with different key (if different)
            if key != other_key {
                let context = AgentContext::new().with_manual_key(&other_key);
                let rules = engine.get_rules_for_context(&context);

                prop_assert_eq!(rules.len(), 0);
            }
        }

        /// Property 14: Steering File Reference Resolution
        /// *For any* steering rule with file references, all `#[[file:<path>]]` references
        /// SHALL be resolved to their file contents.
        /// **Validates: Requirements 4.9**
        #[test]
        fn prop_file_references_are_parsed(
            path1 in "[a-z_/]+\\.md",
            path2 in "[a-z_/]+\\.txt",
        ) {
            let content = format!(
                "See #[[file:{}]] and #[[file:{}]] for details.",
                path1, path2
            );

            let references = SteeringEngine::parse_file_references(&content);

            prop_assert_eq!(references.len(), 2);
            prop_assert_eq!(references[0].path.clone(), PathBuf::from(&path1));
            prop_assert_eq!(references[1].path.clone(), PathBuf::from(&path2));
        }

        /// Property: Steering rule serialization round-trip
        #[test]
        fn prop_steering_rule_roundtrip(
            id in "[a-z_]+",
            name in "[a-zA-Z ]+",
            content in "[a-zA-Z ]+",
            priority in 0u8..255,
        ) {
            let rule = SteeringRule::new(id.clone())
                .with_name(name.clone())
                .with_inclusion(SteeringInclusion::Always)
                .with_content(content.clone())
                .with_priority(priority);

            // Serialize to JSON
            let json = serde_json::to_string(&rule).expect("Should serialize");

            // Deserialize back
            let loaded: SteeringRule = serde_json::from_str(&json).expect("Should deserialize");

            // Verify fields match
            prop_assert_eq!(loaded.id, id);
            prop_assert_eq!(loaded.name, name);
            prop_assert_eq!(loaded.content, content);
            prop_assert_eq!(loaded.priority, priority);
            prop_assert_eq!(loaded.inclusion, SteeringInclusion::Always);
        }
    }
}
