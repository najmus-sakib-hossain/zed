//! Toggle prompt for on/off selections

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A toggle prompt for on/off selections.
pub struct Toggle {
    message: String,
    value: bool,
    on_label: String,
    off_label: String,
    state: State,
    last_render_lines: usize,
}

impl Toggle {
    /// Creates a new toggle prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: false,
            on_label: "On".to_string(),
            off_label: "Off".to_string(),
            state: State::Active,
            last_render_lines: 0,
        }
    }

    /// Sets custom labels for on/off states.
    pub fn labels(mut self, on: impl Into<String>, off: impl Into<String>) -> Self {
        self.on_label = on.into();
        self.off_label = off.into();
        self
    }

    /// Sets the initial value.
    pub fn initial_value(mut self, value: bool) -> Self {
        self.value = value;
        self
    }

    fn render_toggle(&self, theme: &super::DxTheme) -> String {
        if self.value {
            format!(
                "{}{}{}",
                theme.success.apply_to("[ "),
                theme.success.apply_to("●").bold(),
                theme.dim.apply_to(" ○ ]")
            )
        } else {
            format!(
                "{}{}{}",
                theme.dim.apply_to("[ ○ "),
                theme.primary.apply_to("●").bold(),
                theme.primary.apply_to(" ]")
            )
        }
    }
}

impl PromptInteraction for Toggle {
    type Output = bool;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    self.state = State::Submit;
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::ArrowLeft
                | console::Key::ArrowRight
                | console::Key::Char(' ')
                | console::Key::Tab => {
                    self.value = !self.value;
                }
                console::Key::Char('y') | console::Key::Char('Y') => {
                    self.value = true;
                }
                console::Key::Char('n') | console::Key::Char('N') => {
                    self.value = false;
                }
                _ => {}
            },
            Event::Error => {
                self.state = State::Error;
            }
        }
    }

    fn render(&mut self, term: &Term) -> io::Result<()> {
        if self.last_render_lines > 0 {
            for _ in 0..self.last_render_lines {
                term.move_cursor_up(1)?;
                term.clear_line()?;
            }
        }

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let mut lines = 0;

        match self.state {
            State::Active => {
                let bar = theme.dim.apply_to(symbols.bar);
                let title_with_spaces = format!("  {}  ", self.message);
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    title_with_spaces.bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // Toggle visualization
                let toggle = self.render_toggle(&theme);
                let label = if self.value {
                    theme.success.apply_to(&self.on_label).bold()
                } else {
                    theme.primary.apply_to(&self.off_label).bold()
                };
                term.write_line(&format!("{}  {}  {}", bar, toggle, label))?;
                lines += 1;

                // Hint
                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme
                        .dim
                        .apply_to("Use Space/Tab to toggle, Enter to confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let display = theme.dim.apply_to(if self.value {
                    &self.on_label
                } else {
                    &self.off_label
                });
                term.write_line(&format!("{} {}  {}", checkmark, self.message, display))?;
                lines += 1;
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            State::Cancel => {
                let bar = theme.dim.apply_to(symbols.bar);
                let symbol = theme.error.apply_to(symbols.step_submit);
                term.write_line(&format!(
                    "{}{} {}  {}",
                    bar,
                    symbol,
                    self.message.strikethrough(),
                    theme.dim.apply_to("cancelled")
                ))?;
                lines += 1;
            }
            State::Error => {
                let bar = theme.dim.apply_to(symbols.bar);
                let symbol = theme.error.apply_to(symbols.step_submit);
                term.write_line(&format!(
                    "{}{} {}  {}",
                    bar,
                    symbol,
                    self.message.bold(),
                    theme.error.apply_to("error")
                ))?;
                lines += 1;
            }
        }

        self.last_render_lines = lines;
        Ok(())
    }

    fn value(&self) -> bool {
        self.value
    }
}

/// Creates a new toggle prompt.
pub fn toggle(message: impl Into<String>) -> Toggle {
    Toggle::new(message)
}
