//! Prompt interaction framework

use console::Key;
use std::io;

/// The current state of a prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The prompt is active and waiting for input.
    Active,
    /// The prompt was cancelled by the user.
    Cancel,
    /// The prompt was submitted successfully.
    Submit,
    /// An error occurred.
    Error,
}

/// Events that can occur during prompt interaction.
#[derive(Debug, Clone)]
pub enum Event {
    /// A key was pressed.
    Key(Key),
    /// An error occurred.
    #[allow(unused)]
    Error,
}

/// A trait for interactive prompts.
pub trait PromptInteraction {
    /// The type of value this prompt produces.
    type Output;

    /// Returns the current state of the prompt.
    fn state(&self) -> State;

    /// Handles an input event.
    fn on(&mut self, event: Event);

    /// Renders the prompt to the terminal.
    fn render(&mut self, term: &console::Term) -> io::Result<()>;

    /// Returns the final value when submitted.
    fn value(&self) -> Self::Output;

    /// Runs the interactive prompt loop.
    fn interact(&mut self) -> io::Result<Self::Output>
    where
        Self::Output: Clone,
    {
        let term = console::Term::stderr();

        // Hide cursor during interaction
        term.hide_cursor()?;

        // Initial render
        self.render(&term)?;

        let result = loop {
            // Read key
            let key = term.read_key()?;

            // Handle the event
            self.on(Event::Key(key));

            // Re-render
            self.render(&term)?;

            // Check state
            match self.state() {
                State::Submit => {
                    break Ok(self.value());
                }
                State::Cancel => {
                    break Err(io::Error::new(io::ErrorKind::Interrupted, "Cancelled"));
                }
                State::Error => {
                    break Err(io::Error::other("Error"));
                }
                State::Active => continue,
            }
        };

        // Show cursor again
        term.show_cursor()?;

        result
    }
}

/// A validation result for prompt inputs.
#[allow(unused)]
pub enum Validate<T> {
    /// The input is valid.
    Valid,
    /// The input is invalid with an error message.
    Invalid(T),
}

impl<T> From<Result<(), T>> for Validate<T> {
    fn from(result: Result<(), T>) -> Self {
        match result {
            Ok(()) => Validate::Valid,
            Err(e) => Validate::Invalid(e),
        }
    }
}

/// Clears lines above the cursor.
#[allow(unused)]
pub fn clear_lines(term: &console::Term, count: usize) -> io::Result<()> {
    for _ in 0..count {
        term.move_cursor_up(1)?;
        term.clear_line()?;
    }
    Ok(())
}

/// Writes a line with ANSI support.
#[allow(unused)]
pub fn write_line(term: &console::Term, text: &str) -> io::Result<()> {
    term.write_line(text)
}
