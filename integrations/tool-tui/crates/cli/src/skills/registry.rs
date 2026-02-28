//! Skill registry implementation

use super::Skill;

use std::collections::HashMap;

pub struct Registry {
    skills: HashMap<String, Skill>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn add(&mut self, skill: Skill) {
        self.skills.insert(skill.id.clone(), skill);
    }

    pub fn remove(&mut self, id: &str) -> Option<Skill> {
        self.skills.remove(id)
    }

    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    pub fn list_all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
