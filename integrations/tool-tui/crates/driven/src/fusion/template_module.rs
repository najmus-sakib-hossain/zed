//! Pre-Compiled Template Module (.dtm format)
//!
//! Binary template format for instant loading.

use bytemuck::{Pod, Zeroable};

use crate::{DrivenError, Result};

use super::{FUSION_MAGIC, FUSION_VERSION};

/// Fusion header (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FusionHeader {
    /// Magic: "DRVF" (4 bytes)
    pub magic: [u8; 4],
    /// Version (2 bytes)
    pub version: u16,
    /// Flags (2 bytes)
    pub flags: u16,
    /// Number of slots (4 bytes)
    pub slot_count: u32,
    /// String table offset (4 bytes)
    pub string_table_offset: u32,
    /// Slot table offset (4 bytes)
    pub slot_table_offset: u32,
    /// Content offset (4 bytes)
    pub content_offset: u32,
    /// Template ID hash (8 bytes)
    pub template_hash: u64,
    /// Source file hash (8 bytes)
    pub source_hash: u64,
    /// Timestamp (8 bytes)
    pub timestamp: u64,
    /// Reserved (16 bytes)
    pub _reserved: [u8; 16],
}

impl FusionHeader {
    /// Create a new header
    pub fn new(slot_count: u32, template_hash: u64, source_hash: u64) -> Self {
        Self {
            magic: *FUSION_MAGIC,
            version: FUSION_VERSION,
            flags: 0,
            slot_count,
            string_table_offset: Self::size() as u32,
            slot_table_offset: 0,
            content_offset: 0,
            template_hash,
            source_hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            _reserved: [0; 16],
        }
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<&Self> {
        if data.len() < Self::size() {
            return Err(DrivenError::InvalidBinary("Fusion header too small".into()));
        }

        let header: &Self = bytemuck::from_bytes(&data[..Self::size()]);

        if &header.magic != FUSION_MAGIC {
            return Err(DrivenError::InvalidBinary("Invalid fusion magic bytes".into()));
        }

        Ok(header)
    }

    /// Header size
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

/// Template slot (16 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TemplateSlot {
    /// Slot ID
    pub slot_id: u16,
    /// Slot type
    pub slot_type: u8,
    /// Flags
    pub flags: u8,
    /// Name string index
    pub name: u32,
    /// Content offset
    pub content_offset: u32,
    /// Content length
    pub content_length: u32,
}

impl TemplateSlot {
    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Slot types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotType {
    /// Text content
    Text = 0,
    /// Variable placeholder
    Variable = 1,
    /// Conditional block
    Conditional = 2,
    /// Loop block
    Loop = 3,
    /// Include directive
    Include = 4,
}

impl From<u8> for SlotType {
    fn from(v: u8) -> Self {
        match v {
            0 => SlotType::Text,
            1 => SlotType::Variable,
            2 => SlotType::Conditional,
            3 => SlotType::Loop,
            4 => SlotType::Include,
            _ => SlotType::Text,
        }
    }
}

/// Fusion module (compiled template)
#[derive(Debug)]
pub struct FusionModule<'a> {
    /// Header
    pub header: &'a FusionHeader,
    /// Raw data
    data: &'a [u8],
}

impl<'a> FusionModule<'a> {
    /// Load from bytes (zero-copy)
    pub fn from_bytes(data: &'a [u8]) -> Result<Self> {
        let header = FusionHeader::from_bytes(data)?;
        Ok(Self { header, data })
    }

    /// Get all slots
    pub fn slots(&self) -> impl Iterator<Item = &'a TemplateSlot> {
        let offset = self.header.slot_table_offset as usize;
        let count = self.header.slot_count as usize;

        if offset + count * TemplateSlot::size() > self.data.len() {
            return [].iter();
        }

        let slot_data = &self.data[offset..offset + count * TemplateSlot::size()];
        let slots: &[TemplateSlot] = bytemuck::cast_slice(slot_data);
        slots.iter()
    }

    /// Get slot by ID
    pub fn get_slot(&self, slot_id: u16) -> Option<&'a TemplateSlot> {
        self.slots().find(|s| s.slot_id == slot_id)
    }

    /// Get slot content
    pub fn slot_content(&self, slot: &TemplateSlot) -> Option<&'a [u8]> {
        let start = self.header.content_offset as usize + slot.content_offset as usize;
        let end = start + slot.content_length as usize;

        if end > self.data.len() {
            return None;
        }

        Some(&self.data[start..end])
    }

    /// Template hash
    pub fn template_hash(&self) -> u64 {
        self.header.template_hash
    }

    /// Source hash for cache invalidation
    pub fn source_hash(&self) -> u64 {
        self.header.source_hash
    }

    /// Check if cache is stale
    pub fn is_stale(&self, current_source_hash: u64) -> bool {
        self.header.source_hash != current_source_hash
    }
}

/// Fusion module builder
#[derive(Debug)]
pub struct FusionBuilder {
    /// Slots
    slots: Vec<BuiltSlot>,
    /// String table
    strings: Vec<String>,
    /// Content buffer
    content: Vec<u8>,
    /// Template hash
    template_hash: u64,
    /// Source hash
    source_hash: u64,
}

#[derive(Debug)]
struct BuiltSlot {
    slot_type: SlotType,
    name: String,
    content: Vec<u8>,
}

impl FusionBuilder {
    /// Create a new builder
    pub fn new(template_hash: u64, source_hash: u64) -> Self {
        Self {
            slots: Vec::new(),
            strings: Vec::new(),
            content: Vec::new(),
            template_hash,
            source_hash,
        }
    }

    /// Add a text slot
    pub fn add_text(&mut self, name: &str, content: &str) -> u16 {
        let slot_id = self.slots.len() as u16;
        self.slots.push(BuiltSlot {
            slot_type: SlotType::Text,
            name: name.to_string(),
            content: content.as_bytes().to_vec(),
        });
        slot_id
    }

    /// Add a variable slot
    pub fn add_variable(&mut self, name: &str) -> u16 {
        let slot_id = self.slots.len() as u16;
        self.slots.push(BuiltSlot {
            slot_type: SlotType::Variable,
            name: name.to_string(),
            content: Vec::new(),
        });
        slot_id
    }

    /// Build the fusion module
    pub fn build(self) -> Vec<u8> {
        let mut header =
            FusionHeader::new(self.slots.len() as u32, self.template_hash, self.source_hash);

        // Build string table
        let mut string_bytes = Vec::new();
        let mut string_offsets = Vec::new();

        for slot in &self.slots {
            string_offsets.push(string_bytes.len() as u32);
            string_bytes.extend_from_slice(slot.name.as_bytes());
        }

        // Build slot table
        let mut slot_bytes = Vec::new();
        let mut content_bytes = Vec::new();

        for (i, slot) in self.slots.iter().enumerate() {
            let slot_entry = TemplateSlot {
                slot_id: i as u16,
                slot_type: slot.slot_type as u8,
                flags: 0,
                name: string_offsets.get(i).copied().unwrap_or(0),
                content_offset: content_bytes.len() as u32,
                content_length: slot.content.len() as u32,
            };
            slot_bytes.extend_from_slice(bytemuck::bytes_of(&slot_entry));
            content_bytes.extend_from_slice(&slot.content);
        }

        // Calculate offsets
        header.string_table_offset = FusionHeader::size() as u32;
        header.slot_table_offset = header.string_table_offset + string_bytes.len() as u32;
        header.content_offset = header.slot_table_offset + slot_bytes.len() as u32;

        // Assemble output
        let mut output = Vec::new();
        output.extend_from_slice(header.to_bytes());
        output.extend_from_slice(&string_bytes);
        output.extend_from_slice(&slot_bytes);
        output.extend_from_slice(&content_bytes);

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(FusionHeader::size(), 64);
    }

    #[test]
    fn test_slot_size() {
        assert_eq!(TemplateSlot::size(), 16);
    }

    #[test]
    fn test_roundtrip() {
        let mut builder = FusionBuilder::new(12345, 67890);
        builder.add_text("greeting", "Hello, World!");
        builder.add_variable("name");

        let bytes = builder.build();
        let module = FusionModule::from_bytes(&bytes).unwrap();

        assert_eq!(module.header.slot_count, 2);
        assert_eq!(module.template_hash(), 12345);
    }
}
