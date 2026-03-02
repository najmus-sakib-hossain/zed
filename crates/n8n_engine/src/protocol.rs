//! Workflow-as-JSON Protocol
//!
//! This module defines the data protocol between the Rust AI and the n8n engine.
//! All communication uses these strongly-typed structures for safety and clarity.

use serde::{Deserialize, Serialize};

/// A complete workflow definition that can be executed by n8n
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    /// Unique workflow identifier (optional for dynamic workflows)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Human-readable workflow name
    pub name: String,

    /// The nodes that make up this workflow
    pub nodes: Vec<WorkflowNode>,

    /// Connections between nodes (node outputs → node inputs)
    pub connections: serde_json::Value,

    /// Workflow-level settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<serde_json::Value>,
}

/// A single node in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    /// Unique node identifier within the workflow
    pub id: String,

    /// Human-readable node name
    pub name: String,

    /// n8n node type (e.g., "n8n-nodes-base.httpRequest")
    #[serde(rename = "type")]
    pub node_type: String,

    /// Node type version
    #[serde(rename = "typeVersion")]
    pub type_version: u32,

    /// Position in the workflow canvas [x, y]
    pub position: [i32; 2],

    /// Node-specific parameters
    pub parameters: serde_json::Value,

    /// Optional credentials reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credentials: Option<serde_json::Value>,
}

impl WorkflowNode {
    /// Create a new workflow node
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        node_type: impl Into<String>,
        position: [i32; 2],
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            node_type: node_type.into(),
            type_version: 1,
            position,
            parameters: serde_json::json!({}),
            credentials: None,
        }
    }

    /// Set the node parameters
    pub fn with_parameters(mut self, parameters: serde_json::Value) -> Self {
        self.parameters = parameters;
        self
    }

    /// Set the node credentials
    pub fn with_credentials(mut self, credentials: serde_json::Value) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Set the type version
    pub fn with_type_version(mut self, version: u32) -> Self {
        self.type_version = version;
        self
    }
}

/// Result of a workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Unique execution identifier
    pub execution_id: String,

    /// Execution status: "success", "error", "waiting", "running"
    pub status: String,

    /// Result data from the workflow execution
    pub data: serde_json::Value,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl WorkflowResult {
    /// Check if the execution was successful
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }

    /// Check if the execution failed
    pub fn is_error(&self) -> bool {
        self.status == "error"
    }
}

/// IPC message format for communication with the n8n sidecar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcMessage {
    /// Unique message identifier for request/response matching
    pub id: String,

    /// Message type: "execute", "execute_async", "get_status", "stop", "shutdown"
    #[serde(rename = "type")]
    pub msg_type: String,

    /// Message payload (varies by type)
    pub payload: serde_json::Value,
}

/// IPC response from the n8n sidecar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    /// Request ID this response corresponds to
    pub id: String,

    /// Response type: "result", "error", "ack", "async_result"
    #[serde(rename = "type")]
    pub msg_type: String,

    /// Response payload
    pub payload: serde_json::Value,
}

/// AI Agent's command to the workflow engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWorkflowCommand {
    /// Unique command ID for tracking
    pub command_id: String,

    /// The action type
    pub action: WorkflowAction,

    /// Execution preferences
    pub execution: ExecutionPreference,
}

/// Types of workflow actions the AI can request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowAction {
    /// Execute an existing workflow by ID
    ExecuteExisting {
        workflow_id: String,
        input_data: serde_json::Value,
    },

    /// Build and execute a dynamic workflow on-the-fly
    ExecuteDynamic { workflow: DynamicWorkflow },

    /// Chain multiple workflows together
    ExecuteChain { workflows: Vec<ChainedWorkflow> },

    /// Execute a workflow and pipe results to the AI
    ExecuteAndAnalyze {
        workflow_id: String,
        input_data: serde_json::Value,
        analysis_prompt: String,
    },
}

/// A dynamically constructed workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicWorkflow {
    /// Workflow name
    pub name: String,

    /// Steps to execute
    pub steps: Vec<WorkflowStep>,
}

/// High-level workflow step types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowStep {
    /// Make an HTTP request
    HttpRequest {
        url: String,
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<serde_json::Value>,
    },

    /// Send a Slack message
    SendSlack { channel: String, message: String },

    /// Send an email
    SendEmail {
        to: String,
        subject: String,
        body: String,
    },

    /// Execute a database query
    DatabaseQuery { connection: String, query: String },

    /// Process through an AI model
    AiProcess {
        model: String,
        prompt: String,
        input_field: String,
    },

    /// Execute custom code
    CustomCode {
        language: String, // "javascript" | "python"
        code: String,
    },

    /// Transform data with a JMESPath or JSONata expression
    TransformData {
        expression: String,
        #[serde(default)]
        expression_type: TransformType,
    },

    /// Wait for a condition or time
    Wait {
        #[serde(skip_serializing_if = "Option::is_none")]
        seconds: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        until_condition: Option<String>,
    },

    /// Branch based on a condition
    Conditional {
        condition: String,
        if_true: Vec<WorkflowStep>,
        if_false: Vec<WorkflowStep>,
    },
}

/// Data transformation expression type
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransformType {
    #[default]
    Jsonata,
    Jmespath,
    Javascript,
}

/// A workflow in a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainedWorkflow {
    /// Workflow identifier
    pub workflow_id: String,

    /// Input mapping from previous workflow's output
    pub input_mapping: serde_json::Value,

    /// Optional condition for executing this step (JS expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// Execution preferences for a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPreference {
    /// Whether to wait for the workflow to complete
    pub blocking: bool,

    /// Timeout in milliseconds (0 = no timeout)
    pub timeout_ms: u64,

    /// Number of retry attempts on failure
    pub retry_count: u32,

    /// Execution priority
    pub priority: Priority,
}

impl Default for ExecutionPreference {
    fn default() -> Self {
        Self {
            blocking: true,
            timeout_ms: 30_000,
            retry_count: 0,
            priority: Priority::Normal,
        }
    }
}

/// Execution priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    /// Execute immediately, wait for result
    Critical,
    /// Execute immediately, don't wait
    High,
    /// Queue for execution (default)
    Normal,
    /// Batch with other low-priority tasks
    Low,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Builder for creating workflow definitions
pub struct WorkflowBuilder {
    name: String,
    nodes: Vec<WorkflowNode>,
    connections: serde_json::Map<String, serde_json::Value>,
    node_counter: usize,
}

impl WorkflowBuilder {
    /// Create a new workflow builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: Vec::new(),
            connections: serde_json::Map::new(),
            node_counter: 0,
        }
    }

    /// Add a manual trigger node (start point)
    pub fn add_trigger(mut self) -> Self {
        self.node_counter += 1;
        let node = WorkflowNode::new(
            format!("node_{}", self.node_counter),
            "Start",
            "n8n-nodes-base.manualTrigger",
            [240, 300],
        );
        self.nodes.push(node);
        self
    }

    /// Add an HTTP request node
    pub fn add_http_request(
        mut self,
        name: impl Into<String>,
        url: impl Into<String>,
        method: impl Into<String>,
    ) -> Self {
        self.node_counter += 1;
        let node_id = format!("node_{}", self.node_counter);
        let node = WorkflowNode::new(
            &node_id,
            name,
            "n8n-nodes-base.httpRequest",
            [460 + (self.node_counter as i32 - 1) * 200, 300],
        )
        .with_parameters(serde_json::json!({
            "url": url.into(),
            "method": method.into()
        }));
        self.connect_to_previous(&node_id);
        self.nodes.push(node);
        self
    }

    /// Add a Slack message node
    pub fn add_slack_message(
        mut self,
        name: impl Into<String>,
        channel: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.node_counter += 1;
        let node_id = format!("node_{}", self.node_counter);
        let node = WorkflowNode::new(
            &node_id,
            name,
            "n8n-nodes-base.slack",
            [460 + (self.node_counter as i32 - 1) * 200, 300],
        )
        .with_type_version(2)
        .with_parameters(serde_json::json!({
            "channel": channel.into(),
            "text": message.into()
        }));
        self.connect_to_previous(&node_id);
        self.nodes.push(node);
        self
    }

    /// Add a code execution node
    pub fn add_code(
        mut self,
        name: impl Into<String>,
        code: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        self.node_counter += 1;
        let node_id = format!("node_{}", self.node_counter);
        let node = WorkflowNode::new(
            &node_id,
            name,
            "n8n-nodes-base.code",
            [460 + (self.node_counter as i32 - 1) * 200, 300],
        )
        .with_parameters(serde_json::json!({
            "language": language.into(),
            "jsCode": code.into()
        }));
        self.connect_to_previous(&node_id);
        self.nodes.push(node);
        self
    }

    /// Connect the current node to the previous node
    fn connect_to_previous(&mut self, current_node_id: &str) {
        if self.nodes.is_empty() {
            return;
        }
        let prev_node = &self.nodes[self.nodes.len() - 1];
        let connection = serde_json::json!({
            "main": [[{
                "node": current_node_id,
                "type": "main",
                "index": 0
            }]]
        });
        self.connections
            .insert(prev_node.name.clone(), connection);
    }

    /// Build the workflow definition
    pub fn build(self) -> WorkflowDefinition {
        WorkflowDefinition {
            id: None,
            name: self.name,
            nodes: self.nodes,
            connections: serde_json::Value::Object(self.connections),
            settings: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_builder() {
        let workflow = WorkflowBuilder::new("Test Workflow")
            .add_trigger()
            .add_http_request("Fetch API", "https://api.example.com/data", "GET")
            .add_slack_message("Notify", "#general", "Data fetched!")
            .build();

        assert_eq!(workflow.name, "Test Workflow");
        assert_eq!(workflow.nodes.len(), 3);
        assert_eq!(workflow.nodes[0].node_type, "n8n-nodes-base.manualTrigger");
        assert_eq!(workflow.nodes[1].node_type, "n8n-nodes-base.httpRequest");
        assert_eq!(workflow.nodes[2].node_type, "n8n-nodes-base.slack");
    }

    #[test]
    fn test_workflow_serialization() {
        let workflow = WorkflowBuilder::new("Serialization Test")
            .add_trigger()
            .build();

        let json = serde_json::to_string(&workflow).unwrap();
        let parsed: WorkflowDefinition = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.name, workflow.name);
    }
}
