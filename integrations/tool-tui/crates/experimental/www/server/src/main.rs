//! # dx-www-server CLI
//!
//! Entry point for the Holographic Server

use dx_www_server::{ServerState, serve};
use std::net::SocketAddr;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("dx_www_server=debug,tower_http=debug")
        .init();

    // Create server state
    let state = ServerState::new();

    // Parse address
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;

    // Load artifacts from build/artifacts/macro
    if let Err(e) = state.load_artifacts(std::path::Path::new("build/artifacts/macro")) {
        tracing::warn!("Failed to load initial artifacts: {}", e);
    }

    // Start server
    serve(addr, state).await?;

    Ok(())
}
