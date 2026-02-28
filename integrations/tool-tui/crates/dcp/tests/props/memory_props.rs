//! Property tests for shared memory argument integrity.
//!
//! Feature: dcp-protocol, Property 4: Shared Memory Argument Integrity

use proptest::prelude::*;

use dcp::dispatch::SharedArgs;
use dcp::DCPError;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 4: Shared Memory Argument Integrity
    /// For any arguments written to SharedArrayBuffer, reading those arguments
    /// from the server side SHALL return identical values, and out-of-bounds
    /// access SHALL return an error rather than panicking.
    #[test]
    fn prop_shared_args_i32_integrity(
        values in prop::collection::vec(any::<i32>(), 1..20),
    ) {
        // Write i32 values to buffer
        let mut buffer = Vec::new();
        for value in &values {
            buffer.extend_from_slice(&value.to_le_bytes());
        }

        let args = SharedArgs::new(&buffer, 0);

        // Read back and verify
        for (i, expected) in values.iter().enumerate() {
            let offset = i * 4;
            let actual = args.read_i32_at(offset).unwrap();
            prop_assert_eq!(actual, *expected);
        }
    }

    /// Test i64 read integrity
    #[test]
    fn prop_shared_args_i64_integrity(
        values in prop::collection::vec(any::<i64>(), 1..20),
    ) {
        let mut buffer = Vec::new();
        for value in &values {
            buffer.extend_from_slice(&value.to_le_bytes());
        }

        let args = SharedArgs::new(&buffer, 0);

        for (i, expected) in values.iter().enumerate() {
            let offset = i * 8;
            let actual = args.read_i64_at(offset).unwrap();
            prop_assert_eq!(actual, *expected);
        }
    }

    /// Test u32 read integrity
    #[test]
    fn prop_shared_args_u32_integrity(
        values in prop::collection::vec(any::<u32>(), 1..20),
    ) {
        let mut buffer = Vec::new();
        for value in &values {
            buffer.extend_from_slice(&value.to_le_bytes());
        }

        let args = SharedArgs::new(&buffer, 0);

        for (i, expected) in values.iter().enumerate() {
            let offset = i * 4;
            let actual = args.read_u32_at(offset).unwrap();
            prop_assert_eq!(actual, *expected);
        }
    }

    /// Test f64 read integrity
    #[test]
    fn prop_shared_args_f64_integrity(
        values in prop::collection::vec(
            any::<f64>().prop_filter("must be finite", |f| f.is_finite()),
            1..20
        ),
    ) {
        let mut buffer = Vec::new();
        for value in &values {
            buffer.extend_from_slice(&value.to_le_bytes());
        }

        let args = SharedArgs::new(&buffer, 0);

        for (i, expected) in values.iter().enumerate() {
            let offset = i * 8;
            let actual = args.read_f64_at(offset).unwrap();
            prop_assert!((actual - expected).abs() < f64::EPSILON);
        }
    }

    /// Test bool read integrity
    #[test]
    fn prop_shared_args_bool_integrity(
        values in prop::collection::vec(any::<bool>(), 1..100),
    ) {
        let buffer: Vec<u8> = values.iter().map(|&b| if b { 1 } else { 0 }).collect();

        let args = SharedArgs::new(&buffer, 0);

        for (i, expected) in values.iter().enumerate() {
            let actual = args.read_bool_at(i).unwrap();
            prop_assert_eq!(actual, *expected);
        }
    }

    /// Test string read integrity
    #[test]
    fn prop_shared_args_string_integrity(
        strings in prop::collection::vec("[a-zA-Z0-9]{1,50}", 1..10),
    ) {
        // Build buffer with strings
        let mut buffer = Vec::new();
        let mut offsets = Vec::new();

        for s in &strings {
            offsets.push((buffer.len(), s.len()));
            buffer.extend_from_slice(s.as_bytes());
        }

        let args = SharedArgs::new(&buffer, 0);

        // Read back and verify
        for (i, (offset, len)) in offsets.iter().enumerate() {
            let actual = args.read_str_at(*offset, *len).unwrap();
            prop_assert_eq!(actual, strings[i].as_str());
        }
    }

    /// Test bytes read integrity
    #[test]
    fn prop_shared_args_bytes_integrity(
        data in prop::collection::vec(any::<u8>(), 1..1000),
        read_offset in 0usize..500,
        read_len in 1usize..100,
    ) {
        let args = SharedArgs::new(&data, 0);

        if read_offset + read_len <= data.len() {
            // Valid read
            let actual = args.read_bytes_at(read_offset, read_len).unwrap();
            prop_assert_eq!(actual, &data[read_offset..read_offset + read_len]);
        } else {
            // Out of bounds
            let result = args.read_bytes_at(read_offset, read_len);
            prop_assert_eq!(result, Err(DCPError::OutOfBounds));
        }
    }

    /// Test out-of-bounds i32 access returns error
    #[test]
    fn prop_shared_args_i32_bounds_check(
        buffer_size in 0usize..100,
        offset in 0usize..200,
    ) {
        let buffer = vec![0u8; buffer_size];
        let args = SharedArgs::new(&buffer, 0);

        let result = args.read_i32_at(offset);

        if offset + 4 <= buffer_size {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::OutOfBounds));
        }
    }

    /// Test out-of-bounds i64 access returns error
    #[test]
    fn prop_shared_args_i64_bounds_check(
        buffer_size in 0usize..100,
        offset in 0usize..200,
    ) {
        let buffer = vec![0u8; buffer_size];
        let args = SharedArgs::new(&buffer, 0);

        let result = args.read_i64_at(offset);

        if offset + 8 <= buffer_size {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::OutOfBounds));
        }
    }

    /// Test out-of-bounds f64 access returns error
    #[test]
    fn prop_shared_args_f64_bounds_check(
        buffer_size in 0usize..100,
        offset in 0usize..200,
    ) {
        let buffer = vec![0u8; buffer_size];
        let args = SharedArgs::new(&buffer, 0);

        let result = args.read_f64_at(offset);

        if offset + 8 <= buffer_size {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::OutOfBounds));
        }
    }

    /// Test out-of-bounds bool access returns error
    #[test]
    fn prop_shared_args_bool_bounds_check(
        buffer_size in 0usize..100,
        offset in 0usize..200,
    ) {
        let buffer = vec![0u8; buffer_size];
        let args = SharedArgs::new(&buffer, 0);

        let result = args.read_bool_at(offset);

        if offset < buffer_size {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::OutOfBounds));
        }
    }

    /// Test out-of-bounds string access returns error
    #[test]
    fn prop_shared_args_str_bounds_check(
        buffer_size in 0usize..100,
        offset in 0usize..200,
        len in 1usize..100,
    ) {
        let buffer = vec![b'a'; buffer_size]; // Valid UTF-8
        let args = SharedArgs::new(&buffer, 0);

        let result = args.read_str_at(offset, len);

        if offset + len <= buffer_size {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::OutOfBounds));
        }
    }

    /// Test layout field is preserved
    #[test]
    fn prop_shared_args_layout_preserved(
        layout in any::<u64>(),
        data in prop::collection::vec(any::<u8>(), 0..100),
    ) {
        let args = SharedArgs::new(&data, layout);
        prop_assert_eq!(args.layout(), layout);
    }

    /// Test data reference is preserved
    #[test]
    fn prop_shared_args_data_preserved(
        data in prop::collection::vec(any::<u8>(), 0..100),
    ) {
        let args = SharedArgs::new(&data, 0);
        prop_assert_eq!(args.data(), &data[..]);
    }

    /// Test mixed type reads from same buffer
    #[test]
    fn prop_shared_args_mixed_types(
        i32_val in any::<i32>(),
        i64_val in any::<i64>(),
        f64_val in any::<f64>().prop_filter("finite", |f| f.is_finite()),
        bool_val in any::<bool>(),
    ) {
        // Layout: i32 (4) + i64 (8) + f64 (8) + bool (1) = 21 bytes
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&i32_val.to_le_bytes());
        buffer.extend_from_slice(&i64_val.to_le_bytes());
        buffer.extend_from_slice(&f64_val.to_le_bytes());
        buffer.push(if bool_val { 1 } else { 0 });

        let args = SharedArgs::new(&buffer, 0);

        prop_assert_eq!(args.read_i32_at(0).unwrap(), i32_val);
        prop_assert_eq!(args.read_i64_at(4).unwrap(), i64_val);
        prop_assert!((args.read_f64_at(12).unwrap() - f64_val).abs() < f64::EPSILON);
        prop_assert_eq!(args.read_bool_at(20).unwrap(), bool_val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_buffer() {
        let args = SharedArgs::new(&[], 0);

        assert_eq!(args.read_i32_at(0), Err(DCPError::OutOfBounds));
        assert_eq!(args.read_i64_at(0), Err(DCPError::OutOfBounds));
        assert_eq!(args.read_f64_at(0), Err(DCPError::OutOfBounds));
        assert_eq!(args.read_bool_at(0), Err(DCPError::OutOfBounds));
        assert_eq!(args.read_str_at(0, 1), Err(DCPError::OutOfBounds));
        assert_eq!(args.read_bytes_at(0, 1), Err(DCPError::OutOfBounds));
    }

    #[test]
    fn test_invalid_utf8() {
        let buffer = vec![0xFF, 0xFE]; // Invalid UTF-8
        let args = SharedArgs::new(&buffer, 0);

        let result = args.read_str_at(0, 2);
        assert_eq!(result, Err(DCPError::ValidationFailed));
    }

    #[test]
    fn test_exact_boundary_reads() {
        let buffer = vec![1, 2, 3, 4]; // Exactly 4 bytes
        let args = SharedArgs::new(&buffer, 0);

        // Should succeed at offset 0
        assert!(args.read_i32_at(0).is_ok());

        // Should fail at offset 1 (would need bytes 1-4, but only have 1-3)
        assert_eq!(args.read_i32_at(1), Err(DCPError::OutOfBounds));
    }
}
