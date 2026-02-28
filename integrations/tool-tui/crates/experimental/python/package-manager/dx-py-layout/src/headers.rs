//! Binary format headers for layout cache

use bytemuck::{Pod, Zeroable};

use crate::LAYOUT_VERSION;

/// Layout index header (64 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct LayoutIndexHeader {
    /// Magic: "DXLC"
    pub magic: [u8; 4],
    /// Version
    pub version: u16,
    /// Number of layouts
    pub layout_count: u32,
    /// Hash table offset
    pub hash_table_offset: u32,
    /// Hash table size (slots)
    pub hash_table_size: u32,
    /// Entries offset
    pub entries_offset: u32,
    /// Reserved (split into two arrays for Pod compatibility)
    pub _reserved1: [u8; 32],
    pub _reserved2: [u8; 10],
}

// Compile-time assertion that header is 64 bytes
const _: () = assert!(std::mem::size_of::<LayoutIndexHeader>() == 64);

impl LayoutIndexHeader {
    /// Create a new header
    pub fn new(
        layout_count: u32,
        hash_table_offset: u32,
        hash_table_size: u32,
        entries_offset: u32,
    ) -> Self {
        Self {
            magic: *crate::DXLC_MAGIC,
            version: LAYOUT_VERSION,
            layout_count,
            hash_table_offset,
            hash_table_size,
            entries_offset,
            _reserved1: [0u8; 32],
            _reserved2: [0u8; 10],
        }
    }
}

/// Layout entry (128 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct LayoutEntry {
    /// Project hash (Blake3)
    pub project_hash: [u8; 32],
    /// Layout directory name (relative path, null-terminated)
    pub layout_name: [u8; 64],
    /// Creation timestamp (Unix epoch)
    pub created_at: u64,
    /// Last accessed timestamp (Unix epoch)
    pub last_accessed: u64,
    /// Package count in layout
    pub package_count: u32,
    /// Total size in bytes
    pub total_size: u64,
    /// Reserved
    pub _reserved: [u8; 4],
}

// Compile-time assertion that entry is 128 bytes
const _: () = assert!(std::mem::size_of::<LayoutEntry>() == 128);

impl LayoutEntry {
    /// Create a new layout entry
    pub fn new(
        project_hash: [u8; 32],
        layout_name: &str,
        package_count: u32,
        total_size: u64,
    ) -> Self {
        let mut entry = Self::zeroed();
        entry.project_hash = project_hash;

        // Copy layout name (max 63 chars + null terminator)
        let name_bytes = layout_name.as_bytes();
        let name_len = name_bytes.len().min(63);
        entry.layout_name[..name_len].copy_from_slice(&name_bytes[..name_len]);

        entry.package_count = package_count;
        entry.total_size = total_size;

        // Set timestamps
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        entry.created_at = now;
        entry.last_accessed = now;

        entry
    }

    /// Create a zeroed entry
    pub fn zeroed() -> Self {
        bytemuck::Zeroable::zeroed()
    }

    /// Get the layout name as a string
    pub fn layout_name_str(&self) -> &str {
        let bytes = &self.layout_name;
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..len]).unwrap_or("")
    }
}
