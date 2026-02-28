//! Computer use actions — mouse, keyboard, and system automation.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A computer action that can be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputerAction {
    /// Move mouse to absolute position.
    MouseMove { x: i32, y: i32 },
    /// Click at current position.
    MouseClick {
        button: MouseButton,
        clicks: u32,
    },
    /// Drag from current position to target.
    MouseDrag { to_x: i32, to_y: i32 },
    /// Scroll wheel.
    Scroll { delta_x: i32, delta_y: i32 },
    /// Type text string.
    TypeText { text: String },
    /// Press a key combination.
    KeyPress { keys: Vec<Key> },
    /// Take a screenshot.
    Screenshot,
    /// Wait for a duration (ms).
    Wait { ms: u64 },
    /// Open an application.
    OpenApp { name: String },
    /// Run a shell command.
    RunCommand { command: String },
}

/// Mouse button types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Keyboard keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Escape,
    Backspace,
    Delete,
    Space,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Ctrl,
    Alt,
    Shift,
    Meta, // Win/Cmd
    F(u8),
}

/// State of the computer use agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Planning,
    Executing,
    WaitingForUser,
    Error,
}

/// The computer use agent that executes action sequences.
pub struct ComputerUseAgent {
    state: AgentState,
    action_history: Vec<ComputerAction>,
    max_actions_per_session: usize,
}

impl ComputerUseAgent {
    pub fn new() -> Self {
        Self {
            state: AgentState::Idle,
            action_history: Vec::new(),
            max_actions_per_session: 100,
        }
    }

    pub fn state(&self) -> AgentState {
        self.state
    }

    pub fn history(&self) -> &[ComputerAction] {
        &self.action_history
    }

    /// Execute a single action.
    pub fn execute(&mut self, action: ComputerAction) -> Result<()> {
        if self.action_history.len() >= self.max_actions_per_session {
            return Err(anyhow::anyhow!("Max actions per session exceeded"));
        }

        self.state = AgentState::Executing;

        // Placeholder — real implementation uses platform APIs
        match &action {
            ComputerAction::MouseMove { x, y } => {
                log::info!("Mouse move to ({}, {})", x, y);
            }
            ComputerAction::MouseClick { button, clicks } => {
                log::info!("Mouse click {:?} x{}", button, clicks);
            }
            ComputerAction::TypeText { text } => {
                log::info!("Typing {} chars", text.len());
            }
            ComputerAction::KeyPress { keys } => {
                log::info!("Key press: {:?}", keys);
            }
            ComputerAction::Screenshot => {
                log::info!("Taking screenshot");
            }
            ComputerAction::Wait { ms } => {
                log::info!("Waiting {}ms", ms);
            }
            ComputerAction::RunCommand { command } => {
                log::info!("Running command: {}", command);
            }
            _ => {
                log::info!("Executing action: {:?}", action);
            }
        }

        self.action_history.push(action);
        self.state = AgentState::Idle;
        Ok(())
    }

    /// Execute a sequence of actions.
    pub fn execute_sequence(&mut self, actions: Vec<ComputerAction>) -> Result<()> {
        for action in actions {
            self.execute(action)?;
        }
        Ok(())
    }

    /// Reset the agent state and history.
    pub fn reset(&mut self) {
        self.state = AgentState::Idle;
        self.action_history.clear();
    }
}

impl Default for ComputerUseAgent {
    fn default() -> Self {
        Self::new()
    }
}
