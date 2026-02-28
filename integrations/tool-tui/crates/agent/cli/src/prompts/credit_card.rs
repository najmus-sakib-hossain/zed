//! Credit card input with validation

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct CreditCardInput {
    message: String,
    number: String,
    expiry: String,
    cvv: String,
    active_field: CardField,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

#[derive(PartialEq)]
enum CardField {
    Number,
    Expiry,
    CVV,
}

impl CreditCardInput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            number: String::new(),
            expiry: String::new(),
            cvv: String::new(),
            active_field: CardField::Number,
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    fn format_card_number(&self) -> String {
        let digits: String = self.number.chars().filter(|c| c.is_ascii_digit()).collect();
        let mut formatted = String::new();

        for (i, c) in digits.chars().enumerate() {
            if i > 0 && i % 4 == 0 {
                formatted.push(' ');
            }
            formatted.push(c);
        }

        formatted
    }

    fn format_expiry(&self) -> String {
        let digits: String = self.expiry.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() <= 2 {
            digits
        } else {
            format!("{}/{}", &digits[..2], &digits[2..])
        }
    }

    fn validate_card(&self) -> Result<(), String> {
        let digits: String = self.number.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() < 13 || digits.len() > 19 {
            return Err("Invalid card number length".to_string());
        }

        // Luhn algorithm
        let mut sum = 0;
        let mut double = false;

        for c in digits.chars().rev() {
            let mut digit = c.to_digit(10).unwrap();

            if double {
                digit *= 2;
                if digit > 9 {
                    digit -= 9;
                }
            }

            sum += digit;
            double = !double;
        }

        if sum % 10 != 0 {
            return Err("Invalid card number (failed Luhn check)".to_string());
        }

        let expiry_digits: String = self.expiry.chars().filter(|c| c.is_ascii_digit()).collect();
        if expiry_digits.len() != 4 {
            return Err("Expiry must be MMYY format".to_string());
        }

        let cvv_digits: String = self.cvv.chars().filter(|c| c.is_ascii_digit()).collect();
        if cvv_digits.len() < 3 || cvv_digits.len() > 4 {
            return Err("CVV must be 3 or 4 digits".to_string());
        }

        Ok(())
    }

    fn detect_card_type(&self) -> &'static str {
        let digits: String = self.number.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.starts_with('4') {
            "💳 Visa"
        } else if digits.starts_with("51")
            || digits.starts_with("52")
            || digits.starts_with("53")
            || digits.starts_with("54")
            || digits.starts_with("55")
        {
            "💳 Mastercard"
        } else if digits.starts_with("34") || digits.starts_with("37") {
            "💳 Amex"
        } else if digits.starts_with("6011") || digits.starts_with("65") {
            "💳 Discover"
        } else {
            "💳 Card"
        }
    }
}

impl PromptInteraction for CreditCardInput {
    type Output = (String, String, String);

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => match self.validate_card() {
                    Ok(_) => self.state = State::Submit,
                    Err(msg) => self.error_message = Some(msg),
                },
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.active_field = match self.active_field {
                        CardField::Number => CardField::Expiry,
                        CardField::Expiry => CardField::CVV,
                        CardField::CVV => CardField::Number,
                    };
                    self.error_message = None;
                }
                console::Key::Backspace => {
                    match self.active_field {
                        CardField::Number => {
                            self.number.pop();
                        }
                        CardField::Expiry => {
                            self.expiry.pop();
                        }
                        CardField::CVV => {
                            self.cvv.pop();
                        }
                    }
                    self.error_message = None;
                }
                console::Key::Char(c) if c.is_ascii_digit() => {
                    match self.active_field {
                        CardField::Number if self.number.len() < 19 => {
                            self.number.push(c);
                        }
                        CardField::Expiry if self.expiry.len() < 4 => {
                            self.expiry.push(c);
                        }
                        CardField::CVV if self.cvv.len() < 4 => {
                            self.cvv.push(c);
                        }
                        _ => {}
                    }
                    self.error_message = None;
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

                let card_type = self.detect_card_type();
                term.write_line(&format!("{}  {}", bar, theme.primary.apply_to(card_type)))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let number_marker = if self.active_field == CardField::Number {
                    "▸"
                } else {
                    " "
                };
                let expiry_marker = if self.active_field == CardField::Expiry {
                    "▸"
                } else {
                    " "
                };
                let cvv_marker = if self.active_field == CardField::CVV {
                    "▸"
                } else {
                    " "
                };

                let number_display = if self.format_card_number().is_empty() {
                    format!("█{}", theme.dim.apply_to("1234 5678 9012 3456"))
                } else {
                    format!("{}█", self.format_card_number())
                };

                let expiry_display = if self.format_expiry().is_empty() {
                    format!("█{}", theme.dim.apply_to("MM/YY"))
                } else {
                    format!("{}█", self.format_expiry())
                };

                let cvv_display = if self.cvv.is_empty() {
                    format!("█{}", theme.dim.apply_to("123"))
                } else {
                    format!("{}█", "•".repeat(self.cvv.len()))
                };

                let number_text = if self.active_field == CardField::Number {
                    theme.primary.apply_to(number_display).bold().to_string()
                } else {
                    self.format_card_number()
                };

                let expiry_text = if self.active_field == CardField::Expiry {
                    theme.primary.apply_to(expiry_display).bold().to_string()
                } else {
                    self.format_expiry()
                };

                let cvv_text = if self.active_field == CardField::CVV {
                    theme.primary.apply_to(cvv_display).bold().to_string()
                } else {
                    "•".repeat(self.cvv.len())
                };

                term.write_line(&format!(
                    "{}  {} Card Number: {}",
                    bar, number_marker, number_text
                ))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {} Expiry:      {}",
                    bar, expiry_marker, expiry_text
                ))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {} CVV:         {}",
                    bar, cvv_marker, cvv_text
                ))?;
                lines += 1;

                if let Some(error) = &self.error_message {
                    term.write_line(&format!("{}  {}", bar, theme.error.apply_to(error)))?;
                    lines += 1;
                }

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Tab: next field, Enter: submit")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let masked = format!(
                    "**** **** **** {}",
                    &self
                        .format_card_number()
                        .chars()
                        .rev()
                        .take(4)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>()
                );
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(masked)
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

    fn value(&self) -> (String, String, String) {
        (
            self.format_card_number(),
            self.format_expiry(),
            self.cvv.clone(),
        )
    }
}

pub fn credit_card(message: impl Into<String>) -> CreditCardInput {
    CreditCardInput::new(message)
}
