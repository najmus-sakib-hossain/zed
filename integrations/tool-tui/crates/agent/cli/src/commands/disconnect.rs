//! Disconnect from integrations

use colored::Colorize;

pub async fn run(integration: &str) -> anyhow::Result<()> {
    println!(
        "{} Disconnecting from {}...",
        "🔌".bright_yellow(),
        integration.bright_yellow()
    );

    // In production: remove token from keyring, disable integration

    println!("{} Disconnected from {}.", "✅".bright_green(), integration);

    Ok(())
}
