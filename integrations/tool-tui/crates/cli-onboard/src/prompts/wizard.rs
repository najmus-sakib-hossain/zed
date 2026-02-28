//! Multi-step wizard for guided workflows

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct WizardStep {
    pub title: String,
    pub description: String,
    pub completed: bool,
}

pub struct Wizard {
    message: String,
    steps: Vec<WizardStep>,
    current_step: usize,
    state: State,
    last_render_lines: usize,
}

impl Wizard {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            steps: Vec::new(),
            current_step: 0,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn step(mut self, title: impl Into<String>, description: impl Into<String>) -> Self {
        self.steps.push(WizardStep {
            title: title.into(),
            description: description.into(),
            completed: false,
        });
        self
    }

    pub fn complete_current_step(&mut self) {
        if self.current_step < self.steps.len() {
            self.steps[self.current_step].completed = true;
        }
    }

    pub fn next_step(&mut self) -> bool {
        if self.current_step < self.steps.len() - 1 {
            self.current_step += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_step(&mut self) -> bool {
        if self.current_step > 0 {
            self.current_step -= 1;
            true
        } else {
            false
        }
    }
}

impl PromptInteraction for Wizard {
    type Output = usize;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    self.complete_current_step();
                    if !self.next_step() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => self.state = State::Cancel,
                console::Key::ArrowRight | console::Key::Char('n') => {
                    self.next_step();
                }
                console::Key::ArrowLeft | console::Key::Char('p') => {
                    self.prev_step();
                }
                console::Key::Char(' ') => {
                    self.complete_current_step();
                }
                _ => {}
            },
            Event::Error => self.state = State::Error,
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
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    format!("  {}  ", self.message).bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // Progress indicator
                let progress = format!("Step {} of {}", self.current_step + 1, self.steps.len());
                term.write_line(&format!("{}  {}", bar, theme.primary.apply_to(progress).bold()))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // Show all steps with status
                for (i, step) in self.steps.iter().enumerate() {
                    let icon = if step.completed {
                        theme.success.apply_to("✓").to_string()
                    } else if i == self.current_step {
                        theme.primary.apply_to("▸").to_string()
                    } else {
                        theme.dim.apply_to("○").to_string()
                    };

                    let title = if i == self.current_step {
                        theme.primary.apply_to(&step.title).bold().to_string()
                    } else if step.completed {
                        theme.success.apply_to(&step.title).to_string()
                    } else {
                        theme.dim.apply_to(&step.title).to_string()
                    };

                    term.write_line(&format!("{}  {} {}", bar, icon, title))?;
                    lines += 1;
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // Current step description
                if let Some(step) = self.steps.get(self.current_step) {
                    term.write_line(&format!(
                        "{}  {}",
                        bar,
                        theme.dim.apply_to(&step.description)
                    ))?;
                    lines += 1;
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme
                        .dim
                        .apply_to("Enter: complete & next, Space: mark complete, ← →: navigate")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let completed = self.steps.iter().filter(|s| s.completed).count();
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!(
                        "Completed {} of {} steps",
                        completed,
                        self.steps.len()
                    ))
                ))?;
                lines += 1;
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            _ => {}
        }

        self.last_render_lines = lines;
        Ok(())
    }

    fn value(&self) -> usize {
        self.steps.iter().filter(|s| s.completed).count()
    }
}

pub fn wizard(message: impl Into<String>) -> Wizard {
    Wizard::new(message)
}
