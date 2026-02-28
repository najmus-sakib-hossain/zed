//! Binary format header definitions for DPP and DPL formats
//!
//! These headers use #[repr(C, packed)] for zero-copy memory-mapped access.

use bytemuck::{Pod, Zeroable};

/// DPP Header - 64 bytes, fixed layout for O(1) section access
///
/// The DPP (Dx Python Package) format is a binary package format designed
/// for zero-copy access and instant metadata retrieval.
///
/// Layout (64 bytes total):
/// - magic: 4 bytes
/// - version: 2 bytes
/// - flags: 2 bytes
/// - metadata_offset: 4 bytes
/// - files_offset: 4 bytes
/// - bytecode_offset: 4 bytes
/// - native_offset: 4 bytes
/// - deps_offset: 4 bytes
/// - total_size: 8 bytes
/// - uncompressed_size: 8 bytes
/// - blake3_hash: 32 bytes (but we only use 20 to fit 64 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DppHeader {
    /// Magic number: "DPP\x01"
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
    /// Flags (compression, platform)
    pub flags: u16,

    // Section offsets for O(1) access (20 bytes)
    /// Offset to package metadata section
    pub metadata_offset: u32,
    /// Offset to file table section
    pub files_offset: u32,
    /// Offset to pre-compiled bytecode (.pyc)
    pub bytecode_offset: u32,
    /// Offset to native extensions (.so/.pyd)
    pub native_offset: u32,
    /// Offset to pre-resolved dependency graph
    pub deps_offset: u32,

    // Sizes (16 bytes)
    /// Total package size in bytes
    pub total_size: u64,
    /// Uncompressed size in bytes
    pub uncompressed_size: u64,

    // Integrity (20 bytes - truncated BLAKE3)
    /// BLAKE3 hash of content (truncated to 20 bytes)
    pub blake3_hash: [u8; 20],
}

// Compile-time assertion that DppHeader is exactly 64 bytes
const _: () = assert!(std::mem::size_of::<DppHeader>() == 64);

/// Package metadata - variable length with fixed structure prefix
///
/// The metadata section follows the header and contains:
/// 1. Fixed 8-byte prefix (DppMetadata struct)
/// 2. Variable-length name bytes (name_len bytes)
/// 3. Variable-length version bytes (version_len bytes)
/// 4. Variable-length python_requires bytes (python_requires_len bytes)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DppMetadata {
    /// Length of package name
    pub name_len: u16,
    /// Length of version string
    pub version_len: u16,
    /// Length of python_requires string
    pub python_requires_len: u16,
    /// Padding for alignment
    pub _padding: u16,
    // Followed by: name bytes, version bytes, python_requires bytes
}

impl DppMetadata {
    /// Create new metadata with the given string lengths
    pub const fn new(name_len: u16, version_len: u16, python_requires_len: u16) -> Self {
        Self {
            name_len,
            version_len,
            python_requires_len,
            _padding: 0,
        }
    }

    /// Calculate the total size of the metadata section including variable-length strings
    pub const fn total_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.name_len as usize
            + self.version_len as usize
            + self.python_requires_len as usize
    }

    /// Get the offset of the name string within the metadata section
    pub const fn name_offset(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    /// Get the offset of the version string within the metadata section
    pub const fn version_offset(&self) -> usize {
        self.name_offset() + self.name_len as usize
    }

    /// Get the offset of the python_requires string within the metadata section
    pub const fn python_requires_offset(&self) -> usize {
        self.version_offset() + self.version_len as usize
    }
}

/// DPP compression flags
pub mod dpp_flags {
    /// No compression
    pub const NONE: u16 = 0;
    /// Zstd compression
    pub const ZSTD: u16 = 1;
    /// LZ4 compression
    pub const LZ4: u16 = 2;
}

/// DPL Header - instant access to lock state
///
/// The DPL (Dx Python Lock) format is a binary lock file format with
/// hash table for O(1) package lookup.
///
/// Layout:
/// - magic: 4 bytes
/// - version: 2 bytes
/// - package_count: 4 bytes
/// - _padding: 2 bytes
/// - hash_table_offset: 4 bytes
/// - hash_table_size: 4 bytes
/// - entries_offset: 4 bytes
/// - python_version: 16 bytes
/// - platform: 32 bytes
/// - resolved_at: 8 bytes
/// - content_hash: 32 bytes
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DplHeader {
    /// Magic number: "DPL\x01"
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
    /// Number of packages in lock file
    pub package_count: u32,
    /// Padding for alignment
    pub _padding: u16,

    // Hash table for O(1) lookup
    /// Offset to hash table
    pub hash_table_offset: u32,
    /// Size of hash table (number of slots)
    pub hash_table_size: u32,

    /// Offset to package entries
    pub entries_offset: u32,

    // Resolution metadata
    /// Python version (e.g., "3.12.0")
    pub python_version: [u8; 16],
    /// Platform string (e.g., "manylinux_2_17_x86_64")
    pub platform: [u8; 32],
    /// Unix timestamp when resolved
    pub resolved_at: u64,

    // Integrity
    /// BLAKE3 hash of all entries
    pub content_hash: [u8; 32],
}

impl DplHeader {
    /// Get the Python version as a string (trimmed of null bytes)
    pub fn python_version_str(&self) -> &str {
        let bytes = &self.python_version;
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..len]).unwrap_or("")
    }

    /// Get the platform as a string (trimmed of null bytes)
    pub fn platform_str(&self) -> &str {
        let bytes = &self.platform;
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..len]).unwrap_or("")
    }
}

/// Package entry in DPL - fixed 128 bytes for predictable layout
///
/// Layout (128 bytes total):
/// - name_hash: 8 bytes
/// - name: 48 bytes
/// - version: 24 bytes
/// - source_type: 1 byte
/// - version_major: 2 bytes
/// - version_minor: 2 bytes
/// - version_patch: 2 bytes
/// - extras_bitmap: 8 bytes (supports up to 64 extras)
/// - source_hash: 32 bytes
/// - _padding: 1 byte
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DplEntry {
    /// FNV-1a hash of package name for fast lookup
    pub name_hash: u64,
    /// Package name (null-terminated, max 47 chars + null)
    pub name: [u8; 48],
    /// Version string (null-terminated, max 23 chars + null)
    pub version: [u8; 24],
    /// Source type: 0=PyPI, 1=Git, 2=URL, 3=Path
    pub source_type: u8,
    /// Major version number (parsed from version string)
    pub version_major: u16,
    /// Minor version number (parsed from version string)
    pub version_minor: u16,
    /// Patch version number (parsed from version string)
    pub version_patch: u16,
    /// Extras bitmap (bit N = 1 means extra N is enabled, up to 64 extras)
    pub extras_bitmap: u64,
    /// Source integrity hash (BLAKE3)
    pub source_hash: [u8; 32],
    /// Padding for alignment
    pub _padding: u8,
}

// Compile-time assertion that DplEntry is exactly 128 bytes
const _: () = assert!(std::mem::size_of::<DplEntry>() == 128);

impl DplEntry {
    /// Create a new DplEntry with the given name, version, and hash
    pub fn new(name: &str, version: &str, source_hash: [u8; 32]) -> Self {
        let mut entry = Self::zeroed();
        entry.name_hash = fnv1a_hash(name.as_bytes());

        // Copy name (max 47 chars + null terminator)
        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len().min(47);
        entry.name[..name_len].copy_from_slice(&name_bytes[..name_len]);

        // Copy version (max 23 chars + null terminator)
        let version_bytes = version.as_bytes();
        let version_len = version_bytes.len().min(23);
        entry.version[..version_len].copy_from_slice(&version_bytes[..version_len]);

        // Parse version components
        let (major, minor, patch) = parse_version_components(version);
        entry.version_major = major;
        entry.version_minor = minor;
        entry.version_patch = patch;

        entry.source_type = SourceType::PyPi as u8;
        entry.source_hash = source_hash;
        entry.extras_bitmap = 0;
        entry._padding = 0;
        entry
    }

    /// Create a new DplEntry with extras
    pub fn new_with_extras(
        name: &str,
        version: &str,
        source_hash: [u8; 32],
        extras_bitmap: u64,
    ) -> Self {
        let mut entry = Self::new(name, version, source_hash);
        entry.extras_bitmap = extras_bitmap;
        entry
    }

    /// Create a zeroed entry
    pub fn zeroed() -> Self {
        bytemuck::Zeroable::zeroed()
    }

    /// Get the package name as a string (trimmed of null bytes)
    pub fn name_str(&self) -> &str {
        let bytes = &self.name;
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..len]).unwrap_or("")
    }

    /// Get the version as a string (trimmed of null bytes)
    pub fn version_str(&self) -> &str {
        let bytes = &self.version;
        let len = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        std::str::from_utf8(&bytes[..len]).unwrap_or("")
    }

    /// Get the source type
    pub fn source_type(&self) -> SourceType {
        SourceType::from(self.source_type)
    }

    /// Check if a specific extra is enabled (by index, 0-63)
    pub fn has_extra(&self, extra_index: u8) -> bool {
        if extra_index >= 64 {
            return false;
        }
        (self.extras_bitmap & (1u64 << extra_index)) != 0
    }

    /// Set an extra as enabled (by index, 0-63)
    pub fn set_extra(&mut self, extra_index: u8) {
        if extra_index < 64 {
            self.extras_bitmap |= 1u64 << extra_index;
        }
    }

    /// Get the parsed version components
    pub fn version_components(&self) -> (u16, u16, u16) {
        (self.version_major, self.version_minor, self.version_patch)
    }
}

/// Parse version string into (major, minor, patch) components
fn parse_version_components(version: &str) -> (u16, u16, u16) {
    let mut parts = version.split(['.', '-', '+']);

    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    (major, minor, patch)
}

/// FNV-1a hash function for package name lookup
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Source type for package entries
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceType {
    PyPi = 0,
    Git = 1,
    Url = 2,
    Path = 3,
}

impl From<u8> for SourceType {
    fn from(value: u8) -> Self {
        match value {
            0 => SourceType::PyPi,
            1 => SourceType::Git,
            2 => SourceType::Url,
            3 => SourceType::Path,
            _ => SourceType::PyPi, // Default fallback
        }
    }
}
