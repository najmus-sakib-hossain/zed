//! WebSocket handler for real-time communication

use super::{GatewayMessage, GatewayRequest, GatewayResponse, RpcRegistry};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WebSocketHandler {
    rpc: Arc<RwLock<RpcRegistry>>,
}

impl WebSocketHandler {
    pub fn new(rpc: Arc<RwLock<RpcRegistry>>) -> Self {
        Self { rpc }
    }

    pub async fn handle_message(&self, msg: String) -> Result<String> {
        let message: GatewayMessage = serde_json::from_str(&msg)?;

        match message {
            GatewayMessage::Request(req) => {
                let response = self.handle_request(req).await?;
                Ok(serde_json::to_string(&GatewayMessage::Response(response))?)
            }
            _ => Ok(msg),
        }
    }

    async fn handle_request(&self, req: GatewayRequest) -> Result<GatewayResponse> {
        let rpc = self.rpc.read().await;

        match rpc.call(&req.method, req.params.clone()) {
            Ok(result) => Ok(GatewayResponse::success(req.id, result)),
            Err(e) => Ok(GatewayResponse::error(req.id, e.to_string())),
        }
    }
}
