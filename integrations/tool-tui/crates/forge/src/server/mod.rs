//! Server module for HTTP API and WebSocket support
//!
//! This module provides the HTTP server and API endpoints.
//! The full server functionality requires the "daemon" feature.
//! Semantic analysis requires the "semantic-analysis" feature.

#[cfg(feature = "daemon")]
pub mod api;
pub mod authentication;
pub mod lsp;
#[cfg(feature = "semantic-analysis")]
pub mod semantic_analyzer;

use anyhow::Result;
use std::path::PathBuf;

#[cfg(feature = "daemon")]
pub async fn start(port: u16, path: PathBuf) -> Result<()> {
    api::serve(port, path).await
}

#[cfg(not(feature = "daemon"))]
pub async fn start(_port: u16, _path: PathBuf) -> Result<()> {
    Err(anyhow::anyhow!(
        "Server functionality requires the 'daemon' feature. \
         Enable it in Cargo.toml: dx-forge = {{ features = [\"daemon\"] }}"
    ))
}
