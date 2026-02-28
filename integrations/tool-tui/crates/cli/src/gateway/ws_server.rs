//! Production WebSocket Server Implementation
//!
//! Real WebSocket server using tokio-tungstenite with support for:
//! - Multiple concurrent connections
//! - RPC method invocation
//! - Event broadcasting
//! - Connection health monitoring
//! - Graceful shutdown

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error as WsError, Message},
};

use super::protocol::{GatewayMessage, GatewayResponse, PROTOCOL_VERSION};
use super::rpc::{MethodRegistry, RpcEvent, RpcRequest, RpcResponse, StateVersion};
use super::{ConnectedClient, GatewayConfig, GatewayError, GatewayEvent, GatewayState};

/// WebSocket connection with sender channel
pub struct WsConnection {
    /// Client ID
    pub client_id: String,
    /// Message sender
    pub tx: mpsc::UnboundedSender<Message>,
    /// Last activity timestamp
    pub last_activity: Instant,
    /// Subscribed events
    pub subscriptions: Vec<String>,
}

/// Production WebSocket server
pub struct WsServer {
    /// Server configuration
    config: GatewayConfig,
    /// Gateway state
    state: Arc<GatewayState>,
    /// Active WebSocket connections
    connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    /// RPC method registry
    registry: Arc<MethodRegistry>,
    /// Broadcast channel for events
    event_tx: broadcast::Sender<RpcEvent>,
    /// State version for cache invalidation
    state_version: Arc<RwLock<StateVersion>>,
}

impl WsServer {
    /// Create a new WebSocket server
    pub fn new(config: GatewayConfig, state: Arc<GatewayState>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            config,
            state,
            connections: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(MethodRegistry::new()),
            event_tx,
            state_version: Arc::new(RwLock::new(StateVersion {
                presence: 1,
                health: 1,
                config: 1,
            })),
        }
    }

    /// Run the WebSocket server
    pub async fn run(
        self,
        addr: SocketAddr,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<(), GatewayError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| GatewayError::BindError(e.to_string()))?;

        tracing::info!("🚀 WebSocket server listening on ws://{}", addr);

        let server = Arc::new(self);

        // Spawn health check task
        let health_server = Arc::clone(&server);
        tokio::spawn(async move {
            health_server.run_health_checks().await;
        });

        loop {
            tokio::select! {
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            let server = Arc::clone(&server);
                            tokio::spawn(async move {
                                if let Err(e) = server.handle_connection(stream, peer_addr).await {
                                    tracing::warn!("Connection error from {}: {}", peer_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutdown signal received, closing WebSocket server");
                    break;
                }
            }
        }

        // Close all connections gracefully
        let connections = server.connections.read().await;
        for (_, conn) in connections.iter() {
            let _ = conn.tx.send(Message::Close(None));
        }

        Ok(())
    }

    /// Handle a new WebSocket connection
    async fn handle_connection(
        &self,
        stream: TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<(), GatewayError> {
        // Check connection limit
        if !self.state.can_accept_connection().await {
            tracing::warn!("Connection limit reached, rejecting {}", peer_addr);
            return Err(GatewayError::ConnectionLimitReached);
        }

        // Perform WebSocket handshake
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| GatewayError::WebSocketError(e.to_string()))?;

        let client_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("New connection: {} from {}", client_id, peer_addr);

        // Split the stream
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Create message channel for this connection
        let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

        // Register connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(
                client_id.clone(),
                WsConnection {
                    client_id: client_id.clone(),
                    tx: tx.clone(),
                    last_activity: Instant::now(),
                    subscriptions: Vec::new(),
                },
            );
        }

        // Register client in state
        let client = ConnectedClient {
            id: client_id.clone(),
            name: "Unknown".to_string(),
            platform: "unknown".to_string(),
            address: peer_addr,
            connected_at: Instant::now(),
            last_activity: Instant::now(),
            authenticated: !self.config.require_auth,
        };
        self.state.add_client(client).await;

        // Send welcome message
        let welcome = serde_json::json!({
            "type": "welcome",
            "client_id": client_id,
            "protocol_version": PROTOCOL_VERSION,
            "server_version": env!("CARGO_PKG_VERSION"),
            "auth_required": self.config.require_auth
        });
        let _ = tx.send(Message::Text(welcome.to_string()));

        // Subscribe to events
        let mut event_rx = self.event_tx.subscribe();
        let event_tx = tx.clone();
        let event_client_id = client_id.clone();
        let event_connections = Arc::clone(&self.connections);

        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                // Check if client is subscribed to this event
                let subscribed = {
                    let connections = event_connections.read().await;
                    connections
                        .get(&event_client_id)
                        .map(|c| {
                            c.subscriptions.is_empty()
                                || c.subscriptions.contains(&event.event)
                                || c.subscriptions.iter().any(|s| event.event.starts_with(s))
                        })
                        .unwrap_or(false)
                };

                if subscribed {
                    let msg = serde_json::to_string(&event).unwrap_or_default();
                    if event_tx.send(Message::Text(msg)).is_err() {
                        break;
                    }
                }
            }
        });

        // Spawn sender task
        let sender_client_id = client_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sender.send(msg).await.is_err() {
                    tracing::debug!("Failed to send to {}", sender_client_id);
                    break;
                }
            }
        });

        // Process incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    // Update last activity
                    {
                        let mut connections = self.connections.write().await;
                        if let Some(conn) = connections.get_mut(&client_id) {
                            conn.last_activity = Instant::now();
                        }
                    }

                    // Process message
                    let response = self.process_message(&client_id, &text).await;
                    if let Some(response) = response {
                        let _ = tx.send(Message::Text(response));
                    }
                }
                Ok(Message::Binary(data)) => {
                    // Handle binary protocol
                    if let Ok(text) = String::from_utf8(data) {
                        let response = self.process_message(&client_id, &text).await;
                        if let Some(response) = response {
                            let _ = tx.send(Message::Text(response));
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    let _ = tx.send(Message::Pong(data));
                }
                Ok(Message::Pong(_)) => {
                    // Connection is alive
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("Client {} disconnected", client_id);
                    break;
                }
                Ok(Message::Frame(_)) => {}
                Err(e) => {
                    tracing::warn!("WebSocket error for {}: {}", client_id, e);
                    break;
                }
            }
        }

        // Cleanup
        self.connections.write().await.remove(&client_id);
        self.state.remove_client(&client_id).await;

        // Increment presence version
        {
            let mut version = self.state_version.write().await;
            version.presence += 1;
        }

        Ok(())
    }

    /// Process an incoming message
    async fn process_message(&self, client_id: &str, text: &str) -> Option<String> {
        // Try to parse as RPC request
        if let Ok(rpc_request) = serde_json::from_str::<RpcRequest>(text) {
            let response =
                self.registry.invoke(Arc::clone(&self.state), client_id, rpc_request).await;
            return Some(serde_json::to_string(&response).unwrap_or_default());
        }

        // Try to parse as legacy GatewayMessage
        if let Ok(gateway_msg) = serde_json::from_str::<GatewayMessage>(text) {
            let response = self.handle_gateway_message(client_id, gateway_msg).await;
            return Some(serde_json::to_string(&response).unwrap_or_default());
        }

        // Try to parse as generic JSON with "type" field
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(msg_type) = json.get("type").and_then(|v| v.as_str()) {
                return self.handle_typed_message(client_id, msg_type, &json).await;
            }
        }

        // Unknown message format
        let error = serde_json::json!({
            "type": "error",
            "message": "Unknown message format"
        });
        Some(error.to_string())
    }

    /// Handle legacy GatewayMessage
    async fn handle_gateway_message(
        &self,
        client_id: &str,
        msg: GatewayMessage,
    ) -> GatewayResponse {
        match msg {
            GatewayMessage::Ping => GatewayResponse::Pong,

            GatewayMessage::Auth { code } => {
                match self.state.pending_pairings.read().await.get(&code) {
                    Some(session) if !session.is_expired() => {
                        if let Some(client) = self.state.clients.write().await.get_mut(client_id) {
                            client.authenticated = true;
                        }
                        let _ = self.state.event_tx.send(GatewayEvent::ClientAuthenticated {
                            client_id: client_id.to_string(),
                        });
                        GatewayResponse::AuthSuccess
                    }
                    _ => GatewayResponse::Error {
                        message: "Invalid or expired pairing code".to_string(),
                    },
                }
            }

            GatewayMessage::Identify { name, platform } => {
                if let Some(client) = self.state.clients.write().await.get_mut(client_id) {
                    client.name = name;
                    client.platform = platform;
                }
                GatewayResponse::Identified
            }

            GatewayMessage::Subscribe { events } => {
                let mut connections = self.connections.write().await;
                if let Some(conn) = connections.get_mut(client_id) {
                    conn.subscriptions = events;
                }
                GatewayResponse::Subscribed
            }

            GatewayMessage::Command {
                request_id,
                command,
                args,
            } => {
                // Check authentication
                let authenticated = self
                    .state
                    .clients
                    .read()
                    .await
                    .get(client_id)
                    .map(|c| c.authenticated)
                    .unwrap_or(false);

                if self.config.require_auth && !authenticated {
                    return GatewayResponse::Error {
                        message: "Authentication required".to_string(),
                    };
                }

                // Check if command is allowed
                if !self.state.is_command_allowed(&command) {
                    return GatewayResponse::Error {
                        message: format!("Command not allowed: {}", command),
                    };
                }

                // Convert to RPC and execute
                let rpc_request = RpcRequest {
                    id: request_id.clone(),
                    method: format!("command.{}", command),
                    params: serde_json::json!({ "args": args }),
                    timestamp: None,
                };

                let response =
                    self.registry.invoke(Arc::clone(&self.state), client_id, rpc_request).await;

                if response.error.is_some() {
                    GatewayResponse::Error {
                        message: response
                            .error
                            .map(|e| e.message)
                            .unwrap_or_else(|| "Unknown error".to_string()),
                    }
                } else {
                    GatewayResponse::CommandAck { request_id }
                }
            }
        }
    }

    /// Handle typed message with "type" field
    async fn handle_typed_message(
        &self,
        client_id: &str,
        msg_type: &str,
        json: &serde_json::Value,
    ) -> Option<String> {
        match msg_type {
            "subscribe" => {
                let events: Vec<String> = json
                    .get("events")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let mut connections = self.connections.write().await;
                if let Some(conn) = connections.get_mut(client_id) {
                    conn.subscriptions = events.clone();
                }

                Some(
                    serde_json::json!({
                        "type": "subscribed",
                        "events": events
                    })
                    .to_string(),
                )
            }

            "unsubscribe" => {
                let mut connections = self.connections.write().await;
                if let Some(conn) = connections.get_mut(client_id) {
                    conn.subscriptions.clear();
                }

                Some(
                    serde_json::json!({
                        "type": "unsubscribed"
                    })
                    .to_string(),
                )
            }

            _ => None,
        }
    }

    /// Broadcast an event to all connected clients
    pub async fn broadcast_event(&self, event: RpcEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Send a message to a specific client
    pub async fn send_to_client(&self, client_id: &str, message: &str) -> Result<(), GatewayError> {
        let connections = self.connections.read().await;
        if let Some(conn) = connections.get(client_id) {
            conn.tx
                .send(Message::Text(message.to_string()))
                .map_err(|_| GatewayError::WebSocketError("Failed to send".to_string()))
        } else {
            Err(GatewayError::ProtocolError(format!("Client not found: {}", client_id)))
        }
    }

    /// Run periodic health checks
    async fn run_health_checks(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            let now = Instant::now();
            let timeout = Duration::from_secs(120);

            // Find stale connections
            let stale: Vec<String> = {
                let connections = self.connections.read().await;
                connections
                    .iter()
                    .filter(|(_, conn)| now.duration_since(conn.last_activity) > timeout)
                    .map(|(id, _)| id.clone())
                    .collect()
            };

            // Remove stale connections
            for client_id in stale {
                tracing::info!("Removing stale connection: {}", client_id);

                if let Some(conn) = self.connections.write().await.remove(&client_id) {
                    let _ = conn.tx.send(Message::Close(None));
                }

                self.state.remove_client(&client_id).await;
            }

            // Send ping to all connections
            let connections = self.connections.read().await;
            for (_, conn) in connections.iter() {
                let _ = conn.tx.send(Message::Ping(vec![]));
            }

            tracing::debug!("Health check: {} active connections", connections.len());
        }
    }

    /// Get number of active connections
    pub async fn connection_count(&self) -> usize {
        self.connections.read().await.len()
    }
}

/// Run the production WebSocket server
pub async fn run_server(
    addr: SocketAddr,
    state: Arc<GatewayState>,
    shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), GatewayError> {
    let server = WsServer::new(state.config.clone(), state);
    server.run(addr, shutdown_rx).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::new(config.clone()));
        let server = WsServer::new(config, state);

        assert_eq!(server.connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_rpc_processing() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::new(config.clone()));
        let server = WsServer::new(config, state);

        let request = r#"{"id": "test-1", "method": "ping", "params": {}}"#;
        let response = server.process_message("client-1", request).await;

        assert!(response.is_some());
        let response_json: serde_json::Value = serde_json::from_str(&response.unwrap()).unwrap();
        assert_eq!(response_json["id"], "test-1");
        assert!(response_json["result"]["pong"].as_bool().unwrap_or(false));
    }

    #[tokio::test]
    async fn test_gateway_message_ping() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::new(config.clone()));
        let server = WsServer::new(config, state);

        let request = r#"{"type": "ping"}"#;
        let response = server.process_message("client-1", request).await;

        assert!(response.is_some());
        assert!(response.unwrap().contains("pong"));
    }
}
