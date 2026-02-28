//! Progress bar for tracking completion

use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A progress bar for tracking completion of tasks.
#[allow(unused)]
pub struct ProgressBar {
    message: String,
    current: u64,
    total: u64,
    width: usize,
    term: Term,
    last_render_lines: usize,
}

#[allow(unused)]
impl ProgressBar {
    /// Creates a new progress bar.
    pub fn new(message: impl Into<String>, total: u64) -> Self {
        Self {
            message: message.into(),
            current: 0,
            total,
            width: 30,
            term: Term::stderr(),
            last_render_lines: 0,
        }
    }

    /// Sets the progress bar width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    /// Starts the progress bar.
    pub fn start(&mut self) -> io::Result<()> {
        self.term.hide_cursor()?;
        self.render()
    }

    /// Updates the current progress.
    pub fn set(&mut self, value: u64) -> io::Result<()> {
        self.current = value.min(self.total);
        self.render()
    }

    /// Increments the progress by the given amount.
    pub fn inc(&mut self, delta: u64) -> io::Result<()> {
        self.current = (self.current + delta).min(self.total);
        self.render()
    }

    /// Updates the message.
    pub fn set_message(&mut self, message: impl Into<String>) -> io::Result<()> {
        self.message = message.into();
        self.render()
    }

    /// Finishes the progress bar with a success message.
    pub fn finish(&mut self, message: impl Into<String>) -> io::Result<()> {
        self.clear()?;

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let symbol = theme.success.apply_to(symbols.step_submit);
        let msg = message.into();

        self.term.write_line(&format!("{} {}", symbol, msg.bold()))?;
        self.term.show_cursor()?;
        self.last_render_lines = 1;
        Ok(())
    }

    /// Finishes the progress bar with an error message.
    pub fn finish_error(&mut self, message: impl Into<String>) -> io::Result<()> {
        self.clear()?;

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let symbol = theme.error.apply_to(symbols.step_active);
        let msg = message.into();

        self.term.write_line(&format!("{} {}", symbol, theme.error.apply_to(msg)))?;
        self.term.show_cursor()?;
        self.last_render_lines = 1;
        Ok(())
    }

    /// Clears the progress bar from the terminal.
    fn clear(&mut self) -> io::Result<()> {
        for _ in 0..self.last_render_lines {
            self.term.move_cursor_up(1)?;
            self.term.clear_line()?;
        }
        Ok(())
    }

    /// Renders the progress bar.
    fn render(&mut self) -> io::Result<()> {
        self.clear()?;

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let mut lines = 0;

        // Title line
        let symbol = theme.primary.apply_to(symbols.step_active);
        self.term.write_line(&format!("{} {}", symbol, self.message.bold()))?;
        lines += 1;

        // Progress bar line
        let bar = theme.dim.apply_to(symbols.bar);
        let progress = if self.total > 0 {
            (self.current as f64 / self.total as f64).min(1.0)
        } else {
            0.0
        };
        let filled = (progress * self.width as f64) as usize;
        let empty = self.width - filled;

        let filled_bar = theme.primary.apply_to("█".repeat(filled));
        let empty_bar = theme.dim.apply_to("░".repeat(empty));
        let percent = (progress * 100.0) as u32;

        let padding = if percent < 10 { " " } else { "" };

        self.term.write_line(&format!(
            "{}  {}{} {}{}% ({}/{})",
            bar, filled_bar, empty_bar, padding, percent, self.current, self.total
        ))?;
        lines += 1;

        // Bottom bar
        let bar_end = theme.dim.apply_to(symbols.bar_end);
        self.term.write_line(&format!("{}", bar_end))?;
        lines += 1;

        self.last_render_lines = lines;
        self.term.flush()?;
        Ok(())
    }
}

/// Creates a new progress bar.
#[allow(unused)]
pub fn progress(message: impl Into<String>, total: u64) -> ProgressBar {
    ProgressBar::new(message, total)
}
