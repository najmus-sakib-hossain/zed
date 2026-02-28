//! Rollback functionality for AI changes

use anyhow::Result;
use std::path::PathBuf;

/// Rollback state
#[derive(Debug, Clone)]
pub struct RollbackState {
    pub change_id: String,
    pub timestamp: u64,
    pub files: Vec<FileBackup>,
}

/// File backup for rollback
#[derive(Debug, Clone)]
pub struct FileBackup {
    pub path: PathBuf,
    pub content: Vec<u8>,
    pub existed: bool,
}

/// Get rollback directory
fn rollback_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("dx")
        .join("rollback")
}

/// Create a rollback point before applying changes
pub fn create_rollback_point(change_id: &str, files: &[PathBuf]) -> Result<RollbackState> {
    let dir = rollback_dir();
    std::fs::create_dir_all(&dir)?;

    let mut backups = Vec::new();

    for file in files {
        let backup = if file.exists() {
            FileBackup {
                path: file.clone(),
                content: std::fs::read(file)?,
                existed: true,
            }
        } else {
            FileBackup {
                path: file.clone(),
                content: Vec::new(),
                existed: false,
            }
        };
        backups.push(backup);
    }

    let state = RollbackState {
        change_id: change_id.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        files: backups,
    };

    // Save rollback state
    save_rollback_state(&state)?;

    Ok(state)
}

/// Save rollback state to disk
fn save_rollback_state(state: &RollbackState) -> Result<()> {
    let dir = rollback_dir();
    let path = dir.join(format!("{}.rollback", state.change_id));

    // Simple binary format:
    // - 8 bytes: timestamp
    // - 4 bytes: number of files
    // For each file:
    // - 4 bytes: path length
    // - N bytes: path
    // - 1 byte: existed flag
    // - 4 bytes: content length
    // - N bytes: content

    let mut data = Vec::new();

    // Timestamp
    data.extend_from_slice(&state.timestamp.to_le_bytes());

    // Number of files
    data.extend_from_slice(&(state.files.len() as u32).to_le_bytes());

    for backup in &state.files {
        let path_bytes = backup.path.to_string_lossy().as_bytes().to_vec();

        // Path length and path
        data.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(&path_bytes);

        // Existed flag
        data.push(if backup.existed { 1 } else { 0 });

        // Content length and content
        data.extend_from_slice(&(backup.content.len() as u32).to_le_bytes());
        data.extend_from_slice(&backup.content);
    }

    std::fs::write(&path, data)?;

    Ok(())
}

/// Load rollback state from disk
fn load_rollback_state(change_id: &str) -> Result<RollbackState> {
    let dir = rollback_dir();
    let path = dir.join(format!("{}.rollback", change_id));

    if !path.exists() {
        anyhow::bail!("No rollback state found for change {}", change_id);
    }

    let data = std::fs::read(&path)?;
    let mut pos = 0;

    // Timestamp
    let timestamp = u64::from_le_bytes(data[pos..pos + 8].try_into()?);
    pos += 8;

    // Number of files
    let num_files = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
    pos += 4;

    let mut files = Vec::with_capacity(num_files);

    for _ in 0..num_files {
        // Path length and path
        let path_len = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
        pos += 4;

        let path_str = String::from_utf8(data[pos..pos + path_len].to_vec())?;
        pos += path_len;

        // Existed flag
        let existed = data[pos] == 1;
        pos += 1;

        // Content length and content
        let content_len = u32::from_le_bytes(data[pos..pos + 4].try_into()?) as usize;
        pos += 4;

        let content = data[pos..pos + content_len].to_vec();
        pos += content_len;

        files.push(FileBackup {
            path: PathBuf::from(path_str),
            content,
            existed,
        });
    }

    Ok(RollbackState {
        change_id: change_id.to_string(),
        timestamp,
        files,
    })
}

/// Rollback a change
pub fn rollback(change_id: &str) -> Result<()> {
    let state = load_rollback_state(change_id)?;

    for backup in &state.files {
        if backup.existed {
            // Restore original content
            if let Some(parent) = backup.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&backup.path, &backup.content)?;
        } else {
            // File didn't exist before, remove it
            if backup.path.exists() {
                std::fs::remove_file(&backup.path)?;
            }
        }
    }

    // Remove rollback state
    let dir = rollback_dir();
    let path = dir.join(format!("{}.rollback", change_id));
    std::fs::remove_file(&path)?;

    Ok(())
}

/// List available rollback points
pub fn list_rollback_points() -> Result<Vec<(String, u64)>> {
    let dir = rollback_dir();

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut points = Vec::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "rollback" {
                if let Some(stem) = path.file_stem() {
                    let change_id = stem.to_string_lossy().to_string();

                    // Read timestamp from file
                    if let Ok(data) = std::fs::read(&path) {
                        if data.len() >= 8 {
                            let timestamp =
                                u64::from_le_bytes(data[0..8].try_into().unwrap_or([0; 8]));
                            points.push((change_id, timestamp));
                        }
                    }
                }
            }
        }
    }

    // Sort by timestamp descending
    points.sort_by(|a, b| b.1.cmp(&a.1));

    Ok(points)
}

/// Clean up old rollback points
pub fn cleanup_rollback_points(max_age_days: u32) -> Result<u32> {
    let dir = rollback_dir();

    if !dir.exists() {
        return Ok(0);
    }

    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - (max_age_days as u64 * 86400);

    let mut removed = 0;

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(ext) = path.extension() {
            if ext == "rollback" {
                if let Ok(data) = std::fs::read(&path) {
                    if data.len() >= 8 {
                        let timestamp = u64::from_le_bytes(data[0..8].try_into().unwrap_or([0; 8]));

                        if timestamp < cutoff {
                            std::fs::remove_file(&path)?;
                            removed += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(removed)
}

/// Get storage used by rollback points
pub fn rollback_storage_used() -> Result<u64> {
    let dir = rollback_dir();

    if !dir.exists() {
        return Ok(0);
    }

    let mut total = 0u64;

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        if let Ok(metadata) = entry.metadata() {
            total += metadata.len();
        }
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_rollback_dir() {
        let dir = rollback_dir();
        assert!(dir.to_string_lossy().contains("dx"));
    }
}
