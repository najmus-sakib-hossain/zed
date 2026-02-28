//! Color picker prompt for RGB/Hex color selection

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct ColorPicker {
    message: String,
    r: u8,
    g: u8,
    b: u8,
    mode: ColorMode,
    state: State,
    last_render_lines: usize,
}

#[derive(PartialEq)]
enum ColorMode {
    Red,
    Green,
    Blue,
}

impl ColorPicker {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            r: 128,
            g: 128,
            b: 128,
            mode: ColorMode::Red,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn initial_color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.r = r;
        self.g = g;
        self.b = b;
        self
    }

    fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl PromptInteraction for ColorPicker {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => self.state = State::Submit,
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.mode = match self.mode {
                        ColorMode::Red => ColorMode::Green,
                        ColorMode::Green => ColorMode::Blue,
                        ColorMode::Blue => ColorMode::Red,
                    };
                }
                console::Key::ArrowUp | console::Key::Char('+') => {
                    let val = match self.mode {
                        ColorMode::Red => &mut self.r,
                        ColorMode::Green => &mut self.g,
                        ColorMode::Blue => &mut self.b,
                    };
                    *val = val.saturating_add(5);
                }
                console::Key::ArrowDown | console::Key::Char('-') => {
                    let val = match self.mode {
                        ColorMode::Red => &mut self.r,
                        ColorMode::Green => &mut self.g,
                        ColorMode::Blue => &mut self.b,
                    };
                    *val = val.saturating_sub(5);
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

                // Color preview
                let preview = format!("████████ {}", self.to_hex());
                term.write_line(&format!("{}  {}", bar, preview))?;
                lines += 1;

                // RGB sliders
                let r_marker = if self.mode == ColorMode::Red { ">" } else { " " };
                let g_marker = if self.mode == ColorMode::Green { ">" } else { " " };
                let b_marker = if self.mode == ColorMode::Blue { ">" } else { " " };

                term.write_line(&format!("{}  {} R: {:3}", bar, r_marker, self.r))?;
                lines += 1;
                term.write_line(&format!("{}  {} G: {:3}", bar, g_marker, self.g))?;
                lines += 1;
                term.write_line(&format!("{}  {} B: {:3}", bar, b_marker, self.b))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Tab: switch, ↑↓: adjust, Enter: confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(self.to_hex())
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

    fn value(&self) -> String {
        self.to_hex()
    }
}

pub fn color_picker(message: impl Into<String>) -> ColorPicker {
    ColorPicker::new(message)
}
