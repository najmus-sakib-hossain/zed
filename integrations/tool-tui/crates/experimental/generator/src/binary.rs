//! Binary Template Format (.dxt) - Feature #1
//!
//! Pre-compiled binary templates with zero runtime parsing.
//! Templates are compiled at build time to `.dxt` format and memory-mapped directly.
//!
//! ## Binary Template Structure
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │  HEADER (16 bytes)                       │
//! │  - Magic: b"DXT1"                        │
//! │  - Version: u16                          │
//! │  - Flags: u16                            │
//! │  - Checksum: Blake3 (8 bytes truncated)  │
//! ├─────────────────────────────────────────┤
//! │  STRING TABLE                            │
//! │  - Count: u32                            │
//! │  - Entries: [(offset: u32, len: u32)]    │
//! │  - Data: concatenated UTF-8 bytes        │
//! ├─────────────────────────────────────────┤
//! │  PLACEHOLDER TABLE                       │
//! │  - Count: u32                            │
//! │  - Entries: PlaceholderEntry[]           │
//! ├─────────────────────────────────────────┤
//! │  INSTRUCTION STREAM                      │
//! │  - Bytecode for conditionals/loops       │
//! ├─────────────────────────────────────────┤
//! │  METADATA BLOCK                          │
//! │  - Template name, parameters schema      │
//! │  - Dependencies, capabilities            │
//! └─────────────────────────────────────────┘
//! ```

use crate::error::{GeneratorError, Result};
use bytemuck::{Pod, Zeroable};

/// Magic number for DXT files: "DXT1"
pub const DXT_MAGIC: [u8; 4] = *b"DXT1";

/// Current DXT format version
pub const DXT_VERSION: u16 = 1;

/// Header size in bytes
pub const HEADER_SIZE: usize = 16;

/// Maximum string table size (4 MB)
pub const MAX_STRING_TABLE_SIZE: usize = 4 * 1024 * 1024;

/// Maximum instruction stream size (1 MB)
pub const MAX_INSTRUCTION_SIZE: usize = 1024 * 1024;

// ============================================================================
// Header Flags
// ============================================================================

/// Template has no control flow (Micro mode eligible)
pub const FLAG_STATIC: u16 = 0x0001;

/// Template is signed with Ed25519
pub const FLAG_SIGNED: u16 = 0x0002;

/// Template uses string deduplication
pub const FLAG_DEDUPED: u16 = 0x0004;

/// Template has been optimized for size
pub const FLAG_OPTIMIZED: u16 = 0x0008;

/// Template contains unsafe code markers
pub const FLAG_UNSAFE: u16 = 0x0010;

// ============================================================================
// DXT Header
// ============================================================================

/// DXT file header (16 bytes, fixed layout).
///
/// Memory layout is guaranteed via `#[repr(C)]` and `Pod` derive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct DxtHeader {
    /// Magic number: b"DXT1"
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
    /// Flags (see FLAG_* constants)
    pub flags: u16,
    /// Blake3 checksum (truncated to 8 bytes)
    pub checksum: [u8; 8],
}

impl DxtHeader {
    /// Create a new DXT header with the given flags.
    #[must_use]
    pub fn new(flags: u16, checksum: [u8; 8]) -> Self {
        Self {
            magic: DXT_MAGIC,
            version: DXT_VERSION,
            flags,
            checksum,
        }
    }

    /// Validate the header magic and version.
    pub fn validate(&self) -> Result<()> {
        if self.magic != DXT_MAGIC {
            return Err(GeneratorError::InvalidMagic {
                expected: DXT_MAGIC,
                actual: self.magic,
            });
        }

        if self.version > DXT_VERSION {
            return Err(GeneratorError::UnsupportedVersion {
                version: self.version,
                max_supported: DXT_VERSION,
            });
        }

        Ok(())
    }

    /// Check if the template is static (Micro mode eligible).
    #[must_use]
    pub fn is_static(&self) -> bool {
        self.flags & FLAG_STATIC != 0
    }

    /// Check if the template is signed.
    #[must_use]
    pub fn is_signed(&self) -> bool {
        self.flags & FLAG_SIGNED != 0
    }

    /// Check if the template uses string deduplication.
    #[must_use]
    pub fn is_deduped(&self) -> bool {
        self.flags & FLAG_DEDUPED != 0
    }

    /// Check if the template is optimized.
    #[must_use]
    pub fn is_optimized(&self) -> bool {
        self.flags & FLAG_OPTIMIZED != 0
    }

    /// Check if the template contains unsafe markers.
    #[must_use]
    pub fn has_unsafe(&self) -> bool {
        self.flags & FLAG_UNSAFE != 0
    }

    /// Convert header to bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }

    /// Parse header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(GeneratorError::invalid_template("File too small for DXT header"));
        }
        let header: &Self = bytemuck::from_bytes(&bytes[..HEADER_SIZE]);
        header.validate()?;
        Ok(header)
    }
}

// ============================================================================
// String Table
// ============================================================================

/// Entry in the string table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct StringEntry {
    /// Offset into string data section
    pub offset: u32,
    /// Length in bytes
    pub length: u32,
}

/// String table for static text segments.
///
/// All static text in the template is stored here once and referenced by index.
/// This enables string deduplication (e.g., "className" stored once, not 500 times).
#[derive(Clone, Debug)]
pub struct StringTable {
    /// String entry metadata
    entries: Vec<StringEntry>,
    /// Concatenated string data
    data: Vec<u8>,
}

impl StringTable {
    /// Create an empty string table.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            data: Vec::new(),
        }
    }

    /// Add a string and return its index.
    pub fn add(&mut self, s: &str) -> u32 {
        let offset = self.data.len() as u32;
        let length = s.len() as u32;
        self.data.extend_from_slice(s.as_bytes());
        self.entries.push(StringEntry { offset, length });
        (self.entries.len() - 1) as u32
    }

    /// Get a string by index.
    #[must_use]
    pub fn get(&self, index: u32) -> Option<&str> {
        let entry = self.entries.get(index as usize)?;
        let start = entry.offset as usize;
        let end = start + entry.length as usize;
        std::str::from_utf8(&self.data[start..end]).ok()
    }

    /// Number of strings in the table.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the table is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total size in bytes (for serialization).
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        4 + // count
        self.entries.len() * std::mem::size_of::<StringEntry>() +
        self.data.len()
    }

    /// Serialize to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.size_bytes());
        out.extend_from_slice(&(self.entries.len() as u32).to_le_bytes());
        for entry in &self.entries {
            out.extend_from_slice(bytemuck::bytes_of(entry));
        }
        out.extend_from_slice(&self.data);
        out
    }

    /// Deserialize from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 4 {
            return Err(GeneratorError::invalid_template("String table too small"));
        }

        let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
        let entry_size = std::mem::size_of::<StringEntry>();
        let entries_end = 4 + count * entry_size;

        if bytes.len() < entries_end {
            return Err(GeneratorError::invalid_template("String table entries truncated"));
        }

        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let start = 4 + i * entry_size;
            // Copy bytes to aligned buffer to avoid alignment issues
            let mut entry_bytes = [0u8; 8]; // StringEntry is 8 bytes (2 x u32)
            entry_bytes.copy_from_slice(&bytes[start..start + entry_size]);
            let entry: StringEntry = *bytemuck::from_bytes(&entry_bytes);
            entries.push(entry);
        }

        let data = bytes[entries_end..].to_vec();

        Ok(Self { entries, data })
    }
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Placeholder Table
// ============================================================================

/// Placeholder type indicators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PlaceholderType {
    /// Variable substitution: {{ name }}
    Variable = 0,
    /// Conditional block: {% if ... %}
    Conditional = 1,
    /// Loop block: {% for ... %}
    Loop = 2,
    /// Include directive: {% include ... %}
    Include = 3,
    /// Comment: {# ... #}
    Comment = 4,
    /// Raw block: {% raw %}
    Raw = 5,
}

impl TryFrom<u8> for PlaceholderType {
    type Error = GeneratorError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::Variable),
            1 => Ok(Self::Conditional),
            2 => Ok(Self::Loop),
            3 => Ok(Self::Include),
            4 => Ok(Self::Comment),
            5 => Ok(Self::Raw),
            _ => {
                Err(GeneratorError::invalid_template(format!("Invalid placeholder type: {value}")))
            }
        }
    }
}

/// Entry in the placeholder table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct PlaceholderEntry {
    /// Offset in output buffer where placeholder content goes
    pub output_offset: u32,
    /// Maximum length for this placeholder (for fixed-size slots)
    pub max_length: u16,
    /// Placeholder type (PlaceholderType as u8)
    pub placeholder_type: u8,
    /// Reserved for alignment
    pub _reserved: u8,
    /// Variable ID (index into parameter list)
    pub variable_id: u32,
}

impl PlaceholderEntry {
    /// Create a new placeholder entry.
    #[must_use]
    pub fn new(
        output_offset: u32,
        max_length: u16,
        placeholder_type: PlaceholderType,
        variable_id: u32,
    ) -> Self {
        Self {
            output_offset,
            max_length,
            placeholder_type: placeholder_type as u8,
            _reserved: 0,
            variable_id,
        }
    }

    /// Get the placeholder type.
    pub fn get_type(&self) -> Result<PlaceholderType> {
        PlaceholderType::try_from(self.placeholder_type)
    }
}

// ============================================================================
// Instruction Opcodes (for Macro mode)
// ============================================================================

/// Bytecode opcodes for the Macro renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Opcode {
    /// No operation
    Nop = 0x00,
    /// Push static text segment: PUSH_TEXT string_id:u32
    PushText = 0x01,
    /// Push variable value: PUSH_VAR var_id:u32
    PushVar = 0x02,
    /// Conditional jump if false: JMP_FALSE offset:i32
    JmpFalse = 0x10,
    /// Unconditional jump: JMP offset:i32
    Jmp = 0x11,
    /// Begin loop: LOOP_BEGIN var_id:u32 iter_id:u32
    LoopBegin = 0x20,
    /// End loop (jump back to begin): LOOP_END
    LoopEnd = 0x21,
    /// Include template: INCLUDE template_id:u32
    Include = 0x30,
    /// End of stream
    End = 0xFF,
}

impl TryFrom<u8> for Opcode {
    type Error = GeneratorError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(Self::Nop),
            0x01 => Ok(Self::PushText),
            0x02 => Ok(Self::PushVar),
            0x10 => Ok(Self::JmpFalse),
            0x11 => Ok(Self::Jmp),
            0x20 => Ok(Self::LoopBegin),
            0x21 => Ok(Self::LoopEnd),
            0x30 => Ok(Self::Include),
            0xFF => Ok(Self::End),
            _ => Err(GeneratorError::InvalidBytecode {
                offset: 0,
                opcode: value,
            }),
        }
    }
}

// ============================================================================
// Binary Template
// ============================================================================

/// A compiled binary template (.dxt format).
///
/// This is the in-memory representation of a .dxt file.
/// Templates are typically memory-mapped for zero-copy access.
#[derive(Clone, Debug)]
pub struct BinaryTemplate {
    /// File header
    pub header: DxtHeader,
    /// String table (static text segments)
    pub strings: StringTable,
    /// Placeholder entries
    pub placeholders: Vec<PlaceholderEntry>,
    /// Bytecode instructions (for Macro mode)
    pub instructions: Vec<u8>,
    /// Template name
    pub name: String,
    /// Parameter names (indexed by variable_id)
    pub param_names: Vec<String>,
}

impl BinaryTemplate {
    /// Create a new binary template builder.
    #[must_use]
    pub fn builder(name: impl Into<String>) -> BinaryTemplateBuilder {
        BinaryTemplateBuilder::new(name)
    }

    /// Check if this template can use Micro mode (static, no control flow).
    #[must_use]
    pub fn is_micro_eligible(&self) -> bool {
        self.header.is_static()
    }

    /// Serialize to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Header
        out.extend_from_slice(self.header.as_bytes());

        // String table
        let strings_bytes = self.strings.to_bytes();
        out.extend_from_slice(&(strings_bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&strings_bytes);

        // Placeholder table
        out.extend_from_slice(&(self.placeholders.len() as u32).to_le_bytes());
        for ph in &self.placeholders {
            out.extend_from_slice(bytemuck::bytes_of(ph));
        }

        // Instructions
        out.extend_from_slice(&(self.instructions.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.instructions);

        // Metadata: name
        let name_bytes = self.name.as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(name_bytes);

        // Metadata: param names
        out.extend_from_slice(&(self.param_names.len() as u16).to_le_bytes());
        for param in &self.param_names {
            let param_bytes = param.as_bytes();
            out.extend_from_slice(&(param_bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(param_bytes);
        }

        out
    }

    /// Calculate Blake3 checksum for the template content.
    #[must_use]
    pub fn calculate_checksum(&self) -> [u8; 8] {
        let content = self.to_bytes();
        let hash = blake3::hash(&content[HEADER_SIZE..]); // Skip header for checksum
        let mut checksum = [0u8; 8];
        checksum.copy_from_slice(&hash.as_bytes()[..8]);
        checksum
    }
}

// ============================================================================
// Binary Template Builder
// ============================================================================

/// Builder for constructing binary templates.
pub struct BinaryTemplateBuilder {
    name: String,
    strings: StringTable,
    placeholders: Vec<PlaceholderEntry>,
    instructions: Vec<u8>,
    param_names: Vec<String>,
    flags: u16,
}

impl BinaryTemplateBuilder {
    /// Create a new builder with the given template name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            strings: StringTable::new(),
            placeholders: Vec::new(),
            instructions: Vec::new(),
            param_names: Vec::new(),
            flags: 0,
        }
    }

    /// Add a static string and return its index.
    pub fn add_string(&mut self, s: &str) -> u32 {
        self.strings.add(s)
    }

    /// Add a parameter and return its variable ID.
    pub fn add_param(&mut self, name: impl Into<String>) -> u32 {
        self.param_names.push(name.into());
        (self.param_names.len() - 1) as u32
    }

    /// Add a placeholder.
    pub fn add_placeholder(&mut self, entry: PlaceholderEntry) -> &mut Self {
        self.placeholders.push(entry);
        self
    }

    /// Add a bytecode instruction.
    pub fn add_instruction(&mut self, opcode: Opcode) -> &mut Self {
        self.instructions.push(opcode as u8);
        self
    }

    /// Add instruction with u32 argument.
    pub fn add_instruction_u32(&mut self, opcode: Opcode, arg: u32) -> &mut Self {
        self.instructions.push(opcode as u8);
        self.instructions.extend_from_slice(&arg.to_le_bytes());
        self
    }

    /// Add instruction with i32 argument.
    pub fn add_instruction_i32(&mut self, opcode: Opcode, arg: i32) -> &mut Self {
        self.instructions.push(opcode as u8);
        self.instructions.extend_from_slice(&arg.to_le_bytes());
        self
    }

    /// Mark template as static (Micro mode eligible).
    pub fn set_static(&mut self, is_static: bool) -> &mut Self {
        if is_static {
            self.flags |= FLAG_STATIC;
        } else {
            self.flags &= !FLAG_STATIC;
        }
        self
    }

    /// Mark template as signed.
    pub fn set_signed(&mut self, is_signed: bool) -> &mut Self {
        if is_signed {
            self.flags |= FLAG_SIGNED;
        } else {
            self.flags &= !FLAG_SIGNED;
        }
        self
    }

    /// Build the binary template.
    #[must_use]
    pub fn build(self) -> BinaryTemplate {
        let mut template = BinaryTemplate {
            header: DxtHeader::new(self.flags, [0; 8]),
            strings: self.strings,
            placeholders: self.placeholders,
            instructions: self.instructions,
            name: self.name,
            param_names: self.param_names,
        };

        // Calculate and set checksum
        let checksum = template.calculate_checksum();
        template.header.checksum = checksum;

        template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_validation() {
        let header = DxtHeader::new(FLAG_STATIC, [0; 8]);
        assert!(header.validate().is_ok());
        assert!(header.is_static());
        assert!(!header.is_signed());
    }

    #[test]
    fn test_invalid_magic() {
        let mut header = DxtHeader::new(0, [0; 8]);
        header.magic = *b"BAD!";
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_string_table() {
        let mut table = StringTable::new();
        let idx1 = table.add("hello");
        let idx2 = table.add("world");

        assert_eq!(table.get(idx1), Some("hello"));
        assert_eq!(table.get(idx2), Some("world"));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn test_string_table_roundtrip() {
        let mut table = StringTable::new();
        table.add("foo");
        table.add("bar");
        table.add("baz");

        let bytes = table.to_bytes();
        let restored = StringTable::from_bytes(&bytes).unwrap();

        assert_eq!(restored.len(), 3);
        assert_eq!(restored.get(0), Some("foo"));
        assert_eq!(restored.get(1), Some("bar"));
        assert_eq!(restored.get(2), Some("baz"));
    }

    #[test]
    fn test_binary_template_builder() {
        let mut builder = BinaryTemplate::builder("test_template");
        let s1 = builder.add_string("Hello, ");
        let p1 = builder.add_param("name");
        builder.add_string("!");

        builder.add_placeholder(PlaceholderEntry::new(
            7, // After "Hello, "
            64,
            PlaceholderType::Variable,
            p1,
        ));

        builder.set_static(true);

        let template = builder.build();

        assert_eq!(template.name, "test_template");
        assert!(template.is_micro_eligible());
        assert_eq!(template.strings.get(s1), Some("Hello, "));
        assert_eq!(template.param_names[p1 as usize], "name");
    }

    #[test]
    fn test_placeholder_entry() {
        let entry = PlaceholderEntry::new(100, 32, PlaceholderType::Variable, 0);
        assert_eq!(entry.output_offset, 100);
        assert_eq!(entry.max_length, 32);
        assert_eq!(entry.get_type().unwrap(), PlaceholderType::Variable);
    }

    #[test]
    fn test_opcode_conversion() {
        assert_eq!(Opcode::try_from(0x01).unwrap(), Opcode::PushText);
        assert_eq!(Opcode::try_from(0xFF).unwrap(), Opcode::End);
        assert!(Opcode::try_from(0x99).is_err());
    }
}
