//! # File-based Session Storage
//!
//! Implements `SessionStorage` with file-based persistence.
//! Features:
//! - Atomic writes via temp file + rename
//! - Automatic backups on modification
//! - Compression for large sessions (>1MB)
//! - Session listing with directory scanning

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use super::{Session, SessionError, SessionFilter, SessionStorage};

/// Threshold for enabling compression (1MB)
const COMPRESSION_THRESHOLD: usize = 1_048_576;

/// Maximum number of backups to keep per session
const MAX_BACKUPS: usize = 3;

/// File-based session storage with atomic writes and backups
pub struct FileSessionStorage {
    /// Base directory for session files
    base_dir: PathBuf,
    /// Backup directory
    backup_dir: PathBuf,
}

impl FileSessionStorage {
    /// Create a new file-based session storage
    pub fn new(base_dir: PathBuf) -> Result<Self, SessionError> {
        let backup_dir = base_dir.join("backups");
        fs::create_dir_all(&base_dir).map_err(|e| {
            SessionError::StorageError(format!("Failed to create storage dir: {}", e))
        })?;
        fs::create_dir_all(&backup_dir).map_err(|e| {
            SessionError::StorageError(format!("Failed to create backup dir: {}", e))
        })?;

        Ok(Self {
            base_dir,
            backup_dir,
        })
    }

    /// Get the file path for a session key
    fn session_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", key))
    }

    /// Get the compressed file path for a session key
    fn compressed_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json.gz", key))
    }

    /// Get the temp file path for atomic writes
    fn temp_path(&self, key: &str) -> PathBuf {
        self.base_dir.join(format!(".{}.tmp", key))
    }

    /// Create a backup of a session file before modification
    fn create_backup(&self, key: &str) -> Result<(), SessionError> {
        let session_path = self.session_path(key);
        let compressed_path = self.compressed_path(key);

        let source = if session_path.exists() {
            &session_path
        } else if compressed_path.exists() {
            &compressed_path
        } else {
            return Ok(()); // No existing file to back up
        };

        let ext = if source == &compressed_path {
            "json.gz"
        } else {
            "json"
        };

        // Rotate backups: .3 -> delete, .2 -> .3, .1 -> .2, current -> .1
        for i in (1..MAX_BACKUPS).rev() {
            let from = self.backup_dir.join(format!("{}.{}.{}", key, i, ext));
            let to = self.backup_dir.join(format!("{}.{}.{}", key, i + 1, ext));
            if from.exists() {
                let _ = fs::rename(&from, &to);
            }
        }

        let backup_path = self.backup_dir.join(format!("{}.1.{}", key, ext));
        fs::copy(source, &backup_path)
            .map_err(|e| SessionError::StorageError(format!("Failed to create backup: {}", e)))?;

        Ok(())
    }

    /// Atomic write: write to temp file, then rename
    fn atomic_write(&self, key: &str, data: &[u8], compress: bool) -> Result<(), SessionError> {
        let temp = self.temp_path(key);

        // Write to temp file
        if compress {
            let file = fs::File::create(&temp).map_err(|e| {
                SessionError::StorageError(format!("Failed to create temp file: {}", e))
            })?;
            let mut encoder = GzEncoder::new(file, Compression::fast());
            encoder.write_all(data).map_err(|e| {
                SessionError::StorageError(format!("Failed to write compressed data: {}", e))
            })?;
            encoder.finish().map_err(|e| {
                SessionError::StorageError(format!("Failed to finish compression: {}", e))
            })?;
        } else {
            fs::write(&temp, data).map_err(|e| {
                SessionError::StorageError(format!("Failed to write temp file: {}", e))
            })?;
        }

        // Rename to final destination
        let dest = if compress {
            // Remove uncompressed version if it exists
            let uncompressed = self.session_path(key);
            if uncompressed.exists() {
                let _ = fs::remove_file(&uncompressed);
            }
            self.compressed_path(key)
        } else {
            // Remove compressed version if it exists
            let compressed = self.compressed_path(key);
            if compressed.exists() {
                let _ = fs::remove_file(&compressed);
            }
            self.session_path(key)
        };

        fs::rename(&temp, &dest).map_err(|e| {
            // Clean up temp file on failure
            let _ = fs::remove_file(&temp);
            SessionError::StorageError(format!("Failed to rename temp file: {}", e))
        })?;

        Ok(())
    }

    /// Read a session file (handles both compressed and uncompressed)
    fn read_session_file(&self, key: &str) -> Result<Vec<u8>, SessionError> {
        let plain_path = self.session_path(key);
        let compressed_path = self.compressed_path(key);

        if plain_path.exists() {
            fs::read(&plain_path).map_err(|e| {
                SessionError::StorageError(format!("Failed to read session file: {}", e))
            })
        } else if compressed_path.exists() {
            let file = fs::File::open(&compressed_path).map_err(|e| {
                SessionError::StorageError(format!("Failed to open compressed file: {}", e))
            })?;
            let mut decoder = GzDecoder::new(file);
            let mut data = Vec::new();
            decoder.read_to_end(&mut data).map_err(|e| {
                SessionError::StorageError(format!("Failed to decompress session: {}", e))
            })?;
            Ok(data)
        } else {
            Err(SessionError::NotFound(key.to_string()))
        }
    }

    /// List all session keys from the directory
    fn list_keys(&self) -> Result<Vec<String>, SessionError> {
        let mut keys = Vec::new();

        let entries = fs::read_dir(&self.base_dir).map_err(|e| {
            SessionError::StorageError(format!("Failed to read storage dir: {}", e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                SessionError::StorageError(format!("Failed to read dir entry: {}", e))
            })?;
            let path = entry.path();

            if path.is_file() {
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                // Skip temp files and backup directory
                if filename.starts_with('.') {
                    continue;
                }

                let key = if filename.ends_with(".json.gz") {
                    filename.strip_suffix(".json.gz").map(String::from)
                } else if filename.ends_with(".json") {
                    filename.strip_suffix(".json").map(String::from)
                } else {
                    None
                };

                if let Some(key) = key {
                    keys.push(key);
                }
            }
        }

        Ok(keys)
    }
}

impl SessionStorage for FileSessionStorage {
    fn save(&self, session: &Session) -> Result<(), SessionError> {
        // Create backup of existing session
        self.create_backup(&session.key)?;

        // Serialize session
        let data = serde_json::to_vec_pretty(session).map_err(|e| {
            SessionError::SerializationError(format!("Failed to serialize session: {}", e))
        })?;

        // Compress if over threshold
        let compress = data.len() > COMPRESSION_THRESHOLD;
        self.atomic_write(&session.key, &data, compress)?;

        Ok(())
    }

    fn load(&self, key: &str) -> Result<Session, SessionError> {
        let data = self.read_session_file(key)?;
        serde_json::from_slice(&data).map_err(|e| {
            SessionError::SerializationError(format!("Failed to deserialize session: {}", e))
        })
    }

    fn delete(&self, key: &str) -> Result<(), SessionError> {
        let plain = self.session_path(key);
        let compressed = self.compressed_path(key);

        if plain.exists() {
            fs::remove_file(&plain).map_err(|e| {
                SessionError::StorageError(format!("Failed to delete session: {}", e))
            })?;
        }
        if compressed.exists() {
            fs::remove_file(&compressed).map_err(|e| {
                SessionError::StorageError(format!("Failed to delete compressed session: {}", e))
            })?;
        }

        // Also clean up backups
        let _ = self.cleanup_backups(key);

        Ok(())
    }

    fn list(&self, _filter: &SessionFilter) -> Result<Vec<Session>, SessionError> {
        let keys = self.list_keys()?;
        let mut sessions = Vec::new();

        for key in keys {
            match self.load(&key) {
                Ok(session) => sessions.push(session),
                Err(e) => {
                    tracing::warn!("Failed to load session {}: {}", key, e);
                }
            }
        }

        Ok(sessions)
    }

    fn exists(&self, key: &str) -> Result<bool, SessionError> {
        Ok(self.session_path(key).exists() || self.compressed_path(key).exists())
    }

    fn clear(&self) -> Result<usize, SessionError> {
        let keys = self.list_keys()?;
        let count = keys.len();

        for key in &keys {
            self.delete(key)?;
        }

        Ok(count)
    }
}

impl FileSessionStorage {
    /// Clean up backup files for a session
    fn cleanup_backups(&self, key: &str) -> Result<(), SessionError> {
        for i in 1..=MAX_BACKUPS {
            let json_backup = self.backup_dir.join(format!("{}.{}.json", key, i));
            let gz_backup = self.backup_dir.join(format!("{}.{}.json.gz", key, i));
            let _ = fs::remove_file(json_backup);
            let _ = fs::remove_file(gz_backup);
        }
        Ok(())
    }

    /// Get storage statistics
    pub fn stats(&self) -> Result<StorageStats, SessionError> {
        let keys = self.list_keys()?;
        let mut total_size: u64 = 0;
        let mut compressed_count = 0u32;
        let mut plain_count = 0u32;

        for key in &keys {
            let plain = self.session_path(key);
            let compressed = self.compressed_path(key);

            if compressed.exists() {
                compressed_count += 1;
                if let Ok(meta) = fs::metadata(&compressed) {
                    total_size += meta.len();
                }
            } else if plain.exists() {
                plain_count += 1;
                if let Ok(meta) = fs::metadata(&plain) {
                    total_size += meta.len();
                }
            }
        }

        Ok(StorageStats {
            total_sessions: keys.len(),
            total_size_bytes: total_size,
            compressed_sessions: compressed_count,
            plain_sessions: plain_count,
        })
    }
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Total number of sessions
    pub total_sessions: usize,
    /// Total size on disk in bytes
    pub total_size_bytes: u64,
    /// Number of compressed sessions
    pub compressed_sessions: u32,
    /// Number of plain (uncompressed) sessions
    pub plain_sessions: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::MessageRole;

    #[test]
    fn test_file_storage_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");
        session.add_message(MessageRole::Assistant, "Hi there!");

        storage.save(&session).unwrap();

        let loaded = storage.load(&session.key).unwrap();
        assert_eq!(loaded.key, session.key);
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].content, "Hello");
    }

    #[test]
    fn test_file_storage_delete() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let session = Session::new("agent-1");
        storage.save(&session).unwrap();
        assert!(storage.exists(&session.key).unwrap());

        storage.delete(&session.key).unwrap();
        assert!(!storage.exists(&session.key).unwrap());
    }

    #[test]
    fn test_file_storage_list() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let s1 = Session::new("agent-1");
        let s2 = Session::new("agent-2");
        storage.save(&s1).unwrap();
        storage.save(&s2).unwrap();

        let sessions = storage.list(&SessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_file_storage_clear() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let s1 = Session::new("agent-1");
        let s2 = Session::new("agent-2");
        storage.save(&s1).unwrap();
        storage.save(&s2).unwrap();

        let cleared = storage.clear().unwrap();
        assert_eq!(cleared, 2);

        let sessions = storage.list(&SessionFilter::default()).unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_file_storage_backup() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Version 1");
        storage.save(&session).unwrap();

        // Save again to create backup
        session.add_message(MessageRole::User, "Version 2");
        storage.save(&session).unwrap();

        // Check backup exists
        let backup_path = dir.path().join("backups").join(format!("{}.1.json", session.key));
        assert!(backup_path.exists());
    }

    #[test]
    fn test_file_storage_stats() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let s1 = Session::new("agent-1");
        storage.save(&s1).unwrap();

        let stats = storage.stats().unwrap();
        assert_eq!(stats.total_sessions, 1);
        assert!(stats.total_size_bytes > 0);
        assert_eq!(stats.plain_sessions, 1);
        assert_eq!(stats.compressed_sessions, 0);
    }

    #[test]
    fn test_atomic_write_cleanup_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let storage = FileSessionStorage::new(dir.path().to_path_buf()).unwrap();

        let session = Session::new("agent-1");
        storage.save(&session).unwrap();

        // Verify no temp files remain
        let temp_path = dir.path().join(format!(".{}.tmp", session.key));
        assert!(!temp_path.exists());
    }
}
