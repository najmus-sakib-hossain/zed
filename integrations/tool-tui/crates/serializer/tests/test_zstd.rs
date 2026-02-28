//! Test Zstd compression functionality

#[cfg(feature = "compression")]
#[test]
fn test_zstd_basic() {
    use serializer::machine::compress::{compress_zstd, decompress_zstd};

    let original = b"Hello, World! This is a test of Zstd compression.";
    let compressed = compress_zstd(original).expect("compression failed");
    let decompressed = decompress_zstd(&compressed).expect("decompression failed");

    assert_eq!(decompressed, original);
    println!("Zstd test passed: {} bytes -> {} bytes", original.len(), compressed.len());
}

#[cfg(feature = "compression")]
#[test]
fn test_zstd_levels() {
    use serializer::machine::compress::{CompressionLevel, compress_zstd_level, decompress_zstd};

    let original: Vec<u8> = std::iter::repeat(b'A').take(1000).collect();

    let fast = compress_zstd_level(&original, CompressionLevel::Fast).unwrap();
    let default = compress_zstd_level(&original, CompressionLevel::Default).unwrap();
    let high = compress_zstd_level(&original, CompressionLevel::High).unwrap();

    // All should decompress correctly
    assert_eq!(decompress_zstd(&fast).unwrap(), original);
    assert_eq!(decompress_zstd(&default).unwrap(), original);
    assert_eq!(decompress_zstd(&high).unwrap(), original);

    println!("Zstd levels test passed");
}
