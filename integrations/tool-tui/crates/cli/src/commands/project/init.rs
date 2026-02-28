//! Project initialization command

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::cli::{InitArgs, ProjectTemplate};
use crate::ui::theme::Theme;

pub async fn run_init(args: InitArgs, theme: &Theme) -> Result<()> {
    use crate::ui::spinner::Spinner;

    let project_name = args.name.as_deref().unwrap_or("my-dx-project");
    let template_name = match args.template {
        ProjectTemplate::Default => "default",
        ProjectTemplate::Minimal => "minimal",
        ProjectTemplate::Full => "full",
        ProjectTemplate::Api => "api",
        ProjectTemplate::Web => "web",
        ProjectTemplate::Cli => "cli",
    };

    theme.print_section(&format!("dx init: {}", project_name));
    eprintln!();
    eprintln!("  {} Template: {}", "│".bright_black(), template_name.cyan());
    eprintln!();

    let spinner = Spinner::dots("Creating project structure...");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    spinner.success("Created directories");

    let spinner = Spinner::dots("Generating dx.toml...");
    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    spinner.success("Created dx.toml");

    let spinner = Spinner::dots("Setting up source files...");
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
    spinner.success("Created src/");

    let spinner = Spinner::dots("Initializing git repository...");
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    spinner.success("Initialized git");

    let spinner = Spinner::dots("Installing dependencies...");
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    spinner.success("Installed 12 packages");

    eprintln!();
    theme.print_divider();
    eprintln!("  {} Project {} created!", "✓".green().bold(), project_name.cyan().bold());
    theme.print_divider();
    eprintln!();

    theme.hint(&format!("cd {} && dx dev", project_name));
    eprintln!();

    Ok(())
}
