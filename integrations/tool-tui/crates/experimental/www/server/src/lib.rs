//! # dx-server: The Holographic Server
//!
//! High-performance SSR & Edge Runtime for dx-www
//!
//! **Role:** Serve Binary Snapshots, Handle SSR Inflation (SEO), Manage State
//! **Philosophy:** "Write TSX, Serve Binary"

// Clippy lint configuration for server crate
#![allow(clippy::vec_init_then_push)] // Intentional pattern for building response vectors
#![allow(clippy::doc_nested_refdefs)] // Documentation style choice

//! ## Architecture
//!
//! ```text
//! ┌──────────────┐
//! │  TSX Files   │
//! └──────┬───────┘
//!        ↓
//! ┌──────────────┐
//! │ dx-compiler  │
//! └──────┬───────┘
//!        ↓
//! ┌──────────────┐
//! │  .dxb Files  │ ← Binary Format
//! └──────┬───────┘
//!        ↓
//! ┌──────────────┐
//! │  dx-server   │ ← YOU ARE HERE
//! └──────┬───────┘
//!        ↓
//! ┌──────────────┐
//! │   Browser    │
//! │  (dx-cache)  │
//! └──────────────┘
//! ```

pub mod csrf;
pub mod delta;
pub mod error_handler;
pub mod error_pages;
mod handlers;
pub mod memory_optimized;
pub mod ops;
pub mod rate_limiter;
pub mod request_context;
pub mod security_headers;
mod ssr;
pub mod stream;

// Ecosystem integrations
pub mod ecosystem;

// Ecosystem handlers (feature-gated)
#[cfg(feature = "query")]
pub mod rpc_handler;

#[cfg(feature = "auth")]
pub mod auth_middleware;

#[cfg(feature = "sync")]
pub mod ws_handler;

pub use handlers::*;
pub use ssr::*;

use axum::{Router, routing::get};

use dashmap::DashMap;
use dx_www_packet::Template;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

// Re-export production ops types for convenience
pub use ops::{
    CheckResult, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerError, CircuitState,
    GracefulShutdown, HealthCheck, HealthChecker, HealthState, HealthStatus, ShutdownConfig,
    ShutdownError, liveness_handler, readiness_handler,
};

/// Global server state shared across all request handlers.
///
/// `ServerState` manages all runtime data for the dx-server, including:
/// - Binary artifact caching for fast serving
/// - Template caching for SSR rendering
/// - Version tracking for delta patching
///
/// # Thread Safety
///
/// All fields use thread-safe containers (`Arc`, `DashMap`, `Mutex`, `RwLock`)
/// and can be safely shared across async tasks.
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{ServerState, serve};
/// use std::net::SocketAddr;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let state = ServerState::new();
///     
///     // Load compiled artifacts
///     state.load_artifacts(Path::new("dist"))?;
///     
///     // Set project directory for static files
///     state.set_project_dir("my-app".into());
///     
///     // Start server
///     let addr: SocketAddr = "127.0.0.1:3000".parse()?;
///     serve(addr, state).await
/// }
/// ```
#[derive(Clone)]
pub struct ServerState {
    /// Binary snapshot cache (path -> bytes)
    pub binary_cache: Arc<DashMap<String, Vec<u8>>>,
    /// Template cache (id -> Template) - stores full Template structs
    pub template_cache: Arc<DashMap<u32, Template>>,
    /// Version storage for delta patching (hash -> binary data)
    /// Stores last 5 versions of each artifact for patch generation
    pub version_store: Arc<std::sync::Mutex<delta::VersionStore>>,
    /// Current version hash (artifact name -> hash)
    pub current_version: Arc<DashMap<String, String>>,
    /// Project directory for serving static files (index.html, etc.)
    pub project_dir: Arc<std::sync::RwLock<Option<std::path::PathBuf>>>,
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            binary_cache: Arc::new(DashMap::new()),
            template_cache: Arc::new(DashMap::new()),
            version_store: Arc::new(std::sync::Mutex::new(delta::VersionStore::new(5))),
            current_version: Arc::new(DashMap::new()),
            project_dir: Arc::new(std::sync::RwLock::new(None)),
        }
    }

    /// Load artifacts from build output directory
    pub fn load_artifacts(&self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("📦 Loading artifacts from {}", path.display());

        // Load templates.json (parsed templates)
        let templates_path = path.join("templates.json");
        if templates_path.exists() {
            let json_str = std::fs::read_to_string(&templates_path)?;
            let templates: Vec<Template> = serde_json::from_str(&json_str)?;

            tracing::info!("  ✓ Loaded {} templates", templates.len());

            // Populate cache with full Template structs
            for template in templates {
                self.template_cache.insert(template.id, template);
            }
        } else {
            tracing::warn!("  ⚠️ templates.json not found");
        }

        // Load layout.bin (raw binary for streaming)
        let layout_path = path.join("layout.bin");
        if layout_path.exists() {
            let bytes = std::fs::read(&layout_path)?;
            self.binary_cache.insert("layout.bin".to_string(), bytes.clone());

            // Store version for delta patching
            let hash = match self.version_store.lock() {
                Ok(mut store) => store.store(bytes),
                Err(poisoned) => {
                    tracing::warn!("version_store lock poisoned, recovering");
                    poisoned.into_inner().store(bytes.clone())
                }
            };
            self.current_version.insert("layout.bin".to_string(), hash.clone());
            if let Some(entry) = self.binary_cache.get("layout.bin") {
                tracing::debug!(
                    "  ✓ Cached layout.bin ({} bytes, hash: {})",
                    entry.len(),
                    &hash[..8]
                );
            }
        }

        // Load app.wasm
        let wasm_path = path.join("app.wasm");
        if wasm_path.exists() {
            let bytes = std::fs::read(&wasm_path)?;
            let size = bytes.len();
            self.binary_cache.insert("app.wasm".to_string(), bytes.clone());

            // Store version for delta patching
            let hash = match self.version_store.lock() {
                Ok(mut store) => store.store(bytes),
                Err(poisoned) => {
                    tracing::warn!("version_store lock poisoned, recovering");
                    poisoned.into_inner().store(bytes.clone())
                }
            };
            self.current_version.insert("app.wasm".to_string(), hash.clone());
            tracing::info!("  ✓ Loaded app.wasm ({} bytes, hash: {})", size, &hash[..8]);
        }

        Ok(())
    }

    /// Register a template manually (for testing or dynamic loading)
    pub fn register_template(&self, template: Template) {
        let id = template.id;
        self.template_cache.insert(id, template);
        tracing::debug!("📄 Registered template {}", id);
    }

    /// Set project directory for serving static files
    pub fn set_project_dir(&self, dir: std::path::PathBuf) {
        tracing::info!("📁 Project directory: {}", dir.display());
        match self.project_dir.write() {
            Ok(mut guard) => *guard = Some(dir),
            Err(poisoned) => {
                tracing::warn!("project_dir lock poisoned, recovering");
                *poisoned.into_inner() = Some(dir);
            }
        }
    }
}

/// Build the Axum router with all routes and middleware.
///
/// Creates a fully configured router with:
/// - `/` - Root index with bot detection and SSR support
/// - `/health` - Health check endpoint (backward compatibility)
/// - `/health/live` - Liveness probe endpoint (Requirement 3.4)
/// - `/health/ready` - Readiness probe endpoint (Requirement 3.3)
/// - `/favicon.ico` - Favicon serving
/// - `/stream/:app_id` - Binary streaming endpoint
///
/// Optional ecosystem routes (feature-gated):
/// - `/api/rpc` - RPC handler (requires `query` feature)
/// - `/api/auth/login` - Auth handler (requires `auth` feature)
/// - `/ws` - WebSocket handler (requires `sync` feature)
///
/// # Middleware Stack
///
/// The router includes the following middleware (in order):
/// 1. Compression (gzip/deflate)
/// 2. CORS (permissive by default)
/// 3. Tracing (request logging)
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{ServerState, build_router};
/// use std::net::SocketAddr;
///
/// #[tokio::main]
/// async fn main() {
///     let state = ServerState::new();
///     let app = build_router(state);
///     
///     let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
///         .await
///         .unwrap();
///     axum::serve(listener, app).await.unwrap();
/// }
/// ```
pub fn build_router(state: ServerState) -> Router {
    // Create a default health checker for readiness probes
    let health_checker = Arc::new(HealthChecker::with_version(env!("CARGO_PKG_VERSION")));

    build_router_with_health_checker(state, health_checker)
}

/// Build the Axum router with a custom health checker.
///
/// This variant allows you to provide a custom `HealthChecker` with
/// application-specific health checks (e.g., database connectivity,
/// external service availability).
///
/// # Arguments
///
/// * `state` - Server state with loaded artifacts
/// * `health_checker` - Custom health checker with registered health checks
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{ServerState, build_router_with_health_checker, HealthChecker, HealthCheck, CheckResult};
/// use std::sync::Arc;
///
/// // Create custom health checker with database check
/// let mut checker = HealthChecker::with_version("1.0.0");
/// // checker.add_check(Box::new(DatabaseHealthCheck::new(pool)));
///
/// let state = ServerState::new();
/// let app = build_router_with_health_checker(state, Arc::new(checker));
/// ```
pub fn build_router_with_health_checker(
    state: ServerState,
    health_checker: Arc<HealthChecker>,
) -> Router {
    let mut router = Router::new()
        // Root index (supports bot detection + SSR)
        .route("/", get(handlers::serve_index))
        // Health check (backward compatibility)
        .route("/health", get(handlers::health_check))
        // Liveness probe - indicates if the process is alive (Requirement 3.4)
        .route("/health/live", get(ops::liveness_handler))
        // Favicon (prevent 404)
        .route("/favicon.ico", get(handlers::serve_favicon))
        // Binary streaming endpoint (Day 16: The Binary Streamer)
        .route("/stream/:app_id", get(handlers::serve_binary_stream));

    // Add readiness probe with health checker state (Requirement 3.3, 3.5)
    router = router.route("/health/ready", get(ops::readiness_handler).with_state(health_checker));

    // Add ecosystem routes
    #[cfg(feature = "query")]
    {
        use axum::routing::post;
        router = router.route("/api/rpc", post(rpc_handler::handle_rpc));
    }

    #[cfg(feature = "auth")]
    {
        use axum::routing::post;
        router = router
            .route("/api/auth/login", post(auth_middleware::handle_login))
            .route("/api/auth/refresh", post(auth_middleware::handle_refresh))
            .route("/api/auth/logout", post(auth_middleware::handle_logout));
    }

    #[cfg(feature = "sync")]
    {
        router = router.route("/ws", get(ws_handler::handle_ws_upgrade));
    }

    router
        // Add state
        .with_state(state)
        // Middleware
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

/// Start the dx-server on the specified address.
///
/// This is the main entry point for running the dx-server. It creates
/// a TCP listener, builds the router, and starts serving requests.
///
/// # Arguments
///
/// * `addr` - Socket address to bind to (e.g., `127.0.0.1:3000`)
/// * `state` - Server state with loaded artifacts
///
/// # Errors
///
/// Returns an error if:
/// - The TCP listener cannot bind to the address
/// - The server encounters a fatal error during operation
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{ServerState, serve};
/// use std::net::SocketAddr;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     // Initialize tracing for logging
///     tracing_subscriber::fmt::init();
///     
///     // Create and configure state
///     let state = ServerState::new();
///     state.load_artifacts(Path::new("dist"))?;
///     
///     // Start server
///     let addr: SocketAddr = "0.0.0.0:3000".parse()?;
///     println!("Starting server at http://{}", addr);
///     serve(addr, state).await
/// }
/// ```
pub async fn serve(addr: SocketAddr, state: ServerState) -> Result<(), Box<dyn std::error::Error>> {
    // Use default shutdown configuration (30 second timeout)
    serve_with_shutdown(addr, state, ShutdownConfig::default()).await
}

/// Start the dx-server with custom shutdown configuration.
///
/// This variant allows you to configure the graceful shutdown behavior,
/// including the timeout for in-flight requests.
///
/// # Arguments
///
/// * `addr` - Socket address to bind to
/// * `state` - Server state with loaded artifacts
/// * `shutdown_config` - Configuration for graceful shutdown behavior
///
/// # Graceful Shutdown (Requirements 3.1, 3.2)
///
/// When a SIGTERM or SIGINT signal is received:
/// 1. The server stops accepting new connections
/// 2. In-flight requests are allowed to complete (up to timeout)
/// 3. After timeout, remaining connections are forcefully terminated
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{ServerState, serve_with_shutdown, ShutdownConfig};
/// use std::net::SocketAddr;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let state = ServerState::new();
///     let addr: SocketAddr = "0.0.0.0:3000".parse()?;
///     
///     // Configure 60 second graceful shutdown timeout
///     let shutdown_config = ShutdownConfig::with_timeout(Duration::from_secs(60));
///     
///     serve_with_shutdown(addr, state, shutdown_config).await
/// }
/// ```
pub async fn serve_with_shutdown(
    addr: SocketAddr,
    state: ServerState,
    shutdown_config: ShutdownConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🚀 dx-server starting at {}", addr);

    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("✨ dx-server ready - The Holographic Server is online");
    tracing::info!("📦 Binary streaming enabled");
    tracing::info!("🔍 SEO inflation ready");
    tracing::info!("⚡ Delta patching active");
    tracing::info!("🏥 Health probes: /health/live, /health/ready");
    tracing::info!("⏱️  Graceful shutdown timeout: {:?}", shutdown_config.timeout);

    // Create graceful shutdown handler
    let shutdown = GracefulShutdown::new(shutdown_config.clone());

    // Serve with graceful shutdown support (Requirements 3.1, 3.2)
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.wait_for_signal().await;
            tracing::info!("🛑 Shutdown signal received, initiating graceful shutdown...");
            tracing::info!(
                "⏳ Waiting up to {:?} for in-flight requests to complete",
                shutdown_config.timeout
            );
        })
        .await?;

    tracing::info!("👋 dx-server shutdown complete");

    Ok(())
}

/// Start the dx-server with a custom health checker.
///
/// This variant allows you to provide a custom `HealthChecker` with
/// application-specific health checks for the readiness probe.
///
/// # Arguments
///
/// * `addr` - Socket address to bind to
/// * `state` - Server state with loaded artifacts
/// * `health_checker` - Custom health checker with registered health checks
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{ServerState, serve_with_health_checker, HealthChecker};
/// use std::net::SocketAddr;
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let state = ServerState::new();
///     let addr: SocketAddr = "0.0.0.0:3000".parse()?;
///     
///     // Create custom health checker
///     let checker = Arc::new(HealthChecker::with_version("1.0.0"));
///     
///     serve_with_health_checker(addr, state, checker).await
/// }
/// ```
pub async fn serve_with_health_checker(
    addr: SocketAddr,
    state: ServerState,
    health_checker: Arc<HealthChecker>,
) -> Result<(), Box<dyn std::error::Error>> {
    serve_with_health_checker_and_shutdown(addr, state, health_checker, ShutdownConfig::default())
        .await
}

/// Start the dx-server with custom health checker and shutdown configuration.
///
/// This is the most flexible variant, allowing full customization of both
/// health checking and graceful shutdown behavior.
///
/// # Arguments
///
/// * `addr` - Socket address to bind to
/// * `state` - Server state with loaded artifacts
/// * `health_checker` - Custom health checker with registered health checks
/// * `shutdown_config` - Configuration for graceful shutdown behavior
///
/// # Example
///
/// ```rust,no_run
/// use dx_www_server::{
///     ServerState, serve_with_health_checker_and_shutdown,
///     HealthChecker, ShutdownConfig,
/// };
/// use std::net::SocketAddr;
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let state = ServerState::new();
///     let addr: SocketAddr = "0.0.0.0:3000".parse()?;
///     
///     // Create custom health checker with database check
///     let mut checker = HealthChecker::with_version("1.0.0");
///     // checker.add_check(Box::new(DatabaseHealthCheck::new(pool)));
///     
///     // Configure 60 second graceful shutdown timeout
///     let shutdown_config = ShutdownConfig::with_timeout(Duration::from_secs(60));
///     
///     serve_with_health_checker_and_shutdown(
///         addr,
///         state,
///         Arc::new(checker),
///         shutdown_config,
///     ).await
/// }
/// ```
pub async fn serve_with_health_checker_and_shutdown(
    addr: SocketAddr,
    state: ServerState,
    health_checker: Arc<HealthChecker>,
    shutdown_config: ShutdownConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("🚀 dx-server starting at {}", addr);

    let app = build_router_with_health_checker(state, health_checker);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("✨ dx-server ready - The Holographic Server is online");
    tracing::info!("📦 Binary streaming enabled");
    tracing::info!("🔍 SEO inflation ready");
    tracing::info!("⚡ Delta patching active");
    tracing::info!("🏥 Health probes: /health/live, /health/ready");
    tracing::info!("⏱️  Graceful shutdown timeout: {:?}", shutdown_config.timeout);

    // Create graceful shutdown handler
    let shutdown = GracefulShutdown::new(shutdown_config.clone());

    // Serve with graceful shutdown support (Requirements 3.1, 3.2)
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.wait_for_signal().await;
            tracing::info!("🛑 Shutdown signal received, initiating graceful shutdown...");
            tracing::info!(
                "⏳ Waiting up to {:?} for in-flight requests to complete",
                shutdown_config.timeout
            );
        })
        .await?;

    tracing::info!("👋 dx-server shutdown complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::time::Duration;
    use tower::ServiceExt;

    #[test]
    fn test_state_creation() {
        let state = ServerState::new();
        assert_eq!(state.binary_cache.len(), 0);
        assert_eq!(state.template_cache.len(), 0);
    }

    #[tokio::test]
    async fn test_build_router_creates_health_endpoints() {
        let state = ServerState::new();
        let app = build_router(state);

        // Test /health/live endpoint (Requirement 3.4)
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/health/live").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_build_router_readiness_endpoint() {
        let state = ServerState::new();
        let app = build_router(state);

        // Test /health/ready endpoint (Requirement 3.3)
        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // With no health checks, should return 200 OK
        assert_eq!(response.status(), StatusCode::OK);

        // Verify response body
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "healthy");
    }

    #[tokio::test]
    async fn test_build_router_with_custom_health_checker() {
        use ops::ManualHealthCheck;

        let state = ServerState::new();

        // Create a health checker with an unhealthy check
        let manual_check = Arc::new(ManualHealthCheck::new("database"));
        manual_check.set_unhealthy("connection pool exhausted");

        // Wrapper to use Arc<ManualHealthCheck> as a HealthCheck
        struct ArcHealthCheck(Arc<ManualHealthCheck>);

        #[async_trait::async_trait]
        impl HealthCheck for ArcHealthCheck {
            fn name(&self) -> &str {
                self.0.name()
            }
            async fn check(&self) -> CheckResult {
                self.0.check().await
            }
        }

        let mut checker = HealthChecker::with_version("1.0.0");
        checker.add_check(Box::new(ArcHealthCheck(manual_check)));

        let app = build_router_with_health_checker(state, Arc::new(checker));

        // Test /health/ready endpoint returns 503 when unhealthy (Requirement 3.5)
        let response = app
            .oneshot(Request::builder().uri("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        // Verify response body indicates unhealthy status
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "unhealthy");
    }

    #[tokio::test]
    async fn test_backward_compatible_health_endpoint() {
        let state = ServerState::new();
        let app = build_router(state);

        // Test /health endpoint still works (backward compatibility)
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_shutdown_config_default() {
        let config = ShutdownConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_shutdown_config_custom_timeout() {
        let config = ShutdownConfig::with_timeout(Duration::from_secs(60));
        assert_eq!(config.timeout, Duration::from_secs(60));
    }
}
