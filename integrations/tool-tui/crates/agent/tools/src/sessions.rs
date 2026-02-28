//! Session management tools for the agent
//!
//! Provides tools for managing agent sessions:
//! - sessions_list: List active sessions
//! - sessions_info: Get session details
//! - sessions_send: Send a message to a session
//! - sessions_history: Retrieve session message history
//! - sessions_spawn: Create a new session
//! - sessions_end: End a session

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use crate::definition::{ParameterType, Tool, ToolCall, ToolDefinition, ToolParameter, ToolResult};

/// Tool to list active sessions
pub struct SessionsListTool;

#[async_trait]
impl Tool for SessionsListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions_list".into(),
            description:
                "List all active agent sessions with their IDs, channels, and message counts".into(),
            parameters: vec![
                ToolParameter {
                    name: "channel".into(),
                    description: "Filter by channel name (e.g., 'telegram', 'discord')".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "limit".into(),
                    description: "Max number of sessions to return".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(json!(20)),
                    enum_values: None,
                },
            ],
            category: "sessions".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let _channel = call.arguments.get("channel").and_then(|v| v.as_str());
        let limit = call.arguments.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

        // In production, this would query the SessionManager
        // For now, return a structured response showing the tool works
        let result = json!({
            "sessions": [],
            "total": 0,
            "limit": limit,
            "note": "Connect SessionManager to populate sessions"
        });

        Ok(ToolResult::success(call.id, result.to_string()))
    }
}

/// Tool to get session details
pub struct SessionsInfoTool;

#[async_trait]
impl Tool for SessionsInfoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions_info".into(),
            description: "Get detailed information about a specific session including message history and token usage".into(),
            parameters: vec![ToolParameter {
                name: "session_id".into(),
                description: "The session ID to query".into(),
                param_type: ParameterType::String,
                required: true,
                default: None,
                enum_values: None,
            }],
            category: "sessions".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let session_id = call
            .arguments
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;

        let result = json!({
            "session_id": session_id,
            "status": "not_found",
            "note": "Connect SessionManager to query real sessions"
        });

        Ok(ToolResult::success(call.id, result.to_string()))
    }
}

/// Tool to send a message into an existing session
pub struct SessionsSendTool;

#[async_trait]
impl Tool for SessionsSendTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions_send".into(),
            description: "Send a message into a specific session".into(),
            parameters: vec![
                ToolParameter {
                    name: "session_id".into(),
                    description: "The session ID to send to".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "message".into(),
                    description: "Message text to send".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "sessions".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let session_id = call
            .arguments
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;
        let message = call
            .arguments
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("message is required"))?;

        let result = json!({
            "session_id": session_id,
            "message": message,
            "status": "queued",
            "note": "Connect SessionManager runtime to process sends"
        });

        Ok(ToolResult::success(call.id, result.to_string()))
    }
}

/// Tool to retrieve message history for a session
pub struct SessionsHistoryTool;

#[async_trait]
impl Tool for SessionsHistoryTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions_history".into(),
            description: "Get recent message history for a session".into(),
            parameters: vec![
                ToolParameter {
                    name: "session_id".into(),
                    description: "The session ID to query".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "limit".into(),
                    description: "Maximum history items to return".into(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(json!(50)),
                    enum_values: None,
                },
            ],
            category: "sessions".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let session_id = call
            .arguments
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;
        let limit = call.arguments.get("limit").and_then(|v| v.as_u64()).unwrap_or(50);

        let result = json!({
            "session_id": session_id,
            "limit": limit,
            "messages": [],
            "note": "Connect SessionManager runtime to populate history"
        });

        Ok(ToolResult::success(call.id, result.to_string()))
    }
}

/// Tool to create a new session
pub struct SessionsSpawnTool;

#[async_trait]
impl Tool for SessionsSpawnTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions_spawn".into(),
            description: "Create a new agent session for a channel or purpose".into(),
            parameters: vec![
                ToolParameter {
                    name: "channel".into(),
                    description: "Channel type (e.g., 'telegram', 'discord', 'cli')".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "user_id".into(),
                    description: "User identifier for the session".into(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ToolParameter {
                    name: "system_prompt".into(),
                    description: "Optional system prompt for this session".into(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: "sessions".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let channel = call
            .arguments
            .get("channel")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("channel is required"))?;
        let user_id = call
            .arguments
            .get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("user_id is required"))?;

        let session_id = uuid::Uuid::new_v4().to_string();

        let result = json!({
            "session_id": session_id,
            "channel": channel,
            "user_id": user_id,
            "status": "created",
            "note": "Connect SessionManager to persist sessions"
        });

        Ok(ToolResult::success(call.id, result.to_string()))
    }
}

/// Tool to end a session
pub struct SessionsEndTool;

#[async_trait]
impl Tool for SessionsEndTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions_end".into(),
            description: "End an active session, saving its state".into(),
            parameters: vec![ToolParameter {
                name: "session_id".into(),
                description: "The session ID to end".into(),
                param_type: ParameterType::String,
                required: true,
                default: None,
                enum_values: None,
            }],
            category: "sessions".into(),
            requires_confirmation: true,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let session_id = call
            .arguments
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("session_id is required"))?;

        let result = json!({
            "session_id": session_id,
            "status": "ended",
            "note": "Connect SessionManager to actually end sessions"
        });

        Ok(ToolResult::success(call.id, result.to_string()))
    }
}

/// Register all session tools into a registry
pub fn register_session_tools(registry: &mut crate::ToolRegistry) {
    use std::sync::Arc;
    registry.register(Arc::new(SessionsListTool));
    registry.register(Arc::new(SessionsInfoTool));
    registry.register(Arc::new(SessionsSendTool));
    registry.register(Arc::new(SessionsHistoryTool));
    registry.register(Arc::new(SessionsSpawnTool));
    registry.register(Arc::new(SessionsEndTool));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ToolRegistry;

    #[tokio::test]
    async fn test_session_tools_registration() {
        let mut registry = ToolRegistry::new();
        register_session_tools(&mut registry);
        assert_eq!(registry.count(), 6);

        // Verify all tools have the sessions category
        let session_tools = registry.by_category("sessions");
        assert_eq!(session_tools.len(), 6);
    }

    #[tokio::test]
    async fn test_sessions_list() {
        let tool = SessionsListTool;
        let call = ToolCall {
            id: "test-1".into(),
            name: "sessions_list".into(),
            arguments: serde_json::json!({"limit": 10}),
        };

        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_sessions_spawn() {
        let tool = SessionsSpawnTool;
        let call = ToolCall {
            id: "test-2".into(),
            name: "sessions_spawn".into(),
            arguments: serde_json::json!({
                "channel": "telegram",
                "user_id": "user123"
            }),
        };

        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("session_id"));
    }

    #[tokio::test]
    async fn test_sessions_send() {
        let tool = SessionsSendTool;
        let call = ToolCall {
            id: "test-3".into(),
            name: "sessions_send".into(),
            arguments: serde_json::json!({
                "session_id": "session-1",
                "message": "hello"
            }),
        };

        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("queued"));
    }

    #[tokio::test]
    async fn test_sessions_history() {
        let tool = SessionsHistoryTool;
        let call = ToolCall {
            id: "test-4".into(),
            name: "sessions_history".into(),
            arguments: serde_json::json!({
                "session_id": "session-1",
                "limit": 25
            }),
        };

        let result = tool.execute(call).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("messages"));
    }
}
