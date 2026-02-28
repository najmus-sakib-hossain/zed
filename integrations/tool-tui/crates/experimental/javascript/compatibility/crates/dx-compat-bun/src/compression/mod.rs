//! Bun compression functions.
//!
//! High-performance compression using gzip, deflate, brotli, and zstd.
//! Targets 450 MB/s gzip throughput.

use crate::error::{BunError, BunResult};
use flate2::read::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder};
use flate2::Compression;
use std::io::Read;

/// Compression level (0-9 for most algorithms).
#[derive(Debug, Clone, Copy)]
pub struct CompressionLevel(pub u32);

impl Default for CompressionLevel {
    fn default() -> Self {
        Self(6)
    }
}

impl From<u32> for CompressionLevel {
    fn from(level: u32) -> Self {
        Self(level.min(9))
    }
}

// ============================================================================
// Gzip
// ============================================================================

/// Gzip compress data synchronously.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-9, default: 6)
///
/// # Returns
/// Compressed data in gzip format.
pub fn gzip_sync(data: &[u8], level: Option<u32>) -> BunResult<Vec<u8>> {
    let level = Compression::new(level.unwrap_or(6));
    let mut encoder = GzEncoder::new(data, level);
    let mut result = Vec::new();
    encoder
        .read_to_end(&mut result)
        .map_err(|e| BunError::Compression(format!("gzip compress failed: {}", e)))?;
    Ok(result)
}

/// Gzip decompress data synchronously.
///
/// # Arguments
/// * `data` - Gzip compressed data
///
/// # Returns
/// Decompressed data.
pub fn gunzip_sync(data: &[u8]) -> BunResult<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(|e| BunError::Compression(format!("gzip decompress failed: {}", e)))?;
    Ok(result)
}

/// Gzip compress data asynchronously.
pub async fn gzip(data: Vec<u8>, level: Option<u32>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || gzip_sync(&data, level))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

/// Gzip decompress data asynchronously.
pub async fn gunzip(data: Vec<u8>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || gunzip_sync(&data))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

// ============================================================================
// Deflate
// ============================================================================

/// Deflate compress data synchronously.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-9, default: 6)
///
/// # Returns
/// Compressed data in deflate format.
pub fn deflate_sync(data: &[u8], level: Option<u32>) -> BunResult<Vec<u8>> {
    let level = Compression::new(level.unwrap_or(6));
    let mut encoder = DeflateEncoder::new(data, level);
    let mut result = Vec::new();
    encoder
        .read_to_end(&mut result)
        .map_err(|e| BunError::Compression(format!("deflate compress failed: {}", e)))?;
    Ok(result)
}

/// Deflate decompress data synchronously.
///
/// # Arguments
/// * `data` - Deflate compressed data
///
/// # Returns
/// Decompressed data.
pub fn inflate_sync(data: &[u8]) -> BunResult<Vec<u8>> {
    let mut decoder = DeflateDecoder::new(data);
    let mut result = Vec::new();
    decoder
        .read_to_end(&mut result)
        .map_err(|e| BunError::Compression(format!("deflate decompress failed: {}", e)))?;
    Ok(result)
}

/// Deflate compress data asynchronously.
pub async fn deflate(data: Vec<u8>, level: Option<u32>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || deflate_sync(&data, level))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

/// Deflate decompress data asynchronously.
pub async fn inflate(data: Vec<u8>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || inflate_sync(&data))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

// ============================================================================
// Brotli
// ============================================================================

/// Brotli compress data synchronously.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (0-11, default: 6)
///
/// # Returns
/// Compressed data in brotli format.
pub fn brotli_compress_sync(data: &[u8], level: Option<u32>) -> BunResult<Vec<u8>> {
    let mut result = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: level.unwrap_or(6).min(11) as i32,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut std::io::Cursor::new(data), &mut result, &params)
        .map_err(|e| BunError::Compression(format!("brotli compress failed: {}", e)))?;
    Ok(result)
}

/// Brotli decompress data synchronously.
///
/// # Arguments
/// * `data` - Brotli compressed data
///
/// # Returns
/// Decompressed data.
pub fn brotli_decompress_sync(data: &[u8]) -> BunResult<Vec<u8>> {
    let mut result = Vec::new();
    brotli::BrotliDecompress(&mut std::io::Cursor::new(data), &mut result)
        .map_err(|e| BunError::Compression(format!("brotli decompress failed: {}", e)))?;
    Ok(result)
}

/// Brotli compress data asynchronously.
pub async fn brotli_compress(data: Vec<u8>, level: Option<u32>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || brotli_compress_sync(&data, level))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

/// Brotli decompress data asynchronously.
pub async fn brotli_decompress(data: Vec<u8>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || brotli_decompress_sync(&data))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

// ============================================================================
// Zstandard (Zstd)
// ============================================================================

/// Zstd compress data synchronously.
///
/// # Arguments
/// * `data` - Data to compress
/// * `level` - Compression level (1-22, default: 3)
///
/// # Returns
/// Compressed data in zstd format.
pub fn zstd_compress_sync(data: &[u8], level: Option<i32>) -> BunResult<Vec<u8>> {
    let level = level.unwrap_or(3);
    zstd::encode_all(std::io::Cursor::new(data), level)
        .map_err(|e| BunError::Compression(format!("zstd compress failed: {}", e)))
}

/// Zstd decompress data synchronously.
///
/// # Arguments
/// * `data` - Zstd compressed data
///
/// # Returns
/// Decompressed data.
pub fn zstd_decompress_sync(data: &[u8]) -> BunResult<Vec<u8>> {
    zstd::decode_all(std::io::Cursor::new(data))
        .map_err(|e| BunError::Compression(format!("zstd decompress failed: {}", e)))
}

/// Zstd compress data asynchronously.
pub async fn zstd_compress(data: Vec<u8>, level: Option<i32>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || zstd_compress_sync(&data, level))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

/// Zstd decompress data asynchronously.
pub async fn zstd_decompress(data: Vec<u8>) -> BunResult<Vec<u8>> {
    tokio::task::spawn_blocking(move || zstd_decompress_sync(&data))
        .await
        .map_err(|e| BunError::Compression(format!("Task join error: {}", e)))?
}

// ============================================================================
// Compression Format Detection
// ============================================================================

/// Detected compression format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// Gzip format
    Gzip,
    /// Deflate format
    Deflate,
    /// Brotli format
    Brotli,
    /// Zstandard format
    Zstd,
    /// Unknown or uncompressed
    Unknown,
}

/// Detect compression format from magic bytes.
pub fn detect_format(data: &[u8]) -> CompressionFormat {
    if data.len() < 2 {
        return CompressionFormat::Unknown;
    }

    // Gzip: 1f 8b
    if data[0] == 0x1f && data[1] == 0x8b {
        return CompressionFormat::Gzip;
    }

    // Zstd: 28 b5 2f fd
    if data.len() >= 4 && data[0] == 0x28 && data[1] == 0xb5 && data[2] == 0x2f && data[3] == 0xfd {
        return CompressionFormat::Zstd;
    }

    // Deflate: typically starts with 78 (9c, da, 01, 5e)
    if data[0] == 0x78 && (data[1] == 0x9c || data[1] == 0xda || data[1] == 0x01 || data[1] == 0x5e)
    {
        return CompressionFormat::Deflate;
    }

    CompressionFormat::Unknown
}

/// Decompress data, auto-detecting the format.
pub fn decompress_auto(data: &[u8]) -> BunResult<Vec<u8>> {
    match detect_format(data) {
        CompressionFormat::Gzip => gunzip_sync(data),
        CompressionFormat::Deflate => inflate_sync(data),
        CompressionFormat::Zstd => zstd_decompress_sync(data),
        CompressionFormat::Brotli => brotli_decompress_sync(data),
        CompressionFormat::Unknown => {
            Err(BunError::Compression("Unknown compression format".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_round_trip() {
        let data = b"hello world, this is a test of gzip compression";
        let compressed = gzip_sync(data, None).unwrap();
        let decompressed = gunzip_sync(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_gzip_compression_levels() {
        let data = b"hello world".repeat(100);
        let fast = gzip_sync(&data, Some(1)).unwrap();
        let best = gzip_sync(&data, Some(9)).unwrap();
        // Best compression should be smaller or equal
        assert!(best.len() <= fast.len());
    }

    #[test]
    fn test_deflate_round_trip() {
        let data = b"hello world, this is a test of deflate compression";
        let compressed = deflate_sync(data, None).unwrap();
        let decompressed = inflate_sync(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_brotli_round_trip() {
        let data = b"hello world, this is a test of brotli compression";
        let compressed = brotli_compress_sync(data, None).unwrap();
        let decompressed = brotli_decompress_sync(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_zstd_round_trip() {
        let data = b"hello world, this is a test of zstd compression";
        let compressed = zstd_compress_sync(data, None).unwrap();
        let decompressed = zstd_decompress_sync(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_detect_gzip() {
        let data = b"hello world";
        let compressed = gzip_sync(data, None).unwrap();
        assert_eq!(detect_format(&compressed), CompressionFormat::Gzip);
    }

    #[test]
    fn test_detect_zstd() {
        let data = b"hello world";
        let compressed = zstd_compress_sync(data, None).unwrap();
        assert_eq!(detect_format(&compressed), CompressionFormat::Zstd);
    }

    #[test]
    fn test_decompress_auto_gzip() {
        let data = b"hello world";
        let compressed = gzip_sync(data, None).unwrap();
        let decompressed = decompress_auto(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_decompress_auto_zstd() {
        let data = b"hello world";
        let compressed = zstd_compress_sync(data, None).unwrap();
        let decompressed = decompress_auto(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_empty_data() {
        let data: &[u8] = b"";
        let compressed = gzip_sync(data, None).unwrap();
        let decompressed = gunzip_sync(&compressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[tokio::test]
    async fn test_async_gzip() {
        let data = b"hello world".to_vec();
        let compressed = gzip(data.clone(), None).await.unwrap();
        let decompressed = gunzip(compressed).await.unwrap();
        assert_eq!(decompressed, data);
    }

    #[tokio::test]
    async fn test_async_zstd() {
        let data = b"hello world".to_vec();
        let compressed = zstd_compress(data.clone(), None).await.unwrap();
        let decompressed = zstd_decompress(compressed).await.unwrap();
        assert_eq!(decompressed, data);
    }
}
