//! Bun.file() and Bun.write() file operations.
//!
//! High-performance file I/O using memory-mapped I/O for large files,
//! targeting 1 GB/s read throughput.

use crate::error::{BunError, BunResult};
use bytes::Bytes;
use std::path::{Path, PathBuf};
use tokio::fs as async_fs;

/// Threshold for using memory-mapped I/O (1MB).
const MMAP_THRESHOLD: u64 = 1_048_576;

/// BunFile handle with lazy loading.
///
/// Provides a file handle compatible with Bun.file() API.
pub struct BunFile {
    path: PathBuf,
    start: u64,
    end: Option<u64>,
}

impl BunFile {
    /// Create a new file handle.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            start: 0,
            end: None,
        }
    }

    /// Get the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read as text.
    pub async fn text(&self) -> BunResult<String> {
        let bytes = self.array_buffer().await?;
        String::from_utf8(bytes).map_err(|e| BunError::File(format!("Invalid UTF-8: {}", e)))
    }

    /// Parse as JSON.
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> BunResult<T> {
        let content = self.text().await?;
        serde_json::from_str(&content)
            .map_err(|e| BunError::File(format!("JSON parse error: {}", e)))
    }

    /// Read as bytes (ArrayBuffer equivalent).
    pub async fn array_buffer(&self) -> BunResult<Vec<u8>> {
        let metadata = async_fs::metadata(&self.path)
            .await
            .map_err(|e| BunError::File(format!("Failed to get metadata: {}", e)))?;

        let file_size = metadata.len();

        // Determine actual read range
        let start = self.start;
        let end = self.end.unwrap_or(file_size).min(file_size);

        if start >= end {
            return Ok(Vec::new());
        }

        let read_size = end - start;

        // Use memory-mapped I/O for large files
        if read_size > MMAP_THRESHOLD {
            self.read_mmap(start, end).await
        } else {
            self.read_standard(start, end).await
        }
    }

    /// Read using memory-mapped I/O.
    async fn read_mmap(&self, start: u64, end: u64) -> BunResult<Vec<u8>> {
        let path = self.path.clone();
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path)
                .map_err(|e| BunError::File(format!("Failed to open file: {}", e)))?;
            let mmap = unsafe {
                memmap2::Mmap::map(&file)
                    .map_err(|e| BunError::File(format!("Failed to mmap: {}", e)))?
            };

            let start = start as usize;
            let end = end as usize;
            Ok(mmap[start..end].to_vec())
        })
        .await
        .map_err(|e| BunError::File(format!("Task join error: {}", e)))?
    }

    /// Read using standard I/O.
    async fn read_standard(&self, start: u64, end: u64) -> BunResult<Vec<u8>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        let mut file = async_fs::File::open(&self.path)
            .await
            .map_err(|e| BunError::File(format!("Failed to open file: {}", e)))?;

        if start > 0 {
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(|e| BunError::File(format!("Failed to seek: {}", e)))?;
        }

        let read_size = (end - start) as usize;
        let mut buffer = vec![0u8; read_size];
        file.read_exact(&mut buffer)
            .await
            .map_err(|e| BunError::File(format!("Failed to read: {}", e)))?;

        Ok(buffer)
    }

    /// Get a readable stream.
    pub fn stream(&self) -> FileStream {
        FileStream {
            path: self.path.clone(),
            start: self.start,
            end: self.end,
            chunk_size: 64 * 1024, // 64KB chunks
        }
    }

    /// Get file size.
    pub async fn size(&self) -> BunResult<u64> {
        let metadata = async_fs::metadata(&self.path)
            .await
            .map_err(|e| BunError::File(format!("Failed to get metadata: {}", e)))?;

        let file_size = metadata.len();
        let start = self.start;
        let end = self.end.unwrap_or(file_size).min(file_size);

        Ok(end.saturating_sub(start))
    }

    /// Get MIME type based on extension.
    pub fn type_(&self) -> &str {
        match self.path.extension().and_then(|e| e.to_str()) {
            Some("txt") => "text/plain",
            Some("html") | Some("htm") => "text/html",
            Some("css") => "text/css",
            Some("js") | Some("mjs") => "application/javascript",
            Some("ts") | Some("tsx") => "application/typescript",
            Some("json") => "application/json",
            Some("xml") => "application/xml",
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("pdf") => "application/pdf",
            Some("zip") => "application/zip",
            Some("gz") | Some("gzip") => "application/gzip",
            Some("tar") => "application/x-tar",
            Some("mp3") => "audio/mpeg",
            Some("mp4") => "video/mp4",
            Some("webm") => "video/webm",
            Some("wasm") => "application/wasm",
            _ => "application/octet-stream",
        }
    }

    /// Slice file (lazy, zero-copy).
    ///
    /// Returns a new BunFile handle that represents a slice of the original file.
    pub fn slice(&self, start: u64, end: Option<u64>) -> BunFile {
        let new_start = self.start + start;
        let new_end = match (self.end, end) {
            (Some(self_end), Some(slice_end)) => Some((self.start + slice_end).min(self_end)),
            (Some(self_end), None) => Some(self_end),
            (None, Some(slice_end)) => Some(self.start + slice_end),
            (None, None) => None,
        };

        BunFile {
            path: self.path.clone(),
            start: new_start,
            end: new_end,
        }
    }

    /// Check if file exists.
    pub async fn exists(&self) -> bool {
        async_fs::metadata(&self.path).await.is_ok()
    }
}

/// File stream for reading file in chunks.
pub struct FileStream {
    path: PathBuf,
    start: u64,
    end: Option<u64>,
    chunk_size: usize,
}

impl FileStream {
    /// Set chunk size.
    pub fn chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Read all chunks.
    pub async fn collect(&self) -> BunResult<Vec<Bytes>> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt};

        let mut file = async_fs::File::open(&self.path)
            .await
            .map_err(|e| BunError::File(format!("Failed to open file: {}", e)))?;

        let metadata = file
            .metadata()
            .await
            .map_err(|e| BunError::File(format!("Failed to get metadata: {}", e)))?;

        let file_size = metadata.len();
        let start = self.start;
        let end = self.end.unwrap_or(file_size).min(file_size);

        if start > 0 {
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(|e| BunError::File(format!("Failed to seek: {}", e)))?;
        }

        let mut chunks = Vec::new();
        let mut remaining = (end - start) as usize;

        while remaining > 0 {
            let to_read = remaining.min(self.chunk_size);
            let mut buffer = vec![0u8; to_read];
            let read = file
                .read(&mut buffer)
                .await
                .map_err(|e| BunError::File(format!("Failed to read: {}", e)))?;

            if read == 0 {
                break;
            }

            buffer.truncate(read);
            chunks.push(Bytes::from(buffer));
            remaining -= read;
        }

        Ok(chunks)
    }
}

/// Data that can be written to a file.
pub enum WriteData {
    /// String data
    String(String),
    /// Byte data
    Bytes(Vec<u8>),
    /// Bytes wrapper
    BytesWrapper(Bytes),
    /// Another file (copy)
    File(BunFile),
}

impl From<String> for WriteData {
    fn from(s: String) -> Self {
        WriteData::String(s)
    }
}

impl From<&str> for WriteData {
    fn from(s: &str) -> Self {
        WriteData::String(s.to_string())
    }
}

impl From<Vec<u8>> for WriteData {
    fn from(b: Vec<u8>) -> Self {
        WriteData::Bytes(b)
    }
}

impl From<&[u8]> for WriteData {
    fn from(b: &[u8]) -> Self {
        WriteData::Bytes(b.to_vec())
    }
}

impl From<Bytes> for WriteData {
    fn from(b: Bytes) -> Self {
        WriteData::BytesWrapper(b)
    }
}

impl From<BunFile> for WriteData {
    fn from(f: BunFile) -> Self {
        WriteData::File(f)
    }
}

/// Write data to file (Bun.write()).
///
/// # Arguments
/// * `path` - The file path to write to
/// * `data` - The data to write (string, bytes, or another file)
///
/// # Returns
/// The number of bytes written.
pub async fn write(path: impl AsRef<Path>, data: impl Into<WriteData>) -> BunResult<usize> {
    let path = path.as_ref();
    let data = data.into();

    let bytes = match data {
        WriteData::String(s) => s.into_bytes(),
        WriteData::Bytes(b) => b,
        WriteData::BytesWrapper(b) => b.to_vec(),
        WriteData::File(f) => f.array_buffer().await?,
    };

    let len = bytes.len();
    async_fs::write(path, &bytes)
        .await
        .map_err(|e| BunError::File(format!("Failed to write file: {}", e)))?;

    Ok(len)
}

/// Create a file handle (Bun.file()).
pub fn file(path: impl AsRef<Path>) -> BunFile {
    BunFile::new(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_write_and_read_text() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        write(&path, "hello world").await.unwrap();

        let file = BunFile::new(&path);
        let content = file.text().await.unwrap();

        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn test_write_and_read_bytes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.bin");

        let data = vec![1u8, 2, 3, 4, 5];
        write(&path, data.clone()).await.unwrap();

        let file = BunFile::new(&path);
        let content = file.array_buffer().await.unwrap();

        assert_eq!(content, data);
    }

    #[tokio::test]
    async fn test_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");

        write(&path, r#"{"name": "test", "value": 42}"#).await.unwrap();

        let file = BunFile::new(&path);
        let value: serde_json::Value = file.json().await.unwrap();

        assert_eq!(value["name"], "test");
        assert_eq!(value["value"], 42);
    }

    #[tokio::test]
    async fn test_size() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        write(&path, "hello").await.unwrap();

        let file = BunFile::new(&path);
        let size = file.size().await.unwrap();

        assert_eq!(size, 5);
    }

    #[tokio::test]
    async fn test_slice() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        write(&path, "hello world").await.unwrap();

        let file = BunFile::new(&path);
        let sliced = file.slice(0, Some(5));
        let content = sliced.text().await.unwrap();

        assert_eq!(content, "hello");
    }

    #[tokio::test]
    async fn test_slice_middle() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        write(&path, "hello world").await.unwrap();

        let file = BunFile::new(&path);
        let sliced = file.slice(6, Some(11));
        let content = sliced.text().await.unwrap();

        assert_eq!(content, "world");
    }

    #[test]
    fn test_mime_types() {
        assert_eq!(BunFile::new("test.txt").type_(), "text/plain");
        assert_eq!(BunFile::new("test.html").type_(), "text/html");
        assert_eq!(BunFile::new("test.json").type_(), "application/json");
        assert_eq!(BunFile::new("test.png").type_(), "image/png");
        assert_eq!(BunFile::new("test.unknown").type_(), "application/octet-stream");
    }

    #[tokio::test]
    async fn test_exists() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        let file = BunFile::new(&path);
        assert!(!file.exists().await);

        write(&path, "test").await.unwrap();
        assert!(file.exists().await);
    }

    #[tokio::test]
    async fn test_stream() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");

        write(&path, "hello world").await.unwrap();

        let file = BunFile::new(&path);
        let stream = file.stream().chunk_size(5);
        let chunks = stream.collect().await.unwrap();

        assert!(!chunks.is_empty());
        let total: Vec<u8> = chunks.iter().flat_map(|c| c.to_vec()).collect();
        assert_eq!(String::from_utf8(total).unwrap(), "hello world");
    }
}
