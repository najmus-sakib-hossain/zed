use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use anyhow::{Context, Result};
use walkdir::WalkDir;

use crate::core::hash::hash_file;
use crate::core::manifest::deserialize_file_entry;
use crate::core::repository::Repository;
use crate::db::metadata::MetadataDb;
use crate::util::human::short_hex;
use crate::util::ignore::ForgeIgnore;

fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

pub fn run() -> Result<()> {
    let cwd = std::env::current_dir().context("get current dir")?;
    let repo = Repository::discover(&cwd)?;
    let db = MetadataDb::open(&repo.metadata_db_path())?;

    let head_raw = fs::read_to_string(repo.head_path()).unwrap_or_default();
    let branch = head_raw
        .strip_prefix("ref: ")
        .map(|r| r.trim().split('/').next_back().unwrap_or("detached").to_string())
        .unwrap_or_else(|| "detached".to_string());
    println!("On branch: {}", branch);

    match repo.read_head()? {
        Some(id) => println!("HEAD: {}", short_hex(&id)),
        None => println!("HEAD: <none>"),
    }

    let staged = db.get_staged_files()?;
    if !staged.is_empty() {
        println!("\nStaged files:");
        for (path, _) in &staged {
            println!("+ {}", path);
        }
    }

    let mut tracked = BTreeMap::new();
    for (path, bytes) in db.get_all_tracked_files()? {
        let entry = deserialize_file_entry(&bytes)?;
        tracked.insert(path, entry);
    }

    let ignore = ForgeIgnore::load(&repo.root);
    let mut working = BTreeMap::new();

    for entry in WalkDir::new(&repo.root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if path.components().any(|c| c.as_os_str() == ".forge") {
            continue;
        }
        if ignore.is_ignored(path) {
            continue;
        }

        let rel = pathdiff::diff_paths(path, &repo.root)
            .unwrap_or_else(|| path.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/");

        let meta = entry.metadata().context("read file metadata")?;
        working.insert(rel, (path.to_path_buf(), meta));
    }

    let mut untracked = BTreeSet::new();
    let mut modified = BTreeSet::new();
    let mut deleted = BTreeSet::new();

    for (path, (_abs, _meta)) in &working {
        if !tracked.contains_key(path) {
            untracked.insert(path.clone());
        }
    }

    for (path, entry) in &tracked {
        let Some((abs, meta)) = working.get(path) else {
            deleted.insert(path.clone());
            continue;
        };

        let size_differs = meta.len() != entry.size;
        if size_differs {
            modified.insert(path.clone());
            continue;
        }

        let mtime = mtime_ns(meta);
        if mtime != entry.mtime_ns {
            let hash = hash_file(abs)?;
            if hash.as_bytes() != &entry.file_hash {
                modified.insert(path.clone());
            }
        }
    }

    if !modified.is_empty() || !deleted.is_empty() || !untracked.is_empty() {
        println!("\nWorking tree changes:");
    }

    for path in modified {
        println!("M modified {}", path);
    }
    for path in deleted {
        println!("D deleted {}", path);
    }
    for path in untracked {
        println!("? untracked {}", path);
    }

    Ok(())
}
