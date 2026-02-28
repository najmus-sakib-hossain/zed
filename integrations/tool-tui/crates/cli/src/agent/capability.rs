//! Capability Analysis and Gap Detection
//!
//! This module provides capability tracking, gap detection, and skill
//! requirement analysis for the AI agent.
//!
//! # Capabilities
//!
//! Capabilities represent skills the agent can perform:
//! - Built-in: Core functionality (file operations, shell, etc.)
//! - Learned: Dynamically acquired via self-update
//! - Plugin: Provided by WASM/native plugins
//!
//! # Gap Detection
//!
//! The analyzer examines user requests to detect missing capabilities
//! and generates skill requirements that can be acquired via self-update.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::agent::capability::{CapabilityAnalyzer, Capability};
//!
//! let mut analyzer = CapabilityAnalyzer::new();
//!
//! // Register a capability
//! analyzer.register_capability(Capability {
//!     name: "weather".to_string(),
//!     description: "Get weather information".to_string(),
//!     config: None,
//!     handler: None,
//! });
//!
//! // Analyze a request for gaps
//! let gaps = analyzer.analyze_request("What's the weather in Tokyo?");
//! // Returns empty because "weather" capability is registered
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A capability the agent can perform
#[derive(Serialize, Deserialize)]
pub struct Capability {
    /// Unique name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Configuration (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
    /// Handler function (runtime only)
    #[serde(skip)]
    pub handler: Option<CapabilityHandler>,
}

/// Handler function type for capabilities
pub type CapabilityHandler = Box<dyn Fn(&str) -> String + Send + Sync>;

impl fmt::Debug for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Capability")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("config", &self.config)
            .field("handler", &self.handler.as_ref().map(|_| "<fn>"))
            .finish()
    }
}

impl Clone for Capability {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            config: self.config.clone(),
            handler: None, // Cannot clone function pointer, reset to None
        }
    }
}

impl Capability {
    /// Create a new capability
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            config: None,
            handler: None,
        }
    }

    /// Create with configuration
    pub fn with_config(name: &str, description: &str, config: serde_json::Value) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            config: Some(config),
            handler: None,
        }
    }
}

/// A gap in capabilities detected from a request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGap {
    /// What capability is missing
    pub capability: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Why this capability is needed
    pub reason: String,
    /// Suggested skill requirement
    pub requirement: Option<SkillRequirement>,
}

/// Requirements to acquire a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRequirement {
    /// Skill name
    pub name: String,
    /// Type of skill
    pub skill_type: SkillType,
    /// Required configuration keys
    pub required_config: Vec<String>,
    /// Optional configuration keys
    pub optional_config: Vec<String>,
    /// Dependencies on other capabilities
    pub dependencies: Vec<String>,
    /// Example configuration
    pub example_config: Option<serde_json::Value>,
}

/// Types of skills
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillType {
    /// Configuration-based (just needs .sr file)
    Config,
    /// Requires API integration
    Api,
    /// Requires WASM plugin
    Plugin,
    /// Requires native binary
    Native,
    /// Requires external service
    External,
}

/// Capability analyzer for gap detection
pub struct CapabilityAnalyzer {
    /// Registered capabilities
    capabilities: HashMap<String, Capability>,
    /// Capability patterns for detection
    patterns: Vec<CapabilityPattern>,
}

/// Pattern for detecting capability requirements
struct CapabilityPattern {
    /// Capability name
    capability: String,
    /// Keywords that suggest this capability
    keywords: Vec<String>,
    /// Weight/importance
    weight: f32,
}

impl CapabilityAnalyzer {
    /// Create a new analyzer with built-in patterns
    pub fn new() -> Self {
        let mut analyzer = Self {
            capabilities: HashMap::new(),
            patterns: Vec::new(),
        };

        // Register built-in capabilities
        analyzer.register_builtin_capabilities();
        analyzer.register_detection_patterns();

        analyzer
    }

    /// Register built-in capabilities
    fn register_builtin_capabilities(&mut self) {
        let builtins = vec![
            ("file_read", "Read files from the filesystem"),
            ("file_write", "Write files to the filesystem"),
            ("shell_execute", "Execute shell commands"),
            ("http_request", "Make HTTP requests"),
            ("code_analysis", "Analyze code structure"),
            ("code_generation", "Generate code"),
            ("memory_search", "Search conversation history"),
            ("config_management", "Manage configuration files"),
        ];

        for (name, desc) in builtins {
            self.capabilities.insert(name.to_string(), Capability::new(name, desc));
        }
    }

    /// Register detection patterns
    fn register_detection_patterns(&mut self) {
        let patterns = vec![
            // Weather
            (
                "weather",
                vec![
                    "weather",
                    "temperature",
                    "forecast",
                    "rain",
                    "sunny",
                    "cloudy",
                ],
                0.8,
            ),
            // Email
            ("email", vec!["email", "mail", "inbox", "send email", "gmail", "outlook"], 0.8),
            // Calendar
            ("calendar", vec!["calendar", "schedule", "meeting", "appointment", "event"], 0.8),
            // Music
            (
                "music",
                vec![
                    "play music",
                    "spotify",
                    "song",
                    "playlist",
                    "album",
                    "artist",
                ],
                0.8,
            ),
            // Smart home
            (
                "smart_home",
                vec![
                    "lights",
                    "thermostat",
                    "smart home",
                    "home assistant",
                    "hue",
                ],
                0.8,
            ),
            // Messaging
            (
                "messaging",
                vec!["whatsapp", "telegram", "slack", "discord", "send message"],
                0.7,
            ),
            // Browser
            ("browser", vec!["browse", "open website", "scrape", "click", "webpage"], 0.7),
            // Voice
            ("voice", vec!["speak", "say", "voice", "tts", "read aloud"], 0.7),
            // Image
            ("image", vec!["screenshot", "photo", "image", "picture", "camera"], 0.7),
            // Git
            ("git", vec!["git", "commit", "push", "pull", "branch", "merge"], 0.6),
            // Database
            ("database", vec!["database", "sql", "query", "table", "record"], 0.7),
            // Notes
            ("notes", vec!["note", "notion", "obsidian", "bear", "take note"], 0.7),
            // Tasks
            ("tasks", vec!["todo", "task", "reminder", "things", "trello"], 0.7),
        ];

        for (capability, keywords, weight) in patterns {
            self.patterns.push(CapabilityPattern {
                capability: capability.to_string(),
                keywords: keywords.into_iter().map(String::from).collect(),
                weight,
            });
        }
    }

    /// Register a capability
    pub fn register_capability(&mut self, capability: Capability) {
        self.capabilities.insert(capability.name.clone(), capability);
    }

    /// Unregister a capability
    pub fn unregister_capability(&mut self, name: &str) {
        self.capabilities.remove(name);
    }

    /// Check if a capability exists
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.contains_key(name)
    }

    /// Get a capability
    pub fn get_capability(&self, name: &str) -> Option<&Capability> {
        self.capabilities.get(name)
    }

    /// List all capabilities
    pub fn list(&self) -> Vec<String> {
        self.capabilities.keys().cloned().collect()
    }

    /// Analyze a request for capability gaps
    pub fn analyze_request(&self, request: &str) -> Vec<CapabilityGap> {
        let request_lower = request.to_lowercase();
        let mut gaps = Vec::new();

        for pattern in &self.patterns {
            // Check if we already have this capability
            if self.capabilities.contains_key(&pattern.capability) {
                continue;
            }

            // Count keyword matches
            let matches: usize =
                pattern.keywords.iter().filter(|kw| request_lower.contains(kw.as_str())).count();

            if matches > 0 {
                let confidence = (matches as f32 / pattern.keywords.len() as f32) * pattern.weight;

                if confidence >= 0.3 {
                    gaps.push(CapabilityGap {
                        capability: pattern.capability.clone(),
                        confidence,
                        reason: format!("Request mentions {} related term(s)", matches),
                        requirement: self.generate_requirement(&pattern.capability),
                    });
                }
            }
        }

        // Sort by confidence
        gaps.sort_by(|a, b| {
            b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal)
        });

        gaps
    }

    /// Generate skill requirement for a capability
    fn generate_requirement(&self, capability: &str) -> Option<SkillRequirement> {
        let requirements: HashMap<&str, SkillRequirement> = [
            (
                "weather",
                SkillRequirement {
                    name: "weather".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec![],
                    optional_config: vec!["api_key".to_string(), "location".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "provider": "wttr.in",
                        "default_location": "auto"
                    })),
                },
            ),
            (
                "email",
                SkillRequirement {
                    name: "email".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec!["provider".to_string()],
                    optional_config: vec!["credentials".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "provider": "gmail",
                        "oauth": true
                    })),
                },
            ),
            (
                "messaging",
                SkillRequirement {
                    name: "messaging".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec!["channel".to_string()],
                    optional_config: vec!["webhook_url".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "channels": ["whatsapp", "telegram", "discord"]
                    })),
                },
            ),
            (
                "browser",
                SkillRequirement {
                    name: "browser".to_string(),
                    skill_type: SkillType::Native,
                    required_config: vec![],
                    optional_config: vec!["headless".to_string(), "timeout".to_string()],
                    dependencies: vec![],
                    example_config: Some(serde_json::json!({
                        "headless": true,
                        "timeout_ms": 30000
                    })),
                },
            ),
            (
                "voice",
                SkillRequirement {
                    name: "voice".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec!["provider".to_string()],
                    optional_config: vec!["voice_id".to_string(), "speed".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "provider": "elevenlabs",
                        "voice": "default"
                    })),
                },
            ),
            (
                "smart_home",
                SkillRequirement {
                    name: "smart_home".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec!["platform".to_string()],
                    optional_config: vec!["api_url".to_string(), "token".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "platform": "home_assistant",
                        "url": "http://homeassistant.local:8123"
                    })),
                },
            ),
            (
                "notes",
                SkillRequirement {
                    name: "notes".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec!["provider".to_string()],
                    optional_config: vec!["api_key".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "provider": "notion",
                        "database_id": ""
                    })),
                },
            ),
            (
                "tasks",
                SkillRequirement {
                    name: "tasks".to_string(),
                    skill_type: SkillType::Api,
                    required_config: vec!["provider".to_string()],
                    optional_config: vec!["project".to_string()],
                    dependencies: vec!["http_request".to_string()],
                    example_config: Some(serde_json::json!({
                        "provider": "github",
                        "repo": ""
                    })),
                },
            ),
        ]
        .into_iter()
        .collect();

        requirements.get(capability).cloned()
    }

    /// Get statistics about capabilities
    pub fn stats(&self) -> CapabilityStats {
        let total = self.capabilities.len();
        let with_handlers = self.capabilities.values().filter(|c| c.handler.is_some()).count();
        let with_config = self.capabilities.values().filter(|c| c.config.is_some()).count();

        CapabilityStats {
            total,
            with_handlers,
            with_config,
        }
    }
}

impl Default for CapabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability statistics
#[derive(Debug, Clone)]
pub struct CapabilityStats {
    /// Total capabilities
    pub total: usize,
    /// Capabilities with handlers
    pub with_handlers: usize,
    /// Capabilities with config
    pub with_config: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = CapabilityAnalyzer::new();
        assert!(!analyzer.list().is_empty());
    }

    #[test]
    fn test_builtin_capabilities() {
        let analyzer = CapabilityAnalyzer::new();
        assert!(analyzer.has_capability("file_read"));
        assert!(analyzer.has_capability("shell_execute"));
    }

    #[test]
    fn test_gap_detection() {
        let analyzer = CapabilityAnalyzer::new();

        let gaps = analyzer.analyze_request("What's the weather like in Tokyo?");
        assert!(!gaps.is_empty());
        assert_eq!(gaps[0].capability, "weather");
    }

    #[test]
    fn test_no_gap_when_capable() {
        let mut analyzer = CapabilityAnalyzer::new();
        analyzer.register_capability(Capability::new("weather", "Get weather info"));

        let gaps = analyzer.analyze_request("What's the weather?");
        assert!(gaps.iter().all(|g| g.capability != "weather"));
    }

    #[test]
    fn test_register_unregister() {
        let mut analyzer = CapabilityAnalyzer::new();

        analyzer.register_capability(Capability::new("test", "Test capability"));
        assert!(analyzer.has_capability("test"));

        analyzer.unregister_capability("test");
        assert!(!analyzer.has_capability("test"));
    }
}
