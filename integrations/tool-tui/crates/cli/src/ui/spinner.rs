//! Spinner and loading indicators
//!
//! Provides animated spinners for async operations with Vercel-style output.

use crate::ui::theme::icons;
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use std::time::Duration;

/// A modern spinner for async operations
///
/// Requirement 3.1: Display animated sequence at 80ms intervals
pub struct Spinner {
    pb: ProgressBar,
    message: String,
}

impl Spinner {
    /// Create a new spinner with a message
    ///
    /// Uses the braille dot animation sequence (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏)
    /// with 80ms tick interval as per requirements.
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        let pb = ProgressBar::new_spinner();

        // Requirement 3.1: Animated sequence at 80ms intervals
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&[
                    "   ⠋", "   ⠙", "   ⠹", "   ⠸", "   ⠼", "   ⠴", "   ⠦", "   ⠧", "   ⠇", "   ⠏",
                ])
                .template("  {spinner:.cyan} {msg}")
                .unwrap(),
        );

        pb.set_message(message.clone());
        pb.enable_steady_tick(Duration::from_millis(80));

        Self { pb, message }
    }

    /// Create a spinner with circle style
    #[allow(dead_code)]
    pub fn circle(message: impl Into<String>) -> Self {
        let message = message.into();
        let pb = ProgressBar::new_spinner();

        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["   ◐", "   ◓", "   ◑", "   ◒"])
                .template("  {spinner:.cyan} {msg}")
                .unwrap(),
        );

        pb.set_message(message.clone());
        pb.enable_steady_tick(Duration::from_millis(80));

        Self { pb, message }
    }

    /// Create a spinner with dots style (alias for new)
    pub fn dots(message: impl Into<String>) -> Self {
        Self::new(message)
    }

    /// Update the spinner message
    ///
    /// Requirement 3.4: Allow updating message during operation
    pub fn set_message(&self, message: impl Into<String>) {
        self.pb.set_message(message.into());
    }

    /// Get the current message
    #[allow(dead_code)]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Mark spinner as successful with green checkmark
    ///
    /// Requirement 3.2: Clear and display success message with ✓
    pub fn finish_success(self, message: impl Into<String>) {
        self.pb.finish_and_clear();
        eprintln!("  {} {}", icons::SUCCESS.green().bold(), message.into().white());
    }

    /// Mark spinner as successful (alias)
    pub fn success(self, message: impl Into<String>) {
        self.finish_success(message);
    }

    /// Mark spinner as failed with red X
    ///
    /// Requirement 3.3: Clear and display error message with ✗
    pub fn finish_error(self, message: impl Into<String>) {
        self.pb.finish_and_clear();
        eprintln!("  {} {}", icons::ERROR.red().bold(), message.into().red());
    }

    /// Mark spinner as failed (alias)
    pub fn error(self, message: impl Into<String>) {
        self.finish_error(message);
    }

    /// Mark spinner as warning with yellow warning symbol
    ///
    /// Requirement 3.5: Display yellow warning symbol
    pub fn finish_warn(self, message: impl Into<String>) {
        self.pb.finish_and_clear();
        eprintln!("  {} {}", icons::WARNING.yellow().bold(), message.into().yellow());
    }

    /// Mark spinner as warning (alias)
    pub fn warn(self, message: impl Into<String>) {
        self.finish_warn(message);
    }

    /// Finish the spinner without a status
    pub fn finish(self) {
        self.pb.finish_and_clear();
    }

    /// Stop the spinner without clearing (for intermediate output)
    #[allow(dead_code)]
    pub fn suspend<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.pb.suspend(f)
    }

    // === Methods that return formatted strings (for testing) ===

    /// Format a success message and return as string
    pub fn format_success(message: &str) -> String {
        format!("  {} {}", icons::SUCCESS.green().bold(), message.white())
    }

    /// Format an error message and return as string
    pub fn format_error(message: &str) -> String {
        format!("  {} {}", icons::ERROR.red().bold(), message.red())
    }

    /// Format a warning message and return as string
    pub fn format_warn(message: &str) -> String {
        format!("  {} {}", icons::WARNING.yellow().bold(), message.yellow())
    }

    /// Format a success message without colors
    pub fn format_success_plain(message: &str) -> String {
        format!("  {} {}", icons::SUCCESS, message)
    }

    /// Format an error message without colors
    pub fn format_error_plain(message: &str) -> String {
        format!("  {} {}", icons::ERROR, message)
    }

    /// Format a warning message without colors
    pub fn format_warn_plain(message: &str) -> String {
        format!("  {} {}", icons::WARNING, message)
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
    }
}

/// Multi-spinner for parallel operations
#[allow(dead_code)]
pub struct MultiSpinner {
    spinners: Vec<Spinner>,
}

#[allow(dead_code)]
impl MultiSpinner {
    pub fn new() -> Self {
        Self {
            spinners: Vec::new(),
        }
    }

    pub fn add(&mut self, message: impl Into<String>) -> usize {
        let spinner = Spinner::new(message);
        self.spinners.push(spinner);
        self.spinners.len() - 1
    }

    pub fn finish_one(&mut self, index: usize, message: &str) {
        if let Some(spinner) = self.spinners.get(index) {
            spinner.set_message(message);
        }
    }
}

impl Default for MultiSpinner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_spinner_creation() {
        // Just verify it doesn't panic
        let _spinner = Spinner::new("Loading...");
    }

    #[test]
    fn test_spinner_message_update() {
        let spinner = Spinner::new("Initial message");
        spinner.set_message("Updated message");
        spinner.finish();
    }

    // Property tests for spinner output formatting
    // These test the format methods which mirror the actual output

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_spinner_success_has_checkmark(message in "[a-zA-Z0-9 ]{1,50}") {
            let output = Spinner::format_success_plain(&message);
            prop_assert!(output.contains("✓"), "Success should contain ✓");
            prop_assert!(output.contains(&message), "Success should contain message");
        }

        #[test]
        fn prop_spinner_error_has_x(message in "[a-zA-Z0-9 ]{1,50}") {
            let output = Spinner::format_error_plain(&message);
            prop_assert!(output.contains("✗"), "Error should contain ✗");
            prop_assert!(output.contains(&message), "Error should contain message");
        }

        #[test]
        fn prop_spinner_warn_has_warning(message in "[a-zA-Z0-9 ]{1,50}") {
            let output = Spinner::format_warn_plain(&message);
            prop_assert!(output.contains("⚠"), "Warning should contain ⚠");
            prop_assert!(output.contains(&message), "Warning should contain message");
        }
    }
}
