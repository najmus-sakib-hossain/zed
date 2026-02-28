//! Media search and download commands

use anyhow::Result;
use console::style;
use std::path::Path;

use crate::cli_unified::args::OutputFormat;
use crate::cli_unified::output::{print_info, print_success, print_table_header, print_table_row};
use crate::{DxMedia, MediaType};

pub async fn cmd_search(
    query: &str,
    media_type: &str,
    provider: Option<&str>,
    limit: usize,
    format: &OutputFormat,
) -> Result<()> {
    print_info(&format!("🔍 Searching for {}...", query));

    let dx = DxMedia::new()?;
    let mut search = dx.search(query);

    // Set media type
    let media_type_enum = match media_type.to_lowercase().as_str() {
        "image" | "img" => MediaType::Image,
        "video" | "vid" => MediaType::Video,
        "audio" | "sound" => MediaType::Audio,
        _ => MediaType::Image, // Default to image
    };
    search = search.media_type(media_type_enum);

    // Set provider if specified
    if let Some(p) = provider {
        search = search.provider(p);
    }

    let results = search.execute().await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&results)?);
        }
        OutputFormat::Table => {
            println!("\n{}", style(format!("Found {} assets", results.total_count)).green());
            print_table_header(&["Title", "Provider", "Type", "License"]);

            for asset in results.assets.iter().take(limit) {
                print_table_row(&[
                    asset.title.clone(),
                    asset.provider.clone(),
                    format!("{:?}", asset.media_type),
                    format!("{:?}", asset.license),
                ]);
            }
            println!("{}", "─".repeat(80));
        }
        OutputFormat::Simple => {
            for asset in results.assets.iter().take(limit) {
                println!("{} ({})", asset.title, asset.provider);
            }
        }
    }

    Ok(())
}

pub async fn cmd_download(asset_id: &str, output: &Path, _provider: Option<&str>) -> Result<()> {
    print_info(&format!("📥 Downloading asset: {}", asset_id));

    let _dx = DxMedia::new()?;

    // If it's a URL, download directly
    if asset_id.starts_with("http://") || asset_id.starts_with("https://") {
        // Simple URL download - create temp file
        let client = reqwest::Client::new();
        let response = client.get(asset_id).send().await?;
        let filename = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .unwrap_or("download");
        let filepath = output.join(filename);
        let bytes = response.bytes().await?;
        std::fs::write(&filepath, bytes)?;
        print_success(&format!("Downloaded to: {}", filepath.display()));
        return Ok(());
    }

    // Otherwise, need provider
    let _provider_name = _provider.ok_or_else(|| {
        anyhow::anyhow!("Provider required when using asset ID. Use --provider <name>")
    })?;

    print_info("Download by asset ID not yet implemented");
    print_success("Use a direct URL instead");

    Ok(())
}

pub async fn cmd_providers(provider_type: &str, _format: &OutputFormat) -> Result<()> {
    print_info("📚 Available Providers\n");

    let show_media = provider_type == "all" || provider_type == "media";
    let show_icon = provider_type == "all" || provider_type == "icon";
    let show_font = provider_type == "all" || provider_type == "font";

    if show_media {
        println!("{}", style("Media Providers (FREE - No API Keys)").yellow().bold());
        println!("─────────────────────────────────────────────────────────────────────");
        println!("  • Openverse         700M+ images/audio  https://openverse.org");
        println!("  • Wikimedia         90M+ images         https://commons.wikimedia.org");
        println!("  • NASA Images       140K+ images        https://images.nasa.gov");
        println!("  • Met Museum        470K+ images        https://metmuseum.org");
        println!("  • Rijksmuseum       700K+ images        https://rijksmuseum.nl");
        println!("  • Cleveland Museum  36K+ images         https://clevelandart.org");
        println!("  • Library Congress  25M+ items          https://loc.gov");
        println!("  • DPLA              40M+ items          https://dp.la");
        println!("  • Europeana         50M+ items          https://europeana.eu");
        println!("  • Lorem Picsum      Placeholder images  https://picsum.photos");
        println!("  • Poly Haven        3D assets           https://polyhaven.com");
        println!();
        println!("{}", style("Media Providers (PREMIUM - API Keys)").yellow().bold());
        println!("─────────────────────────────────────────────────────────────────────");
        println!("  • Unsplash          5M+ photos          https://unsplash.com");
        println!("  • Pexels            3.5M+ photos/videos https://pexels.com");
        println!("  • Pixabay           4.2M+ images/videos https://pixabay.com");
        println!("  • Giphy             Millions of GIFs    https://giphy.com");
        println!("  • Freesound         600K+ sounds        https://freesound.org");
        println!("  • Smithsonian       4.5M+ images        https://si.edu");
        println!();
    }

    if show_icon {
        println!("{}", style("Icon Providers").yellow().bold());
        println!("─────────────────────────────────────────────────────────────────────");
        println!("  • 200+ icon packs with 100K+ icons");
        println!("  • Lucide, Solar, Material, FontAwesome, Heroicons, and more");
        println!("  • Use: media icon packs");
        println!();
    }

    if show_font {
        println!("{}", style("Font Providers").yellow().bold());
        println!("─────────────────────────────────────────────────────────────────────");
        println!("  • Google Fonts      1,562 fonts        https://fonts.google.com");
        println!("  • Bunny Fonts       1,478 fonts        https://fonts.bunny.net");
        println!("  • Fontsource        1,562 fonts        https://fontsource.org");
        println!("  • Font Squirrel     1,082 fonts        https://fontsquirrel.com");
        println!("  • DaFont            8,500 fonts        https://dafont.com");
        println!("  • FontShare         100 fonts          https://fontshare.com");
        println!();
    }

    println!("{}", "═".repeat(70));

    Ok(())
}

pub async fn cmd_health() -> Result<()> {
    print_info("🏥 Checking provider health...\n");

    let dx = DxMedia::new()?;
    let health = dx.health_check().await;

    println!("{}", style("Provider Health Status").green().bold());
    println!("{}", "─".repeat(50));

    for result in &health.providers {
        let status = if result.available {
            style("✅ OK").green()
        } else {
            style("❌ Error").red()
        };
        println!("{:<30} {}", result.provider, status);
    }

    println!("{}", "─".repeat(50));
    println!(
        "Healthy: {} / {}",
        health.providers.iter().filter(|r| r.available).count(),
        health.providers.len()
    );

    Ok(())
}
