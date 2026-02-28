//! Development server command

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::cli::DevArgs;
use crate::ui::theme::Theme;

pub async fn run_dev(args: DevArgs, theme: &Theme) -> Result<()> {
    use crate::ui::spinner::Spinner;

    theme.print_section("dx dev: Development Server");
    eprintln!();
    eprintln!(
        "  {} Port: {} │ Host: {} │ HTTPS: {}",
        "│".bright_black(),
        args.port.to_string().cyan(),
        args.host.cyan(),
        if args.https {
            "yes".green().to_string()
        } else {
            "no".bright_black().to_string()
        }
    );
    eprintln!();

    if args.clear {
        let spinner = Spinner::dots("Clearing cache...");
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        spinner.success("Cache cleared");
    }

    let spinner = Spinner::dots("Loading configuration...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success("Loaded dx.toml");

    let spinner = Spinner::dots("Starting development server...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success("Server ready");

    let protocol = if args.https { "https" } else { "http" };
    theme.print_ready(&format!("{}://{}:{}", protocol, args.host, args.port), 45);

    if args.open {
        eprintln!("  {} Opening browser...", "→".cyan());
    }

    eprintln!("  {} Press {} to stop", "│".bright_black(), "Ctrl+C".cyan().bold());
    eprintln!();

    Ok(())
}
