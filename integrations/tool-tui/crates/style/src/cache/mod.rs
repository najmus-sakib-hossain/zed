//! Cache module for dx-style
//!
//! Provides caching functionality for CSS class extraction and generation,
//! including snapshot-based persistence and group definition caching.

use ahash::AHashSet;
use std::hash::Hasher;
mod snapshot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};

/// Get the default cache directory path
pub fn default_cache_dir() -> String {
    ".dx/cache".to_string()
}

fn cache_file_path(cache_dir: &str) -> String {
    format!("{}/cache.json", cache_dir)
}

#[derive(Serialize, Deserialize)]
struct CacheDump {
    classes: Vec<String>,
    html_hash: u64,
    groups: Option<GroupDump>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupDefDump {
    pub utilities: Vec<String>,
    pub allow_extend: bool,
    pub raw_tokens: Vec<String>,
    pub dev_tokens: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroupDump {
    pub definitions: BTreeMap<String, GroupDefDump>,
    pub cached_css: BTreeMap<String, String>,
}

#[allow(dead_code)]
pub fn load_cache() -> (AHashSet<String>, u64, u64, Option<GroupDump>) {
    load_cache_from_dir(&default_cache_dir())
}

/// Load cache from a specific directory
pub fn load_cache_from_dir(cache_dir: &str) -> (AHashSet<String>, u64, u64, Option<GroupDump>) {
    if let Some((set, html_hash, checksum)) = snapshot::load_snapshot_from_dir(cache_dir) {
        return (set, html_hash, checksum, None);
    }
    if let Ok(mut f) = File::open(cache_file_path(cache_dir)) {
        let mut buf = String::new();
        if f.read_to_string(&mut buf).is_ok() {
            if let Ok(dump) = serde_json::from_str::<CacheDump>(&buf) {
                let mut h = ahash::AHasher::default();
                for c in &dump.classes {
                    h.write(c.as_bytes());
                }
                return (
                    dump.classes.into_iter().collect(),
                    dump.html_hash,
                    h.finish(),
                    dump.groups,
                );
            }
        }
    }

    (AHashSet::default(), 0, 0, None)
}

pub fn save_cache(
    cache: &AHashSet<String>,
    html_hash: u64,
    groups: Option<&GroupDump>,
) -> Result<(), Box<dyn std::error::Error>> {
    save_cache_to_dir(cache, html_hash, groups, &default_cache_dir())
}

/// Save cache to a specific directory
pub fn save_cache_to_dir(
    cache: &AHashSet<String>,
    html_hash: u64,
    groups: Option<&GroupDump>,
    cache_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let dump = CacheDump {
        classes: cache.iter().cloned().collect(),
        html_hash,
        groups: groups.cloned(),
    };
    let bytes = serde_json::to_string(&dump)?.into_bytes();
    if let Err(e) = fs::create_dir_all(cache_dir) {
        return Err(Box::new(e));
    }
    let mut f = File::create(cache_file_path(cache_dir))?;
    f.write_all(&bytes)?;
    snapshot::save_snapshot_to_dir(cache, html_hash, cache_dir);
    Ok(())
}
