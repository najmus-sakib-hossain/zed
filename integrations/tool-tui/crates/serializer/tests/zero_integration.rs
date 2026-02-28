//! Integration tests for DX-Zero format

use serializer::zero::{
    DxFormat, DxZeroBuilder, DxZeroHeader, DxZeroSlot, FLAG_HAS_HEAP, FLAG_LITTLE_ENDIAN,
    HEAP_MARKER, INLINE_MARKER, detect_format,
};

#[test]
fn test_header_creation_and_validation() {
    let header = DxZeroHeader::new();

    assert_eq!(header.magic, [0x5A, 0x44]);
    assert_eq!(header.version, 0x01);
    assert!(header.flags & FLAG_LITTLE_ENDIAN != 0);

    assert!(header.validate().is_ok());
}

#[test]
fn test_slot_inline() {
    let data = b"Hello";
    let slot = DxZeroSlot::inline_from_bytes(data).unwrap();

    assert!(slot.is_inline());
    assert!(!slot.is_heap());
    assert_eq!(slot.inline_len(), 5);
    assert_eq!(slot.inline_data(), data);
    assert_eq!(slot.inline_str(), "Hello");
}

#[test]
fn test_slot_heap() {
    let slot = DxZeroSlot::heap_reference(100, 500);

    assert!(!slot.is_inline());
    assert!(slot.is_heap());
    assert_eq!(slot.heap_offset(), 100);
    assert_eq!(slot.heap_length(), 500);
}

#[test]
fn test_builder_simple_struct() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 13, 2);

    // Write fields
    builder.write_u64(0, 12345);
    builder.write_u32(8, 30);
    builder.write_bool(12, true);
    builder.write_string(13, "John");
    builder.write_string(29, "john@test.com");

    let size = builder.finish();

    // Verify header
    assert_eq!(buffer[0], 0x5A);
    assert_eq!(buffer[1], 0x44);
    assert_eq!(buffer[2], 0x01);

    // Verify size
    assert!(size >= 4 + 13 + 32); // header + fixed + slots
}

#[test]
fn test_inline_optimization() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

    // Small string (inline)
    builder.write_string(0, "Test");
    let size = builder.finish();

    // Check slot is inline
    let slot_data = &buffer[4..20];
    assert_eq!(slot_data[15], INLINE_MARKER);
    assert_eq!(slot_data[0], 4); // Length
    assert_eq!(&slot_data[1..5], b"Test");

    // Size should be minimal (header + slot, no heap)
    assert_eq!(size, 20);
}

#[test]
fn test_heap_allocation() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

    // Large string (heap)
    let long_string = "This is a very long string that exceeds the 14 byte inline limit";
    builder.write_string(0, long_string);
    let size = builder.finish();

    // Check slot is heap reference
    let slot_data = &buffer[4..20];
    assert_eq!(slot_data[15], HEAP_MARKER);

    // Check heap data exists - size should include header(4) + slot(16) + heap data
    // The heap data includes the string bytes
    assert!(
        size >= 20 + long_string.len(),
        "size {} should be >= {}",
        size,
        20 + long_string.len()
    );

    // Verify flags indicate heap
    assert!(buffer[3] & FLAG_HAS_HEAP != 0);
}

#[test]
fn test_all_primitive_types() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 30, 0);

    builder.write_u8(0, 255);
    builder.write_i8(1, -128);
    builder.write_u16(2, 65535);
    builder.write_i16(4, -32768);
    builder.write_u32(6, 4294967295);
    builder.write_i32(10, -2147483648);
    builder.write_f32(14, 3.14159);
    builder.write_f64(18, 2.718281828);
    builder.write_bool(26, true);
    builder.write_bool(27, false);

    builder.finish();

    // Verify values
    assert_eq!(buffer[4], 255);
    assert_eq!(buffer[5] as i8, -128);
    assert_eq!(buffer[30], 1); // true
    assert_eq!(buffer[31], 0); // false
}

#[test]
fn test_multiple_strings() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 3);

    builder.write_string(0, "Alice"); // Inline
    builder.write_string(16, "Bob"); // Inline
    builder.write_string(32, "This is a very long string for heap storage"); // Heap

    let size = builder.finish();

    // First two should be inline
    assert_eq!(buffer[4 + 15], INLINE_MARKER);
    assert_eq!(buffer[20 + 15], INLINE_MARKER);

    // Third should be heap
    assert_eq!(buffer[36 + 15], HEAP_MARKER);

    assert!(size > 52);
}

#[test]
fn test_format_detection() {
    let mut buffer = Vec::new();
    let builder = DxZeroBuilder::new(&mut buffer, 0, 0);
    builder.finish();

    assert_eq!(detect_format(&buffer), DxFormat::Zero);
}

#[test]
fn test_max_inline_size() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

    // Exactly 14 bytes (max inline)
    builder.write_string(0, "12345678901234");
    builder.finish();

    let slot_data = &buffer[4..20];
    assert_eq!(slot_data[15], INLINE_MARKER);
    assert_eq!(slot_data[0], 14);
}

#[test]
fn test_one_over_inline_size() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

    // 15 bytes (one over max inline)
    builder.write_string(0, "123456789012345");
    builder.finish();

    let slot_data = &buffer[4..20];
    assert_eq!(slot_data[15], HEAP_MARKER);
}

#[test]
fn test_empty_string() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 1);

    builder.write_string(0, "");
    builder.finish();

    let slot_data = &buffer[4..20];
    assert_eq!(slot_data[15], INLINE_MARKER);
    assert_eq!(slot_data[0], 0);
}

#[test]
fn test_mixed_inline_heap() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 8, 3);

    builder.write_u64(0, 999);
    builder.write_string(8, "Short"); // Inline
    builder.write_string(24, "Medium length string"); // Heap
    builder.write_string(40, "X"); // Inline

    builder.finish();

    // First string: inline
    assert_eq!(buffer[4 + 8 + 15], INLINE_MARKER);

    // Second string: heap
    assert_eq!(buffer[4 + 24 + 15], HEAP_MARKER);

    // Third string: inline
    assert_eq!(buffer[4 + 40 + 15], INLINE_MARKER);
}

#[test]
fn test_roundtrip_simple() {
    // Serialize
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 12, 1);

    builder.write_u64(0, 42);
    builder.write_u32(8, 100);
    builder.write_string(12, "Test");

    builder.finish();

    // Deserialize and verify
    assert_eq!(buffer[0..2], [0x5A, 0x44]);

    // Read u64
    let id = u64::from_le_bytes([
        buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9], buffer[10], buffer[11],
    ]);
    assert_eq!(id, 42);

    // Read u32
    let count = u32::from_le_bytes([buffer[12], buffer[13], buffer[14], buffer[15]]);
    assert_eq!(count, 100);

    // Read inline string
    let slot = &buffer[16..32];
    assert_eq!(slot[15], INLINE_MARKER);
    assert_eq!(slot[0], 4);
    assert_eq!(&slot[1..5], b"Test");
}

#[test]
fn test_unicode_strings() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 0, 3);

    builder.write_string(0, "Hello 世界"); // Unicode (12 bytes UTF-8)
    builder.write_string(16, "🚀 Rocket"); // Emoji (11 bytes UTF-8)
    builder.write_string(32, "Привет"); // Cyrillic (12 bytes UTF-8)

    builder.finish();

    // All should work with UTF-8
    // Header(4) + 3 slots(48) = 52 minimum
    assert!(buffer.len() >= 52, "buffer.len() {} should be >= 52", buffer.len());
}

#[test]
fn test_zero_values() {
    let mut buffer = Vec::new();
    let mut builder = DxZeroBuilder::new(&mut buffer, 21, 0);

    builder.write_u64(0, 0);
    builder.write_u32(8, 0);
    builder.write_bool(12, false);
    builder.write_f64(13, 0.0);

    builder.finish();

    // All zeros should be stored correctly
    for i in 4..25 {
        assert_eq!(buffer[i], 0);
    }
}

#[test]
fn test_max_values() {
    let mut buffer = Vec::new();
    // Need 22 bytes: u8(1) + u16(2) + u32(4) + u64(8) + i8(1) + i16(2) + i32(4) = 22
    // Offsets: 0, 1, 3, 7, 15, 16, 18 -> last write at 18 needs 4 bytes = 22 total
    let mut builder = DxZeroBuilder::new(&mut buffer, 22, 0);

    builder.write_u8(0, u8::MAX);
    builder.write_u16(1, u16::MAX);
    builder.write_u32(3, u32::MAX);
    builder.write_u64(7, u64::MAX);
    builder.write_i8(15, i8::MAX);
    builder.write_i16(16, i16::MAX);
    builder.write_i32(18, i32::MAX);

    builder.finish();

    // Verify max values stored
    assert_eq!(buffer[4], 255);
}
