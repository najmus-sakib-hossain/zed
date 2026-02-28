use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Context, Result};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ChunkStore {
    pub base_dir: PathBuf,
}

impl ChunkStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn chunk_path(&self, hash: &blake3::Hash) -> PathBuf {
        let hex_hash = hash.to_hex().to_string();
        self.base_dir
            .join(&hex_hash[0..2])
            .join(&hex_hash[2..])
    }

    pub fn contains(&self, hash: &blake3::Hash) -> bool {
        self.chunk_path(hash).exists()
    }

    pub fn store(&self, hash: &blake3::Hash, compressed_data: &[u8]) -> Result<bool> {
        let target = self.chunk_path(hash);
        if target.exists() {
            return Ok(false);
        }

        let parent = target.parent().context("chunk path missing parent")?;
        fs::create_dir_all(parent).with_context(|| format!("create shard dir {}", parent.display()))?;

        let tmp = target.with_extension(format!("tmp.{}", std::process::id()));
        fs::write(&tmp, compressed_data).with_context(|| format!("write temp chunk {}", tmp.display()))?;

        match fs::rename(&tmp, &target) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&tmp);
                Ok(false)
            }
            Err(err) => {
                let _ = fs::remove_file(&tmp);
                Err(err).with_context(|| format!("rename temp chunk to {}", target.display()))
            }
        }
    }

    pub fn read(&self, hash: &blake3::Hash) -> Result<Vec<u8>> {
        let path = self.chunk_path(hash);
        fs::read(&path).with_context(|| format!("read chunk {}", path.display()))
    }

    pub fn remove(&self, hash: &blake3::Hash) -> Result<bool> {
        let path = self.chunk_path(hash);
        match fs::remove_file(&path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err).with_context(|| format!("remove chunk {}", path.display())),
        }
    }

    pub fn list_all(&self) -> Result<Vec<blake3::Hash>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut hashes = Vec::new();
        for entry in WalkDir::new(&self.base_dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let rel = entry
                .path()
                .strip_prefix(&self.base_dir)
                .with_context(|| "strip base dir prefix")?;
            let parts: Vec<_> = rel.iter().collect();
            if parts.len() != 2 {
                continue;
            }
            let shard = parts[0].to_string_lossy();
            let tail = parts[1].to_string_lossy();
            let full = format!("{}{}", shard, tail);
            let Ok(bytes) = hex::decode(&full) else {
                continue;
            };
            let Ok(arr) = <[u8; 32]>::try_from(bytes.as_slice()) else {
                continue;
            };
            hashes.push(blake3::Hash::from(arr));
        }
        Ok(hashes)
    }

    pub fn total_size(&self) -> Result<u64> {
        if !self.base_dir.exists() {
            return Ok(0);
        }

        let mut total = 0u64;
        for entry in WalkDir::new(&self.base_dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            total += entry.metadata().with_context(|| "metadata read failed")?.len();
        }
        Ok(total)
    }

    pub fn chunk_count(&self) -> Result<usize> {
        Ok(self.list_all()?.len())
    }
}
