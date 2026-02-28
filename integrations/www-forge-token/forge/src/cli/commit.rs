use std::fs;

use anyhow::{bail, Context, Result};
use chrono::Utc;

use crate::core::manifest::{deserialize_file_entry, serialize_commit, Commit};
use crate::core::repository::Repository;
use crate::db::metadata::MetadataDb;
use crate::util::human::short_hex;

pub fn run(message: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("get current dir")?;
    let repo = Repository::discover(&cwd)?;
    let db = MetadataDb::open(&repo.metadata_db_path())?;

    let staged = db.get_staged_files()?;
    if staged.is_empty() {
        bail!("Nothing staged");
    }

    let mut files = Vec::with_capacity(staged.len());
    for (_path, bytes) in &staged {
        files.push(deserialize_file_entry(bytes)?);
    }

    let mut parents = Vec::new();
    if let Some(parent) = repo.read_head()? {
        parents.push(parent);
    }

    let author = std::env::var("GIT_AUTHOR_NAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());

    let timestamp_ns = Utc::now().timestamp_nanos_opt().unwrap_or_else(|| Utc::now().timestamp() * 1_000_000_000);

    let draft = Commit {
        id: [0u8; 32],
        parents,
        files,
        message: message.to_string(),
        author,
        timestamp_ns,
    };

    let draft_bytes = serialize_commit(&draft)?;
    let commit_id = *blake3::hash(&draft_bytes).as_bytes();

    let commit = Commit {
        id: commit_id,
        ..draft
    };
    let commit_bytes = serialize_commit(&commit)?;
    let id_hex = hex::encode(commit_id);

    let manifest_path = repo.forge_dir.join("manifests").join(&id_hex);
    fs::write(&manifest_path, &commit_bytes)
        .with_context(|| format!("write manifest {}", manifest_path.display()))?;

    db.store_commit(&id_hex, &commit_bytes)?;

    for (path, bytes) in staged {
        db.store_file_entry(&path, &bytes)?;
    }

    repo.update_head(&commit_id)?;
    db.clear_staging()?;

    println!(
        "Committed {} — {} files, message: {}",
        short_hex(&commit_id),
        commit.files.len(),
        message
    );

    Ok(())
}
