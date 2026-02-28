//! Branch Rules System
//!
//! Parse and validate rules from .sr files

use anyhow::Result;

use super::{Action, Condition, LogLevel, Rule};

/// Parse rules from .sr file content
pub fn parse_rules(content: &str) -> Result<Vec<Rule>> {
    let mut rules = vec![];
    let mut current_rule: Option<RuleBuilder> = None;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse rule directive
        if line.starts_with("rule:") {
            // Save previous rule if exists
            if let Some(builder) = current_rule.take() {
                rules.push(builder.build()?);
            }

            let name = line.trim_start_matches("rule:").trim();
            current_rule = Some(RuleBuilder::new(name));
        } else if let Some(ref mut builder) = current_rule {
            // Parse rule properties
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "condition" => builder.condition = Some(parse_condition(value)?),
                    "action" => builder.action = Some(parse_action(value)?),
                    "priority" => builder.priority = value.parse().ok(),
                    _ => {} // Unknown key, ignore
                }
            }
        }
    }

    // Save last rule
    if let Some(builder) = current_rule {
        rules.push(builder.build()?);
    }

    Ok(rules)
}

/// Rule builder for parsing
struct RuleBuilder {
    name: String,
    condition: Option<Condition>,
    action: Option<Action>,
    priority: Option<u8>,
}

impl RuleBuilder {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            condition: None,
            action: None,
            priority: None,
        }
    }

    fn build(self) -> Result<Rule> {
        Ok(Rule {
            id: uuid::Uuid::new_v4().to_string(),
            condition: self.condition.unwrap_or(Condition::Always),
            action: self.action.unwrap_or(Action::Route),
            priority: self.priority.unwrap_or(1),
        })
    }
}

/// Parse condition from string
fn parse_condition(value: &str) -> Result<Condition> {
    let value = value.trim();

    // Handle operators
    if value.contains(" && ") {
        let parts: Vec<_> = value
            .split(" && ")
            .map(|s| parse_condition(s.trim()))
            .collect::<Result<Vec<_>>>()?;
        return Ok(Condition::And(parts));
    }

    if value.contains(" || ") {
        let parts: Vec<_> = value
            .split(" || ")
            .map(|s| parse_condition(s.trim()))
            .collect::<Result<Vec<_>>>()?;
        return Ok(Condition::Or(parts));
    }

    if value.starts_with("!") || value.starts_with("not ") {
        let inner = value.trim_start_matches("!").trim_start_matches("not ").trim();
        return Ok(Condition::Not(Box::new(parse_condition(inner)?)));
    }

    // Handle simple conditions
    if value == "always" || value == "*" {
        return Ok(Condition::Always);
    }

    if value.starts_with("pattern:") {
        let pattern = value.trim_start_matches("pattern:").trim();
        return Ok(Condition::FilePattern(pattern.to_string()));
    }

    if value.starts_with("score<") {
        let threshold: u32 = value.trim_start_matches("score<").trim().parse()?;
        return Ok(Condition::ScoreBelow(threshold));
    }

    if value.starts_with("score>") {
        let threshold: u32 = value.trim_start_matches("score>").trim().parse()?;
        return Ok(Condition::ScoreAbove(threshold));
    }

    if value.starts_with("score:") {
        let range = value.trim_start_matches("score:").trim();
        if let Some((min, max)) = range.split_once('-') {
            return Ok(Condition::ScoreRange {
                min: min.trim().parse()?,
                max: max.trim().parse()?,
            });
        }
    }

    if value.starts_with("type:") {
        let file_type = value.trim_start_matches("type:").trim();
        let ft = match file_type.to_lowercase().as_str() {
            "rust" | "rs" => super::FileType::Rust,
            "javascript" | "js" => super::FileType::JavaScript,
            "typescript" | "ts" => super::FileType::TypeScript,
            "python" | "py" => super::FileType::Python,
            "go" => super::FileType::Go,
            "markdown" | "md" => super::FileType::Markdown,
            "config" => super::FileType::Config,
            _ => super::FileType::Other,
        };
        return Ok(Condition::FileType(ft));
    }

    // Treat as expression
    Ok(Condition::Expression(value.to_string()))
}

/// Parse action from string
fn parse_action(value: &str) -> Result<Action> {
    let value = value.trim();

    if value == "route" {
        return Ok(Action::Route);
    }

    if value.starts_with("block:") {
        let reason = value.trim_start_matches("block:").trim();
        return Ok(Action::Block {
            reason: reason.to_string(),
        });
    }

    if value.starts_with("transform:") {
        let transformer = value.trim_start_matches("transform:").trim();
        return Ok(Action::Transform {
            transformer: transformer.to_string(),
        });
    }

    if value.starts_with("log:") {
        let level = value.trim_start_matches("log:").trim();
        let log_level = match level.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info,
        };
        return Ok(Action::Log { level: log_level });
    }

    if value.starts_with("bot:") {
        let bot_id = value.trim_start_matches("bot:").trim();
        return Ok(Action::ApplyBot {
            bot_id: bot_id.to_string(),
        });
    }

    if value.starts_with("chain:") {
        let branch_id = value.trim_start_matches("chain:").trim();
        return Ok(Action::Chain {
            branch_id: branch_id.to_string(),
        });
    }

    // Default to route
    Ok(Action::Route)
}

/// Validate rules
pub fn validate_rules(rules: &[Rule]) -> Vec<ValidationError> {
    let mut errors = vec![];

    for (i, rule) in rules.iter().enumerate() {
        // Check for duplicate IDs
        for (j, other) in rules.iter().enumerate() {
            if i != j && rule.id == other.id {
                errors.push(ValidationError {
                    rule_index: i,
                    message: format!("Duplicate rule ID: {}", rule.id),
                });
            }
        }

        // Validate condition
        if let Err(e) = validate_condition(&rule.condition) {
            errors.push(ValidationError {
                rule_index: i,
                message: format!("Invalid condition: {}", e),
            });
        }

        // Validate action
        if let Err(e) = validate_action(&rule.action) {
            errors.push(ValidationError {
                rule_index: i,
                message: format!("Invalid action: {}", e),
            });
        }
    }

    errors
}

fn validate_condition(condition: &Condition) -> Result<()> {
    match condition {
        Condition::FilePattern(pattern) => {
            glob::Pattern::new(pattern)?;
        }
        Condition::ScoreRange { min, max } if min > max => {
            return Err(anyhow::anyhow!("Score range min > max"));
        }
        Condition::And(conditions) | Condition::Or(conditions) => {
            for cond in conditions {
                validate_condition(cond)?;
            }
        }
        Condition::Not(inner) => {
            validate_condition(inner)?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_action(_action: &Action) -> Result<()> {
    // All actions are valid by default
    Ok(())
}

/// Validation error
pub struct ValidationError {
    pub rule_index: usize,
    pub message: String,
}

/// Serialize rules to .sr format
pub fn serialize_rules(rules: &[Rule]) -> String {
    let mut output = String::new();
    output.push_str("# DX Traffic Branch Rules\n\n");

    for rule in rules {
        output.push_str(&format!("rule: {}\n", rule.id));
        output.push_str(&format!("  condition: {}\n", serialize_condition(&rule.condition)));
        output.push_str(&format!("  action: {}\n", serialize_action(&rule.action)));
        output.push_str(&format!("  priority: {}\n", rule.priority));
        output.push('\n');
    }

    output
}

fn serialize_condition(condition: &Condition) -> String {
    match condition {
        Condition::Always => "always".to_string(),
        Condition::FilePattern(p) => format!("pattern:{}", p),
        Condition::ScoreBelow(t) => format!("score<{}", t),
        Condition::ScoreAbove(t) => format!("score>{}", t),
        Condition::ScoreRange { min, max } => format!("score:{}-{}", min, max),
        Condition::FileType(ft) => format!("type:{:?}", ft).to_lowercase(),
        Condition::Expression(e) => e.clone(),
        Condition::And(conditions) => {
            conditions.iter().map(serialize_condition).collect::<Vec<_>>().join(" && ")
        }
        Condition::Or(conditions) => {
            conditions.iter().map(serialize_condition).collect::<Vec<_>>().join(" || ")
        }
        Condition::Not(inner) => format!("!{}", serialize_condition(inner)),
    }
}

fn serialize_action(action: &Action) -> String {
    match action {
        Action::Route => "route".to_string(),
        Action::Block { reason } => format!("block:{}", reason),
        Action::Transform { transformer } => format!("transform:{}", transformer),
        Action::Log { level } => format!("log:{:?}", level).to_lowercase(),
        Action::ApplyBot { bot_id } => format!("bot:{}", bot_id),
        Action::Chain { branch_id } => format!("chain:{}", branch_id),
    }
}
