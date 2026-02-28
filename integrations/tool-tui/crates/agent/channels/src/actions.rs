//! Action handlers and callback dispatch.
//!
//! When a user clicks a button, selects a menu item, or
//! issues an inline command, the platform delivers a callback
//! that is routed through this registry.

use anyhow::{Result, bail};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Kind of interactive action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionType {
    /// Button press.
    Button,
    /// Menu / select choice.
    Select,
    /// Slash command.
    Command,
    /// Generic callback.
    Callback,
}

/// Describes a single action a user can trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action identifier (e.g. `"approve_pr"`).
    pub id: String,
    /// Kind of action.
    pub action_type: ActionType,
    /// Human-readable label.
    pub label: String,
    /// Arbitrary payload data.
    pub data: serde_json::Value,
}

/// Context delivered alongside a triggered action.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// Channel the action originated from.
    pub channel_id: String,
    /// Message the action is attached to.
    pub message_id: String,
    /// User who triggered the action.
    pub user_id: String,
    /// The action itself.
    pub action: Action,
}

/// Result returned after handling an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded.
    pub success: bool,
    /// Optional response message shown to the user.
    pub message: Option<String>,
    /// Optional replacement data for the action UI.
    pub data: Option<serde_json::Value>,
}

impl ActionResult {
    /// Shorthand for a successful result with a message.
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
            data: None,
        }
    }

    /// Shorthand for a failed result.
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: None,
        }
    }
}

/// Trait for action handler implementations.
pub trait ActionHandler: Send + Sync {
    /// Handle an incoming action and return a result.
    fn handle(&self, ctx: &ActionContext) -> Result<ActionResult>;
}

/// Blanket impl: any `Fn(&ActionContext) -> Result<ActionResult>`
/// that is `Send + Sync` is an `ActionHandler`.
impl<F> ActionHandler for F
where
    F: Fn(&ActionContext) -> Result<ActionResult> + Send + Sync,
{
    fn handle(&self, ctx: &ActionContext) -> Result<ActionResult> {
        (self)(ctx)
    }
}

/// Registry of action handlers keyed by action ID.
///
/// Thread-safe; can be shared across tasks with `Arc`.
#[derive(Clone)]
pub struct ActionRegistry {
    handlers: Arc<DashMap<String, Arc<dyn ActionHandler>>>,
}

impl ActionRegistry {
    /// Create a new, empty registry.
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(DashMap::new()),
        }
    }

    /// Register a handler for an action ID.
    pub fn register_handler(&self, action_id: impl Into<String>, handler: Arc<dyn ActionHandler>) {
        self.handlers.insert(action_id.into(), handler);
    }

    /// Register a closure as a handler.
    pub fn on<F>(&self, action_id: impl Into<String>, f: F)
    where
        F: Fn(&ActionContext) -> Result<ActionResult> + Send + Sync + 'static,
    {
        self.handlers.insert(action_id.into(), Arc::new(f));
    }

    /// Dispatch an action context to the matching handler.
    pub fn handle_action(&self, ctx: &ActionContext) -> Result<ActionResult> {
        match self.handlers.get(&ctx.action.id) {
            Some(handler) => handler.handle(ctx),
            None => bail!("No handler registered for action '{}'", ctx.action.id),
        }
    }

    /// Check whether a handler exists for the given action ID.
    pub fn has_handler(&self, action_id: &str) -> bool {
        self.handlers.contains_key(action_id)
    }

    /// List all registered action IDs.
    pub fn list_actions(&self) -> Vec<String> {
        self.handlers.iter().map(|r| r.key().clone()).collect()
    }

    /// Remove a handler.
    pub fn remove_handler(&self, action_id: &str) {
        self.handlers.remove(action_id);
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_ctx(action_id: &str) -> ActionContext {
        ActionContext {
            channel_id: "tg".into(),
            message_id: "msg-1".into(),
            user_id: "user-1".into(),
            action: Action {
                id: action_id.into(),
                action_type: ActionType::Button,
                label: "Click me".into(),
                data: serde_json::Value::Null,
            },
        }
    }

    #[test]
    fn test_register_and_handle() {
        let reg = ActionRegistry::new();
        reg.on("greet", |_ctx| Ok(ActionResult::ok("Hello!")));

        let ctx = sample_ctx("greet");
        let result = reg.handle_action(&ctx).expect("should succeed");
        assert!(result.success);
        assert_eq!(result.message.as_deref(), Some("Hello!"));
    }

    #[test]
    fn test_unknown_action() {
        let reg = ActionRegistry::new();
        let ctx = sample_ctx("nonexistent");
        let result = reg.handle_action(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_has_handler() {
        let reg = ActionRegistry::new();
        assert!(!reg.has_handler("x"));
        reg.on("x", |_| Ok(ActionResult::ok("ok")));
        assert!(reg.has_handler("x"));
    }

    #[test]
    fn test_list_actions() {
        let reg = ActionRegistry::new();
        reg.on("a", |_| Ok(ActionResult::ok("")));
        reg.on("b", |_| Ok(ActionResult::ok("")));

        let list = reg.list_actions();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_remove_handler() {
        let reg = ActionRegistry::new();
        reg.on("x", |_| Ok(ActionResult::ok("")));
        assert!(reg.has_handler("x"));
        reg.remove_handler("x");
        assert!(!reg.has_handler("x"));
    }

    #[test]
    fn test_action_result_helpers() {
        let ok = ActionResult::ok("done");
        assert!(ok.success);
        let fail = ActionResult::fail("oops");
        assert!(!fail.success);
    }
}
