//! Progress bars for long-running operations
//!
//! Provides styled progress bars with Vercel-inspired design.

use crate::ui::theme::icons;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;

/// A styled progress bar
///
/// Requirement 4.1: Display completion as visual bar with percentage
pub struct Progress {
    pb: ProgressBar,
}

impl Progress {
    /// Create a new progress bar with a total count
    ///
    /// Requirement 4.1: Display completion as visual bar with percentage
    pub fn new(total: u64, message: impl Into<String>) -> Self {
        let pb = ProgressBar::new(total);

        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {msg}\n  [{bar:40.cyan/bright_black}] {pos}/{len} {percent}%")
                .unwrap()
                .progress_chars("━━─"),
        );

        pb.set_message(message.into());

        Self { pb }
    }

    /// Create a byte-based progress bar (for downloads)
    ///
    /// Requirement 4.2: Show bytes downloaded, total bytes, and transfer speed
    pub fn download(total: u64, filename: impl Into<String>) -> Self {
        let pb = ProgressBar::new(total);

        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "  Downloading {msg}\n  [{bar:40.cyan/bright_black}] {bytes}/{total_bytes} ({bytes_per_sec})",
                )
                .unwrap()
                .progress_chars("━━─"),
        );

        pb.set_message(filename.into());

        Self { pb }
    }

    /// Increment the progress
    pub fn inc(&self, delta: u64) {
        self.pb.inc(delta);
    }

    /// Set the current position
    pub fn set(&self, pos: u64) {
        self.pb.set_position(pos);
    }

    /// Set the current position (alias)
    pub fn set_position(&self, pos: u64) {
        self.set(pos);
    }

    /// Update the message
    #[allow(dead_code)]
    pub fn set_message(&self, message: impl Into<String>) {
        self.pb.set_message(message.into());
    }

    /// Finish the progress bar and clear it
    ///
    /// Requirement 4.4: Clear from terminal when complete
    pub fn finish(&self) {
        self.pb.finish_and_clear();
    }

    /// Finish the progress bar with a success message
    pub fn finish_success(self, message: impl Into<String>) {
        self.pb.finish_and_clear();
        eprintln!("  {} {}", icons::SUCCESS.green().bold(), message.into().white());
    }

    /// Finish the progress bar with an error
    #[allow(dead_code)]
    pub fn finish_error(self, message: impl Into<String>) {
        self.pb.finish_and_clear();
        eprintln!("  {} {}", icons::ERROR.red().bold(), message.into().red());
    }

    /// Get the inner progress bar for advanced usage
    #[allow(dead_code)]
    pub fn inner(&self) -> &ProgressBar {
        &self.pb
    }
}

/// Progress tracker for multiple parallel operations
///
/// Requirement 4.3: Support multiple concurrent progress bars
pub struct MultiProgressBar {
    mp: MultiProgress,
    bars: Vec<ProgressBar>,
}

impl MultiProgressBar {
    /// Create a new multi-progress bar container
    pub fn new() -> Self {
        Self {
            mp: MultiProgress::new(),
            bars: Vec::new(),
        }
    }

    /// Add a new progress bar
    ///
    /// Returns the index of the new bar for later updates
    pub fn add(&mut self, total: u64, message: impl Into<String>) -> usize {
        let pb = self.mp.add(ProgressBar::new(total));

        pb.set_style(
            ProgressStyle::default_bar()
                .template("  {prefix:.cyan} [{bar:30.cyan/bright_black}] {percent}%")
                .unwrap()
                .progress_chars("━━─"),
        );

        pb.set_prefix(message.into());

        self.bars.push(pb);
        self.bars.len() - 1
    }

    /// Set the position of a specific progress bar
    pub fn set(&self, index: usize, pos: u64) {
        if let Some(pb) = self.bars.get(index) {
            pb.set_position(pos);
        }
    }

    /// Increment a specific progress bar
    pub fn inc(&self, index: usize, delta: u64) {
        if let Some(pb) = self.bars.get(index) {
            pb.inc(delta);
        }
    }

    /// Finish a specific progress bar
    pub fn finish_one(&self, index: usize) {
        if let Some(pb) = self.bars.get(index) {
            pb.finish();
        }
    }

    /// Get the number of progress bars
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Check if there are no progress bars
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// Finish all progress bars
    pub fn finish_all(self) {
        for pb in self.bars {
            pb.finish_and_clear();
        }
    }
}

impl Default for MultiProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

// Keep the old name as an alias for backwards compatibility
pub type MultiProgressTracker = MultiProgressBar;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_progress_creation() {
        let progress = Progress::new(100, "Processing...");
        progress.set(50);
        progress.finish();
    }

    #[test]
    fn test_progress_download() {
        let progress = Progress::download(1024 * 1024, "file.zip");
        progress.inc(1024);
        progress.finish();
    }

    #[test]
    fn test_multi_progress_creation() {
        let mp = MultiProgressBar::new();
        assert!(mp.is_empty());
        assert_eq!(mp.len(), 0);
    }

    #[test]
    fn test_multi_progress_add() {
        let mut mp = MultiProgressBar::new();
        let idx = mp.add(100, "Task 1");
        assert_eq!(idx, 0);
        assert_eq!(mp.len(), 1);
        assert!(!mp.is_empty());
    }

    // Feature: dx-cli, Property 5: Multi-Progress Bar Addition
    // Validates: Requirements 4.3
    //
    // For any sequence of progress bar additions to a MultiProgressBar,
    // the number of bars should equal the number of add() calls.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_multi_progress_bar_count(num_bars in 1usize..20) {
            let mut mp = MultiProgressBar::new();

            for i in 0..num_bars {
                let idx = mp.add(100, format!("Task {}", i));
                prop_assert_eq!(idx, i, "Index should match iteration");
            }

            prop_assert_eq!(
                mp.len(),
                num_bars,
                "Number of bars should equal number of add() calls"
            );
        }

        #[test]
        fn prop_multi_progress_bar_indices_sequential(num_bars in 1usize..20) {
            let mut mp = MultiProgressBar::new();
            let mut indices = Vec::new();

            for i in 0..num_bars {
                let idx = mp.add(100, format!("Task {}", i));
                indices.push(idx);
            }

            // Verify indices are sequential starting from 0
            for (expected, actual) in indices.iter().enumerate() {
                prop_assert_eq!(
                    *actual,
                    expected,
                    "Indices should be sequential"
                );
            }
        }
    }
}
