//! Download command implementation.

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::DxMedia;
use crate::cli::args::DownloadArgs;
use crate::error::{DxError, Result};

/// Execute the download command.
pub async fn execute(args: DownloadArgs, quiet: bool) -> Result<()> {
    let dx = DxMedia::new()?;

    // Parse asset ID (format: provider:id)
    let (provider_name, asset_id) = parse_asset_id(&args.asset_id)?;

    if !quiet {
        println!("{} {}:{}", "Looking up".cyan(), provider_name, asset_id);
    }

    // Try to get the asset directly by ID first
    let asset = if let Some(provider) = dx.registry().get(provider_name) {
        if let Ok(Some(asset)) = provider.get_by_id(asset_id).await {
            Some(asset)
        } else {
            None
        }
    } else {
        None
    };

    // If direct lookup failed, fall back to search
    let asset = if let Some(asset) = asset {
        asset
    } else {
        // Search for the asset by ID in the specific provider
        let mut query = crate::types::SearchQuery::new(asset_id);
        query.providers = vec![provider_name.to_string()];
        query.count = 20; // Limit to avoid rate limits on anonymous requests

        let search_result = dx.search_query(&query).await?;

        // Find the asset with matching ID
        search_result
            .assets
            .into_iter()
            .find(|a| a.id == asset_id && a.provider == provider_name)
            .ok_or_else(|| DxError::NoResults {
                query: format!("{}:{}", provider_name, asset_id),
            })?
    };

    // Show progress
    let spinner = if !quiet {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.set_message(format!("Downloading '{}'...", asset.title));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    // Download
    let path = if let Some(ref output_dir) = args.output {
        dx.download_to(&asset, std::path::Path::new(output_dir)).await?
    } else {
        dx.download(&asset).await?
    };

    // Rename if custom filename provided
    if let Some(ref filename) = args.filename {
        let new_path = path.parent().unwrap_or(std::path::Path::new(".")).join(filename);
        tokio::fs::rename(&path, &new_path).await.map_err(|e| DxError::FileIo {
            path: path.clone(),
            message: format!("Failed to rename file: {}", e),
            source: Some(e),
        })?;

        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        if !quiet {
            println!("{} {}", "Downloaded:".green().bold(), new_path.display());
        }
    } else {
        if let Some(pb) = spinner {
            pb.finish_and_clear();
        }

        if !quiet {
            println!("{} {}", "Downloaded:".green().bold(), path.display());
        }
    }

    Ok(())
}

/// Parse asset ID in format "provider:id" or just "id".
fn parse_asset_id(asset_id: &str) -> Result<(&str, &str)> {
    if let Some((provider, id)) = asset_id.split_once(':') {
        Ok((provider, id))
    } else {
        // Default to openverse if no provider specified
        Ok(("openverse", asset_id))
    }
}
