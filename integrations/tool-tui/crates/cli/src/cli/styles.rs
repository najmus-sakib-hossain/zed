//! Custom clap styles for CLI

use clap::builder::styling::{AnsiColor, Styles};

/// Custom clap styles for a modern look
pub fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default().bold())
        .usage(AnsiColor::Cyan.on_default().bold())
        .literal(AnsiColor::White.on_default().bold())
        .placeholder(AnsiColor::BrightBlack.on_default())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Red.on_default())
        .error(AnsiColor::Red.on_default().bold())
}
