//! Lua hook execution engine

use std::path::Path;
use thiserror::Error;
use tracing::debug;

use mlua::{Function, Lua, Value};

use super::events::{HookEvent, HookResult};

/// Hook engine errors
#[derive(Debug, Error)]
pub enum HookError {
    #[error("Lua error: {0}")]
    Lua(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Script not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialize(String),
}

impl From<mlua::Error> for HookError {
    fn from(e: mlua::Error) -> Self {
        HookError::Lua(e.to_string())
    }
}

/// Lua hook execution engine with sandboxing
pub struct HookEngine {
    lua: Lua,
}

impl HookEngine {
    /// Create a new sandboxed Lua engine
    pub fn new() -> Result<Self, HookError> {
        let lua = Lua::new();

        // Set up sandboxed environment: remove dangerous functions
        lua.load(
            r#"
            -- Remove dangerous globals for sandboxing
            os.execute = nil
            os.exit = nil
            os.remove = nil
            os.rename = nil
            os.tmpname = nil
            io = nil
            loadfile = nil
            dofile = nil
            
            -- Provide safe utility functions
            function log(msg)
                _dx_logs = _dx_logs or {}
                table.insert(_dx_logs, tostring(msg))
            end
            
            function notify(msg)
                log("[NOTIFY] " .. tostring(msg))
            end
            "#,
        )
        .exec()?;

        Ok(Self { lua })
    }

    /// Load a hook script from file
    pub fn load_script(&self, path: &Path) -> Result<(), HookError> {
        let content = std::fs::read_to_string(path)?;
        self.lua.load(&content).exec()?;
        debug!("Loaded hook script: {}", path.display());
        Ok(())
    }

    /// Load a hook script from string
    pub fn load_script_string(&self, name: &str, script: &str) -> Result<(), HookError> {
        self.lua.load(script).set_name(name).exec()?;
        debug!("Loaded hook script: {}", name);
        Ok(())
    }

    /// Execute a hook function
    pub fn execute_hook(
        &self,
        function_name: &str,
        event: &HookEvent,
    ) -> Result<HookResult, HookError> {
        // Clear logs
        self.lua.load("_dx_logs = {}").exec()?;

        // Try to call the function
        let globals = self.lua.globals();
        let func: Function = match globals.get(function_name) {
            Ok(f) => f,
            Err(_) => {
                debug!("Hook function '{}' not found", function_name);
                return Ok(HookResult::default());
            }
        };

        // Create event data as Lua table
        let event_table = self.lua.create_table()?;
        event_table.set("type", format!("{:?}", event.event_type))?;
        event_table.set("source", event.source.clone())?;

        let data_table = self.lua.create_table()?;
        for (key, value) in &event.data {
            match value {
                serde_json::Value::String(s) => data_table.set(key.as_str(), s.as_str())?,
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        data_table.set(key.as_str(), i)?;
                    } else if let Some(f) = n.as_f64() {
                        data_table.set(key.as_str(), f)?;
                    }
                }
                serde_json::Value::Bool(b) => data_table.set(key.as_str(), *b)?,
                _ => data_table.set(key.as_str(), value.to_string())?,
            }
        }
        event_table.set("data", data_table)?;

        // Call the function
        let result: Value = func.call(event_table)?;

        // Collect logs
        let logs: Vec<String> = self
            .lua
            .load("return _dx_logs or {}")
            .eval::<mlua::Table>()
            .ok()
            .map(|t| {
                let mut logs = Vec::new();
                for pair in t.pairs::<i64, String>() {
                    if let Ok((_, v)) = pair {
                        logs.push(v);
                    }
                }
                logs
            })
            .unwrap_or_default();

        // Parse result
        let propagate = !matches!(&result, Value::Boolean(false));

        Ok(HookResult {
            propagate,
            modified_data: None,
            logs,
        })
    }

    /// Check if a function exists in the loaded scripts
    pub fn has_function(&self, name: &str) -> bool {
        self.lua.globals().get::<Function>(name).is_ok()
    }
}

impl Default for HookEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create Lua engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::HookEventType;

    #[test]
    fn test_engine_creation() {
        let engine = HookEngine::new().unwrap();
        assert!(!engine.has_function("nonexistent"));
    }

    #[test]
    fn test_load_and_execute_script() {
        let engine = HookEngine::new().unwrap();

        engine
            .load_script_string(
                "test",
                r#"
                function on_message(event)
                    log("Received: " .. (event.data.content or "unknown"))
                    return true
                end
                "#,
            )
            .unwrap();

        assert!(engine.has_function("on_message"));

        let event = HookEvent::new(HookEventType::MessageReceived, "test")
            .with_data("content", serde_json::json!("Hello!"));

        let result = engine.execute_hook("on_message", &event).unwrap();
        assert!(result.propagate);
        assert!(!result.logs.is_empty());
        assert!(result.logs[0].contains("Received: Hello!"));
    }

    #[test]
    fn test_hook_can_stop_propagation() {
        let engine = HookEngine::new().unwrap();

        engine
            .load_script_string(
                "test",
                r#"
                function on_spam(event)
                    if event.data.content == "spam" then
                        log("Blocked spam!")
                        return false
                    end
                    return true
                end
                "#,
            )
            .unwrap();

        let event = HookEvent::new(HookEventType::MessageReceived, "test")
            .with_data("content", serde_json::json!("spam"));

        let result = engine.execute_hook("on_spam", &event).unwrap();
        assert!(!result.propagate);
    }

    #[test]
    fn test_sandboxing() {
        let engine = HookEngine::new().unwrap();

        // os.execute should be nil (sandboxed)
        let result = engine.load_script_string(
            "test",
            r#"
            function test_sandbox(event)
                if os.execute then
                    log("UNSAFE: os.execute available")
                    return false
                else
                    log("SAFE: os.execute blocked")
                    return true
                end
            end
            "#,
        );
        assert!(result.is_ok());

        let event = HookEvent::new(HookEventType::Custom("test".into()), "test");
        let result = engine.execute_hook("test_sandbox", &event).unwrap();
        assert!(result.propagate); // Should be true because os.execute is nil
        assert!(result.logs[0].contains("SAFE"));
    }
}
