use std::collections::{BTreeMap, BTreeSet};
use std::fs;

use anyhow::{Context, Result};

use crate::core::hash::hash_file;
use crate::core::manifest::{deserialize_commit, deserialize_file_entry, FileEntry};
use crate::core::repository::Repository;
use crate::db::metadata::MetadataDb;

fn load_commit_files(repo: &Repository, hex_id: &str) -> Result<BTreeMap<String, FileEntry>> {
    let path = repo.forge_dir.join("manifests").join(hex_id);
    let bytes = fs::read(&path).with_context(|| format!("read manifest {}", path.display()))?;
    let commit = deserialize_commit(&bytes)?;
    Ok(commit
        .files
        .into_iter()
        .map(|f| (f.path.clone(), f))
        .collect::<BTreeMap<_, _>>())
}

fn print_diff(old_map: &BTreeMap<String, FileEntry>, new_map: &BTreeMap<String, FileEntry>, path_filter: Option<&str>) {
    let mut paths = BTreeSet::new();
    paths.extend(old_map.keys().cloned());
    paths.extend(new_map.keys().cloned());

    for path in paths {
        if let Some(filter) = path_filter {
            if path != filter {
                continue;
            }
        }

        match (old_map.get(&path), new_map.get(&path)) {
            (None, Some(new_entry)) => {
                println!("A {} (0 -> {} bytes)", path, new_entry.size);
            }
            (Some(old_entry), None) => {
                println!("R {} ({} -> 0 bytes)", path, old_entry.size);
            }
            (Some(old_entry), Some(new_entry)) => {
                if old_entry.file_hash != new_entry.file_hash {
                    let old_set: BTreeSet<[u8; 32]> = old_entry.chunks.iter().map(|c| c.hash).collect();
                    let new_set: BTreeSet<[u8; 32]> = new_entry.chunks.iter().map(|c| c.hash).collect();
                    let reused = old_set.intersection(&new_set).count();
                    let changed = new_set.len().saturating_sub(reused);
                    let reuse_pct = if new_set.is_empty() {
                        0.0
                    } else {
                        (reused as f64 / new_set.len() as f64) * 100.0
                    };
                    println!(
                        "M {} ({} -> {} bytes, {} chunks changed, {:.1}% reused)",
                        path,
                        old_entry.size,
                        new_entry.size,
                        changed,
                        reuse_pct
                    );
                }
            }
            (None, None) => {}
        }
    }
}

pub fn run(path: Option<&str>, commit1: Option<&str>, commit2: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("get current dir")?;
    let repo = Repository::discover(&cwd)?;

    if let (Some(c1), Some(c2)) = (commit1, commit2) {
        let old_map = load_commit_files(&repo, c1)?;
        let new_map = load_commit_files(&repo, c2)?;
        print_diff(&old_map, &new_map, path);
        return Ok(());
    }

    let db = MetadataDb::open(&repo.metadata_db_path())?;

    let head_map = if let Some(head) = repo.read_head()? {
        load_commit_files(&repo, &hex::encode(head))?
    } else {
        BTreeMap::new()
    };

    let mut current_map = head_map.clone();

    for (staged_path, bytes) in db.get_staged_files()? {
        let entry = deserialize_file_entry(&bytes)?;
        current_map.insert(staged_path, entry);
    }

    for (tracked_path, bytes) in db.get_all_tracked_files()? {
        if current_map.contains_key(&tracked_path) {
            continue;
        }
        let mut entry = deserialize_file_entry(&bytes)?;
        let abs = repo.root.join(&tracked_path);
        if abs.exists() {
            let meta = fs::metadata(&abs)?;
            if meta.len() != entry.size {
                entry.size = meta.len();
                entry.file_hash = *hash_file(&abs)?.as_bytes();
            }
            current_map.insert(tracked_path, entry);
        }
    }

    print_diff(&head_map, &current_map, path);
    Ok(())
}
