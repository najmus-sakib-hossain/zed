//! Transparent overlay window for displaying grammar suggestions
//! and edit predictions over any application.
//!
//! Each platform uses a different approach for rendering a transparent
//! always-on-top window:
//! - macOS: NSWindow with transparent background
//! - Windows: WS_EX_LAYERED + WS_EX_TOPMOST
//! - Linux X11: override-redirect window
//! - Linux Wayland: layer-shell protocol

use anyhow::Result;

/// Configuration for the suggestion overlay window.
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Maximum width of the overlay in pixels.
    pub max_width: u32,
    /// Maximum number of suggestion lines shown.
    pub max_lines: u32,
    /// Opacity (0.0 = fully transparent, 1.0 = fully opaque).
    pub opacity: f32,
    /// Font size for suggestion text.
    pub font_size: f32,
    /// Background color (RGBA).
    pub background_color: [u8; 4],
    /// Whether to show squiggly underlines for grammar errors.
    pub show_squiggles: bool,
    /// Auto-hide delay in milliseconds after showing a suggestion.
    pub auto_hide_ms: u64,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            max_width: 600,
            max_lines: 4,
            opacity: 0.95,
            font_size: 14.0,
            background_color: [30, 30, 40, 240],
            show_squiggles: true,
            auto_hide_ms: 5000,
        }
    }
}

/// A suggestion to show in the overlay.
#[derive(Debug, Clone)]
pub struct OverlaySuggestion {
    /// The original text with the error.
    pub original: String,
    /// The suggested replacement.
    pub replacement: String,
    /// A short explanation of the suggestion.
    pub explanation: String,
    /// Severity level (maps to squiggly color).
    pub severity: SuggestionSeverity,
    /// Position in screen coordinates where to show the overlay.
    pub screen_x: i32,
    pub screen_y: i32,
}

/// Severity of a grammar/style suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionSeverity {
    /// 🔴 Definitive error (misspelling, broken grammar).
    Error,
    /// 🟡 Suggestion (wordiness, passive voice).
    Warning,
    /// 🔵 Style (stronger synonym, conciseness).
    Style,
    /// 💜 AI insight (restructuring, tone adjustment).
    AiInsight,
}

/// The transparent overlay window used to show suggestions over any app.
pub struct OverlayWindow {
    config: OverlayConfig,
    visible: bool,
    current_suggestions: Vec<OverlaySuggestion>,
}

impl OverlayWindow {
    /// Create a new overlay window with the given configuration.
    pub fn new(config: OverlayConfig) -> Result<Self> {
        log::info!("Creating suggestion overlay window");
        Ok(Self {
            config,
            visible: false,
            current_suggestions: Vec::new(),
        })
    }

    /// Show suggestions at the specified screen position.
    pub fn show_suggestions(&mut self, suggestions: Vec<OverlaySuggestion>) -> Result<()> {
        if suggestions.is_empty() {
            self.hide()?;
            return Ok(());
        }

        self.current_suggestions = suggestions;
        self.visible = true;

        #[cfg(target_os = "macos")]
        self.show_macos()?;

        #[cfg(target_os = "windows")]
        self.show_windows()?;

        #[cfg(target_os = "linux")]
        self.show_linux()?;

        Ok(())
    }

    /// Hide the overlay.
    pub fn hide(&mut self) -> Result<()> {
        self.visible = false;
        self.current_suggestions.clear();
        log::debug!("Overlay hidden");
        Ok(())
    }

    /// Check if overlay is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update overlay configuration.
    pub fn set_config(&mut self, config: OverlayConfig) {
        self.config = config;
    }

    pub fn config(&self) -> &OverlayConfig {
        &self.config
    }

    #[cfg(target_os = "macos")]
    fn show_macos(&self) -> Result<()> {
        // NSWindow with:
        // - styleMask: .borderless
        // - backgroundColor: NSColor.clear
        // - isOpaque: false
        // - level: .floating
        // - ignoresMouseEvents: true (for click-through)
        // Render via GPUI within the NSWindow
        log::debug!("macOS: Showing overlay with {} suggestions", self.current_suggestions.len());
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn show_windows(&self) -> Result<()> {
        // CreateWindowExW with:
        // - WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT
        // - UpdateLayeredWindow for per-pixel alpha
        // - SetWindowPos for positioning
        // Render via GPUI/DirectX within the layered window
        log::debug!("Windows: Showing overlay with {} suggestions", self.current_suggestions.len());
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn show_linux(&self) -> Result<()> {
        // Detect X11 vs Wayland:
        // X11: override-redirect window with _NET_WM_WINDOW_TYPE_DOCK
        // Wayland: zwlr_layer_shell_v1 with layer = overlay
        // Render via GPUI/Vulkan within the window
        log::debug!("Linux: Showing overlay with {} suggestions", self.current_suggestions.len());
        Ok(())
    }
}
