use std::fs;
use std::io::Write;

use anyhow::{Context, Result};

use crate::core::manifest::deserialize_commit;
use crate::core::repository::Repository;
use crate::store::cas::ChunkStore;
use crate::store::compression;

pub fn run(commit_id_hex: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("get current dir")?;
    let repo = Repository::discover(&cwd)?;
    let store = ChunkStore::new(repo.forge_dir.join("objects/chunks"));

    let manifest_path = repo.forge_dir.join("manifests").join(commit_id_hex);
    let bytes = fs::read(&manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    let commit = deserialize_commit(&bytes)?;

    for entry in &commit.files {
        let out_path = repo.root.join(&entry.path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create parent dirs {}", parent.display()))?;
        }

        let mut file = fs::File::create(&out_path)
            .with_context(|| format!("create output file {}", out_path.display()))?;

        for chunk in &entry.chunks {
            let hash = blake3::Hash::from(chunk.hash);
            let compressed = store.read(&hash)?;
            let raw = compression::decompress(&compressed)?;
            file.write_all(&raw)
                .with_context(|| format!("write data to {}", out_path.display()))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&out_path, fs::Permissions::from_mode(entry.mode))
                .with_context(|| format!("set mode on {}", out_path.display()))?;
        }
    }

    fs::write(repo.head_path(), format!("{}\n", commit_id_hex)).context("write detached HEAD")?;
    println!(
        "Checked out {} ({} files)",
        &commit_id_hex[..commit_id_hex.len().min(12)],
        commit.files.len()
    );
    Ok(())
}
