//! # Hot Module Replacement (HMR)
//!
//! WebSocket-based hot module replacement for DX development.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     WebSocket      ┌─────────────┐
//! │  Dev Server │ ←───────────────→  │   Browser   │
//! │  (Watcher)  │    HMR Protocol    │  (Runtime)  │
//! └─────────────┘                    └─────────────┘
//! ```
//!
//! ## HMR Protocol Messages
//!
//! - `connect`: Initial connection with capabilities
//! - `update`: Module changed, apply patch
//! - `full-reload`: Full page reload required
//! - `error`: Compilation error with overlay
//! - `ping/pong`: Keep-alive

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;

/// HMR errors
#[derive(Debug, Error)]
pub enum HmrError {
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Compilation error: {0}")]
    Compilation(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Hot update failed: {0}")]
    UpdateFailed(String),
}

pub type HmrResult<T> = Result<T, HmrError>;

/// HMR message types (server → client)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum HmrServerMessage {
    /// Initial connection response
    Connected {
        /// Session ID
        session_id: String,
        /// Server capabilities
        capabilities: HmrCapabilities,
    },

    /// Module update available
    Update {
        /// Update ID
        update_id: u64,
        /// Modules that changed
        modules: Vec<ModuleUpdate>,
        /// Timestamp
        timestamp: u64,
    },

    /// Full reload required (HMR not possible)
    FullReload {
        /// Reason for full reload
        reason: String,
    },

    /// Compilation error
    Error {
        /// Error type
        error_type: HmrErrorType,
        /// Error message
        message: String,
        /// File path (if applicable)
        file: Option<String>,
        /// Line number
        line: Option<u32>,
        /// Column number
        column: Option<u32>,
        /// Stack trace
        stack: Option<String>,
    },

    /// Keep-alive pong
    Pong {
        /// Timestamp
        timestamp: u64,
    },

    /// Build started
    BuildStart {
        /// Timestamp
        timestamp: u64,
    },

    /// Build completed
    BuildComplete {
        /// Timestamp
        timestamp: u64,
        /// Duration in ms
        duration_ms: u64,
    },

    /// CSS update (can be applied without JS reload)
    CssUpdate {
        /// CSS module path
        path: String,
        /// New CSS content
        css: String,
    },
}

/// HMR message types (client → server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum HmrClientMessage {
    /// Client connecting
    Connect {
        /// Client type
        client_type: HmrClientType,
        /// Client capabilities
        capabilities: HmrCapabilities,
    },

    /// Keep-alive ping
    Ping {
        /// Timestamp
        timestamp: u64,
    },

    /// Update applied successfully
    UpdateApplied {
        /// Update ID
        update_id: u64,
    },

    /// Update failed
    UpdateFailed {
        /// Update ID
        update_id: u64,
        /// Error message
        error: String,
    },

    /// Request full state
    RequestFullState,
}

/// HMR client types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HmrClientType {
    Browser,
    DevTools,
    Cli,
}

/// HMR capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HmrCapabilities {
    /// Supports CSS hot update
    pub css: bool,
    /// Supports component hot swap
    pub components: bool,
    /// Supports state preservation
    pub state_preservation: bool,
    /// Supports error overlay
    pub error_overlay: bool,
    /// Supports binary patches
    pub binary_patches: bool,
}

/// Error types for HMR errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HmrErrorType {
    Syntax,
    Type,
    Runtime,
    Network,
}

/// Module update information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleUpdate {
    /// Module path (relative to project root)
    pub path: String,
    /// Module ID (hash)
    pub id: String,
    /// Update type
    pub update_type: ModuleUpdateType,
    /// New module content (base64 encoded if binary)
    pub content: Option<String>,
    /// Dependencies that need to be re-evaluated
    pub deps: Vec<String>,
    /// CSS classes that changed
    pub css_changes: Vec<CssChange>,
}

/// Module update types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModuleUpdateType {
    /// Full module replacement
    Full,
    /// Partial patch (hot swap)
    Patch,
    /// Only CSS changed
    CssOnly,
    /// Module removed
    Removed,
    /// New module added
    Added,
}

/// CSS change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssChange {
    /// Class name
    pub class: String,
    /// New styles
    pub styles: String,
    /// Removed
    pub removed: bool,
}

/// Module dependency graph
#[derive(Debug, Default)]
pub struct ModuleGraph {
    /// Module metadata
    modules: HashMap<String, ModuleInfo>,
    /// Forward dependencies (module → modules it imports)
    deps: HashMap<String, HashSet<String>>,
    /// Reverse dependencies (module → modules that import it)
    rdeps: HashMap<String, HashSet<String>>,
}

/// Module information
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module path
    pub path: PathBuf,
    /// Module ID (content hash)
    pub id: String,
    /// Last modified time
    pub mtime: u64,
    /// Module type
    pub module_type: ModuleType,
    /// Accepts hot updates
    pub hot_acceptable: bool,
}

/// Module types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleType {
    Component,
    Page,
    Layout,
    Script,
    Style,
    Asset,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, path: &str, info: ModuleInfo) {
        self.modules.insert(path.to_string(), info);
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        self.deps.entry(from.to_string()).or_default().insert(to.to_string());
        self.rdeps.entry(to.to_string()).or_default().insert(from.to_string());
    }

    /// Get modules affected by a change
    pub fn get_affected_modules(&self, changed: &str) -> Vec<String> {
        let mut affected = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![changed.to_string()];

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());
            affected.push(current.clone());

            // Add reverse dependencies
            if let Some(rdeps) = self.rdeps.get(&current) {
                for rdep in rdeps {
                    queue.push(rdep.clone());
                }
            }
        }

        affected
    }

    /// Check if a module can be hot-updated
    pub fn can_hot_update(&self, path: &str) -> bool {
        if let Some(info) = self.modules.get(path) {
            if !info.hot_acceptable {
                return false;
            }
        }

        // Check if any affected module cannot accept hot updates
        let affected = self.get_affected_modules(path);
        for module_path in affected {
            if let Some(info) = self.modules.get(&module_path) {
                if !info.hot_acceptable {
                    return false;
                }
            }
        }

        true
    }

    /// Get module info
    pub fn get_module(&self, path: &str) -> Option<&ModuleInfo> {
        self.modules.get(path)
    }

    /// Remove a module
    pub fn remove_module(&mut self, path: &str) {
        self.modules.remove(path);
        self.deps.remove(path);
        self.rdeps.remove(path);

        // Remove from other modules' deps
        for deps in self.deps.values_mut() {
            deps.remove(path);
        }
        for rdeps in self.rdeps.values_mut() {
            rdeps.remove(path);
        }
    }
}

/// HMR runtime (client-side)
pub struct HmrRuntime {
    /// Session ID
    session_id: Option<String>,
    /// Module registry
    modules: HashMap<String, RegisteredModule>,
    /// Update callbacks
    update_handlers: HashMap<String, Vec<Box<dyn Fn(&ModuleUpdate) + Send + Sync>>>,
    /// Pending updates
    pending_updates: Vec<ModuleUpdate>,
}

impl std::fmt::Debug for HmrRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HmrRuntime")
            .field("session_id", &self.session_id)
            .field("modules", &self.modules)
            .field("update_handlers_count", &self.update_handlers.len())
            .field("pending_updates", &self.pending_updates)
            .finish()
    }
}

/// A registered module
#[derive(Debug)]
pub struct RegisteredModule {
    /// Module ID
    pub id: String,
    /// Module path
    pub path: String,
    /// Module state (for preservation)
    pub state: Option<Vec<u8>>,
    /// Accept handler
    pub accept: bool,
    /// Dispose handler registered
    pub dispose: bool,
}

impl HmrRuntime {
    pub fn new() -> Self {
        Self {
            session_id: None,
            modules: HashMap::new(),
            update_handlers: HashMap::new(),
            pending_updates: Vec::new(),
        }
    }

    /// Handle incoming message
    pub fn handle_message(&mut self, msg: HmrServerMessage) -> HmrResult<Option<HmrClientMessage>> {
        match msg {
            HmrServerMessage::Connected {
                session_id,
                capabilities,
            } => {
                self.session_id = Some(session_id);
                log::info!("HMR connected with capabilities: {:?}", capabilities);
                Ok(None)
            }

            HmrServerMessage::Update {
                update_id, modules, ..
            } => {
                for module in &modules {
                    if let Err(e) = self.apply_module_update(module) {
                        return Ok(Some(HmrClientMessage::UpdateFailed {
                            update_id,
                            error: e.to_string(),
                        }));
                    }
                }
                Ok(Some(HmrClientMessage::UpdateApplied { update_id }))
            }

            HmrServerMessage::FullReload { reason } => {
                log::info!("Full reload required: {}", reason);
                // In browser, this would trigger location.reload()
                Ok(None)
            }

            HmrServerMessage::Error {
                error_type,
                message,
                file,
                line,
                column,
                ..
            } => {
                log::error!(
                    "HMR error ({:?}): {} at {:?}:{}:{}",
                    error_type,
                    message,
                    file,
                    line.unwrap_or(0),
                    column.unwrap_or(0)
                );
                Ok(None)
            }

            HmrServerMessage::Pong { .. } => Ok(None),
            HmrServerMessage::BuildStart { .. } => Ok(None),
            HmrServerMessage::BuildComplete { duration_ms, .. } => {
                log::info!("Build completed in {}ms", duration_ms);
                Ok(None)
            }
            HmrServerMessage::CssUpdate { path, css } => {
                self.apply_css_update(&path, &css)?;
                Ok(None)
            }
        }
    }

    /// Apply a module update
    fn apply_module_update(&mut self, update: &ModuleUpdate) -> HmrResult<()> {
        match update.update_type {
            ModuleUpdateType::Full | ModuleUpdateType::Patch => {
                // Store the update for the module
                if let Some(module) = self.modules.get_mut(&update.path) {
                    // Call dispose handlers if registered
                    if module.dispose {
                        // Save state before dispose
                    }
                    module.id = update.id.clone();
                }

                // Call update handlers
                if let Some(handlers) = self.update_handlers.get(&update.path) {
                    for handler in handlers {
                        handler(update);
                    }
                }
            }

            ModuleUpdateType::CssOnly => {
                if let Some(content) = &update.content {
                    self.apply_css_update(&update.path, content)?;
                }
            }

            ModuleUpdateType::Removed => {
                self.modules.remove(&update.path);
            }

            ModuleUpdateType::Added => {
                self.modules.insert(
                    update.path.clone(),
                    RegisteredModule {
                        id: update.id.clone(),
                        path: update.path.clone(),
                        state: None,
                        accept: true,
                        dispose: false,
                    },
                );
            }
        }

        Ok(())
    }

    /// Apply CSS update (browser-side)
    fn apply_css_update(&self, _path: &str, _css: &str) -> HmrResult<()> {
        // In browser, this would:
        // 1. Find existing <style> or <link> element
        // 2. Replace content or href
        // This is a no-op in Rust; actual implementation in JS/WASM
        Ok(())
    }

    /// Register a module
    pub fn register_module(&mut self, path: &str, id: &str) {
        self.modules.insert(
            path.to_string(),
            RegisteredModule {
                id: id.to_string(),
                path: path.to_string(),
                state: None,
                accept: false,
                dispose: false,
            },
        );
    }

    /// Module accepts hot updates
    pub fn accept(&mut self, path: &str) {
        if let Some(module) = self.modules.get_mut(path) {
            module.accept = true;
        }
    }

    /// Register dispose handler
    pub fn dispose(&mut self, path: &str) {
        if let Some(module) = self.modules.get_mut(path) {
            module.dispose = true;
        }
    }
}

impl Default for HmrRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// HMR Server
pub struct HmrServer {
    /// Module graph
    pub graph: Arc<RwLock<ModuleGraph>>,
    /// Connected clients
    clients: Arc<RwLock<Vec<HmrClient>>>,
    /// Next update ID
    next_update_id: Arc<RwLock<u64>>,
    /// Build in progress
    building: Arc<RwLock<bool>>,
    /// Build start time
    build_start: Arc<RwLock<Option<Instant>>>,
}

/// A connected HMR client
#[derive(Debug)]
pub struct HmrClient {
    /// Client ID
    pub id: u64,
    /// Client type
    pub client_type: HmrClientType,
    /// Capabilities
    pub capabilities: HmrCapabilities,
    /// Message sender (would be WebSocket sender in real impl)
    pub connected_at: Instant,
}

impl HmrServer {
    pub fn new() -> Self {
        Self {
            graph: Arc::new(RwLock::new(ModuleGraph::new())),
            clients: Arc::new(RwLock::new(Vec::new())),
            next_update_id: Arc::new(RwLock::new(0)),
            building: Arc::new(RwLock::new(false)),
            build_start: Arc::new(RwLock::new(None)),
        }
    }

    /// Handle client connection
    pub fn on_connect(
        &self,
        client_type: HmrClientType,
        capabilities: HmrCapabilities,
    ) -> HmrServerMessage {
        let client_id = {
            let mut clients = self.clients.write().unwrap();
            let id = clients.len() as u64;
            clients.push(HmrClient {
                id,
                client_type,
                capabilities: capabilities.clone(),
                connected_at: Instant::now(),
            });
            id
        };

        HmrServerMessage::Connected {
            session_id: format!("dx-hmr-{}", client_id),
            capabilities: HmrCapabilities {
                css: true,
                components: true,
                state_preservation: true,
                error_overlay: true,
                binary_patches: true,
            },
        }
    }

    /// Handle file change
    pub fn on_file_change(&self, path: &Path) -> Option<HmrServerMessage> {
        let path_str = path.to_string_lossy().to_string();

        // Start build
        {
            let mut building = self.building.write().unwrap();
            *building = true;
            let mut start = self.build_start.write().unwrap();
            *start = Some(Instant::now());
        }

        let graph = self.graph.read().unwrap();

        // Check if this module can be hot-updated
        if !graph.can_hot_update(&path_str) {
            return Some(HmrServerMessage::FullReload {
                reason: format!("Module {} cannot be hot-updated", path_str),
            });
        }

        // Get affected modules
        let affected = graph.get_affected_modules(&path_str);

        // Generate update ID
        let update_id = {
            let mut id = self.next_update_id.write().unwrap();
            *id += 1;
            *id
        };

        // Create module updates
        let modules: Vec<ModuleUpdate> = affected
            .iter()
            .map(|p| {
                let module_info = graph.get_module(p);
                ModuleUpdate {
                    path: p.clone(),
                    id: module_info.map(|m| m.id.clone()).unwrap_or_default(),
                    update_type: if p == &path_str {
                        ModuleUpdateType::Full
                    } else {
                        ModuleUpdateType::Patch
                    },
                    content: None, // Would be filled with actual content
                    deps: Vec::new(),
                    css_changes: Vec::new(),
                }
            })
            .collect();

        // End build
        let duration_ms = {
            let mut building = self.building.write().unwrap();
            *building = false;
            let start = self.build_start.read().unwrap();
            start.map(|s| s.elapsed().as_millis() as u64).unwrap_or(0)
        };

        Some(HmrServerMessage::Update {
            update_id,
            modules,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        })
    }

    /// Handle compilation error
    pub fn on_error(
        &self,
        error: &str,
        file: Option<&Path>,
        line: Option<u32>,
        column: Option<u32>,
    ) -> HmrServerMessage {
        HmrServerMessage::Error {
            error_type: HmrErrorType::Syntax,
            message: error.to_string(),
            file: file.map(|p| p.to_string_lossy().to_string()),
            line,
            column,
            stack: None,
        }
    }

    /// Get connected client count
    pub fn client_count(&self) -> usize {
        self.clients.read().unwrap().len()
    }
}

impl Default for HmrServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate HMR client script (injected into page)
pub fn generate_hmr_client_script(port: u16) -> String {
    format!(
        r#"
(function() {{
    'use strict';
    
    const HMR_PORT = {port};
    const HMR_HOST = window.location.hostname || 'localhost';
    
    let ws = null;
    let reconnectAttempts = 0;
    const maxReconnectAttempts = 10;
    const reconnectDelay = 1000;
    
    // Module registry
    const modules = new Map();
    const acceptCallbacks = new Map();
    const disposeCallbacks = new Map();
    
    // Error overlay
    let errorOverlay = null;
    
    function connect() {{
        ws = new WebSocket(`ws://${{HMR_HOST}}:${{HMR_PORT}}/__hmr`);
        
        ws.onopen = () => {{
            console.log('[DX HMR] Connected');
            reconnectAttempts = 0;
            ws.send(JSON.stringify({{
                type: 'connect',
                clientType: 'browser',
                capabilities: {{
                    css: true,
                    components: true,
                    statePreservation: true,
                    errorOverlay: true,
                    binaryPatches: true
                }}
            }}));
        }};
        
        ws.onmessage = (event) => {{
            const msg = JSON.parse(event.data);
            handleMessage(msg);
        }};
        
        ws.onclose = () => {{
            console.log('[DX HMR] Disconnected');
            scheduleReconnect();
        }};
        
        ws.onerror = (err) => {{
            console.error('[DX HMR] Error:', err);
        }};
    }}
    
    function scheduleReconnect() {{
        if (reconnectAttempts < maxReconnectAttempts) {{
            reconnectAttempts++;
            setTimeout(connect, reconnectDelay * reconnectAttempts);
        }}
    }}
    
    function handleMessage(msg) {{
        switch (msg.type) {{
            case 'connected':
                console.log('[DX HMR] Session:', msg.sessionId);
                break;
                
            case 'update':
                hideErrorOverlay();
                for (const mod of msg.modules) {{
                    applyUpdate(mod, msg.updateId);
                }}
                break;
                
            case 'full-reload':
                console.log('[DX HMR] Full reload:', msg.reason);
                window.location.reload();
                break;
                
            case 'error':
                showErrorOverlay(msg);
                break;
                
            case 'build-start':
                console.log('[DX HMR] Building...');
                break;
                
            case 'build-complete':
                console.log(`[DX HMR] Built in ${{msg.durationMs}}ms`);
                break;
                
            case 'css-update':
                applyCssUpdate(msg.path, msg.css);
                break;
        }}
    }}
    
    function applyUpdate(mod, updateId) {{
        console.log(`[DX HMR] Updating: ${{mod.path}}`);
        
        try {{
            // Call dispose callbacks
            const dispose = disposeCallbacks.get(mod.path);
            if (dispose) {{
                dispose();
            }}
            
            // Update module
            if (mod.updateType === 'css-only') {{
                applyCssUpdate(mod.path, mod.content);
            }} else if (mod.content) {{
                // Execute new module code
                const fn = new Function('module', 'exports', mod.content);
                const module = {{ exports: {{}} }};
                fn(module, module.exports);
                modules.set(mod.path, module.exports);
            }}
            
            // Call accept callbacks
            const accept = acceptCallbacks.get(mod.path);
            if (accept) {{
                accept(mod);
            }}
            
            // Notify server
            ws.send(JSON.stringify({{
                type: 'update-applied',
                updateId: updateId
            }}));
        }} catch (err) {{
            console.error('[DX HMR] Update failed:', err);
            ws.send(JSON.stringify({{
                type: 'update-failed',
                updateId: updateId,
                error: err.message
            }}));
        }}
    }}
    
    function applyCssUpdate(path, css) {{
        let style = document.querySelector(`style[data-dx-css="${{path}}"]`);
        if (!style) {{
            style = document.createElement('style');
            style.setAttribute('data-dx-css', path);
            document.head.appendChild(style);
        }}
        style.textContent = css;
    }}
    
    function showErrorOverlay(error) {{
        if (!errorOverlay) {{
            errorOverlay = document.createElement('div');
            errorOverlay.id = 'dx-error-overlay';
            errorOverlay.innerHTML = `
                <style>
                    #dx-error-overlay {{
                        position: fixed;
                        top: 0;
                        left: 0;
                        right: 0;
                        bottom: 0;
                        background: rgba(0, 0, 0, 0.9);
                        color: #ff5555;
                        font-family: monospace;
                        font-size: 14px;
                        padding: 20px;
                        overflow: auto;
                        z-index: 99999;
                    }}
                    #dx-error-overlay h1 {{
                        color: #ff5555;
                        margin: 0 0 20px;
                    }}
                    #dx-error-overlay .file {{
                        color: #8be9fd;
                    }}
                    #dx-error-overlay .line {{
                        color: #bd93f9;
                    }}
                    #dx-error-overlay pre {{
                        background: #282a36;
                        padding: 15px;
                        border-radius: 5px;
                        overflow-x: auto;
                    }}
                    #dx-error-overlay button {{
                        position: absolute;
                        top: 10px;
                        right: 10px;
                        background: #44475a;
                        border: none;
                        color: #f8f8f2;
                        padding: 10px 20px;
                        cursor: pointer;
                        border-radius: 5px;
                    }}
                </style>
                <button onclick="document.getElementById('dx-error-overlay').remove()">×</button>
                <div class="content"></div>
            `;
            document.body.appendChild(errorOverlay);
        }}
        
        const content = errorOverlay.querySelector('.content');
        content.innerHTML = `
            <h1>⚠️ ${{error.errorType.toUpperCase()}} ERROR</h1>
            <p class="file">${{error.file || 'Unknown file'}}</p>
            <p class="line">Line ${{error.line || '?'}}, Column ${{error.column || '?'}}</p>
            <pre>${{escapeHtml(error.message)}}</pre>
            ${{error.stack ? `<pre>${{escapeHtml(error.stack)}}</pre>` : ''}}
        `;
    }}
    
    function hideErrorOverlay() {{
        if (errorOverlay) {{
            errorOverlay.remove();
            errorOverlay = null;
        }}
    }}
    
    function escapeHtml(str) {{
        return str
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
    }}
    
    // Export HMR API
    window.__DX_HMR__ = {{
        accept: (path, callback) => {{
            acceptCallbacks.set(path, callback);
        }},
        dispose: (path, callback) => {{
            disposeCallbacks.set(path, callback);
        }},
        decline: (path) => {{
            // Mark module as not hot-updatable
        }},
        invalidate: () => {{
            window.location.reload();
        }},
        data: {{}},
        status: () => ws?.readyState === WebSocket.OPEN ? 'ready' : 'disconnected'
    }};
    
    // Start connection
    connect();
}})();
"#,
        port = port
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_graph() {
        let mut graph = ModuleGraph::new();

        graph.add_module(
            "app.pg",
            ModuleInfo {
                path: PathBuf::from("app.pg"),
                id: "abc123".into(),
                mtime: 0,
                module_type: ModuleType::Page,
                hot_acceptable: true,
            },
        );

        graph.add_module(
            "button.cp",
            ModuleInfo {
                path: PathBuf::from("button.cp"),
                id: "def456".into(),
                mtime: 0,
                module_type: ModuleType::Component,
                hot_acceptable: true,
            },
        );

        graph.add_dependency("app.pg", "button.cp");

        let affected = graph.get_affected_modules("button.cp");
        assert!(affected.contains(&"button.cp".to_string()));
        assert!(affected.contains(&"app.pg".to_string()));
    }

    #[test]
    fn test_hmr_server_connect() {
        let server = HmrServer::new();
        let msg = server.on_connect(HmrClientType::Browser, HmrCapabilities::default());

        if let HmrServerMessage::Connected { session_id, .. } = msg {
            assert!(session_id.starts_with("dx-hmr-"));
        } else {
            panic!("Expected Connected message");
        }
    }

    #[test]
    fn test_hmr_runtime() {
        let mut runtime = HmrRuntime::new();
        runtime.register_module("test.cp", "abc123");
        runtime.accept("test.cp");

        assert!(runtime.modules.get("test.cp").unwrap().accept);
    }
}
