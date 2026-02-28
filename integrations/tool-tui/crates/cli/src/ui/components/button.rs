//! Button component

use crossterm::style::{Color, Stylize};

pub struct Button {
    pub label: String,
    pub variant: ButtonVariant,
    pub disabled: bool,
}

pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Success,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Primary,
            disabled: false,
        }
    }

    pub fn render(&self) -> String {
        let color = match self.variant {
            ButtonVariant::Primary => Color::Blue,
            ButtonVariant::Secondary => Color::Grey,
            ButtonVariant::Danger => Color::Red,
            ButtonVariant::Success => Color::Green,
        };

        if self.disabled {
            format!("[ {} ]", self.label.clone().dark_grey())
        } else {
            format!("[ {} ]", self.label.clone().with(color).bold())
        }
    }
}
