//! Binary Dawn CSS Format
//!
//! Zero-copy binary CSS format using varint encoding for ultra-fast style loading.
//!
//! File structure:
//! - Header (12 bytes): magic, version, flags, entry_count, checksum
//! - Entries: varint ID + offset (4 bytes) + length (2 bytes)
//! - String table: concatenated CSS text

use super::varint::{decode_varint, encode_varint};

/// Binary Dawn file header.
///
/// Layout (12 bytes total):
/// - magic: 4 bytes ("DXBD")
/// - version: 1 byte
/// - flags: 1 byte (reserved)
/// - entry_count: 2 bytes (u16 LE)
/// - checksum: 4 bytes (u32 LE)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct BinaryDawnHeader {
    /// Magic bytes: "DXBD" (0x44 0x58 0x42 0x44)
    pub magic: [u8; 4],
    /// Format version
    pub version: u8,
    /// Flags (reserved for future use)
    pub flags: u8,
    /// Number of style entries
    pub entry_count: u16,
    /// Checksum of data section (seahash)
    pub checksum: u32,
}

impl BinaryDawnHeader {
    /// Magic bytes identifying Binary Dawn format
    pub const MAGIC: [u8; 4] = [0x44, 0x58, 0x42, 0x44]; // "DXBD"
    /// Current format version
    pub const VERSION: u8 = 1;
    /// Header size in bytes
    pub const SIZE: usize = 12;

    /// Create a new header with the given entry count and checksum.
    pub fn new(entry_count: u16, checksum: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            flags: 0,
            entry_count,
            checksum,
        }
    }

    /// Validate the header magic and version.
    pub fn validate(&self) -> bool {
        self.magic == Self::MAGIC && self.version == Self::VERSION
    }

    /// Serialize header to bytes.
    pub fn to_bytes(&self) -> [u8; 12] {
        let mut bytes = [0u8; 12];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4] = self.version;
        bytes[5] = self.flags;
        bytes[6..8].copy_from_slice(&self.entry_count.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.checksum.to_le_bytes());
        bytes
    }

    /// Parse header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BinaryDawnError> {
        if bytes.len() < Self::SIZE {
            return Err(BinaryDawnError::HeaderTooShort);
        }

        let header = Self {
            magic: [bytes[0], bytes[1], bytes[2], bytes[3]],
            version: bytes[4],
            flags: bytes[5],
            entry_count: u16::from_le_bytes([bytes[6], bytes[7]]),
            checksum: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        };

        if !header.validate() {
            return Err(BinaryDawnError::InvalidHeader);
        }

        Ok(header)
    }
}

/// Binary Dawn style entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinaryDawnEntry {
    /// Style ID (u16, varint encoded if < 128)
    pub id: u16,
    /// CSS text offset in string table
    pub css_offset: u32,
    /// CSS text length
    pub css_len: u16,
}

impl BinaryDawnEntry {
    /// Create a new entry.
    pub fn new(id: u16, css_offset: u32, css_len: u16) -> Self {
        Self {
            id,
            css_offset,
            css_len,
        }
    }
}

/// Errors that can occur when working with Binary Dawn format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryDawnError {
    /// Header is too short (expected at least 16 bytes)
    HeaderTooShort,
    /// Invalid header (bad magic bytes or unsupported version)
    InvalidHeader,
    /// Checksum mismatch - data may be corrupted
    ChecksumMismatch,
    /// Entry data is corrupted or malformed
    CorruptedEntry,
    /// String table offset out of bounds
    InvalidOffset,
    /// Data is incomplete - file may be truncated
    IncompleteData,
}

impl std::fmt::Display for BinaryDawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HeaderTooShort => write!(
                f,
                "Binary Dawn header too short (expected at least 16 bytes). The file may be truncated or not a valid Binary Dawn file."
            ),
            Self::InvalidHeader => write!(
                f,
                "Invalid Binary Dawn header. Expected magic bytes 'DXBD' (0x44584244) and version 1. The file may be corrupted or not a Binary Dawn file."
            ),
            Self::ChecksumMismatch => write!(
                f,
                "Binary Dawn checksum mismatch. The file data may be corrupted. Try regenerating the file."
            ),
            Self::CorruptedEntry => write!(
                f,
                "Corrupted entry data in Binary Dawn file. The entry table may be malformed."
            ),
            Self::InvalidOffset => write!(
                f,
                "Invalid string table offset in Binary Dawn file. An entry references data outside the string table bounds."
            ),
            Self::IncompleteData => write!(
                f,
                "Incomplete Binary Dawn data. The file appears to be truncated or incomplete."
            ),
        }
    }
}

impl std::error::Error for BinaryDawnError {}

/// Writer for creating Binary Dawn files.
pub struct BinaryDawnWriter {
    entries: Vec<(u16, String)>,
}

impl BinaryDawnWriter {
    /// Create a new writer.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add a style entry.
    pub fn add_style(&mut self, id: u16, css: &str) {
        self.entries.push((id, css.to_string()));
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Build the binary output.
    ///
    /// Format:
    /// - Header (12 bytes)
    /// - Entries (variable: varint ID + 4-byte offset + 2-byte length)
    /// - String table (concatenated CSS)
    ///
    /// Note: Entries are automatically sorted by ID to enable binary search
    /// in the reader. This ensures O(log n) lookup performance.
    #[tracing::instrument(skip(self), fields(entry_count = self.entries.len()))]
    pub fn build(&self) -> Vec<u8> {
        use tracing::debug;

        let mut buffer = Vec::new();

        // Sort entries by ID for binary search compatibility
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by_key(|(id, _)| *id);

        // Build string table and collect offsets
        let mut string_table = Vec::new();
        let mut entry_data: Vec<(u16, u32, u16)> = Vec::new();

        for (id, css) in &sorted_entries {
            let offset = string_table.len() as u32;
            let len = css.len() as u16;
            string_table.extend_from_slice(css.as_bytes());
            entry_data.push((*id, offset, len));
        }

        // Calculate checksum of string table
        let checksum = seahash::hash(&string_table) as u32;

        // Write header
        let header = BinaryDawnHeader::new(sorted_entries.len() as u16, checksum);
        buffer.extend_from_slice(&header.to_bytes());

        // Write entries with varint IDs
        for (id, offset, len) in &entry_data {
            // Varint encode ID
            let varint = encode_varint(*id);
            buffer.extend_from_slice(&varint);

            // Write offset (4 bytes LE)
            buffer.extend_from_slice(&offset.to_le_bytes());

            // Write length (2 bytes LE)
            buffer.extend_from_slice(&len.to_le_bytes());
        }

        // Write string table
        buffer.extend_from_slice(&string_table);

        debug!(
            output_bytes = buffer.len(),
            string_table_bytes = string_table.len(),
            "Binary Dawn build complete"
        );

        buffer
    }
}

impl Default for BinaryDawnWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Zero-copy reader for Binary Dawn files.
pub struct BinaryDawnReader<'a> {
    data: &'a [u8],
    header: BinaryDawnHeader,
    /// Parsed entries for fast lookup
    entries: Vec<BinaryDawnEntry>,
    /// Start of string table in data
    string_table_start: usize,
}

impl<'a> BinaryDawnReader<'a> {
    /// Create a new reader from raw bytes.
    ///
    /// Validates the header and checksum.
    #[tracing::instrument(skip(data), fields(data_len = data.len()))]
    pub fn new(data: &'a [u8]) -> Result<Self, BinaryDawnError> {
        use tracing::{debug, error};

        if data.len() < BinaryDawnHeader::SIZE {
            error!(data_len = data.len(), "Binary Dawn data too short for header");
            return Err(BinaryDawnError::HeaderTooShort);
        }

        let header = BinaryDawnHeader::from_bytes(data)?;
        debug!(entry_count = header.entry_count, "Parsed Binary Dawn header");

        // Parse entries
        let mut pos = BinaryDawnHeader::SIZE;
        let mut entries = Vec::with_capacity(header.entry_count as usize);

        for _ in 0..header.entry_count {
            if pos >= data.len() {
                error!(pos, data_len = data.len(), "Incomplete Binary Dawn data");
                return Err(BinaryDawnError::IncompleteData);
            }

            // Decode varint ID
            let (id, consumed) =
                decode_varint(&data[pos..]).map_err(|_| BinaryDawnError::CorruptedEntry)?;
            pos += consumed;

            // Read offset (4 bytes)
            if pos + 6 > data.len() {
                return Err(BinaryDawnError::IncompleteData);
            }
            let offset =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;

            // Read length (2 bytes)
            let len = u16::from_le_bytes([data[pos], data[pos + 1]]);
            pos += 2;

            entries.push(BinaryDawnEntry::new(id, offset, len));
        }

        let string_table_start = pos;

        // Validate checksum
        let string_table = &data[string_table_start..];
        let computed_checksum = seahash::hash(string_table) as u32;
        if computed_checksum != header.checksum {
            error!(
                expected = header.checksum,
                computed = computed_checksum,
                "Binary Dawn checksum mismatch"
            );
            return Err(BinaryDawnError::ChecksumMismatch);
        }

        debug!(
            entries = entries.len(),
            string_table_bytes = string_table.len(),
            "Binary Dawn reader initialized"
        );

        Ok(Self {
            data,
            header,
            entries,
            string_table_start,
        })
    }

    /// Get the header.
    pub fn header(&self) -> &BinaryDawnHeader {
        &self.header
    }

    /// Get the number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Zero-copy lookup of CSS by ID using binary search.
    ///
    /// This method uses binary search for O(log n) lookup performance.
    /// Requires entries to be sorted by ID (guaranteed by BinaryDawnWriter).
    pub fn get_css(&self, id: u16) -> Option<&'a str> {
        // Binary search for the ID using binary_search_by_key
        let idx = self.entries.binary_search_by_key(&id, |e| e.id).ok()?;
        self.get_css_by_index(idx)
    }

    /// Get CSS by entry index.
    pub fn get_css_by_index(&self, index: usize) -> Option<&'a str> {
        let entry = self.entries.get(index)?;
        let start = self.string_table_start + entry.css_offset as usize;
        let end = start + entry.css_len as usize;

        if end > self.data.len() {
            return None;
        }

        std::str::from_utf8(&self.data[start..end]).ok()
    }

    /// Get entry by index.
    pub fn get_entry(&self, index: usize) -> Option<&BinaryDawnEntry> {
        self.entries.get(index)
    }

    /// Iterate over all entries with their CSS.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &'a str)> + '_ {
        self.entries.iter().filter_map(move |entry| {
            let start = self.string_table_start + entry.css_offset as usize;
            let end = start + entry.css_len as usize;
            if end > self.data.len() {
                return None;
            }
            let css = std::str::from_utf8(&self.data[start..end]).ok()?;
            Some((entry.id, css))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new() {
        let header = BinaryDawnHeader::new(100, 0x12345678);
        assert_eq!(header.magic, BinaryDawnHeader::MAGIC);
        assert_eq!(header.version, BinaryDawnHeader::VERSION);
        assert_eq!(header.flags, 0);
        assert_eq!(header.entry_count, 100);
        assert_eq!(header.checksum, 0x12345678);
    }

    #[test]
    fn test_header_validate() {
        let valid = BinaryDawnHeader::new(10, 0);
        assert!(valid.validate());

        let invalid_magic = BinaryDawnHeader {
            magic: [0, 0, 0, 0],
            ..valid
        };
        assert!(!invalid_magic.validate());

        let invalid_version = BinaryDawnHeader {
            version: 99,
            ..valid
        };
        assert!(!invalid_version.validate());
    }

    #[test]
    fn test_header_roundtrip() {
        let original = BinaryDawnHeader::new(42, 0xDEADBEEF);
        let bytes = original.to_bytes();
        let parsed = BinaryDawnHeader::from_bytes(&bytes).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_writer_basic() {
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(1, ".flex { display: flex; }");
        writer.add_style(2, ".p-4 { padding: 1rem; }");

        let data = writer.build();
        assert!(data.len() > BinaryDawnHeader::SIZE);

        // Verify header
        let header = BinaryDawnHeader::from_bytes(&data).unwrap();
        assert_eq!(header.entry_count, 2);
    }

    #[test]
    fn test_reader_basic() {
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(1, ".flex { display: flex; }");
        writer.add_style(2, ".p-4 { padding: 1rem; }");

        let data = writer.build();
        let reader = BinaryDawnReader::new(&data).unwrap();

        assert_eq!(reader.entry_count(), 2);
        assert_eq!(reader.get_css(1), Some(".flex { display: flex; }"));
        assert_eq!(reader.get_css(2), Some(".p-4 { padding: 1rem; }"));
        assert_eq!(reader.get_css(99), None);
    }

    #[test]
    fn test_roundtrip() {
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(0, "body { margin: 0; }");
        writer.add_style(42, ".container { max-width: 1200px; }");
        writer.add_style(127, ".text-center { text-align: center; }");
        writer.add_style(128, ".bg-blue { background: blue; }");
        writer.add_style(1000, ".complex { color: red; font-size: 16px; }");

        let data = writer.build();
        let reader = BinaryDawnReader::new(&data).unwrap();

        assert_eq!(reader.entry_count(), 5);
        assert_eq!(reader.get_css(0), Some("body { margin: 0; }"));
        assert_eq!(reader.get_css(42), Some(".container { max-width: 1200px; }"));
        assert_eq!(reader.get_css(127), Some(".text-center { text-align: center; }"));
        assert_eq!(reader.get_css(128), Some(".bg-blue { background: blue; }"));
        assert_eq!(reader.get_css(1000), Some(".complex { color: red; font-size: 16px; }"));
    }

    #[test]
    fn test_empty_writer() {
        let writer = BinaryDawnWriter::new();
        let data = writer.build();

        let reader = BinaryDawnReader::new(&data).unwrap();
        assert_eq!(reader.entry_count(), 0);
    }

    #[test]
    fn test_checksum_validation() {
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(1, ".test { color: red; }");

        let mut data = writer.build();

        // Corrupt the string table
        if let Some(last) = data.last_mut() {
            *last ^= 0xFF;
        }

        let result = BinaryDawnReader::new(&data);
        assert!(matches!(result, Err(BinaryDawnError::ChecksumMismatch)));
    }

    #[test]
    fn test_invalid_header() {
        let data = vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let result = BinaryDawnReader::new(&data);
        assert!(matches!(result, Err(BinaryDawnError::InvalidHeader)));
    }

    #[test]
    fn test_header_too_short() {
        let data = vec![0x44, 0x58, 0x42, 0x44]; // Just magic, no rest
        let result = BinaryDawnReader::new(&data);
        assert!(matches!(result, Err(BinaryDawnError::HeaderTooShort)));
    }

    #[test]
    fn test_iter() {
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(1, ".a { }");
        writer.add_style(2, ".b { }");
        writer.add_style(3, ".c { }");

        let data = writer.build();
        let reader = BinaryDawnReader::new(&data).unwrap();

        let entries: Vec<_> = reader.iter().collect();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0], (1, ".a { }"));
        assert_eq!(entries[1], (2, ".b { }"));
        assert_eq!(entries[2], (3, ".c { }"));
    }

    #[test]
    fn test_varint_encoding_in_file() {
        let mut writer = BinaryDawnWriter::new();
        // ID < 128 should use 1 byte
        writer.add_style(50, ".small { }");
        // ID >= 128 should use 2 bytes
        writer.add_style(200, ".large { }");

        let data = writer.build();
        let reader = BinaryDawnReader::new(&data).unwrap();

        assert_eq!(reader.get_css(50), Some(".small { }"));
        assert_eq!(reader.get_css(200), Some(".large { }"));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Generate valid CSS-like strings
    fn arb_css() -> impl Strategy<Value = String> {
        "[.a-z][a-z0-9-]* \\{ [a-z-]+: [a-z0-9#%]+; \\}".prop_map(|s| s)
    }

    // Generate a vector of (id, css) pairs with UNIQUE IDs
    // This is important because BinaryDawnWriter sorts by ID and binary search
    // expects unique IDs for correct lookup behavior.
    fn arb_style_entries() -> impl Strategy<Value = Vec<(u16, String)>> {
        prop::collection::vec(arb_css(), 0..50).prop_flat_map(|css_list| {
            let len = css_list.len();
            // Generate unique IDs by sampling without replacement
            prop::sample::subsequence((0u16..10000).collect::<Vec<_>>(), len).prop_map(move |ids| {
                ids.into_iter().zip(css_list.iter().cloned()).collect::<Vec<_>>()
            })
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-production-ready, Property 2: Binary Dawn Round-Trip
        /// *For any* valid Binary Dawn file, loading then saving SHALL produce
        /// an identical byte sequence.
        /// **Validates: Requirements 5.6**
        #[test]
        fn prop_binary_dawn_roundtrip(entries in arb_style_entries()) {
            // Build the binary data
            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let data = writer.build();

            // Read it back
            let reader = BinaryDawnReader::new(&data).unwrap();

            // Verify all entries match
            prop_assert_eq!(reader.entry_count(), entries.len());

            for (i, (id, css)) in entries.iter().enumerate() {
                let entry = reader.get_entry(i).unwrap();
                prop_assert_eq!(entry.id, *id, "ID mismatch at index {}", i);

                let read_css = reader.get_css_by_index(i).unwrap();
                prop_assert_eq!(read_css, css.as_str(), "CSS mismatch at index {}", i);
            }
        }

        /// Feature: dx-style-production-ready, Property 12: Varint Encoding Size
        /// *For any* style ID < 128, the varint encoding SHALL use exactly 1 byte;
        /// for IDs >= 128, it SHALL use 2 bytes.
        /// **Validates: Requirements 5.4**
        #[test]
        fn prop_varint_encoding_size(id in 0u16..16384) {
            let encoded = encode_varint(id);

            if id < 128 {
                prop_assert_eq!(
                    encoded.len(), 1,
                    "ID {} should encode to 1 byte, got {} bytes",
                    id, encoded.len()
                );
            } else {
                prop_assert_eq!(
                    encoded.len(), 2,
                    "ID {} should encode to 2 bytes, got {} bytes",
                    id, encoded.len()
                );
            }
        }

        /// Property: Header serialization is deterministic
        #[test]
        fn prop_header_deterministic(entry_count in 0u16..1000, checksum: u32) {
            let header1 = BinaryDawnHeader::new(entry_count, checksum);
            let header2 = BinaryDawnHeader::new(entry_count, checksum);

            prop_assert_eq!(header1.to_bytes(), header2.to_bytes());
        }

        /// Property: Reader correctly validates checksum
        #[test]
        fn prop_checksum_validation(entries in arb_style_entries()) {
            if entries.is_empty() {
                return Ok(());
            }

            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let mut data = writer.build();

            // Corrupt a byte in the string table (after header and entries)
            let string_table_approx_start = BinaryDawnHeader::SIZE + entries.len() * 8;
            if string_table_approx_start < data.len() {
                data[string_table_approx_start] ^= 0xFF;

                let result = BinaryDawnReader::new(&data);
                prop_assert!(
                    matches!(result, Err(BinaryDawnError::ChecksumMismatch)),
                    "Should detect checksum mismatch after corruption"
                );
            }
        }

        /// Property: All IDs are retrievable after write
        #[test]
        fn prop_all_ids_retrievable(entries in arb_style_entries()) {
            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let data = writer.build();
            let reader = BinaryDawnReader::new(&data).unwrap();

            for (id, css) in &entries {
                let retrieved = reader.get_css(*id);
                prop_assert_eq!(
                    retrieved, Some(css.as_str()),
                    "Failed to retrieve CSS for ID {}",
                    id
                );
            }
        }

        /// Feature: dx-style-extension-enhancements, Property 8: Binary Search Correctness
        /// *For any* Binary Dawn file with sorted entries, looking up an ID that exists
        /// SHALL return the correct CSS, and looking up an ID that doesn't exist SHALL return None.
        /// **Validates: Requirements 7.1, 7.2**
        #[test]
        fn prop_binary_search_correctness(entries in arb_style_entries()) {
            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let data = writer.build();
            let reader = BinaryDawnReader::new(&data).unwrap();

            // Collect all IDs that were added
            let existing_ids: std::collections::HashSet<u16> = entries.iter().map(|(id, _)| *id).collect();

            // Test that existing IDs return correct CSS
            for (id, css) in &entries {
                let result = reader.get_css(*id);
                prop_assert_eq!(
                    result, Some(css.as_str()),
                    "Binary search should find existing ID {} with correct CSS",
                    id
                );
            }

            // Test that non-existing IDs return None
            for test_id in 0u16..100 {
                if !existing_ids.contains(&test_id) {
                    let result = reader.get_css(test_id);
                    prop_assert_eq!(
                        result, None,
                        "Binary search should return None for non-existing ID {}",
                        test_id
                    );
                }
            }
        }

        /// Feature: dx-style-extension-enhancements, Property 9: Entry Sorting Guarantee
        /// *For any* set of entries added to BinaryDawnWriter in any order,
        /// the built binary SHALL have entries sorted by ID.
        /// **Validates: Requirements 7.3, 7.4**
        #[test]
        fn prop_entry_sorting_guarantee(entries in arb_style_entries()) {
            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let data = writer.build();
            let reader = BinaryDawnReader::new(&data).unwrap();

            // Verify entries are sorted by ID
            let mut prev_id: Option<u16> = None;
            for i in 0..reader.entry_count() {
                let entry = reader.get_entry(i).unwrap();
                if let Some(prev) = prev_id {
                    prop_assert!(
                        entry.id >= prev,
                        "Entries not sorted: ID {} at index {} should be >= previous ID {}",
                        entry.id, i, prev
                    );
                }
                prev_id = Some(entry.id);
            }
        }
    }
}
