//! # HTIP v1 Protocol Definition
//!
//! The exact binary layout of an HTIP stream.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::codec::{BinaryDecoder, BinaryEncoder};
use crate::opcodes::{Operation, TemplateDef};
use crate::{MAGIC_BYTES, Result, VERSION};

/// HTIP v1 Header (92 bytes fixed)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct HtipHeader {
    /// Magic bytes: b"DXB1"
    pub magic: [u8; 4],
    /// Version: 1
    pub version: u8,
    /// Reserved (alignment)
    pub _reserved: [u8; 3],
    /// Ed25519 signature (64 bytes)
    pub signature: [u8; 64],
    /// Number of templates in dictionary
    pub template_count: u16,
    /// Alignment padding
    pub _padding: [u8; 2],
    /// Number of strings in string table
    pub string_count: u32,
    /// Total size of templates section (bytes)
    pub total_templates_size: u32,
    /// Total size of opcodes section (bytes)
    pub total_opcodes_size: u32,
    /// CRC32 checksum of payload (after header)
    pub checksum: u32,
}

impl HtipHeader {
    pub const SIZE: usize = 92;

    pub fn new() -> Self {
        Self {
            magic: *MAGIC_BYTES,
            version: VERSION,
            _reserved: [0; 3],
            signature: [0; 64],
            template_count: 0,
            _padding: [0; 2],
            string_count: 0,
            total_templates_size: 0,
            total_opcodes_size: 0,
            checksum: 0,
        }
    }

    pub fn verify(&self) -> crate::Result<()> {
        if &self.magic != MAGIC_BYTES {
            return Err(crate::DxBinaryError::InvalidMagic);
        }
        if self.version != VERSION {
            return Err(crate::DxBinaryError::UnsupportedVersion(self.version));
        }
        Ok(())
    }
}

impl Default for HtipHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete HTIP payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtipPayload {
    pub strings: Vec<String>,
    pub templates: Vec<TemplateDef>,
    pub operations: Vec<Operation>,
}

impl HtipPayload {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            templates: Vec::new(),
            operations: Vec::new(),
        }
    }

    /// Encode payload to binary using DX codec
    pub fn encode(&self) -> Vec<u8> {
        let mut encoder = BinaryEncoder::new(self.estimate_size());

        // Encode strings
        encoder.write_string_array(&self.strings);

        // Encode templates
        encoder.write_u32(self.templates.len() as u32);
        for template in &self.templates {
            template.encode(&mut encoder);
        }

        // Encode operations
        encoder.write_u32(self.operations.len() as u32);
        for op in &self.operations {
            op.encode(&mut encoder);
        }

        encoder.finish()
    }

    /// Decode payload from binary using DX codec
    pub fn decode(data: &[u8]) -> Result<Self> {
        let mut decoder = BinaryDecoder::new(data);

        // Decode strings
        let strings = decoder.read_string_array()?;

        // Decode templates
        let template_count = decoder.read_u32()? as usize;
        let mut templates = Vec::with_capacity(template_count);
        for _ in 0..template_count {
            templates.push(TemplateDef::decode(&mut decoder)?);
        }

        // Decode operations
        let op_count = decoder.read_u32()? as usize;
        let mut operations = Vec::with_capacity(op_count);
        for _ in 0..op_count {
            operations.push(Operation::decode(&mut decoder)?);
        }

        Ok(Self {
            strings,
            templates,
            operations,
        })
    }

    pub fn estimate_size(&self) -> usize {
        let string_size: usize = self.strings.iter().map(|s| s.len() + 4).sum();
        let template_size = self.templates.len() * 128;
        let op_size = self.operations.len() * 32;
        string_size + template_size + op_size + 12
    }
}

impl Default for HtipPayload {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::{Binding, BindingType, Instantiate};

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<HtipHeader>(), HtipHeader::SIZE);
    }

    #[test]
    fn test_header_alignment() {
        assert!(std::mem::align_of::<HtipHeader>() >= 4);
    }

    #[test]
    fn test_header_new() {
        let header = HtipHeader::new();
        assert_eq!(&header.magic, MAGIC_BYTES);
        assert_eq!(header.version, VERSION);
        assert!(header.verify().is_ok());
    }

    #[test]
    fn test_header_pod() {
        let header = HtipHeader::new();
        let bytes: &[u8] = bytemuck::bytes_of(&header);
        assert_eq!(bytes.len(), HtipHeader::SIZE);

        let parsed: &HtipHeader = bytemuck::from_bytes(&bytes[..HtipHeader::SIZE]);
        assert_eq!(&parsed.magic, MAGIC_BYTES);
    }

    #[test]
    fn test_payload_roundtrip() {
        let payload = HtipPayload {
            strings: vec!["hello".to_string(), "world".to_string()],
            templates: vec![TemplateDef {
                id: 0,
                html_string_id: 0,
                bindings: vec![Binding {
                    slot_id: 0,
                    binding_type: BindingType::Text,
                    path: vec![0, 1],
                }],
            }],
            operations: vec![Operation::Instantiate(Instantiate {
                instance_id: 1,
                template_id: 0,
                parent_id: 0,
            })],
        };

        let encoded = payload.encode();
        let decoded = HtipPayload::decode(&encoded).unwrap();

        assert_eq!(decoded.strings, payload.strings);
        assert_eq!(decoded.templates.len(), payload.templates.len());
        assert_eq!(decoded.operations.len(), payload.operations.len());
    }
}
