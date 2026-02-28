//! Machine format types for DXM documents using RKYV.
//!
//! DX-Markdown Machine Format IS RKYV.
//!
//! This module provides zero-overhead wrappers around RKYV serialization,
//! using pure RKYV wire format with no modifications.

use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::error::{BinaryError, BinaryResult};
use crate::types::*;

// ============================================================================
// Machine Format Types (RKYV)
// ============================================================================

/// Machine-optimized document representation.
///
/// Uses RKYV for zero-copy serialization/deserialization.
#[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct MachineDocument {
    pub meta: MachineMeta,
    pub refs: Vec<(String, String)>,
    pub nodes: Vec<MachineNode>,
}

/// Machine-optimized metadata.
#[derive(Debug, Clone, PartialEq, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct MachineMeta {
    pub version: String,
    pub token_count: u32,
    pub sections: Vec<MachineSection>,
    pub critical: u16,
    pub important: u16,
    pub low: u16,
}

/// Machine-optimized section info.
#[derive(Debug, Clone, PartialEq, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct MachineSection {
    pub title: String,
    pub level: u8,
    pub offset: u32,
    pub token_count: u32,
}

/// Machine-optimized node.
#[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub enum MachineNode {
    Header {
        level: u8,
        content: Vec<MachineInline>,
        priority: u8,
    },
    Paragraph(Vec<MachineInline>),
    CodeBlock {
        language: Option<String>,
        content: String,
        priority: u8,
    },
    Table {
        schema: Vec<String>,
        rows: Vec<Vec<MachineCell>>,
    },
    List {
        ordered: bool,
        items: Vec<MachineListItem>,
    },
    SemanticBlock {
        block_type: u8,
        priority: u8,
        content: Vec<MachineInline>,
    },
    HorizontalRule,
}

/// Machine-optimized inline node (flattened to avoid recursion).
#[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub enum MachineInline {
    Text(String),
    Bold(String), // Flattened - just the text content
    Italic(String),
    Strikethrough(String),
    Code(String),
    Reference(String),
    Link {
        text: String,
        url: String,
        title: Option<String>,
    },
    Image {
        alt: String,
        url: String,
        title: Option<String>,
    },
}

/// Machine-optimized cell value.
#[derive(Debug, Clone, PartialEq, Archive, RkyvSerialize, RkyvDeserialize)]
pub enum MachineCell {
    Text(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
}

/// Machine-optimized list item (flattened - no nesting).
#[derive(Debug, Clone, PartialEq, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct MachineListItem {
    pub content: Vec<MachineInline>,
    pub indent_level: u8, // Track nesting via indent level instead
}

/// Machine-optimized list.
#[derive(Debug, Clone, PartialEq, Default, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct MachineList {
    pub ordered: bool,
    pub items: Vec<MachineListItem>,
}

// ============================================================================
// Conversion: DxmDocument -> MachineDocument
// ============================================================================

impl From<&DxmDocument> for MachineDocument {
    fn from(doc: &DxmDocument) -> Self {
        Self {
            meta: (&doc.meta).into(),
            refs: doc.refs.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            nodes: doc.nodes.iter().map(|n| n.into()).collect(),
        }
    }
}

impl From<&DxmMeta> for MachineMeta {
    fn from(meta: &DxmMeta) -> Self {
        Self {
            version: meta.version.clone(),
            token_count: meta.token_count as u32,
            sections: meta.sections.iter().map(|s| s.into()).collect(),
            critical: meta.priorities.critical as u16,
            important: meta.priorities.important as u16,
            low: meta.priorities.low as u16,
        }
    }
}

impl From<&SectionInfo> for MachineSection {
    fn from(info: &SectionInfo) -> Self {
        Self {
            title: info.title.clone(),
            level: info.level,
            offset: info.offset as u32,
            token_count: info.token_count as u32,
        }
    }
}

impl From<&DxmNode> for MachineNode {
    fn from(node: &DxmNode) -> Self {
        match node {
            DxmNode::Header(h) => Self::Header {
                level: h.level,
                content: h.content.iter().map(|i| i.into()).collect(),
                priority: priority_to_u8(&h.priority),
            },
            DxmNode::Paragraph(p) => Self::Paragraph(p.iter().map(|i| i.into()).collect()),
            DxmNode::CodeBlock(cb) => Self::CodeBlock {
                language: cb.language.clone(),
                content: cb.content.clone(),
                priority: priority_to_u8(&cb.priority),
            },
            DxmNode::Table(t) => Self::Table {
                schema: t.schema.iter().map(|c| c.name.clone()).collect(),
                rows: t.rows.iter().map(|row| row.iter().map(|c| c.into()).collect()).collect(),
            },
            DxmNode::List(l) => Self::List {
                ordered: l.ordered,
                items: l.items.iter().map(|i| i.into()).collect(),
            },
            DxmNode::SemanticBlock(sb) => Self::SemanticBlock {
                block_type: semantic_type_to_u8(&sb.block_type),
                priority: priority_to_u8(&sb.priority),
                content: sb.content.iter().map(|i| i.into()).collect(),
            },
            DxmNode::HorizontalRule => Self::HorizontalRule,
        }
    }
}

impl From<&InlineNode> for MachineInline {
    fn from(node: &InlineNode) -> Self {
        match node {
            InlineNode::Text(t) => Self::Text(t.clone()),
            InlineNode::Bold(inner) => Self::Bold(flatten_inlines(inner)),
            InlineNode::Italic(inner) => Self::Italic(flatten_inlines(inner)),
            InlineNode::Strikethrough(inner) => Self::Strikethrough(flatten_inlines(inner)),
            InlineNode::Code(c) => Self::Code(c.clone()),
            InlineNode::Reference(r) => Self::Reference(r.clone()),
            InlineNode::Link { text, url, title } => Self::Link {
                text: flatten_inlines(text),
                url: url.clone(),
                title: title.clone(),
            },
            InlineNode::Image { alt, url, title } => Self::Image {
                alt: alt.clone(),
                url: url.clone(),
                title: title.clone(),
            },
        }
    }
}

// Helper to flatten inline nodes to plain text
fn flatten_inlines(inlines: &[InlineNode]) -> String {
    inlines
        .iter()
        .map(|node| match node {
            InlineNode::Text(t) => t.clone(),
            InlineNode::Bold(inner) => flatten_inlines(inner),
            InlineNode::Italic(inner) => flatten_inlines(inner),
            InlineNode::Strikethrough(inner) => flatten_inlines(inner),
            InlineNode::Code(c) => c.clone(),
            InlineNode::Reference(r) => format!("^{}", r),
            InlineNode::Link { text, url, .. } => format!("{} ({})", flatten_inlines(text), url),
            InlineNode::Image { alt, .. } => alt.clone(),
        })
        .collect::<Vec<_>>()
        .join("")
}

impl From<&CellValue> for MachineCell {
    fn from(value: &CellValue) -> Self {
        match value {
            CellValue::Text(t) => Self::Text(t.clone()),
            CellValue::Integer(i) => Self::Integer(*i),
            CellValue::Float(f) => Self::Float(*f),
            CellValue::Boolean(b) => Self::Boolean(*b),
            CellValue::Null => Self::Null,
        }
    }
}

impl From<&ListItem> for MachineListItem {
    fn from(item: &ListItem) -> Self {
        Self {
            content: item.content.iter().map(|i| i.into()).collect(),
            indent_level: 0, // Flattened - nesting info lost in machine format
        }
    }
}

impl From<&ListNode> for MachineList {
    fn from(node: &ListNode) -> Self {
        Self {
            ordered: node.ordered,
            items: flatten_list_items(&node.items, 0),
        }
    }
}

// Helper to flatten nested lists
fn flatten_list_items(items: &[ListItem], level: u8) -> Vec<MachineListItem> {
    let mut result = Vec::new();
    for item in items {
        result.push(MachineListItem {
            content: item.content.iter().map(|i| i.into()).collect(),
            indent_level: level,
        });
        if let Some(nested) = &item.nested {
            result.extend(flatten_list_items(&nested.items, level + 1));
        }
    }
    result
}

// ============================================================================
// Conversion: MachineDocument -> DxmDocument
// ============================================================================

impl From<MachineDocument> for DxmDocument {
    fn from(doc: MachineDocument) -> Self {
        Self {
            meta: doc.meta.into(),
            refs: doc.refs.into_iter().collect(),
            nodes: doc.nodes.into_iter().map(|n| n.into()).collect(),
        }
    }
}

impl From<MachineMeta> for DxmMeta {
    fn from(meta: MachineMeta) -> Self {
        Self {
            version: meta.version,
            token_count: meta.token_count as usize,
            sections: meta.sections.into_iter().map(|s| s.into()).collect(),
            priorities: PriorityDistribution {
                critical: meta.critical as usize,
                important: meta.important as usize,
                low: meta.low as usize,
            },
        }
    }
}

impl From<MachineSection> for SectionInfo {
    fn from(info: MachineSection) -> Self {
        Self {
            title: info.title,
            level: info.level,
            offset: info.offset as usize,
            token_count: info.token_count as usize,
        }
    }
}

impl From<MachineNode> for DxmNode {
    fn from(node: MachineNode) -> Self {
        match node {
            MachineNode::Header {
                level,
                content,
                priority,
            } => Self::Header(HeaderNode {
                level,
                content: content.into_iter().map(|i| i.into()).collect(),
                priority: u8_to_priority(priority),
            }),
            MachineNode::Paragraph(p) => Self::Paragraph(p.into_iter().map(|i| i.into()).collect()),
            MachineNode::CodeBlock {
                language,
                content,
                priority,
            } => Self::CodeBlock(CodeBlockNode {
                language,
                content,
                priority: u8_to_priority(priority),
            }),
            MachineNode::Table { schema, rows } => Self::Table(TableNode {
                schema: schema
                    .into_iter()
                    .map(|name| ColumnDef {
                        name,
                        type_hint: None,
                    })
                    .collect(),
                rows: rows
                    .into_iter()
                    .map(|row| row.into_iter().map(|c| c.into()).collect())
                    .collect(),
            }),
            MachineNode::List { ordered, items } => Self::List(ListNode {
                ordered,
                items: items.into_iter().map(|i| i.into()).collect(),
            }),
            MachineNode::SemanticBlock {
                block_type,
                priority,
                content,
            } => Self::SemanticBlock(SemanticBlockNode {
                block_type: u8_to_semantic_type(block_type),
                priority: u8_to_priority(priority),
                content: content.into_iter().map(|i| i.into()).collect(),
            }),
            MachineNode::HorizontalRule => Self::HorizontalRule,
        }
    }
}

impl From<MachineInline> for InlineNode {
    fn from(node: MachineInline) -> Self {
        match node {
            MachineInline::Text(t) => Self::Text(t),
            MachineInline::Bold(t) => Self::Bold(vec![Self::Text(t)]),
            MachineInline::Italic(t) => Self::Italic(vec![Self::Text(t)]),
            MachineInline::Strikethrough(t) => Self::Strikethrough(vec![Self::Text(t)]),
            MachineInline::Code(c) => Self::Code(c),
            MachineInline::Reference(r) => Self::Reference(r),
            MachineInline::Link { text, url, title } => Self::Link {
                text: vec![Self::Text(text)],
                url,
                title,
            },
            MachineInline::Image { alt, url, title } => Self::Image { alt, url, title },
        }
    }
}

impl From<MachineCell> for CellValue {
    fn from(value: MachineCell) -> Self {
        match value {
            MachineCell::Text(t) => Self::Text(t),
            MachineCell::Integer(i) => Self::Integer(i),
            MachineCell::Float(f) => Self::Float(f),
            MachineCell::Boolean(b) => Self::Boolean(b),
            MachineCell::Null => Self::Null,
        }
    }
}

impl From<MachineListItem> for ListItem {
    fn from(item: MachineListItem) -> Self {
        Self {
            content: item.content.into_iter().map(|i| i.into()).collect(),
            nested: None, // Nesting info lost in machine format
        }
    }
}

impl From<MachineList> for ListNode {
    fn from(node: MachineList) -> Self {
        Self {
            ordered: node.ordered,
            items: node.items.into_iter().map(|i| i.into()).collect(),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn priority_to_u8(priority: &Option<Priority>) -> u8 {
    match priority {
        None => 0,
        Some(Priority::Low) => 1,
        Some(Priority::Important) => 2,
        Some(Priority::Critical) => 3,
    }
}

fn u8_to_priority(value: u8) -> Option<Priority> {
    match value {
        0 => None,
        1 => Some(Priority::Low),
        2 => Some(Priority::Important),
        3 => Some(Priority::Critical),
        _ => None,
    }
}

fn semantic_type_to_u8(t: &SemanticBlockType) -> u8 {
    match t {
        SemanticBlockType::Warning => 0,
        SemanticBlockType::FAQ => 1,
        SemanticBlockType::Quote => 2,
        SemanticBlockType::Info => 3,
        SemanticBlockType::Example => 4,
    }
}

fn u8_to_semantic_type(value: u8) -> SemanticBlockType {
    match value {
        0 => SemanticBlockType::Warning,
        1 => SemanticBlockType::FAQ,
        2 => SemanticBlockType::Quote,
        3 => SemanticBlockType::Info,
        4 => SemanticBlockType::Example,
        _ => SemanticBlockType::Info,
    }
}

// ============================================================================
// Serialization API (Pure RKYV)
// ============================================================================

/// Serialize a DxmDocument to machine format (RKYV).
///
/// DX-Markdown Machine Format IS RKYV - this is a zero-overhead wrapper
/// around `rkyv::to_bytes()` with `#[inline(always)]` for optimal performance.
#[inline(always)]
pub fn serialize_machine(doc: &DxmDocument) -> BinaryResult<Vec<u8>> {
    let machine_doc: MachineDocument = doc.into();

    // Estimate size to prevent memory issues
    let estimated_size = estimate_doc_size(&machine_doc);
    if estimated_size > 100_000_000 {
        // 100MB limit
        return Err(BinaryError::InvalidFormat(format!(
            "Document too large for RKYV serialization: ~{} bytes",
            estimated_size
        )));
    }

    rkyv::to_bytes::<rkyv::rancor::Error>(&machine_doc)
        .map(|bytes| bytes.into_vec())
        .map_err(|e| BinaryError::InvalidFormat(format!("RKYV serialization failed: {}", e)))
}

fn estimate_doc_size(doc: &MachineDocument) -> usize {
    // Rough estimate: count strings and nodes
    let mut size = 1000; // base overhead
    size += doc.refs.len() * 100;
    size += doc.nodes.len() * 500;
    size += doc.meta.sections.len() * 200;
    size
}

/// Deserialize a DxmDocument from machine format (RKYV).
///
/// DX-Markdown Machine Format IS RKYV - this is a zero-overhead wrapper
/// around `rkyv::from_bytes()` with `#[inline(always)]` for optimal performance.
#[inline(always)]
pub fn deserialize_machine(bytes: &[u8]) -> BinaryResult<DxmDocument> {
    let machine_doc: MachineDocument =
        rkyv::from_bytes::<MachineDocument, rkyv::rancor::Error>(bytes).map_err(|e| {
            BinaryError::InvalidFormat(format!("RKYV deserialization failed: {}", e))
        })?;
    Ok(machine_doc.into())
}
