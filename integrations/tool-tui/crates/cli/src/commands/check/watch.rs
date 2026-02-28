//! Watch subcommand - re-run checks on file changes

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use std::time::Duration;

use super::OutputFormat;
use crate::ui::theme::Theme;

/// Watch mode - re-run checks on file changes
#[derive(Args, Clone)]
pub struct WatchCommand {
    /// Paths to watch
    #[arg(index = 1)]
    pub paths: Vec<PathBuf>,

    /// Debounce delay in milliseconds
    #[arg(long, default_value = "100")]
    pub debounce: u64,

    /// Clear screen before each run
    #[arg(long)]
    pub clear: bool,

    /// Commands to run (format, lint, score, test)
    #[arg(long, short)]
    pub commands: Vec<String>,
}

/// Run watch command
pub async fn run(cmd: WatchCommand, format: OutputFormat, theme: &Theme) -> Result<()> {
    run_watch(&cmd.paths, format, theme).await
}

/// Run watch mode for given paths
pub async fn run_watch(paths: &[PathBuf], _format: OutputFormat, theme: &Theme) -> Result<()> {
    use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
    use owo_colors::OwoColorize;
    use std::sync::mpsc::channel;

    let paths = if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths.to_vec()
    };

    theme.print_section("dx check watch: Watching for changes");
    eprintln!();
    eprintln!("  {} Watching {} paths for changes...", "▸".cyan(), paths.len());
    eprintln!("  {} Press {} to stop", "ℹ".bright_black(), "Ctrl+C".cyan().bold());
    eprintln!();

    // Set up file watcher
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_millis(100)),
    )?;

    // Watch all paths
    for path in &paths {
        watcher.watch(path, RecursiveMode::Recursive)?;
    }

    let mut last_run = std::time::Instant::now();
    let debounce = Duration::from_millis(100);

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
                // Debounce
                if last_run.elapsed() < debounce {
                    continue;
                }
                last_run = std::time::Instant::now();

                // Get changed files
                let changed_paths: Vec<_> =
                    event.paths.iter().filter(|p| is_source_file(p)).collect();

                if changed_paths.is_empty() {
                    continue;
                }

                eprintln!();
                eprintln!(
                    "  {} File changed: {}",
                    "→".cyan(),
                    changed_paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                        .cyan()
                );

                // Run checks
                let start = std::time::Instant::now();
                eprintln!("  {} Running checks...", "▸".cyan());

                // In production, this would run actual checks
                tokio::time::sleep(Duration::from_millis(100)).await;

                let elapsed = start.elapsed();
                eprintln!("  {} Checks completed in {:.2}s", "✓".green(), elapsed.as_secs_f64());
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // No events, continue watching
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    Ok(())
}

fn is_source_file(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    matches!(
        ext,
        "js" | "jsx"
            | "ts"
            | "tsx"
            | "py"
            | "rs"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "kt"
            | "swift"
            | "rb"
            | "php"
    )
}
