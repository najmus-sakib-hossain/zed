//! Traffic Branch Router
//!
//! Routes requests to appropriate branches based on rules and conditions

use anyhow::Result;
use std::path::PathBuf;

use super::{Action, Branch, Condition, FileType};

/// Route a request to the best matching branch
pub fn route<'a>(input: &PathBuf, branches: &'a [Branch]) -> Option<&'a Branch> {
    let matches = find_matching_branches(input, branches).ok()?;
    matches.into_iter().next().map(|(b, _)| b)
}

/// Find all matching branches with scores
pub fn find_matching_branches<'a>(
    input: &PathBuf,
    branches: &'a [Branch],
) -> Result<Vec<(&'a Branch, f32)>> {
    let mut matches = vec![];

    for branch in branches.iter().filter(|b| b.active) {
        let score = calculate_match_score(input, branch)?;
        if score > 0.0 {
            matches.push((branch, score));
        }
    }

    // Sort by score descending, then by percentage for tie-breaking
    matches.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.0.percentage.cmp(&a.0.percentage))
    });

    Ok(matches)
}

/// Calculate match score for a branch
fn calculate_match_score(input: &PathBuf, branch: &Branch) -> Result<f32> {
    let mut total_score = 0.0;
    let mut matched_rules = 0;

    for rule in &branch.rules {
        if evaluate_condition(input, &rule.condition)? {
            total_score += rule.priority as f32;
            matched_rules += 1;
        }
    }

    if matched_rules == 0 {
        return Ok(0.0);
    }

    // Normalize score and factor in percentage
    let normalized = total_score / matched_rules as f32;
    Ok(normalized * (branch.percentage as f32 / 100.0))
}

/// Evaluate a condition against input
fn evaluate_condition(input: &PathBuf, condition: &Condition) -> Result<bool> {
    match condition {
        Condition::Always => Ok(true),

        Condition::FilePattern(pattern) => {
            let glob = glob::Pattern::new(pattern)?;
            Ok(glob.matches_path(input))
        }

        Condition::ScoreBelow(threshold) => {
            // TODO: Get actual score from check results
            let score = 500; // Placeholder
            Ok(score < *threshold)
        }

        Condition::ScoreAbove(threshold) => {
            let score = 500; // Placeholder
            Ok(score > *threshold)
        }

        Condition::ScoreRange { min, max } => {
            let score = 500; // Placeholder
            Ok(score >= *min && score <= *max)
        }

        Condition::FileType(expected) => {
            let actual = detect_file_type(input);
            Ok(std::mem::discriminant(&actual) == std::mem::discriminant(expected))
        }

        Condition::Expression(_expr) => {
            // TODO: Evaluate custom expression
            Ok(false)
        }

        Condition::And(conditions) => {
            for cond in conditions {
                if !evaluate_condition(input, cond)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }

        Condition::Or(conditions) => {
            for cond in conditions {
                if evaluate_condition(input, cond)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        Condition::Not(inner) => Ok(!evaluate_condition(input, inner)?),
    }
}

/// Detect file type from path
fn detect_file_type(path: &PathBuf) -> FileType {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match ext.to_lowercase().as_str() {
        "rs" => FileType::Rust,
        "js" | "jsx" | "mjs" | "cjs" => FileType::JavaScript,
        "ts" | "tsx" | "mts" | "cts" => FileType::TypeScript,
        "py" | "pyi" => FileType::Python,
        "go" => FileType::Go,
        "md" | "markdown" => FileType::Markdown,
        "json" | "yaml" | "yml" | "toml" | "ini" => FileType::Config,
        _ => FileType::Other,
    }
}

/// Execute action for matched rule
pub fn execute_action(_input: &PathBuf, action: &Action) -> Result<ActionResult> {
    match action {
        Action::Route => Ok(ActionResult::Route),

        Action::Block { reason } => Ok(ActionResult::Blocked {
            reason: reason.clone(),
        }),

        Action::Transform { transformer: _ } => {
            // TODO: Apply transformation
            Ok(ActionResult::Transformed)
        }

        Action::Log { level: _ } => {
            // Log the request
            Ok(ActionResult::Logged)
        }

        Action::ApplyBot { bot_id } => {
            // Route to specific bot
            Ok(ActionResult::BotApplied {
                bot_id: bot_id.clone(),
            })
        }

        Action::Chain { branch_id } => Ok(ActionResult::Chained {
            branch_id: branch_id.clone(),
        }),
    }
}

/// Result of executing an action
pub enum ActionResult {
    Route,
    Blocked { reason: String },
    Transformed,
    Logged,
    BotApplied { bot_id: String },
    Chained { branch_id: String },
}

/// Weighted random selection based on percentages
pub fn select_weighted<'a>(branches: &'a [Branch]) -> Option<&'a Branch> {
    let total: u32 = branches.iter().filter(|b| b.active).map(|b| b.percentage as u32).sum();
    if total == 0 {
        return None;
    }

    let mut rng = rand::random::<u32>() % total;

    for branch in branches.iter().filter(|b| b.active) {
        if rng < branch.percentage as u32 {
            return Some(branch);
        }
        rng -= branch.percentage as u32;
    }

    branches.iter().filter(|b| b.active).last()
}
