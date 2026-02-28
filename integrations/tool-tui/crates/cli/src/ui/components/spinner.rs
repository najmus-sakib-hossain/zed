//! Spinner component
//!
//! Displays an animated spinner with a message.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::spinner::Spinner;
//!
//! let spinner = Spinner::new("Loading...");
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::time::Instant;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// An animated loading spinner with a message.
///
/// Spinners indicate ongoing background work. The animation
/// cycles through Braille dot patterns for a smooth effect.
///
/// # Features
///
/// - Smooth Braille-based animation
/// - Configurable animation speed
/// - Custom message text
/// - Cyan color scheme
///
/// # Example
///
/// ```rust,ignore
/// use dx_cli::ui::components::spinner::Spinner;
///
/// let spinner = Spinner::new("Loading data...")
///     .frame_duration(100);  // Slower animation
///
/// // Render in your draw loop
/// spinner.render(frame, area);
/// ```
pub struct Spinner {
    /// Message displayed next to the spinner
    message: String,
    /// When the spinner started (for animation timing)
    start_time: Instant,
    /// Milliseconds between animation frames
    frame_duration_ms: u64,
}

impl Spinner {
    /// Create a new spinner with message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            start_time: Instant::now(),
            frame_duration_ms: 80,
        }
    }

    /// Set frame duration in milliseconds
    pub fn frame_duration(mut self, ms: u64) -> Self {
        self.frame_duration_ms = ms;
        self
    }

    fn get_current_frame(&self) -> &str {
        let elapsed = self.start_time.elapsed().as_millis() as u64;
        let frame_index = (elapsed / self.frame_duration_ms) as usize % SPINNER_FRAMES.len();
        SPINNER_FRAMES[frame_index]
    }

    /// Render the spinner
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let frame = self.get_current_frame();
        let line = Line::from(vec![
            Span::styled(frame, Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(&self.message, Style::default().fg(Color::White)),
        ]);

        let paragraph = Paragraph::new(vec![line]);
        f.render_widget(paragraph, area);
    }
}
