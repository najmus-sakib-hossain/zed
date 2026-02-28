//! Self-update command implementation

use anyhow::Result;
use console::style;

use crate::utils::update::{UpdateApplier, UpdateChecker, UpdateDownloader};

/// Execute self update command
pub async fn execute_update(force: bool, yes: bool) -> Result<()> {
    println!("\n  {} Checking for updates...\n", style("[*]").cyan().bold());

    let checker = UpdateChecker::new();
    let current = checker.current_version();

    let update_info = match checker.check().await? {
        Some(info) => info,
        None => {
            println!(
                "  {} Already on latest version: {}\n",
                style("[✓]").green().bold(),
                style(current).cyan()
            );
            return Ok(());
        }
    };

    if !force && update_info.current_version == update_info.new_version {
        println!(
            "  {} Already on latest version: {}\n",
            style("[✓]").green().bold(),
            style(current).cyan()
        );
        return Ok(());
    }

    // Display update info
    println!("  {} Update available!", style("[!]").yellow().bold());
    println!(
        "    {} {}",
        style("Current:").dim(),
        style(&update_info.current_version).yellow()
    );
    println!("    {} {}", style("New:").dim(), style(&update_info.new_version).green().bold());
    println!("    {} {}", style("Size:").dim(), format_bytes(update_info.preferred_size()));

    if update_info.has_delta() {
        println!("    {} Delta patch available", style("Note:").dim());
    }

    if !update_info.release_notes.is_empty() {
        println!("\n  {} Release Notes:", style("[i]").cyan());
        println!("    {}", style(&update_info.release_notes).dim());
    }
    println!();

    // Confirm update
    if !yes {
        let confirm = crate::confirm("Apply update?").interact()?;
        if !confirm {
            println!("\n  {} Update cancelled\n", style("[X]").red());
            return Ok(());
        }
    }

    // Download update
    println!("  {} Downloading update...", style("[*]").cyan().bold());
    let downloader = UpdateDownloader::new()?;
    let binary = downloader.download(&update_info)?;
    let signature = downloader.download_signature(&update_info.signature)?;

    // TODO: Load public key from embedded or config
    let public_key = vec![0u8; 32]; // Placeholder

    // Apply update
    println!("  {} Applying update...", style("[*]").cyan().bold());
    let applier = UpdateApplier::for_current_exe()?;
    applier.apply_update(&binary, &signature, &public_key)?;

    println!(
        "\n  {} Update successful! Restart dx to use the new version.\n",
        style("[✓]").green().bold()
    );

    Ok(())
}

/// Execute self info command
pub async fn execute_info() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let target = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    println!("\n  ╔══════════════════════════════════════════════════════════════╗");
    println!(
        "  ║   {} DX CLI Information                                    ║",
        style("[i]").cyan().bold()
    );
    println!("  ╠══════════════════════════════════════════════════════════════╣");
    println!(
        "  ║   {} Version:        {}                                  ║",
        style("→").dim(),
        style(version).cyan()
    );
    println!(
        "  ║   {} Platform:       {}-{}                           ║",
        style("→").dim(),
        style(target).cyan(),
        style(arch).cyan()
    );

    if let Ok(exe) = std::env::current_exe() {
        println!("  ║   {} Binary:         {}  ║", style("→").dim(), style(exe.display()).dim());
    }

    println!("  ╚══════════════════════════════════════════════════════════════╝");
    println!();

    Ok(())
}

/// Execute self uninstall command
pub async fn execute_uninstall(yes: bool) -> Result<()> {
    if !yes {
        println!(
            "\n  {} This will remove the dx CLI from your system.",
            style("[!]").yellow().bold()
        );
        let confirm = crate::confirm("Are you sure?").interact()?;
        if !confirm {
            println!("\n  {} Uninstall cancelled\n", style("[X]").red());
            return Ok(());
        }
    }

    println!("\n  {} Uninstalling dx CLI...", style("[*]").cyan().bold());

    // Remove binary
    if let Ok(exe) = std::env::current_exe() {
        std::fs::remove_file(&exe)?;
        println!("  {} Removed binary: {}", style("[✓]").green(), style(exe.display()).dim());
    }

    // Remove data directory
    if let Some(data_dir) = dirs::data_local_dir() {
        let dx_dir = data_dir.join("dx");
        if dx_dir.exists() {
            std::fs::remove_dir_all(&dx_dir)?;
            println!("  {} Removed data: {}", style("[✓]").green(), style(dx_dir.display()).dim());
        }
    }

    println!("\n  {} DX CLI uninstalled successfully\n", style("[✓]").green().bold());

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
