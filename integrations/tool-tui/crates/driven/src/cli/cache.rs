//! Cache Command
//!
//! Manage binary cache and fusion templates.

use crate::Result;
use crate::fusion::BinaryCache;
use console::style;
use std::path::Path;

/// Cache command for managing binary caches
#[derive(Debug)]
pub struct CacheCommand;

/// Extended cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    /// Number of entries
    pub entries: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Oldest entry timestamp
    pub oldest_entry: Option<std::time::SystemTime>,
}

impl CacheCommand {
    /// Show cache status
    pub fn status(cache_dir: &Path) -> Result<()> {
        println!("{} Cache Status", style("📦").bold());
        println!();

        let cache = BinaryCache::open(cache_dir)?;
        let internal_stats = cache.stats();
        let extended = Self::compute_extended_stats(cache_dir)?;

        println!("  Directory: {}", cache_dir.display());
        println!("  Entries: {}", internal_stats.entries);
        println!(
            "  Size: {} bytes / {} max",
            internal_stats.size_bytes, internal_stats.max_size_bytes
        );

        if extended.oldest_entry.is_some() {
            println!(
                "  Oldest: {:?}",
                extended.oldest_entry.map(|t| t.elapsed().unwrap_or_default())
            );
        }

        Ok(())
    }

    /// Compute extended statistics
    fn compute_extended_stats(cache_dir: &Path) -> Result<CacheStats> {
        let mut stats = CacheStats::default();

        if !cache_dir.exists() {
            return Ok(stats);
        }

        for entry in std::fs::read_dir(cache_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                stats.entries += 1;
                stats.total_size += metadata.len() as usize;

                let modified = metadata.modified().ok();
                if stats.oldest_entry.is_none() || modified < stats.oldest_entry {
                    stats.oldest_entry = modified;
                }
            }
        }

        Ok(stats)
    }

    /// Clear the cache
    pub fn clear(cache_dir: &Path) -> Result<()> {
        println!("{} Clearing cache at {}", style("🗑").bold(), cache_dir.display());

        let removed = Self::clear_cache_dir(cache_dir)?;

        println!("{} Removed {} cache entries", style("✓").green().bold(), removed);

        Ok(())
    }

    /// Clear all cache entries in directory
    fn clear_cache_dir(cache_dir: &Path) -> Result<usize> {
        let mut removed = 0;

        if cache_dir.exists() {
            for entry in std::fs::read_dir(cache_dir)? {
                let entry = entry?;
                if entry.metadata()?.is_file() {
                    std::fs::remove_file(entry.path())?;
                    removed += 1;
                }
            }
        }

        Ok(removed)
    }

    /// Prune old entries
    pub fn prune(cache_dir: &Path, max_age_secs: u64) -> Result<()> {
        println!(
            "{} Pruning cache entries older than {} seconds",
            style("🧹").bold(),
            max_age_secs
        );

        let removed =
            Self::prune_cache_dir(cache_dir, std::time::Duration::from_secs(max_age_secs))?;

        println!("{} Removed {} old entries", style("✓").green().bold(), removed);

        Ok(())
    }

    /// Prune entries older than max_age
    fn prune_cache_dir(cache_dir: &Path, max_age: std::time::Duration) -> Result<usize> {
        let mut removed = 0;
        let now = std::time::SystemTime::now();

        if cache_dir.exists() {
            for entry in std::fs::read_dir(cache_dir)? {
                let entry = entry?;
                if entry.metadata()?.is_file() {
                    if let Ok(modified) = entry.metadata()?.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age > max_age {
                                std::fs::remove_file(entry.path())?;
                                removed += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(removed)
    }

    /// Warm the cache with templates from source directory
    pub fn warm(cache_dir: &Path, source_dir: &Path) -> Result<()> {
        println!("{} Warming cache from {}", style("🔥").bold(), source_dir.display());

        // Just scan and report for now
        let mut file_count = 0;

        for entry in walkdir::WalkDir::new(source_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "drv" || ext == "md") {
                file_count += 1;
            }
        }

        println!("{} Found {} template files in source", style("✓").green().bold(), file_count);
        println!("  Cache directory: {}", cache_dir.display());

        Ok(())
    }

    /// Show cache entries
    pub fn list(cache_dir: &Path) -> Result<()> {
        println!("{} Cache Entries", style("📋").bold());
        println!();

        let entries = Self::list_cache_entries(cache_dir)?;

        if entries.is_empty() {
            println!("  No cache entries found.");
        } else {
            for (key, size, age) in entries {
                let age_str = if age.as_secs() < 60 {
                    format!("{}s ago", age.as_secs())
                } else if age.as_secs() < 3600 {
                    format!("{}m ago", age.as_secs() / 60)
                } else {
                    format!("{}h ago", age.as_secs() / 3600)
                };

                println!("  {} ({} bytes, {})", key, size, age_str);
            }
        }

        Ok(())
    }

    /// List cache entries
    fn list_cache_entries(cache_dir: &Path) -> Result<Vec<(String, usize, std::time::Duration)>> {
        let mut entries = Vec::new();
        let now = std::time::SystemTime::now();

        if cache_dir.exists() {
            for entry in std::fs::read_dir(cache_dir)? {
                let entry = entry?;
                let metadata = entry.metadata()?;

                if metadata.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let size = metadata.len() as usize;
                    let age = metadata
                        .modified()
                        .ok()
                        .and_then(|m| now.duration_since(m).ok())
                        .unwrap_or_default();

                    entries.push((name, size, age));
                }
            }
        }

        entries.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(entries)
    }
}
