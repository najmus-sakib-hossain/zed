use std::path::Path;

use anyhow::{Context, Result};
use redb::ReadableTable;

pub const FILES_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("files");
pub const CHUNKS_TABLE: redb::TableDefinition<&str, u32> = redb::TableDefinition::new("chunks");
pub const COMMITS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("commits");
pub const STAGING_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("staging");
/// Maps "file_path" → JSON array of mirror targets for that file.
pub const MIRRORS_TABLE: redb::TableDefinition<&str, &[u8]> = redb::TableDefinition::new("mirrors");

pub struct MetadataDb {
    pub db: redb::Database,
}

fn chunk_hex(hash: &[u8; 32]) -> String {
    hex::encode(hash)
}

impl MetadataDb {
    pub fn create(path: &Path) -> Result<Self> {
        let db = redb::Database::create(path).with_context(|| format!("create redb {}", path.display()))?;
        let write_txn = db.begin_write().context("begin write transaction")?;
        {
            write_txn.open_table(FILES_TABLE).context("open FILES_TABLE")?;
            write_txn.open_table(CHUNKS_TABLE).context("open CHUNKS_TABLE")?;
            write_txn
                .open_table(COMMITS_TABLE)
                .context("open COMMITS_TABLE")?;
            write_txn
                .open_table(STAGING_TABLE)
                .context("open STAGING_TABLE")?;
            write_txn
                .open_table(MIRRORS_TABLE)
                .context("open MIRRORS_TABLE")?;
        }
        write_txn.commit().context("commit create schema")?;
        Ok(Self { db })
    }

    pub fn open(path: &Path) -> Result<Self> {
        let db = redb::Database::open(path).with_context(|| format!("open redb {}", path.display()))?;
        // Ensure MIRRORS_TABLE exists for older repos that were init'd before it was added.
        {
            let write_txn = db.begin_write().context("begin write txn for schema migration")?;
            write_txn.open_table(MIRRORS_TABLE).context("ensure MIRRORS_TABLE")?;
            write_txn.commit().context("commit schema migration")?;
        }
        Ok(Self { db })
    }

    pub fn is_chunk_known(&self, hash: &[u8; 32]) -> Result<bool> {
        let hex = chunk_hex(hash);
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn.open_table(CHUNKS_TABLE).context("open chunks table")?;
        Ok(table.get(hex.as_str()).context("get chunk")?.is_some())
    }

    pub fn insert_chunk(&self, hash: &[u8; 32]) -> Result<()> {
        let hex = chunk_hex(hash);
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn.open_table(CHUNKS_TABLE).context("open chunks table")?;
            let count = table
                .get(hex.as_str())
                .context("get chunk count")?
                .map(|v| v.value())
                .unwrap_or(0);
            table
                .insert(hex.as_str(), count.saturating_add(1))
                .context("insert chunk count")?;
        }
        write_txn.commit().context("commit insert chunk")?;
        Ok(())
    }

    pub fn decrement_chunk(&self, hash: &[u8; 32]) -> Result<bool> {
        let hex = chunk_hex(hash);
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        let reached_zero;
        {
            let mut table = write_txn.open_table(CHUNKS_TABLE).context("open chunks table")?;
            let existing_count = table
                .get(hex.as_str())
                .context("get chunk count")?
                .map(|current| current.value());
            if let Some(count) = existing_count {
                if count <= 1 {
                    table.remove(hex.as_str()).context("remove chunk key")?;
                    reached_zero = true;
                } else {
                    table
                        .insert(hex.as_str(), count - 1)
                        .context("decrement chunk count")?;
                    reached_zero = false;
                }
            } else {
                reached_zero = false;
            }
        }
        write_txn.commit().context("commit decrement chunk")?;
        Ok(reached_zero)
    }

    pub fn stage_file(&self, path: &str, entry_bytes: &[u8]) -> Result<()> {
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn
                .open_table(STAGING_TABLE)
                .context("open staging table")?;
            table.insert(path, entry_bytes).context("insert staged file")?;
        }
        write_txn.commit().context("commit stage file")?;
        Ok(())
    }

    pub fn unstage_file(&self, path: &str) -> Result<()> {
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn
                .open_table(STAGING_TABLE)
                .context("open staging table")?;
            table.remove(path).context("remove staged file")?;
        }
        write_txn.commit().context("commit unstage file")?;
        Ok(())
    }

    pub fn get_staged_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn
            .open_table(STAGING_TABLE)
            .context("open staging table")?;
        let mut out = Vec::new();
        for entry in table.iter().context("iterate staging table")? {
            let (key, val) = entry.context("read staged table entry")?;
            out.push((key.value().to_string(), val.value().to_vec()));
        }
        Ok(out)
    }

    pub fn clear_staging(&self) -> Result<()> {
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn
                .open_table(STAGING_TABLE)
                .context("open staging table")?;
            let mut keys: Vec<String> = Vec::new();
            for entry in table.iter().context("iterate staging table")? {
                let (key, _) = entry.context("read staging row")?;
                keys.push(key.value().to_string());
            }
            for key in keys {
                table.remove(key.as_str()).context("remove staged key")?;
            }
        }
        write_txn.commit().context("commit clear staging")?;
        Ok(())
    }

    pub fn store_commit(&self, id_hex: &str, commit_bytes: &[u8]) -> Result<()> {
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn
                .open_table(COMMITS_TABLE)
                .context("open commits table")?;
            table
                .insert(id_hex, commit_bytes)
                .context("insert commit bytes")?;
        }
        write_txn.commit().context("commit commit-bytes write")?;
        Ok(())
    }

    pub fn get_commit(&self, id_hex: &str) -> Result<Option<Vec<u8>>> {
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn
            .open_table(COMMITS_TABLE)
            .context("open commits table")?;
        Ok(table
            .get(id_hex)
            .context("read commit bytes")?
            .map(|v| v.value().to_vec()))
    }

    pub fn store_file_entry(&self, path: &str, entry_bytes: &[u8]) -> Result<()> {
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn.open_table(FILES_TABLE).context("open files table")?;
            table.insert(path, entry_bytes).context("insert file entry")?;
        }
        write_txn.commit().context("commit file entry write")?;
        Ok(())
    }

    pub fn get_file_entry(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn.open_table(FILES_TABLE).context("open files table")?;
        Ok(table
            .get(path)
            .context("read file entry")?
            .map(|v| v.value().to_vec()))
    }

    pub fn get_all_tracked_files(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn.open_table(FILES_TABLE).context("open files table")?;
        let mut out = Vec::new();
        for entry in table.iter().context("iterate files table")? {
            let (key, val) = entry.context("read files table entry")?;
            out.push((key.value().to_string(), val.value().to_vec()));
        }
        Ok(out)
    }

    // ---- mirror target persistence -----------------------------------------

    /// Store mirror targets (as JSON bytes) for a file path.
    pub fn store_mirror_targets(&self, file_path: &str, targets_json: &[u8]) -> Result<()> {
        let write_txn = self.db.begin_write().context("begin write transaction")?;
        {
            let mut table = write_txn.open_table(MIRRORS_TABLE).context("open mirrors table")?;
            table.insert(file_path, targets_json).context("insert mirror targets")?;
        }
        write_txn.commit().context("commit mirror targets")?;
        Ok(())
    }

    /// Load mirror targets (as JSON bytes) for a file path.
    pub fn get_mirror_targets(&self, file_path: &str) -> Result<Option<Vec<u8>>> {
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn.open_table(MIRRORS_TABLE).context("open mirrors table")?;
        Ok(table
            .get(file_path)
            .context("read mirror targets")?
            .map(|v| v.value().to_vec()))
    }

    /// List all file paths that have mirror targets stored.
    pub fn get_all_mirror_targets(&self) -> Result<Vec<(String, Vec<u8>)>> {
        let read_txn = self.db.begin_read().context("begin read transaction")?;
        let table = read_txn.open_table(MIRRORS_TABLE).context("open mirrors table")?;
        let mut out = Vec::new();
        for entry in table.iter().context("iterate mirrors table")? {
            let (key, val) = entry.context("read mirrors row")?;
            out.push((key.value().to_string(), val.value().to_vec()));
        }
        Ok(out)
    }
}
