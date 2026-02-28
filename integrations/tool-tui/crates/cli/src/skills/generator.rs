//! Skill generator

use super::Skill;
use anyhow::Result;

pub fn generate(name: &str, description: &str) -> Result<Skill> {
    Ok(Skill {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        description: description.to_string(),
        category: "custom".to_string(),
        parameters: Default::default(),
        code: String::new(),
    })
}
