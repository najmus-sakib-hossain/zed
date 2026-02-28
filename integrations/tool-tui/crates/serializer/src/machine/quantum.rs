//! DX-Quantum: Compile-Time Field Offsets (COMMENTED OUT - NOT CURRENTLY USED)
//!
//! The killer feature: Compile-time field offset computation.
//! rkyv computes offsets at runtime, this approach does it at compile time.

use core::marker::PhantomData;

/// Compile-time layout descriptor for zero-copy structures
///
/// # Type Parameters
/// - `T`: The type being described
/// - `FIXED_SIZE`: Total size of fixed (primitive) fields
/// - `SLOT_COUNT`: Number of variable-length slots
#[derive(Debug, Clone, Copy)]
pub struct QuantumLayout<T, const FIXED_SIZE: usize, const SLOT_COUNT: usize> {
    _marker: PhantomData<T>,
}

impl<T, const FIXED_SIZE: usize, const SLOT_COUNT: usize> Default
    for QuantumLayout<T, FIXED_SIZE, SLOT_COUNT>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const FIXED_SIZE: usize, const SLOT_COUNT: usize> QuantumLayout<T, FIXED_SIZE, SLOT_COUNT> {
    /// Header size (magic + version + flags)
    pub const HEADER_SIZE: usize = 4;

    /// Slot size (16 bytes each for inline optimization)
    pub const SLOT_SIZE: usize = 16;

    /// Total fixed region size
    pub const TOTAL_FIXED: usize = FIXED_SIZE;

    /// Total slots region size
    pub const TOTAL_SLOTS: usize = SLOT_COUNT * 16;

    /// Heap offset (where variable data begins)
    pub const HEAP_OFFSET: usize = Self::HEADER_SIZE + FIXED_SIZE + Self::TOTAL_SLOTS;

    /// Minimum buffer size required
    pub const MIN_SIZE: usize = Self::HEAP_OFFSET;

    /// Create a new layout descriptor
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    /// Get compile-time offset for a field at given index
    #[inline(always)]
    pub const fn field_offset(field_offset_in_fixed: usize) -> usize {
        Self::HEADER_SIZE + field_offset_in_fixed
    }

    /// Get compile-time offset for a slot at given index
    #[inline(always)]
    pub const fn slot_offset(slot_index: usize) -> usize {
        Self::HEADER_SIZE + FIXED_SIZE + (slot_index * 16)
    }
}

/// Trait for types with compile-time quantum layout
pub trait QuantumType: Sized {
    /// Fixed fields size in bytes
    const FIXED_SIZE: usize;

    /// Number of variable-length slots
    const SLOT_COUNT: usize;

    /// Header size
    const HEADER_SIZE: usize = 4;

    /// Slot size
    const SLOT_SIZE: usize = 16;

    /// Total size before heap
    const MIN_SIZE: usize = Self::HEADER_SIZE + Self::FIXED_SIZE + (Self::SLOT_COUNT * 16);
}

/// Zero-copy quantum reader with compile-time offsets
///
/// All field access is a single assembly instruction with no bounds checking
/// in release builds (when offsets are compile-time constants).
#[repr(transparent)]
pub struct QuantumReader<'a> {
    /// The underlying byte slice
    data: &'a [u8],
}

impl<'a> QuantumReader<'a> {
    /// Create a new quantum reader from a byte slice
    ///
    /// # Safety
    /// Caller must ensure the byte slice contains valid DX-Machine data
    #[inline(always)]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get the underlying byte slice
    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8] {
        self.data
    }

    /// Read u8 at compile-time offset (single MOV instruction)
    #[inline(always)]
    pub const fn read_u8<const OFFSET: usize>(&self) -> u8 {
        self.data[OFFSET]
    }

    /// Read u16 at compile-time offset (single MOV instruction)
    #[inline(always)]
    pub fn read_u16<const OFFSET: usize>(&self) -> u16 {
        u16::from_le_bytes([self.data[OFFSET], self.data[OFFSET + 1]])
    }

    /// Read u32 at compile-time offset (single MOV instruction)
    #[inline(always)]
    pub fn read_u32<const OFFSET: usize>(&self) -> u32 {
        u32::from_le_bytes([
            self.data[OFFSET],
            self.data[OFFSET + 1],
            self.data[OFFSET + 2],
            self.data[OFFSET + 3],
        ])
    }

    /// Read u64 at compile-time offset (single MOV instruction)
    #[inline(always)]
    pub fn read_u64<const OFFSET: usize>(&self) -> u64 {
        u64::from_le_bytes([
            self.data[OFFSET],
            self.data[OFFSET + 1],
            self.data[OFFSET + 2],
            self.data[OFFSET + 3],
            self.data[OFFSET + 4],
            self.data[OFFSET + 5],
            self.data[OFFSET + 6],
            self.data[OFFSET + 7],
        ])
    }

    /// Read i8 at compile-time offset
    #[inline(always)]
    pub const fn read_i8<const OFFSET: usize>(&self) -> i8 {
        self.data[OFFSET] as i8
    }

    /// Read i16 at compile-time offset
    #[inline(always)]
    pub fn read_i16<const OFFSET: usize>(&self) -> i16 {
        self.read_u16::<OFFSET>() as i16
    }

    /// Read i32 at compile-time offset
    #[inline(always)]
    pub fn read_i32<const OFFSET: usize>(&self) -> i32 {
        self.read_u32::<OFFSET>() as i32
    }

    /// Read i64 at compile-time offset
    #[inline(always)]
    pub fn read_i64<const OFFSET: usize>(&self) -> i64 {
        self.read_u64::<OFFSET>() as i64
    }

    /// Read f32 at compile-time offset
    #[inline(always)]
    pub fn read_f32<const OFFSET: usize>(&self) -> f32 {
        f32::from_le_bytes([
            self.data[OFFSET],
            self.data[OFFSET + 1],
            self.data[OFFSET + 2],
            self.data[OFFSET + 3],
        ])
    }

    /// Read f64 at compile-time offset
    #[inline(always)]
    pub fn read_f64<const OFFSET: usize>(&self) -> f64 {
        f64::from_le_bytes([
            self.data[OFFSET],
            self.data[OFFSET + 1],
            self.data[OFFSET + 2],
            self.data[OFFSET + 3],
            self.data[OFFSET + 4],
            self.data[OFFSET + 5],
            self.data[OFFSET + 6],
            self.data[OFFSET + 7],
        ])
    }

    /// Read bool at compile-time offset
    #[inline(always)]
    pub const fn read_bool<const OFFSET: usize>(&self) -> bool {
        self.data[OFFSET] != 0
    }

    /// Read inline string at compile-time slot offset (no pointer chase!)
    ///
    /// This is the key optimization: inline strings don't require pointer dereferencing.
    #[inline(always)]
    pub fn read_inline_str<const SLOT_OFFSET: usize>(&self) -> Option<&str> {
        // Check if inline (marker byte at offset + 15)
        if self.data[SLOT_OFFSET + 15] != 0x00 {
            return None; // Heap string
        }

        let len = self.data[SLOT_OFFSET] as usize;
        if len > 14 {
            return None; // Invalid
        }

        // Safety: We've validated the length
        let bytes = &self.data[SLOT_OFFSET + 1..SLOT_OFFSET + 1 + len];
        core::str::from_utf8(bytes).ok()
    }

    /// Read string (handles both inline and heap)
    #[inline(always)]
    pub fn read_str<const SLOT_OFFSET: usize, const HEAP_START: usize>(&self) -> Option<&str> {
        let marker = self.data[SLOT_OFFSET + 15];

        if marker == 0x00 {
            // Inline
            let len = self.data[SLOT_OFFSET] as usize;
            if len > 14 {
                return None;
            }
            let bytes = &self.data[SLOT_OFFSET + 1..SLOT_OFFSET + 1 + len];
            core::str::from_utf8(bytes).ok()
        } else if marker == 0xFF {
            // Heap
            let offset = u32::from_le_bytes([
                self.data[SLOT_OFFSET],
                self.data[SLOT_OFFSET + 1],
                self.data[SLOT_OFFSET + 2],
                self.data[SLOT_OFFSET + 3],
            ]) as usize;
            let length = u32::from_le_bytes([
                self.data[SLOT_OFFSET + 4],
                self.data[SLOT_OFFSET + 5],
                self.data[SLOT_OFFSET + 6],
                self.data[SLOT_OFFSET + 7],
            ]) as usize;

            let start = HEAP_START + offset;
            let end = start + length;
            if end > self.data.len() {
                return None;
            }

            core::str::from_utf8(&self.data[start..end]).ok()
        } else {
            None
        }
    }

    /// Read inline bytes at compile-time slot offset
    #[inline(always)]
    pub fn read_inline_bytes<const SLOT_OFFSET: usize>(&self) -> Option<&[u8]> {
        if self.data[SLOT_OFFSET + 15] != 0x00 {
            return None;
        }

        let len = self.data[SLOT_OFFSET] as usize;
        if len > 14 {
            return None;
        }

        Some(&self.data[SLOT_OFFSET + 1..SLOT_OFFSET + 1 + len])
    }

    // =========================================================================
    // UNCHECKED ACCESSORS (for maximum performance)
    // =========================================================================
    // These bypass bounds checking for sub-nanosecond field access.
    // Use only when you have validated the buffer at a higher level.

    /// Read u8 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET < data.len()`
    #[inline(always)]
    pub unsafe fn read_u8_unchecked<const OFFSET: usize>(&self) -> u8 {
        // SAFETY: Caller guarantees OFFSET is within bounds.
        // get_unchecked bypasses bounds checking for maximum performance.
        unsafe { *self.data.get_unchecked(OFFSET) }
    }

    /// Read u16 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET + 1 < data.len()`
    #[inline(always)]
    pub unsafe fn read_u16_unchecked<const OFFSET: usize>(&self) -> u16 {
        // SAFETY: Caller guarantees OFFSET + 1 is within bounds.
        // We use read_unaligned to handle potentially misaligned data.
        unsafe {
            let ptr = self.data.as_ptr().add(OFFSET) as *const u16;
            ptr.read_unaligned()
        }
    }

    /// Read u32 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET + 3 < data.len()`
    #[inline(always)]
    pub unsafe fn read_u32_unchecked<const OFFSET: usize>(&self) -> u32 {
        // SAFETY: Caller guarantees OFFSET + 3 is within bounds.
        // We use read_unaligned to handle potentially misaligned data.
        // from_le converts from little-endian byte order.
        unsafe {
            let ptr = self.data.as_ptr().add(OFFSET) as *const u32;
            u32::from_le(ptr.read_unaligned())
        }
    }

    /// Read u64 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET + 7 < data.len()`
    #[inline(always)]
    pub unsafe fn read_u64_unchecked<const OFFSET: usize>(&self) -> u64 {
        // SAFETY: Caller guarantees OFFSET + 7 is within bounds.
        // We use read_unaligned to handle potentially misaligned data.
        // from_le converts from little-endian byte order.
        unsafe {
            let ptr = self.data.as_ptr().add(OFFSET) as *const u64;
            u64::from_le(ptr.read_unaligned())
        }
    }

    /// Read i64 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET + 7 < data.len()`
    #[inline(always)]
    pub unsafe fn read_i64_unchecked<const OFFSET: usize>(&self) -> i64 {
        // SAFETY: Caller guarantees OFFSET + 7 is within bounds.
        // We delegate to read_u64_unchecked which handles the unsafe pointer operations.
        unsafe { self.read_u64_unchecked::<OFFSET>() as i64 }
    }

    /// Read f32 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET + 3 < data.len()`
    #[inline(always)]
    pub unsafe fn read_f32_unchecked<const OFFSET: usize>(&self) -> f32 {
        // SAFETY: Caller guarantees OFFSET + 3 is within bounds.
        // We delegate to read_u32_unchecked which handles the unsafe pointer operations.
        // from_bits reinterprets the u32 bit pattern as f32.
        unsafe {
            let bits = self.read_u32_unchecked::<OFFSET>();
            f32::from_bits(bits)
        }
    }

    /// Read f64 at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET + 7 < data.len()`
    #[inline(always)]
    pub unsafe fn read_f64_unchecked<const OFFSET: usize>(&self) -> f64 {
        // SAFETY: Caller guarantees OFFSET + 7 is within bounds.
        // We delegate to read_u64_unchecked which handles the unsafe pointer operations.
        // from_bits reinterprets the u64 bit pattern as f64.
        unsafe {
            let bits = self.read_u64_unchecked::<OFFSET>();
            f64::from_bits(bits)
        }
    }

    /// Read bool at compile-time offset (no bounds check)
    ///
    /// # Safety
    /// Caller must ensure `OFFSET < data.len()`
    #[inline(always)]
    pub unsafe fn read_bool_unchecked<const OFFSET: usize>(&self) -> bool {
        // SAFETY: Caller guarantees OFFSET is within bounds.
        // get_unchecked bypasses bounds checking. We interpret non-zero as true.
        unsafe { *self.data.get_unchecked(OFFSET) != 0 }
    }
}

/// Zero-copy quantum writer with compile-time offsets
#[repr(transparent)]
pub struct QuantumWriter<'a> {
    /// The underlying byte slice
    data: &'a mut [u8],
}

impl<'a> QuantumWriter<'a> {
    /// Create a new quantum writer from a mutable byte slice
    #[inline(always)]
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { data }
    }

    /// Write u8 at compile-time offset (single MOV instruction)
    #[inline(always)]
    pub fn write_u8<const OFFSET: usize>(&mut self, value: u8) {
        self.data[OFFSET] = value;
    }

    /// Write u16 at compile-time offset
    #[inline(always)]
    pub fn write_u16<const OFFSET: usize>(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.data[OFFSET] = bytes[0];
        self.data[OFFSET + 1] = bytes[1];
    }

    /// Write u32 at compile-time offset
    #[inline(always)]
    pub fn write_u32<const OFFSET: usize>(&mut self, value: u32) {
        let bytes = value.to_le_bytes();
        self.data[OFFSET] = bytes[0];
        self.data[OFFSET + 1] = bytes[1];
        self.data[OFFSET + 2] = bytes[2];
        self.data[OFFSET + 3] = bytes[3];
    }

    /// Write u64 at compile-time offset
    #[inline(always)]
    pub fn write_u64<const OFFSET: usize>(&mut self, value: u64) {
        let bytes = value.to_le_bytes();
        self.data[OFFSET] = bytes[0];
        self.data[OFFSET + 1] = bytes[1];
        self.data[OFFSET + 2] = bytes[2];
        self.data[OFFSET + 3] = bytes[3];
        self.data[OFFSET + 4] = bytes[4];
        self.data[OFFSET + 5] = bytes[5];
        self.data[OFFSET + 6] = bytes[6];
        self.data[OFFSET + 7] = bytes[7];
    }

    /// Write i8 at compile-time offset
    #[inline(always)]
    pub fn write_i8<const OFFSET: usize>(&mut self, value: i8) {
        self.write_u8::<OFFSET>(value as u8);
    }

    /// Write i16 at compile-time offset
    #[inline(always)]
    pub fn write_i16<const OFFSET: usize>(&mut self, value: i16) {
        self.write_u16::<OFFSET>(value as u16);
    }

    /// Write i32 at compile-time offset
    #[inline(always)]
    pub fn write_i32<const OFFSET: usize>(&mut self, value: i32) {
        self.write_u32::<OFFSET>(value as u32);
    }

    /// Write i64 at compile-time offset
    #[inline(always)]
    pub fn write_i64<const OFFSET: usize>(&mut self, value: i64) {
        self.write_u64::<OFFSET>(value as u64);
    }

    /// Write f32 at compile-time offset
    #[inline(always)]
    pub fn write_f32<const OFFSET: usize>(&mut self, value: f32) {
        let bytes = value.to_le_bytes();
        self.data[OFFSET] = bytes[0];
        self.data[OFFSET + 1] = bytes[1];
        self.data[OFFSET + 2] = bytes[2];
        self.data[OFFSET + 3] = bytes[3];
    }

    /// Write f64 at compile-time offset
    #[inline(always)]
    pub fn write_f64<const OFFSET: usize>(&mut self, value: f64) {
        let bytes = value.to_le_bytes();
        self.data[OFFSET] = bytes[0];
        self.data[OFFSET + 1] = bytes[1];
        self.data[OFFSET + 2] = bytes[2];
        self.data[OFFSET + 3] = bytes[3];
        self.data[OFFSET + 4] = bytes[4];
        self.data[OFFSET + 5] = bytes[5];
        self.data[OFFSET + 6] = bytes[6];
        self.data[OFFSET + 7] = bytes[7];
    }

    /// Write bool at compile-time offset
    #[inline(always)]
    pub fn write_bool<const OFFSET: usize>(&mut self, value: bool) {
        self.data[OFFSET] = value as u8;
    }

    /// Write inline string at compile-time slot offset
    ///
    /// Returns true if the string was inlined, false if too large.
    #[inline(always)]
    pub fn write_inline_str<const SLOT_OFFSET: usize>(&mut self, value: &str) -> bool {
        let bytes = value.as_bytes();
        if bytes.len() > 14 {
            return false;
        }

        self.data[SLOT_OFFSET] = bytes.len() as u8;
        self.data[SLOT_OFFSET + 1..SLOT_OFFSET + 1 + bytes.len()].copy_from_slice(bytes);
        self.data[SLOT_OFFSET + 15] = 0x00; // Inline marker

        true
    }

    /// Write inline bytes at compile-time slot offset
    #[inline(always)]
    pub fn write_inline_bytes<const SLOT_OFFSET: usize>(&mut self, value: &[u8]) -> bool {
        if value.len() > 14 {
            return false;
        }

        self.data[SLOT_OFFSET] = value.len() as u8;
        self.data[SLOT_OFFSET + 1..SLOT_OFFSET + 1 + value.len()].copy_from_slice(value);
        self.data[SLOT_OFFSET + 15] = 0x00;

        true
    }

    /// Write heap reference at compile-time slot offset
    #[inline(always)]
    pub fn write_heap_ref<const SLOT_OFFSET: usize>(&mut self, offset: u32, length: u32) {
        let off_bytes = offset.to_le_bytes();
        let len_bytes = length.to_le_bytes();

        self.data[SLOT_OFFSET] = off_bytes[0];
        self.data[SLOT_OFFSET + 1] = off_bytes[1];
        self.data[SLOT_OFFSET + 2] = off_bytes[2];
        self.data[SLOT_OFFSET + 3] = off_bytes[3];
        self.data[SLOT_OFFSET + 4] = len_bytes[0];
        self.data[SLOT_OFFSET + 5] = len_bytes[1];
        self.data[SLOT_OFFSET + 6] = len_bytes[2];
        self.data[SLOT_OFFSET + 7] = len_bytes[3];
        self.data[SLOT_OFFSET + 15] = 0xFF; // Heap marker
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_layout() {
        // User struct: id(u64=8) + age(u32=4) + active(bool=1) + score(f64=8) = 21 bytes
        // 3 slots for name, email, bio
        type UserLayout = QuantumLayout<(), 21, 3>;

        assert_eq!(UserLayout::HEADER_SIZE, 4);
        assert_eq!(UserLayout::TOTAL_FIXED, 21);
        assert_eq!(UserLayout::TOTAL_SLOTS, 48);
        assert_eq!(UserLayout::HEAP_OFFSET, 73);
        assert_eq!(UserLayout::MIN_SIZE, 73);

        // Field offsets
        assert_eq!(UserLayout::field_offset(0), 4); // id at offset 4
        assert_eq!(UserLayout::field_offset(8), 12); // age at offset 12
        assert_eq!(UserLayout::field_offset(12), 16); // active at offset 16
        assert_eq!(UserLayout::field_offset(13), 17); // score at offset 17

        // Slot offsets
        assert_eq!(UserLayout::slot_offset(0), 25); // name slot
        assert_eq!(UserLayout::slot_offset(1), 41); // email slot
        assert_eq!(UserLayout::slot_offset(2), 57); // bio slot
    }

    #[test]
    fn test_quantum_reader_primitives() {
        // Create a buffer with known values
        let mut buffer = [0u8; 32];

        // Write u64 at offset 4
        buffer[4..12].copy_from_slice(&12345u64.to_le_bytes());

        // Write u32 at offset 12
        buffer[12..16].copy_from_slice(&30u32.to_le_bytes());

        // Write bool at offset 16
        buffer[16] = 1;

        // Write f64 at offset 17
        buffer[17..25].copy_from_slice(&98.5f64.to_le_bytes());

        let reader = QuantumReader::new(&buffer);

        assert_eq!(reader.read_u64::<4>(), 12345);
        assert_eq!(reader.read_u32::<12>(), 30);
        assert!(reader.read_bool::<16>());
        assert!((reader.read_f64::<17>() - 98.5).abs() < 0.001);
    }

    #[test]
    fn test_quantum_reader_inline_string() {
        let mut buffer = [0u8; 32];

        // Write inline string "Hello" at slot offset 0
        buffer[0] = 5; // length
        buffer[1..6].copy_from_slice(b"Hello");
        buffer[15] = 0x00; // inline marker

        let reader = QuantumReader::new(&buffer);
        assert_eq!(reader.read_inline_str::<0>(), Some("Hello"));
    }

    #[test]
    fn test_quantum_writer_primitives() {
        let mut buffer = [0u8; 32];

        {
            let mut writer = QuantumWriter::new(&mut buffer);
            writer.write_u64::<4>(12345);
            writer.write_u32::<12>(30);
            writer.write_bool::<16>(true);
            writer.write_f64::<17>(98.5);
        }

        let reader = QuantumReader::new(&buffer);
        assert_eq!(reader.read_u64::<4>(), 12345);
        assert_eq!(reader.read_u32::<12>(), 30);
        assert!(reader.read_bool::<16>());
        assert!((reader.read_f64::<17>() - 98.5).abs() < 0.001);
    }

    #[test]
    fn test_quantum_writer_inline_string() {
        let mut buffer = [0u8; 32];

        {
            let mut writer = QuantumWriter::new(&mut buffer);
            assert!(writer.write_inline_str::<0>("Hello"));
        }

        let reader = QuantumReader::new(&buffer);
        assert_eq!(reader.read_inline_str::<0>(), Some("Hello"));
    }
}
