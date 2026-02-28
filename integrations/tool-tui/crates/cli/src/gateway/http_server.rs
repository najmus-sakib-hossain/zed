//! HTTP Server with Webhook Support
//!
//! Axum-based HTTP server providing:
//! - REST API for gateway control
//! - Webhook endpoints for external integrations
//! - OpenAI-compatible API endpoints
//! - Static file serving for control UI

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Router,
    extract::{Path, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};

use super::rpc::{MethodRegistry, RpcRequest, RpcResponse};
use super::{GatewayConfig, GatewayError, GatewayState};

/// HTTP server for gateway API and webhooks
pub struct HttpServer {
    /// Gateway state
    state: Arc<GatewayState>,
    /// RPC method registry
    registry: Arc<MethodRegistry>,
    /// Webhook handlers
    webhooks: Arc<WebhookRegistry>,
}

impl HttpServer {
    /// Create a new HTTP server
    pub fn new(state: Arc<GatewayState>) -> Self {
        Self {
            state,
            registry: Arc::new(MethodRegistry::new()),
            webhooks: Arc::new(WebhookRegistry::new()),
        }
    }

    /// Build the Axum router
    pub fn router(&self) -> Router {
        let state = AppState {
            gateway: Arc::clone(&self.state),
            registry: Arc::clone(&self.registry),
            webhooks: Arc::clone(&self.webhooks),
        };

        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers(Any);

        Router::new()
            // Health and info
            .route("/health", get(health_handler))
            .route("/info", get(info_handler))
            // RPC endpoint
            .route("/rpc", post(rpc_handler))
            // REST API
            .route("/api/v1/channels", get(list_channels))
            .route("/api/v1/channels/:id/status", get(channel_status))
            .route("/api/v1/channels/:id/connect", post(connect_channel))
            .route("/api/v1/channels/:id/disconnect", post(disconnect_channel))
            .route("/api/v1/channels/:id/qr", get(channel_qr))
            .route("/api/v1/sessions", get(list_sessions))
            .route("/api/v1/sessions/:key", get(get_session))
            .route("/api/v1/sessions/:key", delete(delete_session))
            .route("/api/v1/config", get(get_config))
            .route("/api/v1/config", post(update_config))
            .route("/api/v1/clients", get(list_clients))
            .route("/api/v1/pairing", post(create_pairing))
            // OpenAI-compatible API
            .route("/v1/chat/completions", post(openai_chat_completions))
            .route("/v1/models", get(openai_list_models))
            // Webhooks
            .route("/webhooks/:id", post(webhook_handler))
            .route("/webhooks", get(list_webhooks))
            .route("/webhooks", post(register_webhook))
            .route("/webhooks/:id", delete(unregister_webhook))
            // CORS
            .layer(cors)
            .with_state(state)
    }

    /// Run the HTTP server
    pub async fn run(
        self,
        addr: SocketAddr,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<(), GatewayError> {
        let router = self.router();

        tracing::info!("🌐 HTTP server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| GatewayError::BindError(e.to_string()))?;

        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.recv().await;
                tracing::info!("HTTP server shutting down");
            })
            .await
            .map_err(|e| GatewayError::BindError(e.to_string()))?;

        Ok(())
    }
}

/// Shared application state
#[derive(Clone)]
struct AppState {
    gateway: Arc<GatewayState>,
    registry: Arc<MethodRegistry>,
    webhooks: Arc<WebhookRegistry>,
}

// ============================================================================
// Health & Info Handlers
// ============================================================================

async fn health_handler(State(state): State<AppState>) -> Json<Value> {
    let client_count = state.gateway.client_count().await;
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "clients": client_count,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn info_handler(State(state): State<AppState>) -> Json<Value> {
    let client_count = state.gateway.client_count().await;
    Json(json!({
        "name": "dx-gateway",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": super::protocol::PROTOCOL_VERSION,
        "connected_clients": client_count,
        "max_clients": state.gateway.config.max_connections,
        "auth_required": state.gateway.config.require_auth,
        "mdns_enabled": state.gateway.config.mdns_enabled,
        "service_name": state.gateway.config.service_name
    }))
}

// ============================================================================
// RPC Handler
// ============================================================================

#[derive(Debug, Deserialize)]
struct RpcRequestBody {
    #[serde(flatten)]
    request: RpcRequest,
}

async fn rpc_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RpcRequestBody>,
) -> Json<RpcResponse> {
    // Get client ID from header or generate one
    let client_id = headers
        .get("x-client-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http-client")
        .to_string();

    let response = state.registry.invoke(state.gateway, &client_id, body.request).await;
    Json(response)
}

// ============================================================================
// Channel Handlers
// ============================================================================

async fn list_channels(State(_state): State<AppState>) -> Json<Value> {
    // TODO: Get from actual channel registry
    Json(json!({
        "channels": [
            {"id": "whatsapp", "name": "WhatsApp", "status": "disconnected", "enabled": false},
            {"id": "telegram", "name": "Telegram", "status": "disconnected", "enabled": false},
            {"id": "discord", "name": "Discord", "status": "disconnected", "enabled": false},
            {"id": "slack", "name": "Slack", "status": "disconnected", "enabled": false},
            {"id": "signal", "name": "Signal", "status": "disconnected", "enabled": false},
            {"id": "imessage", "name": "iMessage", "status": "disconnected", "enabled": false}
        ]
    }))
}

async fn channel_status(
    State(_state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Json<Value> {
    Json(json!({
        "channel_id": channel_id,
        "status": "disconnected",
        "last_heartbeat": null,
        "error": null
    }))
}

async fn connect_channel(
    State(_state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Json<Value> {
    tracing::info!("HTTP: Connecting channel {}", channel_id);
    Json(json!({
        "channel_id": channel_id,
        "status": "connecting"
    }))
}

async fn disconnect_channel(
    State(_state): State<AppState>,
    Path(channel_id): Path<String>,
) -> Json<Value> {
    tracing::info!("HTTP: Disconnecting channel {}", channel_id);
    Json(json!({
        "channel_id": channel_id,
        "status": "disconnected"
    }))
}

async fn channel_qr(State(_state): State<AppState>, Path(channel_id): Path<String>) -> Json<Value> {
    tracing::info!("HTTP: Requesting QR for {}", channel_id);
    Json(json!({
        "channel_id": channel_id,
        "qr_pending": true,
        "timeout_seconds": 60
    }))
}

// ============================================================================
// Session Handlers
// ============================================================================

async fn list_sessions(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "sessions": []
    }))
}

async fn get_session(
    State(_state): State<AppState>,
    Path(session_key): Path<String>,
) -> Json<Value> {
    Json(json!({
        "session_key": session_key,
        "messages": [],
        "created_at": chrono::Utc::now().timestamp()
    }))
}

async fn delete_session(
    State(_state): State<AppState>,
    Path(session_key): Path<String>,
) -> Json<Value> {
    tracing::info!("HTTP: Deleting session {}", session_key);
    Json(json!({
        "deleted": true,
        "session_key": session_key
    }))
}

// ============================================================================
// Config Handlers
// ============================================================================

async fn get_config(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "gateway": {
            "host": state.gateway.config.host,
            "port": state.gateway.config.port,
            "mdns_enabled": state.gateway.config.mdns_enabled,
            "require_auth": state.gateway.config.require_auth
        },
        "channels": {},
        "agent": {}
    }))
}

#[derive(Debug, Deserialize)]
struct ConfigUpdate {
    #[serde(flatten)]
    updates: Value,
}

async fn update_config(
    State(_state): State<AppState>,
    Json(body): Json<ConfigUpdate>,
) -> Json<Value> {
    tracing::info!("HTTP: Config update: {:?}", body.updates);
    Json(json!({
        "updated": true,
        "changes": body.updates
    }))
}

// ============================================================================
// Client Handlers
// ============================================================================

async fn list_clients(State(state): State<AppState>) -> Json<Value> {
    let clients = state.gateway.clients.read().await;
    let client_list: Vec<_> = clients
        .values()
        .map(|c| {
            json!({
                "id": c.id,
                "name": c.name,
                "platform": c.platform,
                "authenticated": c.authenticated,
                "address": c.address.to_string()
            })
        })
        .collect();

    Json(json!({
        "clients": client_list,
        "count": client_list.len()
    }))
}

async fn create_pairing(State(state): State<AppState>) -> Json<Value> {
    let code = super::pairing::generate_code();
    let session = super::PairingSession::new(code.clone(), Duration::from_secs(300));
    state.gateway.pending_pairings.write().await.insert(code.clone(), session);

    Json(json!({
        "code": code,
        "expires_in": 300,
        "qr_data": format!("dx://pair?code={}", code)
    }))
}

// ============================================================================
// OpenAI-Compatible API
// ============================================================================

#[derive(Debug, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

async fn openai_chat_completions(
    State(_state): State<AppState>,
    Json(body): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    let message_content = body.messages.last().map(|m| m.content.clone()).unwrap_or_default();

    tracing::info!(
        "OpenAI API: {} model, {} messages, stream={}",
        body.model,
        body.messages.len(),
        body.stream
    );

    // TODO: Integrate with actual LLM client
    let response = json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": body.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": format!("Echo: {}", message_content)
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": message_content.len() / 4,
            "completion_tokens": message_content.len() / 4 + 10,
            "total_tokens": message_content.len() / 2 + 10
        }
    });

    Json(response)
}

async fn openai_list_models(State(_state): State<AppState>) -> Json<Value> {
    Json(json!({
        "object": "list",
        "data": [
            {
                "id": "gpt-4o",
                "object": "model",
                "created": 1700000000,
                "owned_by": "openai"
            },
            {
                "id": "claude-3-5-sonnet",
                "object": "model",
                "created": 1700000000,
                "owned_by": "anthropic"
            },
            {
                "id": "llama-3.3-70b",
                "object": "model",
                "created": 1700000000,
                "owned_by": "meta"
            },
            {
                "id": "gemini-2.0-flash",
                "object": "model",
                "created": 1700000000,
                "owned_by": "google"
            }
        ]
    }))
}

// ============================================================================
// Webhook System
// ============================================================================

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook ID
    pub id: String,
    /// Webhook URL
    pub url: String,
    /// Events to subscribe to
    pub events: Vec<String>,
    /// Secret for signature verification
    pub secret: Option<String>,
    /// Whether webhook is enabled
    pub enabled: bool,
}

/// Webhook registry
pub struct WebhookRegistry {
    webhooks: tokio::sync::RwLock<Vec<WebhookConfig>>,
}

impl WebhookRegistry {
    pub fn new() -> Self {
        Self {
            webhooks: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    pub async fn register(&self, config: WebhookConfig) {
        self.webhooks.write().await.push(config);
    }

    pub async fn unregister(&self, id: &str) -> bool {
        let mut webhooks = self.webhooks.write().await;
        let len_before = webhooks.len();
        webhooks.retain(|w| w.id != id);
        webhooks.len() < len_before
    }

    pub async fn list(&self) -> Vec<WebhookConfig> {
        self.webhooks.read().await.clone()
    }

    pub async fn dispatch(&self, event: &str, payload: &Value) {
        let webhooks = self.webhooks.read().await;
        let client = reqwest::Client::new();

        for webhook in webhooks.iter() {
            if !webhook.enabled {
                continue;
            }

            if !webhook.events.is_empty() && !webhook.events.contains(&event.to_string()) {
                continue;
            }

            let body = json!({
                "event": event,
                "payload": payload,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            let url = webhook.url.clone();
            let client = client.clone();

            tokio::spawn(async move {
                match client.post(&url).json(&body).send().await {
                    Ok(response) => {
                        tracing::debug!("Webhook {} delivered: {}", url, response.status());
                    }
                    Err(e) => {
                        tracing::warn!("Webhook {} failed: {}", url, e);
                    }
                }
            });
        }
    }
}

async fn webhook_handler(
    State(_state): State<AppState>,
    Path(webhook_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    tracing::info!("Webhook received: {} - {:?}", webhook_id, body);

    // TODO: Process webhook based on webhook_id
    Json(json!({
        "received": true,
        "webhook_id": webhook_id
    }))
}

async fn list_webhooks(State(state): State<AppState>) -> Json<Value> {
    let webhooks = state.webhooks.list().await;
    Json(json!({
        "webhooks": webhooks
    }))
}

#[derive(Debug, Deserialize)]
struct RegisterWebhookRequest {
    url: String,
    events: Option<Vec<String>>,
    secret: Option<String>,
}

async fn register_webhook(
    State(state): State<AppState>,
    Json(body): Json<RegisterWebhookRequest>,
) -> Json<Value> {
    let webhook = WebhookConfig {
        id: uuid::Uuid::new_v4().to_string(),
        url: body.url,
        events: body.events.unwrap_or_default(),
        secret: body.secret,
        enabled: true,
    };

    let id = webhook.id.clone();
    state.webhooks.register(webhook).await;

    Json(json!({
        "registered": true,
        "webhook_id": id
    }))
}

async fn unregister_webhook(
    State(state): State<AppState>,
    Path(webhook_id): Path<String>,
) -> Json<Value> {
    let removed = state.webhooks.unregister(&webhook_id).await;
    Json(json!({
        "removed": removed,
        "webhook_id": webhook_id
    }))
}

/// Run the HTTP server
pub async fn run_http_server(
    addr: SocketAddr,
    state: Arc<GatewayState>,
    shutdown_rx: mpsc::Receiver<()>,
) -> Result<(), GatewayError> {
    let server = HttpServer::new(state);
    server.run(addr, shutdown_rx).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::new(config).await);
        let server = HttpServer::new(state);
        let router = server.router();

        let response = router
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_info_endpoint() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::new(config).await);
        let server = HttpServer::new(state);
        let router = server.router();

        let response = router
            .oneshot(Request::builder().uri("/info").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_channels_list() {
        let config = GatewayConfig::default();
        let state = Arc::new(GatewayState::new(config).await);
        let server = HttpServer::new(state);
        let router = server.router();

        let response = router
            .oneshot(Request::builder().uri("/api/v1/channels").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
