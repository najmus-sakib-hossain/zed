//! Cross-platform GUI automation — mouse, keyboard, template matching.
//!
//! Provides a unified interface inspired by `rustautogui` and `autopilot-rs`
//! for controlling the mouse and keyboard across platforms.

use anyhow::Result;

/// Mouse button types.
#[derive(Debug, Clone, Copy)]
pub enum AutoMouseButton {
    Left,
    Right,
    Middle,
}

/// A screen coordinate.
#[derive(Debug, Clone, Copy)]
pub struct ScreenPoint {
    pub x: i32,
    pub y: i32,
}

/// Automation controller for mouse and keyboard operations.
pub struct AutomationController {
    /// Current mouse position.
    current_pos: ScreenPoint,
    /// Whether to add small delays between actions (human-like).
    humanize: bool,
    /// Delay between actions in milliseconds.
    action_delay_ms: u64,
    /// Whether to use real platform input (true) or just log (false/dry-run).
    live: bool,
}

impl AutomationController {
    pub fn new() -> Self {
        Self {
            current_pos: ScreenPoint { x: 0, y: 0 },
            humanize: true,
            action_delay_ms: 50,
            live: true,
        }
    }

    /// Create a dry-run controller that only logs actions.
    pub fn dry_run() -> Self {
        Self {
            current_pos: ScreenPoint { x: 0, y: 0 },
            humanize: false,
            action_delay_ms: 0,
            live: false,
        }
    }

    fn maybe_delay(&self) {
        if self.humanize && self.action_delay_ms > 0 {
            std::thread::sleep(std::time::Duration::from_millis(self.action_delay_ms));
        }
    }

    /// Move the mouse to screen coordinates.
    pub fn mouse_move(&mut self, x: i32, y: i32) -> Result<()> {
        self.current_pos = ScreenPoint { x, y };
        if self.live {
            crate::input::mouse_move(x, y)?;
        } else {
            log::debug!("(dry) Mouse move to ({}, {})", x, y);
        }
        self.maybe_delay();
        Ok(())
    }

    /// Click at the current position.
    pub fn mouse_click(&self, button: AutoMouseButton) -> Result<()> {
        if self.live {
            let btn = match button {
                AutoMouseButton::Left => crate::input::MouseBtn::Left,
                AutoMouseButton::Right => crate::input::MouseBtn::Right,
                AutoMouseButton::Middle => crate::input::MouseBtn::Middle,
            };
            crate::input::mouse_click(btn, 1)?;
        } else {
            log::debug!(
                "(dry) Mouse click {:?} at ({}, {})",
                button, self.current_pos.x, self.current_pos.y,
            );
        }
        Ok(())
    }

    /// Double-click at the current position.
    pub fn mouse_double_click(&self) -> Result<()> {
        if self.live {
            crate::input::mouse_click(crate::input::MouseBtn::Left, 2)?;
        } else {
            log::debug!(
                "(dry) Double-click at ({}, {})",
                self.current_pos.x, self.current_pos.y,
            );
        }
        Ok(())
    }

    /// Drag from current position to target.
    pub fn mouse_drag(&mut self, to_x: i32, to_y: i32) -> Result<()> {
        let from = self.current_pos;
        if self.live {
            crate::input::mouse_drag(to_x, to_y)?;
        } else {
            log::debug!(
                "(dry) Drag ({}, {}) -> ({}, {})",
                from.x, from.y, to_x, to_y,
            );
        }
        self.current_pos = ScreenPoint { x: to_x, y: to_y };
        self.maybe_delay();
        Ok(())
    }

    /// Scroll the mouse wheel.
    pub fn scroll(&self, delta_x: i32, delta_y: i32) -> Result<()> {
        if self.live {
            crate::input::scroll(delta_x, delta_y)?;
        } else {
            log::debug!("(dry) Scroll ({}, {})", delta_x, delta_y);
        }
        Ok(())
    }

    /// Type text string (simulates keystrokes).
    pub fn type_text(&self, text: &str) -> Result<()> {
        if self.live {
            crate::input::type_text(text)?;
        } else {
            log::debug!("(dry) Typing: {:?}", &text[..text.len().min(50)]);
        }
        Ok(())
    }

    /// Press a key combination (e.g., ["ctrl", "c"]).
    pub fn key_press(&self, keys: &[&str]) -> Result<()> {
        if self.live {
            crate::input::key_press(keys)?;
        } else {
            log::debug!("(dry) Key press: {:?}", keys);
        }
        Ok(())
    }

    /// Find an image template on screen and return its center coordinates.
    pub fn find_on_screen(&self, _template_png: &[u8], _confidence: f64) -> Result<Option<ScreenPoint>> {
        log::debug!("Template matching not yet implemented");
        Ok(None)
    }

    /// Get current screen size (real platform query).
    pub fn screen_size(&self) -> Result<(u32, u32)> {
        crate::input::screen_size()
    }

    /// Get current mouse position (real platform query when live).
    pub fn current_position(&self) -> ScreenPoint {
        if self.live {
            if let Ok((x, y)) = crate::input::cursor_position() {
                return ScreenPoint { x, y };
            }
        }
        self.current_pos
    }

    /// Set whether to add human-like delays.
    pub fn set_humanize(&mut self, humanize: bool) {
        self.humanize = humanize;
    }

    /// Set delay between actions.
    pub fn set_action_delay(&mut self, delay_ms: u64) {
        self.action_delay_ms = delay_ms;
    }

    /// Toggle live mode (true = real input, false = dry-run).
    pub fn set_live(&mut self, live: bool) {
        self.live = live;
    }
}

impl Default for AutomationController {
    fn default() -> Self {
        Self::new()
    }
}
