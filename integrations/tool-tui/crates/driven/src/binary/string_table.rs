//! Interned String Table
//!
//! O(1) string access with u32 indices - no length prefixes needed.

use crate::{DrivenError, Result};
use std::collections::HashMap;

/// String ID (index into string table)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StringId(pub u32);

impl StringId {
    /// Null/empty string ID
    pub const NULL: Self = Self(0);

    /// Create from raw index
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    /// Get raw index
    pub fn index(self) -> u32 {
        self.0
    }

    /// Check if null
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

/// String table for zero-copy string access
#[derive(Debug)]
pub struct StringTable<'a> {
    /// Number of strings
    count: u32,
    /// Offset table (O(1) access)
    offsets: &'a [u32],
    /// Packed string data
    data: &'a [u8],
}

impl<'a> StringTable<'a> {
    /// Parse string table from bytes (zero-copy)
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(DrivenError::InvalidBinary("String table too small".into()));
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let total_size = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let offset_table_size = (count as usize) * 4;
        if bytes.len() < 8 + offset_table_size {
            return Err(DrivenError::InvalidBinary("String table offset table truncated".into()));
        }

        // Safety: We've verified the size
        let offsets_bytes = &bytes[8..8 + offset_table_size];
        let offsets: &[u32] = bytemuck::cast_slice(offsets_bytes);

        let data_start = 8 + offset_table_size;
        let data_end = data_start + total_size as usize;
        if bytes.len() < data_end {
            return Err(DrivenError::InvalidBinary("String table data truncated".into()));
        }

        let data = &bytes[data_start..data_end];

        Ok(Self {
            count,
            offsets,
            data,
        })
    }

    /// Get string by ID (O(1))
    pub fn get(&self, id: StringId) -> Option<&'a str> {
        if id.is_null() {
            return Some("");
        }

        let idx = id.0 as usize;
        if idx == 0 || idx > self.count as usize {
            return None;
        }

        // Adjust for 1-based indexing
        let idx = idx - 1;

        let start = self.offsets[idx] as usize;
        let end = if idx + 1 < self.count as usize {
            self.offsets[idx + 1] as usize
        } else {
            self.data.len()
        };

        if end > self.data.len() || start > end {
            return None;
        }

        std::str::from_utf8(&self.data[start..end]).ok()
    }

    /// Number of strings
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Iterate over all strings
    pub fn iter(&self) -> impl Iterator<Item = (StringId, &'a str)> + '_ {
        (1..=self.count).filter_map(move |i| {
            let id = StringId(i);
            self.get(id).map(|s| (id, s))
        })
    }
}

/// Builder for creating string tables
#[derive(Debug, Default)]
pub struct StringTableBuilder {
    /// Strings to intern
    strings: Vec<String>,
    /// Deduplication map
    index: HashMap<String, StringId>,
}

impl StringTableBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern a string, returning its ID
    pub fn intern(&mut self, s: &str) -> StringId {
        if s.is_empty() {
            return StringId::NULL;
        }

        if let Some(&id) = self.index.get(s) {
            return id;
        }

        let id = StringId((self.strings.len() + 1) as u32);
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), id);
        id
    }

    /// Get ID for existing string (if interned)
    pub fn get(&self, s: &str) -> Option<StringId> {
        if s.is_empty() {
            return Some(StringId::NULL);
        }
        self.index.get(s).copied()
    }

    /// Number of interned strings
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Build the binary string table
    pub fn build(&self) -> Vec<u8> {
        let count = self.strings.len() as u32;

        // Calculate total string data size
        let total_size: usize = self.strings.iter().map(|s| s.len()).sum();

        // Build offset table
        let mut offsets = Vec::with_capacity(self.strings.len());
        let mut offset = 0u32;
        for s in &self.strings {
            offsets.push(offset);
            offset += s.len() as u32;
        }

        // Build output
        let mut output = Vec::with_capacity(8 + offsets.len() * 4 + total_size);

        // Header
        output.extend_from_slice(&count.to_le_bytes());
        output.extend_from_slice(&(total_size as u32).to_le_bytes());

        // Offset table
        for off in &offsets {
            output.extend_from_slice(&off.to_le_bytes());
        }

        // String data
        for s in &self.strings {
            output.extend_from_slice(s.as_bytes());
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_id_null() {
        assert!(StringId::NULL.is_null());
        assert!(!StringId(1).is_null());
    }

    #[test]
    fn test_builder_intern() {
        let mut builder = StringTableBuilder::new();

        let id1 = builder.intern("hello");
        let id2 = builder.intern("world");
        let id3 = builder.intern("hello"); // Duplicate

        assert_eq!(id1, id3); // Same ID for duplicate
        assert_ne!(id1, id2);
        assert_eq!(builder.len(), 2);
    }

    #[test]
    fn test_roundtrip() {
        let mut builder = StringTableBuilder::new();
        let id1 = builder.intern("hello");
        let id2 = builder.intern("world");
        let id3 = builder.intern("test");

        let bytes = builder.build();
        let table = StringTable::from_bytes(&bytes).unwrap();

        assert_eq!(table.len(), 3);
        assert_eq!(table.get(id1), Some("hello"));
        assert_eq!(table.get(id2), Some("world"));
        assert_eq!(table.get(id3), Some("test"));
        assert_eq!(table.get(StringId::NULL), Some(""));
    }

    #[test]
    fn test_empty_string() {
        let builder = StringTableBuilder::new();
        let id = builder.get("").unwrap();
        assert!(id.is_null());
    }
}
