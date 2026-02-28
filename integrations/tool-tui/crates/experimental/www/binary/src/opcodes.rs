//! # HTIP v1 Opcodes
//!
//! The 11 operations that define the entire web rendering protocol.
//!
//! ## Opcode Overview
//!
//! | Opcode | Value | Description |
//! |--------|-------|-------------|
//! | TemplateDef | 0x01 | Define a new template |
//! | Instantiate | 0x02 | Create instance from template |
//! | PatchText | 0x03 | Update text content |
//! | PatchAttr | 0x04 | Update attribute value |
//! | PatchClassToggle | 0x05 | Toggle CSS class |
//! | AttachEvent | 0x06 | Attach event handler |
//! | RemoveNode | 0x07 | Remove node from DOM |
//! | BatchStart | 0x08 | Begin batch operation |
//! | BatchCommit | 0x09 | Commit batch operation |
//! | SetProperty | 0x0A | Set DOM property |
//! | AppendChild | 0x0B | Append child node |

use crate::Result;
use crate::codec::{BinaryDecoder, BinaryEncoder};
use serde::{Deserialize, Serialize};

// ============================================================================
// Opcode Enum
// ============================================================================

/// HTIP v1 opcodes - each fits in a single byte.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpcodeV1 {
    /// Define a new template (0x01)
    TemplateDef = 0x01,
    /// Create instance from template (0x02)
    Instantiate = 0x02,
    /// Update text content (0x03)
    PatchText = 0x03,
    /// Update attribute value (0x04)
    PatchAttr = 0x04,
    /// Toggle CSS class (0x05)
    PatchClassToggle = 0x05,
    /// Attach event handler (0x06)
    AttachEvent = 0x06,
    /// Remove node from DOM (0x07)
    RemoveNode = 0x07,
    /// Begin batch operation (0x08)
    BatchStart = 0x08,
    /// Commit batch operation (0x09)
    BatchCommit = 0x09,
    /// Set DOM property (0x0A)
    SetProperty = 0x0A,
    /// Append child node (0x0B)
    AppendChild = 0x0B,
}

impl OpcodeV1 {
    /// Convert from u8 to opcode, returning None for invalid values.
    pub fn from_u8(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(Self::TemplateDef),
            0x02 => Some(Self::Instantiate),
            0x03 => Some(Self::PatchText),
            0x04 => Some(Self::PatchAttr),
            0x05 => Some(Self::PatchClassToggle),
            0x06 => Some(Self::AttachEvent),
            0x07 => Some(Self::RemoveNode),
            0x08 => Some(Self::BatchStart),
            0x09 => Some(Self::BatchCommit),
            0x0A => Some(Self::SetProperty),
            0x0B => Some(Self::AppendChild),
            _ => None,
        }
    }

    /// Convert opcode to u8.
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

// ============================================================================
// Template Definition
// ============================================================================

/// Template definition with HTML content and bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDef {
    /// Unique template identifier
    pub id: u16,
    /// String table index for HTML content
    pub html_string_id: u32,
    /// Dynamic bindings within the template
    pub bindings: Vec<Binding>,
}

impl TemplateDef {
    /// Encode template definition to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u16(self.id);
        e.write_u32(self.html_string_id);
        e.write_u32(self.bindings.len() as u32);
        for b in &self.bindings {
            b.encode(e);
        }
    }

    /// Decode template definition from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        let id = d.read_u16()?;
        let html_string_id = d.read_u32()?;
        let n = d.read_u32()? as usize;
        let mut bindings = Vec::with_capacity(n);
        for _ in 0..n {
            bindings.push(Binding::decode(d)?);
        }
        Ok(Self {
            id,
            html_string_id,
            bindings,
        })
    }
}

// ============================================================================
// Binding Types
// ============================================================================

/// A binding slot within a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Binding {
    /// Slot identifier within the template
    pub slot_id: u16,
    /// Type of binding (text, attribute, etc.)
    pub binding_type: BindingType,
    /// Path to the bound value in state
    pub path: Vec<u8>,
}

impl Binding {
    /// Encode binding to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u16(self.slot_id);
        self.binding_type.encode(e);
        e.write_u8_array(&self.path);
    }

    /// Decode binding from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            slot_id: d.read_u16()?,
            binding_type: BindingType::decode(d)?,
            path: d.read_u8_array()?,
        })
    }
}

/// Type of binding for a template slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BindingType {
    /// Text content binding
    Text,
    /// Attribute binding with name
    Attribute { attr_name_id: u32 },
    /// Property binding with name
    Property { prop_name_id: u32 },
    /// Event binding with type
    Event { event_type_id: u32 },
    /// CSS class binding
    Class,
}

impl BindingType {
    /// Encode binding type to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        match self {
            Self::Text => e.write_u8(0),
            Self::Attribute { attr_name_id } => {
                e.write_u8(1);
                e.write_u32(*attr_name_id);
            }
            Self::Property { prop_name_id } => {
                e.write_u8(2);
                e.write_u32(*prop_name_id);
            }
            Self::Event { event_type_id } => {
                e.write_u8(3);
                e.write_u32(*event_type_id);
            }
            Self::Class => e.write_u8(4),
        }
    }

    /// Decode binding type from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        match d.read_u8()? {
            0 => Ok(Self::Text),
            1 => Ok(Self::Attribute {
                attr_name_id: d.read_u32()?,
            }),
            2 => Ok(Self::Property {
                prop_name_id: d.read_u32()?,
            }),
            3 => Ok(Self::Event {
                event_type_id: d.read_u32()?,
            }),
            4 => Ok(Self::Class),
            v => Err(crate::DxBinaryError::InvalidOpcode(v)),
        }
    }
}

// ============================================================================
// Operation Payloads
// ============================================================================

/// Instantiate a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instantiate {
    /// Unique instance identifier
    pub instance_id: u32,
    /// Template to instantiate
    pub template_id: u16,
    /// Parent node to attach to
    pub parent_id: u32,
}

impl Instantiate {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
        e.write_u16(self.template_id);
        e.write_u32(self.parent_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
            template_id: d.read_u16()?,
            parent_id: d.read_u32()?,
        })
    }
}

/// Patch text content of a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchText {
    /// Target instance
    pub instance_id: u32,
    /// Slot within instance
    pub slot_id: u16,
    /// String table index for new text
    pub string_id: u32,
}

impl PatchText {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
        e.write_u16(self.slot_id);
        e.write_u32(self.string_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
            slot_id: d.read_u16()?,
            string_id: d.read_u32()?,
        })
    }
}

/// Patch attribute value of a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchAttr {
    /// Target instance
    pub instance_id: u32,
    /// Slot within instance
    pub slot_id: u16,
    /// String table index for attribute name
    pub attr_name_id: u32,
    /// String table index for attribute value
    pub value_id: u32,
}

impl PatchAttr {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
        e.write_u16(self.slot_id);
        e.write_u32(self.attr_name_id);
        e.write_u32(self.value_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
            slot_id: d.read_u16()?,
            attr_name_id: d.read_u32()?,
            value_id: d.read_u32()?,
        })
    }
}

/// Toggle a CSS class on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchClassToggle {
    /// Target instance
    pub instance_id: u32,
    /// String table index for class name
    pub class_name_id: u32,
    /// Whether to add or remove the class
    pub enabled: bool,
}

impl PatchClassToggle {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
        e.write_u32(self.class_name_id);
        e.write_bool(self.enabled);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
            class_name_id: d.read_u32()?,
            enabled: d.read_bool()?,
        })
    }
}

/// Attach an event handler to a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachEvent {
    /// Target instance
    pub instance_id: u32,
    /// String table index for event type (e.g., "click")
    pub event_type_id: u32,
    /// Handler function identifier
    pub handler_id: u32,
}

impl AttachEvent {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
        e.write_u32(self.event_type_id);
        e.write_u32(self.handler_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
            event_type_id: d.read_u32()?,
            handler_id: d.read_u32()?,
        })
    }
}

/// Remove a node from the DOM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveNode {
    /// Instance to remove
    pub instance_id: u32,
}

impl RemoveNode {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
        })
    }
}

/// Begin a batch of operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStart {
    /// Unique batch identifier
    pub batch_id: u32,
}

impl BatchStart {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.batch_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            batch_id: d.read_u32()?,
        })
    }
}

/// Commit a batch of operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCommit {
    /// Batch identifier to commit
    pub batch_id: u32,
}

impl BatchCommit {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.batch_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            batch_id: d.read_u32()?,
        })
    }
}

/// Set a DOM property on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetProperty {
    /// Target instance
    pub instance_id: u32,
    /// String table index for property name
    pub prop_name_id: u32,
    /// Property value
    pub value: PropertyValue,
}

impl SetProperty {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.instance_id);
        e.write_u32(self.prop_name_id);
        self.value.encode(e);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            instance_id: d.read_u32()?,
            prop_name_id: d.read_u32()?,
            value: PropertyValue::decode(d)?,
        })
    }
}

/// Property value types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    /// String value (string table index)
    String(u32),
    /// Numeric value
    Number(f64),
    /// Boolean value
    Boolean(bool),
    /// Null value
    Null,
}

impl PropertyValue {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        match self {
            Self::String(id) => {
                e.write_u8(0);
                e.write_u32(*id);
            }
            Self::Number(n) => {
                e.write_u8(1);
                e.write_f64(*n);
            }
            Self::Boolean(b) => {
                e.write_u8(2);
                e.write_bool(*b);
            }
            Self::Null => e.write_u8(3),
        }
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        match d.read_u8()? {
            0 => Ok(Self::String(d.read_u32()?)),
            1 => Ok(Self::Number(d.read_f64()?)),
            2 => Ok(Self::Boolean(d.read_bool()?)),
            3 => Ok(Self::Null),
            v => Err(crate::DxBinaryError::InvalidOpcode(v)),
        }
    }
}

/// Append a child node to a parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendChild {
    /// Parent node
    pub parent_id: u32,
    /// Child node to append
    pub child_id: u32,
}

impl AppendChild {
    /// Encode to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u32(self.parent_id);
        e.write_u32(self.child_id);
    }

    /// Decode from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        Ok(Self {
            parent_id: d.read_u32()?,
            child_id: d.read_u32()?,
        })
    }
}

// ============================================================================
// Operation Enum
// ============================================================================

/// A single HTIP operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    /// Define a template
    TemplateDef(TemplateDef),
    /// Instantiate a template
    Instantiate(Instantiate),
    /// Patch text content
    PatchText(PatchText),
    /// Patch attribute
    PatchAttr(PatchAttr),
    /// Toggle CSS class
    PatchClassToggle(PatchClassToggle),
    /// Attach event handler
    AttachEvent(AttachEvent),
    /// Remove node
    RemoveNode(RemoveNode),
    /// Begin batch
    BatchStart(BatchStart),
    /// Commit batch
    BatchCommit(BatchCommit),
    /// Set property
    SetProperty(SetProperty),
    /// Append child
    AppendChild(AppendChild),
}

impl Operation {
    /// Get the opcode for this operation.
    pub fn opcode(&self) -> OpcodeV1 {
        match self {
            Self::TemplateDef(_) => OpcodeV1::TemplateDef,
            Self::Instantiate(_) => OpcodeV1::Instantiate,
            Self::PatchText(_) => OpcodeV1::PatchText,
            Self::PatchAttr(_) => OpcodeV1::PatchAttr,
            Self::PatchClassToggle(_) => OpcodeV1::PatchClassToggle,
            Self::AttachEvent(_) => OpcodeV1::AttachEvent,
            Self::RemoveNode(_) => OpcodeV1::RemoveNode,
            Self::BatchStart(_) => OpcodeV1::BatchStart,
            Self::BatchCommit(_) => OpcodeV1::BatchCommit,
            Self::SetProperty(_) => OpcodeV1::SetProperty,
            Self::AppendChild(_) => OpcodeV1::AppendChild,
        }
    }

    /// Encode operation to binary.
    pub fn encode(&self, e: &mut BinaryEncoder) {
        e.write_u8(self.opcode().to_u8());
        match self {
            Self::TemplateDef(op) => op.encode(e),
            Self::Instantiate(op) => op.encode(e),
            Self::PatchText(op) => op.encode(e),
            Self::PatchAttr(op) => op.encode(e),
            Self::PatchClassToggle(op) => op.encode(e),
            Self::AttachEvent(op) => op.encode(e),
            Self::RemoveNode(op) => op.encode(e),
            Self::BatchStart(op) => op.encode(e),
            Self::BatchCommit(op) => op.encode(e),
            Self::SetProperty(op) => op.encode(e),
            Self::AppendChild(op) => op.encode(e),
        }
    }

    /// Decode operation from binary.
    pub fn decode(d: &mut BinaryDecoder) -> Result<Self> {
        match OpcodeV1::from_u8(d.read_u8()?) {
            Some(OpcodeV1::TemplateDef) => Ok(Self::TemplateDef(TemplateDef::decode(d)?)),
            Some(OpcodeV1::Instantiate) => Ok(Self::Instantiate(Instantiate::decode(d)?)),
            Some(OpcodeV1::PatchText) => Ok(Self::PatchText(PatchText::decode(d)?)),
            Some(OpcodeV1::PatchAttr) => Ok(Self::PatchAttr(PatchAttr::decode(d)?)),
            Some(OpcodeV1::PatchClassToggle) => {
                Ok(Self::PatchClassToggle(PatchClassToggle::decode(d)?))
            }
            Some(OpcodeV1::AttachEvent) => Ok(Self::AttachEvent(AttachEvent::decode(d)?)),
            Some(OpcodeV1::RemoveNode) => Ok(Self::RemoveNode(RemoveNode::decode(d)?)),
            Some(OpcodeV1::BatchStart) => Ok(Self::BatchStart(BatchStart::decode(d)?)),
            Some(OpcodeV1::BatchCommit) => Ok(Self::BatchCommit(BatchCommit::decode(d)?)),
            Some(OpcodeV1::SetProperty) => Ok(Self::SetProperty(SetProperty::decode(d)?)),
            Some(OpcodeV1::AppendChild) => Ok(Self::AppendChild(AppendChild::decode(d)?)),
            None => Err(crate::DxBinaryError::InvalidOpcode(0)),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_conversion() {
        for i in 0x01u8..=0x0Bu8 {
            assert_eq!(OpcodeV1::from_u8(i).unwrap().to_u8(), i);
        }
    }

    #[test]
    fn test_opcode_size() {
        assert_eq!(std::mem::size_of::<OpcodeV1>(), 1);
    }

    #[test]
    fn test_invalid_opcode() {
        assert!(OpcodeV1::from_u8(0x00).is_none());
        assert!(OpcodeV1::from_u8(0xFF).is_none());
    }

    #[test]
    fn test_template_def_roundtrip() {
        let template = TemplateDef {
            id: 42,
            html_string_id: 100,
            bindings: vec![Binding {
                slot_id: 1,
                binding_type: BindingType::Text,
                path: vec![0, 1, 2],
            }],
        };

        let mut encoder = BinaryEncoder::new(256);
        template.encode(&mut encoder);
        let bytes = encoder.finish();

        let mut decoder = BinaryDecoder::new(&bytes);
        let decoded = TemplateDef::decode(&mut decoder).unwrap();

        assert_eq!(template.id, decoded.id);
        assert_eq!(template.html_string_id, decoded.html_string_id);
        assert_eq!(template.bindings.len(), decoded.bindings.len());
    }

    #[test]
    fn test_operation_roundtrip() {
        let ops = vec![
            Operation::PatchText(PatchText {
                instance_id: 1,
                slot_id: 2,
                string_id: 3,
            }),
            Operation::RemoveNode(RemoveNode { instance_id: 42 }),
            Operation::SetProperty(SetProperty {
                instance_id: 1,
                prop_name_id: 2,
                value: PropertyValue::Boolean(true),
            }),
        ];

        for op in ops {
            let mut encoder = BinaryEncoder::new(256);
            op.encode(&mut encoder);
            let bytes = encoder.finish();

            let mut decoder = BinaryDecoder::new(&bytes);
            let decoded = Operation::decode(&mut decoder).unwrap();

            assert_eq!(op.opcode(), decoded.opcode());
        }
    }
}
