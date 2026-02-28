//! # DX DevTools
//!
//! Browser extension and debugging tools for DX applications.
//!
//! ## Features
//!
//! - Component tree inspector
//! - State viewer and editor
//! - Performance profiler
//! - Network inspector (HTIP binary)
//! - HMR status
//! - Time-travel debugging

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// DevTools message (extension ↔ page)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum DevToolsMessage {
    // Page → Extension
    /// Component tree update
    ComponentTree { root: ComponentNode, timestamp: u64 },

    /// State snapshot
    StateSnapshot {
        state: HashMap<String, serde_json::Value>,
        version: u64,
    },

    /// Performance metrics
    PerformanceMetrics { metrics: PerformanceData },

    /// HTIP network event
    NetworkEvent { event: NetworkEventData },

    /// Console message
    ConsoleMessage {
        level: LogLevel,
        message: String,
        timestamp: u64,
    },

    // Extension → Page
    /// Request component tree
    RequestTree,

    /// Request state snapshot
    RequestState,

    /// Update state value
    UpdateState {
        path: String,
        value: serde_json::Value,
    },

    /// Highlight component
    HighlightComponent { id: String },

    /// Start profiling
    StartProfiling,

    /// Stop profiling
    StopProfiling,

    /// Time-travel to snapshot
    TimeTravel { snapshot_id: u64 },
}

/// Component node in the tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentNode {
    /// Component instance ID
    pub id: String,
    /// Component name
    pub name: String,
    /// Component type
    pub component_type: String,
    /// Props
    pub props: HashMap<String, serde_json::Value>,
    /// Local state
    pub state: HashMap<String, serde_json::Value>,
    /// Children components
    pub children: Vec<ComponentNode>,
    /// Source location
    pub source: Option<SourceLocation>,
    /// Render count
    pub render_count: u32,
    /// Last render duration (µs)
    pub render_time_us: u64,
}

/// Source location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Performance data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceData {
    /// Frame timings
    pub frames: Vec<FrameTiming>,
    /// Component render times
    pub component_renders: Vec<ComponentRender>,
    /// HTIP patch times
    pub patch_times: Vec<PatchTiming>,
    /// Memory usage
    pub memory: MemoryUsage,
    /// FPS
    pub fps: f64,
    /// JS heap size
    pub heap_size: u64,
}

/// Frame timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameTiming {
    pub timestamp: u64,
    pub duration_us: u64,
    pub scripting_us: u64,
    pub rendering_us: u64,
    pub painting_us: u64,
}

/// Component render timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRender {
    pub component_id: String,
    pub component_name: String,
    pub timestamp: u64,
    pub duration_us: u64,
    pub phase: RenderPhase,
}

/// Render phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RenderPhase {
    Mount,
    Update,
    Unmount,
}

/// Patch timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchTiming {
    pub opcode: String,
    pub timestamp: u64,
    pub duration_us: u64,
    pub target: String,
}

/// Memory usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub wasm_memory_bytes: u64,
    pub dom_nodes: u32,
    pub event_listeners: u32,
    pub components: u32,
    pub state_entries: u32,
}

/// Network event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEventData {
    /// Request ID
    pub id: u64,
    /// URL
    pub url: String,
    /// Event type
    pub event_type: NetworkEventType,
    /// HTIP opcodes (if applicable)
    pub opcodes: Option<Vec<OpcodeInfo>>,
    /// Size in bytes
    pub size: u64,
    /// Duration in ms
    pub duration_ms: Option<u64>,
    /// Status code
    pub status: Option<u16>,
}

/// Network event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkEventType {
    Request,
    Response,
    WebSocket,
    HtipPatch,
}

/// HTIP opcode info for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpcodeInfo {
    pub opcode: u8,
    pub name: String,
    pub size: u32,
    pub params: HashMap<String, String>,
}

/// DevTools server (runs in page)
pub struct DevToolsServer {
    /// Component registry
    components: HashMap<String, ComponentNode>,
    /// State snapshots for time-travel
    snapshots: Vec<StateSnapshot>,
    /// Max snapshots to keep
    max_snapshots: usize,
    /// Current snapshot index
    current_snapshot: usize,
    /// Profiling enabled
    profiling: bool,
    /// Performance data
    perf_data: PerformanceData,
    /// Start time
    start_time: Instant,
}

/// State snapshot for time-travel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub id: u64,
    pub timestamp: u64,
    pub state: HashMap<String, serde_json::Value>,
    pub action: Option<String>,
}

impl DevToolsServer {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            snapshots: Vec::new(),
            max_snapshots: 100,
            current_snapshot: 0,
            profiling: false,
            perf_data: PerformanceData::default(),
            start_time: Instant::now(),
        }
    }

    /// Handle message from extension
    pub fn handle_message(&mut self, msg: DevToolsMessage) -> Option<DevToolsMessage> {
        match msg {
            DevToolsMessage::RequestTree => Some(DevToolsMessage::ComponentTree {
                root: self.build_component_tree(),
                timestamp: self.timestamp(),
            }),

            DevToolsMessage::RequestState => {
                let state = self.get_current_state();
                Some(DevToolsMessage::StateSnapshot {
                    state,
                    version: self.snapshots.len() as u64,
                })
            }

            DevToolsMessage::UpdateState { path, value } => {
                self.update_state(&path, value);
                None
            }

            DevToolsMessage::HighlightComponent { id } => {
                self.highlight_component(&id);
                None
            }

            DevToolsMessage::StartProfiling => {
                self.profiling = true;
                self.perf_data = PerformanceData::default();
                None
            }

            DevToolsMessage::StopProfiling => {
                self.profiling = false;
                Some(DevToolsMessage::PerformanceMetrics {
                    metrics: self.perf_data.clone(),
                })
            }

            DevToolsMessage::TimeTravel { snapshot_id } => {
                self.time_travel(snapshot_id);
                None
            }

            _ => None,
        }
    }

    /// Register a component
    pub fn register_component(&mut self, id: &str, node: ComponentNode) {
        self.components.insert(id.to_string(), node);
    }

    /// Unregister a component
    pub fn unregister_component(&mut self, id: &str) {
        self.components.remove(id);
    }

    /// Record state change
    pub fn record_state_change(
        &mut self,
        state: HashMap<String, serde_json::Value>,
        action: Option<&str>,
    ) {
        let snapshot = StateSnapshot {
            id: self.snapshots.len() as u64,
            timestamp: self.timestamp(),
            state,
            action: action.map(String::from),
        };

        self.snapshots.push(snapshot);
        self.current_snapshot = self.snapshots.len() - 1;

        // Trim old snapshots
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.remove(0);
            self.current_snapshot = self.snapshots.len() - 1;
        }
    }

    /// Record component render
    pub fn record_render(
        &mut self,
        component_id: &str,
        component_name: &str,
        duration: Duration,
        phase: RenderPhase,
    ) {
        if !self.profiling {
            return;
        }

        self.perf_data.component_renders.push(ComponentRender {
            component_id: component_id.to_string(),
            component_name: component_name.to_string(),
            timestamp: self.timestamp(),
            duration_us: duration.as_micros() as u64,
            phase,
        });
    }

    /// Record frame timing
    pub fn record_frame(&mut self, scripting: Duration, rendering: Duration, painting: Duration) {
        if !self.profiling {
            return;
        }

        let total = scripting + rendering + painting;
        self.perf_data.frames.push(FrameTiming {
            timestamp: self.timestamp(),
            duration_us: total.as_micros() as u64,
            scripting_us: scripting.as_micros() as u64,
            rendering_us: rendering.as_micros() as u64,
            painting_us: painting.as_micros() as u64,
        });

        // Update FPS
        if self.perf_data.frames.len() >= 10 {
            let recent: Vec<_> = self.perf_data.frames.iter().rev().take(10).collect();
            let avg_frame_time: f64 =
                recent.iter().map(|f| f.duration_us as f64).sum::<f64>() / recent.len() as f64;
            self.perf_data.fps = 1_000_000.0 / avg_frame_time;
        }
    }

    /// Record HTIP patch
    pub fn record_patch(&mut self, opcode: &str, target: &str, duration: Duration) {
        if !self.profiling {
            return;
        }

        self.perf_data.patch_times.push(PatchTiming {
            opcode: opcode.to_string(),
            timestamp: self.timestamp(),
            duration_us: duration.as_micros() as u64,
            target: target.to_string(),
        });
    }

    /// Build component tree
    fn build_component_tree(&self) -> ComponentNode {
        // Find root component(s) and build tree
        // For now, return a placeholder
        ComponentNode {
            id: "root".into(),
            name: "App".into(),
            component_type: "page".into(),
            props: HashMap::new(),
            state: HashMap::new(),
            children: self.components.values().cloned().collect(),
            source: None,
            render_count: 0,
            render_time_us: 0,
        }
    }

    /// Get current state
    fn get_current_state(&self) -> HashMap<String, serde_json::Value> {
        self.snapshots.last().map(|s| s.state.clone()).unwrap_or_default()
    }

    /// Update state value
    fn update_state(&mut self, _path: &str, _value: serde_json::Value) {
        // Would update actual state through DX runtime
    }

    /// Highlight component in page
    fn highlight_component(&self, _id: &str) {
        // Would trigger highlight overlay in page
    }

    /// Time-travel to snapshot
    fn time_travel(&mut self, snapshot_id: u64) {
        if let Some(idx) = self.snapshots.iter().position(|s| s.id == snapshot_id) {
            self.current_snapshot = idx;
            // Would restore state through DX runtime
        }
    }

    /// Get timestamp
    fn timestamp(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

impl Default for DevToolsServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate DevTools panel HTML
pub fn generate_devtools_panel_html() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>DX DevTools</title>
    <style>
        * { box-sizing: border-box; margin: 0; padding: 0; }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #1e1e1e;
            color: #d4d4d4;
            font-size: 12px;
        }
        .container { display: flex; height: 100vh; }
        .sidebar {
            width: 300px;
            border-right: 1px solid #333;
            display: flex;
            flex-direction: column;
        }
        .tabs {
            display: flex;
            background: #252526;
            border-bottom: 1px solid #333;
        }
        .tab {
            padding: 8px 16px;
            cursor: pointer;
            border-bottom: 2px solid transparent;
        }
        .tab.active {
            background: #1e1e1e;
            border-bottom-color: #007acc;
        }
        .panel { flex: 1; overflow: auto; padding: 10px; }
        .main { flex: 1; display: flex; flex-direction: column; }
        
        /* Component tree */
        .tree-node { padding-left: 15px; }
        .tree-node-header {
            display: flex;
            align-items: center;
            padding: 3px 5px;
            cursor: pointer;
        }
        .tree-node-header:hover { background: #2a2d2e; }
        .tree-node-name { color: #9cdcfe; }
        .tree-node-tag { color: #569cd6; margin-right: 5px; }
        
        /* State viewer */
        .state-entry { padding: 5px; border-bottom: 1px solid #333; }
        .state-key { color: #9cdcfe; }
        .state-value { color: #ce9178; }
        .state-type { color: #4ec9b0; font-size: 10px; }
        
        /* Performance */
        .metric { display: flex; justify-content: space-between; padding: 5px; }
        .metric-value { color: #4ec9b0; font-weight: bold; }
        .chart { height: 100px; background: #252526; margin: 10px 0; }
        
        /* Network */
        .network-entry { 
            display: flex; 
            padding: 5px; 
            border-bottom: 1px solid #333;
        }
        .network-entry:hover { background: #2a2d2e; }
        .network-url { flex: 1; color: #9cdcfe; overflow: hidden; text-overflow: ellipsis; }
        .network-size { color: #6a9955; width: 80px; text-align: right; }
        .network-time { color: #dcdcaa; width: 80px; text-align: right; }
    </style>
</head>
<body>
    <div class="container">
        <div class="sidebar">
            <div class="tabs">
                <div class="tab active" data-panel="components">Components</div>
                <div class="tab" data-panel="state">State</div>
            </div>
            <div class="panel" id="sidebar-panel">
                <!-- Component tree or state viewer -->
            </div>
        </div>
        <div class="main">
            <div class="tabs">
                <div class="tab active" data-panel="props">Props</div>
                <div class="tab" data-panel="perf">Performance</div>
                <div class="tab" data-panel="network">Network</div>
            </div>
            <div class="panel" id="main-panel">
                <!-- Details panel -->
            </div>
        </div>
    </div>
    <script src="devtools.js"></script>
</body>
</html>"#
}

/// Generate DevTools panel JavaScript
pub fn generate_devtools_panel_js() -> &'static str {
    r#"
(function() {
    'use strict';
    
    let port = null;
    let componentTree = null;
    let stateSnapshot = null;
    let selectedComponent = null;
    let perfData = null;
    let networkEvents = [];
    
    // Connect to background script
    function connect() {
        port = chrome.runtime.connect({ name: 'dx-devtools' });
        port.onMessage.addListener(handleMessage);
        
        // Request initial data
        sendToPage({ type: 'request-tree' });
        sendToPage({ type: 'request-state' });
    }
    
    function sendToPage(msg) {
        chrome.devtools.inspectedWindow.eval(
            `window.__DX_DEVTOOLS__ && window.__DX_DEVTOOLS__.receive(${JSON.stringify(msg)})`
        );
    }
    
    function handleMessage(msg) {
        switch (msg.type) {
            case 'component-tree':
                componentTree = msg.root;
                renderComponentTree();
                break;
            case 'state-snapshot':
                stateSnapshot = msg.state;
                renderStateViewer();
                break;
            case 'performance-metrics':
                perfData = msg.metrics;
                renderPerformance();
                break;
            case 'network-event':
                networkEvents.push(msg.event);
                renderNetwork();
                break;
        }
    }
    
    function renderComponentTree() {
        const panel = document.getElementById('sidebar-panel');
        if (!componentTree) {
            panel.innerHTML = '<p>No components detected</p>';
            return;
        }
        panel.innerHTML = renderTreeNode(componentTree);
    }
    
    function renderTreeNode(node, depth = 0) {
        const children = node.children.map(c => renderTreeNode(c, depth + 1)).join('');
        return `
            <div class="tree-node" style="padding-left: ${depth * 15}px">
                <div class="tree-node-header" onclick="selectComponent('${node.id}')">
                    <span class="tree-node-tag">&lt;${node.name}&gt;</span>
                </div>
                ${children}
            </div>
        `;
    }
    
    function renderStateViewer() {
        const panel = document.getElementById('sidebar-panel');
        if (!stateSnapshot) {
            panel.innerHTML = '<p>No state available</p>';
            return;
        }
        
        let html = '';
        for (const [key, value] of Object.entries(stateSnapshot)) {
            const type = typeof value;
            html += `
                <div class="state-entry">
                    <span class="state-key">${key}</span>: 
                    <span class="state-value">${JSON.stringify(value)}</span>
                    <span class="state-type">${type}</span>
                </div>
            `;
        }
        panel.innerHTML = html;
    }
    
    function renderPerformance() {
        const panel = document.getElementById('main-panel');
        if (!perfData) {
            panel.innerHTML = '<p>Start profiling to see performance data</p>';
            return;
        }
        
        panel.innerHTML = `
            <div class="metric">
                <span>FPS</span>
                <span class="metric-value">${perfData.fps.toFixed(1)}</span>
            </div>
            <div class="metric">
                <span>Components</span>
                <span class="metric-value">${perfData.memory.components}</span>
            </div>
            <div class="metric">
                <span>DOM Nodes</span>
                <span class="metric-value">${perfData.memory.domNodes}</span>
            </div>
            <div class="metric">
                <span>WASM Memory</span>
                <span class="metric-value">${(perfData.memory.wasmMemoryBytes / 1024).toFixed(1)} KB</span>
            </div>
            <div class="chart" id="fps-chart"></div>
            <h4>Recent Renders</h4>
            ${perfData.componentRenders.slice(-10).map(r => `
                <div class="metric">
                    <span>${r.componentName}</span>
                    <span class="metric-value">${r.durationUs}µs</span>
                </div>
            `).join('')}
        `;
    }
    
    function renderNetwork() {
        const panel = document.getElementById('main-panel');
        panel.innerHTML = networkEvents.slice(-50).map(e => `
            <div class="network-entry">
                <span class="network-url">${e.url}</span>
                <span class="network-size">${formatBytes(e.size)}</span>
                <span class="network-time">${e.durationMs || '-'}ms</span>
            </div>
        `).join('');
    }
    
    function formatBytes(bytes) {
        if (bytes < 1024) return bytes + ' B';
        if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
        return (bytes / 1024 / 1024).toFixed(1) + ' MB';
    }
    
    window.selectComponent = function(id) {
        selectedComponent = id;
        sendToPage({ type: 'highlight-component', id });
    };
    
    // Tab switching
    document.querySelectorAll('.tab').forEach(tab => {
        tab.addEventListener('click', () => {
            const panel = tab.dataset.panel;
            tab.parentElement.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
            tab.classList.add('active');
            
            switch (panel) {
                case 'components': renderComponentTree(); break;
                case 'state': renderStateViewer(); break;
                case 'perf': renderPerformance(); break;
                case 'network': renderNetwork(); break;
            }
        });
    });
    
    // Connect on load
    connect();
})();
"#
}

/// Generate browser extension manifest
pub fn generate_extension_manifest() -> &'static str {
    r#"{
  "manifest_version": 3,
  "name": "DX DevTools",
  "version": "1.0.0",
  "description": "Developer tools for DX framework applications",
  "permissions": ["activeTab", "scripting"],
  "devtools_page": "devtools.html",
  "icons": {
    "16": "icons/icon16.png",
    "48": "icons/icon48.png",
    "128": "icons/icon128.png"
  },
  "content_scripts": [{
    "matches": ["<all_urls>"],
    "js": ["content-script.js"],
    "run_at": "document_start"
  }],
  "background": {
    "service_worker": "background.js"
  }
}"#
}

/// Generate content script
pub fn generate_content_script() -> &'static str {
    r#"
// DX DevTools Content Script
(function() {
    'use strict';
    
    // Check if DX app
    if (!window.__DX__) return;
    
    // Set up message channel
    window.__DX_DEVTOOLS__ = {
        receive(msg) {
            if (window.__DX_DEVTOOLS_SERVER__) {
                const response = window.__DX_DEVTOOLS_SERVER__.handle(msg);
                if (response) {
                    window.postMessage({ type: 'dx-devtools', payload: response }, '*');
                }
            }
        }
    };
    
    // Forward messages to extension
    window.addEventListener('message', (event) => {
        if (event.data && event.data.type === 'dx-devtools') {
            chrome.runtime.sendMessage(event.data.payload);
        }
    });
})();
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_devtools_server() {
        let mut server = DevToolsServer::new();

        // Register component
        server.register_component(
            "btn-1",
            ComponentNode {
                id: "btn-1".into(),
                name: "Button".into(),
                component_type: "component".into(),
                props: HashMap::new(),
                state: HashMap::new(),
                children: Vec::new(),
                source: None,
                render_count: 1,
                render_time_us: 100,
            },
        );

        // Request tree
        let response = server.handle_message(DevToolsMessage::RequestTree);
        assert!(matches!(response, Some(DevToolsMessage::ComponentTree { .. })));
    }

    #[test]
    fn test_state_snapshots() {
        let mut server = DevToolsServer::new();

        let mut state = HashMap::new();
        state.insert("count".into(), serde_json::json!(0));
        server.record_state_change(state, Some("init"));

        let mut state2 = HashMap::new();
        state2.insert("count".into(), serde_json::json!(1));
        server.record_state_change(state2, Some("increment"));

        assert_eq!(server.snapshots.len(), 2);
    }
}
