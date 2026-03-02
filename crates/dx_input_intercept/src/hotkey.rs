//! Global hotkey management for DX.
//!
//! Uses `global-hotkey` patterns for cross-platform hotkey bindings
//! that work regardless of which application is in the foreground.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

/// A hotkey binding combining modifiers and a key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HotkeyBinding {
    pub key: HotkeyKey,
    pub modifiers: HotkeyModifiers,
    pub description: String,
}

/// Modifier keys for hotkey bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HotkeyModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    /// Command on macOS, Super/Win on Windows/Linux.
    pub meta: bool,
}

/// Common keys for hotkey bindings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HotkeyKey {
    Char(char),
    Space,
    Tab,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

/// Action triggered by a hotkey.
#[derive(Debug, Clone)]
pub enum HotkeyAction {
    /// Toggle voice input (start/stop listening).
    ToggleVoice,
    /// Open the floating AI panel.
    OpenAiPanel,
    /// Toggle system-wide grammar checking.
    ToggleGrammar,
    /// Accept the current suggestion.
    AcceptSuggestion,
    /// Dismiss the current suggestion.
    DismissSuggestion,
    /// Open command palette with DX commands.
    DxCommandPalette,
    /// Custom action with string identifier.
    Custom(String),
}

/// Manages global hotkey registrations.
pub struct HotkeyManager {
    bindings: HashMap<HotkeyBinding, HotkeyAction>,
    handlers: HashMap<String, Arc<dyn Fn(&HotkeyAction) + Send + Sync>>,
    registered: bool,
}

impl HotkeyManager {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            handlers: HashMap::new(),
            registered: false,
        }
    }

    /// Register default DX hotkeys.
    pub fn register_defaults(&mut self) -> Result<()> {
        // Default hotkey bindings:
        // Ctrl+Shift+Space — Toggle voice input
        self.register(
            HotkeyBinding {
                key: HotkeyKey::Space,
                modifiers: HotkeyModifiers {
                    control: true,
                    shift: true,
                    ..Default::default()
                },
                description: "Toggle voice input".to_string(),
            },
            HotkeyAction::ToggleVoice,
        )?;

        // Ctrl+Shift+A — Open AI panel
        self.register(
            HotkeyBinding {
                key: HotkeyKey::Char('a'),
                modifiers: HotkeyModifiers {
                    control: true,
                    shift: true,
                    ..Default::default()
                },
                description: "Open AI panel".to_string(),
            },
            HotkeyAction::OpenAiPanel,
        )?;

        // Ctrl+Shift+G — Toggle grammar
        self.register(
            HotkeyBinding {
                key: HotkeyKey::Char('g'),
                modifiers: HotkeyModifiers {
                    control: true,
                    shift: true,
                    ..Default::default()
                },
                description: "Toggle grammar checking".to_string(),
            },
            HotkeyAction::ToggleGrammar,
        )?;

        log::info!("Default DX hotkeys registered");
        Ok(())
    }

    /// Register a hotkey binding.
    pub fn register(&mut self, binding: HotkeyBinding, action: HotkeyAction) -> Result<()> {
        log::debug!("Registering hotkey: {:?} → {:?}", binding, action);
        self.bindings.insert(binding, action);
        // global-hotkey crate integration pending
        Ok(())
    }

    /// Unregister a hotkey binding.
    pub fn unregister(&mut self, binding: &HotkeyBinding) -> Result<()> {
        self.bindings.remove(binding);
        log::debug!("Unregistered hotkey: {:?}", binding);
        Ok(())
    }

    /// Set a handler for a specific action type.
    pub fn on_action(
        &mut self,
        name: &str,
        handler: impl Fn(&HotkeyAction) + Send + Sync + 'static,
    ) {
        self.handlers.insert(name.to_string(), Arc::new(handler));
    }

    /// List all registered bindings.
    pub fn bindings(&self) -> impl Iterator<Item = (&HotkeyBinding, &HotkeyAction)> {
        self.bindings.iter()
    }

    /// Start listening for hotkey events.
    pub fn start(&mut self) -> Result<()> {
        if self.registered {
            return Ok(());
        }
        // global-hotkey crate: GlobalHotKeyManager::new() + register_all
        self.registered = true;
        log::info!("Global hotkey listener started");
        Ok(())
    }

    /// Stop listening for hotkey events.
    pub fn stop(&mut self) {
        self.registered = false;
        log::info!("Global hotkey listener stopped");
    }
}

impl Default for HotkeyManager {
    fn default() -> Self {
        Self::new()
    }
}
