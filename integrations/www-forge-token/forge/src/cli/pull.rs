use std::path::Path;

use anyhow::{Context, Result};

use crate::core::repository::Repository;
use crate::db::metadata::MetadataDb;

/// Mirror record stored per-file — matches the struct in push.rs.
#[derive(serde::Deserialize, Debug)]
struct MirrorRecord {
    backend: String,
    url: String,
}

pub fn run(remote: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("get cwd")?;
    let repo = Repository::discover(&cwd)?;

    let db = MetadataDb::open(&repo.metadata_db_path())?;
    let all_targets = db.get_all_mirror_targets()?;

    if all_targets.is_empty() {
        println!("No mirror targets found. Push first with `forge push --mirror all-free`.");
        return Ok(());
    }

    let rt = tokio::runtime::Runtime::new().context("create tokio runtime")?;
    let client = reqwest::Client::new();

    let mut downloaded: usize = 0;
    let mut failed: usize = 0;

    println!(
        "Pulling {} mirrored file(s)… (remote: {remote})",
        all_targets.len()
    );

    for (file_path, json_bytes) in &all_targets {
        let records: Vec<MirrorRecord> = serde_json::from_slice(json_bytes)
            .with_context(|| format!("parse mirror records for {file_path}"))?;

        if records.is_empty() {
            continue;
        }

        // Try each mirror until one succeeds
        let mut success = false;
        for rec in &records {
            match rt.block_on(try_download(&client, &rec.url)) {
                Ok(data) => {
                    // Write file to working directory
                    let out_path = Path::new(file_path);
                    if let Some(parent) = out_path.parent() {
                        std::fs::create_dir_all(parent).ok();
                    }
                    std::fs::write(out_path, &data)
                        .with_context(|| format!("write {file_path}"))?;
                    println!("  ✓ {} ← {} ({})", file_path, rec.backend, rec.url);
                    downloaded += 1;
                    success = true;
                    break;
                }
                Err(e) => {
                    tracing::debug!("  mirror {} failed for {}: {}", rec.backend, file_path, e);
                    continue;
                }
            }
        }

        if !success {
            eprintln!(
                "  ✗ {} — all {} mirror(s) failed",
                file_path,
                records.len()
            );
            failed += 1;
        }
    }

    println!();
    if failed == 0 {
        println!("Pull complete ✓  {downloaded} file(s) restored.");
    } else {
        println!("Pull finished — {downloaded} restored, {failed} failed.");
    }

    Ok(())
}

/// Attempt to download a file from a mirror URL.
///
/// For most backends the URL is a direct HTTPS link. Some backends
/// (GitHub raw, Google Drive, Dropbox) need adjusted URLs.
async fn try_download(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let download_url = resolve_download_url(url);
    let resp = client
        .get(&download_url)
        .header("User-Agent", "forge/0.1")
        .send()
        .await
        .context("HTTP request")?;

    if !resp.status().is_success() {
        anyhow::bail!("HTTP {} from {}", resp.status(), download_url);
    }

    let bytes = resp.bytes().await.context("read body")?;
    Ok(bytes.to_vec())
}

/// Rewrite public viewer URLs to raw download URLs where possible.
fn resolve_download_url(url: &str) -> String {
    // GitHub blob → raw
    if url.contains("github.com") && url.contains("/blob/") {
        return url
            .replace("github.com", "raw.githubusercontent.com")
            .replace("/blob/", "/");
    }

    // Google Drive — rewrite to direct download
    if url.contains("drive.google.com/file/d/") {
        if let Some(id) = url
            .strip_prefix("https://drive.google.com/file/d/")
            .and_then(|s| s.split('/').next())
        {
            return format!("https://drive.google.com/uc?export=download&id={id}");
        }
    }

    // Dropbox — add dl=1 for direct download
    if url.contains("dropbox.com") {
        let base = url.split('?').next().unwrap_or(url);
        return format!("{base}?dl=1");
    }

    // Mega — can't easily direct-download from the public URL without JS,
    // but return as-is; the HTTP attempt will fail and we fall through to
    // the next mirror.
    url.to_string()
}
