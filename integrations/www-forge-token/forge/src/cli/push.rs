use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context, Result};

use crate::core::manifest::deserialize_commit;
use crate::core::repository::Repository;
use crate::db::metadata::MetadataDb;
use crate::mirror::auth::AuthStore;
use crate::mirror::backends::{
    dropbox::DropboxBackend,
    gdrive::GoogleDriveBackend,
    github::GitHubBackend,
    mega::MegaBackend,
    pinterest::PinterestBackend,
    r2::R2Backend,
    sketchfab::SketchfabBackend,
    soundcloud::SoundCloudBackend,
    youtube::YouTubeBackend,
};
use crate::mirror::{MirrorBackend, MirrorDispatcher, MirrorTarget};
use crate::store::cas::ChunkStore;
use crate::store::compression;

/// Serialisable record stored per-file in MIRRORS_TABLE.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct MirrorRecord {
    backend: String,
    url: String,
}

impl From<(&str, &MirrorTarget)> for MirrorRecord {
    fn from((backend, target): (&str, &MirrorTarget)) -> Self {
        Self {
            backend: backend.to_string(),
            url: target.public_url(),
        }
    }
}

pub fn run(remote: &str, mirror: Option<&str>, pro: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("get cwd")?;
    let repo = Repository::discover(&cwd)?;

    // Build the tokio runtime (all mirror backends are async).
    let rt = tokio::runtime::Runtime::new().context("create tokio runtime")?;

    // Determine mode ----------------------------------------------------------
    let mirror_mode = mirror.unwrap_or(if pro { "pro" } else { "all-free" });

    // Read latest commit to know what files to push --------------------------
    let head_id = repo
        .read_head()?
        .ok_or_else(|| anyhow::anyhow!("nothing to push — no commits yet"))?;
    let head_hex = hex::encode(head_id);

    let manifest_path = repo.forge_dir.join("manifests").join(&head_hex);
    let manifest_bytes = std::fs::read(&manifest_path)
        .with_context(|| format!("read manifest {}", manifest_path.display()))?;
    let commit = deserialize_commit(&manifest_bytes)?;

    println!(
        "Pushing commit {} ({} files) → mirror: {mirror_mode}",
        &head_hex[..12],
        commit.files.len(),
    );

    // Build backends ----------------------------------------------------------
    let auth = Arc::new(AuthStore::open(&repo.forge_dir)?);
    let backends = build_backends(&auth, mirror_mode, &repo)?;

    if backends.is_empty() {
        bail!(
            "No mirror backends available for mode '{mirror_mode}'.\n\
             Run `forge auth all-free` first, or specify `--mirror <backend>`."
        );
    }

    let dispatcher = MirrorDispatcher::new(backends);
    let store = ChunkStore::new(repo.forge_dir.join("objects/chunks"));
    let db = MetadataDb::open(&repo.metadata_db_path())?;

    let mut total_ok: usize = 0;
    let mut total_err: usize = 0;

    // Mirror each file --------------------------------------------------------
    for entry in &commit.files {
        // Reassemble the whole file from chunks
        let mut data = Vec::with_capacity(entry.size as usize);
        for chunk_ref in &entry.chunks {
            let hash = blake3::Hash::from(chunk_ref.hash);
            let compressed = store.read(&hash)?;
            let raw = compression::decompress(&compressed)?;
            data.extend_from_slice(&raw);
        }

        let file_path = Path::new(&entry.path);
        let results = rt.block_on(dispatcher.mirror(file_path, data));

        let mut records: Vec<MirrorRecord> = Vec::new();
        for r in &results {
            match &r.target {
                Ok(target) => {
                    let url = target.public_url();
                    println!("  ✓ {} → {}", r.backend, url);
                    records.push(MirrorRecord::from((r.backend, target)));
                    total_ok += 1;
                }
                Err(e) => {
                    eprintln!("  ✗ {} — {}", r.backend, e);
                    total_err += 1;
                }
            }
        }

        // Persist mirror targets for this file in redb
        if !records.is_empty() {
            let json = serde_json::to_vec(&records).context("serialize mirror records")?;
            db.store_mirror_targets(&entry.path, &json)?;
        }
    }

    println!();
    if total_err == 0 {
        println!(
            "Push complete ✓  {total_ok} mirror(s) across {} files. No errors.",
            commit.files.len()
        );
    } else {
        println!(
            "Push finished — {total_ok} succeeded, {total_err} failed. Run with --verbose for details."
        );
    }

    // Also note the remote for future QUIC transport (informational)
    if remote != "origin" {
        println!("(remote '{remote}' noted — QUIC transport not yet available)");
    }

    Ok(())
}

/// Assemble the list of backends based on mode string.
fn build_backends(
    auth: &Arc<AuthStore>,
    mode: &str,
    repo: &Repository,
) -> Result<Vec<Arc<dyn MirrorBackend>>> {
    let mut out: Vec<Arc<dyn MirrorBackend>> = Vec::new();

    // Helper: only add a backend if the user has authed it.
    let has_auth = |name: &str| -> bool { auth.load(name).ok().flatten().is_some() };

    match mode {
        "all-free" => {
            if has_auth("youtube")    { out.push(Arc::new(YouTubeBackend::new(Arc::clone(auth)))); }
            if has_auth("pinterest")  { out.push(Arc::new(PinterestBackend::new(Arc::clone(auth)))); }
            if has_auth("soundcloud") { out.push(Arc::new(SoundCloudBackend::new(Arc::clone(auth)))); }
            if has_auth("sketchfab")  { out.push(Arc::new(SketchfabBackend::new(Arc::clone(auth)))); }
            if has_auth("github") {
                let github_repo = read_github_repo(repo)?;
                out.push(Arc::new(GitHubBackend::new(Arc::clone(auth), github_repo)));
            }
        }
        "pro" => {
            // Pro = R2/S3 as primary catch-all + optional free mirrors
            if has_auth("r2") {
                let r2_info = auth.load("r2")?.unwrap();
                let bucket = r2_info.extra["bucket"].as_str().unwrap_or("forge").to_string();
                let endpoint = r2_info.extra["endpoint"].as_str().unwrap_or("").to_string();
                out.push(Arc::new(R2Backend::new(Arc::clone(auth), bucket, endpoint)));
            }
            if has_auth("gdrive")  { out.push(Arc::new(GoogleDriveBackend::new(Arc::clone(auth)))); }
            if has_auth("dropbox") { out.push(Arc::new(DropboxBackend::new(Arc::clone(auth)))); }
            if has_auth("mega")    { out.push(Arc::new(MegaBackend::new(Arc::clone(auth)))); }
            // Also add free backends if authed
            if has_auth("youtube")    { out.push(Arc::new(YouTubeBackend::new(Arc::clone(auth)))); }
            if has_auth("pinterest")  { out.push(Arc::new(PinterestBackend::new(Arc::clone(auth)))); }
            if has_auth("soundcloud") { out.push(Arc::new(SoundCloudBackend::new(Arc::clone(auth)))); }
            if has_auth("sketchfab")  { out.push(Arc::new(SketchfabBackend::new(Arc::clone(auth)))); }
            if has_auth("github") {
                let github_repo = read_github_repo(repo)?;
                out.push(Arc::new(GitHubBackend::new(Arc::clone(auth), github_repo)));
            }
        }
        // Single backend by name
        single => {
            match single {
                "youtube"    if has_auth("youtube")    => out.push(Arc::new(YouTubeBackend::new(Arc::clone(auth)))),
                "pinterest"  if has_auth("pinterest")  => out.push(Arc::new(PinterestBackend::new(Arc::clone(auth)))),
                "soundcloud" if has_auth("soundcloud") => out.push(Arc::new(SoundCloudBackend::new(Arc::clone(auth)))),
                "sketchfab"  if has_auth("sketchfab")  => out.push(Arc::new(SketchfabBackend::new(Arc::clone(auth)))),
                "github"     if has_auth("github")     => {
                    let github_repo = read_github_repo(repo)?;
                    out.push(Arc::new(GitHubBackend::new(Arc::clone(auth), github_repo)));
                }
                "gdrive"     if has_auth("gdrive")     => out.push(Arc::new(GoogleDriveBackend::new(Arc::clone(auth)))),
                "dropbox"    if has_auth("dropbox")     => out.push(Arc::new(DropboxBackend::new(Arc::clone(auth)))),
                "mega"       if has_auth("mega")        => out.push(Arc::new(MegaBackend::new(Arc::clone(auth)))),
                "r2"         if has_auth("r2")          => {
                    let r2_info = auth.load("r2")?.unwrap();
                    let bucket = r2_info.extra["bucket"].as_str().unwrap_or("forge").to_string();
                    let endpoint = r2_info.extra["endpoint"].as_str().unwrap_or("").to_string();
                    out.push(Arc::new(R2Backend::new(Arc::clone(auth), bucket, endpoint)));
                }
                name => {
                    if !has_auth(name) {
                        bail!("Backend '{name}' is not authenticated. Run: forge auth {name}");
                    }
                    bail!("Unknown backend: '{name}'");
                }
            }
        }
    }

    Ok(out)
}

/// Read the github mirror repo from config.toml or default from `forge auth` extra field.
fn read_github_repo(repo: &Repository) -> Result<String> {
    // Try config.toml first
    let cfg = repo.read_config()?;
    if let Some(url) = &cfg.remote_url {
        // Extract owner/repo from a github URL
        if let Some(path) = url.strip_prefix("https://github.com/") {
            let repo_path = path.trim_end_matches('/').trim_end_matches(".git");
            return Ok(repo_path.to_string());
        }
    }
    // Fallback: use author/forge-mirror
    let author = std::env::var("GIT_AUTHOR_NAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "forge-user".to_string());
    Ok(format!("{author}/forge-mirror"))
}
