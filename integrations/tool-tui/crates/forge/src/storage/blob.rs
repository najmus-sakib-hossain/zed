/// Blob Storage System for Cloudflare R2
///
/// This module provides efficient binary blob storage using FlatBuffers for serialization.
/// All file content and metadata are stored as binary blobs in R2, making it faster and
/// more cost-effective than traditional Git storage.
///
/// Features:
/// - Platform-native I/O for optimal performance
/// - Memory-mapped I/O for large blobs (>1MB)
/// - Parallel compression for files >100KB
/// - SHA-256 integrity verification
use anyhow::{Context, Result, anyhow};
use memmap2::Mmap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

use crate::platform_io::{PlatformIO, create_platform_io};

/// Threshold for using memory-mapped I/O (1MB)
const MMAP_THRESHOLD: u64 = 1024 * 1024;

/// Threshold for using parallel compression (100KB)
const PARALLEL_COMPRESSION_THRESHOLD: usize = 100 * 1024;

/// Blob metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobMetadata {
    /// SHA-256 hash of the blob content
    pub hash: String,

    /// Original file path
    pub path: String,

    /// Blob size in bytes
    pub size: u64,

    /// Original (uncompressed) size in bytes, if compressed
    pub original_size: Option<u64>,

    /// MIME type
    pub mime_type: String,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Compression algorithm used (if any)
    pub compression: Option<String>,
}

/// Binary blob representation
#[derive(Debug)]
pub struct Blob {
    pub metadata: BlobMetadata,
    pub content: Vec<u8>,
}

impl Blob {
    /// Create a new blob from file content using platform-native I/O
    pub async fn from_file(path: &Path) -> Result<Self> {
        Self::from_file_with_io(path, create_platform_io()).await
    }

    /// Create a new blob from file content using specified I/O backend
    pub async fn from_file_with_io(path: &Path, io: Arc<dyn PlatformIO>) -> Result<Self> {
        let metadata = std::fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;
        let file_size = metadata.len();

        // Use memory-mapped I/O for large files
        let content = if file_size > MMAP_THRESHOLD {
            Self::read_with_mmap(path)?
        } else {
            io.read_all(path).await?
        };

        let hash = compute_hash(&content);
        let size = content.len() as u64;
        let mime_type = detect_mime_type(path);

        let metadata = BlobMetadata {
            hash: hash.clone(),
            path: path.display().to_string(),
            size,
            original_size: None,
            mime_type,
            created_at: chrono::Utc::now(),
            compression: None,
        };

        Ok(Self { metadata, content })
    }

    /// Read file using memory-mapped I/O for large files
    fn read_with_mmap(path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path)
            .with_context(|| format!("Failed to open file for mmap: {}", path.display()))?;

        // Safety: We're only reading the file, and the file won't be modified
        // while we're reading it (single-threaded access pattern)
        let mmap = unsafe {
            Mmap::map(&file).with_context(|| format!("Failed to mmap file: {}", path.display()))?
        };

        Ok(mmap.to_vec())
    }

    /// Create blob from raw content
    pub fn from_content(path: &str, content: Vec<u8>) -> Self {
        let hash = compute_hash(&content);
        let size = content.len() as u64;
        let mime_type = detect_mime_type_from_path(path);

        let metadata = BlobMetadata {
            hash: hash.clone(),
            path: path.to_string(),
            size,
            original_size: None,
            mime_type,
            created_at: chrono::Utc::now(),
            compression: None,
        };

        Self { metadata, content }
    }

    /// Serialize blob to binary format
    pub fn to_binary(&self) -> Result<Vec<u8>> {
        // Simple binary format:
        // [metadata_len: u32][metadata_json][content]

        let metadata_json = serde_json::to_vec(&self.metadata)?;
        let metadata_len = metadata_json.len() as u32;

        let mut binary = Vec::with_capacity(4 + metadata_json.len() + self.content.len());
        binary.extend_from_slice(&metadata_len.to_le_bytes());
        binary.extend_from_slice(&metadata_json);
        binary.extend_from_slice(&self.content);

        Ok(binary)
    }

    /// Deserialize blob from binary format
    pub fn from_binary(binary: &[u8]) -> Result<Self> {
        if binary.len() < 4 {
            anyhow::bail!("Invalid blob: too short");
        }

        let metadata_len =
            u32::from_le_bytes([binary[0], binary[1], binary[2], binary[3]]) as usize;

        if binary.len() < 4 + metadata_len {
            anyhow::bail!("Invalid blob: metadata truncated");
        }

        let metadata_json = &binary[4..4 + metadata_len];
        let metadata: BlobMetadata = serde_json::from_slice(metadata_json)?;

        let content = binary[4 + metadata_len..].to_vec();

        Ok(Self { metadata, content })
    }

    /// Compress blob content using LZ4
    /// Uses parallel compression for files >100KB
    pub fn compress(&mut self) -> Result<()> {
        if self.metadata.compression.is_some() {
            return Ok(()); // Already compressed
        }

        let compressed = if self.content.len() > PARALLEL_COMPRESSION_THRESHOLD {
            // Use parallel compression for large files
            self.compress_parallel()?
        } else {
            lz4::block::compress(&self.content, None, false)?
        };

        // Only use compression if it actually reduces size
        if compressed.len() < self.content.len() {
            // Remember original size so we can safely decompress later
            self.metadata.original_size = Some(self.metadata.size);
            self.content = compressed;
            self.metadata.compression = Some("lz4".to_string());
            self.metadata.size = self.content.len() as u64;
        }

        Ok(())
    }

    /// Compress content in parallel chunks
    fn compress_parallel(&self) -> Result<Vec<u8>> {
        const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks

        let chunks: Vec<&[u8]> = self.content.chunks(CHUNK_SIZE).collect();

        // Compress chunks in parallel
        let compressed_chunks: Vec<Vec<u8>> = chunks
            .par_iter()
            .map(|chunk| lz4::block::compress(chunk, None, false))
            .collect::<Result<Vec<_>, _>>()?;

        // Combine compressed chunks with length prefixes
        let total_size: usize = compressed_chunks.iter().map(|c| 4 + c.len()).sum();
        let mut result = Vec::with_capacity(total_size + 4);

        // Write number of chunks
        result.extend_from_slice(&(compressed_chunks.len() as u32).to_le_bytes());

        // Write each chunk with its length
        for chunk in compressed_chunks {
            result.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }

    /// Decompress blob content
    pub fn decompress(&mut self) -> Result<()> {
        if self.metadata.compression.is_none() {
            return Ok(()); // Not compressed
        }

        // Check if this is parallel-compressed data
        let decompressed = if self.content.len() >= 4 {
            let num_chunks = u32::from_le_bytes([
                self.content[0],
                self.content[1],
                self.content[2],
                self.content[3],
            ]) as usize;

            // Heuristic: if num_chunks is reasonable, try parallel decompression
            if num_chunks > 0 && num_chunks < 10000 {
                match self.decompress_parallel() {
                    Ok(data) => data,
                    Err(_) => {
                        // Fall back to regular decompression
                        let original_size =
                            self.metadata.original_size.unwrap_or(self.metadata.size) as i32;
                        lz4::block::decompress(&self.content, Some(original_size))?
                    }
                }
            } else {
                let original_size =
                    self.metadata.original_size.unwrap_or(self.metadata.size) as i32;
                lz4::block::decompress(&self.content, Some(original_size))?
            }
        } else {
            let original_size = self.metadata.original_size.unwrap_or(self.metadata.size) as i32;
            lz4::block::decompress(&self.content, Some(original_size))?
        };

        self.content = decompressed;
        self.metadata.compression = None;
        self.metadata.original_size = None;
        self.metadata.size = self.content.len() as u64;

        Ok(())
    }

    /// Decompress parallel-compressed content
    fn decompress_parallel(&self) -> Result<Vec<u8>> {
        if self.content.len() < 4 {
            anyhow::bail!("Invalid parallel compressed data");
        }

        let num_chunks = u32::from_le_bytes([
            self.content[0],
            self.content[1],
            self.content[2],
            self.content[3],
        ]) as usize;

        let mut offset = 4;
        let mut chunks = Vec::with_capacity(num_chunks);

        for _ in 0..num_chunks {
            if offset + 4 > self.content.len() {
                anyhow::bail!("Invalid parallel compressed data: truncated");
            }

            let chunk_len = u32::from_le_bytes([
                self.content[offset],
                self.content[offset + 1],
                self.content[offset + 2],
                self.content[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + chunk_len > self.content.len() {
                anyhow::bail!("Invalid parallel compressed data: chunk truncated");
            }

            chunks.push(&self.content[offset..offset + chunk_len]);
            offset += chunk_len;
        }

        // Decompress chunks in parallel
        let decompressed_chunks: Vec<Vec<u8>> = chunks
            .par_iter()
            .map(|chunk| {
                // Use a reasonable max size for each chunk
                lz4::block::decompress(chunk, Some(128 * 1024))
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Combine decompressed chunks
        let total_size: usize = decompressed_chunks.iter().map(|c| c.len()).sum();
        let mut result = Vec::with_capacity(total_size);
        for chunk in decompressed_chunks {
            result.extend_from_slice(&chunk);
        }

        Ok(result)
    }

    /// Get blob hash (content-addressable)
    pub fn hash(&self) -> &str {
        &self.metadata.hash
    }

    /// Verify blob integrity by checking SHA-256 hash
    pub fn verify_integrity(&self) -> Result<bool> {
        let computed_hash = compute_hash(&self.content);
        Ok(computed_hash == self.metadata.hash)
    }

    /// Verify integrity and return error with details if failed
    pub fn verify_integrity_strict(&self) -> Result<()> {
        let computed_hash = compute_hash(&self.content);
        if computed_hash != self.metadata.hash {
            return Err(anyhow!(
                "Blob integrity verification failed for '{}': expected hash '{}', got '{}'",
                self.metadata.path,
                self.metadata.hash,
                computed_hash
            ));
        }
        Ok(())
    }
}

/// Compute SHA-256 hash of content
pub fn compute_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

/// Detect MIME type from file path
fn detect_mime_type(path: &Path) -> String {
    detect_mime_type_from_path(&path.display().to_string())
}

/// Detect MIME type from path string
fn detect_mime_type_from_path(path: &str) -> String {
    let path_lower = path.to_lowercase();

    if path_lower.ends_with(".rs") {
        "text/x-rust".to_string()
    } else if path_lower.ends_with(".js") || path_lower.ends_with(".mjs") {
        "text/javascript".to_string()
    } else if path_lower.ends_with(".ts") {
        "text/typescript".to_string()
    } else if path_lower.ends_with(".tsx") {
        "text/tsx".to_string()
    } else if path_lower.ends_with(".json") {
        "application/json".to_string()
    } else if path_lower.ends_with(".md") {
        "text/markdown".to_string()
    } else if path_lower.ends_with(".html") {
        "text/html".to_string()
    } else if path_lower.ends_with(".css") {
        "text/css".to_string()
    } else if path_lower.ends_with(".toml") {
        "application/toml".to_string()
    } else if path_lower.ends_with(".yaml") || path_lower.ends_with(".yml") {
        "application/yaml".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

/// Blob repository for local caching with platform-native I/O
pub struct BlobRepository {
    cache_dir: PathBuf,
    io: Arc<dyn PlatformIO>,
}

impl BlobRepository {
    /// Create new blob repository with default platform I/O
    pub fn new(forge_dir: &Path) -> Result<Self> {
        Self::with_io(forge_dir, create_platform_io())
    }

    /// Create new blob repository with specified I/O backend
    pub fn with_io(forge_dir: &Path, io: Arc<dyn PlatformIO>) -> Result<Self> {
        let cache_dir = forge_dir.join("blobs");
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir, io })
    }

    /// Store blob locally using platform-native I/O
    pub async fn store_local(&self, blob: &Blob) -> Result<()> {
        let hash = blob.hash();
        let blob_path = self.get_blob_path(hash);

        // Create directory structure (first 2 chars of hash)
        if let Some(parent) = blob_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let binary = blob.to_binary()?;
        self.io.write_all(&blob_path, &binary).await?;

        Ok(())
    }

    /// Load blob from local cache using platform-native I/O
    pub async fn load_local(&self, hash: &str) -> Result<Blob> {
        let blob_path = self.get_blob_path(hash);
        let binary = self.io.read_all(&blob_path).await.context("Blob not found in cache")?;

        let blob = Blob::from_binary(&binary)?;

        // Verify integrity on read
        blob.verify_integrity_strict()?;

        Ok(blob)
    }

    /// Load blob without integrity verification (for performance-critical paths)
    pub async fn load_local_unchecked(&self, hash: &str) -> Result<Blob> {
        let blob_path = self.get_blob_path(hash);
        let binary = self.io.read_all(&blob_path).await.context("Blob not found in cache")?;

        Blob::from_binary(&binary)
    }

    /// Check if blob exists locally
    pub async fn exists_local(&self, hash: &str) -> bool {
        self.get_blob_path(hash).exists()
    }

    /// Get blob storage path (content-addressable)
    fn get_blob_path(&self, hash: &str) -> PathBuf {
        // Store blobs like Git: .dx/forge/blobs/ab/cdef1234...
        let prefix = &hash[..2.min(hash.len())];
        let suffix = if hash.len() > 2 { &hash[2..] } else { "" };
        self.cache_dir.join(prefix).join(suffix)
    }

    /// Get the I/O backend name
    pub fn backend_name(&self) -> &'static str {
        self.io.backend_name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blob_serialization() {
        let content = b"Hello, world!".to_vec();
        let blob = Blob::from_content("test.txt", content.clone());

        let binary = blob.to_binary().unwrap();
        let restored = Blob::from_binary(&binary).unwrap();

        assert_eq!(blob.metadata.hash, restored.metadata.hash);
        assert_eq!(blob.content, restored.content);
        assert_eq!(blob.metadata.path, restored.metadata.path);
    }

    #[tokio::test]
    async fn test_blob_compression() {
        let content = b"Hello, world! ".repeat(1000);
        let mut blob = Blob::from_content("test.txt", content.clone());

        let original_size = blob.metadata.size;
        blob.compress().unwrap();
        let compressed_size = blob.metadata.size;

        assert!(compressed_size < original_size);
        assert_eq!(blob.metadata.compression, Some("lz4".to_string()));

        blob.decompress().unwrap();
        assert_eq!(blob.content, content);
        assert_eq!(blob.metadata.compression, None);
    }

    #[tokio::test]
    async fn test_blob_integrity_verification() {
        let content = b"Test content for integrity check".to_vec();
        let blob = Blob::from_content("test.txt", content.clone());

        // Verify integrity passes for valid blob
        assert!(blob.verify_integrity().unwrap());
        assert!(blob.verify_integrity_strict().is_ok());
    }

    #[tokio::test]
    async fn test_blob_integrity_failure() {
        let content = b"Test content".to_vec();
        let mut blob = Blob::from_content("test.txt", content);

        // Corrupt the content
        blob.content = b"Corrupted content".to_vec();

        // Verify integrity fails
        assert!(!blob.verify_integrity().unwrap());
        assert!(blob.verify_integrity_strict().is_err());
    }

    #[test]
    fn test_compute_hash() {
        let content = b"Hello, world!";
        let hash = compute_hash(content);
        // SHA-256 of "Hello, world!" is known
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }
}

/// Property-based tests for blob storage
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    // Strategy for generating arbitrary blob content
    fn arbitrary_content() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..10000)
    }

    // Strategy for generating valid file paths
    fn arbitrary_filename() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9_]{0,20}\\.(txt|rs|json|md)"
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8: Blob Integrity Round-Trip
        /// For any content, creating a blob, serializing, deserializing,
        /// and verifying integrity should always succeed.
        #[test]
        fn prop_blob_integrity_roundtrip(content in arbitrary_content(), filename in arbitrary_filename()) {
            let blob = Blob::from_content(&filename, content.clone());

            // Verify hash is computed correctly
            let expected_hash = compute_hash(&content);
            prop_assert_eq!(&blob.metadata.hash, &expected_hash);

            // Serialize and deserialize
            let binary = blob.to_binary().unwrap();
            let restored = Blob::from_binary(&binary).unwrap();

            // Verify integrity after round-trip
            prop_assert!(restored.verify_integrity().unwrap());
            prop_assert!(restored.verify_integrity_strict().is_ok());

            // Verify content is preserved
            prop_assert_eq!(restored.content, content);
            prop_assert_eq!(restored.metadata.hash, expected_hash);
        }

        /// Property: Compression round-trip preserves content
        #[test]
        fn prop_compression_roundtrip(content in arbitrary_content()) {
            if content.is_empty() {
                return Ok(());
            }

            let mut blob = Blob::from_content("test.txt", content.clone());
            let original_hash = blob.metadata.hash.clone();

            // Compress
            blob.compress().unwrap();

            // Decompress
            blob.decompress().unwrap();

            // Content should be preserved
            prop_assert_eq!(&blob.content, &content);

            // Hash should match original after decompression
            let new_hash = compute_hash(&blob.content);
            prop_assert_eq!(new_hash, original_hash);
        }

        /// Property: Corrupted content fails integrity check
        #[test]
        fn prop_corruption_detected(content in arbitrary_content(), corruption_byte in any::<u8>()) {
            if content.is_empty() {
                return Ok(());
            }

            let mut blob = Blob::from_content("test.txt", content.clone());

            // Corrupt a byte in the content
            let idx = blob.content.len() / 2;
            if blob.content[idx] != corruption_byte {
                blob.content[idx] = corruption_byte;

                // Integrity check should fail
                prop_assert!(!blob.verify_integrity().unwrap());
                prop_assert!(blob.verify_integrity_strict().is_err());
            }
        }
    }

    /// Property 7: Concurrent Storage Operations
    /// Multiple concurrent reads and writes should not corrupt data.
    #[tokio::test]
    async fn prop_concurrent_storage_operations() {
        use std::sync::Arc;
        use tokio::sync::Barrier;

        let temp_dir = TempDir::new().unwrap();
        let repo = Arc::new(BlobRepository::new(temp_dir.path()).unwrap());

        // Create test blobs
        let num_blobs = 20;
        let mut blobs = Vec::new();
        for i in 0..num_blobs {
            let content = format!("Test content for blob {}", i).into_bytes();
            let blob = Blob::from_content(&format!("test_{}.txt", i), content);
            blobs.push(blob);
        }

        // Store all blobs first
        for blob in &blobs {
            repo.store_local(blob).await.unwrap();
        }

        // Concurrent reads - all tasks start at the same time
        let barrier = Arc::new(Barrier::new(num_blobs));
        let mut handles = Vec::new();

        for blob in &blobs {
            let repo = Arc::clone(&repo);
            let hash = blob.hash().to_string();
            let expected_content = blob.content.clone();
            let barrier = Arc::clone(&barrier);

            handles.push(tokio::spawn(async move {
                barrier.wait().await;

                // Read blob and verify
                let loaded = repo.load_local(&hash).await.unwrap();
                assert_eq!(loaded.content, expected_content);
                assert!(loaded.verify_integrity().unwrap());
            }));
        }

        // Wait for all reads to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Concurrent writes with different content
        let barrier = Arc::new(Barrier::new(num_blobs));
        let mut handles = Vec::new();

        for i in 0..num_blobs {
            let repo = Arc::clone(&repo);
            let barrier = Arc::clone(&barrier);

            handles.push(tokio::spawn(async move {
                barrier.wait().await;

                let content = format!("Updated content for blob {}", i).into_bytes();
                let blob = Blob::from_content(&format!("updated_{}.txt", i), content);
                repo.store_local(&blob).await.unwrap();

                // Verify we can read it back
                let loaded = repo.load_local(blob.hash()).await.unwrap();
                assert!(loaded.verify_integrity().unwrap());
            }));
        }

        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test that load_local verifies integrity and rejects corrupted blobs
    #[tokio::test]
    async fn test_load_local_integrity_verification() {
        let temp_dir = TempDir::new().unwrap();
        let repo = BlobRepository::new(temp_dir.path()).unwrap();

        // Create and store a blob
        let content = b"Test content for integrity".to_vec();
        let blob = Blob::from_content("test.txt", content);
        let hash = blob.hash().to_string();

        repo.store_local(&blob).await.unwrap();

        // Load should succeed with valid blob
        let loaded = repo.load_local(&hash).await;
        assert!(loaded.is_ok());

        // Now corrupt the stored blob directly
        let blob_path = temp_dir.path().join("blobs").join(&hash[..2]).join(&hash[2..]);
        let mut binary = tokio::fs::read(&blob_path).await.unwrap();

        // Corrupt the content portion (after metadata)
        if binary.len() > 100 {
            let idx = binary.len() - 10;
            binary[idx] ^= 0xFF;
            tokio::fs::write(&blob_path, &binary).await.unwrap();

            // Load should now fail due to integrity check
            let result = repo.load_local(&hash).await;
            assert!(result.is_err());
        }
    }

    /// Property 23: Concurrent Read Support
    /// Multiple concurrent reads to the same blob should all succeed
    /// and return identical data.
    #[tokio::test]
    async fn prop_concurrent_read_support() {
        use std::sync::Arc;
        use tokio::sync::Barrier;

        let temp_dir = TempDir::new().unwrap();
        let repo = Arc::new(BlobRepository::new(temp_dir.path()).unwrap());

        // Create and store a blob
        let content = b"Test content for concurrent reads - this is some longer content to make the test more meaningful".to_vec();
        let blob = Blob::from_content("test.txt", content.clone());
        let hash = blob.hash().to_string();

        repo.store_local(&blob).await.unwrap();

        // Concurrent reads
        let num_readers = 20;
        let barrier = Arc::new(Barrier::new(num_readers));
        let mut handles = Vec::new();

        for _ in 0..num_readers {
            let repo = Arc::clone(&repo);
            let hash = hash.clone();
            let expected_content = content.clone();
            let barrier = Arc::clone(&barrier);

            handles.push(tokio::spawn(async move {
                // Wait for all readers to be ready
                barrier.wait().await;

                // Read the blob
                let loaded = repo.load_local(&hash).await.unwrap();

                // Verify content matches
                assert_eq!(loaded.content, expected_content);
                assert!(loaded.verify_integrity().unwrap());
            }));
        }

        // Wait for all reads to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Property 24: Write Serialization
    /// Concurrent writes to the same blob should result in consistent state.
    #[tokio::test]
    async fn prop_write_serialization() {
        use std::sync::Arc;
        use tokio::sync::Barrier;

        let temp_dir = TempDir::new().unwrap();
        let repo = Arc::new(BlobRepository::new(temp_dir.path()).unwrap());

        // Create blobs with same path but different content
        let num_writers = 10;
        let barrier = Arc::new(Barrier::new(num_writers));
        let mut handles = Vec::new();

        for i in 0..num_writers {
            let repo = Arc::clone(&repo);
            let barrier = Arc::clone(&barrier);

            handles.push(tokio::spawn(async move {
                // Wait for all writers to be ready
                barrier.wait().await;

                // Create and store a blob
                let content = format!("Content from writer {}", i).into_bytes();
                let blob = Blob::from_content("shared.txt", content);
                let hash = blob.hash().to_string();

                repo.store_local(&blob).await.unwrap();

                // Return the hash we wrote
                hash
            }));
        }

        // Collect all hashes
        let mut hashes = Vec::new();
        for handle in handles {
            hashes.push(handle.await.unwrap());
        }

        // All written blobs should be readable and valid
        for hash in &hashes {
            let loaded = repo.load_local(hash).await.unwrap();
            assert!(loaded.verify_integrity().unwrap());
        }
    }
}
