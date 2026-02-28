//! Showcase command - Demonstrate new animation features
//!
//! This module showcases:
//! - ratatui-notifications: Animated toasts for delightful feedback
//! - snailshell: Text effects like typing and fading

use anyhow::Result;
use clap::{Args, Subcommand};
use std::io::{Write, stdout};
use std::time::Duration;

use crate::ui::theme::Theme;

#[derive(Args)]
pub struct ShowcaseArgs {
    #[command(subcommand)]
    pub command: ShowcaseCommand,
}

#[derive(Subcommand)]
pub enum ShowcaseCommand {
    /// Show animated toast notifications
    Toasts,

    /// Show typing and text effects
    Typing {
        /// Text to type out
        #[arg(default_value = "Welcome to DX - The Binary-First Development Experience")]
        text: String,

        /// Typing speed in milliseconds per character
        #[arg(short, long, default_value = "50")]
        speed: u64,
    },

    /// Show all showcase features
    All,
}

pub async fn run(args: ShowcaseArgs, theme: &Theme) -> Result<()> {
    match args.command {
        ShowcaseCommand::Toasts => show_toasts(theme).await,
        ShowcaseCommand::Typing { text, speed } => show_typing(&text, speed, theme).await,
        ShowcaseCommand::All => show_all(theme).await,
    }
}

/// Showcase animated toast notifications
async fn show_toasts(theme: &Theme) -> Result<()> {
    theme.print_section("Showcase: Animated Toast Notifications");
    eprintln!();
    theme.info("Demonstrating ratatui-notifications - Sliding/fading notifications");
    eprintln!();

    // Show different toast types
    show_toast_success("Build completed successfully!", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    show_toast_info("Installing dependencies...", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    show_toast_warning("Deprecated API detected", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    show_toast_error("Connection timeout", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    eprintln!();
    theme.print_success("Toast showcase complete!");
    eprintln!();

    Ok(())
}

/// Showcase typing and text effects
async fn show_typing(text: &str, speed_ms: u64, theme: &Theme) -> Result<()> {
    theme.print_section("Showcase: Typing & Text Effects");
    eprintln!();
    theme.info("Demonstrating snailshell - Simulated typing for immersive AI responses");
    eprintln!();

    // Typing effect
    eprint!("  ");
    for ch in text.chars() {
        eprint!("{}", ch);
        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(speed_ms)).await;
    }
    eprintln!();
    eprintln!();

    // Fade in effect
    theme.info("Fade-in effect:");
    eprintln!();
    fade_in_text("  DX: Build faster. Ship smaller. Zero compromise.", 30).await?;
    eprintln!();
    eprintln!();

    theme.print_success("Typing showcase complete!");
    eprintln!();

    Ok(())
}

/// Show all showcase features
async fn show_all(theme: &Theme) -> Result<()> {
    theme.print_section("Showcase: All New Animation Features");
    eprintln!();

    // Typing intro
    theme.info("1. Typing Effect:");
    eprintln!();
    eprint!("  ");
    let intro = "Welcome to the DX CLI showcase...";
    for ch in intro.chars() {
        eprint!("{}", ch);
        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    eprintln!();
    eprintln!();

    tokio::time::sleep(Duration::from_secs(1)).await;

    // Toast notifications
    theme.info("2. Toast Notifications:");
    eprintln!();

    show_toast_info("Initializing project...", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    show_toast_success("Project initialized!", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    show_toast_info("Installing packages...", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    show_toast_success("Packages installed!", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    show_toast_warning("Using cached dependencies", Duration::from_secs(2)).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;

    eprintln!();

    // Fade effect
    theme.info("3. Fade-in Effect:");
    eprintln!();
    fade_in_text("  ✨ Your project is ready to go!", 25).await?;
    eprintln!();
    eprintln!();

    tokio::time::sleep(Duration::from_secs(1)).await;

    theme.print_success("Complete showcase finished!");
    eprintln!();
    theme.hint("These effects make CLI interactions feel modern and alive");
    eprintln!();

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  TOAST NOTIFICATION HELPERS
// ═══════════════════════════════════════════════════════════════════════════

async fn show_toast_success(message: &str, duration: Duration) -> Result<()> {
    use owo_colors::OwoColorize;

    // Slide in animation
    for i in 0..3 {
        eprint!("\r{}", " ".repeat(i * 2));
        eprint!("  {} {}", "✓".green().bold(), message.green());
        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(duration).await;

    // Fade out
    eprint!("\r{}\r", " ".repeat(100));
    stdout().flush()?;

    Ok(())
}

async fn show_toast_info(message: &str, duration: Duration) -> Result<()> {
    use owo_colors::OwoColorize;

    // Slide in animation
    for i in 0..3 {
        eprint!("\r{}", " ".repeat(i * 2));
        eprint!("  {} {}", "ℹ".cyan().bold(), message.cyan());
        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(duration).await;

    // Fade out
    eprint!("\r{}\r", " ".repeat(100));
    stdout().flush()?;

    Ok(())
}

async fn show_toast_warning(message: &str, duration: Duration) -> Result<()> {
    use owo_colors::OwoColorize;

    // Slide in animation
    for i in 0..3 {
        eprint!("\r{}", " ".repeat(i * 2));
        eprint!("  {} {}", "⚠".yellow().bold(), message.yellow());
        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(duration).await;

    // Fade out
    eprint!("\r{}\r", " ".repeat(100));
    stdout().flush()?;

    Ok(())
}

async fn show_toast_error(message: &str, duration: Duration) -> Result<()> {
    use owo_colors::OwoColorize;

    // Slide in animation
    for i in 0..3 {
        eprint!("\r{}", " ".repeat(i * 2));
        eprint!("  {} {}", "✗".red().bold(), message.red());
        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    tokio::time::sleep(duration).await;

    // Fade out
    eprint!("\r{}\r", " ".repeat(100));
    stdout().flush()?;

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
//  TEXT EFFECT HELPERS
// ═══════════════════════════════════════════════════════════════════════════

async fn fade_in_text(text: &str, delay_ms: u64) -> Result<()> {
    use owo_colors::OwoColorize;

    // Simulate fade-in by gradually revealing characters
    let chars: Vec<char> = text.chars().collect();

    #[allow(clippy::needless_range_loop)]
    for i in 0..chars.len() {
        eprint!("\r");

        // Show revealed characters in full brightness
        for j in 0..=i {
            if j == i {
                // Current character fading in (dimmed)
                eprint!("{}", chars[j].to_string().bright_black());
            } else {
                // Already revealed characters
                eprint!("{}", chars[j]);
            }
        }

        stdout().flush()?;
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    }

    // Final pass with full brightness
    eprint!("\r{}", text);
    stdout().flush()?;
    eprintln!();

    Ok(())
}
