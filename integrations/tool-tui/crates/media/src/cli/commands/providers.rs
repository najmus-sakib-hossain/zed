//! Providers command implementation.

use colored::Colorize;

use crate::DxMedia;
use crate::cli::args::{OutputFormat, ProvidersArgs};
use crate::error::Result;

/// Execute the providers command.
pub async fn execute(args: ProvidersArgs, format: OutputFormat) -> Result<()> {
    let dx = DxMedia::new()?;
    let registry = dx.registry();

    let providers = if args.available {
        registry.available()
    } else {
        registry.all()
    };

    match format {
        OutputFormat::Json | OutputFormat::JsonCompact => {
            let json: Vec<serde_json::Value> = providers
                .iter()
                .map(|p| {
                    let mut obj = serde_json::json!({
                        "name": p.name(),
                        "display_name": p.display_name(),
                        "available": p.is_available(),
                        "requires_api_key": p.requires_api_key(),
                        "supported_types": p.supported_media_types()
                            .iter()
                            .map(|t| t.as_str())
                            .collect::<Vec<_>>(),
                    });

                    if args.detailed {
                        // Try to get extended info if available
                        // We can't easily downcast, so we'll use the trait methods we have
                        obj["base_url"] = serde_json::json!(p.base_url());
                        obj["rate_limit"] = serde_json::json!({
                            "requests_per_window": p.rate_limit().requests_per_window(),
                            "window_secs": p.rate_limit().window_secs(),
                        });
                    }

                    obj
                })
                .collect();

            if matches!(format, OutputFormat::JsonCompact) {
                println!("{}", serde_json::to_string(&json)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&json)?);
            }
        }
        OutputFormat::Tsv => {
            println!("name\tdisplay_name\tavailable\trequires_api_key");
            for p in &providers {
                println!(
                    "{}\t{}\t{}\t{}",
                    p.name(),
                    p.display_name(),
                    p.is_available(),
                    p.requires_api_key()
                );
            }
        }
        OutputFormat::Text => {
            let stats = registry.stats();

            println!("{}", "Available Providers".bold().cyan());
            println!(
                "{} {} total, {} available, {} need API keys",
                "Stats:".dimmed(),
                stats.total,
                stats.available.to_string().green(),
                stats.unavailable.to_string().yellow()
            );
            println!();

            for p in &providers {
                let status = if p.is_available() {
                    "✓".green()
                } else {
                    "✗".red()
                };

                let types: Vec<&str> =
                    p.supported_media_types().iter().map(|t| t.as_str()).collect();

                println!(
                    "  {} {} {}",
                    status,
                    p.display_name().bold(),
                    format!("({})", p.name()).dimmed()
                );
                println!("      {} {}", "Types:".dimmed(), types.join(", "));

                if args.detailed {
                    let rate = p.rate_limit();
                    if rate.is_limited() {
                        println!(
                            "      {} {}/{}s",
                            "Rate:".dimmed(),
                            rate.requests_per_window(),
                            rate.window_secs()
                        );
                    } else {
                        println!("      {} unlimited", "Rate:".dimmed());
                    }

                    if p.requires_api_key() && !p.is_available() {
                        println!("      {} {}", "Note:".yellow(), "API key required".yellow());
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}
