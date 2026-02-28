//! Model download and management

use anyhow::{Context, Result};
use hf_hub::{Repo, RepoType, api::sync::Api};
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

pub struct ModelManager {
    cache_dir: PathBuf,
    api: Api,
}

impl ModelManager {
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let api = Api::new().context("Failed to initialize HF API")?;

        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir, api })
    }

    /// Download model from Hugging Face
    pub fn download_model(&self, model_id: &str, revision: Option<&str>) -> Result<PathBuf> {
        let repo = Repo::with_revision(
            model_id.to_string(),
            RepoType::Model,
            revision.unwrap_or("main").to_string(),
        );

        let api = self.api.repo(repo);

        // Download required files
        let files = vec!["model.safetensors", "tokenizer.json", "config.json"];

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        let model_dir = self.cache_dir.join(model_id.replace('/', "_"));
        std::fs::create_dir_all(&model_dir)?;

        for file in files {
            pb.set_message(format!("Downloading {}", file));

            let path = api.get(file).context(format!("Failed to download {}", file))?;

            let dest = model_dir.join(file);
            std::fs::copy(path, dest)?;

            pb.inc(1);
        }

        pb.finish_with_message("Download complete");

        Ok(model_dir)
    }

    /// Check if model exists locally
    pub fn model_exists(&self, model_id: &str) -> bool {
        let model_dir = self.cache_dir.join(model_id.replace('/', "_"));
        model_dir.exists() && model_dir.join("model.safetensors").exists()
    }

    /// Get local model path
    pub fn get_model_path(&self, model_id: &str) -> PathBuf {
        self.cache_dir.join(model_id.replace('/', "_"))
    }

    /// List downloaded models
    pub fn list_models(&self) -> Result<Vec<String>> {
        let mut models = Vec::new();

        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    models.push(name.replace('_', "/"));
                }
            }
        }

        Ok(models)
    }
}
