//! # dx-packet: Binary Protocol Types
//!
//! Zero-dependency crate defining the memory layout contract between
//! dx-server (serializer) and dx-client (WASM runtime).
//!
//! All types are `#[repr(C)]` for predictable memory layout and zero-copy parsing.
//!
//! ## HTIP v2 Wire Format
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │  Ed25519 Signature (64 bytes)          │  ← Verified by JS loader BEFORE WASM
//! ├────────────────────────────────────────┤
//! │  HtipHeader (16 bytes)                 │
//! ├────────────────────────────────────────┤
//! │  String Table (variable)               │
//! ├────────────────────────────────────────┤
//! │  Template Dictionary (variable)        │
//! ├────────────────────────────────────────┤
//! │  Opcode Stream (variable)              │
//! └────────────────────────────────────────┘
//! ```

#![no_std]
extern crate alloc;

// ============================================================================
// HEADER
// ============================================================================

/// HTIP Header - first 16 bytes after signature
///
/// Memory Layout:
/// ```text
/// Offset  Size  Field
/// 0       2     magic (0x4458 = "DX")
/// 2       1     version
/// 3       1     flags
/// 4       2     template_count
/// 6       2     string_count
/// 8       4     opcode_count
/// 12      4     payload_size
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct HtipHeader {
    /// Magic bytes: 0x4458 ("DX" in little-endian)
    pub magic: u16,
    /// Protocol version (currently 2)
    pub version: u8,
    /// Flags: bit 0 = has_strings, bit 1 = has_templates
    pub flags: u8,
    /// Number of templates in dictionary
    pub template_count: u16,
    /// Number of strings in string table
    pub string_count: u16,
    /// Number of opcodes in stream
    pub opcode_count: u32,
    /// Total payload size (excluding signature)
    pub payload_size: u32,
}

impl HtipHeader {
    pub const MAGIC: u16 = 0x4458; // "DX"
    pub const VERSION: u8 = 2;
    pub const SIZE: usize = 16;

    /// Validate header magic and version
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC && self.version == Self::VERSION
    }
}

// ============================================================================
// OPCODES
// ============================================================================

/// Opcode types for DOM manipulation
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpType {
    /// Clone template and append to parent
    Clone = 1,
    /// Update text content of node
    PatchText = 2,
    /// Update attribute value
    PatchAttr = 3,
    /// Toggle CSS class
    ClassToggle = 4,
    /// Remove node from DOM
    Remove = 5,
    /// Set style property
    SetStyle = 6,
    /// Batch start marker
    BatchStart = 7,
    /// Batch commit marker
    BatchCommit = 8,
}

impl OpType {
    #[inline]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Clone),
            2 => Some(Self::PatchText),
            3 => Some(Self::PatchAttr),
            4 => Some(Self::ClassToggle),
            5 => Some(Self::Remove),
            6 => Some(Self::SetStyle),
            7 => Some(Self::BatchStart),
            8 => Some(Self::BatchCommit),
            _ => None,
        }
    }
}

/// Fixed-size opcode header (4 bytes)
///
/// Memory Layout:
/// ```text
/// Offset  Size  Field
/// 0       1     op_type
/// 1       1     reserved (alignment)
/// 2       2     target_id
/// ```
///
/// Payload follows inline based on op_type
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct OpcodeHeader {
    /// Operation type
    pub op_type: u8,
    /// Reserved for alignment
    pub reserved: u8,
    /// Target node ID (0 = root)
    pub target_id: u16,
}

impl OpcodeHeader {
    pub const SIZE: usize = 4;
}

// ============================================================================
// OPCODE PAYLOADS
// ============================================================================

/// Clone operation: instantiate template
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClonePayload {
    /// Template ID to clone
    pub template_id: u16,
    /// Parent node ID
    pub parent_id: u16,
}

/// Text patch: update node text content
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PatchTextPayload {
    /// String table index for new text
    pub string_idx: u16,
    /// Reserved
    pub reserved: u16,
}

/// Attribute patch: update attribute value
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PatchAttrPayload {
    /// String table index for attribute name
    pub attr_name_idx: u16,
    /// String table index for attribute value
    pub attr_value_idx: u16,
}

/// Class toggle: add/remove CSS class
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ClassTogglePayload {
    /// String table index for class name
    pub class_name_idx: u16,
    /// 1 = add, 0 = remove
    pub enable: u8,
    /// Reserved
    pub reserved: u8,
}

/// Style set: update inline style
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SetStylePayload {
    /// String table index for property name
    pub prop_name_idx: u16,
    /// String table index for property value
    pub prop_value_idx: u16,
}

// ============================================================================
// STRING TABLE
// ============================================================================

/// String entry header in string table
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StringEntry {
    /// Offset into string data region
    pub offset: u32,
    /// Length in bytes
    pub len: u16,
    /// Reserved
    pub reserved: u16,
}

impl StringEntry {
    pub const SIZE: usize = 8;
}

// ============================================================================
// TEMPLATE DICTIONARY
// ============================================================================

/// Template entry in template dictionary
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TemplateEntry {
    /// Template ID
    pub id: u16,
    /// String table index for HTML content
    pub html_string_idx: u16,
    /// Number of slots in template
    pub slot_count: u8,
    /// Reserved
    pub reserved: [u8; 3],
}

impl TemplateEntry {
    pub const SIZE: usize = 8;
}

// ============================================================================
// ERROR CODES (No strings, just u8)
// ============================================================================

/// Error codes for dx-client (no string formatting)
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    /// Success
    Ok = 0,
    /// Invalid magic bytes
    InvalidMagic = 1,
    /// Unsupported version
    UnsupportedVersion = 2,
    /// Invalid opcode
    InvalidOpcode = 3,
    /// Template not found
    TemplateNotFound = 4,
    /// String index out of bounds
    StringIndexOutOfBounds = 5,
    /// Node not found
    NodeNotFound = 6,
    /// Buffer too small
    BufferTooSmall = 7,
}

// ============================================================================
// CONSTANTS
// ============================================================================

/// Ed25519 signature size (verified by JS before WASM)
pub const SIGNATURE_SIZE: usize = 64;

/// Maximum templates
pub const MAX_TEMPLATES: u16 = 4096;

/// Maximum strings in table
pub const MAX_STRINGS: u16 = 65535;

/// Maximum nodes in registry
pub const MAX_NODES: u16 = 65535;

// ============================================================================
// SHARED TYPES (Compiler <-> Server)
// ============================================================================

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

/// Template definition for static HTML structure
/// Used by Compiler (Writer) and Server (Reader)
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct Template {
    pub id: u32,
    pub html: alloc::string::String, // Static HTML with <!--SLOT_N--> markers
    pub slots: alloc::vec::Vec<SlotDef>, // Metadata for each slot
    pub hash: alloc::string::String, // For deduplication
}

/// Slot definition for dynamic content
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct SlotDef {
    pub slot_id: u32,
    pub slot_type: SlotType,
    pub path: alloc::vec::Vec<u32>, // DOM path
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum SlotType {
    Text,      // Text node content
    Attribute, // Element attribute
    Property,  // DOM property
    Event,     // Event listener
}

/// .dxb file structure container
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub struct DxbArtifact {
    pub version: u8,
    pub capabilities: CapabilitiesManifest,
    pub templates: alloc::vec::Vec<Template>,
    pub wasm_size: u32,
}

// ============================================================================
// STREAMING PROTOCOL (Day 16)
// ============================================================================

/// Binary streaming chunk types for progressive loading
/// Enables client to start execution before full download completes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    /// Header: Magic bytes + Version + Signature (64 bytes)
    Header = 0x01,
    /// Layout: Template dictionary (layout.bin)
    Layout = 0x02,
    /// State: Initial state data (state.bin)
    State = 0x03,
    /// Wasm: Runtime logic (logic.wasm)
    Wasm = 0x04,
    /// Patch: Delta patch (binary diff)
    Patch = 0x05,
    /// End of stream marker
    Eof = 0xFF,
}

impl ChunkType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(ChunkType::Header),
            0x02 => Some(ChunkType::Layout),
            0x03 => Some(ChunkType::State),
            0x04 => Some(ChunkType::Wasm),
            0x05 => Some(ChunkType::Patch),
            0xFF => Some(ChunkType::Eof),
            _ => None,
        }
    }
}

/// Block size for XOR patching (4KB - cache friendly)
pub const BLOCK_SIZE: usize = 4096;

/// Chunk header for binary streaming
/// Total size: 5 bytes (1 type + 4 length)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ChunkHeader {
    /// Type of chunk (see ChunkType enum)
    pub chunk_type: u8,
    /// Length of data following this header (Little Endian)
    pub length: u32,
}

impl ChunkHeader {
    pub fn new(chunk_type: ChunkType, length: u32) -> Self {
        Self {
            chunk_type: chunk_type as u8,
            length,
        }
    }

    /// Serialize header to bytes (5 bytes)
    pub fn to_bytes(&self) -> [u8; 5] {
        let mut bytes = [0u8; 5];
        bytes[0] = self.chunk_type;
        bytes[1..5].copy_from_slice(&self.length.to_le_bytes());
        bytes
    }

    /// Deserialize header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 5 {
            return None;
        }
        Some(Self {
            chunk_type: bytes[0],
            length: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
        })
    }
}

/// Header for delta patches (binary diffs)
/// Enables bandwidth-efficient updates by sending only changed bytes
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PatchHeader {
    /// Hash of the base version (what client has)
    pub base_version_hash: u64,
    /// Hash of the target version (what we're patching to)
    pub target_version_hash: u64,
    /// Patch algorithm: 1 = Block XOR, 2 = VCDIFF (future)
    pub patch_algorithm: u8,
}

impl PatchHeader {
    pub fn new(base_hash: u64, target_hash: u64, algorithm: u8) -> Self {
        Self {
            base_version_hash: base_hash,
            target_version_hash: target_hash,
            patch_algorithm: algorithm,
        }
    }

    /// Serialize to 17 bytes: [base:8][target:8][algo:1]
    pub fn to_bytes(&self) -> [u8; 17] {
        let mut bytes = [0u8; 17];
        bytes[0..8].copy_from_slice(&self.base_version_hash.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.target_version_hash.to_le_bytes());
        bytes[16] = self.patch_algorithm;
        bytes
    }

    /// Deserialize from 17 bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 17 {
            return None;
        }
        let base = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let target = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        let algo = bytes[16];
        Some(Self {
            base_version_hash: base,
            target_version_hash: target,
            patch_algorithm: algo,
        })
    }
}

// ============================================================================
// SECURITY & CAPABILITIES
// ============================================================================

/// Capabilities manifest for security
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct CapabilitiesManifest {
    pub network: bool,
    pub storage: bool,
    pub geolocation: bool,
    pub camera: bool,
    pub microphone: bool,
    pub signature: alloc::vec::Vec<u8>,
}
