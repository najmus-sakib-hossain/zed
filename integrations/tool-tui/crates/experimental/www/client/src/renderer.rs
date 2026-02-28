//! Renderer: Process HTIP stream and apply to DOM
//!
//! This is the core execution engine.

use core::ptr;
use dx_packet::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Node};

use crate::node_registry::NodeRegistry;
use crate::string_table::StringTableReader;
use crate::template_cache::TemplateCache;

/// Main renderer
pub struct Renderer {
    template_cache: TemplateCache,
    node_registry: NodeRegistry,
    document: Document,
    root: Option<Element>,
}

impl Renderer {
    /// Create new renderer
    pub fn new() -> Result<Self, u8> {
        let window = web_sys::window().ok_or(1u8)?;
        let document = window.document().ok_or(2u8)?;

        Ok(Self {
            template_cache: TemplateCache::new()?,
            node_registry: NodeRegistry::new(),
            document,
            root: None,
        })
    }

    /// Set root element for rendering
    pub fn set_root(&mut self, selector: &str) -> Result<(), u8> {
        self.root = self
            .document
            .query_selector(selector)
            .ok()
            .flatten()
            .map(|e| e.dyn_into::<Element>().ok())
            .flatten();

        if self.root.is_some() {
            Ok(())
        } else {
            Err(6u8) // NodeNotFound
        }
    }

    /// Process HTIP stream
    pub fn process_stream(&mut self, data: &[u8], header: &HtipHeader) -> Result<(), u8> {
        let mut offset = HtipHeader::SIZE;

        // Parse string table if present
        let strings = if header.string_count > 0 {
            Some(StringTableReader::new(data, offset, header.string_count))
        } else {
            None
        };

        // Skip past string table
        if let Some(ref s) = strings {
            // Calculate string table size: entries + data
            offset += (header.string_count as usize) * StringEntry::SIZE;
            // Note: string data size would need to be in header or calculated
        }

        // Parse templates if present
        if header.template_count > 0 {
            for _ in 0..header.template_count {
                if offset + TemplateEntry::SIZE > data.len() {
                    return Err(ErrorCode::BufferTooSmall as u8);
                }

                let entry = unsafe {
                    ptr::read_unaligned(data.as_ptr().add(offset) as *const TemplateEntry)
                };
                offset += TemplateEntry::SIZE;

                // Get HTML from string table
                if let Some(ref strings) = strings {
                    if let Some(html) = strings.get(entry.html_string_idx) {
                        self.template_cache.register(entry.id, html)?;
                    }
                }
            }
        }

        // Process opcodes
        for _ in 0..header.opcode_count {
            if offset + OpcodeHeader::SIZE > data.len() {
                return Err(ErrorCode::BufferTooSmall as u8);
            }

            let op_header =
                unsafe { ptr::read_unaligned(data.as_ptr().add(offset) as *const OpcodeHeader) };
            offset += OpcodeHeader::SIZE;

            // Execute opcode
            offset = self.execute_op(&op_header, data, offset, &strings)?;
        }

        Ok(())
    }

    /// Execute a single opcode
    fn execute_op(
        &mut self,
        header: &OpcodeHeader,
        data: &[u8],
        mut offset: usize,
        strings: &Option<StringTableReader>,
    ) -> Result<usize, u8> {
        let op_type = OpType::from_u8(header.op_type).ok_or(ErrorCode::InvalidOpcode as u8)?;

        match op_type {
            OpType::Clone => {
                let payload = self.read_payload::<ClonePayload>(data, &mut offset)?;
                self.execute_clone(header.target_id, &payload)?;
            }
            OpType::PatchText => {
                let payload = self.read_payload::<PatchTextPayload>(data, &mut offset)?;
                if let Some(ref s) = strings {
                    self.execute_patch_text(header.target_id, &payload, s)?;
                }
            }
            OpType::PatchAttr => {
                let payload = self.read_payload::<PatchAttrPayload>(data, &mut offset)?;
                if let Some(ref s) = strings {
                    self.execute_patch_attr(header.target_id, &payload, s)?;
                }
            }
            OpType::ClassToggle => {
                let payload = self.read_payload::<ClassTogglePayload>(data, &mut offset)?;
                if let Some(ref s) = strings {
                    self.execute_class_toggle(header.target_id, &payload, s)?;
                }
            }
            OpType::Remove => {
                self.execute_remove(header.target_id)?;
            }
            OpType::SetStyle => {
                let payload = self.read_payload::<SetStylePayload>(data, &mut offset)?;
                if let Some(ref s) = strings {
                    self.execute_set_style(header.target_id, &payload, s)?;
                }
            }
            OpType::BatchStart | OpType::BatchCommit => {
                // Batch markers are no-ops in this implementation
                // Future: could defer DOM writes until commit
            }
        }

        Ok(offset)
    }

    /// Read a payload struct from data
    fn read_payload<T: Copy>(&self, data: &[u8], offset: &mut usize) -> Result<T, u8> {
        let size = core::mem::size_of::<T>();
        if *offset + size > data.len() {
            return Err(ErrorCode::BufferTooSmall as u8);
        }

        let payload = unsafe { ptr::read_unaligned(data.as_ptr().add(*offset) as *const T) };
        *offset += size;

        Ok(payload)
    }

    // ========================================================================
    // Opcode Executors
    // ========================================================================

    fn execute_clone(&mut self, _target_id: u16, payload: &ClonePayload) -> Result<(), u8> {
        let cloned = self.template_cache.clone_template(payload.template_id)?;

        // Register the cloned node
        let _node_id = self.node_registry.register(cloned.clone());

        // Append to parent
        if payload.parent_id == 0 {
            // Append to root
            if let Some(ref root) = self.root {
                let _ = root.append_child(&cloned);
            }
        } else if let Some(parent) = self.node_registry.get(payload.parent_id) {
            let _ = parent.append_child(&cloned);
        }

        Ok(())
    }

    fn execute_patch_text(
        &mut self,
        target_id: u16,
        payload: &PatchTextPayload,
        strings: &StringTableReader,
    ) -> Result<(), u8> {
        let text =
            strings.get(payload.string_idx).ok_or(ErrorCode::StringIndexOutOfBounds as u8)?;

        if let Some(node) = self.node_registry.get(target_id) {
            node.set_text_content(Some(text));
        }

        Ok(())
    }

    fn execute_patch_attr(
        &mut self,
        target_id: u16,
        payload: &PatchAttrPayload,
        strings: &StringTableReader,
    ) -> Result<(), u8> {
        let name = strings
            .get(payload.attr_name_idx)
            .ok_or(ErrorCode::StringIndexOutOfBounds as u8)?;
        let value = strings
            .get(payload.attr_value_idx)
            .ok_or(ErrorCode::StringIndexOutOfBounds as u8)?;

        if let Some(node) = self.node_registry.get(target_id) {
            if let Some(elem) = node.dyn_ref::<Element>() {
                let _ = elem.set_attribute(name, value);
            }
        }

        Ok(())
    }

    // ========================================================================
    // INLINE JS HELPERS (Zero-Overhead)
    // ========================================================================

    fn execute_class_toggle(
        &mut self,
        target_id: u16,
        payload: &ClassTogglePayload,
        strings: &StringTableReader,
    ) -> Result<(), u8> {
        let class_name = strings
            .get(payload.class_name_idx)
            .ok_or(ErrorCode::StringIndexOutOfBounds as u8)?;

        if let Some(node) = self.node_registry.get(target_id) {
            // Use inline JS helper for minimal WASM size
            toggle_class(node, class_name, payload.enable != 0);
        }

        Ok(())
    }

    fn execute_remove(&mut self, target_id: u16) -> Result<(), u8> {
        if let Some(node) = self.node_registry.remove(target_id) {
            if let Some(parent) = node.parent_node() {
                let _ = parent.remove_child(&node);
            }
        }
        Ok(())
    }

    fn execute_set_style(
        &mut self,
        target_id: u16,
        payload: &SetStylePayload,
        strings: &StringTableReader,
    ) -> Result<(), u8> {
        let prop = strings
            .get(payload.prop_name_idx)
            .ok_or(ErrorCode::StringIndexOutOfBounds as u8)?;
        let value = strings
            .get(payload.prop_value_idx)
            .ok_or(ErrorCode::StringIndexOutOfBounds as u8)?;

        if let Some(node) = self.node_registry.get(target_id) {
            // Use inline JS helper
            set_style(node, prop, value);
        }

        Ok(())
    }

    /// Get node count
    pub fn node_count(&self) -> u32 {
        self.node_registry.count()
    }
}

// Inline JS snippets - fastest and smallest way to touch DOM
#[wasm_bindgen(inline_js = "
    export function toggle_class(node, name, enable) {
        if (node instanceof Element) {
            if (enable) node.classList.add(name);
            else node.classList.remove(name);
        }
    }
    export function set_style(node, prop, val) {
        if (node instanceof HTMLElement) {
            node.style.setProperty(prop, val);
        }
    }
")]
extern "C" {
    fn toggle_class(node: &Node, name: &str, enable: bool);
    fn set_style(node: &Node, prop: &str, val: &str);
}
