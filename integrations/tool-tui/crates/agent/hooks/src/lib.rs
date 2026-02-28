//! DX Agent Hooks - Lua scripting engine for automation
//!
//! Provides a sandboxed Lua runtime for user-defined hooks that trigger
//! on events like messages, file changes, commands, etc.

pub mod engine;
pub mod events;
pub mod registry;

pub use engine::{HookEngine, HookError};
pub use events::{HookEvent, HookEventType};
pub use registry::HookRegistry;
