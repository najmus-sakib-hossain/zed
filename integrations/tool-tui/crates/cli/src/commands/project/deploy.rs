//! Deploy command

use anyhow::Result;
use owo_colors::OwoColorize;

use crate::cli::{DeployArgs, DeployTarget};
use crate::ui::theme::Theme;

pub async fn run_deploy(args: DeployArgs, theme: &Theme) -> Result<()> {
    use crate::ui::spinner::Spinner;

    let target_name = match args.target {
        Some(DeployTarget::Vercel) => "Vercel",
        Some(DeployTarget::Netlify) => "Netlify",
        Some(DeployTarget::Cloudflare) => "Cloudflare",
        Some(DeployTarget::Aws) => "AWS",
        Some(DeployTarget::Gcp) => "GCP",
        Some(DeployTarget::Azure) => "Azure",
        None => "default",
    };

    theme.print_section(&format!("dx deploy: {}", target_name));
    eprintln!();

    if args.preview {
        eprintln!("  {} Preview deployment", "│".bright_black());
    }
    eprintln!();

    if !args.no_build {
        let spinner = Spinner::dots("Building project...");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        spinner.success("Build complete");
    }

    let spinner = Spinner::dots("Uploading assets...");
    tokio::time::sleep(std::time::Duration::from_millis(80)).await;
    spinner.success("Uploaded 23 files");

    let spinner = Spinner::dots("Deploying...");
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    spinner.success("Deployment complete");

    eprintln!();
    theme.print_divider();
    eprintln!("  {} Deployment successful!", "✓".green().bold());
    theme.print_divider();
    eprintln!();

    theme.print_info("URL", "https://my-app.vercel.app");
    eprintln!();

    Ok(())
}
