//! Health check endpoint.

use std::sync::Arc;

use axum::{Json, Router, routing::get};

use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/health", get(health_check))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
