//! Property-based tests for SafeDeserializer.
//!
//! These tests verify the correctness properties defined in the design document:
//! - Property 1: Deserialization Safety Invariant
//! - Property 6: Safe Deserializer Round-Trip

#[cfg(test)]
mod property_tests {
    // use crate::machine::safe_deserialize::SafeDeserializer;
    use crate::safety::SafetyError;
    use proptest::prelude::*;
    use std::mem::size_of;

    // ========================================================================
    // Property 1: Deserialization Safety Invariant
    // Feature: codebase-safety-audit, Property 1: Deserialization Safety Invariant
    // Validates: Requirements 1.1, 1.2, 1.3, 1.5
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property 1: For any type T and byte slice, deserialization succeeds iff:
        /// - slice.len() >= size_of::<T>() AND
        /// - slice.as_ptr() is aligned to align_of::<T>()
        #[test]
        fn prop_deserialize_validates_size_u32(
            buffer_len in 0usize..100,
        ) {
            let buffer = vec![0u8; buffer_len];
            let mut deserializer = SafeDeserializer::new(&buffer);

            let result: Result<&u32, _> = deserializer.read();

            if buffer_len >= size_of::<u32>() {
                // Buffer is large enough, should succeed (alignment is guaranteed for Vec)
                prop_assert!(result.is_ok(), "Expected Ok for buffer_len={}", buffer_len);
            } else {
                // Buffer too small, should fail
                prop_assert!(
                    matches!(result, Err(SafetyError::BufferTooSmall { .. })),
                    "Expected BufferTooSmall for buffer_len={}, got {:?}",
                    buffer_len, result
                );
            }
        }

        /// Property 1: Test with u64 type (8-byte alignment)
        #[test]
        fn prop_deserialize_validates_size_u64(
            buffer_len in 0usize..100,
        ) {
            let buffer = vec![0u8; buffer_len];
            let mut deserializer = SafeDeserializer::new(&buffer);

            let result: Result<&u64, _> = deserializer.read();

            if buffer_len >= size_of::<u64>() {
                prop_assert!(result.is_ok(), "Expected Ok for buffer_len={}", buffer_len);
            } else {
                prop_assert!(
                    matches!(result, Err(SafetyError::BufferTooSmall { .. })),
                    "Expected BufferTooSmall for buffer_len={}, got {:?}",
                    buffer_len, result
                );
            }
        }

        /// Property 1: Test sequential reads consume buffer correctly
        #[test]
        fn prop_sequential_reads_track_position(
            num_values in 1usize..10,
        ) {
            let buffer_size = num_values * size_of::<u32>();
            let buffer = vec![0u8; buffer_size];
            let mut deserializer = SafeDeserializer::new(&buffer);

            for i in 0..num_values {
                let result: Result<&u32, _> = deserializer.read();
                prop_assert!(result.is_ok(), "Read {} should succeed", i);
                prop_assert_eq!(
                    deserializer.position(),
                    (i + 1) * size_of::<u32>(),
                    "Position after read {}", i
                );
            }

            // Next read should fail (buffer exhausted)
            let result: Result<&u32, _> = deserializer.read();
            prop_assert!(result.is_err(), "Read past end should fail");
        }

        /// Property 1: Test read_slice validates count * size
        #[test]
        fn prop_read_slice_validates_total_size(
            buffer_len in 0usize..100,
            count in 0usize..20,
        ) {
            // Use an aligned buffer to avoid alignment issues
            // Vec<u32> guarantees proper alignment for u32
            let aligned_buffer: Vec<u32> = vec![0u32; buffer_len.div_ceil(4)];
            // SAFETY: Creating a byte view of the aligned buffer.
            // aligned_buffer is a valid Vec<u32>, so its pointer and length are valid.
            // We're creating a byte slice view that doesn't exceed the buffer bounds.
            let buffer = unsafe {
                std::slice::from_raw_parts(
                    aligned_buffer.as_ptr() as *const u8,
                    buffer_len.min(aligned_buffer.len() * 4)
                )
            };
            let actual_buffer_len = buffer.len();
            let mut deserializer = SafeDeserializer::new(buffer);

            let result: Result<&[u32], _> = deserializer.read_slice(count);
            let needed = count * size_of::<u32>();

            if actual_buffer_len >= needed {
                prop_assert!(result.is_ok(), "Expected Ok for buffer_len={}, count={}", actual_buffer_len, count);
                prop_assert_eq!(result.unwrap().len(), count);
            } else {
                prop_assert!(result.is_err(), "Expected error for buffer_len={}, count={}", actual_buffer_len, count);
            }
        }

        /// Property 1: Test that misaligned buffers are rejected
        #[test]
        fn prop_misaligned_buffer_rejected(
            base_len in 16usize..100,
            misalign in 1usize..8,
        ) {
            // Create an aligned buffer
            let aligned: Vec<u64> = vec![0; (base_len / 8) + 2];
            let base_ptr = aligned.as_ptr() as *const u8;

            // SAFETY: We're creating a test slice to verify misalignment detection.
            // base_ptr points to a valid Vec<u64>, and we're creating a slice within its bounds.
            // The misalignment is intentional to test the safety checks.
            // Create a misaligned slice
            let slice = unsafe {
                let ptr = base_ptr.add(misalign);
                let len = base_len.saturating_sub(misalign);
                std::slice::from_raw_parts(ptr, len.min(aligned.len() * 8 - misalign))
            };

            let mut deserializer = SafeDeserializer::new(slice);
            let result: Result<&u64, _> = deserializer.read();

            // Should fail due to misalignment (u64 requires 8-byte alignment)
            if slice.len() >= 8 {
                prop_assert!(
                    matches!(result, Err(SafetyError::Misaligned { .. })),
                    "Expected Misaligned for misalign={}, got {:?}",
                    misalign, result
                );
            }
        }
    }

    // ========================================================================
    // Property 6: Safe Deserializer Round-Trip
    // Feature: codebase-safety-audit, Property 6: Safe Deserializer Round-Trip
    // Validates: Requirements 1.1, 1.2
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(500))]

        /// Property 6: Round-trip for u32 values
        #[test]
        fn prop_roundtrip_u32(value: u32) {
            let bytes = value.to_le_bytes();
            let mut deserializer = SafeDeserializer::new(&bytes);

            let read_value: &u32 = deserializer.read().unwrap();
            prop_assert_eq!(*read_value, value);
        }

        /// Property 6: Round-trip for u64 values
        #[test]
        fn prop_roundtrip_u64(value: u64) {
            let bytes = value.to_le_bytes();
            let mut deserializer = SafeDeserializer::new(&bytes);

            let read_value: &u64 = deserializer.read().unwrap();
            prop_assert_eq!(*read_value, value);
        }

        /// Property 6: Round-trip for i64 values
        #[test]
        fn prop_roundtrip_i64(value: i64) {
            let bytes = value.to_le_bytes();
            let mut deserializer = SafeDeserializer::new(&bytes);

            let read_value: &i64 = deserializer.read().unwrap();
            prop_assert_eq!(*read_value, value);
        }

        /// Property 6: Round-trip for f64 values
        #[test]
        fn prop_roundtrip_f64(value: f64) {
            let bytes = value.to_le_bytes();
            let mut deserializer = SafeDeserializer::new(&bytes);

            let read_value: &f64 = deserializer.read().unwrap();
            // Use to_bits for comparison to handle NaN correctly
            prop_assert_eq!(read_value.to_bits(), value.to_bits());
        }

        /// Property 6: Round-trip for slice of u32 values
        #[test]
        fn prop_roundtrip_slice_u32(values in prop::collection::vec(any::<u32>(), 0..20)) {
            // Create a properly aligned buffer
            let buffer: Vec<u32> = values.clone();
            // SAFETY: Creating a byte view of a valid u32 slice.
            // buffer is a valid Vec<u32>, so its pointer and length are valid.
            // We're creating a byte slice view of exactly buffer.len() * 4 bytes.
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    buffer.as_ptr() as *const u8,
                    buffer.len() * size_of::<u32>()
                )
            };

            let mut deserializer = SafeDeserializer::new(bytes);
            let read_values: &[u32] = deserializer.read_slice(values.len()).unwrap();

            prop_assert_eq!(read_values, values.as_slice());
        }
    }

    // ========================================================================
    // Additional Safety Tests
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Test that skip validates bounds
        #[test]
        fn prop_skip_validates_bounds(
            buffer_len in 0usize..100,
            skip_len in 0usize..150,
        ) {
            let buffer = vec![0u8; buffer_len];
            let mut deserializer = SafeDeserializer::new(&buffer);

            let result = deserializer.skip(skip_len);

            if skip_len <= buffer_len {
                prop_assert!(result.is_ok());
                prop_assert_eq!(deserializer.position(), skip_len);
            } else {
                prop_assert!(result.is_err());
            }
        }

        /// Test that seek validates bounds
        #[test]
        fn prop_seek_validates_bounds(
            buffer_len in 0usize..100,
            seek_pos in 0usize..150,
        ) {
            let buffer = vec![0u8; buffer_len];
            let mut deserializer = SafeDeserializer::new(&buffer);

            let result = deserializer.seek(seek_pos);

            if seek_pos <= buffer_len {
                prop_assert!(result.is_ok());
                prop_assert_eq!(deserializer.position(), seek_pos);
            } else {
                prop_assert!(result.is_err());
            }
        }

        /// Test remaining() is always accurate
        #[test]
        fn prop_remaining_accurate(
            buffer_len in 0usize..100,
            reads in 0usize..10,
        ) {
            let buffer = vec![0u8; buffer_len];
            let mut deserializer = SafeDeserializer::new(&buffer);

            prop_assert_eq!(deserializer.remaining(), buffer_len);

            let actual_reads = reads.min(buffer_len);
            for _ in 0..actual_reads {
                let _ = deserializer.read_bytes(1);
            }

            prop_assert_eq!(
                deserializer.remaining(),
                buffer_len.saturating_sub(actual_reads)
            );
        }
    }
}
