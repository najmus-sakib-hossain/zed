//! # Hot Reload Server
//!
//! WebSocket server for sending hot reload notifications to connected clients.

#![allow(dead_code)]

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use crate::error::{DxError, DxResult};

// =============================================================================
// Hot Reload Server
// =============================================================================

/// WebSocket server for hot reload.
pub struct HotReloadServer {
    /// Server port
    port: u16,
    /// Connected client IDs
    clients: Arc<RwLock<HashSet<u64>>>,
    /// Message broadcast channel
    broadcast_tx: broadcast::Sender<HotReloadMessage>,
    /// Next client ID
    next_client_id: Arc<std::sync::atomic::AtomicU64>,
}

impl HotReloadServer {
    /// Create a new hot reload server.
    pub fn new(port: u16) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);

        Self {
            port,
            clients: Arc::new(RwLock::new(HashSet::new())),
            broadcast_tx,
            next_client_id: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Start the WebSocket server.
    pub async fn start(&self) -> DxResult<()> {
        // WebSocket server implementation would go here
        // Using tokio-tungstenite or similar
        Ok(())
    }

    /// Stop the server.
    pub async fn stop(self) -> DxResult<()> {
        // Send shutdown message to all clients
        let _ = self.broadcast_tx.send(HotReloadMessage::Shutdown);
        Ok(())
    }

    /// Notify clients of a file change.
    pub async fn notify_change(&self, path: &PathBuf) -> DxResult<()> {
        let change_type = self.detect_change_type(path);

        let message = HotReloadMessage::FileChanged {
            path: path.to_string_lossy().to_string(),
            change_type,
        };

        self.broadcast_tx.send(message).map_err(|_| DxError::IoError {
            path: Some(path.clone()),
            message: "Failed to send hot reload notification".to_string(),
        })?;

        Ok(())
    }

    /// Notify clients of an error.
    pub async fn notify_error(&self, error: &str) -> DxResult<()> {
        let message = HotReloadMessage::Error {
            message: error.to_string(),
        };

        let _ = self.broadcast_tx.send(message);
        Ok(())
    }

    /// Notify clients that an error was resolved.
    pub async fn notify_error_resolved(&self) -> DxResult<()> {
        let message = HotReloadMessage::ErrorResolved;
        let _ = self.broadcast_tx.send(message);
        Ok(())
    }

    /// Detect the type of change based on file extension.
    fn detect_change_type(&self, path: &PathBuf) -> ChangeType {
        match path.extension().and_then(|e| e.to_str()) {
            Some("pg") | Some("cp") => ChangeType::Component,
            Some("css") => ChangeType::Style,
            Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") => ChangeType::Script,
            _ => ChangeType::Asset,
        }
    }

    /// Get the number of connected clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Get the server port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Subscribe to hot reload messages.
    pub fn subscribe(&self) -> broadcast::Receiver<HotReloadMessage> {
        self.broadcast_tx.subscribe()
    }
}

// =============================================================================
// Hot Reload Messages
// =============================================================================

/// Messages sent to hot reload clients.
#[derive(Debug, Clone)]
pub enum HotReloadMessage {
    /// A file was changed
    FileChanged {
        /// Path to the changed file
        path: String,
        /// Type of change
        change_type: ChangeType,
    },
    /// Full page reload required
    FullReload,
    /// Compilation error occurred
    Error {
        /// Error message
        message: String,
    },
    /// Previous error was resolved
    ErrorResolved,
    /// Server is shutting down
    Shutdown,
}

/// Type of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// Component file (.pg, .cp)
    Component,
    /// Style file (.css)
    Style,
    /// Script file (.rs, .py, .js, .ts, .go)
    Script,
    /// Static asset
    Asset,
}

// =============================================================================
// Client JavaScript
// =============================================================================

/// Generate the hot reload client JavaScript code.
pub fn client_script(port: u16) -> String {
    format!(
        r#"
(function() {{
    const ws = new WebSocket('ws://localhost:{}/hot-reload');
    
    ws.onopen = function() {{
        console.log('[DX] Hot reload connected');
    }};
    
    ws.onmessage = function(event) {{
        const message = JSON.parse(event.data);
        
        switch (message.type) {{
            case 'FileChanged':
                if (message.change_type === 'Style') {{
                    // Hot replace CSS
                    const links = document.querySelectorAll('link[rel="stylesheet"]');
                    links.forEach(link => {{
                        const url = new URL(link.href);
                        url.searchParams.set('t', Date.now());
                        link.href = url.toString();
                    }});
                }} else if (message.change_type === 'Component') {{
                    // Hot replace component
                    window.__DX_HOT_UPDATE__(message.path);
                }} else {{
                    // Full reload for other changes
                    location.reload();
                }}
                break;
                
            case 'FullReload':
                location.reload();
                break;
                
            case 'Error':
                window.__DX_SHOW_ERROR__(message.message);
                break;
                
            case 'ErrorResolved':
                window.__DX_HIDE_ERROR__();
                break;
        }}
    }};
    
    ws.onclose = function() {{
        console.log('[DX] Hot reload disconnected, reconnecting...');
        setTimeout(() => location.reload(), 1000);
    }};
}})();
"#,
        port
    )
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_reload_server_new() {
        let server = HotReloadServer::new(3001);
        assert_eq!(server.port(), 3001);
    }

    #[test]
    fn test_detect_change_type() {
        let server = HotReloadServer::new(3001);

        assert_eq!(server.detect_change_type(&PathBuf::from("page.pg")), ChangeType::Component);
        assert_eq!(server.detect_change_type(&PathBuf::from("style.css")), ChangeType::Style);
        assert_eq!(server.detect_change_type(&PathBuf::from("handler.rs")), ChangeType::Script);
        assert_eq!(server.detect_change_type(&PathBuf::from("image.png")), ChangeType::Asset);
    }

    #[test]
    fn test_client_script() {
        let script = client_script(3001);
        assert!(script.contains("ws://localhost:3001"));
        assert!(script.contains("Hot reload"));
    }
}
