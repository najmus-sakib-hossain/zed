use serializer::llm::convert::{
    CompressionAlgorithm, document_to_machine_with_compression, machine_to_document,
};
use serializer::llm::types::{DxDocument, DxLlmValue};

fn create_test_doc() -> DxDocument {
    let mut doc = DxDocument::new();
    for i in 0..100 {
        doc.context.insert(
            format!("field_{}", i),
            DxLlmValue::Str(format!("This is test value number {}", i)),
        );
    }
    doc
}

#[test]
fn test_lz4_compression_roundtrip() {
    let original = create_test_doc();
    let machine = document_to_machine_with_compression(&original, CompressionAlgorithm::Lz4);
    let restored = machine_to_document(&machine).unwrap();

    assert_eq!(original.context.len(), restored.context.len());
    for (key, value) in &original.context {
        assert_eq!(restored.context.get(key), Some(value));
    }

    println!("LZ4: {} bytes", machine.as_bytes().len());
}

#[test]
fn test_zstd_compression_roundtrip() {
    let original = create_test_doc();
    let machine = document_to_machine_with_compression(&original, CompressionAlgorithm::Zstd);
    let restored = machine_to_document(&machine).unwrap();

    assert_eq!(original.context.len(), restored.context.len());
    for (key, value) in &original.context {
        assert_eq!(restored.context.get(key), Some(value));
    }

    println!("Zstd: {} bytes", machine.as_bytes().len());
}

#[test]
fn test_no_compression_roundtrip() {
    let original = create_test_doc();
    let machine = document_to_machine_with_compression(&original, CompressionAlgorithm::None);
    let restored = machine_to_document(&machine).unwrap();

    assert_eq!(original.context.len(), restored.context.len());
    for (key, value) in &original.context {
        assert_eq!(restored.context.get(key), Some(value));
    }

    println!("None: {} bytes", machine.as_bytes().len());
}

#[test]
fn test_lz4_caching() {
    let original = create_test_doc();
    let machine = document_to_machine_with_compression(&original, CompressionAlgorithm::Lz4);

    // First access - decompresses
    let start = std::time::Instant::now();
    let _ = machine_to_document(&machine).unwrap();
    let first = start.elapsed();

    // Second access - cached
    let start = std::time::Instant::now();
    let _ = machine_to_document(&machine).unwrap();
    let cached = start.elapsed();

    println!("LZ4 - First: {:?}, Cached: {:?}", first, cached);
    assert!(cached < first, "Cached access should be faster");
}

#[test]
fn test_zstd_caching() {
    let original = create_test_doc();
    let machine = document_to_machine_with_compression(&original, CompressionAlgorithm::Zstd);

    // First access - decompresses
    let start = std::time::Instant::now();
    let _ = machine_to_document(&machine).unwrap();
    let first = start.elapsed();

    // Second access - cached
    let start = std::time::Instant::now();
    let _ = machine_to_document(&machine).unwrap();
    let cached = start.elapsed();

    println!("Zstd - First: {:?}, Cached: {:?}", first, cached);
    assert!(cached < first, "Cached access should be faster");
}

#[test]
fn test_compression_comparison() {
    let original = create_test_doc();

    let lz4 = document_to_machine_with_compression(&original, CompressionAlgorithm::Lz4);
    let zstd = document_to_machine_with_compression(&original, CompressionAlgorithm::Zstd);
    let none = document_to_machine_with_compression(&original, CompressionAlgorithm::None);

    println!("\nCompression comparison:");
    println!("  None: {} bytes", none.as_bytes().len());
    println!(
        "  LZ4:  {} bytes ({:.1}% of uncompressed)",
        lz4.as_bytes().len(),
        (lz4.as_bytes().len() as f64 / none.as_bytes().len() as f64) * 100.0
    );
    println!(
        "  Zstd: {} bytes ({:.1}% of uncompressed)",
        zstd.as_bytes().len(),
        (zstd.as_bytes().len() as f64 / none.as_bytes().len() as f64) * 100.0
    );

    // Both should be smaller than uncompressed
    assert!(lz4.as_bytes().len() < none.as_bytes().len());
    assert!(zstd.as_bytes().len() < none.as_bytes().len());
}
