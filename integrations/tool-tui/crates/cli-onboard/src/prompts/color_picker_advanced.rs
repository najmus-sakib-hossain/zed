//! Advanced color picker with RGB/HSL/HEX support

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

#[derive(PartialEq, Clone, Copy)]
pub enum ColorMode {
    RGB,
    HSL,
    HEX,
}

pub struct ColorPickerAdvanced {
    message: String,
    mode: ColorMode,
    r: u8,
    g: u8,
    b: u8,
    active_channel: usize,
    state: State,
    last_render_lines: usize,
}

impl ColorPickerAdvanced {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            mode: ColorMode::RGB,
            r: 128,
            g: 128,
            b: 128,
            active_channel: 0,
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

    pub fn mode(mut self, mode: ColorMode) -> Self {
        self.mode = mode;
        self
    }

    fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    fn to_hsl(&self) -> (u16, u8, u8) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let l = (max + min) / 2.0;

        if delta == 0.0 {
            return (0, 0, (l * 100.0) as u8);
        }

        let s = if l < 0.5 {
            delta / (max + min)
        } else {
            delta / (2.0 - max - min)
        };

        let h = if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let h = if h < 0.0 { h + 360.0 } else { h };

        (h as u16, (s * 100.0) as u8, (l * 100.0) as u8)
    }

    fn render_color_preview(&self) -> String {
        format!("██████ {}", self.to_hex())
    }
}

impl PromptInteraction for ColorPickerAdvanced {
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
                        ColorMode::RGB => ColorMode::HSL,
                        ColorMode::HSL => ColorMode::HEX,
                        ColorMode::HEX => ColorMode::RGB,
                    };
                }
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.active_channel > 0 {
                        self.active_channel -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if self.active_channel < 2 {
                        self.active_channel += 1;
                    }
                }
                console::Key::ArrowRight | console::Key::Char('l') => match self.active_channel {
                    0 => self.r = self.r.saturating_add(5),
                    1 => self.g = self.g.saturating_add(5),
                    2 => self.b = self.b.saturating_add(5),
                    _ => {}
                },
                console::Key::ArrowLeft | console::Key::Char('h') => match self.active_channel {
                    0 => self.r = self.r.saturating_sub(5),
                    1 => self.g = self.g.saturating_sub(5),
                    2 => self.b = self.b.saturating_sub(5),
                    _ => {}
                },
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
                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.primary.apply_to(self.render_color_preview())
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // RGB values
                let r_marker = if self.active_channel == 0 { "▸" } else { " " };
                let g_marker = if self.active_channel == 1 { "▸" } else { " " };
                let b_marker = if self.active_channel == 2 { "▸" } else { " " };

                let r_display = if self.active_channel == 0 {
                    theme.primary.apply_to(format!("{:3}", self.r)).bold().to_string()
                } else {
                    format!("{:3}", self.r)
                };

                let g_display = if self.active_channel == 1 {
                    theme.primary.apply_to(format!("{:3}", self.g)).bold().to_string()
                } else {
                    format!("{:3}", self.g)
                };

                let b_display = if self.active_channel == 2 {
                    theme.primary.apply_to(format!("{:3}", self.b)).bold().to_string()
                } else {
                    format!("{:3}", self.b)
                };

                term.write_line(&format!("{}  {} R: {}", bar, r_marker, r_display))?;
                lines += 1;

                term.write_line(&format!("{}  {} G: {}", bar, g_marker, g_display))?;
                lines += 1;

                term.write_line(&format!("{}  {} B: {}", bar, b_marker, b_display))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // HSL values
                let (h, s, l) = self.to_hsl();
                term.write_line(&format!(
                    "{}  HSL: {}°, {}%, {}%",
                    bar,
                    theme.dim.apply_to(h),
                    theme.dim.apply_to(s),
                    theme.dim.apply_to(l)
                ))?;
                lines += 1;

                // HEX value
                term.write_line(&format!("{}  HEX: {}", bar, theme.dim.apply_to(self.to_hex())))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme
                        .dim
                        .apply_to("↑↓: select channel, ← →: adjust, Tab: mode, Enter: confirm")
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

pub fn color_picker_advanced(message: impl Into<String>) -> ColorPickerAdvanced {
    ColorPickerAdvanced::new(message)
}
