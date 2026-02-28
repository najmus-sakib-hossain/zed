//! Skill knowledge base

use std::collections::HashMap;

pub struct KnowledgeBase {
    facts: HashMap<String, String>,
}

impl KnowledgeBase {
    pub fn new() -> Self {
        Self {
            facts: HashMap::new(),
        }
    }

    pub fn add_fact(&mut self, key: String, value: String) {
        self.facts.insert(key, value);
    }

    pub fn get_fact(&self, key: &str) -> Option<&String> {
        self.facts.get(key)
    }
}

impl Default for KnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}
