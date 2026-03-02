//! AI Workflow Router
//!
//! This module provides fast in-process workflow routing for the AI agent.
//! It maps AI intents to workflow templates and handles parameter injection.
//!
//! The router operates at nanosecond speed since it's pure Rust - no IPC or network calls.

use crate::protocol::{WorkflowBuilder, WorkflowDefinition, WorkflowNode};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing decision from the AI workflow router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Action to take: "execute", "reject", "defer"
    pub action: String,

    /// Workflow to execute (if action is "execute")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowDefinition>,

    /// Input data for the workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_data: Option<serde_json::Value>,

    /// Whether to wait for the workflow to complete
    pub blocking: bool,

    /// Execution priority
    pub priority: String,

    /// Reason for rejection (if action is "reject")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A workflow template that can be instantiated with parameters
#[derive(Debug, Clone)]
pub struct WorkflowTemplate {
    /// Template name
    pub name: String,

    /// Function that builds the workflow given parameters
    pub builder: fn(serde_json::Value) -> WorkflowDefinition,

    /// Whether this workflow should block by default
    pub blocking: bool,

    /// Parameter schema description (for documentation)
    pub parameter_schema: serde_json::Value,
}

/// The AI Workflow Router
///
/// Routes AI intents to n8n workflows at nanosecond speed.
/// This runs entirely in-process with no IPC overhead.
pub struct AiWorkflowRouter {
    /// Registry of workflow templates by intent name
    templates: HashMap<String, WorkflowTemplate>,

    /// Intents that should be handled natively in Rust (not delegated to n8n)
    rust_native_intents: Vec<String>,
}

impl AiWorkflowRouter {
    /// Create a new AI workflow router with default templates
    pub fn new() -> Result<Self> {
        let mut router = Self {
            templates: HashMap::new(),
            rust_native_intents: vec![
                "classify_text".to_string(),
                "extract_entities".to_string(),
                "compute_embedding".to_string(),
                "cache_lookup".to_string(),
            ],
        };

        // Register default workflow templates
        router.register_default_templates();

        Ok(router)
    }

    /// Register the built-in workflow templates
    fn register_default_templates(&mut self) {
        // Slack notification workflow
        self.templates.insert(
            "send_slack".to_string(),
            WorkflowTemplate {
                name: "AI Slack Notifier".to_string(),
                builder: |params| {
                    let channel = params
                        .get("channel")
                        .and_then(|c| c.as_str())
                        .unwrap_or("#general");
                    let message = params
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("");

                    WorkflowBuilder::new("AI Slack Notifier")
                        .add_trigger()
                        .add_slack_message("Send Slack", channel, message)
                        .build()
                },
                blocking: false,
                parameter_schema: serde_json::json!({
                    "channel": { "type": "string", "description": "Slack channel" },
                    "message": { "type": "string", "description": "Message text" }
                }),
            },
        );

        // HTTP request workflow
        self.templates.insert(
            "http_request".to_string(),
            WorkflowTemplate {
                name: "AI HTTP Request".to_string(),
                builder: |params| {
                    let url = params
                        .get("url")
                        .and_then(|u| u.as_str())
                        .unwrap_or("https://example.com");
                    let method = params
                        .get("method")
                        .and_then(|m| m.as_str())
                        .unwrap_or("GET");

                    WorkflowBuilder::new("AI HTTP Request")
                        .add_trigger()
                        .add_http_request("HTTP Request", url, method)
                        .build()
                },
                blocking: true,
                parameter_schema: serde_json::json!({
                    "url": { "type": "string", "description": "Request URL" },
                    "method": { "type": "string", "description": "HTTP method" }
                }),
            },
        );

        // Email notification workflow
        self.templates.insert(
            "send_email".to_string(),
            WorkflowTemplate {
                name: "AI Email Sender".to_string(),
                builder: |params| {
                    let to = params
                        .get("to")
                        .and_then(|t| t.as_str())
                        .unwrap_or("");
                    let subject = params
                        .get("subject")
                        .and_then(|s| s.as_str())
                        .unwrap_or("AI Notification");
                    let body = params
                        .get("body")
                        .and_then(|b| b.as_str())
                        .unwrap_or("");

                    let mut nodes = vec![
                        WorkflowNode::new(
                            "trigger",
                            "Start",
                            "n8n-nodes-base.manualTrigger",
                            [240, 300],
                        ),
                        WorkflowNode::new(
                            "email",
                            "Send Email",
                            "n8n-nodes-base.emailSend",
                            [460, 300],
                        )
                        .with_parameters(serde_json::json!({
                            "toEmail": to,
                            "subject": subject,
                            "text": body
                        })),
                    ];

                    WorkflowDefinition {
                        id: None,
                        name: "AI Email Sender".to_string(),
                        nodes,
                        connections: serde_json::json!({
                            "Start": {
                                "main": [[{ "node": "Send Email", "type": "main", "index": 0 }]]
                            }
                        }),
                        settings: None,
                    }
                },
                blocking: false,
                parameter_schema: serde_json::json!({
                    "to": { "type": "string", "description": "Recipient email" },
                    "subject": { "type": "string", "description": "Email subject" },
                    "body": { "type": "string", "description": "Email body" }
                }),
            },
        );

        // Code execution workflow
        self.templates.insert(
            "run_code".to_string(),
            WorkflowTemplate {
                name: "AI Code Runner".to_string(),
                builder: |params| {
                    let code = params
                        .get("code")
                        .and_then(|c| c.as_str())
                        .unwrap_or("return items;");
                    let language = params
                        .get("language")
                        .and_then(|l| l.as_str())
                        .unwrap_or("javaScript");

                    WorkflowBuilder::new("AI Code Runner")
                        .add_trigger()
                        .add_code("Execute Code", code, language)
                        .build()
                },
                blocking: true,
                parameter_schema: serde_json::json!({
                    "code": { "type": "string", "description": "Code to execute" },
                    "language": { "type": "string", "description": "javaScript or python" }
                }),
            },
        );

        // Database query workflow
        self.templates.insert(
            "query_database".to_string(),
            WorkflowTemplate {
                name: "AI Database Query".to_string(),
                builder: |params| {
                    let query = params
                        .get("query")
                        .and_then(|q| q.as_str())
                        .unwrap_or("SELECT 1");

                    let nodes = vec![
                        WorkflowNode::new(
                            "trigger",
                            "Start",
                            "n8n-nodes-base.manualTrigger",
                            [240, 300],
                        ),
                        WorkflowNode::new(
                            "postgres",
                            "Query Database",
                            "n8n-nodes-base.postgres",
                            [460, 300],
                        )
                        .with_parameters(serde_json::json!({
                            "operation": "executeQuery",
                            "query": query
                        })),
                    ];

                    WorkflowDefinition {
                        id: None,
                        name: "AI Database Query".to_string(),
                        nodes,
                        connections: serde_json::json!({
                            "Start": {
                                "main": [[{ "node": "Query Database", "type": "main", "index": 0 }]]
                            }
                        }),
                        settings: None,
                    }
                },
                blocking: true,
                parameter_schema: serde_json::json!({
                    "query": { "type": "string", "description": "SQL query" }
                }),
            },
        );
    }

    /// Register a custom workflow template
    pub fn register_template(&mut self, intent: &str, template: WorkflowTemplate) {
        self.templates.insert(intent.to_string(), template);
    }

    /// Add an intent to the Rust-native list (not delegated to n8n)
    pub fn add_rust_native_intent(&mut self, intent: &str) {
        self.rust_native_intents.push(intent.to_string());
    }

    /// Check if an intent should be delegated to n8n
    pub fn should_delegate_to_n8n(&self, intent: &str) -> bool {
        !self.rust_native_intents.contains(&intent.to_string())
    }

    /// Route an AI decision to a workflow
    ///
    /// This is the main entry point - takes an AI decision and returns a routing decision.
    /// Runs at nanosecond speed since it's pure Rust.
    pub fn route_ai_decision(&self, decision: serde_json::Value) -> Result<serde_json::Value> {
        let intent = decision
            .get("intent")
            .and_then(|i| i.as_str())
            .unwrap_or("");

        let parameters = decision
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        let context = decision.get("context").cloned();

        // Check if this is a Rust-native intent
        if !self.should_delegate_to_n8n(intent) {
            return Ok(serde_json::json!({
                "action": "native",
                "intent": intent,
                "reason": "Handled natively in Rust"
            }));
        }

        // Look up the workflow template
        let template = match self.templates.get(intent) {
            Some(t) => t,
            None => {
                return Ok(serde_json::json!({
                    "action": "reject",
                    "reason": format!("No workflow template for intent: {}", intent)
                }));
            }
        };

        // Build the workflow with the provided parameters
        let workflow = (template.builder)(parameters.clone());

        // Determine execution priority
        let priority = context
            .as_ref()
            .and_then(|c| c.get("priority"))
            .and_then(|p| p.as_str())
            .unwrap_or("normal");

        Ok(serde_json::json!({
            "action": "execute",
            "workflow": workflow,
            "input_data": parameters,
            "blocking": template.blocking,
            "priority": priority
        }))
    }

    /// Get all registered workflow intents
    pub fn list_intents(&self) -> Vec<&str> {
        self.templates.keys().map(|k| k.as_str()).collect()
    }

    /// Get the parameter schema for an intent
    pub fn get_parameter_schema(&self, intent: &str) -> Option<&serde_json::Value> {
        self.templates.get(intent).map(|t| &t.parameter_schema)
    }
}

impl Default for AiWorkflowRouter {
    fn default() -> Self {
        Self::new().expect("Failed to create default router")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_slack_intent() {
        let router = AiWorkflowRouter::new().unwrap();

        let decision = serde_json::json!({
            "intent": "send_slack",
            "parameters": {
                "channel": "#engineering",
                "message": "Hello, world!"
            },
            "context": { "priority": "high" }
        });

        let result = router.route_ai_decision(decision).unwrap();

        assert_eq!(result.get("action").unwrap().as_str().unwrap(), "execute");
        assert!(result.get("workflow").is_some());
        assert_eq!(result.get("priority").unwrap().as_str().unwrap(), "high");
    }

    #[test]
    fn test_route_unknown_intent() {
        let router = AiWorkflowRouter::new().unwrap();

        let decision = serde_json::json!({
            "intent": "unknown_action",
            "parameters": {}
        });

        let result = router.route_ai_decision(decision).unwrap();

        assert_eq!(result.get("action").unwrap().as_str().unwrap(), "reject");
        assert!(result.get("reason").is_some());
    }

    #[test]
    fn test_rust_native_intent() {
        let router = AiWorkflowRouter::new().unwrap();

        let decision = serde_json::json!({
            "intent": "classify_text",
            "parameters": { "text": "Hello world" }
        });

        let result = router.route_ai_decision(decision).unwrap();

        assert_eq!(result.get("action").unwrap().as_str().unwrap(), "native");
    }

    #[test]
    fn test_list_intents() {
        let router = AiWorkflowRouter::new().unwrap();
        let intents = router.list_intents();

        assert!(intents.contains(&"send_slack"));
        assert!(intents.contains(&"http_request"));
        assert!(intents.contains(&"send_email"));
    }
}
