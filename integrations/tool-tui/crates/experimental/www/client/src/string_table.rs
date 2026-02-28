//! String Table Reader: Zero-copy string access
//!
//! Reads strings directly from the binary payload without allocation.

use core::ptr;
use dx_packet::StringEntry;

/// Zero-copy string table reader
pub struct StringTableReader<'a> {
    /// Raw payload bytes
    data: &'a [u8],
    /// Offset to string entry array
    entries_offset: usize,
    /// Offset to string data
    data_offset: usize,
    /// Number of strings
    count: u16,
}

impl<'a> StringTableReader<'a> {
    /// Create reader from raw bytes
    ///
    /// # Arguments
    /// * `data` - Full payload
    /// * `entries_offset` - Offset to StringEntry array
    /// * `count` - Number of strings
    pub fn new(data: &'a [u8], entries_offset: usize, count: u16) -> Self {
        // String data starts after all entries
        let data_offset = entries_offset + (count as usize * StringEntry::SIZE);

        Self {
            data,
            entries_offset,
            data_offset,
            count,
        }
    }

    /// Get string by index (zero-copy)
    pub fn get(&self, idx: u16) -> Option<&'a str> {
        if idx >= self.count {
            return None;
        }

        // Read entry (zero-copy)
        let entry_offset = self.entries_offset + (idx as usize * StringEntry::SIZE);
        if entry_offset + StringEntry::SIZE > self.data.len() {
            return None;
        }

        let entry = unsafe {
            ptr::read_unaligned(self.data.as_ptr().add(entry_offset) as *const StringEntry)
        };

        // Get string slice
        let start = self.data_offset + entry.offset as usize;
        let end = start + entry.len as usize;

        if end > self.data.len() {
            return None;
        }

        // Convert to str (UTF-8 assumed valid from server)
        core::str::from_utf8(&self.data[start..end]).ok()
    }

    /// Get string count
    pub fn count(&self) -> u16 {
        self.count
    }
}
