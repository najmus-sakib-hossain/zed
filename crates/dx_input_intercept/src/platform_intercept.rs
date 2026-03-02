//! Platform-specific input interception.
//!
//! Each platform uses its native input method framework to intercept
//! keystrokes system-wide and provide edit prediction + grammar checking.

use anyhow::Result;
use std::sync::Arc;

/// State of the input interceptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterceptState {
    /// Not yet initialized.
    Uninitialized,
    /// Running and intercepting input.
    Active,
    /// Temporarily paused (e.g., user disabled, or app is in excluded list).
    Paused,
    /// Failed to hook — likely a permissions issue.
    PermissionDenied,
    /// Stopped and cleaned up.
    Stopped,
}

/// An intercepted input event.
#[derive(Debug, Clone)]
pub enum InterceptEvent {
    /// A character was typed.
    CharTyped {
        character: char,
        /// The process name of the foreground application.
        app_process: String,
        /// The window title of the foreground application.
        window_title: String,
    },
    /// A key was pressed (including modifiers).
    KeyDown {
        keycode: u32,
        modifiers: Modifiers,
        app_process: String,
    },
    /// A key was released.
    KeyUp {
        keycode: u32,
        modifiers: Modifiers,
    },
    /// The focused text field changed.
    FocusChanged {
        app_process: String,
        window_title: String,
    },
    /// The application switched.
    AppSwitched {
        new_app_process: String,
        new_window_title: String,
    },
}

/// Modifier key state.
#[derive(Debug, Clone, Copy, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub meta: bool,
}

/// The main input interceptor that dispatches to platform-specific backends.
pub struct InputInterceptor {
    state: InterceptState,
    /// Callback for intercepted events.
    event_handler: Option<Arc<dyn Fn(InterceptEvent) + Send + Sync>>,
    /// Apps to exclude from interception.
    excluded_apps: Vec<String>,
}

impl InputInterceptor {
    /// Create a new interceptor. Attempts to register platform hooks.
    pub fn new() -> Result<Self> {
        let mut interceptor = Self {
            state: InterceptState::Uninitialized,
            event_handler: None,
            excluded_apps: vec![
                // Common terminals — grammar checking is not useful here
                "Terminal".to_string(),
                "iTerm2".to_string(),
                "Alacritty".to_string(),
                "WindowsTerminal".to_string(),
                "cmd.exe".to_string(),
                "powershell.exe".to_string(),
            ],
        };

        interceptor.register_hooks()?;
        Ok(interceptor)
    }

    /// Set the callback for intercepted events.
    pub fn on_event(&mut self, handler: impl Fn(InterceptEvent) + Send + Sync + 'static) {
        self.event_handler = Some(Arc::new(handler));
    }

    /// Add an app to the exclusion list.
    pub fn exclude_app(&mut self, process_name: &str) {
        self.excluded_apps.push(process_name.to_string());
    }

    /// Check if an app is excluded.
    pub fn is_excluded(&self, process_name: &str) -> bool {
        self.excluded_apps
            .iter()
            .any(|excluded| process_name.contains(excluded))
    }

    /// Current state of the interceptor.
    pub fn state(&self) -> InterceptState {
        self.state
    }

    /// Pause interception (disable hooks without removing them).
    pub fn pause(&mut self) {
        if self.state == InterceptState::Active {
            self.state = InterceptState::Paused;
            log::info!("Input interception paused");
        }
    }

    /// Resume interception.
    pub fn resume(&mut self) {
        if self.state == InterceptState::Paused {
            self.state = InterceptState::Active;
            log::info!("Input interception resumed");
        }
    }

    /// Stop and clean up all hooks.
    pub fn stop(&mut self) {
        self.unregister_hooks();
        self.state = InterceptState::Stopped;
        log::info!("Input interception stopped");
    }

    /// Register platform-specific input hooks.
    fn register_hooks(&mut self) -> Result<()> {
        #[cfg(target_os = "macos")]
        {
            self.register_macos_hooks()?;
        }

        #[cfg(target_os = "windows")]
        {
            self.register_windows_hooks()?;
        }

        #[cfg(target_os = "linux")]
        {
            self.register_linux_hooks()?;
        }

        self.state = InterceptState::Active;
        log::info!("Platform input hooks registered successfully");
        Ok(())
    }

    /// Unregister platform hooks.
    fn unregister_hooks(&mut self) {
        // Platform-specific cleanup
        log::info!("Platform input hooks unregistered");
    }

    // ── macOS: CGEventTap + IMK ──────────────────────────────────────────

    #[cfg(target_os = "macos")]
    fn register_macos_hooks(&mut self) -> Result<()> {
        // CGEventTap for system-wide keystroke monitoring.
        // Requires Accessibility permission (System Preferences → Privacy → Accessibility).
        //
        // Implementation plan:
        // 1. CGEventTapCreate with kCGHeadInsertEventTap for key events
        // 2. CFMachPortCreateRunLoopSource to integrate with run loop
        // 3. IMKInputController for proper input method integration
        // 4. AXUIElement for reading focused text field content
        //
        // For now, this is a placeholder — real implementation requires
        // CoreGraphics and Accessibility framework bindings.
        log::info!("macOS: CGEventTap + IMK hooks registered (placeholder)");
        Ok(())
    }

    // ── Windows: TSF + low-level hooks ───────────────────────────────────

    #[cfg(target_os = "windows")]
    fn register_windows_hooks(&mut self) -> Result<()> {
        // Text Services Framework (TSF) for system-wide text input.
        // UI Automation API for reading text field content.
        // Low-level keyboard hook (WH_KEYBOARD_LL) as fallback.
        //
        // Implementation plan:
        // 1. SetWindowsHookEx(WH_KEYBOARD_LL, ...) for keystroke capture
        // 2. ITfThreadMgr for TSF integration
        // 3. IUIAutomation for text field enumeration
        // 4. WS_EX_LAYERED window for overlay suggestions
        //
        // Placeholder — real implementation requires windows-rs bindings.
        log::info!("Windows: TSF + low-level hooks registered (placeholder)");
        Ok(())
    }

    // ── Linux: IBus/Fcitx5 ───────────────────────────────────────────────

    #[cfg(target_os = "linux")]
    fn register_linux_hooks(&mut self) -> Result<()> {
        // X11: IBus engine + XInput2 for keystroke capture.
        // Wayland: Fcitx5 + input-method-v2 protocol.
        // AT-SPI2 for accessibility tree traversal.
        //
        // Implementation plan:
        // 1. Detect X11 vs Wayland (WAYLAND_DISPLAY env var)
        // 2. X11: Register IBus engine component via D-Bus
        // 3. Wayland: Connect to zwp_input_method_v2 protocol
        // 4. AT-SPI2 via atspi D-Bus interface for text field reading
        //
        // Placeholder — real implementation requires dbus-rs and protocol bindings.
        log::info!("Linux: IBus/Fcitx5 hooks registered (placeholder)");
        Ok(())
    }
}

impl Drop for InputInterceptor {
    fn drop(&mut self) {
        if self.state == InterceptState::Active || self.state == InterceptState::Paused {
            self.stop();
        }
    }
}
