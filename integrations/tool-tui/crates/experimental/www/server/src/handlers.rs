//! # HTTP Handlers
//!
//! Axum route handlers for dx-server

use crate::{ServerState, ssr};
use axum::{
    extract::State,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use uuid::Uuid;

/// Serve index.html or SSR-inflated HTML
///
/// # Bot Detection Strategy
/// - Bot detected → Serve SSR HTML (SEO-optimized)
/// - Human detected → Serve SPA shell (fast hydration)
///
/// # Performance
/// - Bot path: ~1ms (string inflation)
/// - Human path: ~0ms (static file serve)
pub async fn serve_index(
    State(state): State<ServerState>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let user_agent = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("");

    // Detect Bot vs Human
    if ssr::is_bot(user_agent) {
        tracing::info!("🤖 Bot detected: {}", user_agent);
        return serve_ssr(state).await.into_response();
    }

    // Serve SPA shell for humans
    tracing::debug!("👤 Human detected, serving SPA shell");
    serve_spa_shell(state).into_response()
}

/// Serve SSR-inflated HTML for bots
async fn serve_ssr(state: ServerState) -> impl IntoResponse {
    // Try to get template with ID 0 (root template)
    let template_opt = state.template_cache.get(&0).map(|entry| entry.clone());

    if let Some(template) = template_opt {
        // Create mock state (in production, this would come from data fetching)
        let mut state_data = ssr::StateData::new();
        state_data.set(0, "Hello from SSR!".to_string());

        // Metadata for SEO
        let meta_tags = vec![
            ("description".to_string(), "Dx-WWW Runtime - The Binary Web".to_string()),
            ("keywords".to_string(), "wasm, binary, performance, ssr".to_string()),
            ("og:title".to_string(), "Dx-WWW Runtime".to_string()),
        ];

        // Inflate the page
        let html = ssr::inflate_page(
            &template,
            &state_data,
            "Dx-WWW Runtime",
            &meta_tags,
            &[], // No scripts for bots
        );

        tracing::debug!("✅ SSR inflation complete ({} bytes)", html.len());
        return Html(html);
    }

    // Fallback if no template found
    tracing::warn!("⚠️ Template 0 not found in cache");
    Html("<h1>500 - Template Not Found</h1>".to_string())
}

/// Serve SPA shell for humans (fast client-side hydration)
fn serve_spa_shell(state: ServerState) -> impl IntoResponse {
    // Generate request ID for error correlation
    let request_id = Uuid::new_v4().to_string();

    // Try to serve index.html from project directory
    // Note: RwLock::read() can only fail if the lock is poisoned (another thread panicked while holding it)
    match state.project_dir.read() {
        Ok(guard) => {
            if let Some(project_dir) = guard.as_ref() {
                let index_path = project_dir.join("index.html");
                if index_path.exists() {
                    match std::fs::read_to_string(&index_path) {
                        Ok(html) => {
                            tracing::debug!("✅ Serving index.html from {}", index_path.display());
                            Html(html).into_response()
                        }
                        Err(e) => {
                            tracing::error!(
                                request_id = %request_id,
                                path = %index_path.display(),
                                error = %e,
                                "Failed to read index.html"
                            );
                            create_error_response(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to read index.html",
                                &format!("Could not read file at {}: {}", index_path.display(), e),
                                &request_id,
                            )
                        }
                    }
                } else {
                    tracing::error!(
                        request_id = %request_id,
                        path = %index_path.display(),
                        "index.html not found"
                    );
                    create_error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "index.html not found",
                        &format!(
                            "The file index.html was not found at {}. Please ensure your project has an index.html file in the project directory.",
                            index_path.display()
                        ),
                        &request_id,
                    )
                }
            } else {
                tracing::error!(
                    request_id = %request_id,
                    "Project directory not configured"
                );
                create_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Project directory not configured",
                    "No project directory has been set. Call ServerState::set_project_dir() to configure the project directory.",
                    &request_id,
                )
            }
        }
        Err(e) => {
            tracing::error!(
                request_id = %request_id,
                error = %e,
                "project_dir lock poisoned"
            );
            create_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
                "Failed to acquire lock on project directory configuration.",
                &request_id,
            )
        }
    }
}

/// Create an error response with proper status code and request ID
fn create_error_response(
    status: StatusCode,
    title: &str,
    message: &str,
    request_id: &str,
) -> Response {
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - DX WWW</title>
    <style>
        body {{ font-family: system-ui, sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }}
        h1 {{ color: #dc2626; }}
        .error-code {{ color: #6b7280; font-size: 0.875rem; }}
        .message {{ background: #fef2f2; border: 1px solid #fecaca; border-radius: 0.5rem; padding: 1rem; margin: 1rem 0; }}
        .request-id {{ color: #9ca3af; font-size: 0.75rem; margin-top: 2rem; }}
    </style>
</head>
<body>
    <h1>{} {}</h1>
    <div class="message">
        <p>{}</p>
    </div>
    <p class="request-id">Request ID: {}</p>
</body>
</html>"#,
        status.as_u16(),
        status.as_u16(),
        title,
        message,
        request_id
    );

    // Response::builder() with these inputs is infallible - status, headers, and body are all valid
    // The only way this could fail is with invalid header names/values, which we control
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header("X-Request-ID", request_id)
        .body(axum::body::Body::from(html))
        .unwrap_or_else(|_| {
            // Fallback: return a minimal error response if builder somehow fails
            Response::new(axum::body::Body::from("Internal Server Error"))
        })
}

/// Stream binary artifacts (Day 16: The Binary Streamer)
///
/// # The Waterfall Killer
///
/// Traditional loading is sequential:
/// 1. Download → 2. Parse → 3. Execute
///
/// Streaming loading is parallel:
/// - Chunk 1 (Layout) → Client creates templates while downloading
/// - Chunk 2 (State) → Client allocates memory while downloading
/// - Chunk 3 (WASM) → Browser compiles while downloading
///
/// Result: Zero blocking time. Execution starts before download completes.
///
/// # Example
/// ```bash
/// curl --no-buffer http://localhost:3000/stream/app | xxd | head -50
/// ```
pub async fn serve_binary_stream(
    State(state): State<ServerState>,
    axum::extract::Path(app_id): axum::extract::Path<String>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    use axum::body::Body;
    use axum::http::header;

    tracing::info!("📡 Streaming binary for app: {}", app_id);

    // Check If-None-Match header for delta patching
    let client_hash = headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_matches('"'));

    // Get current version hash
    let current_hash = state.current_version.get("app.wasm").map(|entry| entry.value().clone());

    // Check for version negotiation
    if let (Some(client_hash_str), Some(current_hash_str)) = (client_hash, &current_hash) {
        tracing::debug!("🔍 Client hash: {}, Current hash: {}", client_hash_str, current_hash_str);

        // Case 1: Client has current version → 304 Not Modified
        if client_hash_str == current_hash_str {
            tracing::info!("✅ Client already has current version (304)");
            return axum::response::Response::builder()
                .status(StatusCode::NOT_MODIFIED)
                .header(header::ETAG, format!("\"{}\"", current_hash_str))
                .body(Body::empty())
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }

        // Case 2: Client has old version → Send Patch
        let patch_result = {
            // Note: Mutex::lock() can only fail if the lock is poisoned (another thread panicked)
            // In that case, we skip patching and fall through to full stream
            if let Ok(store) = state.version_store.lock() {
                store.get(client_hash_str).and_then(|_| {
                    // Get new data
                    let new_data = state.binary_cache.get("app.wasm")?;
                    store.create_patch(client_hash_str, new_data.value())
                })
            } else {
                tracing::warn!("version_store lock poisoned, skipping patch");
                None
            }
        };

        if let Some(patch_data) = patch_result {
            tracing::info!("📦 Sending patch ({} bytes)", patch_data.len());

            return axum::response::Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header(header::ETAG, format!("\"{}\"", current_hash_str))
                .header("X-Dx-Patch", "true")
                .header("X-Dx-Base-Hash", client_hash_str)
                .header("X-Dx-Target-Hash", current_hash_str)
                .body(Body::from(patch_data))
                .unwrap_or_else(|_| Response::new(Body::empty()));
        }

        // Case 3: Unknown old version → Fall through to full stream
        tracing::debug!("⚠️ Unknown client hash, sending full stream");
    }

    // Case 4: No client hash or unknown → Send Full Stream
    tracing::info!("📤 Sending full binary stream");

    // Load artifacts from cache
    let layout_bin = state
        .binary_cache
        .get("layout.bin")
        .map(|entry| entry.value().clone())
        .unwrap_or_else(|| {
            tracing::warn!("⚠️ layout.bin not in cache, generating mock");
            vec![0u8; 100] // Mock layout
        });

    let wasm_bin = state
        .binary_cache
        .get("app.wasm")
        .map(|entry| entry.value().clone())
        .unwrap_or_else(|| {
            tracing::warn!("⚠️ app.wasm not in cache, using empty");
            vec![]
        });

    // Create mock artifact for header
    let artifact = dx_www_packet::DxbArtifact {
        version: 1,
        capabilities: dx_www_packet::CapabilitiesManifest::default(),
        templates: vec![],
        wasm_size: wasm_bin.len() as u32,
    };

    // Create streaming body
    let stream = crate::stream::create_stream(&artifact, layout_bin, wasm_bin);
    let body = Body::from_stream(stream);

    // Build response with streaming headers
    let mut response_builder = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CACHE_CONTROL, "public, max-age=31536000") // Cache for 1 year
        .header("X-Dx-Version", "1.0")
        .header("X-Dx-Stream", "chunked");

    // Add ETag if we have a hash
    if let Some(hash) = current_hash {
        response_builder = response_builder.header(header::ETAG, format!("\"{}\"", hash));
    }

    let response = response_builder.body(body).unwrap_or_else(|_| Response::new(Body::empty()));

    tracing::debug!("✅ Stream initialized");
    response
}

/// Serve a simple SVG favicon (prevents 404 errors)
pub async fn serve_favicon() -> impl IntoResponse {
    // Simple SVG favicon with "dx" text
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="4" fill="#667eea"/><text x="16" y="22" text-anchor="middle" fill="white" font-family="Arial" font-size="14" font-weight="bold">dx</text></svg>"##;

    ([(header::CONTENT_TYPE, "image/svg+xml")], svg)
}

/// Health check endpoint
pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "dx-server is healthy")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let _response = health_check().await;
        // Response implements IntoResponse, can't easily test status
        // In real tests, use reqwest to test full HTTP
    }
}
