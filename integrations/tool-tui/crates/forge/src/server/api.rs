use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path as AxumPath, Query},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use colored::*;
use futures::{SinkExt, StreamExt};
use tower_http::cors::{Any, CorsLayer};

use crate::crdt::Operation;
use crate::server::authentication::{
    AuthManager, ChangePasswordRequest, CreateUserRequest, LoginRequest, LoginResponse,
};
use crate::storage::{Blob, Database, OperationLog, R2Config, R2Storage};
use crate::sync::{GLOBAL_CLOCK, SyncManager, SyncMessage};
use dashmap::DashSet;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub oplog: Arc<OperationLog>,
    pub db: Arc<Database>,
    pub sync: SyncManager,
    pub actor_id: String,
    pub repo_id: String,
    pub seen: Arc<DashSet<Uuid>>,
    pub r2: Option<Arc<R2Storage>>, // R2 storage for blobs
    pub auth: Arc<AuthManager>,     // Authentication manager
}

/// API error type
#[derive(Debug)]
pub enum ApiError {
    Internal(anyhow::Error),
    NotFound(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        let body = Json(serde_json::json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        ApiError::Internal(err.into())
    }
}

/// Blob upload request
#[derive(Debug, Deserialize)]
pub struct UploadBlobRequest {
    pub path: String,
    pub content: String, // Base64 encoded
}

/// Blob upload response
#[derive(Debug, Serialize)]
pub struct UploadBlobResponse {
    pub hash: String,
    pub key: String,
    pub size: u64,
}

pub async fn serve(port: u16, path: PathBuf) -> Result<()> {
    // Initialize DB/oplog
    let forge_path = path.join(".dx/forge");
    let db = Arc::new(Database::new(&forge_path)?);
    db.initialize()?;
    let oplog = Arc::new(OperationLog::new(db.clone())?);

    // Load actor/repo identifiers
    let config_path = forge_path.join("config.json");
    let default_repo_id = {
        let mut hasher = Sha256::new();
        let path_string = forge_path.to_string_lossy().into_owned();
        hasher.update(path_string.as_bytes());
        format!("repo-{:x}", hasher.finalize())
    };

    let (actor_id, repo_id) = if let Ok(bytes) = tokio::fs::read(&config_path).await {
        if let Ok(cfg) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            let actor = cfg
                .get("actor_id")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(whoami::username);
            let repo = cfg
                .get("repo_id")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| default_repo_id.clone());
            (actor, repo)
        } else {
            (whoami::username(), default_repo_id.clone())
        }
    } else {
        (whoami::username(), default_repo_id)
    };

    // Try to load R2 config
    let r2 = match R2Config::from_env() {
        Ok(config) => {
            println!("{} R2 Bucket: {}", "✓".green(), config.bucket_name.bright_white());
            match R2Storage::new(config) {
                Ok(storage) => {
                    println!("{} R2 Storage enabled", "✓".green());
                    Some(Arc::new(storage))
                }
                Err(e) => {
                    println!("{} R2 Storage failed: {}", "⚠".yellow(), e);
                    None
                }
            }
        }
        Err(_) => {
            println!("{} R2 not configured (set R2_* in .env for blob storage)", "ℹ".blue());
            None
        }
    };

    let state = AppState {
        oplog,
        db,
        sync: SyncManager::new(),
        actor_id,
        repo_id,
        seen: Arc::new(DashSet::new()),
        r2,
        auth: Arc::new(AuthManager::new()),
    };

    let app = Router::new()
        // Static files for web UI
        .route("/", get(serve_index))
        .route("/static/styles.css", get(serve_styles))
        .route("/static/app.js", get(serve_app_js))
        // API routes
        .route("/health", get(health_check))
        .route("/ops", get(get_ops))
        .route("/ws", get(ws_handler))
        // Authentication endpoints
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/validate", get(validate_session))
        .route("/api/v1/auth/me", get(get_current_user))
        .route("/api/v1/auth/change-password", post(change_password))
        // User management endpoints
        .route("/api/v1/users", get(list_users))
        .route("/api/v1/users", post(create_user))
        .route("/api/v1/users/{username}", delete(delete_user))
        // File browser endpoints
        .route("/api/v1/files", get(list_files))
        .route("/api/v1/files/{*path}", get(get_file_content))
        // Blob endpoints (if R2 is configured)
        .route("/api/v1/blobs", post(upload_blob))
        .route("/api/v1/blobs/{hash}", get(download_blob))
        .route("/api/v1/blobs/{hash}", delete(delete_blob_handler))
        .route("/api/v1/blobs/{hash}/exists", get(check_blob_exists))
        .route("/api/v1/blobs/batch", post(batch_upload))
        // CORS for web clients
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    println!(
        "{} Server running at {}",
        "✓".green(),
        format!("http://localhost:{}", port).bright_blue()
    );

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(state, socket))
}

async fn handle_ws(state: AppState, socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    // Send handshake immediately with server metadata
    let handshake = SyncMessage::handshake(state.actor_id.clone(), state.repo_id.clone());
    if let Ok(text) = serde_json::to_string(&handshake) {
        let _ = sender.send(Message::Text(text.into())).await;
    }

    // Subscribe to local operations and forward to this client
    let mut rx = state.sync.subscribe();
    let send_task = tokio::spawn(async move {
        while let Ok(op_arc) = rx.recv().await {
            // Forward as JSON text
            if let Ok(text) = serde_json::to_string(&SyncMessage::operation((*op_arc).clone())) {
                if sender.send(Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Receive from client and publish
    let state_recv = state.clone();
    let recv_task = tokio::spawn(async move {
        let oplog = state_recv.oplog.clone();
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let text: String = text.to_string();
                    if let Ok(msg) = serde_json::from_str::<SyncMessage>(&text) {
                        match msg {
                            SyncMessage::Handshake { actor_id, repo_id } => {
                                println!(
                                    "{} Peer handshake: actor={} repo={}",
                                    "↔".bright_blue(),
                                    actor_id.bright_yellow(),
                                    repo_id.bright_white()
                                );
                            }
                            SyncMessage::Operation { operation: op } => {
                                if insert_seen(&state_recv.seen, op.id) {
                                    if let Some(lamport) = op.lamport() {
                                        GLOBAL_CLOCK.observe(lamport);
                                    }
                                    let _ = oplog.append(op.clone());
                                    let _ = state_recv.sync.publish(Arc::new(op));
                                }
                            }
                        }
                    } else if let Ok(op) = serde_json::from_str::<Operation>(&text) {
                        if insert_seen(&state_recv.seen, op.id) {
                            if let Some(lamport) = op.lamport() {
                                GLOBAL_CLOCK.observe(lamport);
                            }
                            let _ = oplog.append(op.clone());
                            let _ = state_recv.sync.publish(Arc::new(op));
                        }
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if let Ok(op) = serde_cbor::from_slice::<Operation>(&bin) {
                        if insert_seen(&state_recv.seen, op.id) {
                            if let Some(lamport) = op.lamport() {
                                GLOBAL_CLOCK.observe(lamport);
                            }
                            let _ = oplog.append(op.clone());
                            let _ = state_recv.sync.publish(Arc::new(op));
                        }
                    }
                }
                Ok(Message::Close(_)) | Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {}
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(send_task, recv_task);
}

#[derive(Deserialize)]
struct OpsQuery {
    file: Option<String>,
    limit: Option<usize>,
}

async fn get_ops(
    State(state): State<AppState>,
    Query(query): Query<OpsQuery>,
) -> Result<Json<Vec<Operation>>, axum::http::StatusCode> {
    let limit = query.limit.unwrap_or(50);
    let result = if let Some(file) = query.file.as_deref() {
        let p = std::path::PathBuf::from(file);
        state.db.get_operations(Some(&p), limit)
    } else {
        state.db.get_operations(None, limit)
    };

    match result {
        Ok(ops) => Ok(Json(ops)),
        Err(_) => Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

const SEEN_LIMIT: usize = 10_000;

fn insert_seen(cache: &DashSet<Uuid>, id: Uuid) -> bool {
    let inserted = cache.insert(id);
    if inserted {
        enforce_seen_limit(cache);
    }
    inserted
}

fn enforce_seen_limit(cache: &DashSet<Uuid>) {
    while cache.len() > SEEN_LIMIT {
        if let Some(entry) = cache.iter().next() {
            let key = *entry.key();
            drop(entry);
            cache.remove(&key);
        } else {
            break;
        }
    }
}

// ========== Blob Storage Endpoints ==========

/// Health check endpoint with R2 status
async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "forge-api",
        "version": env!("CARGO_PKG_VERSION"),
        "r2_enabled": state.r2.is_some(),
    }))
}

/// Upload blob endpoint
async fn upload_blob(
    State(state): State<AppState>,
    Json(req): Json<UploadBlobRequest>,
) -> Result<Json<UploadBlobResponse>, ApiError> {
    let r2 = state
        .r2
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("R2 storage not configured".to_string()))?;

    // Decode base64 content
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD
        .decode(&req.content)
        .map_err(|e| ApiError::BadRequest(format!("Invalid base64: {}", e)))?;

    let blob = Blob::from_content(&req.path, content);
    let hash = blob.hash().to_string();
    let size = blob.metadata.size;

    // Upload to R2
    let key = r2.upload_blob(&blob).await?;

    Ok(Json(UploadBlobResponse { hash, key, size }))
}

/// Download blob endpoint
async fn download_blob(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Response, ApiError> {
    let r2 = state
        .r2
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("R2 storage not configured".to_string()))?;

    let blob = r2
        .download_blob(&hash)
        .await
        .map_err(|_| ApiError::NotFound(format!("Blob not found: {}", hash)))?;

    // Return blob content with metadata headers
    Ok((
        StatusCode::OK,
        [
            ("Content-Type", blob.metadata.mime_type.clone()),
            ("X-Blob-Hash", hash),
            ("X-Blob-Size", blob.metadata.size.to_string()),
        ],
        blob.content,
    )
        .into_response())
}

/// Delete blob endpoint
async fn delete_blob_handler(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let r2 = state
        .r2
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("R2 storage not configured".to_string()))?;

    r2.delete_blob(&hash).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Check if blob exists
async fn check_blob_exists(
    State(state): State<AppState>,
    AxumPath(hash): AxumPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let r2 = state
        .r2
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("R2 storage not configured".to_string()))?;

    let exists = r2.blob_exists(&hash).await?;

    Ok(Json(serde_json::json!({
        "exists": exists,
        "hash": hash,
    })))
}

/// Batch upload request
#[derive(Debug, Deserialize)]
pub struct BatchUploadRequest {
    pub blobs: Vec<UploadBlobRequest>,
}

/// Batch upload response
#[derive(Debug, Serialize)]
pub struct BatchUploadResponse {
    pub uploaded: Vec<UploadBlobResponse>,
    pub failed: Vec<String>,
}

/// Batch upload endpoint
async fn batch_upload(
    State(state): State<AppState>,
    Json(req): Json<BatchUploadRequest>,
) -> Result<Json<BatchUploadResponse>, ApiError> {
    let r2 = state
        .r2
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("R2 storage not configured".to_string()))?;

    let mut uploaded = Vec::new();
    let mut failed = Vec::new();

    use base64::Engine;
    for blob_req in req.blobs {
        match base64::engine::general_purpose::STANDARD.decode(&blob_req.content) {
            Ok(content) => {
                let blob = Blob::from_content(&blob_req.path, content);
                let hash = blob.hash().to_string();
                let size = blob.metadata.size;

                match r2.upload_blob(&blob).await {
                    Ok(key) => {
                        uploaded.push(UploadBlobResponse { hash, key, size });
                    }
                    Err(e) => {
                        failed.push(format!("{}: {}", blob_req.path, e));
                    }
                }
            }
            Err(e) => {
                failed.push(format!("{}: Invalid base64: {}", blob_req.path, e));
            }
        }
    }

    Ok(Json(BatchUploadResponse { uploaded, failed }))
}

// ========== Static File Serving ==========

const INDEX_HTML: &str = include_str!("web_ui/index.html");
const STYLES_CSS: &str = include_str!("web_ui/styles.css");
const APP_JS: &str = include_str!("web_ui/app.js");

async fn serve_index() -> impl IntoResponse {
    (StatusCode::OK, [("Content-Type", "text/html; charset=utf-8")], INDEX_HTML)
}

async fn serve_styles() -> impl IntoResponse {
    (StatusCode::OK, [("Content-Type", "text/css; charset=utf-8")], STYLES_CSS)
}

async fn serve_app_js() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("Content-Type", "application/javascript; charset=utf-8")],
        APP_JS,
    )
}

// ========== Authentication Endpoints ==========

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    let session = state.auth.login(&req.username, &req.password)?;

    Ok(Json(LoginResponse {
        token: session.token,
        username: session.username,
        role: session.role,
        expires_at: session.expires_at,
    }))
}

async fn validate_session(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, ApiError> {
    let token = extract_token(&headers)?;
    state.auth.validate_token(&token)?;
    Ok(StatusCode::OK)
}

async fn get_current_user(
    State(_state): State<AppState>,
    _headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Return dummy user for no-auth mode
    Ok(Json(serde_json::json!({
        "username": "guest",
        "role": "admin",
        "user_id": "guest-id",
    })))
}

async fn change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, ApiError> {
    let token = extract_token(&headers)?;
    let session = state.auth.validate_token(&token)?;

    state
        .auth
        .update_password(&session.username, &req.old_password, &req.new_password)?;
    Ok(StatusCode::OK)
}

fn extract_token(headers: &axum::http::HeaderMap) -> Result<String, ApiError> {
    let auth_header = headers
        .get("Authorization")
        .ok_or_else(|| ApiError::BadRequest("Missing Authorization header".to_string()))?
        .to_str()
        .map_err(|_| ApiError::BadRequest("Invalid Authorization header".to_string()))?;

    auth_header
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::BadRequest("Invalid Authorization format".to_string()))
}

// ========== User Management Endpoints ==========

async fn list_users(
    State(state): State<AppState>,
    _headers: axum::http::HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    // let token = extract_token(&headers)?;
    // let session = state.auth.validate_token(&token)?;

    // Only allow admins and developers to list users
    // if session.role != crate::server::authentication::Role::Admin {
    //     return Err(ApiError::BadRequest("Insufficient permissions".to_string()));
    // }

    let users = state.auth.list_users();
    let user_list: Vec<_> = users
        .into_iter()
        .map(|u| {
            serde_json::json!({
                "username": u.username,
                "email": u.email,
                "role": u.role,
                "created_at": u.created_at,
            })
        })
        .collect();

    Ok(Json(user_list))
}

async fn create_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Result<StatusCode, ApiError> {
    let token = extract_token(&headers)?;
    let session = state.auth.validate_token(&token)?;

    // Only admins can create users
    if session.role != crate::server::authentication::Role::Admin {
        return Err(ApiError::BadRequest("Insufficient permissions".to_string()));
    }

    state.auth.register(req.username, &req.password, req.role)?;
    Ok(StatusCode::CREATED)
}

async fn delete_user(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    AxumPath(username): AxumPath<String>,
) -> Result<StatusCode, ApiError> {
    let token = extract_token(&headers)?;
    let session = state.auth.validate_token(&token)?;

    // Only admins can delete users
    if session.role != crate::server::authentication::Role::Admin {
        return Err(ApiError::BadRequest("Insufficient permissions".to_string()));
    }

    // Prevent self-deletion
    if username == session.username {
        return Err(ApiError::BadRequest("Cannot delete your own account".to_string()));
    }

    state.auth.delete_user(&username)?;
    Ok(StatusCode::NO_CONTENT)
}

// ========== File Browser Endpoints ==========

#[derive(Serialize)]
struct FileInfo {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<u64>,
}

async fn list_files(
    State(_state): State<AppState>,
    _headers: axum::http::HeaderMap,
) -> Result<Json<Vec<FileInfo>>, ApiError> {
    // No auth check needed

    // List files in current directory
    let current_dir = std::env::current_dir().map_err(|e| ApiError::Internal(e.into()))?;

    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&current_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Unknown").to_string();

            // Skip hidden files and forge directory
            if name.starts_with('.') {
                continue;
            }

            let is_dir = path.is_dir();
            let size = if is_dir {
                None
            } else {
                std::fs::metadata(&path).ok().map(|m| m.len())
            };

            files.push(FileInfo {
                name,
                path: path
                    .strip_prefix(&current_dir)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or("")
                    .to_string(),
                is_dir,
                size,
            });
        }
    }

    // Sort: directories first, then by name
    files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(Json(files))
}

#[derive(Serialize)]
struct FileContentResponse {
    content: String,
    path: String,
}

async fn get_file_content(
    State(_state): State<AppState>,
    _headers: axum::http::HeaderMap,
    AxumPath(path): AxumPath<String>,
) -> Result<Json<FileContentResponse>, ApiError> {
    // No auth check needed

    println!("DEBUG: Requested file path: {}", path);

    let current_dir = std::env::current_dir().map_err(|e| ApiError::Internal(e.into()))?;

    let file_path = current_dir.join(&path);
    println!("DEBUG: Resolved file path: {:?}", file_path);

    // Security check: ensure path doesn't escape current directory
    if !file_path.starts_with(&current_dir) {
        return Err(ApiError::BadRequest("Invalid file path".to_string()));
    }

    // Check if file exists and is not a directory
    if !file_path.exists() {
        return Err(ApiError::NotFound(format!("File not found: {}", path)));
    }

    if file_path.is_dir() {
        return Err(ApiError::BadRequest("Path is a directory".to_string()));
    }

    // Read file content
    let content = std::fs::read_to_string(&file_path).map_err(|e| ApiError::Internal(e.into()))?;

    Ok(Json(FileContentResponse { content, path }))
}
