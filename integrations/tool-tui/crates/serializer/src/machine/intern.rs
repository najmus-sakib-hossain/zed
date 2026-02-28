//! String interning for DX-Machine
//!
//! Provides zero-copy string deduplication to reduce serialized size by 50-90%
//! for data with repeated strings (logs, configs, etc.).
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │ Serialized Format                                       │
//! ├─────────────────────────────────────────────────────────┤
//! │ Header (4 bytes) - FLAG_HAS_INTERN set                  │
//! ├─────────────────────────────────────────────────────────┤
//! │ String Pool:                                            │
//! │   - Pool size: u32 (4 bytes)                            │
//! │   - String count: u32 (4 bytes)                         │
//! │   - Offsets: [u32; count] (4 * count bytes)             │
//! │   - Strings: concatenated UTF-8 data                    │
//! ├─────────────────────────────────────────────────────────┤
//! │ RKYV Data (strings replaced with pool indices)          │
//! └─────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;

/// String interning pool for deduplication
pub struct InternPool {
    /// Map from string content to pool index
    map: HashMap<String, u32>,
    /// Ordered list of unique strings
    strings: Vec<String>,
}

impl InternPool {
    /// Create a new empty intern pool
    #[inline]
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            strings: Vec::new(),
        }
    }

    /// Create a pool with pre-allocated capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            strings: Vec::with_capacity(capacity),
        }
    }

    /// Intern a string, returning its pool index
    ///
    /// If the string already exists, returns the existing index.
    /// Otherwise, adds it to the pool and returns the new index.
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.map.get(s) {
            return idx;
        }

        let idx = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.map.insert(s.to_string(), idx);
        idx
    }

    /// Get a string by its pool index
    #[inline]
    pub fn get(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(|s| s.as_str())
    }

    /// Get the number of unique strings in the pool
    #[inline]
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Check if the pool is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Serialize the pool to bytes
    ///
    /// Format:
    /// - Pool size: u32 (total bytes for pool section)
    /// - String count: u32
    /// - Offsets: [u32; count] (byte offset of each string in data section)
    /// - Data: concatenated UTF-8 strings
    pub fn serialize(&self) -> Vec<u8> {
        if self.strings.is_empty() {
            // Empty pool: just write zeros
            return vec![0, 0, 0, 0, 0, 0, 0, 0];
        }

        let count = self.strings.len() as u32;
        let mut offsets = Vec::with_capacity(self.strings.len());
        let mut data = Vec::new();

        // Build offsets and concatenate strings
        for s in &self.strings {
            offsets.push(data.len() as u32);
            data.extend_from_slice(s.as_bytes());
        }

        // Calculate total pool size
        let pool_size = 8 + (offsets.len() * 4) + data.len();

        let mut bytes = Vec::with_capacity(pool_size);

        // Write pool size
        bytes.extend_from_slice(&(pool_size as u32).to_le_bytes());

        // Write string count
        bytes.extend_from_slice(&count.to_le_bytes());

        // Write offsets
        for offset in offsets {
            bytes.extend_from_slice(&offset.to_le_bytes());
        }

        // Write string data
        bytes.extend_from_slice(&data);

        bytes
    }

    /// Deserialize a pool from bytes
    ///
    /// Returns the pool and the number of bytes consumed.
    pub fn deserialize(bytes: &[u8]) -> Result<(Self, usize), InternError> {
        if bytes.len() < 8 {
            return Err(InternError::BufferTooSmall);
        }

        // Read pool size
        let pool_size = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;

        if pool_size == 0 {
            // Empty pool
            return Ok((Self::new(), 8));
        }

        if bytes.len() < pool_size {
            return Err(InternError::BufferTooSmall);
        }

        // Read string count
        let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;

        if count == 0 {
            return Ok((Self::new(), pool_size));
        }

        // Read offsets
        let offsets_start = 8;
        let offsets_end = offsets_start + (count * 4);

        if bytes.len() < offsets_end {
            return Err(InternError::BufferTooSmall);
        }

        let mut offsets = Vec::with_capacity(count);
        for i in 0..count {
            let offset_pos = offsets_start + (i * 4);
            let offset = u32::from_le_bytes([
                bytes[offset_pos],
                bytes[offset_pos + 1],
                bytes[offset_pos + 2],
                bytes[offset_pos + 3],
            ]) as usize;
            offsets.push(offset);
        }

        // Read string data
        let data_start = offsets_end;
        let data = &bytes[data_start..pool_size];

        // Reconstruct strings
        let mut strings = Vec::with_capacity(count);
        let mut map = HashMap::with_capacity(count);

        for (idx, &offset) in offsets.iter().enumerate() {
            let end = if idx + 1 < offsets.len() {
                offsets[idx + 1]
            } else {
                data.len()
            };

            if offset > data.len() || end > data.len() || offset > end {
                return Err(InternError::InvalidOffset);
            }

            let s = std::str::from_utf8(&data[offset..end])
                .map_err(|_| InternError::InvalidUtf8)?
                .to_string();

            map.insert(s.clone(), idx as u32);
            strings.push(s);
        }

        Ok((Self { map, strings }, pool_size))
    }
}

impl Default for InternPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during string interning
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternError {
    /// Buffer too small to contain pool data
    BufferTooSmall,
    /// Invalid string offset in pool
    InvalidOffset,
    /// Invalid UTF-8 in string data
    InvalidUtf8,
}

impl std::fmt::Display for InternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small to contain intern pool"),
            Self::InvalidOffset => write!(f, "Invalid string offset in intern pool"),
            Self::InvalidUtf8 => write!(f, "Invalid UTF-8 in intern pool string data"),
        }
    }
}

impl std::error::Error for InternError {}

/// Serializer with string interning support
pub struct InterningSerializer {
    pool: InternPool,
}

impl InterningSerializer {
    /// Create a new interning serializer
    #[inline]
    pub fn new() -> Self {
        Self {
            pool: InternPool::new(),
        }
    }

    /// Create with pre-allocated capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pool: InternPool::with_capacity(capacity),
        }
    }

    /// Get a reference to the intern pool
    #[inline]
    pub fn pool(&self) -> &InternPool {
        &self.pool
    }

    /// Get a mutable reference to the intern pool
    #[inline]
    pub fn pool_mut(&mut self) -> &mut InternPool {
        &mut self.pool
    }

    /// Intern a string and return its pool index
    #[inline]
    pub fn intern(&mut self, s: &str) -> u32 {
        self.pool.intern(s)
    }

    /// Serialize the intern pool
    #[inline]
    pub fn serialize_pool(&self) -> Vec<u8> {
        self.pool.serialize()
    }
}

impl Default for InterningSerializer {
    fn default() -> Self {
        Self::new()
    }
}

/// Deserializer with string interning support
pub struct InterningDeserializer {
    pool: InternPool,
}

impl InterningDeserializer {
    /// Create a new interning deserializer from serialized bytes
    pub fn new(bytes: &[u8]) -> Result<(Self, usize), InternError> {
        let (pool, consumed) = InternPool::deserialize(bytes)?;
        Ok((Self { pool }, consumed))
    }

    /// Get a string by its pool index
    #[inline]
    pub fn get(&self, idx: u32) -> Option<&str> {
        self.pool.get(idx)
    }

    /// Get a reference to the intern pool
    #[inline]
    pub fn pool(&self) -> &InternPool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_pool_basic() {
        let mut pool = InternPool::new();

        let idx1 = pool.intern("hello");
        let idx2 = pool.intern("world");
        let idx3 = pool.intern("hello"); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first "hello"

        assert_eq!(pool.get(0), Some("hello"));
        assert_eq!(pool.get(1), Some("world"));
        assert_eq!(pool.get(2), None);
    }

    #[test]
    fn test_intern_pool_roundtrip() {
        let mut pool = InternPool::new();
        pool.intern("foo");
        pool.intern("bar");
        pool.intern("baz");
        pool.intern("foo"); // Duplicate

        let bytes = pool.serialize();
        let (deserialized, consumed) = InternPool::deserialize(&bytes).unwrap();

        assert_eq!(consumed, bytes.len());
        assert_eq!(deserialized.len(), 3); // Only unique strings
        assert_eq!(deserialized.get(0), Some("foo"));
        assert_eq!(deserialized.get(1), Some("bar"));
        assert_eq!(deserialized.get(2), Some("baz"));
    }

    #[test]
    fn test_empty_pool() {
        let pool = InternPool::new();
        let bytes = pool.serialize();

        let (deserialized, consumed) = InternPool::deserialize(&bytes).unwrap();
        assert_eq!(consumed, 8);
        assert!(deserialized.is_empty());
    }

    #[test]
    fn test_interning_serializer() {
        let mut serializer = InterningSerializer::new();

        let idx1 = serializer.intern("test");
        let idx2 = serializer.intern("data");
        let idx3 = serializer.intern("test");

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0);

        let bytes = serializer.serialize_pool();
        let (deserializer, _) = InterningDeserializer::new(&bytes).unwrap();

        assert_eq!(deserializer.get(0), Some("test"));
        assert_eq!(deserializer.get(1), Some("data"));
    }

    #[test]
    fn test_unicode_strings() {
        let mut pool = InternPool::new();
        pool.intern("Hello 世界");
        pool.intern("🦀 Rust");
        pool.intern("Привет");

        let bytes = pool.serialize();
        let (deserialized, _) = InternPool::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.get(0), Some("Hello 世界"));
        assert_eq!(deserialized.get(1), Some("🦀 Rust"));
        assert_eq!(deserialized.get(2), Some("Привет"));
    }

    #[test]
    fn test_large_pool() {
        let mut pool = InternPool::new();

        // Add 1000 unique strings
        for i in 0..1000 {
            pool.intern(&format!("string_{}", i));
        }

        // Add duplicates
        for i in 0..500 {
            pool.intern(&format!("string_{}", i));
        }

        assert_eq!(pool.len(), 1000); // Only unique strings

        let bytes = pool.serialize();
        let (deserialized, _) = InternPool::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.len(), 1000);
        for i in 0..1000 {
            assert_eq!(deserialized.get(i), Some(format!("string_{}", i).as_str()));
        }
    }
}
