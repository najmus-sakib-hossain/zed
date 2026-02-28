//! Hook registry - manages hook scripts and triggers

use dashmap::DashMap;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

use super::engine::{HookEngine, HookError};
use super::events::{HookEvent, HookEventType, HookResult};

/// Hook definition
#[derive(Debug, Clone)]
pub struct HookDefinition {
    pub name: String,
    pub path: PathBuf,
    pub events: Vec<HookEventType>,
    pub function_name: String,
    pub priority: i32,
    pub enabled: bool,
}

/// Hook registry - manages all hooks
pub struct HookRegistry {
    hooks: DashMap<String, HookDefinition>,
    engine: HookEngine,
    hooks_dir: PathBuf,
}

impl HookRegistry {
    /// Create a new hook registry
    pub fn new(hooks_dir: PathBuf) -> Result<Self, HookError> {
        let engine = HookEngine::new()?;

        let registry = Self {
            hooks: DashMap::new(),
            engine,
            hooks_dir,
        };

        // Load hooks from directory if it exists
        if registry.hooks_dir.exists() {
            registry.load_hooks_from_dir()?;
        }

        Ok(registry)
    }

    /// Load all .lua hooks from the hooks directory
    pub fn load_hooks_from_dir(&self) -> Result<(), HookError> {
        if !self.hooks_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.hooks_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("lua") {
                match self.load_hook_file(&path) {
                    Ok(name) => info!("Loaded hook: {}", name),
                    Err(e) => warn!("Failed to load hook {}: {}", path.display(), e),
                }
            }
        }

        Ok(())
    }

    /// Load a single hook file
    pub fn load_hook_file(&self, path: &Path) -> Result<String, HookError> {
        let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();

        self.engine.load_script(path)?;

        // Determine which events this hook handles based on function names
        let events = self.detect_events(&name);

        let hook = HookDefinition {
            name: name.clone(),
            path: path.to_path_buf(),
            events,
            function_name: format!("on_{}", name.replace('-', "_")),
            priority: 0,
            enabled: true,
        };

        self.hooks.insert(name.clone(), hook);
        Ok(name)
    }

    /// Register a hook from a string
    pub fn register_hook(
        &self,
        name: &str,
        script: &str,
        events: Vec<HookEventType>,
    ) -> Result<(), HookError> {
        self.engine.load_script_string(name, script)?;

        let hook = HookDefinition {
            name: name.to_string(),
            path: PathBuf::new(),
            events,
            function_name: name.to_string(),
            priority: 0,
            enabled: true,
        };

        self.hooks.insert(name.to_string(), hook);
        info!("Registered hook: {}", name);
        Ok(())
    }

    /// Trigger hooks for an event
    pub fn trigger(&self, event: &HookEvent) -> Vec<HookResult> {
        let mut results = Vec::new();

        // Get hooks sorted by priority
        let mut matching_hooks: Vec<HookDefinition> = self
            .hooks
            .iter()
            .filter(|h| h.enabled && h.events.contains(&event.event_type))
            .map(|h| h.clone())
            .collect();

        matching_hooks.sort_by_key(|h| h.priority);

        for hook in &matching_hooks {
            match self.engine.execute_hook(&hook.function_name, event) {
                Ok(result) => {
                    let should_stop = !result.propagate;
                    results.push(result);
                    if should_stop {
                        break;
                    }
                }
                Err(e) => {
                    warn!("Hook '{}' failed: {}", hook.name, e);
                }
            }
        }

        results
    }

    /// Enable/disable a hook
    pub fn set_enabled(&self, name: &str, enabled: bool) {
        if let Some(mut hook) = self.hooks.get_mut(name) {
            hook.enabled = enabled;
        }
    }

    /// Remove a hook
    pub fn remove_hook(&self, name: &str) {
        self.hooks.remove(name);
    }

    /// List all hooks
    pub fn list_hooks(&self) -> Vec<HookDefinition> {
        self.hooks.iter().map(|h| h.clone()).collect()
    }

    /// Detect event types based on common naming conventions
    fn detect_events(&self, name: &str) -> Vec<HookEventType> {
        let mut events = Vec::new();
        let func_name = format!("on_{}", name.replace('-', "_"));

        if name.contains("message") || self.engine.has_function("on_message") {
            events.push(HookEventType::MessageReceived);
        }
        if name.contains("session") || self.engine.has_function("on_session_start") {
            events.push(HookEventType::SessionStart);
        }
        if name.contains("file") || self.engine.has_function("on_file_changed") {
            events.push(HookEventType::FileChanged);
        }
        if name.contains("command") || self.engine.has_function("on_command") {
            events.push(HookEventType::CommandExecuted);
        }
        if name.contains("error") || self.engine.has_function("on_error") {
            events.push(HookEventType::Error);
        }
        if self.engine.has_function(&func_name) {
            events.push(HookEventType::Custom(name.to_string()));
        }

        if events.is_empty() {
            // Default: trigger on message events
            events.push(HookEventType::MessageReceived);
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = HookRegistry::new(PathBuf::from("/tmp/dx-hooks-test")).unwrap();
        assert!(registry.list_hooks().is_empty());
    }

    #[test]
    fn test_register_and_trigger() {
        let registry = HookRegistry::new(PathBuf::from("/tmp/dx-hooks-test")).unwrap();

        registry
            .register_hook(
                "on_message",
                r#"
                function on_message(event)
                    log("Hook triggered!")
                    return true
                end
                "#,
                vec![HookEventType::MessageReceived],
            )
            .unwrap();

        assert_eq!(registry.list_hooks().len(), 1);

        let event = HookEvent::new(HookEventType::MessageReceived, "test");
        let results = registry.trigger(&event);
        assert_eq!(results.len(), 1);
        assert!(results[0].propagate);
    }

    #[test]
    fn test_enable_disable() {
        let registry = HookRegistry::new(PathBuf::from("/tmp/dx-hooks-test")).unwrap();

        registry
            .register_hook(
                "test_hook",
                r#"function test_hook(event) return true end"#,
                vec![HookEventType::MessageReceived],
            )
            .unwrap();

        // Disable hook
        registry.set_enabled("test_hook", false);

        let event = HookEvent::new(HookEventType::MessageReceived, "test");
        let results = registry.trigger(&event);
        assert!(results.is_empty()); // Disabled hook should not trigger

        // Re-enable
        registry.set_enabled("test_hook", true);
        let results = registry.trigger(&event);
        assert_eq!(results.len(), 1);
    }
}
