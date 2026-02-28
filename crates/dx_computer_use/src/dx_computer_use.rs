//! dx_computer_use — OS automation with AI-driven computer use.
//!
//! Provides mouse/keyboard control, screenshot capture, accessibility tree
//! reading, and safety boundaries for AI-controlled system interaction.

pub mod accessibility;
pub mod actions;
pub mod safety;
pub mod screenshot;
pub mod vision;

pub use accessibility::AccessibilityTree;
pub use actions::{ComputerAction, ComputerUseAgent};
pub use safety::{SafetyBoundary, SafetyConfig};
pub use screenshot::ScreenCapture;
pub use vision::VisionAnalyzer;
