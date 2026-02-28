//! HTIP-Inspired Rule Delivery
//!
//! Binary operation stream for rule synchronization.

use bytemuck::{Pod, Zeroable};

use crate::{DrivenError, Result};

/// Rule operation types (similar to HTIP opcodes)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOperation {
    /// Define a new rule template
    TemplateDefine = 0,
    /// Instantiate a template with bindings
    Instantiate = 1,
    /// Update rule text content
    PatchText = 2,
    /// Update rule metadata
    PatchMeta = 3,
    /// Remove a rule
    Remove = 4,
    /// Begin batch transaction
    BatchStart = 5,
    /// Commit batch transaction
    BatchCommit = 6,
    /// Add a new section
    AddSection = 7,
    /// Reorder rules
    Reorder = 8,
    /// Full sync (replace all)
    FullSync = 9,
    /// End of stream marker
    EndStream = 255,
}

impl From<u8> for RuleOperation {
    fn from(v: u8) -> Self {
        match v {
            0 => RuleOperation::TemplateDefine,
            1 => RuleOperation::Instantiate,
            2 => RuleOperation::PatchText,
            3 => RuleOperation::PatchMeta,
            4 => RuleOperation::Remove,
            5 => RuleOperation::BatchStart,
            6 => RuleOperation::BatchCommit,
            7 => RuleOperation::AddSection,
            8 => RuleOperation::Reorder,
            9 => RuleOperation::FullSync,
            255 => RuleOperation::EndStream,
            _ => RuleOperation::EndStream,
        }
    }
}

/// Operation header (8 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct OperationHeader {
    /// Operation type
    pub op: u8,
    /// Flags
    pub flags: u8,
    /// Target rule ID
    pub target_id: u16,
    /// Payload length
    pub payload_len: u32,
}

impl OperationHeader {
    /// Create a new operation header
    pub fn new(op: RuleOperation, target_id: u16, payload_len: u32) -> Self {
        Self {
            op: op as u8,
            flags: 0,
            target_id,
            payload_len,
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    /// Get operation type
    pub fn operation(&self) -> RuleOperation {
        RuleOperation::from(self.op)
    }
}

/// HTIP-style rule delivery system
#[derive(Debug)]
pub struct HtipDelivery {
    /// Pending operations
    operations: Vec<(OperationHeader, Vec<u8>)>,
    /// In batch mode
    in_batch: bool,
    /// Sequence number
    sequence: u32,
}

impl HtipDelivery {
    /// Create a new delivery instance
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            in_batch: false,
            sequence: 0,
        }
    }

    /// Start a batch operation
    pub fn begin_batch(&mut self) {
        self.operations
            .push((OperationHeader::new(RuleOperation::BatchStart, 0, 0), Vec::new()));
        self.in_batch = true;
    }

    /// Commit the current batch
    pub fn commit_batch(&mut self) {
        if self.in_batch {
            self.operations
                .push((OperationHeader::new(RuleOperation::BatchCommit, 0, 0), Vec::new()));
            self.in_batch = false;
        }
    }

    /// Add a patch text operation
    pub fn patch_text(&mut self, rule_id: u16, new_text: &[u8]) {
        let header = OperationHeader::new(RuleOperation::PatchText, rule_id, new_text.len() as u32);
        self.operations.push((header, new_text.to_vec()));
        self.sequence += 1;
    }

    /// Add a patch metadata operation
    pub fn patch_meta(&mut self, rule_id: u16, metadata: &[u8]) {
        let header = OperationHeader::new(RuleOperation::PatchMeta, rule_id, metadata.len() as u32);
        self.operations.push((header, metadata.to_vec()));
        self.sequence += 1;
    }

    /// Add a remove operation
    pub fn remove(&mut self, rule_id: u16) {
        let header = OperationHeader::new(RuleOperation::Remove, rule_id, 0);
        self.operations.push((header, Vec::new()));
        self.sequence += 1;
    }

    /// Add a full sync operation
    pub fn full_sync(&mut self, data: &[u8]) {
        let header = OperationHeader::new(RuleOperation::FullSync, 0, data.len() as u32);
        self.operations.push((header, data.to_vec()));
        self.sequence += 1;
    }

    /// Serialize all operations to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut output = Vec::new();

        for (header, payload) in &self.operations {
            output.extend_from_slice(bytemuck::bytes_of(header));
            output.extend_from_slice(payload);
        }

        // End marker
        let end = OperationHeader::new(RuleOperation::EndStream, 0, 0);
        output.extend_from_slice(bytemuck::bytes_of(&end));

        output
    }

    /// Clear all pending operations
    pub fn clear(&mut self) {
        self.operations.clear();
        self.in_batch = false;
    }

    /// Get operation count
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u32 {
        self.sequence
    }
}

impl Default for HtipDelivery {
    fn default() -> Self {
        Self::new()
    }
}

/// HTIP stream reader
#[derive(Debug)]
pub struct HtipReader<'a> {
    /// Input data
    data: &'a [u8],
    /// Current position
    pos: usize,
}

impl<'a> HtipReader<'a> {
    /// Create a new reader
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Read next operation
    pub fn next(&mut self) -> Result<Option<(RuleOperation, u16, &'a [u8])>> {
        if self.pos + OperationHeader::size() > self.data.len() {
            return Ok(None);
        }

        let header_bytes = &self.data[self.pos..self.pos + OperationHeader::size()];
        // Copy to aligned buffer to avoid alignment issues
        let mut aligned = [0u8; 8];
        aligned.copy_from_slice(header_bytes);
        let header: OperationHeader = bytemuck::cast(aligned);
        self.pos += OperationHeader::size();

        let op = header.operation();
        if op == RuleOperation::EndStream {
            return Ok(None);
        }

        let payload_len = header.payload_len as usize;
        if self.pos + payload_len > self.data.len() {
            return Err(DrivenError::InvalidBinary("Truncated payload".into()));
        }

        let payload = &self.data[self.pos..self.pos + payload_len];
        self.pos += payload_len;

        Ok(Some((op, header.target_id, payload)))
    }

    /// Read all operations
    pub fn read_all(&mut self) -> Result<Vec<(RuleOperation, u16, Vec<u8>)>> {
        let mut ops = Vec::new();
        while let Some((op, id, payload)) = self.next()? {
            ops.push((op, id, payload.to_vec()));
        }
        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_header_size() {
        assert_eq!(OperationHeader::size(), 8);
    }

    #[test]
    fn test_roundtrip() {
        let mut delivery = HtipDelivery::new();

        delivery.begin_batch();
        delivery.patch_text(1, b"hello");
        delivery.patch_text(2, b"world");
        delivery.commit_batch();

        let bytes = delivery.serialize();

        let mut reader = HtipReader::new(&bytes);
        let ops = reader.read_all().unwrap();

        assert_eq!(ops.len(), 4); // BatchStart, 2x PatchText, BatchCommit
        assert_eq!(ops[0].0, RuleOperation::BatchStart);
        assert_eq!(ops[1].0, RuleOperation::PatchText);
        assert_eq!(ops[1].1, 1);
        assert_eq!(ops[1].2, b"hello");
    }
}
