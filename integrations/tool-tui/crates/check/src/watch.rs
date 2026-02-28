//! Watch Mode for Hot-Reload
//!
//! Monitors .sr files and dx config for changes and automatically recompiles.

use crate::rules::compiler;
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

/// Watch mode configuration
pub struct WatchConfig {
    /// Directory containing .sr files
    pub rules_dir: PathBuf,
    /// Output directory for compiled rules
    pub output_dir: PathBuf,
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            rules_dir: PathBuf::from("rules"),
            output_dir: PathBuf::from("rules"),
            debounce_ms: 250,
        }
    }
}

/// Start watch mode
pub fn watch_rules(config: WatchConfig) -> Result<()> {
    println!("👀 Starting watch mode...");
    println!("   Rules dir: {}", config.rules_dir.display());
    println!("   Output dir: {}", config.output_dir.display());
    println!("   Debounce: {}ms\n", config.debounce_ms);

    // Do initial compilation
    println!("🔨 Initial compilation...");
    if let Err(e) = compiler::compile_from_sr(&config.rules_dir, &config.output_dir) {
        eprintln!("❌ Initial compilation failed: {e}");
        return Err(e);
    }
    println!("✅ Initial compilation complete\n");

    // Setup file watcher
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
        if let Ok(event) = res {
            let _ = tx.send(event);
        }
    })?;

    // Watch rules directory
    watcher.watch(&config.rules_dir, RecursiveMode::Recursive)?;
    println!("👁️  Watching for changes... (Press Ctrl+C to stop)\n");

    let debounce_duration = Duration::from_millis(config.debounce_ms);
    let mut last_compile = std::time::Instant::now();

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(event) => {
                // Check if it's a file modification event
                if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    // Check if it's a .sr file or dx config
                    let is_relevant = event.paths.iter().any(|path| {
                        path.extension().and_then(|e| e.to_str()) == Some("sr")
                            || path.file_name().and_then(|n| n.to_str()) == Some("dx")
                    });

                    if is_relevant {
                        // Debounce
                        let now = std::time::Instant::now();
                        if now.duration_since(last_compile) < debounce_duration {
                            continue;
                        }
                        last_compile = now;

                        // Recompile
                        println!("\n🔄 Change detected, recompiling...");
                        match compiler::compile_from_sr(&config.rules_dir, &config.output_dir) {
                            Ok(compiled) => {
                                println!(
                                    "✅ Recompiled {} rules ({} KB)",
                                    compiled.count,
                                    compiled.binary_size / 1024
                                );
                                println!("👁️  Watching for changes...\n");
                            }
                            Err(e) => {
                                eprintln!("❌ Compilation failed: {e}");
                                println!("👁️  Watching for changes...\n");
                            }
                        }
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(e) => {
                eprintln!("Watch error: {e}");
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert_eq!(config.rules_dir, PathBuf::from("rules"));
        assert_eq!(config.output_dir, PathBuf::from("rules"));
        assert_eq!(config.debounce_ms, 250);
    }
}
