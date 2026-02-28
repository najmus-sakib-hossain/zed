//! Plugin Hook System Module
//!
//! Provides an event-based hook system that allows plugins to respond
//! to lifecycle events in the DX CLI. Hooks are registered by plugins
//! and executed at specific points in the application flow.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;

use super::traits::{DxPlugin, PluginContext};

/// Known hook event names
pub mod events {
    /// Fired before a chat message is processed
    pub const BEFORE_CHAT: &str = "before_chat";
    /// Fired after a chat message is processed
    pub const AFTER_CHAT: &str = "after_chat";
    /// Fired before a command is executed
    pub const BEFORE_EXEC: &str = "before_exec";
    /// Fired after a command is executed
    pub const AFTER_EXEC: &str = "after_exec";
    /// Fired when a client connects
    pub const ON_CONNECT: &str = "on_connect";
    /// Fired when a client disconnects
    pub const ON_DISCONNECT: &str = "on_disconnect";
    /// Fired when a message is received from a channel
    pub const ON_MESSAGE: &str = "on_message";
    /// Fired when an error occurs
    pub const ON_ERROR: &str = "on_error";
    /// Fired before storing a memory
    pub const BEFORE_MEMORY_STORE: &str = "before_memory_store";
    /// Fired after storing a memory
    pub const AFTER_MEMORY_STORE: &str = "after_memory_store";
    /// Fired before creating a session
    pub const BEFORE_SESSION_CREATE: &str = "before_session_create";
    /// Fired after creating a session
    pub const AFTER_SESSION_CREATE: &str = "after_session_create";
    /// Fired when configuration is reloaded
    pub const ON_CONFIG_RELOAD: &str = "on_config_reload";
    /// Fired on gateway startup
    pub const ON_STARTUP: &str = "on_startup";
    /// Fired on gateway shutdown
    pub const ON_SHUTDOWN: &str = "on_shutdown";
}

/// A registered hook handler
#[derive(Debug, Clone)]
pub struct HookHandler {
    /// Plugin name that registered the hook
    pub plugin_name: String,
    /// Handler function/method name within the plugin
    pub handler_name: String,
    /// Priority (lower = earlier)
    pub priority: i32,
    /// Optional filter expression
    pub filter: Option<String>,
}

/// Hook execution result
#[derive(Debug, Clone)]
pub struct HookExecutionResult {
    /// Number of handlers that executed
    pub handlers_executed: usize,
    /// Results from each handler
    pub results: Vec<HandlerResult>,
    /// Total duration
    pub duration: std::time::Duration,
}

/// Result from a single hook handler
#[derive(Debug, Clone)]
pub struct HandlerResult {
    /// Plugin name
    pub plugin_name: String,
    /// Handler name
    pub handler_name: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Handler output (if any)
    pub output: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Handler execution duration
    pub duration: std::time::Duration,
}

/// Hook data passed to handlers
#[derive(Debug, Clone, Default)]
pub struct HookData {
    /// Event name
    pub event: String,
    /// Arbitrary key-value data for the hook
    pub data: HashMap<String, serde_json::Value>,
}

impl HookData {
    /// Create new hook data for an event
    pub fn new(event: &str) -> Self {
        Self {
            event: event.to_string(),
            data: HashMap::new(),
        }
    }

    /// Add a data field
    pub fn with(mut self, key: &str, value: serde_json::Value) -> Self {
        self.data.insert(key.to_string(), value);
        self
    }

    /// Convert to JSON for passing to plugins
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "event": self.event,
            "data": self.data,
        })
    }
}

/// Hook system that manages hook registrations and execution
pub struct HookSystem {
    /// Registered hooks: event name → sorted list of handlers
    hooks: RwLock<HashMap<String, Vec<HookHandler>>>,
    /// Plugin instances for execution
    plugins: Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn DxPlugin>>>>>>,
}

impl HookSystem {
    /// Create a new hook system
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(HashMap::new()),
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a hook system with shared plugin instances
    pub fn with_plugins(
        plugins: Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn DxPlugin>>>>>>,
    ) -> Self {
        Self {
            hooks: RwLock::new(HashMap::new()),
            plugins,
        }
    }

    /// Register a hook handler for an event
    pub async fn register(
        &self,
        event: &str,
        plugin_name: &str,
        handler_name: &str,
        priority: i32,
        filter: Option<String>,
    ) {
        let handler = HookHandler {
            plugin_name: plugin_name.to_string(),
            handler_name: handler_name.to_string(),
            priority,
            filter,
        };

        let mut hooks = self.hooks.write().await;
        let handlers = hooks.entry(event.to_string()).or_insert_with(Vec::new);
        handlers.push(handler);

        // Sort by priority (lowest first)
        handlers.sort_by_key(|h| h.priority);
    }

    /// Unregister all hooks for a plugin
    pub async fn unregister_plugin(&self, plugin_name: &str) {
        let mut hooks = self.hooks.write().await;
        for handlers in hooks.values_mut() {
            handlers.retain(|h| h.plugin_name != plugin_name);
        }
    }

    /// Unregister a specific hook handler
    pub async fn unregister(&self, event: &str, plugin_name: &str, handler_name: &str) {
        let mut hooks = self.hooks.write().await;
        if let Some(handlers) = hooks.get_mut(event) {
            handlers.retain(|h| !(h.plugin_name == plugin_name && h.handler_name == handler_name));
        }
    }

    /// Execute all hooks for an event
    pub async fn execute(&self, hook_data: &HookData) -> HookExecutionResult {
        let started = Instant::now();
        let mut handler_results = Vec::new();

        let handlers = {
            let hooks = self.hooks.read().await;
            hooks.get(&hook_data.event).cloned().unwrap_or_default()
        };

        let plugins = self.plugins.read().await;

        for handler in &handlers {
            // Check filter
            if let Some(ref _filter) = handler.filter {
                // TODO: Implement filter expression evaluation
                // For now, always execute
            }

            let handler_start = Instant::now();

            let result = if let Some(plugin) = plugins.get(&handler.plugin_name) {
                let plugin = plugin.read().await;

                // Build context with hook data
                let ctx = PluginContext::default().with_args(vec![
                    hook_data.to_json().to_string(),
                    handler.handler_name.clone(),
                ]);

                match plugin.execute(&ctx).await {
                    Ok(pr) => HandlerResult {
                        plugin_name: handler.plugin_name.clone(),
                        handler_name: handler.handler_name.clone(),
                        success: pr.is_success(),
                        output: if pr.stdout.is_empty() {
                            None
                        } else {
                            Some(pr.stdout)
                        },
                        error: if pr.stderr.is_empty() {
                            None
                        } else {
                            Some(pr.stderr)
                        },
                        duration: handler_start.elapsed(),
                    },
                    Err(e) => HandlerResult {
                        plugin_name: handler.plugin_name.clone(),
                        handler_name: handler.handler_name.clone(),
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                        duration: handler_start.elapsed(),
                    },
                }
            } else {
                HandlerResult {
                    plugin_name: handler.plugin_name.clone(),
                    handler_name: handler.handler_name.clone(),
                    success: false,
                    output: None,
                    error: Some("Plugin not found or not loaded".to_string()),
                    duration: handler_start.elapsed(),
                }
            };

            handler_results.push(result);
        }

        HookExecutionResult {
            handlers_executed: handler_results.len(),
            results: handler_results,
            duration: started.elapsed(),
        }
    }

    /// List all registered hooks
    pub async fn list_hooks(&self) -> HashMap<String, Vec<HookHandler>> {
        self.hooks.read().await.clone()
    }

    /// List hooks for a specific event
    pub async fn hooks_for_event(&self, event: &str) -> Vec<HookHandler> {
        self.hooks.read().await.get(event).cloned().unwrap_or_default()
    }

    /// Check if any hooks are registered for an event
    pub async fn has_hooks(&self, event: &str) -> bool {
        let hooks = self.hooks.read().await;
        hooks.get(event).map(|h| !h.is_empty()).unwrap_or(false)
    }

    /// Get count of registered handlers across all events
    pub async fn total_handler_count(&self) -> usize {
        let hooks = self.hooks.read().await;
        hooks.values().map(|h| h.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_hook() {
        let system = HookSystem::new();
        system
            .register(events::BEFORE_CHAT, "test-plugin", "handle_chat", 100, None)
            .await;

        let handlers = system.hooks_for_event(events::BEFORE_CHAT).await;
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].plugin_name, "test-plugin");
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let system = HookSystem::new();
        system.register(events::BEFORE_CHAT, "plugin-a", "handler_a", 200, None).await;
        system.register(events::BEFORE_CHAT, "plugin-b", "handler_b", 50, None).await;
        system.register(events::BEFORE_CHAT, "plugin-c", "handler_c", 100, None).await;

        let handlers = system.hooks_for_event(events::BEFORE_CHAT).await;
        assert_eq!(handlers[0].plugin_name, "plugin-b"); // priority 50
        assert_eq!(handlers[1].plugin_name, "plugin-c"); // priority 100
        assert_eq!(handlers[2].plugin_name, "plugin-a"); // priority 200
    }

    #[tokio::test]
    async fn test_unregister_plugin() {
        let system = HookSystem::new();
        system.register(events::BEFORE_CHAT, "plugin-a", "h1", 100, None).await;
        system.register(events::AFTER_CHAT, "plugin-a", "h2", 100, None).await;
        system.register(events::BEFORE_CHAT, "plugin-b", "h3", 100, None).await;

        system.unregister_plugin("plugin-a").await;

        assert_eq!(system.hooks_for_event(events::BEFORE_CHAT).await.len(), 1);
        assert!(system.hooks_for_event(events::AFTER_CHAT).await.is_empty());
    }

    #[tokio::test]
    async fn test_unregister_specific() {
        let system = HookSystem::new();
        system.register(events::BEFORE_CHAT, "plugin-a", "h1", 100, None).await;
        system.register(events::BEFORE_CHAT, "plugin-a", "h2", 200, None).await;

        system.unregister(events::BEFORE_CHAT, "plugin-a", "h1").await;

        let handlers = system.hooks_for_event(events::BEFORE_CHAT).await;
        assert_eq!(handlers.len(), 1);
        assert_eq!(handlers[0].handler_name, "h2");
    }

    #[tokio::test]
    async fn test_has_hooks() {
        let system = HookSystem::new();
        assert!(!system.has_hooks(events::BEFORE_CHAT).await);

        system.register(events::BEFORE_CHAT, "plugin", "handler", 100, None).await;
        assert!(system.has_hooks(events::BEFORE_CHAT).await);
        assert!(!system.has_hooks(events::AFTER_CHAT).await);
    }

    #[tokio::test]
    async fn test_total_handler_count() {
        let system = HookSystem::new();
        assert_eq!(system.total_handler_count().await, 0);

        system.register(events::BEFORE_CHAT, "p1", "h1", 100, None).await;
        system.register(events::AFTER_CHAT, "p1", "h2", 100, None).await;
        system.register(events::ON_ERROR, "p2", "h3", 100, None).await;

        assert_eq!(system.total_handler_count().await, 3);
    }

    #[tokio::test]
    async fn test_execute_no_handlers() {
        let system = HookSystem::new();
        let data = HookData::new(events::BEFORE_CHAT);
        let result = system.execute(&data).await;
        assert_eq!(result.handlers_executed, 0);
    }

    #[tokio::test]
    async fn test_execute_missing_plugin() {
        let system = HookSystem::new();
        system.register(events::BEFORE_CHAT, "nonexistent", "handler", 100, None).await;

        let data = HookData::new(events::BEFORE_CHAT);
        let result = system.execute(&data).await;
        assert_eq!(result.handlers_executed, 1);
        assert!(!result.results[0].success);
        assert!(result.results[0].error.is_some());
    }

    #[test]
    fn test_hook_data() {
        let data = HookData::new("test_event")
            .with("user", serde_json::json!("alice"))
            .with("count", serde_json::json!(42));

        let json = data.to_json();
        assert_eq!(json["event"], "test_event");
        assert_eq!(json["data"]["user"], "alice");
        assert_eq!(json["data"]["count"], 42);
    }

    #[test]
    fn test_list_hooks_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let system = HookSystem::new();
            let hooks = system.list_hooks().await;
            assert!(hooks.is_empty());
        });
    }
}
