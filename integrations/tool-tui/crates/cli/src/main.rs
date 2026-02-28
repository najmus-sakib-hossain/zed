//! DX CLI - The Binary-First Development Experience
//!
//! A modern, high-performance CLI for the DX development platform.
//! Provides unified control over all dx-* tools with a clean, Vercel-like UX.

// Allow dead_code for API completeness
#![allow(dead_code)]
// Allow unnecessary qualifications - many type aliases use full paths for clarity
#![allow(unused_qualifications)]

use clap::Parser;

/// Helper macro for implementing `as_any` and `as_any_mut` for DxComponent
#[macro_export]
macro_rules! impl_component_any {
    ($type:ty) => {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    };
}

// Minimal modules for onboarding only
// mod agent;
// mod automation;
// mod channels;
mod cli;
mod commands;
// mod config;
mod confirm;
// mod deploy;
mod filter; // Keep for multiselect
// mod gateway;
mod input;
// pub mod io;
// pub use io as dx_io;
// pub mod llm;  // Commented out - references ui::chat
// mod memory;
mod multiprogress;
mod multiselect;
// pub mod nodejs;
mod password;
// mod plugin;
mod progress;
mod prompt;
pub mod prompts;
// mod registry;
// mod sandbox;
// mod security;
mod select;
// mod session;
// mod skills;
// mod templates;
#[cfg(test)]
mod tests;
mod theme;
// mod tokens;
mod ui; // Keep minimal UI for theme
mod utils;
mod validate;
mod view; // Keep for multiselect
// mod whatsapp;

use cli::Cli;
// use ui::logger::{Logger, StructuredLogger};
// use utils::crash::CrashReporter;
// use utils::network::NetworkClient;
// use utils::resource::ResourceManager;
// use utils::signal;

// Re-export cliclack functions
pub use confirm::Confirm;
pub use input::Input;
pub use multiprogress::MultiProgress;
pub use multiselect::MultiSelect;
pub use password::Password;
pub use progress::ProgressBar;
pub use select::Select;
pub use validate::Validate;

use console::Term;
use std::fmt::Display;

use theme::THEME;

pub use prompt::cursor::StringCursor;
pub use theme::{Theme, ThemeState, reset_theme, set_theme};

fn term_write(line: impl Display) -> std::io::Result<()> {
    Term::stderr().write_str(line.to_string().as_str())
}

pub fn clear_screen() -> std::io::Result<()> {
    Term::stdout().clear_screen()?;
    Term::stderr().clear_screen()
}

pub fn intro(title: impl Display) -> std::io::Result<()> {
    term_write(THEME.read().unwrap().format_intro(&title.to_string()))
}

pub fn outro(message: impl Display) -> std::io::Result<()> {
    term_write(THEME.read().unwrap().format_outro(&message.to_string()))
}

pub fn outro_cancel(message: impl Display) -> std::io::Result<()> {
    term_write(THEME.read().unwrap().format_outro_cancel(&message.to_string()))
}

pub fn outro_note(prompt: impl Display, message: impl Display) -> std::io::Result<()> {
    term_write(
        THEME
            .read()
            .unwrap()
            .format_outro_note(&prompt.to_string(), &message.to_string()),
    )
}

pub fn input(prompt: impl Display) -> Input {
    Input::new(prompt)
}

pub fn password(prompt: impl Display) -> Password {
    Password::new(prompt)
}

pub fn select<T: Clone + Eq>(prompt: impl Display) -> Select<T> {
    Select::new(prompt)
}

pub fn multiselect<T: Clone + Eq>(prompt: impl Display) -> MultiSelect<T> {
    MultiSelect::new(prompt)
}

pub fn confirm(prompt: impl Display) -> Confirm {
    Confirm::new(prompt)
}

pub fn spinner() -> ProgressBar {
    ProgressBar::new(0).with_spinner_template()
}

pub fn progress_bar(len: u64) -> ProgressBar {
    ProgressBar::new(len)
}

pub fn multi_progress(prompt: impl Display) -> MultiProgress {
    MultiProgress::new(prompt)
}

pub fn note(prompt: impl Display, message: impl Display) -> std::io::Result<()> {
    term_write(THEME.read().unwrap().format_note(&prompt.to_string(), &message.to_string()))
}

pub mod log {
    use super::*;

    fn log(text: impl Display, symbol: impl Display) -> std::io::Result<()> {
        term_write(THEME.read().unwrap().format_log(&text.to_string(), &symbol.to_string()))
    }

    pub fn remark(text: impl Display) -> std::io::Result<()> {
        let symbol = THEME.read().unwrap().remark_symbol();
        log(text, symbol)
    }

    pub fn info(text: impl Display) -> std::io::Result<()> {
        let symbol = THEME.read().unwrap().info_symbol();
        log(text, symbol)
    }

    pub fn warning(message: impl Display) -> std::io::Result<()> {
        let symbol = THEME.read().unwrap().warning_symbol();
        log(message, symbol)
    }

    pub fn error(message: impl Display) -> std::io::Result<()> {
        let symbol = THEME.read().unwrap().error_symbol();
        log(message, symbol)
    }

    pub fn success(message: impl Display) -> std::io::Result<()> {
        let symbol = THEME.read().unwrap().active_symbol();
        log(message, symbol)
    }

    pub fn step(message: impl Display) -> std::io::Result<()> {
        let symbol = THEME.read().unwrap().submit_symbol();
        log(message, symbol)
    }
}

#[tokio::main]
async fn main() {
    // Minimal main for onboarding only
    let cli = Cli::parse();
    let result = cli.run().await;

    if let Err(err) = result {
        eprintln!("Error: {err:#}");
        std::process::exit(1);
    }
}
