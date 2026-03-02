//! OS-level input interception for system-wide grammar checking (Part 18).
//!
//! Intercepts text input across the entire OS — not just inside Zed — to provide
//! real-time grammar and spelling corrections in any application.
//!
//! Platform support:
//! - **macOS**: Accessibility API (AXUIElement) + CGEvent tap
//! - **Linux**: ibus / fcitx input method framework + AT-SPI
//! - **Windows**: Text Services Framework (TSF) + UI Automation

use anyhow::Result;
use std::sync::Arc;

/// Input interception event — text that was typed or committed in any OS application.
#[derive(Debug, Clone)]
pub struct InputEvent {
    /// The text that was typed or committed.
    pub text: String,
    /// The application that received the input.
    pub app_name: String,
    /// The application's bundle identifier or executable path.
    pub app_id: String,
    /// Window title at the time of input.
    pub window_title: String,
    /// Whether this is a commit (IME finalized) or incremental keystroke.
    pub is_commit: bool,
    /// Timestamp in milliseconds since epoch.
    pub timestamp_ms: u64,
}

/// Correction suggestion from the grammar pipeline.
#[derive(Debug, Clone)]
pub struct CorrectionOffer {
    /// Original text span to replace.
    pub original: String,
    /// Suggested replacement.
    pub replacement: String,
    /// Explanation of the correction.
    pub reason: String,
    /// Confidence 0.0 – 1.0.
    pub confidence: f32,
    /// Category: spelling, grammar, style, punctuation.
    pub category: CorrectionCategory,
}

/// Types of correction offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorrectionCategory {
    Spelling,
    Grammar,
    Punctuation,
    Style,
    Clarity,
}

/// Callback for delivering corrections to the user.
pub type CorrectionCallback = Arc<dyn Fn(Vec<CorrectionOffer>) + Send + Sync>;

/// Platform-specific input interception backend.
pub trait InputInterceptor: Send + Sync {
    /// Start intercepting input events system-wide.
    fn start(&mut self) -> Result<()>;

    /// Stop intercepting.
    fn stop(&mut self) -> Result<()>;

    /// Whether interception is currently active.
    fn is_active(&self) -> bool;

    /// Register a callback for incoming text events.
    fn on_input(&mut self, callback: Arc<dyn Fn(InputEvent) + Send + Sync>);
}

// ---------------------------------------------------------------------------
// Interception manager — orchestrates platform backend + grammar pipeline
// ---------------------------------------------------------------------------

/// Manages OS-level input interception and routes text through the grammar pipeline.
pub struct InputInterceptionManager {
    interceptor: Option<Box<dyn InputInterceptor>>,
    correction_callback: Option<CorrectionCallback>,
    /// Minimum text length before triggering grammar check.
    min_buffer_chars: usize,
    /// Buffer of accumulated input for batch checking.
    buffer: String,
    /// Whether the manager is enabled.
    enabled: bool,
    /// Apps where interception is disabled (privacy).
    excluded_apps: Vec<String>,
}

impl InputInterceptionManager {
    pub fn new() -> Self {
        Self {
            interceptor: None,
            correction_callback: None,
            min_buffer_chars: 20,
            buffer: String::new(),
            enabled: false,
            excluded_apps: vec![
                // Default exclusions for sensitive apps
                "1Password".into(),
                "Keychain Access".into(),
                "Terminal".into(),
                "iTerm2".into(),
                "Alacritty".into(),
                "KeePassXC".into(),
            ],
        }
    }

    /// Set the platform-specific interceptor backend.
    pub fn set_interceptor(&mut self, interceptor: Box<dyn InputInterceptor>) {
        self.interceptor = Some(interceptor);
    }

    /// Register a callback for grammar corrections.
    pub fn on_correction(&mut self, callback: CorrectionCallback) {
        self.correction_callback = Some(callback);
    }

    /// Add an app to the exclusion list (no interception in these apps).
    pub fn exclude_app(&mut self, app_name: String) {
        if !self.excluded_apps.contains(&app_name) {
            self.excluded_apps.push(app_name);
        }
    }

    /// Start system-wide interception.
    pub fn start(&mut self) -> Result<()> {
        if let Some(interceptor) = &mut self.interceptor {
            interceptor.start()?;
            self.enabled = true;
            log::info!("Input interception started");
        } else {
            return Err(anyhow::anyhow!("No platform interceptor configured"));
        }
        Ok(())
    }

    /// Stop system-wide interception.
    pub fn stop(&mut self) -> Result<()> {
        if let Some(interceptor) = &mut self.interceptor {
            interceptor.stop()?;
            self.enabled = false;
            self.buffer.clear();
            log::info!("Input interception stopped");
        }
        Ok(())
    }

    /// Process an incoming input event from the OS.
    pub fn handle_input(&mut self, event: InputEvent) {
        // Skip excluded apps
        if self.excluded_apps.iter().any(|a| event.app_name.contains(a)) {
            return;
        }

        self.buffer.push_str(&event.text);

        // Check if we have enough text buffered
        if self.buffer.len() >= self.min_buffer_chars {
            self.flush_buffer();
        }
    }

    /// Force-check buffered text through the grammar pipeline.
    pub fn flush_buffer(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let text = std::mem::take(&mut self.buffer);
        log::debug!("Flushing {} chars to grammar pipeline", text.len());

        // The grammar pipeline integration happens here.
        // In the real implementation, this sends `text` through
        // dx_grammar::GrammarPipeline::check() and delivers results
        // via the correction_callback.
        if let Some(callback) = &self.correction_callback {
            // Placeholder — actual corrections come from the grammar pipeline.
            let corrections = Vec::new();
            if !corrections.is_empty() {
                callback(corrections);
            }
        }
    }

    pub fn is_enabled(&self) -> bool { self.enabled }

    pub fn set_min_buffer_chars(&mut self, chars: usize) {
        self.min_buffer_chars = chars.max(5);
    }
}

impl Default for InputInterceptionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// macOS interceptor stub
// ---------------------------------------------------------------------------

/// macOS input interceptor using Accessibility API + CGEvent tap.
#[cfg(target_os = "macos")]
pub struct MacOsInterceptor {
    active: bool,
    callback: Option<Arc<dyn Fn(InputEvent) + Send + Sync>>,
}

#[cfg(target_os = "macos")]
impl MacOsInterceptor {
    pub fn new() -> Self {
        Self { active: false, callback: None }
    }
}

#[cfg(target_os = "macos")]
impl InputInterceptor for MacOsInterceptor {
    fn start(&mut self) -> Result<()> {
        // Request accessibility permissions, set up CGEvent tap
        self.active = true;
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }
    fn is_active(&self) -> bool { self.active }
    fn on_input(&mut self, callback: Arc<dyn Fn(InputEvent) + Send + Sync>) {
        self.callback = Some(callback);
    }
}

// ---------------------------------------------------------------------------
// Windows interceptor stub
// ---------------------------------------------------------------------------

/// Windows input interceptor using Text Services Framework + UI Automation.
#[cfg(target_os = "windows")]
pub struct WindowsInterceptor {
    active: bool,
    callback: Option<Arc<dyn Fn(InputEvent) + Send + Sync>>,
}

#[cfg(target_os = "windows")]
impl WindowsInterceptor {
    pub fn new() -> Self {
        Self { active: false, callback: None }
    }
}

#[cfg(target_os = "windows")]
impl InputInterceptor for WindowsInterceptor {
    fn start(&mut self) -> Result<()> {
        // Set up TSF / UI Automation text changed event handlers
        self.active = true;
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }
    fn is_active(&self) -> bool { self.active }
    fn on_input(&mut self, callback: Arc<dyn Fn(InputEvent) + Send + Sync>) {
        self.callback = Some(callback);
    }
}

// ---------------------------------------------------------------------------
// Linux interceptor stub
// ---------------------------------------------------------------------------

/// Linux input interceptor using ibus/fcitx + AT-SPI.
#[cfg(target_os = "linux")]
pub struct LinuxInterceptor {
    active: bool,
    callback: Option<Arc<dyn Fn(InputEvent) + Send + Sync>>,
}

#[cfg(target_os = "linux")]
impl LinuxInterceptor {
    pub fn new() -> Self {
        Self { active: false, callback: None }
    }
}

#[cfg(target_os = "linux")]
impl InputInterceptor for LinuxInterceptor {
    fn start(&mut self) -> Result<()> {
        // Connect to ibus or fcitx input method framework
        self.active = true;
        Ok(())
    }
    fn stop(&mut self) -> Result<()> {
        self.active = false;
        Ok(())
    }
    fn is_active(&self) -> bool { self.active }
    fn on_input(&mut self, callback: Arc<dyn Fn(InputEvent) + Send + Sync>) {
        self.callback = Some(callback);
    }
}

/// Create the platform-appropriate interceptor.
pub fn create_platform_interceptor() -> Option<Box<dyn InputInterceptor>> {
    #[cfg(target_os = "macos")]
    { Some(Box::new(MacOsInterceptor::new())) }
    #[cfg(target_os = "windows")]
    { Some(Box::new(WindowsInterceptor::new())) }
    #[cfg(target_os = "linux")]
    { Some(Box::new(LinuxInterceptor::new())) }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    { None }
}
