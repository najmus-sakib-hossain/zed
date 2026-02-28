pub mod blob;
pub mod db;
pub mod git_interop;
pub mod oplog;
pub mod r2;

use anyhow::{Context, Result};
use colored::*;
use ropey::Rope;
use std::path::Path;

pub use blob::{Blob, BlobMetadata, BlobRepository};
pub use db::{Database, DatabasePool, DatabasePoolConfig, PooledConnection};
pub use oplog::OperationLog;
pub use r2::{R2Config, R2Storage, SyncResult, batch_upload_blobs};

const FORGE_DIR: &str = ".dx/forge";

pub async fn init(path: &Path) -> Result<()> {
    let forge_path = path.join(FORGE_DIR);

    tokio::fs::create_dir_all(&forge_path)
        .await
        .with_context(|| format!("Failed to create forge directory: {:?}", forge_path))?;
    tokio::fs::create_dir_all(forge_path.join("objects"))
        .await
        .context("Failed to create objects directory")?;
    tokio::fs::create_dir_all(forge_path.join("refs"))
        .await
        .context("Failed to create refs directory")?;
    tokio::fs::create_dir_all(forge_path.join("logs"))
        .await
        .context("Failed to create logs directory")?;
    tokio::fs::create_dir_all(forge_path.join("context"))
        .await
        .context("Failed to create context directory")?;

    // Initialize database
    let db = Database::new(&forge_path).context("Failed to create database")?;
    db.initialize().context("Failed to initialize database")?;

    // Create config
    let config = serde_json::json!({
        "version": "0.1.0",
        "actor_id": uuid::Uuid::new_v4().to_string(),
        "repo_id": uuid::Uuid::new_v4().to_string(),
        "git_interop": true,
        "real_time_sync": false,
    });

    tokio::fs::write(forge_path.join("config.json"), serde_json::to_string_pretty(&config)?)
        .await
        .context("Failed to write config.json")?;

    Ok(())
}

pub async fn show_log(file: Option<std::path::PathBuf>, limit: usize) -> Result<()> {
    let db = Database::open(".dx/forge").context("Failed to open database for log viewing")?;
    let operations = db
        .get_operations(file.as_deref(), limit)
        .context("Failed to retrieve operations from database")?;

    println!("{}", "Operation Log".cyan().bold());
    println!("{}", "═".repeat(80).bright_black());

    for op in operations {
        let time = op.timestamp.format("%Y-%m-%d %H:%M:%S%.3f");
        let op_type = match &op.op_type {
            crate::crdt::OperationType::Insert { length, .. } => {
                format!("+{} chars", length).green()
            }
            crate::crdt::OperationType::Delete { length, .. } => format!("-{} chars", length).red(),
            crate::crdt::OperationType::Replace {
                old_content,
                new_content,
                ..
            } => format!("~{}->{} chars", old_content.len(), new_content.len()).yellow(),
            crate::crdt::OperationType::FileCreate { .. } => "FILE_CREATE".bright_green(),
            crate::crdt::OperationType::FileDelete => "FILE_DELETE".bright_red(),
            crate::crdt::OperationType::FileRename { old_path, new_path } => {
                format!("RENAME {} -> {}", old_path, new_path).bright_yellow()
            }
        };

        println!(
            "{} {} {} {}",
            format!("[{}]", time).bright_black(),
            op_type.bold(),
            op.file_path.bright_white(),
            format!("({})", op.id).bright_black()
        );
    }

    Ok(())
}

pub async fn git_sync(path: &Path) -> Result<()> {
    git_interop::sync_with_git(path).await
}

pub async fn time_travel(file: &Path, timestamp: Option<String>) -> Result<()> {
    println!("{}", format!("🕐 Time traveling: {}", file.display()).cyan().bold());

    let repo_root =
        std::env::current_dir().context("Failed to get current directory for time travel")?;
    let forge_path = repo_root.join(FORGE_DIR);
    let db = Database::new(&forge_path).context("Failed to open database for time travel")?;
    db.initialize().context("Failed to initialize database for time travel")?;

    let target_path = if file.is_absolute() {
        file.to_path_buf()
    } else {
        repo_root.join(file)
    };
    let target_canon = normalize_path(&target_path);

    let mut operations = db
        .get_operations(None, 2000)
        .context("Failed to retrieve operations for time travel")?;

    // Reconstruct file state at timestamp
    let target_time = if let Some(ts) = timestamp {
        chrono::DateTime::parse_from_rfc3339(&ts)?.with_timezone(&chrono::Utc)
    } else {
        chrono::Utc::now()
    };

    operations.retain(|op| {
        op.timestamp <= target_time
            && normalize_path(std::path::Path::new(&op.file_path)) == target_canon
    });
    operations.sort_by_key(|op| op.timestamp);

    let mut rope = Rope::new();

    for op in operations.iter() {
        match &op.op_type {
            crate::crdt::OperationType::FileCreate { content: c } => {
                rope = Rope::from_str(c);
            }
            crate::crdt::OperationType::Insert {
                position, content, ..
            } => {
                let char_idx = clamp_offset(&rope, position.offset);
                rope.insert(char_idx, content);
            }
            crate::crdt::OperationType::Delete { position, length } => {
                let start = clamp_offset(&rope, position.offset);
                let end = clamp_offset(&rope, start + *length);
                if start < end {
                    rope.remove(start..end);
                }
            }
            crate::crdt::OperationType::Replace {
                position,
                old_content,
                new_content,
            } => {
                let start = clamp_offset(&rope, position.offset);
                let end = clamp_offset(&rope, start + old_content.chars().count());
                if start < end {
                    rope.remove(start..end);
                }
                rope.insert(start, new_content);
            }
            crate::crdt::OperationType::FileDelete => {
                rope = Rope::new();
            }
            crate::crdt::OperationType::FileRename { .. } => {
                // Rename events are handled by resolving the target path above.
            }
        }
    }

    let content = rope.to_string();

    println!("\n{}", "─".repeat(80).bright_black());
    println!("{}", content);
    println!("{}", "─".repeat(80).bright_black());

    Ok(())
}

fn normalize_path(path: &Path) -> std::path::PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn clamp_offset(rope: &Rope, offset: usize) -> usize {
    offset.min(rope.len_chars())
}
