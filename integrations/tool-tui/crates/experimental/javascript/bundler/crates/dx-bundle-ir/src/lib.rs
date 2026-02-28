//! Binary Intermediate Representation
//!
//! Skip text entirely - transform IR to IR for maximum speed

use bytemuck::{Pod, Zeroable};
use dx_bundle_core::StringIdx;

/// Binary IR header
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BinaryIRHeader {
    /// Magic bytes "DXIR"
    pub magic: [u8; 4],
    /// Version
    pub version: u32,
    /// Nodes offset
    pub nodes_offset: u64,
    /// Strings offset
    pub strings_offset: u64,
    /// Node count
    pub node_count: u32,
    /// Padding for alignment
    _pad: u32,
}

impl BinaryIRHeader {
    pub const MAGIC: [u8; 4] = *b"DXIR";

    pub fn new(node_count: u32, nodes_offset: u64, strings_offset: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            version: 1,
            nodes_offset,
            strings_offset,
            node_count,
            _pad: 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == Self::MAGIC
    }
}

/// IR Node types (single byte for cache efficiency)
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IRNodeKind {
    // Module structure
    Module = 0,
    Import = 1,
    Export = 2,
    ExportDefault = 3,

    // Declarations
    VarDecl = 10,
    FuncDecl = 11,
    ClassDecl = 12,

    // Expressions
    CallExpr = 20,
    MemberExpr = 21,
    BinaryExpr = 22,
    UnaryExpr = 23,
    Literal = 24,
    Identifier = 25,

    // JSX (pre-transformed to calls)
    JsxElement = 30,
    JsxFragment = 31,

    // Statements
    ReturnStmt = 40,
    IfStmt = 41,
    ForStmt = 42,
    WhileStmt = 43,
    BlockStmt = 44,

    // Special
    Deleted = 255,
}

/// Fixed-size IR node (32 bytes, cache-friendly)
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct IRNode {
    /// Node kind
    pub kind: u8,
    /// Flags (is_async, is_export, etc.)
    pub flags: u8,
    /// Reserved
    _pad: u16,
    /// Parent node index
    pub parent: u32,
    /// First child index
    pub first_child: u32,
    /// Next sibling index
    pub next_sibling: u32,
    /// Data (kind-specific, inline for small values)
    pub data: [u8; 16],
}

impl IRNode {
    /// Create new node
    pub fn new(kind: IRNodeKind) -> Self {
        Self {
            kind: kind as u8,
            flags: 0,
            _pad: 0,
            parent: 0,
            first_child: 0,
            next_sibling: 0,
            data: [0; 16],
        }
    }

    /// Check if node is deleted
    pub fn is_deleted(&self) -> bool {
        self.kind == IRNodeKind::Deleted as u8
    }

    /// Mark node as deleted
    pub fn delete(&mut self) {
        self.kind = IRNodeKind::Deleted as u8;
    }

    /// Get string index from data
    pub fn get_string_idx(&self) -> StringIdx {
        u32::from_le_bytes(self.data[0..4].try_into().unwrap())
    }

    /// Set string index in data
    pub fn set_string_idx(&mut self, idx: StringIdx) {
        self.data[0..4].copy_from_slice(&idx.to_le_bytes());
    }
}

/// Node flags
pub mod node_flags {
    pub const IS_ASYNC: u8 = 1 << 0;
    pub const IS_EXPORT: u8 = 1 << 1;
    pub const IS_DEFAULT: u8 = 1 << 2;
    pub const IS_CONST: u8 = 1 << 3;
    pub const HAS_CHILDREN: u8 = 1 << 4;
}

/// IR transformer - works directly on binary IR
pub struct IRTransformer<'a> {
    /// All IR nodes
    nodes: &'a mut [IRNode],
    /// String table
    strings: &'a [u8],
}

impl<'a> IRTransformer<'a> {
    /// Create new IR transformer
    pub fn new(nodes: &'a mut [IRNode], strings: &'a [u8]) -> Self {
        Self { nodes, strings }
    }

    /// Transform imports to requires (in-place!)
    pub fn transform_imports(&mut self) {
        for i in 0..self.nodes.len() {
            if self.nodes[i].kind == IRNodeKind::Import as u8 && !self.nodes[i].is_deleted() {
                // Transform import to variable declaration + require call
                self.nodes[i].kind = IRNodeKind::VarDecl as u8;

                // Create require call node (reuse child nodes)
                let first_child = self.nodes[i].first_child as usize;
                if first_child < self.nodes.len() && first_child > 0 {
                    self.nodes[first_child].kind = IRNodeKind::CallExpr as u8;
                    // Callee would be "require" string index
                }
            }
        }
    }

    /// Strip TypeScript types (just mark nodes as deleted)
    pub fn strip_typescript(&mut self) {
        for node in self.nodes.iter_mut() {
            // Check for TypeScript-specific nodes via flags or kind
            if node.flags & 0x80 != 0 {
                // TS_TYPE_FLAG
                node.delete();
            }
        }
    }

    /// Transform JSX to createElement calls (in-place!)
    pub fn transform_jsx(&mut self) {
        for i in 0..self.nodes.len() {
            if self.nodes[i].kind == IRNodeKind::JsxElement as u8 && !self.nodes[i].is_deleted() {
                // Transform to React.createElement call
                self.nodes[i].kind = IRNodeKind::CallExpr as u8;
                // Set callee string index to "React.createElement"
            }
        }
    }

    /// Emit JavaScript from IR
    pub fn emit(&self, output: &mut Vec<u8>) {
        if !self.nodes.is_empty() {
            self.emit_node(0, output);
        }
    }

    fn emit_node(&self, idx: usize, output: &mut Vec<u8>) {
        if idx >= self.nodes.len() {
            return;
        }

        let node = &self.nodes[idx];

        if node.is_deleted() {
            return;
        }

        match node.kind {
            k if k == IRNodeKind::VarDecl as u8 => {
                output.extend_from_slice(b"var ");
                self.emit_children(idx, output);
            }
            k if k == IRNodeKind::CallExpr as u8 => {
                // Emit callee
                let str_idx = node.get_string_idx();
                if let Some(callee) = self.get_string(str_idx) {
                    output.extend_from_slice(callee);
                }
                output.push(b'(');

                // Emit arguments
                let mut first = true;
                let mut child_idx = node.first_child as usize;
                while child_idx != 0 && child_idx < self.nodes.len() {
                    if !first {
                        output.extend_from_slice(b", ");
                    }
                    first = false;
                    self.emit_node(child_idx, output);
                    child_idx = self.nodes[child_idx].next_sibling as usize;
                }

                output.push(b')');
            }
            k if k == IRNodeKind::Literal as u8 => {
                let str_idx = node.get_string_idx();
                if let Some(value) = self.get_string(str_idx) {
                    output.extend_from_slice(value);
                }
            }
            k if k == IRNodeKind::Identifier as u8 => {
                let str_idx = node.get_string_idx();
                if let Some(name) = self.get_string(str_idx) {
                    output.extend_from_slice(name);
                }
            }
            _ => {
                // Generic node - emit children
                self.emit_children(idx, output);
            }
        }
    }

    fn emit_children(&self, idx: usize, output: &mut Vec<u8>) {
        if idx >= self.nodes.len() {
            return;
        }

        let mut child_idx = self.nodes[idx].first_child as usize;
        while child_idx != 0 && child_idx < self.nodes.len() {
            self.emit_node(child_idx, output);
            child_idx = self.nodes[child_idx].next_sibling as usize;
        }
    }

    fn get_string(&self, idx: StringIdx) -> Option<&[u8]> {
        // Simple lookup - in real implementation would use string table structure
        if idx == 0 || idx as usize > self.strings.len() {
            None
        } else {
            Some(&self.strings[..idx as usize])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_node() {
        let mut node = IRNode::new(IRNodeKind::Import);
        assert_eq!(node.kind, IRNodeKind::Import as u8);
        assert!(!node.is_deleted());

        node.delete();
        assert!(node.is_deleted());
    }

    #[test]
    fn test_ir_header() {
        let header = BinaryIRHeader::new(100, 64, 1024);
        assert!(header.is_valid());
        assert_eq!(header.node_count, 100);
    }
}
