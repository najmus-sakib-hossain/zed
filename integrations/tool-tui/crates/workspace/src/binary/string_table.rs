//! String table for deduplication.
//!
//! Stores unique strings once and references them by index,
//! reducing file size for repeated strings like "className", "settings", etc.

use crate::{Error, Result};
use std::collections::HashMap;
use std::io::{Read, Write};

/// String table for deduplication.
#[derive(Debug, Clone, Default)]
pub struct StringTable {
    /// Unique strings in order of insertion.
    strings: Vec<String>,
    /// Map from string to index for O(1) lookup.
    index_map: HashMap<String, u32>,
}

impl StringTable {
    /// Create a new empty string table.
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    /// Add a string to the table, returning its index.
    /// If the string already exists, returns the existing index.
    pub fn add(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.index_map.get(s) {
            return idx;
        }

        let idx = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.index_map.insert(s.to_string(), idx);
        idx
    }

    /// Get a string by index.
    pub fn get(&self, index: u32) -> Option<&str> {
        self.strings.get(index as usize).map(|s| s.as_str())
    }

    /// Get the number of strings in the table.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the table is empty.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Get all strings.
    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    /// Write string table to bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write count
        bytes.extend_from_slice(&(self.strings.len() as u32).to_le_bytes());

        // Calculate offsets
        let mut offsets = Vec::with_capacity(self.strings.len());
        let mut current_offset = 4 + (self.strings.len() * 4); // count + offsets

        for s in &self.strings {
            offsets.push(current_offset as u32);
            current_offset += s.len() + 1; // +1 for null terminator
        }

        // Write offsets
        for offset in &offsets {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }

        // Write strings with null terminators
        for s in &self.strings {
            bytes.extend_from_slice(s.as_bytes());
            bytes.push(0); // null terminator
        }

        bytes
    }

    /// Read string table from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(Error::InvalidBinaryFormat {
                reason: "String table too small".into(),
            });
        }

        let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;

        if count == 0 {
            return Ok(Self::new());
        }

        let offsets_end = 4 + (count * 4);
        if bytes.len() < offsets_end {
            return Err(Error::InvalidBinaryFormat {
                reason: "String table offsets truncated".into(),
            });
        }

        // Read offsets
        let mut offsets = Vec::with_capacity(count);
        for i in 0..count {
            let start = 4 + (i * 4);
            let offset = u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap());
            offsets.push(offset as usize);
        }

        // Read strings
        let mut strings = Vec::with_capacity(count);
        let mut index_map = HashMap::with_capacity(count);

        for (i, &offset) in offsets.iter().enumerate() {
            // Find null terminator
            let end = bytes[offset..]
                .iter()
                .position(|&b| b == 0)
                .map(|pos| offset + pos)
                .unwrap_or(bytes.len());

            let s = std::str::from_utf8(&bytes[offset..end])
                .map_err(|e| Error::InvalidBinaryFormat {
                    reason: format!("Invalid UTF-8 in string table: {}", e),
                })?
                .to_string();

            index_map.insert(s.clone(), i as u32);
            strings.push(s);
        }

        Ok(Self { strings, index_map })
    }

    /// Write string table to a writer.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes).map_err(|e| Error::io("string_table", e))?;
        Ok(bytes.len())
    }

    /// Read string table from a reader.
    pub fn read_from<R: Read>(reader: &mut R, size: usize) -> Result<Self> {
        let mut bytes = vec![0u8; size];
        reader.read_exact(&mut bytes).map_err(|e| Error::io("string_table", e))?;
        Self::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut table = StringTable::new();

        let idx1 = table.add("hello");
        let idx2 = table.add("world");
        let idx3 = table.add("hello"); // duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // should return existing index

        assert_eq!(table.get(0), Some("hello"));
        assert_eq!(table.get(1), Some("world"));
        assert_eq!(table.get(2), None);
    }

    #[test]
    fn test_roundtrip() {
        let mut table = StringTable::new();
        table.add("hello");
        table.add("world");
        table.add("dx-workspace");
        table.add("configuration");

        let bytes = table.to_bytes();
        let parsed = StringTable::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed.get(0), Some("hello"));
        assert_eq!(parsed.get(1), Some("world"));
        assert_eq!(parsed.get(2), Some("dx-workspace"));
        assert_eq!(parsed.get(3), Some("configuration"));
    }

    #[test]
    fn test_empty_table() {
        let table = StringTable::new();
        let bytes = table.to_bytes();
        let parsed = StringTable::from_bytes(&bytes).unwrap();

        assert!(parsed.is_empty());
    }
}
