//! Animated spinner for async operations

use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io::{self, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

/// Spinner animation frames
#[allow(unused)]
const SPINNER_FRAMES: &[&str] = &["◒", "◐", "◓", "◑"];

/// An animated spinner for showing async progress.
#[allow(unused)]
pub struct Spinner {
    message: String,
    term: Term,
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

#[allow(unused)]
impl Spinner {
    /// Creates a new spinner with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            term: Term::stderr(),
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// Starts the spinner animation.
    pub fn start(&mut self) -> io::Result<()> {
        self.running.store(true, Ordering::SeqCst);

        // Hide cursor
        self.term.hide_cursor()?;

        let message = self.message.clone();
        let running = self.running.clone();
        let term = self.term.clone();

        self.handle = Some(thread::spawn(move || {
            let theme = THEME.read().unwrap();
            let symbols = &*SYMBOLS;
            let bar = theme.dim.apply_to(symbols.bar).to_string();
            let mut frame_idx = 0;

            while running.load(Ordering::SeqCst) {
                let spinner = theme
                    .primary
                    .apply_to(SPINNER_FRAMES[frame_idx])
                    .to_string();

                let _ = term.clear_line();
                let _ = write!(&term, "\r{} {} {}", spinner, message.bold(), bar);
                let _ = term.flush();

                frame_idx = (frame_idx + 1) % SPINNER_FRAMES.len();
                thread::sleep(Duration::from_millis(80));
            }
        }));

        Ok(())
    }

    /// Updates the spinner message.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Stops the spinner with a success message.
    pub fn stop(&mut self, message: impl Into<String>) -> io::Result<()> {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let symbol = theme.success.apply_to(symbols.step_submit);
        let msg = message.into();

        self.term.clear_line()?;
        self.term
            .write_line(&format!("{} {}", symbol, msg.bold()))?;

        // Show cursor again
        self.term.show_cursor()?;

        Ok(())
    }

    /// Stops the spinner with an error.
    pub fn stop_error(&mut self, message: impl Into<String>) -> io::Result<()> {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let symbol = theme.error.apply_to(symbols.step_active);
        let msg = message.into();

        self.term.clear_line()?;
        self.term
            .write_line(&format!("{} {}", symbol, theme.error.apply_to(msg)))?;

        // Show cursor again
        self.term.show_cursor()?;

        Ok(())
    }

    /// Stops the spinner without a message (cancelled).
    pub fn cancel(&mut self) -> io::Result<()> {
        self.running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }

        self.term.clear_line()?;

        // Show cursor again
        self.term.show_cursor()?;

        Ok(())
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            let _ = self.cancel();
        }
    }
}

/// Creates a new spinner.
#[allow(unused)]
pub fn spinner(message: impl Into<String>) -> Spinner {
    Spinner::new(message)
}
