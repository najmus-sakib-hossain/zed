//! # HTIP Serializer
//!
//! Server-side: Converts template tree → HTIP binary stream
//!
//! This runs in the dx build tool, not in the browser.

use ed25519_dalek::SigningKey;

use crate::{
    MAGIC_BYTES, Result, VERSION,
    opcodes::*,
    protocol::{HtipHeader, HtipPayload},
    signature::sign_payload,
    string_table::StringTable,
    template::TemplateDictionary,
};

/// HTIP writer (server-side serializer)
pub struct HtipWriter {
    string_table: StringTable,
    template_dict: TemplateDictionary,
    operations: Vec<Operation>,
}

impl HtipWriter {
    /// Create new writer
    pub fn new() -> Self {
        Self {
            string_table: StringTable::new(),
            template_dict: TemplateDictionary::new(),
            operations: Vec::new(),
        }
    }

    /// Add string and get ID
    pub fn add_string(&mut self, s: &str) -> u32 {
        self.string_table.add(s)
    }

    /// Write template definition
    pub fn write_template(&mut self, id: u16, html: &str, bindings: Vec<Binding>) {
        let html_string_id = self.add_string(html);

        let template = TemplateDef {
            id,
            html_string_id,
            bindings,
        };

        self.template_dict.add(template.clone());
        self.operations.push(Operation::TemplateDef(template));
    }

    /// Write instantiate operation
    pub fn write_instantiate(&mut self, instance_id: u32, template_id: u16, parent_id: u32) {
        self.operations.push(Operation::Instantiate(Instantiate {
            instance_id,
            template_id,
            parent_id,
        }));
    }

    /// Write patch text operation
    pub fn write_patch_text(&mut self, instance_id: u32, slot_id: u16, text: &str) {
        let string_id = self.add_string(text);
        self.operations.push(Operation::PatchText(PatchText {
            instance_id,
            slot_id,
            string_id,
        }));
    }

    /// Write patch attribute operation
    pub fn write_patch_attr(
        &mut self,
        instance_id: u32,
        slot_id: u16,
        attr_name: &str,
        value: &str,
    ) {
        let attr_name_id = self.add_string(attr_name);
        let value_id = self.add_string(value);

        self.operations.push(Operation::PatchAttr(PatchAttr {
            instance_id,
            slot_id,
            attr_name_id,
            value_id,
        }));
    }

    /// Write class toggle operation
    pub fn write_class_toggle(&mut self, instance_id: u32, class_name: &str, enabled: bool) {
        let class_name_id = self.add_string(class_name);

        self.operations.push(Operation::PatchClassToggle(PatchClassToggle {
            instance_id,
            class_name_id,
            enabled,
        }));
    }

    /// Write attach event operation
    pub fn write_attach_event(&mut self, instance_id: u32, event_type: &str, handler_id: u32) {
        let event_type_id = self.add_string(event_type);

        self.operations.push(Operation::AttachEvent(AttachEvent {
            instance_id,
            event_type_id,
            handler_id,
        }));
    }

    /// Write remove node operation
    pub fn write_remove_node(&mut self, instance_id: u32) {
        self.operations.push(Operation::RemoveNode(RemoveNode { instance_id }));
    }

    /// Write batch start
    pub fn write_batch_start(&mut self, batch_id: u32) {
        self.operations.push(Operation::BatchStart(BatchStart { batch_id }));
    }

    /// Write batch commit
    pub fn write_batch_commit(&mut self, batch_id: u32) {
        self.operations.push(Operation::BatchCommit(BatchCommit { batch_id }));
    }

    /// Write set property operation
    pub fn write_set_property(&mut self, instance_id: u32, prop_name: &str, value: PropertyValue) {
        let prop_name_id = self.add_string(prop_name);

        self.operations.push(Operation::SetProperty(SetProperty {
            instance_id,
            prop_name_id,
            value,
        }));
    }

    /// Write append child operation
    pub fn write_append_child(&mut self, parent_id: u32, child_id: u32) {
        self.operations.push(Operation::AppendChild(AppendChild {
            parent_id,
            child_id,
        }));
    }

    /// Finish and sign the payload
    pub fn finish_and_sign(self, signing_key: &SigningKey) -> Result<Vec<u8>> {
        // Create payload
        let payload = HtipPayload {
            strings: self.string_table.strings().to_vec(),
            templates: self.template_dict.templates().iter().map(|t| (*t).clone()).collect(),
            operations: self.operations,
        };

        // Serialize payload using DX codec
        let payload_bytes = payload.encode();

        // Sign the payload
        let signature = sign_payload(&payload_bytes, signing_key);

        // Compute CRC32 checksum over payload
        let checksum = crc32fast::hash(&payload_bytes);

        // Create header
        let mut header = HtipHeader::new();
        header.magic = *MAGIC_BYTES;
        header.version = VERSION;
        header.signature = signature.to_bytes();
        header.template_count = self.template_dict.len() as u16;
        header.string_count = self.string_table.len() as u32;
        header.total_templates_size = 0;
        header.total_opcodes_size = payload_bytes.len() as u32;
        header.checksum = checksum;

        // Combine header + payload
        let mut result = Vec::with_capacity(HtipHeader::SIZE + payload_bytes.len());
        result.extend_from_slice(bytemuck::bytes_of(&header));
        result.extend_from_slice(&payload_bytes);

        Ok(result)
    }

    /// Finish without signing (for testing)
    #[cfg(test)]
    pub fn finish_unsigned(self) -> Result<Vec<u8>> {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        self.finish_and_sign(&signing_key)
    }
}

impl Default for HtipWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_writer_basic() {
        let mut writer = HtipWriter::new();

        writer.write_template(0, "<div>Hello</div>", vec![]);
        writer.write_instantiate(1, 0, 0);

        let binary = writer.finish_unsigned().unwrap();

        assert!(binary.len() > HtipHeader::SIZE);
        assert_eq!(&binary[0..4], b"DXB1");
    }

    #[test]
    fn test_writer_string_deduplication() {
        let mut writer = HtipWriter::new();

        let id1 = writer.add_string("test");
        let id2 = writer.add_string("test");

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_writer_operations() {
        let mut writer = HtipWriter::new();

        writer.write_template(0, "<div></div>", vec![]);
        writer.write_instantiate(1, 0, 0);
        writer.write_patch_text(1, 0, "Hello");
        writer.write_class_toggle(1, "active", true);

        assert_eq!(writer.operations.len(), 4);
    }
}
