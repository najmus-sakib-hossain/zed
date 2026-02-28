//! DX-Machine unified slot format
//!
//! The 16-byte slot is the core innovation that enables inline optimization.
//! It can store either inline data (≤14 bytes) or a heap reference.

use std::fmt;

/// Maximum bytes that can be stored inline
pub const MAX_INLINE_SIZE: usize = 14;

/// Marker byte for inline data (byte 15 = 0x00)
pub const INLINE_MARKER: u8 = 0x00;

/// Marker byte for heap reference (byte 15 = 0xFF)
pub const HEAP_MARKER: u8 = 0xFF;

/// 16-byte slot that holds either inline data or heap reference
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DxMachineSlot {
    /// Raw slot data (16 bytes)
    ///
    /// Layout depends on marker byte (byte 15):
    ///
    /// **Inline (marker = 0x00):**
    /// - `[0]`:     length (0-14)
    /// - `[1-14]`:  inline data
    /// - `[15]`:    0x00 (INLINE\_MARKER)
    ///
    /// **Heap (marker = 0xFF):**
    /// - `[0-3]`:   heap offset (u32 LE)
    /// - `[4-7]`:   data length (u32 LE)
    /// - `[8-14]`:  reserved (zero)
    /// - `[15]`:    0xFF (HEAP\_MARKER)
    pub data: [u8; 16],
}

impl DxMachineSlot {
    /// Create empty slot (inline, zero length)
    #[inline]
    pub const fn new() -> Self {
        Self { data: [0; 16] }
    }

    /// Create inline slot from data (≤14 bytes)
    #[inline]
    pub fn inline_from_bytes(bytes: &[u8]) -> Result<Self, SlotError> {
        let len = bytes.len();
        if len > MAX_INLINE_SIZE {
            return Err(SlotError::DataTooLarge {
                size: len,
                max: MAX_INLINE_SIZE,
            });
        }

        let mut slot = Self::new();
        slot.data[0] = len as u8;
        slot.data[1..1 + len].copy_from_slice(bytes);
        slot.data[15] = INLINE_MARKER;

        Ok(slot)
    }

    /// Create heap reference slot
    #[inline]
    pub fn heap_reference(offset: u32, length: u32) -> Self {
        let mut slot = Self::new();
        slot.data[0..4].copy_from_slice(&offset.to_le_bytes());
        slot.data[4..8].copy_from_slice(&length.to_le_bytes());
        slot.data[15] = HEAP_MARKER;
        slot
    }

    /// Check if slot contains inline data
    #[inline]
    pub const fn is_inline(&self) -> bool {
        self.data[15] == INLINE_MARKER
    }

    /// Check if slot is heap reference
    #[inline]
    pub const fn is_heap(&self) -> bool {
        self.data[15] == HEAP_MARKER
    }

    /// Get inline data length (panics if not inline)
    #[inline]
    pub const fn inline_len(&self) -> usize {
        debug_assert!(self.is_inline());
        self.data[0] as usize
    }

    /// Get inline data (panics if not inline)
    #[inline]
    pub fn inline_data(&self) -> &[u8] {
        debug_assert!(self.is_inline());
        let len = self.inline_len();
        &self.data[1..1 + len]
    }

    /// Get inline data as UTF-8 string (panics if not inline or invalid UTF-8)
    ///
    /// # Panics
    ///
    /// Panics if the inline data is not valid UTF-8. Use [`Self::inline_str_checked`]
    /// for fallible conversion.
    #[inline]
    #[allow(clippy::expect_used)] // Intentional panic - documented behavior, use inline_str_checked() for fallible version
    pub fn inline_str(&self) -> &str {
        debug_assert!(self.is_inline());
        let data = self.inline_data();
        // SAFETY: This function is documented to panic on invalid UTF-8.
        // Callers should use inline_str_checked() for fallible conversion.
        // The expect message provides context for debugging if this occurs.
        std::str::from_utf8(data).expect("Invalid UTF-8 in inline data")
    }

    /// Get inline data as UTF-8 string, returning None if invalid UTF-8
    ///
    /// This is the fallible version of [`Self::inline_str`].
    #[inline]
    pub fn inline_str_checked(&self) -> Option<&str> {
        if !self.is_inline() {
            return None;
        }
        let data = self.inline_data();
        std::str::from_utf8(data).ok()
    }

    /// Get heap offset (panics if not heap)
    #[inline]
    pub fn heap_offset(&self) -> u32 {
        debug_assert!(self.is_heap());
        u32::from_le_bytes([self.data[0], self.data[1], self.data[2], self.data[3]])
    }

    /// Get heap data length (panics if not heap)
    #[inline]
    pub fn heap_length(&self) -> u32 {
        debug_assert!(self.is_heap());
        u32::from_le_bytes([self.data[4], self.data[5], self.data[6], self.data[7]])
    }

    /// Get heap reference (panics if not heap)
    #[inline]
    pub fn heap_ref(&self) -> (u32, u32) {
        debug_assert!(self.is_heap());
        (self.heap_offset(), self.heap_length())
    }

    /// Write inline data to slot
    #[inline]
    pub fn write_inline(&mut self, bytes: &[u8]) -> Result<(), SlotError> {
        let len = bytes.len();
        if len > MAX_INLINE_SIZE {
            return Err(SlotError::DataTooLarge {
                size: len,
                max: MAX_INLINE_SIZE,
            });
        }

        self.data[0] = len as u8;
        self.data[1..1 + len].copy_from_slice(bytes);
        self.data[15] = INLINE_MARKER;

        Ok(())
    }

    /// Write heap reference to slot
    #[inline]
    pub fn write_heap(&mut self, offset: u32, length: u32) {
        self.data[0..4].copy_from_slice(&offset.to_le_bytes());
        self.data[4..8].copy_from_slice(&length.to_le_bytes());
        self.data[8..15].fill(0);
        self.data[15] = HEAP_MARKER;
    }

    /// Compare inline data with byte slice (optimized)
    #[inline]
    pub fn eq_inline_bytes(&self, other: &[u8]) -> bool {
        if !self.is_inline() {
            return false;
        }

        let len = self.inline_len();
        if len != other.len() {
            return false;
        }

        self.inline_data() == other
    }

    /// Compare inline data with string (optimized)
    #[inline]
    pub fn eq_inline_str(&self, other: &str) -> bool {
        self.eq_inline_bytes(other.as_bytes())
    }

    /// Get data size (inline or heap)
    #[inline]
    pub fn size(&self) -> usize {
        if self.is_inline() {
            self.inline_len()
        } else {
            self.heap_length() as usize
        }
    }
}

impl Default for DxMachineSlot {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DxMachineSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_inline() {
            let len = self.inline_len();
            let data = self.inline_data();
            if let Ok(s) = std::str::from_utf8(data) {
                f.debug_struct("DxMachineSlot")
                    .field("type", &"inline")
                    .field("len", &len)
                    .field("data", &s)
                    .finish()
            } else {
                f.debug_struct("DxMachineSlot")
                    .field("type", &"inline")
                    .field("len", &len)
                    .field("data", &data)
                    .finish()
            }
        } else if self.is_heap() {
            f.debug_struct("DxMachineSlot")
                .field("type", &"heap")
                .field("offset", &self.heap_offset())
                .field("length", &self.heap_length())
                .finish()
        } else {
            f.debug_struct("DxMachineSlot")
                .field("type", &"invalid")
                .field("marker", &self.data[15])
                .finish()
        }
    }
}

/// Slot operation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotError {
    /// Data too large for inline storage
    DataTooLarge { size: usize, max: usize },
    /// Invalid slot marker byte
    InvalidMarker { found: u8 },
}

impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataTooLarge { size, max } => {
                write!(f, "Data size {} exceeds max inline size {}", size, max)
            }
            Self::InvalidMarker { found } => {
                write!(f, "Invalid slot marker byte: 0x{:02X}", found)
            }
        }
    }
}

impl std::error::Error for SlotError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_inline() {
        let data = b"Hello, World!"; // 13 bytes
        let slot = DxMachineSlot::inline_from_bytes(data).unwrap();

        assert!(slot.is_inline());
        assert!(!slot.is_heap());
        assert_eq!(slot.inline_len(), 13);
        assert_eq!(slot.inline_data(), data);
        assert_eq!(slot.inline_str(), "Hello, World!");
        assert_eq!(slot.size(), 13);
    }

    #[test]
    fn test_slot_heap() {
        let slot = DxMachineSlot::heap_reference(100, 500);

        assert!(slot.is_heap());
        assert!(!slot.is_inline());
        assert_eq!(slot.heap_offset(), 100);
        assert_eq!(slot.heap_length(), 500);
        assert_eq!(slot.heap_ref(), (100, 500));
        assert_eq!(slot.size(), 500);
    }

    #[test]
    fn test_slot_max_inline() {
        let data = b"12345678901234"; // 14 bytes (max)
        let slot = DxMachineSlot::inline_from_bytes(data).unwrap();

        assert!(slot.is_inline());
        assert_eq!(slot.inline_len(), 14);
        assert_eq!(slot.inline_data(), data);
    }

    #[test]
    fn test_slot_too_large() {
        let data = b"123456789012345"; // 15 bytes (too large)
        let result = DxMachineSlot::inline_from_bytes(data);

        assert!(matches!(result, Err(SlotError::DataTooLarge { size: 15, max: 14 })));
    }

    #[test]
    fn test_slot_write_inline() {
        let mut slot = DxMachineSlot::new();
        slot.write_inline(b"Test").unwrap();

        assert!(slot.is_inline());
        assert_eq!(slot.inline_str(), "Test");
    }

    #[test]
    fn test_slot_write_heap() {
        let mut slot = DxMachineSlot::new();
        slot.write_heap(42, 1000);

        assert!(slot.is_heap());
        assert_eq!(slot.heap_offset(), 42);
        assert_eq!(slot.heap_length(), 1000);
    }

    #[test]
    fn test_slot_eq() {
        let slot = DxMachineSlot::inline_from_bytes(b"test").unwrap();

        assert!(slot.eq_inline_bytes(b"test"));
        assert!(!slot.eq_inline_bytes(b"Test"));
        assert!(!slot.eq_inline_bytes(b"testing"));

        assert!(slot.eq_inline_str("test"));
        assert!(!slot.eq_inline_str("Test"));
    }

    #[test]
    fn test_slot_size() {
        assert_eq!(std::mem::size_of::<DxMachineSlot>(), 16);
    }
}
