//! Binary Rule Format (.drv - Driven Rule)
//!
//! Zero-copy binary format for AI coding rules, achieving 75% size reduction
//! and 50-150x faster parsing compared to text-based formats.
//!
//! ## Format Structure
//!
//! ```text
//! DrivenRule (.drv)
//! ├── Header (16 bytes)
//! │   ├── Magic: "DRV\0" (4 bytes)
//! │   ├── Version: u16 (2 bytes)
//! │   ├── Flags: u16 (2 bytes)
//! │   ├── Section Count: u32 (4 bytes)
//! │   └── Checksum: u32 (4 bytes - Blake3 truncated)
//! │
//! ├── String Table (variable)
//! │   ├── Count: u32
//! │   └── Strings: [length: u16, bytes: [u8; length]]...
//! │
//! ├── Persona Section (optional)
//! ├── Standards Section (optional)
//! ├── Context Section (optional)
//! └── Workflow Section (optional)
//! ```

mod decoder;
mod encoder;
mod schema;
mod versioning;

pub use decoder::DrvDecoder;
pub use encoder::DrvEncoder;
pub use schema::{
    ContextSection, DrvHeader, PersonaSection, RuleCategory, RuleEntry, StandardsSection,
    WorkflowSection, WorkflowStep,
};
pub use versioning::{FormatVersion, VersionMigrator};

/// Magic bytes for .drv files
pub const DRV_MAGIC: [u8; 4] = *b"DRV\0";

/// Current format version
pub const DRV_VERSION: u16 = 1;

/// Section type identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SectionType {
    /// String table section
    StringTable = 0x01,
    /// Persona definition
    Persona = 0x02,
    /// Coding standards
    Standards = 0x03,
    /// Project context
    Context = 0x04,
    /// Workflow definition
    Workflow = 0x05,
}

impl TryFrom<u8> for SectionType {
    type Error = crate::DrivenError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(SectionType::StringTable),
            0x02 => Ok(SectionType::Persona),
            0x03 => Ok(SectionType::Standards),
            0x04 => Ok(SectionType::Context),
            0x05 => Ok(SectionType::Workflow),
            _ => Err(crate::DrivenError::InvalidBinary(format!(
                "Unknown section type: 0x{:02x}",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magic_bytes() {
        assert_eq!(&DRV_MAGIC, b"DRV\0");
    }

    #[test]
    fn test_section_type_roundtrip() {
        let types = [
            SectionType::StringTable,
            SectionType::Persona,
            SectionType::Standards,
            SectionType::Context,
            SectionType::Workflow,
        ];

        for t in types {
            let byte = t as u8;
            let back = SectionType::try_from(byte).unwrap();
            assert_eq!(t, back);
        }
    }
}
