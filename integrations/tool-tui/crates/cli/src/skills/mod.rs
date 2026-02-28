//! Skills system for AI automation

pub mod analyzer;
pub mod generator;
pub mod knowledge;
pub mod marketplace;
pub mod registry;
pub mod tester;
pub mod validator;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub parameters: HashMap<String, SkillParameter>,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillParameter {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default: Option<String>,
}

pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    pub fn list(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<&Skill> {
        self.skills.values().filter(|s| s.category == category).collect()
    }

    /// Load skills from a workspace directory (`.dx/skills/`)
    pub fn load_workspace_skills(&mut self, workspace_root: &std::path::Path) -> Result<usize> {
        let skills_dir = workspace_root.join(".dx").join("skills");
        if !skills_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(&skills_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let content = std::fs::read_to_string(&path)?;
                if let Ok(skill) = serde_json::from_str::<Skill>(&content) {
                    self.register(skill);
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Load bundled skills from a directory of skill folders
    pub fn load_bundled_skills(&mut self, bundled_dir: &std::path::Path) -> Result<usize> {
        if !bundled_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in std::fs::read_dir(bundled_dir)? {
            let entry = entry?;
            let skill_json = entry.path().join("skill.json");
            if skill_json.exists() {
                let content = std::fs::read_to_string(&skill_json)?;
                if let Ok(skill) = serde_json::from_str::<Skill>(&content) {
                    self.register(skill);
                    count += 1;
                }
            }
        }
        Ok(count)
    }

    /// Install a skill from a JSON definition string
    pub fn install_skill(&mut self, json: &str) -> Result<String> {
        let skill: Skill = serde_json::from_str(json)?;
        let id = skill.id.clone();
        self.register(skill);
        Ok(id)
    }

    /// Uninstall a skill by ID
    pub fn uninstall_skill(&mut self, id: &str) -> Option<Skill> {
        self.skills.remove(id)
    }

    /// Count installed skills
    pub fn count(&self) -> usize {
        self.skills.len()
    }

    /// Export a skill to JSON
    pub fn export_skill(&self, id: &str) -> Result<String> {
        let skill =
            self.skills.get(id).ok_or_else(|| anyhow::anyhow!("Skill not found: {}", id))?;
        Ok(serde_json::to_string_pretty(skill)?)
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn execute_skill(skill: &Skill, params: HashMap<String, String>) -> Result<String> {
    // TODO: Implement skill execution
    Ok(format!("Executed skill: {}", skill.name))
}
