//! Skill validator

use super::Skill;
use anyhow::Result;

pub fn validate(skill: &Skill) -> Result<ValidationResult> {
    let mut errors = vec![];

    if skill.name.is_empty() {
        errors.push("Skill name cannot be empty".to_string());
    }

    if skill.description.is_empty() {
        errors.push("Skill description cannot be empty".to_string());
    }

    Ok(ValidationResult {
        valid: errors.is_empty(),
        errors,
    })
}

pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}
