//! DX Option Server Entry Point

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod handlers;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "dx_option=debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting DX Option Server...");

    // Initialize app state with Turso connection
    let app_state = dx_option::AppState::new().await?;
    tracing::info!("✅ Connected to Turso database");

    // Build router
    let app = Router::new()
        // Pages
        .route("/", get(handlers::landing_page))
        .route("/docs", get(handlers::docs_page))
        .route("/playground", get(handlers::playground_page))
        .route("/pricing", get(handlers::pricing_page))
        // API Routes
        .route("/api/contact", post(api::contact::submit))
        .route("/api/query", post(api::query::execute))
        .route("/api/subscribe", post(api::subscribe::newsletter))
        // Static assets
        .nest_service("/static", tower_http::services::ServeDir::new("public"))
        // Middleware
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(app_state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("🌐 Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
