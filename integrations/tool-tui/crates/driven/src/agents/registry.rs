//! Agent Registry
//!
//! Manages registration and lookup of agents.

use super::Agent;
use crate::{DrivenError, Result};
use std::collections::HashMap;
use std::path::Path;

/// Registry for managing agents
#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Agent>,
}

impl AgentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Load all built-in agents
    pub fn load_builtin(&mut self) {
        let builtin_agents = super::builtin::all_agents();
        for agent in builtin_agents {
            self.agents.insert(agent.id.clone(), agent);
        }
    }

    /// Load custom agents from a YAML file
    pub fn load_custom(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let agents: Vec<Agent> = serde_yaml::from_str(&content)
            .map_err(|e| DrivenError::Config(format!("Failed to parse agent YAML: {}", e)))?;

        for agent in agents {
            self.register(agent)?;
        }
        Ok(())
    }

    /// Load custom agents from a directory
    pub fn load_custom_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml" || ext == "yml") {
                self.load_custom(&path)?;
            }
        }
        Ok(())
    }

    /// Register a new agent
    pub fn register(&mut self, agent: Agent) -> Result<()> {
        if self.agents.contains_key(&agent.id) {
            return Err(DrivenError::Config(format!(
                "Agent with id '{}' already registered",
                agent.id
            )));
        }
        self.agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    /// Get an agent by ID
    pub fn get(&self, id: &str) -> Option<&Agent> {
        self.agents.get(id)
    }

    /// Get a mutable reference to an agent by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Agent> {
        self.agents.get_mut(id)
    }

    /// List all registered agents
    pub fn list(&self) -> Vec<&Agent> {
        self.agents.values().collect()
    }

    /// List agents by capability
    pub fn list_by_capability(&self, capability: &str) -> Vec<&Agent> {
        self.agents.values().filter(|a| a.has_capability(capability)).collect()
    }

    /// List agents by workflow
    pub fn list_by_workflow(&self, workflow: &str) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| a.workflows.iter().any(|w| w == workflow))
            .collect()
    }

    /// List only built-in agents
    pub fn list_builtin(&self) -> Vec<&Agent> {
        self.agents.values().filter(|a| a.builtin).collect()
    }

    /// List only custom agents
    pub fn list_custom(&self) -> Vec<&Agent> {
        self.agents.values().filter(|a| !a.builtin).collect()
    }

    /// Remove an agent by ID
    pub fn remove(&mut self, id: &str) -> Option<Agent> {
        self.agents.remove(id)
    }

    /// Check if an agent exists
    pub fn contains(&self, id: &str) -> bool {
        self.agents.contains_key(id)
    }

    /// Get the number of registered agents
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Delegate a request from one agent to another
    pub fn delegate(&self, from_agent_id: &str, to_agent_id: &str) -> Result<&Agent> {
        let from_agent = self.get(from_agent_id).ok_or_else(|| {
            DrivenError::Config(format!("Source agent '{}' not found", from_agent_id))
        })?;

        if !from_agent.can_delegate_to(to_agent_id) {
            return Err(DrivenError::Config(format!(
                "Agent '{}' cannot delegate to '{}'",
                from_agent_id, to_agent_id
            )));
        }

        self.get(to_agent_id)
            .ok_or_else(|| DrivenError::Config(format!("Target agent '{}' not found", to_agent_id)))
    }

    /// Process a delegation request
    pub fn process_delegation(
        &self,
        request: &super::DelegationRequest,
    ) -> Result<super::DelegationResult> {
        // Validate the delegation is allowed
        let target_agent = self.delegate(&request.from_agent, &request.to_agent)?;

        // Return success with the target agent info
        Ok(super::DelegationResult::success(
            &target_agent.id,
            Some(format!(
                "Delegated task '{}' from {} to {}",
                request.task, request.from_agent, request.to_agent
            )),
        ))
    }

    /// Get all agents that a given agent can delegate to
    pub fn get_delegates(&self, agent_id: &str) -> Vec<&Agent> {
        let agent = match self.get(agent_id) {
            Some(a) => a,
            None => return Vec::new(),
        };

        agent.delegates_to.iter().filter_map(|id| self.get(id)).collect()
    }

    /// Find the best agent for a given capability
    pub fn find_by_capability(&self, capability: &str) -> Option<&Agent> {
        self.agents.values().find(|a| a.has_capability(capability))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new() {
        let registry = AgentRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_load_builtin() {
        let mut registry = AgentRegistry::new();
        registry.load_builtin();
        assert!(!registry.is_empty());
        assert!(registry.len() >= 15);
    }

    #[test]
    fn test_get_agent() {
        let mut registry = AgentRegistry::new();
        registry.load_builtin();

        let pm = registry.get("pm");
        assert!(pm.is_some());
        assert_eq!(pm.unwrap().name, "Product Manager");
    }

    #[test]
    fn test_list_by_capability() {
        let mut registry = AgentRegistry::new();
        registry.load_builtin();

        let architects = registry.list_by_capability("architecture");
        assert!(!architects.is_empty());
    }

    #[test]
    fn test_delegation() {
        let mut registry = AgentRegistry::new();
        registry.load_builtin();

        // BMad Master can delegate to others
        let result = registry.delegate("bmad-master", "architect");
        assert!(result.is_ok());
    }
}

#[test]
fn test_custom_agent_yaml_parsing() {
    use super::*;

    let yaml = r#"
- id: custom-agent
  name: Custom Agent
  title: Custom Title
  icon: "🔧"
  persona:
    role: Custom Role
    identity: Custom identity description
    communication_style: Custom style
    principles:
      - Principle 1
      - Principle 2
    traits:
      - Trait 1
      - Trait 2
  capabilities:
    - custom-capability
  workflows:
    - custom-workflow
  delegates_to:
    - developer
  builtin: false
"#;

    let agents: Vec<Agent> = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.id, "custom-agent");
    assert_eq!(agent.name, "Custom Agent");
    assert!(agent.has_capability("custom-capability"));
    assert!(agent.can_delegate_to("developer"));
}
