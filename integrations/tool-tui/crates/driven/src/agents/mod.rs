//! Expanded Agent Library
//!
//! Provides 15+ specialized AI agent personas for different development tasks,
//! matching and exceeding BMAD-METHOD capabilities.

mod builtin;
mod registry;

#[cfg(test)]
mod property_tests;

pub use builtin::*;
pub use registry::AgentRegistry;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A specialized AI agent with defined persona and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier for the agent
    pub id: String,
    /// Display name
    pub name: String,
    /// Professional title
    pub title: String,
    /// Icon/emoji for the agent
    pub icon: String,
    /// Agent's persona definition
    pub persona: AgentPersona,
    /// List of capabilities this agent has
    pub capabilities: Vec<String>,
    /// Workflows this agent can execute
    pub workflows: Vec<String>,
    /// Other agents this agent can delegate to
    pub delegates_to: Vec<String>,
    /// Whether this is a built-in agent
    #[serde(default)]
    pub builtin: bool,
}

/// Defines an agent's personality and behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPersona {
    /// The agent's role description
    pub role: String,
    /// Identity statement - who the agent is
    pub identity: String,
    /// Communication style
    pub communication_style: String,
    /// Core principles the agent follows
    pub principles: Vec<String>,
    /// Personality traits
    pub traits: Vec<String>,
}

impl Agent {
    /// Create a new agent with the given parameters
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        title: impl Into<String>,
        icon: impl Into<String>,
        persona: AgentPersona,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            title: title.into(),
            icon: icon.into(),
            persona,
            capabilities: Vec::new(),
            workflows: Vec::new(),
            delegates_to: Vec::new(),
            builtin: false,
        }
    }

    /// Add a capability to this agent
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Add multiple capabilities
    pub fn with_capabilities(mut self, capabilities: Vec<String>) -> Self {
        self.capabilities.extend(capabilities);
        self
    }

    /// Add a workflow this agent can execute
    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflows.push(workflow.into());
        self
    }

    /// Add multiple workflows
    pub fn with_workflows(mut self, workflows: Vec<String>) -> Self {
        self.workflows.extend(workflows);
        self
    }

    /// Add an agent this agent can delegate to
    pub fn with_delegate(mut self, delegate: impl Into<String>) -> Self {
        self.delegates_to.push(delegate.into());
        self
    }

    /// Mark as built-in agent
    pub fn as_builtin(mut self) -> Self {
        self.builtin = true;
        self
    }

    /// Check if this agent has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Check if this agent can delegate to another agent
    pub fn can_delegate_to(&self, agent_id: &str) -> bool {
        self.delegates_to.iter().any(|d| d == agent_id)
    }
}

impl AgentPersona {
    /// Create a new agent persona
    pub fn new(
        role: impl Into<String>,
        identity: impl Into<String>,
        communication_style: impl Into<String>,
    ) -> Self {
        Self {
            role: role.into(),
            identity: identity.into(),
            communication_style: communication_style.into(),
            principles: Vec::new(),
            traits: Vec::new(),
        }
    }

    /// Add a principle
    pub fn with_principle(mut self, principle: impl Into<String>) -> Self {
        self.principles.push(principle.into());
        self
    }

    /// Add multiple principles
    pub fn with_principles(mut self, principles: Vec<String>) -> Self {
        self.principles.extend(principles);
        self
    }

    /// Add a trait
    pub fn with_trait(mut self, trait_: impl Into<String>) -> Self {
        self.traits.push(trait_.into());
        self
    }

    /// Add multiple traits
    pub fn with_traits(mut self, traits: Vec<String>) -> Self {
        self.traits.extend(traits);
        self
    }
}

/// A request to delegate work from one agent to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    /// The agent making the delegation
    pub from_agent: String,
    /// The agent receiving the delegation
    pub to_agent: String,
    /// The task or context being delegated
    pub task: String,
    /// Additional context for the delegation
    pub context: HashMap<String, String>,
}

impl DelegationRequest {
    /// Create a new delegation request
    pub fn new(
        from_agent: impl Into<String>,
        to_agent: impl Into<String>,
        task: impl Into<String>,
    ) -> Self {
        Self {
            from_agent: from_agent.into(),
            to_agent: to_agent.into(),
            task: task.into(),
            context: HashMap::new(),
        }
    }

    /// Add context to the delegation
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Result of a delegation operation
#[derive(Debug, Clone)]
pub struct DelegationResult {
    /// Whether the delegation was successful
    pub success: bool,
    /// The agent that handled the delegation
    pub handled_by: String,
    /// Any output from the delegation
    pub output: Option<String>,
    /// Error message if delegation failed
    pub error: Option<String>,
}

impl DelegationResult {
    /// Create a successful delegation result
    pub fn success(handled_by: impl Into<String>, output: Option<String>) -> Self {
        Self {
            success: true,
            handled_by: handled_by.into(),
            output,
            error: None,
        }
    }

    /// Create a failed delegation result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            handled_by: String::new(),
            output: None,
            error: Some(error.into()),
        }
    }
}
