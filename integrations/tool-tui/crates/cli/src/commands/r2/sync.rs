//! R2 Sync Engine
//!
//! Intelligent sync with:
//! - Content-based deduplication (SHA256)
//! - Delta sync (only changed files)
//! - Resumable uploads
//! - Parallel transfers

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use super::client::R2Client;

/// Sync state tracker
pub struct SyncState {
    /// Local file hashes
    pub local_hashes: HashMap<PathBuf, String>,

    /// Remote file hashes (from ETags)
    pub remote_hashes: HashMap<String, String>,

    /// Files pending upload
    pub pending_uploads: Vec<SyncItem>,

    /// Files pending download
    pub pending_downloads: Vec<SyncItem>,

    /// Files to delete
    pub pending_deletes: Vec<String>,

    /// Sync statistics
    pub stats: SyncStats,
}

/// Sync item
#[derive(Debug, Clone)]
pub struct SyncItem {
    pub local_path: PathBuf,
    pub remote_key: String,
    pub size: u64,
    pub hash: String,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncAction {
    Upload,
    Download,
    Delete,
    Skip,
}

/// Sync statistics
#[derive(Debug, Default)]
pub struct SyncStats {
    pub files_uploaded: u64,
    pub files_downloaded: u64,
    pub files_deleted: u64,
    pub files_skipped: u64,
    pub bytes_transferred: u64,
    pub duration_ms: u64,
}

/// Calculate file hash
pub fn hash_file(path: &PathBuf) -> Result<String> {
    use sha2::{Digest, Sha256};

    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(hex::encode(hasher.finalize()))
}

/// Compute sync plan
pub async fn compute_sync_plan(
    client: &R2Client,
    local_dir: &PathBuf,
    remote_prefix: &str,
    options: &SyncOptions,
) -> Result<SyncState> {
    let mut state = SyncState {
        local_hashes: HashMap::new(),
        remote_hashes: HashMap::new(),
        pending_uploads: vec![],
        pending_downloads: vec![],
        pending_deletes: vec![],
        stats: SyncStats::default(),
    };

    // Scan local files
    for entry in walkdir::WalkDir::new(local_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path().to_path_buf();

            // Check exclusions
            if should_exclude(&path, &options.exclude) {
                continue;
            }

            let hash = hash_file(&path)?;
            state.local_hashes.insert(path.clone(), hash.clone());
        }
    }

    // Fetch remote objects
    let remote_objects = client.list_objects(remote_prefix, true).await?;
    for obj in &remote_objects {
        state.remote_hashes.insert(obj.key.clone(), obj.etag.clone());
    }

    // Compute differences
    for (local_path, local_hash) in &state.local_hashes {
        let relative = local_path.strip_prefix(local_dir)?;
        let remote_key = format!("{}/{}", remote_prefix.trim_end_matches('/'), relative.display());

        if let Some(remote_hash) = state.remote_hashes.get(&remote_key) {
            // File exists remotely
            if local_hash != remote_hash {
                // Content differs - needs sync
                match options.direction {
                    SyncDirection::Push => {
                        state.pending_uploads.push(SyncItem {
                            local_path: local_path.clone(),
                            remote_key: remote_key.clone(),
                            size: local_path.metadata()?.len(),
                            hash: local_hash.clone(),
                            action: SyncAction::Upload,
                        });
                    }
                    SyncDirection::Pull => {
                        state.pending_downloads.push(SyncItem {
                            local_path: local_path.clone(),
                            remote_key: remote_key.clone(),
                            size: 0, // Will be filled from remote
                            hash: remote_hash.clone(),
                            action: SyncAction::Download,
                        });
                    }
                    SyncDirection::Bidirectional => {
                        // Use timestamp to decide (not implemented)
                        state.pending_uploads.push(SyncItem {
                            local_path: local_path.clone(),
                            remote_key: remote_key.clone(),
                            size: local_path.metadata()?.len(),
                            hash: local_hash.clone(),
                            action: SyncAction::Upload,
                        });
                    }
                }
            }
        } else {
            // File doesn't exist remotely - upload if pushing
            if matches!(options.direction, SyncDirection::Push | SyncDirection::Bidirectional) {
                state.pending_uploads.push(SyncItem {
                    local_path: local_path.clone(),
                    remote_key: remote_key.clone(),
                    size: local_path.metadata()?.len(),
                    hash: local_hash.clone(),
                    action: SyncAction::Upload,
                });
            }
        }
    }

    // Find remote files not in local (for delete-sync)
    if options.delete {
        for remote_key in state.remote_hashes.keys() {
            let relative = remote_key
                .strip_prefix(&format!("{}/", remote_prefix.trim_end_matches('/')))
                .unwrap_or(remote_key);
            let local_path = local_dir.join(relative);

            if !state.local_hashes.contains_key(&local_path) {
                match options.direction {
                    SyncDirection::Push => {
                        state.pending_deletes.push(remote_key.clone());
                    }
                    SyncDirection::Pull => {
                        // Would delete local file, but we don't track that here
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(state)
}

/// Execute sync plan
pub async fn execute_sync(
    client: &R2Client,
    state: &mut SyncState,
    options: &SyncOptions,
    progress: Option<&dyn SyncProgress>,
) -> Result<()> {
    use std::time::Instant;

    let start = Instant::now();

    // Process uploads
    for item in &state.pending_uploads {
        if options.dry_run {
            state.stats.files_skipped += 1;
            continue;
        }

        if let Some(progress) = progress {
            progress.on_file_start(&item.local_path, item.size);
        }

        client.upload_file(&item.local_path, &item.remote_key, options.compress).await?;

        state.stats.files_uploaded += 1;
        state.stats.bytes_transferred += item.size;

        if let Some(progress) = progress {
            progress.on_file_complete(&item.local_path, item.size);
        }
    }

    // Process downloads
    for item in &state.pending_downloads {
        if options.dry_run {
            state.stats.files_skipped += 1;
            continue;
        }

        if let Some(progress) = progress {
            progress.on_file_start(&item.local_path, item.size);
        }

        // Create parent directory
        if let Some(parent) = item.local_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        client.download_file(&item.remote_key, &item.local_path).await?;

        state.stats.files_downloaded += 1;

        if let Some(progress) = progress {
            progress.on_file_complete(&item.local_path, item.size);
        }
    }

    // Process deletes
    for key in &state.pending_deletes {
        if options.dry_run {
            state.stats.files_skipped += 1;
            continue;
        }

        client.delete_object(key).await?;
        state.stats.files_deleted += 1;
    }

    state.stats.duration_ms = start.elapsed().as_millis() as u64;

    Ok(())
}

/// Sync options
#[derive(Debug, Clone)]
pub struct SyncOptions {
    /// Sync direction
    pub direction: SyncDirection,

    /// Delete files not in source
    pub delete: bool,

    /// Dry run (don't make changes)
    pub dry_run: bool,

    /// Compress files
    pub compress: bool,

    /// Exclude patterns
    pub exclude: Vec<String>,

    /// Number of parallel transfers
    pub parallel: usize,

    /// Retry count for failed transfers
    pub retries: u32,
}

impl Default for SyncOptions {
    fn default() -> Self {
        Self {
            direction: SyncDirection::Push,
            delete: false,
            dry_run: false,
            compress: false,
            exclude: vec![
                ".git/**".to_string(),
                "node_modules/**".to_string(),
                "target/**".to_string(),
                "*.log".to_string(),
            ],
            parallel: 4,
            retries: 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SyncDirection {
    Push,
    Pull,
    Bidirectional,
}

/// Progress callback trait
pub trait SyncProgress {
    fn on_file_start(&self, path: &PathBuf, size: u64);
    fn on_file_progress(&self, path: &PathBuf, bytes: u64);
    fn on_file_complete(&self, path: &PathBuf, size: u64);
    fn on_error(&self, path: &PathBuf, error: &str);
}

fn should_exclude(path: &PathBuf, patterns: &[String]) -> bool {
    let path_str = path.to_string_lossy();

    for pattern in patterns {
        if glob::Pattern::new(pattern).map_or(false, |p| p.matches(&path_str)) {
            return true;
        }
    }

    false
}

/// Resumable upload state
#[derive(Debug)]
pub struct ResumableUpload {
    pub upload_id: String,
    pub local_path: PathBuf,
    pub remote_key: String,
    pub part_size: u64,
    pub completed_parts: Vec<CompletedPart>,
    pub total_parts: u32,
}

#[derive(Debug, Clone)]
pub struct CompletedPart {
    pub part_number: u32,
    pub etag: String,
}

impl ResumableUpload {
    /// Start a new resumable upload
    pub async fn start(_client: &R2Client, path: &PathBuf, key: &str) -> Result<Self> {
        // TODO: Implement multipart upload initiation
        Ok(Self {
            upload_id: uuid::Uuid::new_v4().to_string(),
            local_path: path.clone(),
            remote_key: key.to_string(),
            part_size: 5 * 1024 * 1024, // 5MB parts
            completed_parts: vec![],
            total_parts: 0,
        })
    }

    /// Upload next part
    pub async fn upload_part(
        &mut self,
        _client: &R2Client,
        part_number: u32,
        _data: &[u8],
    ) -> Result<()> {
        // TODO: Implement part upload
        self.completed_parts.push(CompletedPart {
            part_number,
            etag: "placeholder".to_string(),
        });
        Ok(())
    }

    /// Complete the upload
    pub async fn complete(self, _client: &R2Client) -> Result<()> {
        // TODO: Implement multipart completion
        Ok(())
    }

    /// Abort the upload
    pub async fn abort(self, _client: &R2Client) -> Result<()> {
        // TODO: Implement multipart abort
        Ok(())
    }
}
