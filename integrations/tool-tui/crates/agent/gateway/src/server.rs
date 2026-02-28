//! WebSocket gateway server implementation using axum.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message as WsMessage, WebSocket},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde_json::json;
use tokio::sync::broadcast;
use tracing::{error, info, warn};
use uuid::Uuid;

use dx_agent_protocol::{GatewayEvent, GatewayMessage, GatewayRequest, GatewayResponse};

use crate::auth::AuthManager;
use crate::config::GatewayConfig;
use crate::health::HealthStatus;
use crate::rate_limiter::RateLimiter;
use crate::session_store::SessionStore;

/// Connected client state
pub struct ConnectedClient {
    pub id: String,
    pub addr: SocketAddr,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub session_id: Option<String>,
    pub authenticated: bool,
}

/// Shared gateway state
pub struct GatewayState {
    pub config: GatewayConfig,
    pub clients: DashMap<String, ConnectedClient>,
    pub rate_limiter: RateLimiter,
    pub session_store: SessionStore,
    pub auth_manager: AuthManager,
    pub event_tx: broadcast::Sender<GatewayEvent>,
    pub rpc_handlers: RwLock<std::collections::HashMap<String, RpcHandler>>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

/// RPC handler function type
pub type RpcHandler = Arc<
    dyn Fn(serde_json::Value) -> futures_util::future::BoxFuture<'static, serde_json::Value>
        + Send
        + Sync,
>;

/// The main gateway server
pub struct GatewayServer {
    state: Arc<GatewayState>,
}

impl GatewayServer {
    /// Create a new gateway server with the given configuration
    pub fn new(config: GatewayConfig) -> anyhow::Result<Self> {
        let (event_tx, _) = broadcast::channel(10_000);
        let rate_limiter = RateLimiter::new(
            config.rate_limit.max_requests,
            std::time::Duration::from_secs(config.rate_limit.window_secs),
        );
        let session_store = SessionStore::open(&config.database.path)?;
        let auth_manager = AuthManager::new(&config.auth);

        Ok(Self {
            state: Arc::new(GatewayState {
                config,
                clients: DashMap::new(),
                rate_limiter,
                session_store,
                auth_manager,
                event_tx,
                rpc_handlers: RwLock::new(std::collections::HashMap::new()),
                start_time: chrono::Utc::now(),
            }),
        })
    }

    /// Register an RPC method handler
    pub fn register_rpc<F, Fut>(&self, method: impl Into<String>, handler: F)
    where
        F: Fn(serde_json::Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = serde_json::Value> + Send + 'static,
    {
        let method = method.into();
        let boxed: RpcHandler = Arc::new(move |params| Box::pin(handler(params)));
        self.state.rpc_handlers.write().insert(method, boxed);
    }

    /// Start the gateway server
    pub async fn start(&self) -> anyhow::Result<()> {
        let state = self.state.clone();
        let addr = self.state.config.bind_address();

        let app = self.build_router();

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        info!("DX Gateway listening on ws://{}", addr);
        info!("Health check at http://{}/health", addr);

        // Spawn heartbeat cleanup task
        let cleanup_state = state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                cleanup_state.rate_limiter.cleanup().await;
                // Remove stale clients (no heartbeat for 2× interval)
                let timeout = chrono::Duration::seconds(
                    cleanup_state.config.server.connection_timeout_secs as i64,
                );
                let now = chrono::Utc::now();
                let stale: Vec<String> = cleanup_state
                    .clients
                    .iter()
                    .filter(|entry| now - entry.connected_at > timeout)
                    .map(|entry| entry.id.clone())
                    .collect();
                for id in &stale {
                    cleanup_state.clients.remove(id);
                }
                if !stale.is_empty() {
                    info!("Cleaned up {} stale connections", stale.len());
                }
            }
        });

        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

        Ok(())
    }

    /// Build the axum router
    fn build_router(&self) -> Router {
        let state = self.state.clone();

        let mut router = Router::new()
            .route("/ws", get(ws_upgrade_handler))
            .route("/health", get(health_handler))
            .route("/api/v1/status", get(status_handler))
            .route("/api/v1/sessions", get(sessions_handler))
            // Web UI routes
            .route("/", get(crate::web::dashboard_handler))
            .route("/ui/styles.css", get(crate::web::styles_handler))
            .route("/api/v1/dashboard", get(crate::web::dashboard_api_handler))
            .route(
                "/api/v1/webchat",
                axum::routing::post(crate::web::webchat_handler),
            )
            .route("/api/v1/config", get(crate::web::config_handler))
            .route("/api/v1/clients", get(crate::web::clients_handler))
            .route("/api/v1/logs", get(crate::web::logs_handler))
            .route("/api/v1/skills", get(crate::web::skills_handler))
            .with_state(state);

        // Add CORS if enabled
        if self.state.config.server.cors_enabled {
            use tower_http::cors::{Any, CorsLayer};
            let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);
            router = router.layer(cors);
        }

        // Add tracing
        router = router.layer(tower_http::trace::TraceLayer::new_for_http());

        router
    }

    /// Get number of connected clients
    pub fn connected_count(&self) -> usize {
        self.state.clients.len()
    }

    /// Broadcast an event to all connected clients
    pub fn broadcast_event(&self, event: GatewayEvent) {
        let _ = self.state.event_tx.send(event);
    }
}

// --- Axum Handlers ---

async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<GatewayState>>,
) -> impl IntoResponse {
    // Rate limit check
    if state.rate_limiter.check(addr.ip()).await {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    // Connection limit check
    if state.clients.len() >= state.config.server.max_connections {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    ws.max_message_size(state.config.server.max_message_size)
        .on_upgrade(move |socket| handle_ws_connection(socket, addr, state))
}

async fn handle_ws_connection(socket: WebSocket, addr: SocketAddr, state: Arc<GatewayState>) {
    let client_id = Uuid::new_v4().to_string();
    info!("New WebSocket connection: {} from {}", client_id, addr);

    // Register client
    state.clients.insert(
        client_id.clone(),
        ConnectedClient {
            id: client_id.clone(),
            addr,
            connected_at: chrono::Utc::now(),
            session_id: None,
            authenticated: !state.config.auth.required,
        },
    );

    // Subscribe to broadcast events
    let mut event_rx = state.event_tx.subscribe();

    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Task: forward broadcast events to this client
    let cid = client_id.clone();
    let forward_task = tokio::spawn(async move {
        while let Ok(event) = event_rx.recv().await {
            let msg = GatewayMessage::Event(event);
            if let Ok(json) = serde_json::to_string(&msg) {
                if ws_sender.send(WsMessage::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
        let _ = cid; // keep for logging
    });

    // Main receive loop
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            WsMessage::Text(text) => {
                let response = handle_client_message(&state, &client_id, &text).await;
                if let Some(resp_json) = response {
                    let msg = GatewayMessage::Response(resp_json);
                    if let Ok(json) = serde_json::to_string(&msg) {
                        // We can't send directly since ws_sender is moved;
                        // in production, use a channel. For now, broadcast.
                        let event = GatewayEvent::new(
                            "rpc_response",
                            json!({"client_id": client_id, "response": json}),
                        );
                        let _ = state.event_tx.send(event);
                    }
                }
            }
            WsMessage::Ping(data) => {
                // Auto-handled by axum
                let _ = data;
            }
            WsMessage::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup
    forward_task.abort();
    state.clients.remove(&client_id);
    info!("WebSocket disconnected: {} from {}", client_id, addr);
}

async fn handle_client_message(
    state: &GatewayState,
    client_id: &str,
    text: &str,
) -> Option<GatewayResponse> {
    let msg: GatewayMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            return Some(GatewayResponse::error(
                "unknown",
                dx_agent_protocol::messages::error_codes::PARSE_ERROR,
                format!("Parse error: {}", e),
            ));
        }
    };

    match msg {
        GatewayMessage::Request(req) => {
            // Check auth if required
            if state.config.auth.required {
                if let Some(client) = state.clients.get(client_id) {
                    if !client.authenticated {
                        return Some(GatewayResponse::error(
                            &req.id,
                            dx_agent_protocol::messages::error_codes::AUTH_REQUIRED,
                            "Authentication required",
                        ));
                    }
                }
            }

            // Dispatch to RPC handler
            dispatch_rpc(state, req).await
        }
        GatewayMessage::Ping { timestamp } => {
            let _ = state.event_tx.send(GatewayEvent::new(
                "pong",
                json!({"client_id": client_id, "timestamp": timestamp}),
            ));
            None
        }
        _ => None,
    }
}

async fn dispatch_rpc(state: &GatewayState, req: GatewayRequest) -> Option<GatewayResponse> {
    // Clone the handler out of the lock to avoid holding RwLockReadGuard across await
    let handler = {
        let handlers = state.rpc_handlers.read();
        handlers.get(&req.method).cloned()
    };
    if let Some(handler) = handler {
        let result = handler(req.params.clone()).await;
        Some(GatewayResponse::success(&req.id, result))
    } else {
        Some(GatewayResponse::error(
            &req.id,
            dx_agent_protocol::messages::error_codes::METHOD_NOT_FOUND,
            format!("Method not found: {}", req.method),
        ))
    }
}

async fn health_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let status = HealthStatus {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: (chrono::Utc::now() - state.start_time).num_seconds() as u64,
        connections: state.clients.len(),
        sessions: state.session_store.count().unwrap_or(0),
    };
    Json(status)
}

async fn status_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    Json(json!({
        "gateway": {
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_secs": (chrono::Utc::now() - state.start_time).num_seconds(),
            "connections": state.clients.len(),
            "config": {
                "port": state.config.server.port,
                "auth_required": state.config.auth.required,
                "rate_limit_enabled": state.config.rate_limit.enabled,
            }
        }
    }))
}

async fn sessions_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    match state.session_store.list_sessions() {
        Ok(sessions) => Json(json!({"sessions": sessions})).into_response(),
        Err(e) => {
            error!("Failed to list sessions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_creation() {
        let config = GatewayConfig::default();
        let server = GatewayServer::new(config);
        assert!(server.is_ok());
    }

    #[tokio::test]
    async fn test_rpc_registration() {
        let config = GatewayConfig::default();
        let server = GatewayServer::new(config).expect("server");
        server.register_rpc("test.echo", |params| async move { params });
        let handlers = server.state.rpc_handlers.read();
        assert!(handlers.contains_key("test.echo"));
    }
}
