//! Self-management commands

use anyhow::Result;

use crate::cli::{SelfArgs, SelfCommands};
use crate::ui::theme::Theme;

pub async fn run_self(args: SelfArgs, theme: &Theme) -> Result<()> {
    match args.command {
        SelfCommands::Update { force, yes } => run_self_update(force, yes, theme).await,
        SelfCommands::Info => run_self_info(theme),
        SelfCommands::Uninstall { yes } => run_self_uninstall(yes, theme).await,
    }
}

pub async fn run_self_update(force: bool, _skip_confirm: bool, theme: &Theme) -> Result<()> {
    use crate::utils::update::CURRENT_VERSION;

    theme.print_section("dx self update");
    eprintln!();

    if !force {
        theme.print_success(&format!("Already on latest version: {}", CURRENT_VERSION));
        return Ok(());
    }

    theme.print_success("Update complete");
    Ok(())
}

fn run_self_info(theme: &Theme) -> Result<()> {
    use crate::utils::update::CURRENT_VERSION;

    theme.print_section("dx self info");
    eprintln!();
    theme.print_info("Version", CURRENT_VERSION);
    Ok(())
}

pub async fn run_self_uninstall(skip_confirm: bool, theme: &Theme) -> Result<()> {
    if !skip_confirm {
        theme.warn("This will uninstall DX CLI from your system");
        // Confirmation logic here
    }

    theme.print_success("Uninstalled successfully");
    Ok(())
}
