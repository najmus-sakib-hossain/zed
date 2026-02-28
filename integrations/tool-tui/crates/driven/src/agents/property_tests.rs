//! Property-based tests for the Agent system
//!
//! **Property 18: Custom Agent Definition Loading**
//! *For any* valid agent YAML definition, loading SHALL produce a functional Agent
//! with all specified properties.
//! **Validates: Requirements 6.12**

use super::{Agent, AgentPersona, AgentRegistry};
use proptest::prelude::*;

/// Generate a valid agent ID (kebab-case)
fn arb_agent_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{2,20}".prop_filter("no consecutive hyphens", |s| !s.contains("--"))
}

/// Generate a valid agent name
fn arb_agent_name() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z ]{2,30}".prop_filter("no double spaces", |s| !s.contains("  "))
}

/// Generate a valid icon (emoji or simple char)
fn arb_icon() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("🔧".to_string()),
        Just("📋".to_string()),
        Just("🏗️".to_string()),
        Just("💻".to_string()),
        Just("🎨".to_string()),
        Just("🧪".to_string()),
        Just("📊".to_string()),
        Just("📝".to_string()),
        Just("🏃".to_string()),
        Just("🔒".to_string()),
        Just("⚡".to_string()),
        Just("🚀".to_string()),
        Just("📈".to_string()),
        Just("👀".to_string()),
        Just("🎓".to_string()),
    ]
}

/// Generate a valid capability string
fn arb_capability() -> impl Strategy<Value = String> {
    "[a-z][a-z-]{2,20}"
}

/// Generate a valid workflow string
fn arb_workflow() -> impl Strategy<Value = String> {
    "[a-z][a-z-]{2,20}"
}

/// Generate a valid principle
fn arb_principle() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z ,]{5,50}"
}

/// Generate a valid trait
fn arb_trait() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z ]{3,30}"
}

/// Generate a valid AgentPersona
fn arb_persona() -> impl Strategy<Value = AgentPersona> {
    (
        arb_agent_name(),                             // role
        "[A-Z][a-zA-Z .,]{10,100}",                   // identity
        "[A-Z][a-zA-Z .,]{10,100}",                   // communication_style
        prop::collection::vec(arb_principle(), 1..5), // principles
        prop::collection::vec(arb_trait(), 1..5),     // traits
    )
        .prop_map(|(role, identity, style, principles, traits)| AgentPersona {
            role,
            identity,
            communication_style: style,
            principles,
            traits,
        })
}

/// Generate a valid Agent
fn arb_agent() -> impl Strategy<Value = Agent> {
    (
        arb_agent_id(),
        arb_agent_name(),
        arb_agent_name(), // title
        arb_icon(),
        arb_persona(),
        prop::collection::vec(arb_capability(), 1..5),
        prop::collection::vec(arb_workflow(), 0..3),
        prop::collection::vec(arb_agent_id(), 0..3),
    )
        .prop_map(|(id, name, title, icon, persona, capabilities, workflows, delegates)| {
            Agent {
                id,
                name,
                title,
                icon,
                persona,
                capabilities,
                workflows,
                delegates_to: delegates,
                builtin: false,
            }
        })
}

proptest! {
    /// Property 18: Custom Agent Definition Loading
    /// For any valid agent, serializing to YAML and deserializing back
    /// SHALL produce an equivalent agent with all specified properties.
    #[test]
    fn prop_agent_yaml_round_trip(agent in arb_agent()) {
        // Serialize to YAML
        let yaml = serde_yaml::to_string(&agent).expect("Failed to serialize agent");

        // Deserialize back
        let loaded: Agent = serde_yaml::from_str(&yaml).expect("Failed to deserialize agent");

        // Verify all properties are preserved
        prop_assert_eq!(&agent.id, &loaded.id, "ID mismatch");
        prop_assert_eq!(&agent.name, &loaded.name, "Name mismatch");
        prop_assert_eq!(&agent.title, &loaded.title, "Title mismatch");
        prop_assert_eq!(&agent.icon, &loaded.icon, "Icon mismatch");
        prop_assert_eq!(&agent.persona.role, &loaded.persona.role, "Role mismatch");
        prop_assert_eq!(&agent.persona.identity, &loaded.persona.identity, "Identity mismatch");
        prop_assert_eq!(&agent.persona.communication_style, &loaded.persona.communication_style, "Style mismatch");
        prop_assert_eq!(&agent.persona.principles, &loaded.persona.principles, "Principles mismatch");
        prop_assert_eq!(&agent.persona.traits, &loaded.persona.traits, "Traits mismatch");
        prop_assert_eq!(&agent.capabilities, &loaded.capabilities, "Capabilities mismatch");
        prop_assert_eq!(&agent.workflows, &loaded.workflows, "Workflows mismatch");
        prop_assert_eq!(&agent.delegates_to, &loaded.delegates_to, "Delegates mismatch");
    }

    /// Property: Agent capabilities are preserved after loading
    #[test]
    fn prop_agent_capabilities_preserved(agent in arb_agent()) {
        let yaml = serde_yaml::to_string(&agent).expect("Failed to serialize");
        let loaded: Agent = serde_yaml::from_str(&yaml).expect("Failed to deserialize");

        // All original capabilities should be present
        for cap in &agent.capabilities {
            prop_assert!(loaded.has_capability(cap), "Missing capability: {}", cap);
        }
    }

    /// Property: Agent delegation is preserved after loading
    #[test]
    fn prop_agent_delegation_preserved(agent in arb_agent()) {
        let yaml = serde_yaml::to_string(&agent).expect("Failed to serialize");
        let loaded: Agent = serde_yaml::from_str(&yaml).expect("Failed to deserialize");

        // All original delegates should be present
        for delegate in &agent.delegates_to {
            prop_assert!(loaded.can_delegate_to(delegate), "Missing delegate: {}", delegate);
        }
    }

    /// Property: Registry correctly stores and retrieves agents
    #[test]
    fn prop_registry_stores_agents(agents in prop::collection::vec(arb_agent(), 1..10)) {
        let mut registry = AgentRegistry::new();

        // Make IDs unique
        let mut unique_agents: Vec<Agent> = Vec::new();
        for (i, mut agent) in agents.into_iter().enumerate() {
            agent.id = format!("{}-{}", agent.id, i);
            unique_agents.push(agent);
        }

        // Register all agents
        for agent in &unique_agents {
            registry.register(agent.clone()).expect("Failed to register");
        }

        // Verify all agents can be retrieved
        for agent in &unique_agents {
            let retrieved = registry.get(&agent.id);
            prop_assert!(retrieved.is_some(), "Agent not found: {}", agent.id);
            prop_assert_eq!(&agent.name, &retrieved.unwrap().name);
        }

        // Verify count matches
        prop_assert_eq!(registry.len(), unique_agents.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_yaml_round_trip_simple() {
        let agent = Agent::new(
            "test-agent",
            "Test Agent",
            "Test Title",
            "🔧",
            AgentPersona::new("Test Role", "Test identity description", "Test communication style")
                .with_principle("Test principle".to_string())
                .with_trait("Test trait".to_string()),
        )
        .with_capability("test-cap".to_string())
        .with_workflow("test-workflow".to_string())
        .with_delegate("other-agent".to_string());

        let yaml = serde_yaml::to_string(&agent).unwrap();
        let loaded: Agent = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(agent.id, loaded.id);
        assert_eq!(agent.name, loaded.name);
        assert!(loaded.has_capability("test-cap"));
        assert!(loaded.can_delegate_to("other-agent"));
    }
}

// Property 19: Agent Delegation
// *For any* agent with delegates_to configuration, delegation requests
// SHALL be routed to the specified agents.
// **Validates: Requirements 6.13**

proptest! {
    /// Property 19: Agent Delegation
    /// For any agent with delegates_to configuration, delegation requests
    /// SHALL be routed to the specified agents.
    #[test]
    fn prop_agent_delegation_routing(
        from_agent in arb_agent(),
        to_agent in arb_agent(),
    ) {
        let mut registry = AgentRegistry::new();

        // Create unique IDs
        let from_id = format!("{}-from", from_agent.id);
        let to_id = format!("{}-to", to_agent.id);

        // Create from_agent that can delegate to to_agent
        let mut from = from_agent.clone();
        from.id = from_id.clone();
        from.delegates_to = vec![to_id.clone()];

        let mut to = to_agent.clone();
        to.id = to_id.clone();

        // Register both agents
        registry.register(from).expect("Failed to register from agent");
        registry.register(to).expect("Failed to register to agent");

        // Delegation should succeed
        let result = registry.delegate(&from_id, &to_id);
        prop_assert!(result.is_ok(), "Delegation should succeed");
        prop_assert_eq!(&result.unwrap().id, &to_id);
    }

    /// Property: Delegation fails when not configured
    #[test]
    fn prop_delegation_fails_when_not_configured(
        from_agent in arb_agent(),
        to_agent in arb_agent(),
    ) {
        let mut registry = AgentRegistry::new();

        // Create unique IDs
        let from_id = format!("{}-from", from_agent.id);
        let to_id = format!("{}-to", to_agent.id);

        // Create from_agent that CANNOT delegate to to_agent
        let mut from = from_agent.clone();
        from.id = from_id.clone();
        from.delegates_to = vec![]; // Empty - no delegation allowed

        let mut to = to_agent.clone();
        to.id = to_id.clone();

        // Register both agents
        registry.register(from).expect("Failed to register from agent");
        registry.register(to).expect("Failed to register to agent");

        // Delegation should fail
        let result = registry.delegate(&from_id, &to_id);
        prop_assert!(result.is_err(), "Delegation should fail when not configured");
    }

    /// Property: Get delegates returns correct agents
    #[test]
    fn prop_get_delegates_returns_correct_agents(
        main_agent in arb_agent(),
        delegate_agents in prop::collection::vec(arb_agent(), 1..5),
    ) {
        let mut registry = AgentRegistry::new();

        // Create unique IDs for delegates
        let delegate_ids: Vec<String> = delegate_agents
            .iter()
            .enumerate()
            .map(|(i, a)| format!("{}-delegate-{}", a.id, i))
            .collect();

        // Register delegate agents
        for (i, mut agent) in delegate_agents.into_iter().enumerate() {
            agent.id = delegate_ids[i].clone();
            registry.register(agent).expect("Failed to register delegate");
        }

        // Create main agent that delegates to all
        let mut main = main_agent.clone();
        main.id = format!("{}-main", main_agent.id);
        main.delegates_to = delegate_ids.clone();

        registry.register(main.clone()).expect("Failed to register main agent");

        // Get delegates should return all configured delegates
        let delegates = registry.get_delegates(&main.id);
        prop_assert_eq!(delegates.len(), delegate_ids.len());

        for delegate in delegates {
            prop_assert!(delegate_ids.contains(&delegate.id));
        }
    }
}

#[cfg(test)]
mod delegation_tests {
    use super::*;

    #[test]
    fn test_bmad_master_delegation() {
        let mut registry = AgentRegistry::new();
        registry.load_builtin();

        // BMad Master should be able to delegate to all other agents
        let bmad = registry.get("bmad-master").unwrap();

        // Test delegation to architect
        let result = registry.delegate("bmad-master", "architect");
        assert!(result.is_ok());

        // Test delegation to developer
        let result = registry.delegate("bmad-master", "developer");
        assert!(result.is_ok());

        // Test delegation to security
        let result = registry.delegate("bmad-master", "security");
        assert!(result.is_ok());
    }

    #[test]
    fn test_delegation_request_processing() {
        let mut registry = AgentRegistry::new();
        registry.load_builtin();

        let request = super::super::DelegationRequest::new(
            "bmad-master",
            "architect",
            "Design the system architecture",
        )
        .with_context("project", "my-project")
        .with_context("priority", "high");

        let result = registry.process_delegation(&request);
        assert!(result.is_ok());

        let delegation_result = result.unwrap();
        assert!(delegation_result.success);
        assert_eq!(delegation_result.handled_by, "architect");
    }
}
