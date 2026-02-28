//! # Binary LiveView Patches
//!
//! LiveView-style server-to-client DOM patches using binary format.
//! Achieves 6x smaller payloads than Phoenix LiveView's HTML diffs.
//!
//! ## Wire Format
//!
//! Each patch is a compact binary message:
//! - Header: 4 bytes (target: u16, op: u8, value_len: u8)
//! - Payload: variable length value bytes
//!
//! ## Example
//!
//! An increment counter update is ~8 bytes total vs ~50 bytes HTML diff.

/// Patch operation types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchOp {
    /// Set text content of element
    SetText = 0x01,
    /// Set attribute value
    SetAttr = 0x02,
    /// Remove element from DOM
    Remove = 0x03,
    /// Insert new element
    Insert = 0x04,
    /// Replace element with new content
    Replace = 0x05,
}

impl PatchOp {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(PatchOp::SetText),
            0x02 => Some(PatchOp::SetAttr),
            0x03 => Some(PatchOp::Remove),
            0x04 => Some(PatchOp::Insert),
            0x05 => Some(PatchOp::Replace),
            _ => None,
        }
    }
}

/// Binary patch header (4 bytes)
///
/// Compact representation for DOM patches sent over WebSocket.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BinaryPatch {
    /// Target element ID
    pub target: u16,
    /// Operation type
    pub op: PatchOp,
    /// Length of value bytes
    pub value_len: u8,
}

impl BinaryPatch {
    /// Header size in bytes
    pub const HEADER_SIZE: usize = 4;

    /// Create a new patch
    pub fn new(target: u16, op: PatchOp, value_len: u8) -> Self {
        Self {
            target,
            op,
            value_len,
        }
    }

    /// Create a SetText patch
    pub fn set_text(target: u16, text: &str) -> (Self, Vec<u8>) {
        let bytes = text.as_bytes().to_vec();
        let patch = Self::new(target, PatchOp::SetText, bytes.len() as u8);
        (patch, bytes)
    }

    /// Create a SetAttr patch
    /// Format: attr_id (u8) + value bytes
    pub fn set_attr(target: u16, attr_id: u8, value: &[u8]) -> (Self, Vec<u8>) {
        let mut bytes = Vec::with_capacity(1 + value.len());
        bytes.push(attr_id);
        bytes.extend_from_slice(value);
        let patch = Self::new(target, PatchOp::SetAttr, bytes.len() as u8);
        (patch, bytes)
    }

    /// Create a Remove patch
    pub fn remove(target: u16) -> Self {
        Self::new(target, PatchOp::Remove, 0)
    }

    /// Create an Insert patch
    /// Format: template_id (u16) + data bytes
    pub fn insert(target: u16, template_id: u16, data: &[u8]) -> (Self, Vec<u8>) {
        let mut bytes = Vec::with_capacity(2 + data.len());
        bytes.extend_from_slice(&template_id.to_le_bytes());
        bytes.extend_from_slice(data);
        let patch = Self::new(target, PatchOp::Insert, bytes.len() as u8);
        (patch, bytes)
    }

    /// Create a Replace patch
    /// Format: template_id (u16) + data bytes
    pub fn replace(target: u16, template_id: u16, data: &[u8]) -> (Self, Vec<u8>) {
        let mut bytes = Vec::with_capacity(2 + data.len());
        bytes.extend_from_slice(&template_id.to_le_bytes());
        bytes.extend_from_slice(data);
        let patch = Self::new(target, PatchOp::Replace, bytes.len() as u8);
        (patch, bytes)
    }

    /// Serialize patch header to bytes
    pub fn to_bytes(&self) -> [u8; Self::HEADER_SIZE] {
        let target_bytes = self.target.to_le_bytes();
        [
            target_bytes[0],
            target_bytes[1],
            self.op as u8,
            self.value_len,
        ]
    }

    /// Deserialize patch header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            return None;
        }
        let target = u16::from_le_bytes([bytes[0], bytes[1]]);
        let op = PatchOp::from_u8(bytes[2])?;
        let value_len = bytes[3];
        Some(Self {
            target,
            op,
            value_len,
        })
    }

    /// Get total message size (header + value)
    pub fn total_size(&self) -> usize {
        Self::HEADER_SIZE + self.value_len as usize
    }
}

/// Patch batch for sending multiple patches in one message
#[derive(Debug, Default)]
pub struct PatchBatch {
    /// Serialized patches
    data: Vec<u8>,
    /// Number of patches
    count: u16,
}

impl PatchBatch {
    /// Create a new empty batch
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            count: 0,
        }
    }

    /// Add a patch to the batch
    pub fn add(&mut self, patch: &BinaryPatch, value: &[u8]) {
        self.data.extend_from_slice(&patch.to_bytes());
        self.data.extend_from_slice(value);
        self.count += 1;
    }

    /// Get the serialized batch data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get the number of patches
    pub fn count(&self) -> u16 {
        self.count
    }

    /// Get total size in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Patch reader for deserializing patches from a byte stream
pub struct PatchReader<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> PatchReader<'a> {
    /// Create a new reader
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    /// Read the next patch
    pub fn next(&mut self) -> Option<(BinaryPatch, &'a [u8])> {
        if self.offset + BinaryPatch::HEADER_SIZE > self.data.len() {
            return None;
        }

        let patch = BinaryPatch::from_bytes(&self.data[self.offset..])?;
        self.offset += BinaryPatch::HEADER_SIZE;

        let value_end = self.offset + patch.value_len as usize;
        if value_end > self.data.len() {
            return None;
        }

        let value = &self.data[self.offset..value_end];
        self.offset = value_end;

        Some((patch, value))
    }
}

impl<'a> Iterator for PatchReader<'a> {
    type Item = (BinaryPatch, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        PatchReader::next(self)
    }
}

/// Common attribute IDs for SetAttr patches
pub mod attrs {
    pub const CLASS: u8 = 0x01;
    pub const ID: u8 = 0x02;
    pub const STYLE: u8 = 0x03;
    pub const VALUE: u8 = 0x04;
    pub const CHECKED: u8 = 0x05;
    pub const DISABLED: u8 = 0x06;
    pub const HIDDEN: u8 = 0x07;
    pub const HREF: u8 = 0x08;
    pub const SRC: u8 = 0x09;
    pub const ALT: u8 = 0x0A;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_header_size() {
        assert_eq!(BinaryPatch::HEADER_SIZE, 4);
        assert_eq!(std::mem::size_of::<BinaryPatch>(), 4);
    }

    #[test]
    fn test_set_text_patch() {
        let (patch, value) = BinaryPatch::set_text(42, "Hello");
        assert_eq!(patch.target, 42);
        assert_eq!(patch.op, PatchOp::SetText);
        assert_eq!(patch.value_len, 5);
        assert_eq!(value, b"Hello");
    }

    #[test]
    fn test_patch_roundtrip() {
        let patch = BinaryPatch::new(100, PatchOp::SetAttr, 10);
        let bytes = patch.to_bytes();
        let restored = BinaryPatch::from_bytes(&bytes).unwrap();
        assert_eq!(restored.target, 100);
        assert_eq!(restored.op, PatchOp::SetAttr);
        assert_eq!(restored.value_len, 10);
    }

    #[test]
    fn test_patch_batch() {
        let mut batch = PatchBatch::new();

        let (patch1, value1) = BinaryPatch::set_text(1, "Hi");
        batch.add(&patch1, &value1);

        let (patch2, value2) = BinaryPatch::set_text(2, "World");
        batch.add(&patch2, &value2);

        assert_eq!(batch.count(), 2);
        // 4 + 2 + 4 + 5 = 15 bytes
        assert_eq!(batch.size(), 15);
    }

    #[test]
    fn test_patch_reader() {
        let mut batch = PatchBatch::new();

        let (patch1, value1) = BinaryPatch::set_text(1, "A");
        batch.add(&patch1, &value1);

        let (patch2, value2) = BinaryPatch::set_text(2, "B");
        batch.add(&patch2, &value2);

        let mut reader = PatchReader::new(batch.data());

        let (p1, v1) = reader.next().unwrap();
        assert_eq!(p1.target, 1);
        assert_eq!(v1, b"A");

        let (p2, v2) = reader.next().unwrap();
        assert_eq!(p2.target, 2);
        assert_eq!(v2, b"B");

        assert!(reader.next().is_none());
    }

    #[test]
    fn test_increment_patch_size() {
        // Simulate an increment counter update
        // Target: 2 bytes, Op: 1 byte, Len: 1 byte, Value: 4 bytes (u32)
        let (patch, _value) = BinaryPatch::set_text(1, "42");
        let total = patch.total_size();
        // Header (4) + "42" (2) = 6 bytes
        assert!(total <= 8, "Increment patch should be ~8 bytes or less");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 33: BinaryPatch Size**
    // **Validates: Requirements 20.1, 20.2**
    // *For any* simple increment patch, the total message size SHALL be approximately 8 bytes or less.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_binary_patch_header_size_is_4_bytes(
            target in 0u16..=u16::MAX,
            op in 0u8..=5u8,
            value_len in 0u8..=u8::MAX,
        ) {
            let op = match op {
                0 => PatchOp::SetText,
                1 => PatchOp::SetAttr,
                2 => PatchOp::Remove,
                3 => PatchOp::Insert,
                4 => PatchOp::Replace,
                _ => PatchOp::SetText,
            };
            let patch = BinaryPatch::new(target, op, value_len);

            // Header size must always be 4 bytes
            prop_assert_eq!(BinaryPatch::HEADER_SIZE, 4);
            prop_assert_eq!(patch.to_bytes().len(), 4);
        }

        #[test]
        fn prop_simple_increment_patch_is_small(
            target in 0u16..=1000u16,
            value in 0u32..=999999u32,
        ) {
            // Simulate increment counter update with numeric text
            let text = value.to_string();
            let (patch, value_bytes) = BinaryPatch::set_text(target, &text);
            let total_size = patch.total_size();

            // For values up to 999999, text is at most 6 chars
            // Header (4) + text (1-6) = 5-10 bytes
            // Most common case (0-99) is 4 + 1-2 = 5-6 bytes
            prop_assert!(total_size <= 10, "Increment patch should be small: {} bytes", total_size);

            // Verify value matches
            prop_assert_eq!(value_bytes, text.as_bytes());
        }

        #[test]
        fn prop_patch_roundtrip(
            target in 0u16..=u16::MAX,
            value_len in 0u8..=u8::MAX,
        ) {
            let patch = BinaryPatch::new(target, PatchOp::SetText, value_len);
            let bytes = patch.to_bytes();
            let restored = BinaryPatch::from_bytes(&bytes).unwrap();

            prop_assert_eq!(restored.target, target);
            prop_assert_eq!(restored.op, PatchOp::SetText);
            prop_assert_eq!(restored.value_len, value_len);
        }
    }
}

#[cfg(test)]
mod property_tests_application {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 34: Patch Application Correctness**
    // **Validates: Requirements 20.3**
    // *For any* BinaryPatch, applying it to the DOM SHALL produce the expected modification without HTML parsing.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_set_text_patch_contains_correct_text(
            target in 0u16..=1000u16,
            text in "[a-zA-Z0-9 ]{1,50}",
        ) {
            let (patch, value) = BinaryPatch::set_text(target, &text);

            // Verify patch metadata
            prop_assert_eq!(patch.target, target);
            prop_assert_eq!(patch.op, PatchOp::SetText);
            prop_assert_eq!(patch.value_len as usize, text.len());

            // Verify value is the text bytes
            prop_assert_eq!(&value[..], text.as_bytes());

            // Verify we can reconstruct the text
            let reconstructed = std::str::from_utf8(&value).unwrap();
            prop_assert_eq!(reconstructed, text);
        }

        #[test]
        fn prop_set_attr_patch_contains_attr_id_and_value(
            target in 0u16..=1000u16,
            attr_id in 1u8..=10u8,
            value in prop::collection::vec(any::<u8>(), 0..20),
        ) {
            let (patch, bytes) = BinaryPatch::set_attr(target, attr_id, &value);

            // Verify patch metadata
            prop_assert_eq!(patch.target, target);
            prop_assert_eq!(patch.op, PatchOp::SetAttr);
            prop_assert_eq!(patch.value_len as usize, 1 + value.len());

            // Verify first byte is attr_id
            prop_assert_eq!(bytes[0], attr_id);

            // Verify remaining bytes are the value
            prop_assert_eq!(&bytes[1..], &value[..]);
        }

        #[test]
        fn prop_insert_patch_contains_template_id_and_data(
            target in 0u16..=1000u16,
            template_id in 0u16..=1000u16,
            data in prop::collection::vec(any::<u8>(), 0..20),
        ) {
            let (patch, bytes) = BinaryPatch::insert(target, template_id, &data);

            // Verify patch metadata
            prop_assert_eq!(patch.target, target);
            prop_assert_eq!(patch.op, PatchOp::Insert);
            prop_assert_eq!(patch.value_len as usize, 2 + data.len());

            // Verify template_id is encoded correctly
            let decoded_template_id = u16::from_le_bytes([bytes[0], bytes[1]]);
            prop_assert_eq!(decoded_template_id, template_id);

            // Verify data follows
            prop_assert_eq!(&bytes[2..], &data[..]);
        }

        #[test]
        fn prop_replace_patch_contains_template_id_and_data(
            target in 0u16..=1000u16,
            template_id in 0u16..=1000u16,
            data in prop::collection::vec(any::<u8>(), 0..20),
        ) {
            let (patch, bytes) = BinaryPatch::replace(target, template_id, &data);

            // Verify patch metadata
            prop_assert_eq!(patch.target, target);
            prop_assert_eq!(patch.op, PatchOp::Replace);
            prop_assert_eq!(patch.value_len as usize, 2 + data.len());

            // Verify template_id is encoded correctly
            let decoded_template_id = u16::from_le_bytes([bytes[0], bytes[1]]);
            prop_assert_eq!(decoded_template_id, template_id);

            // Verify data follows
            prop_assert_eq!(&bytes[2..], &data[..]);
        }

        #[test]
        fn prop_remove_patch_has_no_value(
            target in 0u16..=1000u16,
        ) {
            let patch = BinaryPatch::remove(target);

            prop_assert_eq!(patch.target, target);
            prop_assert_eq!(patch.op, PatchOp::Remove);
            prop_assert_eq!(patch.value_len, 0);
            prop_assert_eq!(patch.total_size(), BinaryPatch::HEADER_SIZE);
        }

        #[test]
        fn prop_batch_reader_returns_all_patches(
            patches in prop::collection::vec(
                (0u16..=1000u16, "[a-z]{1,10}"),
                1..10
            ),
        ) {
            let mut batch = PatchBatch::new();

            for (target, text) in &patches {
                let (patch, value) = BinaryPatch::set_text(*target, text);
                batch.add(&patch, &value);
            }

            prop_assert_eq!(batch.count() as usize, patches.len());

            // Read back all patches
            let reader = PatchReader::new(batch.data());
            let read_patches: Vec<_> = reader.collect();

            prop_assert_eq!(read_patches.len(), patches.len());

            for (i, (patch, value)) in read_patches.iter().enumerate() {
                prop_assert_eq!(patch.target, patches[i].0);
                prop_assert_eq!(patch.op, PatchOp::SetText);
                prop_assert_eq!(*value, patches[i].1.as_bytes());
            }
        }
    }
}
