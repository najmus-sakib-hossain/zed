//! Skill tester

use super::Skill;
use anyhow::Result;

pub async fn test_skill(skill: &Skill) -> Result<TestResult> {
    Ok(TestResult {
        skill_id: skill.id.clone(),
        passed: true,
        errors: vec![],
    })
}

pub struct TestResult {
    pub skill_id: String,
    pub passed: bool,
    pub errors: Vec<String>,
}
