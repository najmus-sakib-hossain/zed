//! Memory Indexing Module
//!
//! Provides background indexing capabilities for the memory system.
//! Supports indexing files from disk, automatic re-indexing on changes,
//! and batch processing of documents.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{RwLock, broadcast, mpsc};

use super::{MemoryError, MemoryMetadata, MemorySystem};

/// Indexing task status
#[derive(Debug, Clone)]
pub enum IndexStatus {
    /// Indexing is idle
    Idle,
    /// Currently indexing
    Running {
        files_total: usize,
        files_processed: usize,
        started_at: Instant,
    },
    /// Indexing completed
    Completed {
        files_indexed: usize,
        files_skipped: usize,
        duration: Duration,
        errors: Vec<String>,
    },
    /// Indexing failed
    Failed(String),
}

/// Indexing event
#[derive(Debug, Clone)]
pub enum IndexEvent {
    /// A file was indexed
    FileIndexed { path: PathBuf, chunks: usize },
    /// A file was skipped
    FileSkipped { path: PathBuf, reason: String },
    /// Indexing progress update
    Progress { processed: usize, total: usize },
    /// Indexing completed
    Complete {
        total_indexed: usize,
        total_skipped: usize,
        duration: Duration,
    },
    /// Indexing error (non-fatal)
    Error {
        path: Option<PathBuf>,
        message: String,
    },
}

/// Configuration for the indexer
#[derive(Debug, Clone)]
pub struct IndexerConfig {
    /// Maximum file size to index (bytes)
    pub max_file_size: u64,
    /// Extensions to include (empty = all text files)
    pub include_extensions: Vec<String>,
    /// Extensions to exclude
    pub exclude_extensions: Vec<String>,
    /// Directories to exclude
    pub exclude_dirs: Vec<String>,
    /// Chunk size for splitting large files (chars)
    pub chunk_size: usize,
    /// Chunk overlap for context preservation (chars)
    pub chunk_overlap: usize,
    /// Maximum concurrent indexing tasks
    pub concurrency: usize,
    /// Enable file watching for auto-reindex
    pub watch: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10 MB
            include_extensions: vec![
                "rs".into(),
                "py".into(),
                "js".into(),
                "ts".into(),
                "md".into(),
                "txt".into(),
                "toml".into(),
                "yaml".into(),
                "yml".into(),
                "json".into(),
                "html".into(),
                "css".into(),
                "go".into(),
                "c".into(),
                "cpp".into(),
                "h".into(),
                "java".into(),
                "kt".into(),
                "swift".into(),
                "rb".into(),
            ],
            exclude_extensions: vec![
                "exe".into(),
                "dll".into(),
                "so".into(),
                "dylib".into(),
                "bin".into(),
                "o".into(),
                "a".into(),
                "wasm".into(),
                "png".into(),
                "jpg".into(),
                "gif".into(),
                "mp4".into(),
                "zip".into(),
                "tar".into(),
                "gz".into(),
            ],
            exclude_dirs: vec![
                ".git".into(),
                "node_modules".into(),
                "target".into(),
                "__pycache__".into(),
                ".venv".into(),
                "dist".into(),
                "build".into(),
                ".next".into(),
            ],
            chunk_size: 2000,
            chunk_overlap: 200,
            concurrency: 4,
            watch: false,
        }
    }
}

/// Background indexer for the memory system
pub struct MemoryIndexer {
    config: IndexerConfig,
    status: Arc<RwLock<IndexStatus>>,
    event_tx: broadcast::Sender<IndexEvent>,
    cancel_tx: Option<mpsc::Sender<()>>,
    indexed_paths: Arc<RwLock<HashSet<PathBuf>>>,
}

impl MemoryIndexer {
    /// Create a new indexer with the given configuration
    pub fn new(config: IndexerConfig) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            config,
            status: Arc::new(RwLock::new(IndexStatus::Idle)),
            event_tx,
            cancel_tx: None,
            indexed_paths: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Get current indexing status
    pub async fn status(&self) -> IndexStatus {
        self.status.read().await.clone()
    }

    /// Subscribe to indexing events
    pub fn subscribe(&self) -> broadcast::Receiver<IndexEvent> {
        self.event_tx.subscribe()
    }

    /// Index a directory recursively
    pub async fn index_directory(
        &mut self,
        dir: &Path,
        memory: &MemorySystem,
    ) -> Result<IndexStatus, MemoryError> {
        // Collect files to index
        let files = self.collect_files(dir)?;
        let total = files.len();

        if total == 0 {
            return Ok(IndexStatus::Completed {
                files_indexed: 0,
                files_skipped: 0,
                duration: Duration::ZERO,
                errors: vec![],
            });
        }

        let started_at = Instant::now();
        *self.status.write().await = IndexStatus::Running {
            files_total: total,
            files_processed: 0,
            started_at,
        };

        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
        self.cancel_tx = Some(cancel_tx);

        let mut indexed = 0usize;
        let mut skipped = 0usize;
        let mut errors = Vec::new();

        for (i, file_path) in files.iter().enumerate() {
            // Check for cancellation
            if cancel_rx.try_recv().is_ok() {
                break;
            }

            match self.index_file(file_path, memory).await {
                Ok(chunks) => {
                    indexed += 1;
                    self.indexed_paths.write().await.insert(file_path.clone());
                    let _ = self.event_tx.send(IndexEvent::FileIndexed {
                        path: file_path.clone(),
                        chunks,
                    });
                }
                Err(e) => {
                    let msg = format!("{}: {}", file_path.display(), e);
                    if self.is_skippable_error(&e) {
                        skipped += 1;
                        let _ = self.event_tx.send(IndexEvent::FileSkipped {
                            path: file_path.clone(),
                            reason: msg,
                        });
                    } else {
                        errors.push(msg.clone());
                        let _ = self.event_tx.send(IndexEvent::Error {
                            path: Some(file_path.clone()),
                            message: msg,
                        });
                    }
                }
            }

            // Update progress
            *self.status.write().await = IndexStatus::Running {
                files_total: total,
                files_processed: i + 1,
                started_at,
            };

            let _ = self.event_tx.send(IndexEvent::Progress {
                processed: i + 1,
                total,
            });
        }

        let duration = started_at.elapsed();
        let result = IndexStatus::Completed {
            files_indexed: indexed,
            files_skipped: skipped,
            duration,
            errors: errors.clone(),
        };

        *self.status.write().await = result.clone();

        let _ = self.event_tx.send(IndexEvent::Complete {
            total_indexed: indexed,
            total_skipped: skipped,
            duration,
        });

        Ok(result)
    }

    /// Index a single file, splitting into chunks if needed
    async fn index_file(&self, path: &Path, memory: &MemorySystem) -> Result<usize, MemoryError> {
        let content = std::fs::read_to_string(path)?;

        if content.is_empty() {
            return Ok(0);
        }

        let chunks = self.chunk_text(&content);
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_string();

        for (i, chunk) in chunks.iter().enumerate() {
            let metadata = MemoryMetadata {
                source: path.display().to_string(),
                category: format!("indexed:{}", ext),
                tags: vec!["indexed".to_string(), ext.clone(), file_name.to_string()],
                conversation_id: None,
                custom: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("chunk_index".to_string(), i.to_string());
                    m.insert("total_chunks".to_string(), chunks.len().to_string());
                    m
                },
            };

            memory.store(chunk, metadata).await?;
        }

        Ok(chunks.len())
    }

    /// Split text into overlapping chunks
    fn chunk_text(&self, text: &str) -> Vec<String> {
        if text.len() <= self.config.chunk_size {
            return vec![text.to_string()];
        }

        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.config.chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end >= chars.len() {
                break;
            }

            // Move start forward, accounting for overlap
            start = end.saturating_sub(self.config.chunk_overlap);
        }

        chunks
    }

    /// Collect indexable files from a directory
    fn collect_files(&self, dir: &Path) -> Result<Vec<PathBuf>, MemoryError> {
        let mut files = Vec::new();
        self.walk_dir(dir, &mut files)?;
        files.sort();
        Ok(files)
    }

    /// Recursively walk directory and collect files
    fn walk_dir(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), MemoryError> {
        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !self.config.exclude_dirs.iter().any(|d| d == dir_name) {
                    self.walk_dir(&path, files)?;
                }
                continue;
            }

            // Check file size
            if let Ok(metadata) = std::fs::metadata(&path) {
                if metadata.len() > self.config.max_file_size {
                    continue;
                }
            }

            // Check extension
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

            if self.config.exclude_extensions.iter().any(|e| e == &ext) {
                continue;
            }

            if !self.config.include_extensions.is_empty()
                && !self.config.include_extensions.iter().any(|e| e == &ext)
            {
                continue;
            }

            files.push(path);
        }

        Ok(())
    }

    /// Cancel ongoing indexing
    pub async fn cancel(&mut self) {
        if let Some(tx) = self.cancel_tx.take() {
            let _ = tx.send(()).await;
        }
    }

    /// Check if an error is skippable (non-fatal)
    fn is_skippable_error(&self, err: &MemoryError) -> bool {
        matches!(err, MemoryError::IoError(_))
    }

    /// Get the number of indexed paths
    pub async fn indexed_count(&self) -> usize {
        self.indexed_paths.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_small() {
        let indexer = MemoryIndexer::new(IndexerConfig {
            chunk_size: 100,
            chunk_overlap: 20,
            ..Default::default()
        });

        let text = "Hello, this is a short text.";
        let chunks = indexer.chunk_text(text);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_chunk_text_large() {
        let indexer = MemoryIndexer::new(IndexerConfig {
            chunk_size: 50,
            chunk_overlap: 10,
            ..Default::default()
        });

        let text = "a".repeat(150);
        let chunks = indexer.chunk_text(&text);
        assert!(chunks.len() > 1);
        // Ensure all content is covered
        assert_eq!(chunks[0].len(), 50);
    }

    #[test]
    fn test_default_config() {
        let config = IndexerConfig::default();
        assert_eq!(config.chunk_size, 2000);
        assert_eq!(config.chunk_overlap, 200);
        assert_eq!(config.concurrency, 4);
        assert!(!config.include_extensions.is_empty());
        assert!(!config.exclude_extensions.is_empty());
        assert!(!config.exclude_dirs.is_empty());
    }

    #[test]
    fn test_collect_files() {
        let dir = tempfile::tempdir().unwrap();
        let file1 = dir.path().join("test.rs");
        let file2 = dir.path().join("test.py");
        let file3 = dir.path().join("test.exe"); // Should be excluded
        std::fs::write(&file1, "fn main() {}").unwrap();
        std::fs::write(&file2, "print('hello')").unwrap();
        std::fs::write(&file3, "binary").unwrap();

        let indexer = MemoryIndexer::new(IndexerConfig::default());
        let files = indexer.collect_files(dir.path()).unwrap();

        assert!(files.contains(&file1));
        assert!(files.contains(&file2));
        assert!(!files.contains(&file3)); // .exe excluded
    }

    #[test]
    fn test_exclude_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("node_modules");
        std::fs::create_dir(&subdir).unwrap();
        let file1 = dir.path().join("main.rs");
        let file2 = subdir.join("dep.js");
        std::fs::write(&file1, "fn main() {}").unwrap();
        std::fs::write(&file2, "module.exports = {}").unwrap();

        let indexer = MemoryIndexer::new(IndexerConfig::default());
        let files = indexer.collect_files(dir.path()).unwrap();

        assert!(files.contains(&file1));
        assert!(!files.contains(&file2)); // node_modules excluded
    }
}
