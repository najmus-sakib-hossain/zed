use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::db::metadata::MetadataDb;

#[derive(Debug, Clone)]
pub struct Repository {
    pub root: PathBuf,
    pub forge_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chunk_min: u32,
    pub chunk_avg: u32,
    pub chunk_max: u32,
    pub compression_level: i32,
    pub dict_size: usize,
    pub remote_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            chunk_min: 64 * 1024,
            chunk_avg: 256 * 1024,
            chunk_max: 1024 * 1024,
            compression_level: 8,
            dict_size: 112_640,
            remote_url: None,
        }
    }
}

impl Repository {
    pub fn discover(start: &Path) -> Result<Self> {
        let mut current = start
            .canonicalize()
            .with_context(|| format!("unable to canonicalize {}", start.display()))?;

        loop {
            let forge_dir = current.join(".forge");
            if forge_dir.is_dir() {
                return Ok(Self {
                    root: current,
                    forge_dir,
                });
            }
            if !current.pop() {
                bail!("not inside a Forge repository (missing .forge directory)");
            }
        }
    }

    pub fn init(path: &Path) -> Result<Self> {
        fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
        let root = path
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", path.display()))?;
        let forge_dir = root.join(".forge");

        for rel in [
            "objects/chunks",
            "objects/packs",
            "refs/heads",
            "refs/remotes",
            "manifests",
            "dictionaries",
        ] {
            fs::create_dir_all(forge_dir.join(rel))
                .with_context(|| format!("failed to create .forge/{rel}"))?;
        }

        fs::write(forge_dir.join("HEAD"), b"ref: refs/heads/main\n")
            .context("failed to write HEAD")?;

        let config_toml = toml::to_string_pretty(&Config::default()).context("serialize config")?;
        fs::write(forge_dir.join("config.toml"), config_toml).context("write config.toml")?;

        let db_path = forge_dir.join("metadata.redb");
        MetadataDb::create(&db_path)?;

        Ok(Self { root, forge_dir })
    }

    pub fn objects_dir(&self) -> PathBuf {
        self.forge_dir.join("objects")
    }

    pub fn chunk_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex_hash = hex::encode(hash);
        self.forge_dir
            .join("objects/chunks")
            .join(&hex_hash[0..2])
            .join(&hex_hash[2..])
    }

    pub fn metadata_db_path(&self) -> PathBuf {
        self.forge_dir.join("metadata.redb")
    }

    pub fn config_path(&self) -> PathBuf {
        self.forge_dir.join("config.toml")
    }

    pub fn head_path(&self) -> PathBuf {
        self.forge_dir.join("HEAD")
    }

    pub fn read_head(&self) -> Result<Option<[u8; 32]>> {
        let head = fs::read_to_string(self.head_path()).context("failed to read HEAD")?;
        if let Some(reference) = head.strip_prefix("ref: ") {
            let rel = reference.trim();
            let ref_path = self.forge_dir.join(rel);
            if !ref_path.exists() {
                return Ok(None);
            }
            let commit_hex = fs::read_to_string(&ref_path)
                .with_context(|| format!("failed to read ref {}", rel))?
                .trim()
                .to_string();
            if commit_hex.is_empty() {
                return Ok(None);
            }
            let bytes = hex::decode(&commit_hex).context("invalid commit id in ref")?;
            let id = bytes
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid commit id length in ref"))?;
            return Ok(Some(id));
        }

        let commit_hex = head.trim();
        if commit_hex.is_empty() {
            return Ok(None);
        }
        let bytes = hex::decode(commit_hex).context("invalid detached HEAD id")?;
        let id = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid detached HEAD length"))?;
        Ok(Some(id))
    }

    pub fn update_head(&self, commit_id: &[u8; 32]) -> Result<()> {
        let head = fs::read_to_string(self.head_path()).context("failed to read HEAD")?;
        let hex_id = hex::encode(commit_id);
        if let Some(reference) = head.strip_prefix("ref: ") {
            let rel = reference.trim();
            let ref_path = self.forge_dir.join(rel);
            if let Some(parent) = ref_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create parent dirs for ref {}", parent.display())
                })?;
            }
            fs::write(ref_path, format!("{hex_id}\n")).context("failed to update branch ref")?;
        } else {
            fs::write(self.head_path(), format!("{hex_id}\n")).context("failed to update detached HEAD")?;
        }
        Ok(())
    }

    pub fn read_config(&self) -> Result<Config> {
        let raw = fs::read_to_string(self.config_path()).context("failed to read config.toml")?;
        let cfg: Config = toml::from_str(&raw).context("failed to parse config.toml")?;
        Ok(cfg)
    }
}
