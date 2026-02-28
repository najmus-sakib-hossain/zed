//! Cache management commands
//!
//! Provides commands to manage the layout cache and package store.

use std::path::Path;
use std::sync::Arc;

use dx_py_core::Result;
use dx_py_layout::LayoutCache;
use dx_py_store::PackageStore;

/// Run the cache clean command
pub fn clean(layouts: bool, store: bool, all: bool) -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .map(|p| p.join("dx-py"))
        .unwrap_or_else(|| Path::new(".dx-py-cache").to_path_buf());

    let clean_layouts = all || layouts;
    let clean_store = all || store;

    if !clean_layouts && !clean_store {
        println!("No cache type specified. Use --layouts, --store, or --all");
        return Ok(());
    }

    let mut cleaned_layouts = 0u64;
    let mut cleaned_store = 0u64;

    if clean_layouts {
        let layouts_path = cache_dir.join("layouts");
        if layouts_path.exists() {
            // Count layouts before cleaning
            let store_path = cache_dir.join("store");
            if let Ok(store) = PackageStore::open(&store_path) {
                if let Ok(cache) = LayoutCache::open(&layouts_path, Arc::new(store)) {
                    cleaned_layouts = cache.layout_count() as u64;
                }
            }

            // Remove layouts directory
            std::fs::remove_dir_all(&layouts_path)?;
            println!("✓ Cleared {} cached layouts", cleaned_layouts);
        } else {
            println!("  No layout cache found");
        }
    }

    if clean_store {
        let store_path = cache_dir.join("store");
        if store_path.exists() {
            // Count packages before cleaning
            if let Ok(store) = PackageStore::open(&store_path) {
                if let Ok(hashes) = store.list() {
                    cleaned_store = hashes.len() as u64;
                }
            }

            // Remove store directory
            std::fs::remove_dir_all(&store_path)?;
            println!("✓ Cleared {} cached packages", cleaned_store);
        } else {
            println!("  No package store found");
        }
    }

    if clean_layouts || clean_store {
        println!("\nCache cleaned successfully!");
    }

    Ok(())
}

/// Show cache statistics
pub fn stats() -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .map(|p| p.join("dx-py"))
        .unwrap_or_else(|| Path::new(".dx-py-cache").to_path_buf());

    println!("Cache directory: {}", cache_dir.display());
    println!();

    // Layout cache stats
    let layouts_path = cache_dir.join("layouts");
    let store_path = cache_dir.join("store");

    if store_path.exists() {
        if let Ok(store) = PackageStore::open(&store_path) {
            let store = Arc::new(store);

            // Package store stats
            if let Ok(hashes) = store.list() {
                let mut total_size = 0u64;
                for hash in &hashes {
                    let path = store.get_path(hash);
                    if let Ok(meta) = std::fs::metadata(&path) {
                        total_size += meta.len();
                    }
                }
                println!("Package Store:");
                println!("  Packages: {}", hashes.len());
                println!("  Size: {}", format_size(total_size));
            }

            // Layout cache stats
            if layouts_path.exists() {
                if let Ok(cache) = LayoutCache::open(&layouts_path, store) {
                    let layout_count = cache.layout_count();
                    let mut total_size = 0u64;

                    for entry in cache.iter() {
                        total_size += entry.total_size;
                    }

                    println!("\nLayout Cache:");
                    println!("  Layouts: {}", layout_count);
                    println!("  Size: {}", format_size(total_size));
                }
            } else {
                println!("\nLayout Cache: not initialized");
            }
        }
    } else {
        println!("Cache not initialized. Run 'dx-py install' to populate.");
    }

    Ok(())
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
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
        format!("{} bytes", bytes)
    }
}
