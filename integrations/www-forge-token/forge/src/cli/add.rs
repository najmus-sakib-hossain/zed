use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use memmap2::Mmap;
use walkdir::WalkDir;

use crate::chunking::cdc::ChunkConfig;
use crate::chunking::chunk_file;
use crate::core::hash::hash_bytes;
use crate::core::manifest::{serialize_file_entry, ChunkRef, FileEntry, FileType};
use crate::core::repository::Repository;
use crate::db::metadata::MetadataDb;
use crate::store::cas::ChunkStore;
use crate::store::compression;
use crate::util::human::human_bytes;
use crate::util::ignore::ForgeIgnore;
use crate::util::progress::create_progress_bar;

fn read_file_bytes(path: &Path) -> Result<Vec<u8>> {
    let metadata = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if metadata.len() > 4 * 1024 * 1024 {
        let file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
        let mapped = unsafe { Mmap::map(&file) }.with_context(|| format!("mmap {}", path.display()))?;
        Ok(mapped.to_vec())
    } else {
        fs::read(path).with_context(|| format!("read {}", path.display()))
    }
}

fn gather_files(paths: &[String], root: &Path, ignore: &ForgeIgnore, force: bool) -> Vec<PathBuf> {
    let mut out = BTreeSet::new();

    for raw in paths {
        let path = Path::new(raw);
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            root.join(path)
        };

        if abs.is_file() {
            if force || !ignore.is_ignored(&abs) {
                out.insert(abs);
            }
            continue;
        }

        if abs.is_dir() {
            for entry in WalkDir::new(&abs)
                .into_iter()
                .filter_map(std::result::Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let file = entry.path();
                if file.components().any(|c| c.as_os_str() == ".forge") {
                    continue;
                }
                if !force && ignore.is_ignored(file) {
                    continue;
                }
                out.insert(file.to_path_buf());
            }
        }
    }

    out.into_iter().collect()
}

fn mtime_ns(meta: &fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

#[cfg(unix)]
fn mode(meta: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode()
}

#[cfg(not(unix))]
fn mode(_meta: &fs::Metadata) -> u32 {
    0
}

pub fn run(paths: &[String], force: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("get current dir")?;
    let repo = Repository::discover(&cwd)?;
    let db = MetadataDb::open(&repo.metadata_db_path())?;
    let store = ChunkStore::new(repo.forge_dir.join("objects/chunks"));
    let config = repo.read_config()?;

    let chunk_cfg = ChunkConfig {
        min_size: config.chunk_min,
        avg_size: config.chunk_avg,
        max_size: config.chunk_max,
    };

    let ignore = ForgeIgnore::load(&repo.root);
    let files = gather_files(paths, &repo.root, &ignore, force);
    let total_bytes: u64 = files
        .iter()
        .filter_map(|p| fs::metadata(p).ok().map(|m| m.len()))
        .sum();

    let bar = create_progress_bar(total_bytes);

    let mut staged_files = 0usize;
    let mut new_chunks = 0usize;
    let mut deduped_chunks = 0usize;
    let mut new_chunk_bytes = 0u64;
    let mut dedup_saved_bytes = 0u64;

    for file in files {
        let metadata = fs::metadata(&file).with_context(|| format!("stat {}", file.display()))?;
        let bytes = read_file_bytes(&file)?;
        let header_len = bytes.len().min(128);
        let file_type = FileType::detect(&file, &bytes[..header_len]);
        let chunks = chunk_file(&bytes, file_type, &chunk_cfg);

        let mut refs = Vec::with_capacity(chunks.len());
        for ch in chunks {
            let slice = &bytes[ch.offset..ch.offset + ch.length];
            let hash_arr = *ch.hash.as_bytes();
            let mut compressed_length = ch.length as u32;

            if !db.is_chunk_known(&hash_arr)? {
                let compressed = compression::compress(slice, config.compression_level)?;
                compressed_length = compressed.len() as u32;
                let _ = store.store(&ch.hash, &compressed)?;
                db.insert_chunk(&hash_arr)?;
                new_chunks += 1;
                new_chunk_bytes += ch.length as u64;
            } else {
                deduped_chunks += 1;
                dedup_saved_bytes += ch.length as u64;
            }

            refs.push(ChunkRef {
                hash: hash_arr,
                offset: ch.offset as u64,
                length: ch.length as u32,
                compressed_length,
            });
        }

        let rel_path = pathdiff::diff_paths(&file, &repo.root).unwrap_or_else(|| file.clone());
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        let entry = FileEntry {
            path: rel.clone(),
            size: metadata.len(),
            file_hash: *hash_bytes(&bytes).as_bytes(),
            chunks: refs,
            mode: mode(&metadata),
            mtime_ns: mtime_ns(&metadata),
            file_type,
        };

        let entry_bytes = serialize_file_entry(&entry)?;
        db.stage_file(&rel, &entry_bytes)?;
        staged_files += 1;
        bar.inc(metadata.len());
    }

    bar.finish_and_clear();
    println!(
        "Staged {} files, {} new chunks ({}), {} deduped chunks ({} saved)",
        staged_files,
        new_chunks,
        human_bytes(new_chunk_bytes),
        deduped_chunks,
        human_bytes(dedup_saved_bytes)
    );
    Ok(())
}
