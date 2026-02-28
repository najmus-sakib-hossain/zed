//! # HTIP Deserializer
//!
//! Client-side: Streaming zero-copy parser
//!
//! This runs in the browser WASM runtime.

use ed25519_dalek::{Signature, VerifyingKey};

use crate::{
    DxBinaryError, Result,
    opcodes::{Operation, TemplateDef},
    protocol::{HtipHeader, HtipPayload},
    signature::verify_payload,
};

/// Streaming HTIP deserializer (client-side)
#[derive(Debug)]
pub struct HtipStream {
    payload: HtipPayload,
    current_index: usize,
    verified: bool,
}

impl HtipStream {
    /// Create new stream from binary data
    pub fn new(binary: &[u8], verifying_key: &VerifyingKey) -> Result<Self> {
        // Parse header (zero-copy)
        if binary.len() < HtipHeader::SIZE {
            return Err(DxBinaryError::IoError("Binary too short".to_string()));
        }

        let header: &HtipHeader = bytemuck::from_bytes(&binary[..HtipHeader::SIZE]);

        // Verify header (magic bytes and version)
        header.verify()?;

        // Extract signature
        let signature = Signature::from_bytes(&header.signature);

        // Extract payload
        let payload_bytes = &binary[HtipHeader::SIZE..];

        // Verify CRC32 checksum
        let computed_checksum = crc32fast::hash(payload_bytes);
        if computed_checksum != header.checksum {
            return Err(DxBinaryError::ChecksumMismatch {
                expected: header.checksum,
                actual: computed_checksum,
            });
        }

        // Verify signature
        if !verify_payload(payload_bytes, &signature, verifying_key) {
            return Err(DxBinaryError::SignatureVerificationFailed);
        }

        // Deserialize payload using DX codec
        let payload = HtipPayload::decode(payload_bytes)?;

        Ok(Self {
            payload,
            current_index: 0,
            verified: true,
        })
    }

    /// Get next operation index (use with get_operation)
    pub fn next_index(&mut self) -> Option<usize> {
        if self.current_index >= self.payload.operations.len() {
            return None;
        }

        let index = self.current_index;
        self.current_index += 1;
        Some(index)
    }

    /// Get operation at index
    pub fn get_operation_at(&self, index: usize) -> Option<&Operation> {
        self.payload.operations.get(index)
    }

    /// Get all operations (for iteration without lifetime issues)
    pub fn operations(&self) -> &[Operation] {
        &self.payload.operations
    }

    /// Get string by ID
    pub fn get_string(&self, id: u32) -> Option<&str> {
        self.payload.strings.get(id as usize).map(|s| s.as_str())
    }

    /// Get template by ID
    pub fn get_template(&self, id: u16) -> Option<&TemplateDef> {
        self.payload.templates.iter().find(|t| t.id == id)
    }

    /// Is signature verified
    pub fn is_verified(&self) -> bool {
        self.verified
    }

    /// Reset iterator
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Remaining operations
    pub fn remaining(&self) -> usize {
        self.payload.operations.len() - self.current_index
    }
}

/// Batch processor for applying operations in chunks
pub struct BatchProcessor {
    stream: HtipStream,
    batch_size: usize,
}

impl BatchProcessor {
    /// Create new batch processor
    pub fn new(stream: HtipStream, batch_size: usize) -> Self {
        Self { stream, batch_size }
    }

    /// Process next batch (returns indices instead of references)
    pub fn next_batch(&mut self) -> Vec<usize> {
        let mut batch = Vec::with_capacity(self.batch_size);

        for _ in 0..self.batch_size {
            if self.stream.current_index < self.stream.payload.operations.len() {
                batch.push(self.stream.current_index);
                self.stream.current_index += 1;
            } else {
                break;
            }
        }

        batch
    }

    /// Get operation by index
    pub fn get_operation(&self, index: usize) -> Option<&Operation> {
        self.stream.payload.operations.get(index)
    }

    /// Has more batches
    pub fn has_more(&self) -> bool {
        self.stream.remaining() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serializer::HtipWriter;
    use ed25519_dalek::SigningKey;

    #[test]
    fn test_deserializer_basic() {
        let mut writer = HtipWriter::new();
        writer.write_template(0, "<div>Hello</div>", vec![]);
        writer.write_instantiate(1, 0, 0);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).unwrap();

        let verifying_key = signing_key.verifying_key();
        let stream = HtipStream::new(&binary, &verifying_key).unwrap();

        assert!(stream.is_verified());
        assert_eq!(stream.remaining(), 2);

        let ops = stream.operations();
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0], Operation::TemplateDef(_)));
        assert!(matches!(&ops[1], Operation::Instantiate(_)));
    }

    #[test]
    fn test_deserializer_string_lookup() {
        let mut writer = HtipWriter::new();
        let id = writer.add_string("test string");
        writer.write_template(0, "<div></div>", vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).unwrap();

        let verifying_key = signing_key.verifying_key();
        let stream = HtipStream::new(&binary, &verifying_key).unwrap();

        assert_eq!(stream.get_string(id), Some("test string"));
        assert_eq!(stream.get_string(999), None);
    }

    #[test]
    fn test_deserializer_invalid_signature() {
        let mut writer = HtipWriter::new();
        writer.write_template(0, "<div></div>", vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).unwrap();

        let wrong_key = SigningKey::from_bytes(&[1u8; 32]);
        let verifying_key = wrong_key.verifying_key();

        let result = HtipStream::new(&binary, &verifying_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DxBinaryError::SignatureVerificationFailed));
    }

    #[test]
    fn test_deserializer_invalid_checksum() {
        let mut writer = HtipWriter::new();
        writer.write_template(0, "<div></div>", vec![]);

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let mut binary = writer.finish_and_sign(&signing_key).unwrap();

        if binary.len() > HtipHeader::SIZE + 1 {
            binary[HtipHeader::SIZE + 1] ^= 0xFF;
        }

        let verifying_key = signing_key.verifying_key();
        let result = HtipStream::new(&binary, &verifying_key);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DxBinaryError::ChecksumMismatch { .. }));
    }

    #[test]
    fn test_batch_processor() {
        let mut writer = HtipWriter::new();
        for i in 0..10 {
            writer.write_instantiate(i, 0, 0);
        }

        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let binary = writer.finish_and_sign(&signing_key).unwrap();

        let verifying_key = signing_key.verifying_key();
        let stream = HtipStream::new(&binary, &verifying_key).unwrap();

        let mut processor = BatchProcessor::new(stream, 3);

        let batch1 = processor.next_batch();
        assert_eq!(batch1.len(), 3);

        let batch2 = processor.next_batch();
        assert_eq!(batch2.len(), 3);

        assert!(processor.has_more());
        assert!(processor.get_operation(0).is_some());
    }
}
