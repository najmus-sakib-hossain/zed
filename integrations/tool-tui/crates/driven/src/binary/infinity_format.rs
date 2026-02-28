//! DX ∞ Infinity Format for Rules
//!
//! Inspired by dx-serializer's world-record 186-byte baseline format.

use bytemuck::{Pod, Zeroable};

use crate::{DrivenError, Result};

/// Magic bytes: "DRV∞" (DRV + infinity symbol UTF-8)
pub const INFINITY_MAGIC: &[u8; 4] = b"DRV\x00";

/// Infinity Header (32 bytes, fixed, memory-mapped)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct InfinityHeader {
    /// Magic: "DRV\0" (4 bytes)
    pub magic: [u8; 4],
    /// Format version (2 bytes)
    pub version: u16,
    /// Flags bitfield (2 bytes)
    pub flags: u16,
    /// Checksum (16 bytes)
    pub checksum: [u8; 16],
    /// Reserved (8 bytes)
    pub _reserved: [u8; 8],
}

/// Section offsets for O(1) access
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct SectionOffsets {
    /// String table offset
    pub string_table: u32,
    /// Persona section offset
    pub persona: u32,
    /// Standards section offset
    pub standards: u32,
    /// Workflow section offset
    pub workflow: u32,
    /// Context section offset
    pub context: u32,
    /// Signature offset
    pub signature: u32,
}

/// Rule flags bitfield
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, Default)]
pub struct RuleFlags(pub u16);

impl RuleFlags {
    /// Has persona section
    pub const HAS_PERSONA: u16 = 1 << 0;
    /// Has standards section
    pub const HAS_STANDARDS: u16 = 1 << 1;
    /// Has workflow section
    pub const HAS_WORKFLOW: u16 = 1 << 2;
    /// Has context section
    pub const HAS_CONTEXT: u16 = 1 << 3;
    /// Is cryptographically signed
    pub const IS_SIGNED: u16 = 1 << 4;
    /// Uses fusion cache
    pub const USES_FUSION: u16 = 1 << 5;
    /// Compressed payload
    pub const COMPRESSED: u16 = 1 << 6;
    /// Contains SIMD-optimized data
    pub const SIMD_OPTIMIZED: u16 = 1 << 7;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn has_persona(self) -> bool {
        self.0 & Self::HAS_PERSONA != 0
    }

    pub fn has_standards(self) -> bool {
        self.0 & Self::HAS_STANDARDS != 0
    }

    pub fn has_workflow(self) -> bool {
        self.0 & Self::HAS_WORKFLOW != 0
    }

    pub fn has_context(self) -> bool {
        self.0 & Self::HAS_CONTEXT != 0
    }

    pub fn is_signed(self) -> bool {
        self.0 & Self::IS_SIGNED != 0
    }

    pub fn uses_fusion(self) -> bool {
        self.0 & Self::USES_FUSION != 0
    }

    pub fn set(&mut self, flag: u16) {
        self.0 |= flag;
    }

    pub fn clear(&mut self, flag: u16) {
        self.0 &= !flag;
    }
}

impl InfinityHeader {
    /// Create a new header
    pub fn new(version: u16, flags: RuleFlags) -> Self {
        Self {
            magic: *INFINITY_MAGIC,
            version,
            flags: flags.0,
            checksum: [0; 16],
            _reserved: [0; 8],
        }
    }

    /// Parse header from bytes (zero-copy)
    pub fn from_bytes(data: &[u8]) -> Result<&Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return Err(DrivenError::InvalidBinary("Data too small for infinity header".into()));
        }

        let header: &Self = bytemuck::from_bytes(&data[..std::mem::size_of::<Self>()]);

        if &header.magic != INFINITY_MAGIC {
            return Err(DrivenError::InvalidBinary("Invalid magic bytes".into()));
        }

        Ok(header)
    }

    /// Get flags as typed struct
    pub fn flags(&self) -> RuleFlags {
        RuleFlags(self.flags)
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Header size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Complete infinity rule (header + sections)
#[derive(Debug)]
pub struct InfinityRule<'a> {
    /// The header
    pub header: &'a InfinityHeader,
    /// Section offsets (follows header in binary)
    pub section_offsets: &'a SectionOffsets,
    /// Raw data (memory-mapped)
    data: &'a [u8],
}

impl<'a> InfinityRule<'a> {
    /// Create from raw bytes (zero-copy)
    pub fn from_bytes(data: &'a [u8]) -> Result<Self> {
        let header_size = std::mem::size_of::<InfinityHeader>();
        let offsets_size = std::mem::size_of::<SectionOffsets>();
        let min_size = header_size + offsets_size;

        if data.len() < min_size {
            return Err(DrivenError::InvalidBinary("Data too small for infinity rule".into()));
        }

        let header = InfinityHeader::from_bytes(data)?;
        let section_offsets: &SectionOffsets =
            bytemuck::from_bytes(&data[header_size..header_size + offsets_size]);

        Ok(Self {
            header,
            section_offsets,
            data,
        })
    }

    /// Get string table section
    pub fn string_table_data(&self) -> Option<&'a [u8]> {
        let offset = self.section_offsets.string_table as usize;
        if offset == 0 || offset >= self.data.len() {
            return None;
        }
        Some(&self.data[offset..])
    }

    /// Get persona section
    pub fn persona_data(&self) -> Option<&'a [u8]> {
        if !self.header.flags().has_persona() {
            return None;
        }
        let offset = self.section_offsets.persona as usize;
        if offset == 0 || offset >= self.data.len() {
            return None;
        }
        Some(&self.data[offset..])
    }

    /// Get standards section
    pub fn standards_data(&self) -> Option<&'a [u8]> {
        if !self.header.flags().has_standards() {
            return None;
        }
        let offset = self.section_offsets.standards as usize;
        if offset == 0 || offset >= self.data.len() {
            return None;
        }
        Some(&self.data[offset..])
    }

    /// Get workflow section
    pub fn workflow_data(&self) -> Option<&'a [u8]> {
        if !self.header.flags().has_workflow() {
            return None;
        }
        let offset = self.section_offsets.workflow as usize;
        if offset == 0 || offset >= self.data.len() {
            return None;
        }
        Some(&self.data[offset..])
    }

    /// Get context section
    pub fn context_data(&self) -> Option<&'a [u8]> {
        if !self.header.flags().has_context() {
            return None;
        }
        let offset = self.section_offsets.context as usize;
        if offset == 0 || offset >= self.data.len() {
            return None;
        }
        Some(&self.data[offset..])
    }

    /// Check if signed
    pub fn is_signed(&self) -> bool {
        self.header.flags().is_signed()
    }

    /// Get raw data
    pub fn raw_data(&self) -> &'a [u8] {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(InfinityHeader::size(), 32);
    }

    #[test]
    fn test_flags() {
        let mut flags = RuleFlags::new();
        assert!(!flags.has_persona());

        flags.set(RuleFlags::HAS_PERSONA);
        assert!(flags.has_persona());

        flags.set(RuleFlags::IS_SIGNED);
        assert!(flags.is_signed());
    }

    #[test]
    fn test_header_roundtrip() {
        let header = InfinityHeader::new(1, RuleFlags::new());
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 32);

        let parsed = InfinityHeader::from_bytes(bytes).unwrap();
        assert_eq!(parsed.version, 1);
    }
}
