//! Search command implementation.

use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};

use crate::DxMedia;
use crate::cli::OutputFormatter;
use crate::cli::args::{OutputFormat, SearchArgs};
use crate::error::Result;
use crate::types::SearchQuery;

/// Execute the search command.
pub async fn execute(args: SearchArgs, format: OutputFormat, quiet: bool) -> Result<()> {
    let dx = DxMedia::new()?;

    // Show progress indicator
    let spinner = if !quiet && matches!(format, OutputFormat::Text) {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        let search_type = if args.all {
            "all providers & scrapers"
        } else {
            "providers"
        };
        let mode_str = match args.mode {
            crate::cli::args::SearchModeArg::Quantity => "⚡ quantity",
            crate::cli::args::SearchModeArg::Quality => "🎯 quality",
        };
        pb.set_message(format!(
            "Searching {} for '{}' ({} mode)...",
            search_type,
            args.query_string(),
            mode_str
        ));
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        Some(pb)
    } else {
        None
    };

    // Convert CLI mode to types::SearchMode
    let search_mode: crate::types::SearchMode = args.mode.into();

    // Execute search - use unified search if --all is specified
    let result = if args.all {
        dx.search_all_with_mode(&args.query_string(), args.count, search_mode).await?
    } else {
        // Build the search query for regular search
        let mut query = SearchQuery::new(args.query_string());
        query.count = args.count;
        query.page = args.page;
        query.media_type = args.media_type.and_then(Into::into);
        query.providers = args.providers.clone();
        query.orientation = args.orientation.map(Into::into);
        query.color = args.color.clone();
        query.mode = search_mode;

        dx.search_query(&query).await?
    };

    // Clear spinner
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Format and display results
    let formatter = OutputFormatter::new(format, quiet);
    formatter.format_search_results(&result)?;

    // Auto-download if requested
    if args.download && !result.assets.is_empty() {
        if !quiet {
            println!();
            println!("{}", "Downloading first result...".cyan());
        }

        let asset = &result.assets[0];
        let path = if let Some(ref output_dir) = args.output {
            dx.download_to(asset, std::path::Path::new(output_dir)).await?
        } else {
            dx.download(asset).await?
        };

        if !quiet {
            println!("{} {}", "Downloaded:".green(), path.display());
        }
    }

    Ok(())
}
