//! Gateway message types for the DX Agent protocol.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Top-level gateway message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GatewayMessage {
    /// RPC-style request from client
    Request(GatewayRequest),
    /// Response to an RPC request
    Response(GatewayResponse),
    /// Server-pushed event
    Event(GatewayEvent),
    /// Heartbeat ping
    Ping { timestamp: i64 },
    /// Heartbeat pong
    Pong { timestamp: i64 },
}

/// RPC request from client to gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
    #[serde(default)]
    pub metadata: RequestMetadata,
}

/// Request metadata for tracing and auth
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,
}

/// RPC response from gateway to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ProtocolError>,
}

/// Protocol-level error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Server-pushed event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvent {
    pub event: String,
    pub data: Value,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

/// Presence update for connected users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub user_id: String,
    pub status: PresenceStatus,
    pub last_seen: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
}

/// User presence status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    Offline,
}

/// Typing indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingIndicator {
    pub user_id: String,
    pub channel: String,
    pub is_typing: bool,
    pub timestamp: DateTime<Utc>,
}

/// Session state synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSync {
    pub session_id: String,
    pub action: SessionAction,
    pub data: Value,
}

/// Session actions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Create,
    Update,
    Delete,
    Restore,
    Compact,
}

/// Channel-related events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvent {
    pub channel_type: String,
    pub channel_id: String,
    pub event_type: ChannelEventType,
    pub data: Value,
    pub timestamp: DateTime<Utc>,
}

/// Types of channel events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelEventType {
    MessageReceived,
    MessageSent,
    MessageDelivered,
    MessageRead,
    UserJoined,
    UserLeft,
    ChannelCreated,
    ChannelDeleted,
    Error,
}

// --- Constructors ---

impl GatewayRequest {
    pub fn new(method: impl Into<String>, params: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            method: method.into(),
            params,
            metadata: RequestMetadata::default(),
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.metadata.session_id = Some(session_id.into());
        self
    }
}

impl GatewayResponse {
    pub fn success(id: impl Into<String>, result: Value) -> Self {
        Self {
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: impl Into<String>, code: i32, message: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            result: None,
            error: Some(ProtocolError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

impl GatewayEvent {
    pub fn new(event: impl Into<String>, data: Value) -> Self {
        Self {
            event: event.into(),
            data,
            timestamp: Utc::now(),
        }
    }
}

/// Standard error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const AUTH_REQUIRED: i32 = -32000;
    pub const AUTH_FAILED: i32 = -32001;
    pub const RATE_LIMITED: i32 = -32002;
    pub const SESSION_NOT_FOUND: i32 = -32003;
    pub const CHANNEL_ERROR: i32 = -32004;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_message_serialize() {
        let req = GatewayRequest::new("chat.send", serde_json::json!({"text": "hello"}));
        let msg = GatewayMessage::Request(req);
        let json = serde_json::to_string(&msg).expect("serialize");
        assert!(json.contains("\"type\":\"Request\""));
        assert!(json.contains("chat.send"));
    }

    #[test]
    fn test_response_success() {
        let resp = GatewayResponse::success("123", serde_json::json!({"ok": true}));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_response_error() {
        let resp = GatewayResponse::error("123", error_codes::AUTH_REQUIRED, "Auth required");
        assert!(resp.result.is_none());
        let err = resp.error.as_ref().expect("error");
        assert_eq!(err.code, -32000);
    }

    #[test]
    fn test_presence_roundtrip() {
        let p = PresenceUpdate {
            user_id: "user1".into(),
            status: PresenceStatus::Online,
            last_seen: Utc::now(),
            channel: Some("general".into()),
        };
        let json = serde_json::to_string(&p).expect("serialize");
        let p2: PresenceUpdate = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(p2.status, PresenceStatus::Online);
    }
}
