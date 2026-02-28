//! Gateway protocol messages

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum GatewayMessage {
    Request(GatewayRequest),
    Response(GatewayResponse),
    Event(GatewayEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayRequest {
    pub id: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub id: String,
    pub result: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayEvent {
    pub event: String,
    pub data: Value,
}

impl GatewayRequest {
    pub fn new(method: String, params: Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method,
            params,
        }
    }
}

impl GatewayResponse {
    pub fn success(id: String, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: String, error: String) -> Self {
        Self {
            id,
            result: None,
            error: Some(error),
        }
    }
}
