//! Actix Web HTTP Server for DX Gateway
//!
//! Production-ready HTTP server with:
//! - RESTful API endpoints
//! - WebSocket upgrade support
//! - Static file serving for Control UI
//! - OpenAI-compatible API
//! - Pairing endpoints for iOS/Android/macOS

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, middleware, web};
use actix_ws::Message as WsMessage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::platform_pairing::PairingStore;

// Local struct-based message types for WebSocket JSON-RPC
/// Gateway WebSocket request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsRequest {
    /// Request ID for correlation
    pub id: String,
    /// Method name
    pub method: String,
    /// Parameters
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Gateway WebSocket response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsResponse {
    /// Request ID (matches request)
    pub id: String,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Gateway server configuration
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// HTTP port to bind
    pub port: u16,
    /// Bind address (127.0.0.1 for loopback, 0.0.0.0 for LAN)
    pub bind_address: String,
    /// Enable Control UI
    pub control_ui_enabled: bool,
    /// Control UI base path
    pub control_ui_path: Option<String>,
    /// Enable OpenAI-compatible API
    pub openai_api_enabled: bool,
    /// Enable OpenResponses API  
    pub open_responses_enabled: bool,
    /// Enable TLS
    pub tls_enabled: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<String>,
    /// TLS key path
    pub tls_key_path: Option<String>,
    /// Auth token (if required)
    pub auth_token: Option<String>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 31337,
            bind_address: "127.0.0.1".to_string(),
            control_ui_enabled: true,
            control_ui_path: None,
            openai_api_enabled: true,
            open_responses_enabled: true,
            tls_enabled: false,
            tls_cert_path: None,
            tls_key_path: None,
            auth_token: None,
        }
    }
}

/// Connected client info
#[derive(Debug, Clone, Serialize)]
pub struct ConnectedClient {
    pub id: String,
    pub device_type: DeviceType,
    pub device_name: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: chrono::DateTime<chrono::Utc>,
    pub platform_version: Option<String>,
}

/// Device type for pairing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    MacOS,
    IOS,
    Android,
    Windows,
    Linux,
    Web,
    CLI,
}

/// Gateway runtime state
pub struct GatewayState {
    pub config: GatewayConfig,
    pub pairing_store: PairingStore,
    pub connected_clients: RwLock<HashMap<String, ConnectedClient>>,
    pub presence_version: RwLock<u64>,
    pub health_version: RwLock<u64>,
}

impl GatewayState {
    pub fn new(config: GatewayConfig) -> Self {
        Self {
            config,
            pairing_store: PairingStore::new(),
            connected_clients: RwLock::new(HashMap::new()),
            presence_version: RwLock::new(0),
            health_version: RwLock::new(0),
        }
    }

    pub async fn increment_presence(&self) -> u64 {
        let mut version = self.presence_version.write().await;
        *version += 1;
        *version
    }

    pub async fn increment_health(&self) -> u64 {
        let mut version = self.health_version.write().await;
        *version += 1;
        *version
    }
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
    connected_clients: usize,
    presence_version: u64,
    health_version: u64,
}

/// Pairing request body
#[derive(Deserialize)]
struct PairRequestBody {
    code: String,
    device_type: DeviceType,
    device_name: String,
    platform_version: Option<String>,
}

/// Pairing response
#[derive(Serialize)]
struct PairResponse {
    success: bool,
    client_id: Option<String>,
    token: Option<String>,
    error: Option<String>,
}

/// Generate pairing code response
#[derive(Serialize)]
struct GeneratePairingCodeResponse {
    code: String,
    expires_at: String,
    qr_data: String,
}

/// OpenAI chat completion request
#[derive(Deserialize)]
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

#[derive(Deserialize, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenAI chat completion response
#[derive(Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: &'static str,
    created: i64,
    model: String,
    choices: Vec<ChatChoice>,
    usage: ChatUsage,
}

#[derive(Serialize)]
struct ChatChoice {
    index: u32,
    message: ChatMessage,
    finish_reason: String,
}

#[derive(Serialize)]
struct ChatUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

// ============================================================================
// HTTP Handlers
// ============================================================================

/// Health check endpoint
async fn health_check(state: web::Data<Arc<GatewayState>>) -> impl Responder {
    let clients = state.connected_clients.read().await;
    let presence = *state.presence_version.read().await;
    let health = *state.health_version.read().await;

    HttpResponse::Ok().json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: 0, // TODO: Track actual uptime
        connected_clients: clients.len(),
        presence_version: presence,
        health_version: health,
    })
}

/// List connected clients
async fn list_clients(state: web::Data<Arc<GatewayState>>) -> impl Responder {
    let clients = state.connected_clients.read().await;
    let client_list: Vec<_> = clients.values().cloned().collect();
    HttpResponse::Ok().json(client_list)
}

/// Generate a new pairing code
async fn generate_pairing_code(state: web::Data<Arc<GatewayState>>) -> impl Responder {
    let code = state.pairing_store.generate_code();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);

    // Generate QR code data (dx://pair?code=XXXXXXXX&host=...)
    let qr_data = format!(
        "dx://pair?code={}&host={}:{}",
        code.code, state.config.bind_address, state.config.port
    );

    HttpResponse::Ok().json(GeneratePairingCodeResponse {
        code: code.code,
        expires_at: expires_at.to_rfc3339(),
        qr_data,
    })
}

/// Pair a device with the gateway
async fn pair_device(
    state: web::Data<Arc<GatewayState>>,
    body: web::Json<PairRequestBody>,
) -> impl Responder {
    // Validate pairing code
    let valid = state.pairing_store.validate_code(&body.code);

    if !valid {
        return HttpResponse::BadRequest().json(PairResponse {
            success: false,
            client_id: None,
            token: None,
            error: Some("Invalid or expired pairing code".to_string()),
        });
    }

    // Generate client ID and auth token
    let client_id = uuid::Uuid::new_v4().to_string();
    let token = generate_auth_token();

    // Register the client
    let client = ConnectedClient {
        id: client_id.clone(),
        device_type: body.device_type.clone(),
        device_name: body.device_name.clone(),
        connected_at: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
        platform_version: body.platform_version.clone(),
    };

    {
        let mut clients = state.connected_clients.write().await;
        clients.insert(client_id.clone(), client);
    }

    // Invalidate the pairing code
    state.pairing_store.invalidate_code(&body.code);
    state.increment_presence().await;

    HttpResponse::Ok().json(PairResponse {
        success: true,
        client_id: Some(client_id),
        token: Some(token),
        error: None,
    })
}

/// Unpair a device
async fn unpair_device(
    state: web::Data<Arc<GatewayState>>,
    path: web::Path<String>,
) -> impl Responder {
    let client_id = path.into_inner();

    let mut clients = state.connected_clients.write().await;
    if clients.remove(&client_id).is_some() {
        state.increment_presence().await;
        HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "message": "Device unpaired"
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "error": "Client not found"
        }))
    }
}

/// OpenAI-compatible chat completions endpoint
async fn chat_completions(
    state: web::Data<Arc<GatewayState>>,
    body: web::Json<ChatCompletionRequest>,
) -> impl Responder {
    if !state.config.openai_api_enabled {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": {
                "message": "OpenAI API is disabled",
                "type": "api_error"
            }
        }));
    }

    // TODO: Forward to actual LLM backend
    // For now, return a mock response
    let response = ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion",
        created: chrono::Utc::now().timestamp(),
        model: body.model.clone(),
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: "This is a placeholder response from DX Gateway.".to_string(),
            },
            finish_reason: "stop".to_string(),
        }],
        usage: ChatUsage {
            prompt_tokens: 10,
            completion_tokens: 10,
            total_tokens: 20,
        },
    };

    HttpResponse::Ok().json(response)
}

/// List available models
async fn list_models(state: web::Data<Arc<GatewayState>>) -> impl Responder {
    // TODO: Query actual available models
    HttpResponse::Ok().json(serde_json::json!({
        "object": "list",
        "data": [
            {
                "id": "gpt-4",
                "object": "model",
                "owned_by": "openai"
            },
            {
                "id": "gpt-3.5-turbo",
                "object": "model",
                "owned_by": "openai"
            },
            {
                "id": "claude-3-opus",
                "object": "model",
                "owned_by": "anthropic"
            },
            {
                "id": "gemini-pro",
                "object": "model",
                "owned_by": "google"
            }
        ]
    }))
}

/// Gateway info endpoint
async fn gateway_info(state: web::Data<Arc<GatewayState>>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "name": "DX Gateway",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol_version": "2026.2",
        "features": {
            "control_ui": state.config.control_ui_enabled,
            "openai_api": state.config.openai_api_enabled,
            "open_responses": state.config.open_responses_enabled,
            "tls": state.config.tls_enabled,
            "pairing": true,
            "channels": ["whatsapp", "telegram", "discord", "slack", "signal", "imessage"]
        },
        "endpoints": {
            "ws": format!("ws://{}:{}/ws", state.config.bind_address, state.config.port),
            "http": format!("http://{}:{}", state.config.bind_address, state.config.port),
            "health": "/health",
            "pairing": "/api/pairing",
            "openai": "/v1"
        }
    }))
}

/// Channel status endpoint
async fn channel_status(state: web::Data<Arc<GatewayState>>) -> impl Responder {
    // TODO: Query actual channel status from bridge
    HttpResponse::Ok().json(serde_json::json!({
        "channels": [
            {
                "id": "whatsapp",
                "status": "disconnected",
                "authenticated": false
            },
            {
                "id": "telegram",
                "status": "disconnected",
                "authenticated": false
            },
            {
                "id": "discord",
                "status": "disconnected",
                "authenticated": false
            },
            {
                "id": "slack",
                "status": "disconnected",
                "authenticated": false
            },
            {
                "id": "signal",
                "status": "disconnected",
                "authenticated": false
            },
            {
                "id": "imessage",
                "status": "unavailable",
                "authenticated": false,
                "reason": "macOS only"
            }
        ]
    }))
}

// ============================================================================
// WebSocket Handler
// ============================================================================

/// WebSocket upgrade handler
async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<Arc<GatewayState>>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    // Spawn WebSocket handling task
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = futures_util::StreamExt::next(&mut msg_stream).await {
            match msg {
                WsMessage::Text(text) => {
                    // Parse and handle gateway message
                    if let Ok(ws_request) = serde_json::from_str::<WsRequest>(&text) {
                        let response = handle_ws_request(&state, ws_request).await;
                        let _ = session.text(serde_json::to_string(&response).unwrap()).await;
                    }
                }
                WsMessage::Ping(bytes) => {
                    let _ = session.pong(&bytes).await;
                }
                WsMessage::Close(_) => break,
                _ => {}
            }
        }
    });

    Ok(res)
}

/// Handle incoming gateway WebSocket request
async fn handle_ws_request(state: &Arc<GatewayState>, req: WsRequest) -> WsResponse {
    match req.method.as_str() {
        "health.check" => WsResponse {
            id: req.id,
            result: Some(serde_json::json!({
                "status": "healthy"
            })),
            error: None,
        },
        "clients.list" => {
            let clients = state.connected_clients.read().await;
            let list: Vec<_> = clients.values().cloned().collect();
            WsResponse {
                id: req.id,
                result: Some(serde_json::to_value(list).unwrap()),
                error: None,
            }
        }
        "channels.list" => WsResponse {
            id: req.id,
            result: Some(serde_json::json!([
                "whatsapp", "telegram", "discord", "slack", "signal", "imessage"
            ])),
            error: None,
        },
        "channels.status" => WsResponse {
            id: req.id,
            result: Some(serde_json::json!({
                "whatsapp": {"connected": false},
                "telegram": {"connected": false},
                "discord": {"connected": false}
            })),
            error: None,
        },
        _ => WsResponse {
            id: req.id,
            result: None,
            error: Some(serde_json::json!({
                "code": "METHOD_NOT_FOUND",
                "message": format!("Unknown method: {}", req.method)
            })),
        },
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a secure auth token
fn generate_auth_token() -> String {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

// ============================================================================
// Server Startup
// ============================================================================

/// Start the Actix Web gateway server
pub async fn start_actix_gateway(config: GatewayConfig) -> std::io::Result<()> {
    let bind_addr = format!("{}:{}", config.bind_address, config.port);
    let state = Arc::new(GatewayState::new(config.clone()));

    tracing::info!("Starting DX Gateway on http://{}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        let mut app = App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // Health & Info
            .route("/health", web::get().to(health_check))
            .route("/info", web::get().to(gateway_info))
            // WebSocket
            .route("/ws", web::get().to(ws_handler))
            // Pairing API
            .route("/api/pairing/generate", web::post().to(generate_pairing_code))
            .route("/api/pairing/pair", web::post().to(pair_device))
            .route("/api/pairing/unpair/{client_id}", web::delete().to(unpair_device))
            .route("/api/clients", web::get().to(list_clients))
            // Channel API
            .route("/api/channels/status", web::get().to(channel_status))
            // OpenAI-compatible API
            .route("/v1/chat/completions", web::post().to(chat_completions))
            .route("/v1/models", web::get().to(list_models));

        // Serve Control UI if enabled
        if state.config.control_ui_enabled {
            if let Some(ref ui_path) = state.config.control_ui_path {
                app = app.service(Files::new("/ui", ui_path).index_file("index.html"));
            }
        }

        app
    })
    .bind(&bind_addr)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default();
        assert_eq!(config.port, 31337);
        assert_eq!(config.bind_address, "127.0.0.1");
        assert!(config.control_ui_enabled);
    }

    #[test]
    fn test_generate_auth_token() {
        let token = generate_auth_token();
        assert_eq!(token.len(), 64); // 32 bytes = 64 hex chars
    }
}
