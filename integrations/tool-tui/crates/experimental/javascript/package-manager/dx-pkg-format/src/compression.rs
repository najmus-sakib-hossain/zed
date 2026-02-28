use dx_pkg_core::{Error, Result};

/// Compression type flags
pub const COMPRESSION_NONE: u8 = 0;
pub const COMPRESSION_LZ4: u8 = 1;
pub const COMPRESSION_ZSTD: u8 = 2;

/// Compress data using LZ4 (ultra-fast)
pub fn compress_lz4(data: &[u8]) -> Result<Vec<u8>> {
    Ok(lz4_flex::compress_prepend_size(data))
}

/// Compress data using Zstd (high compression, moderate speed)
pub fn compress_zstd(data: &[u8], level: i32) -> Result<Vec<u8>> {
    zstd::encode_all(data, level).map_err(|e| Error::Compression(e.to_string()))
}

/// Decompress data based on flags
pub fn decompress(data: &[u8], _expected_size: usize, compression_flags: u8) -> Result<Vec<u8>> {
    match compression_flags & 0x03 {
        COMPRESSION_NONE => Ok(data.to_vec()),
        COMPRESSION_LZ4 => {
            lz4_flex::decompress_size_prepended(data).map_err(|e| Error::Compression(e.to_string()))
        }
        COMPRESSION_ZSTD => zstd::decode_all(data).map_err(|e| Error::Compression(e.to_string())),
        _ => Err(Error::Compression("Unknown compression type".into())),
    }
}

/// Choose compression strategy based on file size
pub fn choose_compression(size: usize) -> u8 {
    if size < 1024 {
        COMPRESSION_NONE // Small files: no compression overhead
    } else if size < 100_000 {
        COMPRESSION_LZ4 // Medium files: LZ4 ultra-fast
    } else {
        COMPRESSION_ZSTD // Large files: Zstd better ratio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_roundtrip() {
        let data = b"hello world ".repeat(100);
        let compressed = compress_lz4(&data).unwrap();
        let decompressed = decompress(&compressed, data.len(), COMPRESSION_LZ4).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_zstd_roundtrip() {
        let data = b"hello world ".repeat(100);
        let compressed = compress_zstd(&data, 3).unwrap();
        let decompressed = decompress(&compressed, data.len(), COMPRESSION_ZSTD).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_choose_compression() {
        assert_eq!(choose_compression(512), COMPRESSION_NONE);
        assert_eq!(choose_compression(5000), COMPRESSION_LZ4);
        assert_eq!(choose_compression(200_000), COMPRESSION_ZSTD);
    }
}
