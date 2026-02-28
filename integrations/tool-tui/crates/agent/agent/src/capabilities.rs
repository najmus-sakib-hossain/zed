//! # Capabilities System
//!
//! Define and manage what the agent can do.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A capability that the agent has
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Unique name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Required integrations
    pub required_integrations: Vec<String>,

    /// Whether the capability is enabled
    pub enabled: bool,
}

impl Capability {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            required_integrations: vec![],
            enabled: true,
        }
    }

    pub fn with_integration(mut self, integration: &str) -> Self {
        self.required_integrations.push(integration.to_string());
        self
    }
}

/// Registry of all capabilities
pub struct CapabilityRegistry {
    capabilities: HashMap<String, Capability>,
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            capabilities: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        // Messaging capabilities
        self.register(
            Capability::new("send_message", "Send messages via messaging platforms")
                .with_integration("messaging"),
        );
        self.register(
            Capability::new("receive_message", "Receive messages from platforms")
                .with_integration("messaging"),
        );

        // Browser capabilities
        self.register(
            Capability::new("browse_web", "Navigate and interact with web pages")
                .with_integration("browser"),
        );
        self.register(
            Capability::new("screenshot", "Take screenshots of web pages")
                .with_integration("browser"),
        );

        // File system capabilities
        self.register(Capability::new("read_file", "Read files from the system"));
        self.register(Capability::new("write_file", "Write files to the system"));

        // Command execution
        self.register(Capability::new("run_command", "Execute shell commands"));

        // Code generation
        self.register(Capability::new(
            "generate_code",
            "Generate code in any language",
        ));
        self.register(Capability::new(
            "compile_wasm",
            "Compile code to WebAssembly",
        ));

        // Integration management
        self.register(Capability::new(
            "create_integration",
            "Create new integrations dynamically",
        ));
        self.register(
            Capability::new("create_pr", "Create pull requests for new integrations")
                .with_integration("github"),
        );
    }

    pub fn register(&mut self, capability: Capability) {
        self.capabilities
            .insert(capability.name.clone(), capability);
    }

    pub fn get(&self, name: &str) -> Option<&Capability> {
        self.capabilities.get(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    pub fn list(&self) -> Vec<&Capability> {
        self.capabilities.values().collect()
    }

    pub fn list_enabled(&self) -> Vec<&Capability> {
        self.capabilities.values().filter(|c| c.enabled).collect()
    }

    /// List capabilities in DX format
    pub fn to_dx(&self) -> String {
        let caps: Vec<String> = self
            .capabilities
            .values()
            .map(|c| format!("{}:{}", c.name, if c.enabled { "on" } else { "off" }))
            .collect();

        format!("capabilities:{}[{}]", caps.len(), caps.join(" "))
    }
}
