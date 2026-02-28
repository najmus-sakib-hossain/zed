//! Utility functions

use console::style;
use std::path::Path;
use std::time::Duration;

pub fn format_time(duration: Duration) -> String {
    let micros = duration.as_micros();
    if micros < 1000 {
        format!("{}μs", micros)
    } else if micros < 1_000_000 {
        format!("{:.2}ms", micros as f64 / 1000.0)
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} bytes", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn print_build_summary(
    stats: &crate::build::BuildStats,
    total_time: Duration,
    output_dir: &Path,
) {
    println!();
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!("{}", style("Build Summary").green().bold());
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!();
    println!(
        "  {} {} pages compiled → {}",
        style("✓").green(),
        style(stats.pages).cyan().bold(),
        style(format_size(stats.pages_size)).dim()
    );
    println!(
        "  {} {} components compiled → {}",
        style("✓").green(),
        style(stats.components).cyan().bold(),
        style(format_size(stats.components_size)).dim()
    );
    println!(
        "  {} {} assets copied → {}",
        style("✓").green(),
        style(stats.assets).cyan().bold(),
        style(format_size(stats.assets_size)).dim()
    );
    println!(
        "  {} {} styles processed → {}",
        style("✓").green(),
        style(stats.styles).cyan().bold(),
        style(format_size(stats.styles_size)).dim()
    );
    println!();
    println!(
        "  {} {}",
        style("Total size:").dim(),
        style(format_size(stats.total_size())).cyan().bold()
    );
    println!(
        "  {} {}",
        style("Build time:").dim(),
        style(format_time(total_time)).cyan().bold()
    );
    println!("  {} {}", style("Output:").dim(), style(output_dir.display()).cyan());
    println!();
    println!("{}", style("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").dim());
    println!();
    println!("{}", style("✓ Production build ready!").green().bold());
}
