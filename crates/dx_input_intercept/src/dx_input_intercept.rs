//! dx_input_intercept — System-wide OS input interception.
//!
//! Extends DX's edit prediction and grammar checking to EVERY application on the OS,
//! not just the Zed editor. Uses platform-specific input method frameworks:
//!
//! - **macOS:** CGEventTap + Input Method Kit (IMK) + AXUIElement
//! - **Windows:** Text Services Framework (TSF) + UI Automation API + low-level hooks
//! - **Linux X11:** IBus + XInput2 + AT-SPI2
//! - **Linux Wayland:** Fcitx5 + input-method-v2 + AT-SPI2 + layer shell

pub mod clipboard;
pub mod hotkey;
pub mod overlay;
pub mod platform_intercept;
pub mod text_field_access;

pub use clipboard::ClipboardManager;
pub use hotkey::{HotkeyBinding, HotkeyManager};
pub use overlay::{OverlayConfig, OverlayWindow};
pub use platform_intercept::{InputInterceptor, InterceptEvent, InterceptState};
pub use text_field_access::{TextFieldInfo, TextFieldReader};

/// Initialize the system-wide input interception subsystem.
///
/// # Safety
///
/// This function registers OS-level input hooks and requires appropriate
/// permissions (Accessibility on macOS, etc.).
pub fn init() -> anyhow::Result<InputInterceptor> {
    log::info!("DX Input Intercept: initializing platform hooks");
    InputInterceptor::new()
}
