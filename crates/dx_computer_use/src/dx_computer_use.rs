//! dx_computer_use — OS automation with AI-driven computer use.
//!
//! Provides mouse/keyboard control, screenshot capture, accessibility tree
//! reading, and safety boundaries for AI-controlled system interaction.
//!
//! Integrates:
//! - `rustautogui` / `autopilot-rs` — cross-platform mouse/keyboard/template matching
//! - `screenshots` — cross-platform screen capture
//! - `accesskit` — cross-platform accessibility toolkit
//! - Vision models (local LLaVA or cloud GPT-4V/Claude Vision) for UI understanding

pub mod accessibility;
pub mod accessibility_toolkit;
pub mod actions;
pub mod automation;
pub mod capture;
pub mod input;
pub mod platform_accessibility;
pub mod safety;
pub mod screen_capture;
pub mod screenshot;
pub mod vision;

pub use accessibility::AccessibilityTree;
pub use accessibility_toolkit::{AccessibilityToolkit, AccessibleElement, AccessibleRole};
pub use actions::{ComputerAction, ComputerUseAgent};
pub use automation::{AutomationController, ScreenPoint};
pub use capture::{capture_full_screen, capture_region, capture_window, png_to_base64, png_dimensions};
pub use input::{mouse_move, mouse_click, mouse_drag, scroll, type_text, key_press, cursor_position, screen_size, MouseBtn};
pub use platform_accessibility::{read_focused_window, PlatformNode, NodeBounds};
pub use safety::{SafetyBoundary, SafetyConfig};
pub use screen_capture::{CapturedScreen, ScreenCaptureManager};
pub use screenshot::ScreenCapture;
pub use vision::VisionAnalyzer;
