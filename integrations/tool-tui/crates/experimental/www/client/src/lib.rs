//! # dx-client: Sub-20KB WASM Runtime (Pure FFI)
//!
//! Full-featured Macro runtime without wasm-bindgen bloat.
//! Uses pure FFI imports like dx-client-tiny but with complete HTIP support.
//!
//! ## Features
//! - Template cloning and caching
//! - Incremental DOM patching
//! - State management
//! - Event handling
//!
//! Target: < 20KB unoptimized

#![no_std]

extern crate alloc;

mod allocator;
pub mod style_loader;

// Ecosystem integration (optional features)
#[cfg(any(feature = "dev", feature = "dx-www-error", feature = "dx-www-debug"))]
pub mod ecosystem;

use core::slice;

#[global_allocator]
static ALLOC: allocator::BumpAlloc = allocator::BumpAlloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    #[cfg(target_arch = "wasm32")]
    core::arch::wasm32::unreachable();
    #[cfg(not(target_arch = "wasm32"))]
    loop {}
}

// ============================================================================
// FFI: JavaScript Host Functions
// ============================================================================

unsafe extern "C" {
    // Template operations
    fn host_clone_template(id: u32) -> u32;
    fn host_cache_template(id: u32, html_ptr: *const u8, html_len: u32);

    // DOM operations
    fn host_append(parent: u32, child: u32);
    fn host_remove(node: u32);
    fn host_set_text(node: u32, ptr: *const u8, len: u32);
    fn host_set_attr(node: u32, key_ptr: *const u8, key_len: u32, val_ptr: *const u8, val_len: u32);
    fn host_toggle_class(node: u32, class_ptr: *const u8, class_len: u32, enable: u32);

    // Events
    fn host_listen(node: u32, event_type: u32, handler_id: u32);

    // State
    #[allow(dead_code)]
    fn host_notify_state_change(slot_id: u32);

    // Delta patch operations
    /// Get cached base data for delta patching. Returns length of data written to buffer.
    /// If buffer is too small, returns required size without writing.
    fn host_get_cached_base(cache_id: u32, buf_ptr: *mut u8, buf_len: u32) -> u32;
    /// Store patched result back to cache
    fn host_store_patched(cache_id: u32, data_ptr: *const u8, data_len: u32);

    // Debug
    fn host_log(val: u32);
}

// ============================================================================
// HTIP Opcodes
// ============================================================================

const OP_CLONE: u8 = 1;
const OP_PATCH_TEXT: u8 = 2;
const OP_PATCH_ATTR: u8 = 3;
const OP_CLASS_TOGGLE: u8 = 4;
const OP_REMOVE: u8 = 5;
const OP_EVENT: u8 = 6;
#[allow(dead_code)]
const OP_STATE_UPDATE: u8 = 7;
const OP_TEMPLATE_DEF: u8 = 8;
/// Delta patch opcode - applies a binary diff to cached base data
const OP_DELTA_PATCH: u8 = 9;
const OP_EOF: u8 = 255;

// ============================================================================
// Delta Patch Constants (matching binary/src/delta.rs)
// ============================================================================

/// Magic bytes for delta format
const DELTA_MAGIC: [u8; 4] = *b"DXDL";

/// Delta instruction: Copy block from base
const DELTA_OP_COPY: u8 = 0x01;

/// Delta instruction: Insert literal bytes
const DELTA_OP_LITERAL: u8 = 0x02;

/// Delta header size (magic + version + block_size + base_hash + reserved)
const DELTA_HEADER_SIZE: usize = 16;

// ============================================================================
// State
// ============================================================================

struct Runtime {
    node_count: u32,
    template_count: u32,
}

static mut RUNTIME: Runtime = Runtime {
    node_count: 0,
    template_count: 0,
};

// ============================================================================
// WASM Exports
// ============================================================================

/// Initialize runtime
#[unsafe(no_mangle)]
pub extern "C" fn init() -> u32 {
    unsafe {
        RUNTIME.node_count = 0;
        RUNTIME.template_count = 0;
    }
    0
}

/// Render HTIP stream
#[unsafe(no_mangle)]
pub extern "C" fn render_stream(ptr: *const u8, len: u32) -> u32 {
    if ptr.is_null() || len < 4 {
        return 1;
    }

    unsafe {
        let data = slice::from_raw_parts(ptr, len as usize);
        process_htip_stream(data)
    }
}

/// Process HTIP stream bytes
unsafe fn process_htip_stream(data: &[u8]) -> u32 {
    unsafe {
        let mut offset = 4; // Skip header

        while offset < data.len() {
            let op = data[offset];
            offset += 1;

            match op {
                OP_CLONE => {
                    if offset >= data.len() {
                        break;
                    }
                    let template_id = data[offset] as u32;
                    offset += 1;

                    let node = host_clone_template(template_id);
                    host_append(0, node);
                    RUNTIME.node_count += 1;
                }

                OP_TEMPLATE_DEF => {
                    if offset + 3 >= data.len() {
                        break;
                    }
                    let id = data[offset] as u32;
                    offset += 1;
                    let len = read_u16(data, offset) as usize;
                    offset += 2;

                    if offset + len > data.len() {
                        break;
                    }
                    let html = &data[offset..offset + len];
                    offset += len;

                    host_cache_template(id, html.as_ptr(), len as u32);
                    RUNTIME.template_count += 1;
                }

                OP_PATCH_TEXT => {
                    if offset + 4 >= data.len() {
                        break;
                    }
                    let node_id = read_u16(data, offset) as u32;
                    offset += 2;
                    let text_len = read_u16(data, offset) as usize;
                    offset += 2;

                    if offset + text_len > data.len() {
                        break;
                    }
                    let text = &data[offset..offset + text_len];
                    offset += text_len;

                    host_set_text(node_id, text.as_ptr(), text_len as u32);
                }

                OP_PATCH_ATTR => {
                    if offset + 6 >= data.len() {
                        break;
                    }
                    let node_id = read_u16(data, offset) as u32;
                    offset += 2;
                    let key_len = read_u16(data, offset) as usize;
                    offset += 2;

                    if offset + key_len >= data.len() {
                        break;
                    }
                    let key = &data[offset..offset + key_len];
                    offset += key_len;

                    let val_len = read_u16(data, offset) as usize;
                    offset += 2;

                    if offset + val_len > data.len() {
                        break;
                    }
                    let val = &data[offset..offset + val_len];
                    offset += val_len;

                    host_set_attr(
                        node_id,
                        key.as_ptr(),
                        key_len as u32,
                        val.as_ptr(),
                        val_len as u32,
                    );
                }

                OP_CLASS_TOGGLE => {
                    if offset + 5 >= data.len() {
                        break;
                    }
                    let node_id = read_u16(data, offset) as u32;
                    offset += 2;
                    let class_len = read_u16(data, offset) as usize;
                    offset += 2;

                    if offset + class_len >= data.len() {
                        break;
                    }
                    let class = &data[offset..offset + class_len];
                    offset += class_len;

                    let enable = data[offset] as u32;
                    offset += 1;

                    host_toggle_class(node_id, class.as_ptr(), class_len as u32, enable);
                }

                OP_REMOVE => {
                    if offset + 2 > data.len() {
                        break;
                    }
                    let node_id = read_u16(data, offset) as u32;
                    offset += 2;

                    host_remove(node_id);
                }

                OP_EVENT => {
                    if offset + 5 > data.len() {
                        break;
                    }
                    let node_id = read_u16(data, offset) as u32;
                    offset += 2;
                    let event_type = data[offset] as u32;
                    offset += 1;
                    let handler_id = read_u16(data, offset) as u32;
                    offset += 2;

                    host_listen(node_id, event_type, handler_id);
                }

                OP_DELTA_PATCH => {
                    // Delta patch format:
                    // - cache_id: u32 (4 bytes) - identifies the cached base data
                    // - patch_len: u32 (4 bytes) - length of patch data
                    // - patch_data: [u8; patch_len] - the delta patch
                    if offset + 8 > data.len() {
                        break;
                    }
                    let cache_id = read_u32(data, offset);
                    offset += 4;
                    let patch_len = read_u32(data, offset) as usize;
                    offset += 4;

                    if offset + patch_len > data.len() {
                        break;
                    }
                    let patch_data = &data[offset..offset + patch_len];
                    offset += patch_len;

                    // Apply the delta patch
                    if apply_delta_patch(cache_id, patch_data) != 0 {
                        // Delta patch failed, log error
                        host_log(0xDEAD);
                    }
                }

                OP_EOF => break,

                _ => {
                    // Unknown opcode, stop processing
                    break;
                }
            }
        }
    }
    0
}

/// Event dispatcher (called by JS)
#[unsafe(no_mangle)]
pub extern "C" fn on_event(handler_id: u32) {
    // Dispatch to registered handler
    unsafe {
        host_log(handler_id);
    }
}

/// Get node count
#[unsafe(no_mangle)]
pub extern "C" fn get_node_count() -> u32 {
    unsafe { RUNTIME.node_count }
}

/// Get template count
#[unsafe(no_mangle)]
pub extern "C" fn get_template_count() -> u32 {
    unsafe { RUNTIME.template_count }
}

/// Reset runtime
#[unsafe(no_mangle)]
pub extern "C" fn reset() {
    unsafe {
        RUNTIME.node_count = 0;
        RUNTIME.template_count = 0;
        allocator::reset_heap();
    }
}

// ============================================================================
// Utilities
// ============================================================================

#[inline]
fn read_u16(data: &[u8], offset: usize) -> u16 {
    if offset + 1 >= data.len() {
        return 0;
    }
    (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
}

#[inline]
fn read_u32(data: &[u8], offset: usize) -> u32 {
    if offset + 3 >= data.len() {
        return 0;
    }
    (data[offset] as u32)
        | ((data[offset + 1] as u32) << 8)
        | ((data[offset + 2] as u32) << 16)
        | ((data[offset + 3] as u32) << 24)
}

// ============================================================================
// Delta Patch Application
// ============================================================================

/// Maximum base data size we can handle (64KB)
const MAX_BASE_SIZE: usize = 65536;

/// Maximum result size we can handle (128KB)
const MAX_RESULT_SIZE: usize = 131072;

/// Apply a delta patch to cached base data
///
/// # Returns
/// - 0 on success
/// - 1 on error (invalid patch, base not found, etc.)
unsafe fn apply_delta_patch(cache_id: u32, patch_data: &[u8]) -> u32 {
    // Validate patch header
    if patch_data.len() < DELTA_HEADER_SIZE {
        return 1;
    }

    // Check magic bytes
    if patch_data[0..4] != DELTA_MAGIC {
        return 1;
    }

    // Check version (must be 1)
    if patch_data[4] != 1 {
        return 1;
    }

    // Parse header
    let block_size = read_u16(patch_data, 5) as usize;
    if block_size == 0 {
        return 1;
    }

    // Get base data from cache
    // First, query the size needed
    let base_size = unsafe { host_get_cached_base(cache_id, core::ptr::null_mut(), 0) } as usize;
    if base_size == 0 || base_size > MAX_BASE_SIZE {
        return 1;
    }

    // Allocate buffer for base data using our bump allocator
    let base_buf = allocator::alloc_bytes(base_size);
    if base_buf.is_null() {
        return 1;
    }

    // Read base data into buffer
    let actual_size =
        unsafe { host_get_cached_base(cache_id, base_buf, base_size as u32) } as usize;
    if actual_size != base_size {
        return 1;
    }

    let base_data = unsafe { slice::from_raw_parts(base_buf, base_size) };

    // Allocate result buffer
    let result_buf = allocator::alloc_bytes(MAX_RESULT_SIZE);
    if result_buf.is_null() {
        return 1;
    }

    // Apply delta operations
    let mut result_len: usize = 0;
    let mut offset = DELTA_HEADER_SIZE;

    while offset < patch_data.len() {
        let opcode = patch_data[offset];
        offset += 1;

        match opcode {
            DELTA_OP_COPY => {
                // Copy block from base
                if offset + 4 > patch_data.len() {
                    return 1;
                }
                let block_idx = read_u32(patch_data, offset) as usize;
                offset += 4;

                let start = block_idx * block_size;
                let end = (start + block_size).min(base_size);

                if start >= base_size {
                    return 1;
                }

                let copy_len = end - start;
                if result_len + copy_len > MAX_RESULT_SIZE {
                    return 1;
                }

                // Copy block to result
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        base_data.as_ptr().add(start),
                        result_buf.add(result_len),
                        copy_len,
                    );
                }
                result_len += copy_len;
            }
            DELTA_OP_LITERAL => {
                // Insert literal bytes
                if offset + 2 > patch_data.len() {
                    return 1;
                }
                let literal_len = read_u16(patch_data, offset) as usize;
                offset += 2;

                if offset + literal_len > patch_data.len() {
                    return 1;
                }

                if result_len + literal_len > MAX_RESULT_SIZE {
                    return 1;
                }

                // Copy literal data to result
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        patch_data.as_ptr().add(offset),
                        result_buf.add(result_len),
                        literal_len,
                    );
                }
                offset += literal_len;
                result_len += literal_len;
            }
            _ => {
                // Unknown opcode
                return 1;
            }
        }
    }

    // Store patched result back to cache
    unsafe {
        host_store_patched(cache_id, result_buf, result_len as u32);
    }

    0
}
