//! HTIP Stream Builder for Testing
//!
//! Provides utilities to construct valid and invalid HTIP streams
//! for testing the WASM client's stream processing.

/// HTIP Opcodes (matching client/src/lib.rs)
pub const OP_CLONE: u8 = 1;
pub const OP_PATCH_TEXT: u8 = 2;
pub const OP_PATCH_ATTR: u8 = 3;
pub const OP_CLASS_TOGGLE: u8 = 4;
pub const OP_REMOVE: u8 = 5;
pub const OP_EVENT: u8 = 6;
pub const OP_STATE_UPDATE: u8 = 7;
pub const OP_TEMPLATE_DEF: u8 = 8;
pub const OP_DELTA_PATCH: u8 = 9;
pub const OP_EOF: u8 = 255;

/// Delta patch constants
pub const DELTA_MAGIC: [u8; 4] = *b"DXDL";
pub const DELTA_OP_COPY: u8 = 0x01;
pub const DELTA_OP_LITERAL: u8 = 0x02;

/// HTIP stream builder for constructing test streams
#[derive(Default)]
pub struct HtipBuilder {
    data: Vec<u8>,
}

impl HtipBuilder {
    /// Create a new HTIP stream builder with header
    pub fn new() -> Self {
        let mut builder = Self { data: Vec::new() };
        // Add 4-byte header (magic/version)
        builder.data.extend_from_slice(&[0x48, 0x54, 0x49, 0x50]); // "HTIP"
        builder
    }

    /// Create a builder without header (for testing malformed streams)
    pub fn new_raw() -> Self {
        Self { data: Vec::new() }
    }

    /// Add raw bytes
    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.data.extend_from_slice(bytes);
        self
    }

    /// Add OP_CLONE instruction
    pub fn clone_template(mut self, template_id: u8) -> Self {
        self.data.push(OP_CLONE);
        self.data.push(template_id);
        self
    }

    /// Add OP_TEMPLATE_DEF instruction
    pub fn template_def(mut self, id: u8, html: &[u8]) -> Self {
        self.data.push(OP_TEMPLATE_DEF);
        self.data.push(id);
        // Length as u16 little-endian
        let len = html.len() as u16;
        self.data.push((len & 0xFF) as u8);
        self.data.push((len >> 8) as u8);
        self.data.extend_from_slice(html);
        self
    }

    /// Add OP_PATCH_TEXT instruction
    pub fn patch_text(mut self, node_id: u16, text: &[u8]) -> Self {
        self.data.push(OP_PATCH_TEXT);
        // Node ID as u16 little-endian
        self.data.push((node_id & 0xFF) as u8);
        self.data.push((node_id >> 8) as u8);
        // Text length as u16 little-endian
        let len = text.len() as u16;
        self.data.push((len & 0xFF) as u8);
        self.data.push((len >> 8) as u8);
        self.data.extend_from_slice(text);
        self
    }

    /// Add OP_PATCH_ATTR instruction
    pub fn patch_attr(mut self, node_id: u16, key: &[u8], value: &[u8]) -> Self {
        self.data.push(OP_PATCH_ATTR);
        // Node ID as u16 little-endian
        self.data.push((node_id & 0xFF) as u8);
        self.data.push((node_id >> 8) as u8);
        // Key length as u16 little-endian
        let key_len = key.len() as u16;
        self.data.push((key_len & 0xFF) as u8);
        self.data.push((key_len >> 8) as u8);
        self.data.extend_from_slice(key);
        // Value length as u16 little-endian
        let val_len = value.len() as u16;
        self.data.push((val_len & 0xFF) as u8);
        self.data.push((val_len >> 8) as u8);
        self.data.extend_from_slice(value);
        self
    }

    /// Add OP_CLASS_TOGGLE instruction
    pub fn class_toggle(mut self, node_id: u16, class: &[u8], enable: bool) -> Self {
        self.data.push(OP_CLASS_TOGGLE);
        // Node ID as u16 little-endian
        self.data.push((node_id & 0xFF) as u8);
        self.data.push((node_id >> 8) as u8);
        // Class length as u16 little-endian
        let class_len = class.len() as u16;
        self.data.push((class_len & 0xFF) as u8);
        self.data.push((class_len >> 8) as u8);
        self.data.extend_from_slice(class);
        self.data.push(if enable { 1 } else { 0 });
        self
    }

    /// Add OP_REMOVE instruction
    pub fn remove(mut self, node_id: u16) -> Self {
        self.data.push(OP_REMOVE);
        // Node ID as u16 little-endian
        self.data.push((node_id & 0xFF) as u8);
        self.data.push((node_id >> 8) as u8);
        self
    }

    /// Add OP_EVENT instruction
    pub fn event(mut self, node_id: u16, event_type: u8, handler_id: u16) -> Self {
        self.data.push(OP_EVENT);
        // Node ID as u16 little-endian
        self.data.push((node_id & 0xFF) as u8);
        self.data.push((node_id >> 8) as u8);
        self.data.push(event_type);
        // Handler ID as u16 little-endian
        self.data.push((handler_id & 0xFF) as u8);
        self.data.push((handler_id >> 8) as u8);
        self
    }

    /// Add OP_DELTA_PATCH instruction
    pub fn delta_patch(mut self, cache_id: u32, patch_data: &[u8]) -> Self {
        self.data.push(OP_DELTA_PATCH);
        // Cache ID as u32 little-endian
        self.data.push((cache_id & 0xFF) as u8);
        self.data.push(((cache_id >> 8) & 0xFF) as u8);
        self.data.push(((cache_id >> 16) & 0xFF) as u8);
        self.data.push(((cache_id >> 24) & 0xFF) as u8);
        // Patch length as u32 little-endian
        let len = patch_data.len() as u32;
        self.data.push((len & 0xFF) as u8);
        self.data.push(((len >> 8) & 0xFF) as u8);
        self.data.push(((len >> 16) & 0xFF) as u8);
        self.data.push(((len >> 24) & 0xFF) as u8);
        self.data.extend_from_slice(patch_data);
        self
    }

    /// Add OP_EOF instruction
    pub fn eof(mut self) -> Self {
        self.data.push(OP_EOF);
        self
    }

    /// Add an unknown/invalid opcode
    pub fn invalid_opcode(mut self, opcode: u8) -> Self {
        self.data.push(opcode);
        self
    }

    /// Build the final stream
    pub fn build(self) -> Vec<u8> {
        self.data
    }
}

/// Delta patch builder for constructing test patches
#[derive(Default)]
pub struct DeltaPatchBuilder {
    data: Vec<u8>,
}

impl DeltaPatchBuilder {
    /// Create a new delta patch builder with header
    pub fn new(block_size: u16) -> Self {
        let mut builder = Self { data: Vec::new() };
        // Magic bytes
        builder.data.extend_from_slice(&DELTA_MAGIC);
        // Version (1)
        builder.data.push(1);
        // Block size as u16 little-endian
        builder.data.push((block_size & 0xFF) as u8);
        builder.data.push((block_size >> 8) as u8);
        // Base hash (8 bytes, placeholder)
        builder.data.extend_from_slice(&[0u8; 8]);
        // Reserved (1 byte)
        builder.data.push(0);
        builder
    }

    /// Create a builder with invalid magic
    pub fn new_invalid_magic() -> Self {
        let mut builder = Self { data: Vec::new() };
        builder.data.extend_from_slice(b"XXXX");
        builder.data.push(1);
        builder.data.extend_from_slice(&[64, 0]); // block_size = 64
        builder.data.extend_from_slice(&[0u8; 9]); // hash + reserved
        builder
    }

    /// Create a builder with invalid version
    pub fn new_invalid_version() -> Self {
        let mut builder = Self { data: Vec::new() };
        builder.data.extend_from_slice(&DELTA_MAGIC);
        builder.data.push(99); // Invalid version
        builder.data.extend_from_slice(&[64, 0]); // block_size = 64
        builder.data.extend_from_slice(&[0u8; 9]); // hash + reserved
        builder
    }

    /// Add a COPY instruction
    pub fn copy(mut self, block_idx: u32) -> Self {
        self.data.push(DELTA_OP_COPY);
        // Block index as u32 little-endian
        self.data.push((block_idx & 0xFF) as u8);
        self.data.push(((block_idx >> 8) & 0xFF) as u8);
        self.data.push(((block_idx >> 16) & 0xFF) as u8);
        self.data.push(((block_idx >> 24) & 0xFF) as u8);
        self
    }

    /// Add a LITERAL instruction
    pub fn literal(mut self, data: &[u8]) -> Self {
        self.data.push(DELTA_OP_LITERAL);
        // Length as u16 little-endian
        let len = data.len() as u16;
        self.data.push((len & 0xFF) as u8);
        self.data.push((len >> 8) as u8);
        self.data.extend_from_slice(data);
        self
    }

    /// Add an invalid opcode
    pub fn invalid_opcode(mut self, opcode: u8) -> Self {
        self.data.push(opcode);
        self
    }

    /// Build the final patch
    pub fn build(self) -> Vec<u8> {
        self.data
    }
}
