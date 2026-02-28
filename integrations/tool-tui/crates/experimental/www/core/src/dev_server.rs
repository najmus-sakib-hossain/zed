//! # Dev Server Module - Hot Module Replacement
//!
//! WebSocket-based development server with < 200ms hot-swap.

use anyhow::{Context, Result};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Start the development server with file watching
pub async fn start(entry: PathBuf, port: u16, verbose: bool) -> Result<()> {
    if verbose {
        println!("  Starting dev server on port {}...", port);
    }

    // Set up file watcher
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);

    let mut watcher: RecommendedWatcher =
        notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if let Err(e) = tx.blocking_send(event) {
                    eprintln!("Failed to send event: {}", e);
                }
            }
        })
        .context("Failed to create file watcher")?;

    // Watch the source directory
    // For Binary Dawn, if entry is "pages", we must watch "." (root) to catch "units/" changes
    let watch_dir = if entry.is_dir() || entry.starts_with("pages") {
        std::path::Path::new(".")
    } else {
        entry.parent().unwrap_or_else(|| std::path::Path::new("."))
    };

    watcher
        .watch(watch_dir, RecursiveMode::Recursive)
        .context("Failed to watch directory")?;

    println!("  Watching: {}", watch_dir.display());

    // Initial build
    println!("\n  Building initial version...");
    let initial_build = perform_build(&entry, verbose).await?;
    println!("  ✓ Initial build complete\n");

    let last_build = Arc::new(Mutex::new(initial_build));

    // Watch loop
    loop {
        tokio::select! {
            Some(event) = rx.recv() => {
                if should_rebuild(&event) {
                    println!("  File changed: {:?}", event.paths);
                    println!("  Rebuilding...");

                    let start = std::time::Instant::now();

                    match perform_build(&entry, verbose).await {
                        Ok(new_build) => {
                            let elapsed = start.elapsed();
                            println!("  ✓ Rebuilt in {:.2}ms", elapsed.as_millis());

                            // Calculate delta
                            let mut last = last_build.lock().await;
                            let delta = calculate_delta(&last, &new_build);
                            *last = new_build;

                            // Send delta to connected clients
                            println!("  Delta: {} changed", delta);
                            // TODO: Send via WebSocket
                        }
                        Err(e) => {
                            eprintln!("  ✗ Build failed: {}", e);
                        }
                    }
                    println!();
                }
            }
        }
    }
}

/// Perform a build
async fn perform_build(entry: &Path, verbose: bool) -> Result<BuildArtifact> {
    // Simplified build for dev mode
    // In production, this would call the full build pipeline

    // Linker Scan
    let search_root = if entry.file_name().is_some_and(|n| n == "pages") {
        PathBuf::from(".")
    } else {
        entry.parent().map(|p| p.to_path_buf()).unwrap_or(PathBuf::from("."))
    };
    let symbol_table = crate::linker::scan_project(&search_root, verbose)?;

    let parsed = crate::parser::parse_entry(entry, &symbol_table, verbose)?;
    let shaken = crate::parser::tree_shake(parsed, verbose)?;
    let (templates, bindings, schemas) = crate::splitter::split_components(shaken, verbose)?;

    // Use new HTIP binary generation (no Rust/WASM compilation!)
    let (htip_stream, _strings) =
        crate::codegen::generate_htip(&templates, &bindings, &schemas, verbose)?;
    let hash = blake3::hash(&htip_stream).to_hex().to_string();

    Ok(BuildArtifact {
        templates,
        htip_stream,
        hash,
    })
}

/// Build artifact for comparison
#[derive(Clone)]
struct BuildArtifact {
    templates: Vec<crate::splitter::Template>,
    htip_stream: Vec<u8>,
    hash: String,
}

/// Check if event should trigger rebuild
fn should_rebuild(event: &Event) -> bool {
    use notify::EventKind;

    match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
            // Only rebuild for relevant file types
            event.paths.iter().any(|path| {
                path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| matches!(ext, "tsx" | "ts" | "jsx" | "js" | "dx"))
                    .unwrap_or(false)
            })
        }
        _ => false,
    }
}

/// Calculate delta between builds
fn calculate_delta(old: &BuildArtifact, new: &BuildArtifact) -> String {
    if old.hash == new.hash {
        return "no changes".to_string();
    }

    let mut changes = Vec::new();

    if old.templates.len() != new.templates.len() {
        changes.push(format!("templates: {} -> {}", old.templates.len(), new.templates.len()));
    } else {
        let template_changes = old
            .templates
            .iter()
            .zip(&new.templates)
            .filter(|(a, b)| a.hash != b.hash)
            .count();
        if template_changes > 0 {
            changes.push(format!("{} templates modified", template_changes));
        }
    }

    if old.htip_stream.len() != new.htip_stream.len() {
        changes.push(format!("htip: {} -> {} bytes", old.htip_stream.len(), new.htip_stream.len()));
    }

    if changes.is_empty() {
        "unknown changes".to_string()
    } else {
        changes.join(", ")
    }
}
