use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::optimized_rkyv::OptimizedRkyv;
use std::fs;
use tempfile::TempDir;

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    data: Vec<u8>,
}

impl TestData {
    fn new(id: u64, size: usize) -> Self {
        Self {
            id,
            name: format!("item_{}", id),
            data: vec![0u8; size],
        }
    }
}

#[test]
fn test_adaptive_file_io_small() {
    // Small file (<1KB) should use std::fs
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("small.rkyv");
    let opt = OptimizedRkyv::new();

    let data = TestData::new(1, 100);
    opt.serialize_to_file(&data, &path).unwrap();

    let loaded: TestData = opt.deserialize_from_file(&path).unwrap();
    assert_eq!(data, loaded);

    // Verify file exists and is small
    let metadata = fs::metadata(&path).unwrap();
    assert!(metadata.len() < 1024);
}

#[test]
fn test_adaptive_file_io_large() {
    // Large file (>1KB) should use platform I/O
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("large.rkyv");
    let opt = OptimizedRkyv::new();

    let data = TestData::new(1, 2000);
    opt.serialize_to_file(&data, &path).unwrap();

    let loaded: TestData = opt.deserialize_from_file(&path).unwrap();
    assert_eq!(data, loaded);

    // Verify file exists and is large
    let metadata = fs::metadata(&path).unwrap();
    assert!(metadata.len() > 1024);
}

#[test]
fn test_adaptive_batch_small() {
    // Small batch (<10k) should use pre-allocation
    let opt = OptimizedRkyv::new();
    let items: Vec<TestData> = (0..100).map(|i| TestData::new(i, 50)).collect();

    let serialized = opt.serialize_batch_smart(&items).unwrap();
    assert_eq!(serialized.len(), 100);
}

#[test]
#[cfg(feature = "parallel")]
fn test_adaptive_batch_large() {
    // Large batch (>10k) should use parallel processing
    let opt = OptimizedRkyv::new();
    let items: Vec<TestData> = (0..15000).map(|i| TestData::new(i, 50)).collect();

    let serialized = opt.serialize_batch_smart(&items).unwrap();
    assert_eq!(serialized.len(), 15000);
}

#[test]
fn test_batch_file_operations() {
    let dir = TempDir::new().unwrap();
    let opt = OptimizedRkyv::new();

    let items: Vec<(TestData, _)> = (0..10)
        .map(|i| {
            let data = TestData::new(i, 100);
            let path = dir.path().join(format!("batch_{}.rkyv", i));
            (data, path)
        })
        .collect();

    let items_ref: Vec<_> = items.iter().map(|(d, p)| (d.clone(), p.as_path())).collect();

    // Write batch
    let results = opt.serialize_batch_to_files(&items_ref).unwrap();
    assert_eq!(results.len(), 10);
    assert!(results.iter().all(|r| r.is_ok()));

    // Read batch
    let paths: Vec<_> = items.iter().map(|(_, p)| p.as_path()).collect();
    let loaded = opt.deserialize_batch_from_files::<TestData>(&paths).unwrap();
    assert_eq!(loaded.len(), 10);

    for (i, result) in loaded.iter().enumerate() {
        let data = result.as_ref().unwrap();
        assert_eq!(data.id, i as u64);
    }
}

#[test]
#[cfg(feature = "compression")]
fn test_adaptive_compression() {
    use serializer::machine::compress::CompressionLevel;
    use serializer::machine::optimized_rkyv::CompressedRkyv;

    let mut comp = CompressedRkyv::new(CompressionLevel::Fast);

    // Small data (<100 bytes) - should skip compression
    let small = TestData::new(1, 50);
    let compressed_small = comp.serialize_compressed(&small).unwrap();

    // Large data (>100 bytes) - should compress
    let large = TestData::new(1, 500);
    let compressed_large = comp.serialize_compressed(&large).unwrap();

    // Verify decompression works
    let decompressed: TestData = comp.deserialize_compressed(&compressed_large).unwrap();
    assert_eq!(large, decompressed);
}

#[test]
#[cfg(feature = "arena")]
fn test_adaptive_arena() {
    use serializer::machine::optimized_rkyv::ArenaRkyv;

    let mut arena = ArenaRkyv::new();
    let items: Vec<TestData> = (0..100).map(|i| TestData::new(i, 50)).collect();

    let serialized = arena.serialize_batch(&items).unwrap();
    assert_eq!(serialized.len(), 100);

    // Reset and reuse
    arena.reset();
    let serialized2 = arena.serialize_batch(&items).unwrap();
    assert_eq!(serialized2.len(), 100);
}

#[test]
fn test_backend_name() {
    let opt = OptimizedRkyv::new();
    let backend = opt.backend_name();

    #[cfg(target_os = "linux")]
    assert_eq!(backend, "io_uring");
    #[cfg(target_os = "windows")]
    assert_eq!(backend, "iocp");
    #[cfg(target_os = "macos")]
    assert_eq!(backend, "kqueue");
}
