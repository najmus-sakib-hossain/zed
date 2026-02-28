//! # Codegen Module - HTIP Binary Generator
//!
//! Instead of generating Rust code and compiling to WASM (418KB overhead),
//! this module directly emits HTIP binary opcodes that the dx-client runtime
//! interprets. Result: ~1KB of data instead of 418KB of WASM.
//!
//! ## Architecture Change (Dec 11)
//! OLD: Parse -> Generate Rust -> Compile WASM -> Pack (418KB)
//! NEW: Parse -> Generate Opcodes -> Pack (1KB)
//!
//! The dx-client WASM (22KB) is the ONLY WASM. Apps are pure data.

use anyhow::Result;
use std::collections::HashMap;

use crate::splitter::{Binding, StateSchema, Template};

/// HTIP Header (matches dx_packet::HtipHeader)
const MAGIC: u16 = 0x4458; // "DX"
const VERSION: u8 = 2;

/// String interner for efficient string deduplication
struct StringInterner {
    strings: Vec<String>,
    index: HashMap<String, u16>,
}

impl StringInterner {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u16 {
        if let Some(&idx) = self.index.get(s) {
            return idx;
        }
        let idx = self.strings.len() as u16;
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), idx);
        idx
    }

    fn into_strings(self) -> Vec<String> {
        self.strings
    }
}

/// Generate HTIP binary stream from templates and bindings
///
/// Returns: (htip_stream: Vec<u8>, string_table: Vec<String>)
pub fn generate_htip(
    templates: &[Template],
    bindings: &[Binding],
    _schemas: &[StateSchema],
    verbose: bool,
) -> Result<(Vec<u8>, Vec<String>)> {
    if verbose {
        println!("  Generating HTIP binary stream...");
    }

    let mut interner = StringInterner::new();

    // Build opcodes
    let mut opcodes: Vec<u8> = Vec::new();
    let mut opcode_count: u32 = 0;

    // For each template, emit a Clone opcode (initial render)
    for template in templates {
        let new_node_id = template.id as u16 + 1;
        opcodes.push(1); // op_type = Clone
        opcodes.push(0); // reserved
        opcodes.extend(&new_node_id.to_le_bytes());
        opcodes.extend(&(template.id as u16).to_le_bytes());
        opcodes.extend(&0u16.to_le_bytes()); // parent_id = root
        opcode_count += 1;
    }

    // For each binding, emit a PatchText opcode
    for binding in bindings {
        let text = format!("{{{}}}", binding.expression);
        let string_idx = interner.intern(&text);

        let target_id = binding.slot_id as u16 + 1;
        opcodes.push(2); // op_type = PatchText
        opcodes.push(0); // reserved
        opcodes.extend(&target_id.to_le_bytes());
        opcodes.extend(&string_idx.to_le_bytes());
        opcodes.extend(&0u16.to_le_bytes());
        opcode_count += 1;
    }

    // Build template dictionary (intern HTML into string table)
    let mut template_entries: Vec<u8> = Vec::new();
    for template in templates {
        let html_idx = interner.intern(&template.html);
        template_entries.extend(&(template.id as u16).to_le_bytes());
        template_entries.extend(&html_idx.to_le_bytes());
        template_entries.push(template.slots.len() as u8);
        template_entries.extend(&[0u8; 3]);
    }

    // Build string table binary
    let string_table = interner.into_strings();
    let mut string_entries: Vec<u8> = Vec::new();
    let mut string_data: Vec<u8> = Vec::new();

    for s in &string_table {
        let offset = string_data.len() as u32;
        let len = s.len() as u16;
        string_entries.extend(&offset.to_le_bytes());
        string_entries.extend(&len.to_le_bytes());
        string_entries.extend(&0u16.to_le_bytes());
        string_data.extend(s.as_bytes());
    }

    // Calculate payload size
    let payload_size =
        string_entries.len() + string_data.len() + template_entries.len() + opcodes.len();

    // Build header
    let mut stream = Vec::new();
    stream.extend(&MAGIC.to_le_bytes());
    stream.push(VERSION);
    stream.push(0x03); // flags: has_strings | has_templates
    stream.extend(&(templates.len() as u16).to_le_bytes());
    stream.extend(&(string_table.len() as u16).to_le_bytes());
    stream.extend(&opcode_count.to_le_bytes());
    stream.extend(&(payload_size as u32).to_le_bytes());

    // Append sections
    stream.extend(&string_entries);
    stream.extend(&string_data);
    stream.extend(&template_entries);
    stream.extend(&opcodes);

    if verbose {
        println!("    HTIP stream size: {} bytes", stream.len());
        println!("    String table: {} entries", string_table.len());
        println!("    Templates: {} entries", templates.len());
        println!("    Opcodes: {} entries", opcode_count);
    }

    Ok((stream, string_table))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::splitter::{SlotDef, SlotType};

    #[test]
    fn test_htip_generation() {
        let templates = vec![Template {
            id: 0,
            html: "<div>Hello</div>".to_string(),
            slots: vec![SlotDef {
                slot_id: 0,
                slot_type: SlotType::Text,
                path: vec![0],
            }],
            hash: "test".to_string(),
        }];

        let bindings = vec![Binding {
            slot_id: 0,
            component: "Test".to_string(),
            expression: "self.count".to_string(),
            dirty_bit: 0,
            flag: crate::splitter::BindingFlag::Normal,
            key_expression: None,
        }];

        let schemas = vec![];

        let (stream, strings) = generate_htip(&templates, &bindings, &schemas, false).unwrap();

        assert_eq!(&stream[0..2], &[0x58, 0x44]); // "DX" little-endian
        assert_eq!(stream[2], 2); // version
        assert!(!strings.is_empty());
        assert!(stream.len() < 500, "HTIP stream should be tiny, got {} bytes", stream.len());
    }
}
