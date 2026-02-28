//! # Development Server
//!
//! This module provides the development server with hot reload capabilities.
//!
//! Features:
//! - HTTP server for serving compiled assets
//! - File watcher for detecting source changes
//! - Hot reload via WebSocket for instant updates
//! - Error overlay for displaying compilation errors

mod error_overlay;
mod hot_reload;
mod watcher;

pub use error_overlay::ErrorOverlay;
pub use hot_reload::HotReloadServer;
pub use watcher::FileWatcher;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::broadcast;

use crate::build::BuildPipeline;
use crate::config::DxConfig;
use crate::error::{DxError, DxResult};
use crate::router::FileSystemRouter;

// =============================================================================
// Development Server
// =============================================================================

/// Development server with hot reload support.
pub struct DevServer {
    /// Server configuration
    config: DxConfig,
    /// Project root path
    project_root: PathBuf,
    /// Build pipeline
    build_pipeline: Arc<tokio::sync::RwLock<BuildPipeline>>,
    /// Router
    router: Arc<tokio::sync::RwLock<FileSystemRouter>>,
    /// File watcher
    watcher: Option<FileWatcher>,
    /// Hot reload server
    hot_reload: Option<HotReloadServer>,
    /// Error overlay
    error_overlay: ErrorOverlay,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
}

impl DevServer {
    /// Create a new development server.
    pub fn new(config: &DxConfig, project_root: PathBuf) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config: config.clone(),
            project_root: project_root.clone(),
            build_pipeline: Arc::new(tokio::sync::RwLock::new(BuildPipeline::new(config))),
            router: Arc::new(tokio::sync::RwLock::new(FileSystemRouter::new())),
            watcher: None,
            hot_reload: None,
            error_overlay: ErrorOverlay::new(),
            shutdown_tx,
        }
    }

    /// Start the development server.
    pub async fn start(&mut self) -> DxResult<()> {
        // Initial build
        self.initial_build().await?;

        // Start file watcher
        self.start_watcher()?;

        // Start hot reload WebSocket server
        self.start_hot_reload().await?;

        // Start HTTP server
        self.start_http_server().await?;

        Ok(())
    }

    /// Perform initial build.
    async fn initial_build(&self) -> DxResult<()> {
        let _pipeline = self.build_pipeline.write().await;
        let _router = self.router.write().await;

        // Router initialization would happen here through from_project()
        // For now, just return Ok
        Ok(())
    }

    /// Start the file watcher.
    fn start_watcher(&mut self) -> DxResult<()> {
        let watcher = FileWatcher::new(&self.project_root)?;
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Start the hot reload server.
    async fn start_hot_reload(&mut self) -> DxResult<()> {
        let port = self.config.dev.ws_port.unwrap_or(self.config.dev.port + 1);
        let hot_reload = HotReloadServer::new(port);
        self.hot_reload = Some(hot_reload);
        Ok(())
    }

    /// Start the HTTP server.
    async fn start_http_server(&self) -> DxResult<()> {
        let port = self.config.dev.port;
        let addr: SocketAddr =
            format!("{}:{}", self.config.dev.host, port).parse().map_err(|e| {
                DxError::ConfigValidationError {
                    message: format!("Invalid address: {}", e),
                    field: Some("dev.host".to_string()),
                }
            })?;

        println!("🚀 Development server running at http://{}", addr);
        println!("   Hot reload enabled on port {}", self.config.dev.ws_port.unwrap_or(port + 1));

        // Server loop would go here
        // For now, just return Ok
        Ok(())
    }

    /// Handle a file change event.
    pub async fn on_file_change(&mut self, path: &PathBuf) -> DxResult<()> {
        println!("📝 File changed: {}", path.display());

        // Determine what changed
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "pg" | "cp" => {
                // Component file changed - incremental rebuild
                self.rebuild_component(path).await?;
            }
            "css" => {
                // Style file changed
                self.rebuild_styles(path).await?;
            }
            "rs" | "py" | "js" | "ts" | "go" => {
                // Script file changed
                self.rebuild_script(path).await?;
            }
            _ => {
                // Static asset changed
                self.reload_asset(path).await?;
            }
        }

        // Notify connected clients
        if let Some(hot_reload) = &self.hot_reload {
            hot_reload.notify_change(path).await?;
        }

        Ok(())
    }

    /// Rebuild a component.
    async fn rebuild_component(&self, _path: &PathBuf) -> DxResult<()> {
        let _pipeline = self.build_pipeline.write().await;
        // pipeline.build_incremental(&[path.clone()]).await?;
        Ok(())
    }

    /// Rebuild styles.
    async fn rebuild_styles(&self, _path: &PathBuf) -> DxResult<()> {
        // Rebuild CSS
        Ok(())
    }

    /// Rebuild scripts.
    async fn rebuild_script(&self, _path: &PathBuf) -> DxResult<()> {
        // Rebuild script
        Ok(())
    }

    /// Reload a static asset.
    async fn reload_asset(&self, _path: &PathBuf) -> DxResult<()> {
        // Notify clients to reload asset
        Ok(())
    }

    /// Show compilation error in overlay.
    pub fn show_error(&mut self, error: &DxError) {
        self.error_overlay.show(error);
    }

    /// Clear error overlay.
    pub fn clear_error(&mut self) {
        self.error_overlay.clear();
    }

    /// Stop the development server.
    pub async fn stop(&mut self) -> DxResult<()> {
        let _ = self.shutdown_tx.send(());

        if let Some(watcher) = self.watcher.take() {
            watcher.stop()?;
        }

        if let Some(hot_reload) = self.hot_reload.take() {
            hot_reload.stop().await?;
        }

        Ok(())
    }

    /// Get the server address.
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.dev.host, self.config.dev.port)
    }

    /// Check if hot reload is enabled.
    pub fn hot_reload_enabled(&self) -> bool {
        self.config.dev.hot_reload
    }
}

impl Default for DevServer {
    fn default() -> Self {
        Self::new(&DxConfig::default(), PathBuf::from("."))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev_server_new() {
        let config = DxConfig::default();
        let server = DevServer::new(&config, PathBuf::from("."));
        assert!(server.hot_reload_enabled());
    }

    #[test]
    fn test_dev_server_address() {
        let config = DxConfig::default();
        let server = DevServer::new(&config, PathBuf::from("."));
        assert!(server.address().contains("3000"));
    }
}
