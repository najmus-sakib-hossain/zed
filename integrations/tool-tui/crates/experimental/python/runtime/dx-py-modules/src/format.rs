//! DPM Format definitions - Header and structures
//!
//! The DPM format uses a cache-line aligned header for O(1) section access.

use bitflags::bitflags;

/// Magic bytes for DPM format identification
pub const DPM_MAGIC: [u8; 4] = *b"DPM\x01";

/// Current DPM format version
pub const DPM_VERSION: u32 = 1;

/// DPM Header - 64 bytes, cache-line aligned
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct DpmHeader {
    /// Magic bytes "DPM\x01"
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// Module flags
    pub flags: DpmFlags,
    /// Reserved for alignment
    pub _reserved: u32,

    // Section offsets
    /// Import table offset
    pub imports_offset: u32,
    /// Export table offset
    pub exports_offset: u32,
    /// Functions section offset
    pub functions_offset: u32,
    /// Classes section offset
    pub classes_offset: u32,
    /// Constants section offset
    pub constants_offset: u32,
    /// Type annotations offset for JIT hints
    pub type_annotations_offset: u32,
    /// Module initialization bytecode offset
    pub init_bytecode_offset: u32,

    // Counts
    /// Number of imports
    pub imports_count: u32,
    /// Number of exports
    pub exports_count: u32,
    /// Perfect hash seed for export lookup
    pub export_hash_seed: u32,

    /// BLAKE3 content hash for integrity
    pub content_hash: [u8; 32],
}

impl DpmHeader {
    pub fn new() -> Self {
        Self {
            magic: DPM_MAGIC,
            version: DPM_VERSION,
            flags: DpmFlags::empty(),
            _reserved: 0,
            imports_offset: 0,
            exports_offset: 0,
            functions_offset: 0,
            classes_offset: 0,
            constants_offset: 0,
            type_annotations_offset: 0,
            init_bytecode_offset: 0,
            imports_count: 0,
            exports_count: 0,
            export_hash_seed: 0,
            content_hash: [0u8; 32],
        }
    }

    #[inline]
    pub fn validate_magic(&self) -> bool {
        self.magic == DPM_MAGIC
    }

    pub fn validate(&self) -> Result<(), DpmError> {
        if !self.validate_magic() {
            return Err(DpmError::InvalidMagic);
        }
        if self.version > DPM_VERSION {
            return Err(DpmError::UnsupportedVersion(self.version));
        }
        Ok(())
    }

    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Default for DpmHeader {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DpmFlags: u32 {
        /// Module has been optimized
        const OPTIMIZED = 0x0001;
        /// Contains type annotations
        const HAS_TYPES = 0x0002;
        /// Is a package (__init__.py)
        const IS_PACKAGE = 0x0004;
        /// Has native extensions
        const HAS_NATIVE = 0x0008;
        /// All symbols pre-resolved
        const SYMBOLS_RESOLVED = 0x0010;
    }
}

/// Import entry in the import table
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ImportEntry {
    /// Offset to module name string
    pub module_name_offset: u32,
    /// Offset to symbol name string (0 for module import)
    pub symbol_name_offset: u32,
    /// Import flags
    pub flags: ImportFlags,
    /// Relative import level (0 for absolute)
    pub level: u8,
    /// Reserved for alignment
    pub _reserved: [u8; 2],
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ImportFlags: u8 {
        /// from X import Y
        const FROM_IMPORT = 0x01;
        /// from X import *
        const STAR_IMPORT = 0x02;
        /// Relative import
        const RELATIVE = 0x04;
        /// Import as alias
        const ALIASED = 0x08;
    }
}

/// Export entry in the export table
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ExportEntry {
    /// Hash of the export name (for perfect hashing)
    pub name_hash: u64,
    /// Offset to the name string
    pub name_offset: u32,
    /// Kind of export
    pub kind: ExportKind,
    /// Reserved for alignment
    pub _reserved: [u8; 3],
    /// Offset to the value in the appropriate section
    pub value_offset: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKind {
    Function = 0,
    Class = 1,
    Constant = 2,
    Variable = 3,
    Module = 4,
}

impl ExportKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Function),
            1 => Some(Self::Class),
            2 => Some(Self::Constant),
            3 => Some(Self::Variable),
            4 => Some(Self::Module),
            _ => None,
        }
    }
}

/// DPM Error types
#[derive(Debug, thiserror::Error)]
pub enum DpmError {
    #[error("Invalid magic bytes - not a DPM file")]
    InvalidMagic,

    #[error("Unsupported DPM version: {0}")]
    UnsupportedVersion(u32),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hash mismatch - file may be corrupted")]
    HashMismatch,

    #[error("Invalid section offset")]
    InvalidOffset,

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Compilation error: {0}")]
    CompileError(String),

    #[error("Perfect hash construction failed")]
    PerfectHashFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_alignment() {
        assert_eq!(std::mem::align_of::<DpmHeader>(), 64);
    }

    #[test]
    fn test_magic_validation() {
        let header = DpmHeader::new();
        assert!(header.validate_magic());

        let mut bad = header;
        bad.magic = *b"BAD\x00";
        assert!(!bad.validate_magic());
    }
}
