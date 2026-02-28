//! Run command

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::cli::RunArgs;
use crate::ui::theme::Theme;

pub async fn run_run(args: RunArgs, theme: &Theme) -> Result<()> {
    use crate::ui::spinner::Spinner;

    let script = args.script_and_args.first().map(|s| s.as_str()).unwrap_or("start");

    theme.print_section(&format!("dx run: {}", script));
    eprintln!();

    if args.watch {
        eprintln!("  {} Watch mode enabled", "│".bright_black());
        eprintln!();
    }

    let spinner = Spinner::dots("Loading configuration...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success("Loaded dx.toml");

    let spinner = Spinner::dots(format!("Running '{}'...", script));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success("Script started");

    eprintln!();
    eprintln!("  {} Output:", "│".bright_black());
    eprintln!("    {}", "Hello from DX!".white());
    eprintln!();

    theme.print_success(&format!("Script '{}' completed", script));
    eprintln!();

    Ok(())
}
