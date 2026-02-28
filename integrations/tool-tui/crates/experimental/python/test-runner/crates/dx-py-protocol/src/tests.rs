//! Tests for dx-py-protocol

use super::*;
use dx_py_core::TestCase;
use proptest::prelude::*;

// Property 4: Binary Message Header Size
// Validates that headers are exactly the specified sizes

#[test]
fn test_message_header_size() {
    assert_eq!(TestMessageHeader::SIZE, 32, "TestMessageHeader must be exactly 32 bytes");
    assert_eq!(TestResultHeader::SIZE, 40, "TestResultHeader must be exactly 40 bytes");
}

proptest! {
    /// Feature: dx-py-test-runner, Property 4: Binary Message Header Size
    /// Validates: Requirements 3.1
    #[test]
    fn prop_message_header_roundtrip(
        msg_type in 1u8..=4u8,
        test_id in any::<u16>(),
        file_hash in any::<u64>(),
        payload_len in 0u32..MAX_PAYLOAD_SIZE as u32,
    ) {
        let header = TestMessageHeader {
            magic: PROTOCOL_MAGIC,
            msg_type,
            flags: 0,
            test_id,
            file_hash,
            payload_len,
            reserved: [0; 12],
        };

        let bytes = header.to_bytes();
        prop_assert_eq!(bytes.len(), TestMessageHeader::SIZE);

        let parsed = TestMessageHeader::from_bytes(&bytes).unwrap();
        prop_assert_eq!(parsed.magic, header.magic);
        prop_assert_eq!(parsed.msg_type, header.msg_type);
        prop_assert_eq!(parsed.test_id, header.test_id);
        prop_assert_eq!(parsed.file_hash, header.file_hash);
        prop_assert_eq!(parsed.payload_len, header.payload_len);
    }

    /// Feature: dx-py-test-runner, Property 5: Protocol Message Round-Trip
    /// Validates: Requirements 3.4, 3.5
    #[test]
    fn prop_test_case_roundtrip(
        name in "test_[a-z_]{1,20}",
        line in 1u32..10000u32,
    ) {
        let test = TestCase::new(&name, "test_file.py", line);

        let serialized = BinaryProtocol::serialize_test(&test).unwrap();
        let deserialized = BinaryProtocol::deserialize_test(&serialized).unwrap();

        prop_assert_eq!(test.name, deserialized.name);
        prop_assert_eq!(test.line_number, deserialized.line_number);
        prop_assert_eq!(test.file_path, deserialized.file_path);
    }

    /// Feature: dx-py-test-runner, Property 6: Protocol Error Handling
    /// Validates: Requirements 3.6
    #[test]
    fn prop_invalid_magic_rejected(
        bad_magic in any::<u32>().prop_filter("not valid magic", |m| *m != PROTOCOL_MAGIC),
    ) {
        let header = TestMessageHeader {
            magic: bad_magic,
            msg_type: MessageType::Run as u8,
            flags: 0,
            test_id: 1,
            file_hash: 12345,
            payload_len: 0,
            reserved: [0; 12],
        };

        let result = header.validate();
        prop_assert!(result.is_err());
        match result {
            Err(dx_py_core::ProtocolError::InvalidMagic(m)) => {
                prop_assert_eq!(m, bad_magic);
            }
            _ => prop_assert!(false, "Expected InvalidMagic error"),
        }
    }

    /// Feature: dx-py-test-runner, Property 6: Protocol Error Handling
    /// Validates: Requirements 3.6
    #[test]
    fn prop_invalid_message_type_rejected(
        bad_type in 5u8..=255u8,
    ) {
        let header = TestMessageHeader {
            magic: PROTOCOL_MAGIC,
            msg_type: bad_type,
            flags: 0,
            test_id: 1,
            file_hash: 12345,
            payload_len: 0,
            reserved: [0; 12],
        };

        let result = header.validate();
        prop_assert!(result.is_err());
    }

    /// Feature: dx-py-test-runner, Property 6: Protocol Error Handling
    /// Validates: Requirements 3.6
    #[test]
    fn prop_payload_too_large_rejected(
        payload_len in (MAX_PAYLOAD_SIZE as u32 + 1)..=u32::MAX,
    ) {
        let header = TestMessageHeader {
            magic: PROTOCOL_MAGIC,
            msg_type: MessageType::Run as u8,
            flags: 0,
            test_id: 1,
            file_hash: 12345,
            payload_len,
            reserved: [0; 12],
        };

        let result = header.validate();
        prop_assert!(result.is_err());
        match result {
            Err(dx_py_core::ProtocolError::PayloadTooLarge(size, max)) => {
                prop_assert_eq!(size, payload_len as usize);
                prop_assert_eq!(max, MAX_PAYLOAD_SIZE);
            }
            _ => prop_assert!(false, "Expected PayloadTooLarge error"),
        }
    }
}

// Unit tests

#[test]
fn test_result_header_roundtrip() {
    let header = TestResultHeader {
        test_id: 42,
        status: 0, // Pass
        _padding: 0,
        duration_ns: 1_000_000,
        assertions_passed: 5,
        assertions_failed: 0,
        stdout_len: 100,
        stderr_len: 0,
        traceback_len: 0,
        _reserved: [0; 8],
    };

    let bytes = header.to_bytes();
    assert_eq!(bytes.len(), TestResultHeader::SIZE);

    let parsed = TestResultHeader::from_bytes(&bytes).unwrap();
    assert_eq!(parsed.test_id, header.test_id);
    assert_eq!(parsed.status, header.status);
    assert_eq!(parsed.duration_ns, header.duration_ns);
    assert_eq!(parsed.assertions_passed, header.assertions_passed);
    assert_eq!(parsed.assertions_failed, header.assertions_failed);
}

#[test]
fn test_ring_buffer_basic() {
    let mut buffer = RingBuffer::new(1024);

    assert!(buffer.is_empty());
    assert_eq!(buffer.available(), 0);

    buffer.write(b"hello").unwrap();
    assert!(!buffer.is_empty());
    assert_eq!(buffer.available(), 5);

    let data = buffer.read(5).unwrap();
    assert_eq!(&data, b"hello");
    assert!(buffer.is_empty());
}

#[test]
fn test_ring_buffer_wraparound() {
    let mut buffer = RingBuffer::new(16);

    // Fill most of the buffer
    buffer.write(b"12345678901").unwrap();
    buffer.read(11).unwrap();

    // Write data that wraps around
    buffer.write(b"abcdefgh").unwrap();
    let data = buffer.read(8).unwrap();
    assert_eq!(&data, b"abcdefgh");
}

#[test]
fn test_ring_buffer_full() {
    let mut buffer = RingBuffer::new(8);

    // Try to write more than capacity
    let result = buffer.write(b"123456789");
    assert!(result.is_err());
}
