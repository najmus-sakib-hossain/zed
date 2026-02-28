//! Skill analyzer

use super::Skill;
use anyhow::Result;

pub fn analyze(skill: &Skill) -> Result<AnalysisReport> {
    Ok(AnalysisReport {
        skill_id: skill.id.clone(),
        complexity: 1,
        dependencies: vec![],
    })
}

pub struct AnalysisReport {
    pub skill_id: String,
    pub complexity: u32,
    pub dependencies: Vec<String>,
}
