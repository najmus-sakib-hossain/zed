//! DX-Machine in-place serialization builder

use super::header::DxMachineHeader;
use super::slot::{DxMachineSlot, MAX_INLINE_SIZE};

/// Builder for in-place DX-Machine serialization using RKYV
///
/// This builder writes directly to a pre-allocated buffer without
/// any intermediate allocations or copying. Serialization time is
/// effectively zero since it's just memory writes.
pub struct DxMachineBuilder<'a> {
    /// The target buffer
    buffer: &'a mut Vec<u8>,
    /// Current write position for heap data
    heap_cursor: usize,
    /// Header (written at position 0)
    header: DxMachineHeader,
}

impl<'a> DxMachineBuilder<'a> {
    /// Create a new builder with buffer
    ///
    /// Initializes the header and reserves space for fixed fields + slots.
    /// The `fixed_size` is the size of all primitive fields.
    /// The `slot_count` is the number of variable-length fields.
    #[inline]
    pub fn new(buffer: &'a mut Vec<u8>, fixed_size: usize, slot_count: usize) -> Self {
        let header = DxMachineHeader::new();

        // Calculate heap start offset
        let heap_offset = DxMachineHeader::size() + fixed_size + (slot_count * 16);

        // Reserve space
        buffer.clear();
        buffer.reserve(heap_offset + 256); // Reserve extra for heap

        // Write header placeholder
        buffer.extend_from_slice(&[0u8; 4]);

        // Initialize fixed section with zeros
        buffer.resize(heap_offset, 0);

        Self {
            buffer,
            heap_cursor: heap_offset,
            header,
        }
    }

    /// Write fixed field at offset (direct memory write)
    #[inline]
    pub fn write_fixed<T: Copy>(&mut self, offset: usize, value: T) {
        // SAFETY: Creating a byte view of a Copy type T.
        // &value is a valid reference to T, and we're creating a byte slice of exactly size_of::<T>() bytes.
        // This is safe because T is Copy, so it has no drop glue or references.
        let bytes = unsafe {
            std::slice::from_raw_parts(&value as *const T as *const u8, std::mem::size_of::<T>())
        };

        let start = DxMachineHeader::size() + offset;
        self.buffer[start..start + bytes.len()].copy_from_slice(bytes);
    }

    /// Write u8 at offset
    #[inline]
    pub fn write_u8(&mut self, offset: usize, value: u8) {
        self.buffer[DxMachineHeader::size() + offset] = value;
    }

    /// Write u16 at offset (little-endian)
    #[inline]
    pub fn write_u16(&mut self, offset: usize, value: u16) {
        let start = DxMachineHeader::size() + offset;
        self.buffer[start..start + 2].copy_from_slice(&value.to_le_bytes());
    }

    /// Write u32 at offset (little-endian)
    #[inline]
    pub fn write_u32(&mut self, offset: usize, value: u32) {
        let start = DxMachineHeader::size() + offset;
        self.buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Write u64 at offset (little-endian)
    #[inline]
    pub fn write_u64(&mut self, offset: usize, value: u64) {
        let start = DxMachineHeader::size() + offset;
        self.buffer[start..start + 8].copy_from_slice(&value.to_le_bytes());
    }

    /// Write i8 at offset
    #[inline]
    pub fn write_i8(&mut self, offset: usize, value: i8) {
        self.write_u8(offset, value as u8);
    }

    /// Write i16 at offset (little-endian)
    #[inline]
    pub fn write_i16(&mut self, offset: usize, value: i16) {
        self.write_u16(offset, value as u16);
    }

    /// Write i32 at offset (little-endian)
    #[inline]
    pub fn write_i32(&mut self, offset: usize, value: i32) {
        self.write_u32(offset, value as u32);
    }

    /// Write i64 at offset (little-endian)
    #[inline]
    pub fn write_i64(&mut self, offset: usize, value: i64) {
        self.write_u64(offset, value as u64);
    }

    /// Write f32 at offset (little-endian)
    #[inline]
    pub fn write_f32(&mut self, offset: usize, value: f32) {
        let start = DxMachineHeader::size() + offset;
        self.buffer[start..start + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Write f64 at offset (little-endian)
    #[inline]
    pub fn write_f64(&mut self, offset: usize, value: f64) {
        let start = DxMachineHeader::size() + offset;
        self.buffer[start..start + 8].copy_from_slice(&value.to_le_bytes());
    }

    /// Write bool at offset
    #[inline]
    pub fn write_bool(&mut self, offset: usize, value: bool) {
        self.write_u8(offset, value as u8);
    }

    /// Write string to slot (auto inline/heap optimization)
    #[inline]
    pub fn write_string(&mut self, slot_offset: usize, value: &str) {
        self.write_bytes(slot_offset, value.as_bytes());
    }

    /// Write bytes to slot (auto inline/heap optimization)
    #[inline]
    pub fn write_bytes(&mut self, slot_offset: usize, bytes: &[u8]) {
        let slot_pos = DxMachineHeader::size() + slot_offset;

        if bytes.len() <= MAX_INLINE_SIZE {
            // Inline: fits in slot
            let mut slot = DxMachineSlot::new();
            // SAFETY: We just checked bytes.len() <= MAX_INLINE_SIZE (14 bytes),
            // so write_inline will always succeed
            if slot.write_inline(bytes).is_err() {
                // This should never happen given the length check above
                return;
            }
            self.buffer[slot_pos..slot_pos + 16].copy_from_slice(&slot.data);
        } else {
            // Heap: write to heap section
            let heap_start = self.heap_cursor - (DxMachineHeader::size() + slot_offset);
            let offset = (self.heap_cursor - heap_start) as u32;

            let slot = DxMachineSlot::heap_reference(offset, bytes.len() as u32);
            self.buffer[slot_pos..slot_pos + 16].copy_from_slice(&slot.data);

            // Write data to heap
            self.buffer.extend_from_slice(bytes);
            self.heap_cursor += bytes.len();
            self.header.set_has_heap(true);
        }
    }

    /// Write array to slot (similar to bytes)
    #[inline]
    pub fn write_array<T: Copy>(&mut self, slot_offset: usize, values: &[T]) {
        // SAFETY: Creating a byte view of a slice of Copy types.
        // values.as_ptr() is a valid pointer to the slice, and we're creating a byte slice
        // of exactly size_of_val(values) bytes. This is safe because T is Copy.
        let bytes = unsafe {
            std::slice::from_raw_parts(values.as_ptr() as *const u8, std::mem::size_of_val(values))
        };
        self.write_bytes(slot_offset, bytes);
    }

    /// Finalize and return the serialized length
    #[inline]
    pub fn finish(self) -> usize {
        // Write header at beginning
        self.header.write_to(&mut self.buffer[0..4]);

        // Shrink to actual size
        self.buffer.truncate(self.heap_cursor);

        self.heap_cursor
    }

    /// Get current heap cursor position
    #[inline]
    pub fn heap_position(&self) -> usize {
        self.heap_cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mut buffer = Vec::new();
        let mut builder = DxMachineBuilder::new(&mut buffer, 8, 1); // 8 bytes fixed, 1 slot

        builder.write_u64(0, 12345);
        builder.write_string(8, "test");

        let _size = builder.finish();

        assert!(_size > 0);
        assert_eq!(buffer[0], 0x5A); // Magic
        assert_eq!(buffer[1], 0x44);
    }

    #[test]
    fn test_builder_inline_string() {
        let mut buffer = Vec::new();
        let mut builder = DxMachineBuilder::new(&mut buffer, 0, 1);

        builder.write_string(0, "Hello"); // 5 bytes, inline

        let _size = builder.finish();

        // Check slot is inline
        let slot_data = &buffer[4..20];
        assert_eq!(slot_data[0], 5); // Length
        assert_eq!(slot_data[15], 0x00); // Inline marker
        assert_eq!(&slot_data[1..6], b"Hello");
    }

    #[test]
    fn test_builder_heap_string() {
        let mut buffer = Vec::new();
        let mut builder = DxMachineBuilder::new(&mut buffer, 0, 1);

        let long_str = "This is a very long string that exceeds 14 bytes";
        builder.write_string(0, long_str);

        let size = builder.finish();

        // Check slot is heap reference
        let slot_data = &buffer[4..20];
        assert_eq!(slot_data[15], 0xFF); // Heap marker

        // Check heap data exists
        assert!(size > 20);
    }

    #[test]
    #[allow(clippy::approx_constant)] // Using 3.14 intentionally for test data
    fn test_builder_multiple_fields() {
        let mut buffer = Vec::new();
        let mut builder = DxMachineBuilder::new(&mut buffer, 17, 2);

        builder.write_u64(0, 999);
        builder.write_u32(8, 42);
        builder.write_bool(12, true);
        builder.write_f32(13, 3.14);
        builder.write_string(17, "name");
        builder.write_string(33, "email");

        let _size = builder.finish();
        assert!(_size > 0);
    }

    #[test]
    fn test_builder_primitive_types() {
        let mut buffer = Vec::new();
        let mut builder = DxMachineBuilder::new(&mut buffer, 30, 0);

        builder.write_u8(0, 255);
        builder.write_i8(1, -128);
        builder.write_u16(2, 65535);
        builder.write_i16(4, -32768);
        builder.write_u32(6, 4294967295);
        builder.write_i32(10, -2147483648);
        builder.write_u64(14, u64::MAX);
        builder.write_i64(22, i64::MIN);

        builder.finish();

        // Verify values are written correctly
        assert_eq!(buffer[4], 255); // u8
        assert_eq!(buffer[5] as i8, -128); // i8
    }
}
