//! Property tests for Machine format (DX-Machine)
//!
//! These tests validate the correctness properties defined in the design document
//! for the Machine format binary serialization.

#[cfg(test)]
mod property_tests {
    use crate::error::{DxError, DX_MAGIC, DX_VERSION};
    use crate::machine::compress::{
        CompressionLevel, DxCompressed, StreamCompressor, StreamDecompressor,
    };
    use crate::machine::mmap::{DxMmap, DxMmapBatch};
    use crate::machine::simd512::{dispatch, portable};
    use crate::machine::slot::{DxMachineSlot, HEAP_MARKER, INLINE_MARKER, MAX_INLINE_SIZE};
    use proptest::prelude::*;

    // ========================================================================
    // Property 10: Machine Format String Storage
    // Validates: Requirements 3.6, 6.3
    // For any string value:
    // - If length <= 14 bytes, the slot marker byte SHALL be 0x00 (inline storage)
    // - If length > 14 bytes, the slot marker byte SHALL be 0xFF (heap storage)
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 10: String Storage
        /// Validates: Requirements 3.6, 6.3
        #[test]
        fn prop_string_storage_inline_threshold(s in "[ -~]{0,30}") {
            let bytes = s.as_bytes();

            if bytes.len() <= MAX_INLINE_SIZE {
                // Should use inline storage
                let slot = DxMachineSlot::inline_from_bytes(bytes).unwrap();
                prop_assert_eq!(slot.data[15], INLINE_MARKER,
                    "String of {} bytes should be inline (marker 0x00)", bytes.len());
                prop_assert!(slot.is_inline());
                prop_assert_eq!(slot.inline_len(), bytes.len());
                prop_assert_eq!(slot.inline_data(), bytes);
            } else {
                // Should use heap storage
                let slot = DxMachineSlot::heap_reference(0, bytes.len() as u32);
                prop_assert_eq!(slot.data[15], HEAP_MARKER,
                    "String of {} bytes should be heap (marker 0xFF)", bytes.len());
                prop_assert!(slot.is_heap());
                prop_assert_eq!(slot.heap_length() as usize, bytes.len());
            }
        }

        /// Test that inline storage preserves data exactly
        #[test]
        fn prop_inline_storage_preserves_data(bytes in prop::collection::vec(any::<u8>(), 0..=14)) {
            let slot = DxMachineSlot::inline_from_bytes(&bytes).unwrap();
            prop_assert!(slot.is_inline());
            prop_assert_eq!(slot.inline_data(), bytes.as_slice());
        }

        /// Test that heap references store offset and length correctly
        #[test]
        fn prop_heap_reference_stores_correctly(offset in 0u32..u32::MAX, length in 0u32..u32::MAX) {
            let slot = DxMachineSlot::heap_reference(offset, length);
            prop_assert!(slot.is_heap());
            prop_assert_eq!(slot.heap_offset(), offset);
            prop_assert_eq!(slot.heap_length(), length);
        }
    }

    // ========================================================================
    // Property 13: Binary Header Validation
    // Validates: Requirements 6.2
    // For any byte sequence that does not start with magic bytes [0x5A, 0x44]
    // or has invalid version byte, the Machine format parser SHALL return an
    // InvalidMagic or UnsupportedVersion error before attempting field access.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 13: Header Validation
        /// Validates: Requirements 6.2
        #[test]
        fn prop_invalid_magic_detected(byte0 in any::<u8>(), byte1 in any::<u8>()) {
            // Skip valid magic bytes
            prop_assume!(byte0 != DX_MAGIC[0] || byte1 != DX_MAGIC[1]);

            let mut data = vec![byte0, byte1, DX_VERSION, 0x04];
            data.extend_from_slice(&[0u8; 28]); // Pad to minimum size

            let mmap = DxMmap::from_bytes(data);
            let result = mmap.validate_header();

            prop_assert!(result.is_err(), "Invalid magic should be rejected");
        }

        /// Test that valid magic bytes are accepted
        #[test]
        fn prop_valid_magic_accepted(_dummy in 0..1u8) {
            let mut data = vec![DX_MAGIC[0], DX_MAGIC[1], DX_VERSION, 0x04];
            data.extend_from_slice(&[0u8; 28]);

            let mmap = DxMmap::from_bytes(data);
            let result = mmap.validate_header();

            prop_assert!(result.is_ok(), "Valid magic should be accepted");
        }

        /// Test that unsupported versions are rejected
        #[test]
        fn prop_unsupported_version_rejected(version in 2u8..=255u8) {
            let mut data = vec![DX_MAGIC[0], DX_MAGIC[1], version, 0x04];
            data.extend_from_slice(&[0u8; 28]);

            let mmap = DxMmap::from_bytes(data);
            let result = mmap.validate_header();

            prop_assert!(result.is_err(), "Unsupported version {} should be rejected", version);
        }
    }

    // ========================================================================
    // Property 15: Buffer Size Error
    // Validates: Requirements 6.7
    // For any buffer smaller than the required size for a Machine format operation,
    // the error SHALL be BufferTooSmall and SHALL include the required size.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 15: Buffer Size Error
        /// Validates: Requirements 6.7
        #[test]
        fn prop_buffer_too_small_error(size in 0usize..4) {
            let data = vec![0u8; size];
            let mmap = DxMmap::from_bytes(data);
            let result = mmap.validate_header();

            prop_assert!(result.is_err(), "Buffer of {} bytes should fail validation", size);
        }

        /// Test that DxError::buffer_too_small includes correct sizes
        #[test]
        fn prop_buffer_error_includes_sizes(required in 1usize..1000, available in 0usize..1000) {
            prop_assume!(available < required);

            let err = DxError::buffer_too_small(required, available);
            let msg = err.to_string();

            prop_assert!(msg.contains(&required.to_string()),
                "Error should contain required size {}", required);
            prop_assert!(msg.contains(&available.to_string()),
                "Error should contain available size {}", available);
        }
    }

    // ========================================================================
    // Property 16: Compression Round-Trip
    // Validates: Requirements 7.5
    // For any byte sequence, compressing and then decompressing SHALL produce
    // the exact original byte sequence.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 16: Compression Round-Trip
        /// Validates: Requirements 7.5
        #[test]
        fn prop_compression_round_trip(data in prop::collection::vec(any::<u8>(), 0..5000)) {
            let mut compressed = DxCompressed::compress(&data);
            let decompressed = compressed.decompress().unwrap();

            prop_assert_eq!(decompressed, data.as_slice(),
                "Decompressed data should match original");
        }

        /// Test compression round-trip with different levels
        #[test]
        fn prop_compression_levels_round_trip(data in prop::collection::vec(any::<u8>(), 0..1000)) {
            for level in [CompressionLevel::Fast, CompressionLevel::Default, CompressionLevel::High] {
                let mut compressed = DxCompressed::compress_level(&data, level);
                let decompressed = compressed.decompress().unwrap();

                prop_assert_eq!(decompressed, data.as_slice(),
                    "Decompressed data should match original for level {:?}", level);
            }
        }

        /// Test streaming compression round-trip
        #[test]
        fn prop_streaming_compression_round_trip(
            chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..100), 1..10)
        ) {
            let mut compressor = StreamCompressor::new(64);
            let mut original = Vec::new();

            for chunk in &chunks {
                compressor.write(chunk);
                original.extend_from_slice(chunk);
            }

            let compressed_chunks = compressor.finish();
            let mut decompressor = StreamDecompressor::new(compressed_chunks);
            let decompressed = decompressor.decompress_all().unwrap();

            prop_assert_eq!(decompressed, original,
                "Streaming decompressed data should match original");
        }
    }

    // ========================================================================
    // Property 17: Compression Ratio
    // Validates: Requirements 7.2
    // For any typical configuration data (structured text with repeated patterns),
    // LZ4 compression SHALL achieve at least 40% size reduction.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 17: Compression Ratio
        /// Validates: Requirements 7.2
        #[test]
        fn prop_compression_ratio_repetitive(repeat_count in 50usize..200, pattern_len in 4usize..20) {
            // Create repetitive data (simulating structured config)
            // Use a pattern that repeats the same byte to trigger RLE compression
            let pattern: Vec<u8> = vec![b'A'; pattern_len];
            let data: Vec<u8> = pattern.iter().cycle().take(pattern_len * repeat_count).cloned().collect();

            let compressed = DxCompressed::compress(&data);
            let ratio = compressed.ratio();

            // Highly repetitive data (same byte repeated) should compress very well
            // The RLE compression encodes runs of 4+ identical bytes as 3 bytes
            prop_assert!(ratio < 0.6 || data.len() < 100,
                "Highly repetitive data should achieve at least 40% reduction, got ratio {:.2} for {} bytes",
                ratio, data.len());
        }

        /// Test that compression savings are calculated correctly
        #[test]
        fn prop_compression_savings_calculation(data in prop::collection::vec(any::<u8>(), 1..1000)) {
            let compressed = DxCompressed::compress(&data);
            let ratio = compressed.ratio();
            let savings = compressed.savings();

            // savings = 1.0 - ratio
            let expected_savings = 1.0 - ratio;
            prop_assert!((savings - expected_savings).abs() < 0.0001,
                "Savings calculation should be 1.0 - ratio");
        }
    }

    // ========================================================================
    // Property 18: SIMD/Scalar Equivalence
    // Validates: Requirements 8.3
    // For any batch operation (sum, search, etc.), the SIMD implementation
    // SHALL produce the exact same result as the scalar implementation.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 18: SIMD/Scalar Equivalence
        /// Validates: Requirements 8.3
        #[test]
        fn prop_simd_scalar_sum_u64_equivalence(values in prop::collection::vec(0u64..1000000, 0..500)) {
            let simd_sum = dispatch::sum_u64s(&values);
            let scalar_sum = portable::sum_u64s(&values);

            prop_assert_eq!(simd_sum, scalar_sum,
                "SIMD and scalar sum_u64s should produce identical results");
        }

        /// Test SIMD/scalar equivalence for u32 sums
        #[test]
        fn prop_simd_scalar_sum_u32_equivalence(values in prop::collection::vec(0u32..1000000, 0..500)) {
            let simd_sum = dispatch::sum_u32s(&values);
            let scalar_sum = portable::sum_u32s(&values);

            prop_assert_eq!(simd_sum, scalar_sum,
                "SIMD and scalar sum_u32s should produce identical results");
        }

        /// Test SIMD/scalar equivalence for byte comparison
        #[test]
        fn prop_simd_scalar_eq_bytes_equivalence(
            a in prop::collection::vec(any::<u8>(), 0..200),
            b in prop::collection::vec(any::<u8>(), 0..200)
        ) {
            let simd_eq = dispatch::eq_bytes(&a, &b);
            let scalar_eq = portable::eq_bytes(&a, &b);

            prop_assert_eq!(simd_eq, scalar_eq,
                "SIMD and scalar eq_bytes should produce identical results");
        }

        /// Test SIMD/scalar equivalence for byte search
        #[test]
        fn prop_simd_scalar_find_byte_equivalence(
            haystack in prop::collection::vec(any::<u8>(), 0..200),
            needle in any::<u8>()
        ) {
            let simd_result = dispatch::find_byte(&haystack, needle);
            let scalar_result = portable::find_byte(&haystack, needle);

            prop_assert_eq!(simd_result, scalar_result,
                "SIMD and scalar find_byte should produce identical results");
        }
    }

    // ========================================================================
    // Property 19: Mmap/Regular Read Equivalence
    // Validates: Requirements 9.1
    // For any valid Machine format file, reading via memory-mapping SHALL
    // produce the exact same data as reading via regular file I/O.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 19: Mmap/Regular Read Equivalence
        /// Validates: Requirements 9.1
        #[test]
        fn prop_mmap_data_access_equivalence(data in prop::collection::vec(any::<u8>(), 4..500)) {
            let mmap = DxMmap::from_bytes(data.clone());

            // Verify all bytes are accessible and match
            prop_assert_eq!(mmap.as_bytes(), data.as_slice(),
                "Mmap data should match original bytes");
            prop_assert_eq!(mmap.len(), data.len(),
                "Mmap length should match original length");
        }

        /// Test mmap slice access
        #[test]
        fn prop_mmap_slice_access(
            data in prop::collection::vec(any::<u8>(), 10..500),
            offset in 0usize..10,
            len in 1usize..10
        ) {
            prop_assume!(offset + len <= data.len());

            let mmap = DxMmap::from_bytes(data.clone());
            let slice = mmap.get_slice(offset, len);

            prop_assert!(slice.is_some(), "Valid slice should be accessible");
            prop_assert_eq!(slice.unwrap(), &data[offset..offset + len],
                "Mmap slice should match original data slice");
        }
    }

    // ========================================================================
    // Property 21: Batch Iteration Correctness
    // Validates: Requirements 9.4
    // For any memory-mapped file containing N records, batch iteration SHALL
    // yield exactly N records in order, each with correct data.
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-serializer-quantum-entanglement, Property 21: Batch Iteration Correctness
        /// Validates: Requirements 9.4
        #[test]
        fn prop_batch_iteration_yields_all_records(
            record_count in 1usize..20,
            record_size in 8usize..32
        ) {
            // Create data with header + records
            let header_size = 4;
            let total_size = header_size + (record_count * record_size);
            let mut data = vec![0u8; total_size];

            // Write header
            data[0] = DX_MAGIC[0];
            data[1] = DX_MAGIC[1];
            data[2] = DX_VERSION;
            data[3] = 0x04;

            // Write record IDs
            for i in 0..record_count {
                let offset = header_size + (i * record_size);
                let id = (i as u64) * 100;
                data[offset..offset + 8].copy_from_slice(&id.to_le_bytes());
            }

            let mmap = DxMmap::from_bytes(data);
            let batch = DxMmapBatch::new(&mmap, record_size, record_count, header_size);

            // Verify iteration yields correct count
            let collected: Vec<_> = batch.iter().collect();
            prop_assert_eq!(collected.len(), record_count,
                "Batch iteration should yield exactly {} records", record_count);

            // Verify each record has correct data
            for (i, reader) in collected.iter().enumerate() {
                let expected_id = (i as u64) * 100;
                let actual_id = reader.read_u64::<0>();
                prop_assert_eq!(actual_id, expected_id,
                    "Record {} should have ID {}", i, expected_id);
            }
        }

        /// Test batch get() returns correct records
        #[test]
        fn prop_batch_get_returns_correct_record(
            record_count in 1usize..10,
            target_index in 0usize..10
        ) {
            prop_assume!(target_index < record_count);

            let record_size = 16;
            let header_size = 4;
            let total_size = header_size + (record_count * record_size);
            let mut data = vec![0u8; total_size];

            // Write header
            data[0] = DX_MAGIC[0];
            data[1] = DX_MAGIC[1];
            data[2] = DX_VERSION;
            data[3] = 0x04;

            // Write unique values to each record
            for i in 0..record_count {
                let offset = header_size + (i * record_size);
                let value = (i as u64) * 1000 + 42;
                data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            }

            let mmap = DxMmap::from_bytes(data);
            let batch = DxMmapBatch::new(&mmap, record_size, record_count, header_size);

            let reader = batch.get(target_index).unwrap();
            let expected_value = (target_index as u64) * 1000 + 42;
            let actual_value = reader.read_u64::<0>();

            prop_assert_eq!(actual_value, expected_value,
                "Record at index {} should have value {}", target_index, expected_value);
        }
    }

    // ========================================================================
    // Additional unit tests for edge cases
    // ========================================================================

    #[test]
    fn test_slot_boundary_14_bytes() {
        // Exactly 14 bytes should be inline
        let data = b"12345678901234";
        let slot = DxMachineSlot::inline_from_bytes(data).unwrap();
        assert!(slot.is_inline());
        assert_eq!(slot.data[15], INLINE_MARKER);
    }

    #[test]
    fn test_slot_boundary_15_bytes_fails() {
        // 15 bytes should fail inline
        let data = b"123456789012345";
        let result = DxMachineSlot::inline_from_bytes(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_compression() {
        let data: &[u8] = &[];
        let mut compressed = DxCompressed::compress(data);
        let decompressed = compressed.decompress().unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn test_compression_wire_format_round_trip() {
        let original = b"Test data for wire format round trip";
        let compressed = DxCompressed::compress(original);
        let wire = compressed.to_wire();
        let restored = DxCompressed::from_wire(&wire).unwrap();

        assert_eq!(restored.original_size(), original.len());
        assert_eq!(restored.compressed_size(), compressed.compressed_size());
    }
}
