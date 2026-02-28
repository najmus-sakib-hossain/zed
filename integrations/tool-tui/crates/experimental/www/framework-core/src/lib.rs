//! # dx-core: Memory Layout & Capability Security
//!
//! The Core Memory Manager for dx-www runtime.
//! Implements a Linear Memory Layout with three regions:
//! 1. Static Region: Read-Only dictionaries (Template Strings, Class Names)
//! 2. State Region: `SharedArrayBuffer` for Component State
//! 3. Queue Region: Ring Buffer for Render Opcodes
//!
//! **ACID TEST COMPLIANCE:**
//! - Zero-Copy Memory: All data accessed via byte slices
//! - No String allocations in hot paths
//! - `SharedArrayBuffer` ready for Worker threads

// This crate intentionally uses unsafe code for low-level memory management
// in a WASM environment. The unsafe code is carefully documented with safety
// invariants and is necessary for zero-copy memory access.
#![allow(unsafe_code)]
#![allow(clippy::missing_const_for_fn)] // Many functions can't be const due to pointer operations
#![allow(clippy::must_use_candidate)] // Builder pattern functions don't need must_use
#![allow(clippy::cast_possible_truncation)] // Intentional for u32 sizes in WASM
#![allow(clippy::cast_ptr_alignment)] // Intentional for memory layout
#![allow(clippy::doc_markdown)] // Allow technical terms without backticks
#![allow(clippy::bool_to_int_with_if)] // Clearer than u32::from(bool)

use bytemuck::{Pod, Zeroable};
use std::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// PUBLIC MODULES
// ============================================================================

pub mod icon;

// ============================================================================
// MEMORY LAYOUT CONSTANTS
// ============================================================================

/// Total size of the Linear Memory (16MB for now)
pub const MEMORY_SIZE: usize = 16 * 1024 * 1024;

/// Static Region: 0 - 2MB (Read-Only Data)
pub const STATIC_REGION_START: usize = 0;
/// Size of the Static Region (2MB)
pub const STATIC_REGION_SIZE: usize = 2 * 1024 * 1024;

/// State Region: 2MB - 10MB (Component State)
pub const STATE_REGION_START: usize = STATIC_REGION_SIZE;
/// Size of the State Region (8MB)
pub const STATE_REGION_SIZE: usize = 8 * 1024 * 1024;

/// Queue Region: 10MB - 16MB (Render Opcodes Ring Buffer)
pub const QUEUE_REGION_START: usize = STATE_REGION_START + STATE_REGION_SIZE;
/// Size of the Queue Region (6MB)
pub const QUEUE_REGION_SIZE: usize = 6 * 1024 * 1024;

// ============================================================================
// CAPABILITY MANIFEST (Security Layer)
// ============================================================================

/// Capability flags for security checks
#[repr(transparent)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CapabilityFlags(pub u64);

impl CapabilityFlags {
    /// DOM write capability flag
    pub const DOM_WRITE: u64 = 1 << 0;
    /// DOM read capability flag
    pub const DOM_READ: u64 = 1 << 1;
    /// Network fetch capability flag
    pub const NETWORK_FETCH: u64 = 1 << 2;
    /// Local storage capability flag
    pub const LOCAL_STORAGE: u64 = 1 << 3;
    /// Worker spawn capability flag
    pub const WORKER_SPAWN: u64 = 1 << 4;

    /// Check if a capability flag is set
    pub fn has_capability(&self, flag: u64) -> bool {
        self.0 & flag != 0
    }
}

/// The Capability Manifest header (first 64 bytes of Static Region)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct CapabilityManifest {
    /// Magic number for validation: 0x4458_5757 ("DXWW")
    pub magic: u32,
    /// Version of the binary format
    pub version: u32,
    /// Capability flags
    pub capabilities: CapabilityFlags,
    /// Checksum of the entire binary (CRC32)
    pub checksum: u32,
    /// Reserved for future use
    pub reserved: [u32; 11],
}

impl CapabilityManifest {
    /// Magic number for validation: 0x4458_5757 ("DXWW")
    pub const MAGIC: u32 = 0x4458_5757; // "DXWW"

    /// Validate the capability manifest
    ///
    /// # Errors
    /// Returns an error if the magic number is invalid or the binary is corrupted.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.magic != Self::MAGIC {
            return Err("Invalid magic number - binary corrupted");
        }
        // CRC32 checksum validation is handled at the binary level
        Ok(())
    }
}

// ============================================================================
// MEMORY MANAGER
// ============================================================================

/// Global Memory Manager (Single Instance)
pub struct MemoryManager {
    /// Pointer to the start of Linear Memory
    base_ptr: *mut u8,
    /// Size of the allocated memory (reserved for future bounds checking)
    #[allow(dead_code)]
    size: usize,
    /// Current write offset in State Region (atomic for thread safety)
    state_offset: AtomicU32,
    /// Current write offset in Queue Region (reserved for future multi-threaded queue)
    #[allow(dead_code)]
    queue_offset: AtomicU32,
}

// SAFETY: We're targeting wasm32 which is single-threaded by default.
// SharedArrayBuffer support will require additional synchronization.
unsafe impl Send for MemoryManager {}

// SAFETY: We're targeting wasm32 which is single-threaded by default.
// SharedArrayBuffer support will require additional synchronization.
unsafe impl Sync for MemoryManager {}

impl MemoryManager {
    /// Initialize the Memory Manager with a pre-allocated buffer
    ///
    /// # Safety
    /// The caller must ensure that `base_ptr` points to valid memory
    /// of at least `MEMORY_SIZE` bytes.
    pub unsafe fn new(base_ptr: *mut u8) -> Self {
        Self {
            base_ptr,
            size: MEMORY_SIZE,
            state_offset: AtomicU32::new(0),
            queue_offset: AtomicU32::new(0),
        }
    }

    /// Get the Capability Manifest from Static Region
    ///
    /// # Errors
    /// Returns an error if the manifest validation fails (invalid magic number).
    pub fn get_manifest(&self) -> Result<&CapabilityManifest, &'static str> {
        // SAFETY:
        // - base_ptr is valid and points to at least MEMORY_SIZE bytes (invariant from new())
        // - CapabilityManifest is repr(C) and Pod, so any bit pattern is valid
        // - The manifest is at offset 0, which is within the Static Region
        // - We validate the manifest contents before returning
        unsafe {
            let manifest_ptr = self.base_ptr as *const CapabilityManifest;
            let manifest = &*manifest_ptr;
            manifest.validate()?;
            Ok(manifest)
        }
    }

    /// Get a slice to the Static Region
    pub fn static_region(&self) -> &[u8] {
        // SAFETY:
        // - base_ptr is valid and points to at least MEMORY_SIZE bytes (invariant from new())
        // - STATIC_REGION_START (0) + STATIC_REGION_SIZE (2MB) <= MEMORY_SIZE (16MB)
        // - The returned slice lifetime is tied to &self, preventing use-after-free
        unsafe {
            std::slice::from_raw_parts(self.base_ptr.add(STATIC_REGION_START), STATIC_REGION_SIZE)
        }
    }

    /// Get a mutable slice to the State Region
    pub fn state_region_mut(&mut self) -> &mut [u8] {
        // SAFETY:
        // - base_ptr is valid and points to at least MEMORY_SIZE bytes (invariant from new())
        // - STATE_REGION_START (2MB) + STATE_REGION_SIZE (8MB) <= MEMORY_SIZE (16MB)
        // - &mut self ensures exclusive access, preventing data races
        // - The returned slice lifetime is tied to &mut self
        unsafe {
            std::slice::from_raw_parts_mut(self.base_ptr.add(STATE_REGION_START), STATE_REGION_SIZE)
        }
    }

    /// Get a mutable slice to the Queue Region
    pub fn queue_region_mut(&mut self) -> &mut [u8] {
        // SAFETY:
        // - base_ptr is valid and points to at least MEMORY_SIZE bytes (invariant from new())
        // - QUEUE_REGION_START (10MB) + QUEUE_REGION_SIZE (6MB) = MEMORY_SIZE (16MB)
        // - &mut self ensures exclusive access, preventing data races
        // - The returned slice lifetime is tied to &mut self
        unsafe {
            std::slice::from_raw_parts_mut(self.base_ptr.add(QUEUE_REGION_START), QUEUE_REGION_SIZE)
        }
    }

    /// Allocate space in State Region (returns offset)
    ///
    /// # Errors
    /// Returns an error if the allocation would overflow the State Region.
    pub fn alloc_state(&self, size: u32) -> Result<u32, &'static str> {
        let offset = self.state_offset.fetch_add(size, Ordering::SeqCst);
        if offset + size > STATE_REGION_SIZE as u32 {
            return Err("State Region overflow");
        }
        Ok(offset)
    }

    /// Write bytes to State Region at given offset
    ///
    /// # Safety
    /// Caller must ensure offset + data.len() is within bounds
    pub unsafe fn write_state(&mut self, offset: u32, data: &[u8]) {
        // SAFETY:
        // - Caller guarantees offset + data.len() <= STATE_REGION_SIZE (precondition)
        // - base_ptr is valid and points to at least MEMORY_SIZE bytes (invariant from new())
        // - STATE_REGION_START + offset is within the State Region
        // - &mut self ensures exclusive access, preventing data races
        // - copy_nonoverlapping is safe because source (data) and dest don't overlap
        unsafe {
            let dest = self.base_ptr.add(STATE_REGION_START + offset as usize);
            std::ptr::copy_nonoverlapping(data.as_ptr(), dest, data.len());
        }
    }

    /// Read bytes from State Region at given offset
    ///
    /// # Safety
    /// Caller must ensure offset + len is within bounds
    pub unsafe fn read_state(&self, offset: u32, len: usize) -> &[u8] {
        // SAFETY:
        // - Caller guarantees offset + len <= STATE_REGION_SIZE (precondition)
        // - base_ptr is valid and points to at least MEMORY_SIZE bytes (invariant from new())
        // - STATE_REGION_START + offset is within the State Region
        // - The returned slice lifetime is tied to &self, preventing use-after-free
        unsafe {
            let src = self.base_ptr.add(STATE_REGION_START + offset as usize);
            std::slice::from_raw_parts(src, len)
        }
    }
}

// ============================================================================
// STATIC DICTIONARY STRUCTURES
// ============================================================================

/// A static string stored in the Static Region (u32 offset + u32 length)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct StaticString {
    /// Offset into Static Region
    pub offset: u32,
    /// Length in bytes
    pub len: u32,
}

/// Dictionary of class names (for "No String" rule compliance)
#[repr(C)]
pub struct ClassNameDictionary {
    /// Number of entries
    pub count: u32,
    /// Array of StaticString entries
    pub entries: [StaticString; 256], // Max 256 unique class names
}

impl ClassNameDictionary {
    /// Look up a class name by ID
    pub fn get<'a>(&self, id: u32, static_region: &'a [u8]) -> Option<&'a [u8]> {
        if id >= self.count {
            return None;
        }
        let entry = self.entries[id as usize];
        let start = entry.offset as usize;
        let end = start + entry.len as usize;
        Some(&static_region[start..end])
    }
}

// ============================================================================
// RENDER OPCODE QUEUE (for dx-dom)
// ============================================================================

/// Opcode types for render operations
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// Clone template and append (TemplateID: u32, ParentID: u32)
    Clone = 1,
    /// Update text content (NodeID: u32, TextOffset: u32, TextLen: u32)
    UpdateText = 2,
    /// Update attribute (NodeID: u32, AttrID: u32, ValueOffset: u32, ValueLen: u32)
    UpdateAttr = 3,
    /// Remove node (NodeID: u32)
    Remove = 4,
    /// Toggle class on node (NodeID: u32, ClassNameID: u32, Add: u32 - 1 for add, 0 for remove)
    ClassToggle = 5,
    /// Update style property (NodeID: u32, StyleNameID: u32, ValueOffset: u32)
    Style = 6,
}

/// Render operation in the Queue
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct RenderOp {
    /// The opcode type for this operation
    pub opcode: u8,
    /// Reserved bytes for alignment
    pub reserved: [u8; 3],
    /// First argument (meaning depends on opcode)
    pub arg1: u32,
    /// Second argument (meaning depends on opcode)
    pub arg2: u32,
    /// Third argument (meaning depends on opcode)
    pub arg3: u32,
}

impl RenderOp {
    /// Create a new Clone operation to clone a template and append to parent
    ///
    /// # Arguments
    /// * `template_id` - ID of the template to clone
    /// * `parent_id` - ID of the parent node to append to
    pub fn new_clone(template_id: u32, parent_id: u32) -> Self {
        Self {
            opcode: OpCode::Clone as u8,
            reserved: [0; 3],
            arg1: template_id,
            arg2: parent_id,
            arg3: 0,
        }
    }

    /// Create a new UpdateText operation
    ///
    /// # Arguments
    /// * `node_id` - Target node ID
    /// * `text_offset` - Offset to the text in the state region
    /// * `text_len` - Length of the text in bytes
    pub fn new_update_text(node_id: u32, text_offset: u32, text_len: u32) -> Self {
        Self {
            opcode: OpCode::UpdateText as u8,
            reserved: [0; 3],
            arg1: node_id,
            arg2: text_offset,
            arg3: text_len,
        }
    }

    /// Create a new ClassToggle operation
    ///
    /// # Arguments
    /// * `node_id` - Target node ID
    /// * `class_name_id` - ID of the class name in the static dictionary
    /// * `add` - true to add the class, false to remove it
    pub fn new_class_toggle(node_id: u32, class_name_id: u32, add: bool) -> Self {
        Self {
            opcode: OpCode::ClassToggle as u8,
            reserved: [0; 3],
            arg1: node_id,
            arg2: class_name_id,
            arg3: if add { 1 } else { 0 },
        }
    }

    /// Create a new Style update operation
    ///
    /// # Arguments
    /// * `node_id` - Target node ID
    /// * `style_name_id` - ID of the style property name in the static dictionary
    /// * `value_offset` - Offset to the value in the state region
    pub fn new_style(node_id: u32, style_name_id: u32, value_offset: u32) -> Self {
        Self {
            opcode: OpCode::Style as u8,
            reserved: [0; 3],
            arg1: node_id,
            arg2: style_name_id,
            arg3: value_offset,
        }
    }
}

// ============================================================================
// EXPORTS
// ============================================================================

pub use once_cell::sync::Lazy;

/// Global Memory Manager instance (initialized once)
pub static mut MEMORY: Option<MemoryManager> = None;

/// Initialize the global memory manager
///
/// # Safety
/// Must be called exactly once at startup before any other operations.
/// The caller must ensure that `buffer_ptr` points to valid memory
/// of at least `MEMORY_SIZE` bytes that remains valid for the program lifetime.
#[cfg(target_arch = "wasm32")]
pub unsafe fn init_memory(buffer_ptr: *mut u8) {
    // SAFETY:
    // - Caller guarantees buffer_ptr points to valid memory of at least MEMORY_SIZE bytes
    // - Caller guarantees this is called exactly once (single initialization)
    // - In wasm32, this is typically called from JS with a WebAssembly.Memory buffer
    // - The static mut MEMORY is only written once here and read thereafter
    unsafe {
        MEMORY = Some(MemoryManager::new(buffer_ptr));
    }
}

#[cfg(target_arch = "wasm32")]
pub fn panic_hook() {
    console_error_panic_hook::set_once();
}
