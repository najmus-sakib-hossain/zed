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

    /// Execute a single action with real platform input.
    pub fn execute(&mut self, action: ComputerAction) -> Result<()> {
        if self.action_history.len() >= self.max_actions_per_session {
            return Err(anyhow::anyhow!("Max actions per session exceeded"));
        }

        self.state = AgentState::Executing;

        match &action {
            ComputerAction::MouseMove { x, y } => {
                log::info!("Mouse move to ({}, {})", x, y);
                crate::input::mouse_move(*x, *y)?;
            }
            ComputerAction::MouseClick { button, clicks } => {
                log::info!("Mouse click {:?} x{}", button, clicks);
                let btn = match button {
                    MouseButton::Left => crate::input::MouseBtn::Left,
                    MouseButton::Right => crate::input::MouseBtn::Right,
                    MouseButton::Middle => crate::input::MouseBtn::Middle,
                };
                crate::input::mouse_click(btn, *clicks)?;
            }
            ComputerAction::MouseDrag { to_x, to_y } => {
                log::info!("Mouse drag to ({}, {})", to_x, to_y);
                crate::input::mouse_drag(*to_x, *to_y)?;
            }
            ComputerAction::Scroll { delta_x, delta_y } => {
                log::info!("Scroll ({}, {})", delta_x, delta_y);
                crate::input::scroll(*delta_x, *delta_y)?;
            }
            ComputerAction::TypeText { text } => {
                log::info!("Typing {} chars", text.len());
                crate::input::type_text(text)?;
            }
            ComputerAction::KeyPress { keys } => {
                log::info!("Key press: {:?}", keys);
                let key_strs: Vec<String> = keys.iter().map(|k| key_to_string(k)).collect();
                let key_refs: Vec<&str> = key_strs.iter().map(|s| s.as_str()).collect();
                crate::input::key_press(&key_refs)?;
            }
            ComputerAction::Screenshot => {
                log::info!("Taking screenshot");
                let _png = crate::capture::capture_full_screen()?;
            }
            ComputerAction::Wait { ms } => {
                log::info!("Waiting {}ms", ms);
                std::thread::sleep(std::time::Duration::from_millis(*ms));
            }
            ComputerAction::OpenApp { name } => {
                log::info!("Opening app: {}", name);
                open_application(name)?;
            }
            ComputerAction::RunCommand { command } => {
                log::info!("Running command: {}", command);
                run_shell_command(command)?;
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

/// Convert a Key enum to its string name for the input module.
fn key_to_string(key: &Key) -> String {
    match key {
        Key::Char(c) => c.to_string(),
        Key::Enter => "enter".to_string(),
        Key::Tab => "tab".to_string(),
        Key::Escape => "escape".to_string(),
        Key::Backspace => "backspace".to_string(),
        Key::Delete => "delete".to_string(),
        Key::Space => "space".to_string(),
        Key::Up => "up".to_string(),
        Key::Down => "down".to_string(),
        Key::Left => "left".to_string(),
        Key::Right => "right".to_string(),
        Key::Home => "home".to_string(),
        Key::End => "end".to_string(),
        Key::PageUp => "pageup".to_string(),
        Key::PageDown => "pagedown".to_string(),
        Key::Ctrl => "ctrl".to_string(),
        Key::Alt => "alt".to_string(),
        Key::Shift => "shift".to_string(),
        Key::Meta => "meta".to_string(),
        Key::F(n) => format!("f{}", n),
    }
}

/// Open an application by name using platform-appropriate method.
fn open_application(name: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", name])
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to open app: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", name])
            .status()
            .map_err(|e| anyhow::anyhow!("Failed to open app: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(name)
            .status()
            .or_else(|_| {
                std::process::Command::new(name).status()
            })
            .map_err(|e| anyhow::anyhow!("Failed to open app: {}", e))?;
    }

    Ok(())
}

/// Run a shell command on the platform.
fn run_shell_command(command: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("cmd")
            .args(["/C", command])
            .output()
            .map_err(|e| anyhow::anyhow!("Command failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("Command exited with error: {}", stderr);
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("sh")
            .args(["-c", command])
            .output()
            .map_err(|e| anyhow::anyhow!("Command failed: {}", e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("Command exited with error: {}", stderr);
        }
    }

    Ok(())
}
