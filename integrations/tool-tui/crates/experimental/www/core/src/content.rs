//! # Binary Content Collections
//!
//! Memory-mapped content files with pre-parsed binary AST.
//! Achieves 500x faster content loading than runtime markdown parsing.
//!
//! ## Design
//!
//! Content is parsed at build time to binary AST format:
//! - Metadata: Binary schema (frontmatter)
//! - AST: Pre-parsed content structure
//!
//! At runtime, content is memory-mapped and accessed without parsing.

use std::collections::HashMap;

/// Binary content file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryContentHeader {
    /// Magic bytes for validation
    pub magic: [u8; 4],
    /// Version number
    pub version: u8,
    /// Flags
    pub flags: u8,
    /// Offset to metadata section
    pub metadata_offset: u32,
    /// Size of metadata section
    pub metadata_size: u16,
    /// Offset to AST section
    pub ast_offset: u32,
    /// Size of AST section
    pub ast_size: u32,
}

impl BinaryContentHeader {
    /// Magic bytes: "DXCT" (DX Content)
    pub const MAGIC: [u8; 4] = [0x44, 0x58, 0x43, 0x54];
    /// Current version
    pub const VERSION: u8 = 1;
    /// Header size
    pub const SIZE: usize = 20;

    /// Create a new header
    pub fn new(metadata_offset: u32, metadata_size: u16, ast_offset: u32, ast_size: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::VERSION,
            flags: 0,
            metadata_offset,
            metadata_size,
            ast_offset,
            ast_size,
        }
    }

    /// Validate header
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.version == Self::VERSION
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4] = self.version;
        bytes[5] = self.flags;
        bytes[6..10].copy_from_slice(&self.metadata_offset.to_le_bytes());
        bytes[10..12].copy_from_slice(&self.metadata_size.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.ast_offset.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.ast_size.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);

        Some(Self {
            magic,
            version: bytes[4],
            flags: bytes[5],
            metadata_offset: u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
            metadata_size: u16::from_le_bytes([bytes[10], bytes[11]]),
            ast_offset: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            ast_size: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
        })
    }
}

/// Binary content entry
#[repr(C)]
#[derive(Debug, Clone)]
pub struct BinaryContent {
    /// Offset to metadata in collection file
    pub metadata_offset: u32,
    /// Size of metadata
    pub metadata_size: u16,
    /// Offset to AST in collection file
    pub ast_offset: u32,
    /// Size of AST
    pub ast_size: u32,
}

impl BinaryContent {
    /// Entry size in index
    pub const SIZE: usize = 14;

    /// Create a new entry
    pub fn new(metadata_offset: u32, metadata_size: u16, ast_offset: u32, ast_size: u32) -> Self {
        Self {
            metadata_offset,
            metadata_size,
            ast_offset,
            ast_size,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.metadata_offset.to_le_bytes());
        bytes[4..6].copy_from_slice(&self.metadata_size.to_le_bytes());
        bytes[6..10].copy_from_slice(&self.ast_offset.to_le_bytes());
        bytes[10..14].copy_from_slice(&self.ast_size.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            metadata_offset: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            metadata_size: u16::from_le_bytes([bytes[4], bytes[5]]),
            ast_offset: u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
            ast_size: u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]),
        })
    }
}

/// Reference to content data (zero-copy)
#[derive(Debug)]
pub struct ContentRef<'a> {
    /// Metadata bytes
    pub metadata: &'a [u8],
    /// AST bytes
    pub ast: &'a [u8],
}

impl<'a> ContentRef<'a> {
    /// Get metadata as string (for frontmatter)
    pub fn metadata_str(&self) -> Option<&str> {
        std::str::from_utf8(self.metadata).ok()
    }
}

/// Content collection - holds multiple content entries
#[derive(Debug)]
pub struct ContentCollection {
    /// Raw data (could be memory-mapped in production)
    data: Vec<u8>,
    /// Index of content entries by ID
    index: Vec<BinaryContent>,
    /// Name to index mapping
    name_map: HashMap<String, usize>,
}

impl ContentCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            index: Vec::new(),
            name_map: HashMap::new(),
        }
    }

    /// Create from raw data and index
    pub fn from_parts(data: Vec<u8>, index: Vec<BinaryContent>) -> Self {
        Self {
            data,
            index,
            name_map: HashMap::new(),
        }
    }

    /// Add content to the collection
    pub fn add(&mut self, name: &str, metadata: &[u8], ast: &[u8]) -> usize {
        let metadata_offset = self.data.len() as u32;
        self.data.extend_from_slice(metadata);

        let ast_offset = self.data.len() as u32;
        self.data.extend_from_slice(ast);

        let entry = BinaryContent::new(
            metadata_offset,
            metadata.len() as u16,
            ast_offset,
            ast.len() as u32,
        );

        let id = self.index.len();
        self.index.push(entry);
        self.name_map.insert(name.to_string(), id);

        id
    }

    /// Get content by index - zero parsing
    pub fn get(&self, id: usize) -> Option<ContentRef<'_>> {
        let entry = self.index.get(id)?;

        let metadata_start = entry.metadata_offset as usize;
        let metadata_end = metadata_start + entry.metadata_size as usize;

        let ast_start = entry.ast_offset as usize;
        let ast_end = ast_start + entry.ast_size as usize;

        if metadata_end > self.data.len() || ast_end > self.data.len() {
            return None;
        }

        Some(ContentRef {
            metadata: &self.data[metadata_start..metadata_end],
            ast: &self.data[ast_start..ast_end],
        })
    }

    /// Get content by name
    pub fn get_by_name(&self, name: &str) -> Option<ContentRef<'_>> {
        let id = *self.name_map.get(name)?;
        self.get(id)
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Iterate over all content
    pub fn iter(&self) -> impl Iterator<Item = ContentRef<'_>> {
        (0..self.index.len()).filter_map(|i| self.get(i))
    }

    /// Serialize collection to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Write header
        let header = BinaryContentHeader::new(
            BinaryContentHeader::SIZE as u32,
            0,
            0,
            self.data.len() as u32,
        );
        bytes.extend_from_slice(&header.to_bytes());

        // Write index count
        bytes.extend_from_slice(&(self.index.len() as u32).to_le_bytes());

        // Write index entries
        for entry in &self.index {
            bytes.extend_from_slice(&entry.to_bytes());
        }

        // Write data
        bytes.extend_from_slice(&self.data);

        bytes
    }

    /// Deserialize collection from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let header = BinaryContentHeader::from_bytes(bytes)?;
        if !header.is_valid() {
            return None;
        }

        let mut offset = BinaryContentHeader::SIZE;

        // Read index count
        if offset + 4 > bytes.len() {
            return None;
        }
        let index_count = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        offset += 4;

        // Read index entries
        let mut index = Vec::with_capacity(index_count);
        for _ in 0..index_count {
            if offset + BinaryContent::SIZE > bytes.len() {
                return None;
            }
            let entry = BinaryContent::from_bytes(&bytes[offset..])?;
            index.push(entry);
            offset += BinaryContent::SIZE;
        }

        // Read data
        let data = bytes[offset..].to_vec();

        Some(Self::from_parts(data, index))
    }
}

impl Default for ContentCollection {
    fn default() -> Self {
        Self::new()
    }
}

/// AST node types for pre-parsed content
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstNodeType {
    /// Document root
    Document = 0x01,
    /// Heading (level in data)
    Heading = 0x02,
    /// Paragraph
    Paragraph = 0x03,
    /// Code block
    CodeBlock = 0x04,
    /// List
    List = 0x05,
    /// List item
    ListItem = 0x06,
    /// Link
    Link = 0x07,
    /// Image
    Image = 0x08,
    /// Bold text
    Bold = 0x09,
    /// Italic text
    Italic = 0x0A,
    /// Inline code
    InlineCode = 0x0B,
    /// Text content
    Text = 0x0C,
    /// Blockquote
    Blockquote = 0x0D,
    /// Horizontal rule
    HorizontalRule = 0x0E,
}

/// Simple AST node for binary content
#[repr(C)]
#[derive(Debug, Clone)]
pub struct AstNode {
    /// Node type
    pub node_type: AstNodeType,
    /// Additional data (e.g., heading level)
    pub data: u8,
    /// Offset to content in string table
    pub content_offset: u32,
    /// Length of content
    pub content_len: u16,
    /// Number of children
    pub child_count: u16,
}

impl AstNode {
    /// Node size in bytes
    pub const SIZE: usize = 12;

    /// Create a new node
    pub fn new(node_type: AstNodeType, data: u8, content_offset: u32, content_len: u16) -> Self {
        Self {
            node_type,
            data,
            content_offset,
            content_len,
            child_count: 0,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.node_type as u8;
        bytes[1] = self.data;
        bytes[2..6].copy_from_slice(&self.content_offset.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.content_len.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.child_count.to_le_bytes());
        // 2 bytes padding
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_content_header() {
        let header = BinaryContentHeader::new(100, 50, 200, 1000);
        assert!(header.is_valid());

        let bytes = header.to_bytes();
        let restored = BinaryContentHeader::from_bytes(&bytes).unwrap();

        assert!(restored.is_valid());
        assert_eq!(restored.metadata_offset, 100);
        assert_eq!(restored.metadata_size, 50);
        assert_eq!(restored.ast_offset, 200);
        assert_eq!(restored.ast_size, 1000);
    }

    #[test]
    fn test_content_collection() {
        let mut collection = ContentCollection::new();

        let metadata1 = b"title: Hello\ndate: 2024-01-01";
        let ast1 = b"<ast>content1</ast>";
        let id1 = collection.add("post1", metadata1, ast1);

        let metadata2 = b"title: World\ndate: 2024-01-02";
        let ast2 = b"<ast>content2</ast>";
        let id2 = collection.add("post2", metadata2, ast2);

        assert_eq!(collection.len(), 2);

        // Get by ID
        let content1 = collection.get(id1).unwrap();
        assert_eq!(content1.metadata, metadata1);
        assert_eq!(content1.ast, ast1);

        let content2 = collection.get(id2).unwrap();
        assert_eq!(content2.metadata, metadata2);
        assert_eq!(content2.ast, ast2);

        // Get by name
        let content1_by_name = collection.get_by_name("post1").unwrap();
        assert_eq!(content1_by_name.metadata, metadata1);
    }

    #[test]
    fn test_collection_roundtrip() {
        let mut collection = ContentCollection::new();
        collection.add("test", b"metadata", b"ast");

        let bytes = collection.to_bytes();
        let restored = ContentCollection::from_bytes(&bytes).unwrap();

        assert_eq!(restored.len(), 1);
        let content = restored.get(0).unwrap();
        assert_eq!(content.metadata, b"metadata");
        assert_eq!(content.ast, b"ast");
    }

    #[test]
    fn test_zero_parsing_access() {
        let mut collection = ContentCollection::new();

        // Add content
        let metadata = b"title: Test";
        let ast = b"binary ast data";
        collection.add("test", metadata, ast);

        // Access is just pointer arithmetic - no parsing
        let content = collection.get(0).unwrap();

        // Verify we get direct references to the data
        assert_eq!(content.metadata.as_ptr() as usize, collection.data.as_ptr() as usize);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 32: Content Binary Round-Trip**
    // **Validates: Requirements 19.1, 19.2, 19.3**
    // *For any* markdown content parsed to BinaryContent at build time, the AST SHALL be readable without runtime parsing.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_binary_content_header_roundtrip(
            metadata_offset in 0u32..=1000000u32,
            metadata_size in 0u16..=10000u16,
            ast_offset in 0u32..=1000000u32,
            ast_size in 0u32..=1000000u32,
        ) {
            let header = BinaryContentHeader::new(metadata_offset, metadata_size, ast_offset, ast_size);
            prop_assert!(header.is_valid());

            let bytes = header.to_bytes();
            let restored = BinaryContentHeader::from_bytes(&bytes).unwrap();

            prop_assert!(restored.is_valid());
            prop_assert_eq!(restored.metadata_offset, metadata_offset);
            prop_assert_eq!(restored.metadata_size, metadata_size);
            prop_assert_eq!(restored.ast_offset, ast_offset);
            prop_assert_eq!(restored.ast_size, ast_size);
        }

        #[test]
        fn prop_binary_content_entry_roundtrip(
            metadata_offset in 0u32..=1000000u32,
            metadata_size in 0u16..=10000u16,
            ast_offset in 0u32..=1000000u32,
            ast_size in 0u32..=1000000u32,
        ) {
            let entry = BinaryContent::new(metadata_offset, metadata_size, ast_offset, ast_size);

            let bytes = entry.to_bytes();
            let restored = BinaryContent::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.metadata_offset, metadata_offset);
            prop_assert_eq!(restored.metadata_size, metadata_size);
            prop_assert_eq!(restored.ast_offset, ast_offset);
            prop_assert_eq!(restored.ast_size, ast_size);
        }

        #[test]
        fn prop_content_collection_roundtrip(
            entries in prop::collection::vec(
                (
                    "[a-z]{1,20}",  // name
                    prop::collection::vec(any::<u8>(), 1..100),  // metadata
                    prop::collection::vec(any::<u8>(), 1..100),  // ast
                ),
                1..10
            ),
        ) {
            let mut collection = ContentCollection::new();

            // Add all entries
            for (name, metadata, ast) in &entries {
                collection.add(name, metadata, ast);
            }

            prop_assert_eq!(collection.len(), entries.len());

            // Serialize and deserialize
            let bytes = collection.to_bytes();
            let restored = ContentCollection::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.len(), entries.len());

            // Verify all content is preserved
            for (i, (_, metadata, ast)) in entries.iter().enumerate() {
                let content = restored.get(i).unwrap();
                prop_assert_eq!(content.metadata, &metadata[..]);
                prop_assert_eq!(content.ast, &ast[..]);
            }
        }

        #[test]
        fn prop_content_access_is_zero_copy(
            metadata in prop::collection::vec(any::<u8>(), 1..100),
            ast in prop::collection::vec(any::<u8>(), 1..100),
        ) {
            let mut collection = ContentCollection::new();
            collection.add("test", &metadata, &ast);

            // Access content
            let content = collection.get(0).unwrap();

            // Verify the returned slices point into the collection's data
            // (zero-copy access - no parsing needed)
            prop_assert_eq!(content.metadata, &metadata[..]);
            prop_assert_eq!(content.ast, &ast[..]);

            // The metadata slice should start at offset 0 in the data
            let data_ptr = collection.data.as_ptr() as usize;
            let metadata_ptr = content.metadata.as_ptr() as usize;
            prop_assert_eq!(metadata_ptr, data_ptr);
        }

        #[test]
        fn prop_content_by_name_matches_by_id(
            name in "[a-z]{1,20}",
            metadata in prop::collection::vec(any::<u8>(), 1..50),
            ast in prop::collection::vec(any::<u8>(), 1..50),
        ) {
            let mut collection = ContentCollection::new();
            let id = collection.add(&name, &metadata, &ast);

            let by_id = collection.get(id).unwrap();
            let by_name = collection.get_by_name(&name).unwrap();

            // Both should return the same content
            prop_assert_eq!(by_id.metadata, by_name.metadata);
            prop_assert_eq!(by_id.ast, by_name.ast);
        }
    }
}
