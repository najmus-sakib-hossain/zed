//! # dx-client-tiny: The Absolute Minimum
//!
//! Target: < 400 bytes unoptimized WASM
//! Strategy: Zero heap, zero allocator, pure FFI stubs

#![no_std]
#![no_main]
#![allow(dead_code)]

// Panic handler - required for no_std
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ============================================================================
// FFI: Minimal host imports (only what we need)
// ============================================================================

unsafe extern "C" {
    fn host_clone_template(id: u32) -> u32;
    fn host_append(parent: u32, child: u32);
    fn host_set_text(node: u32, ptr: *const u8, len: u32);
}

// ============================================================================
// WASM Exports (called by JavaScript runtime)
// ============================================================================

/// Initialize - returns 0 for success
#[unsafe(no_mangle)]
pub extern "C" fn init() -> u32 {
    0
}

/// Render template by ID to body (id=0)
#[unsafe(no_mangle)]
pub extern "C" fn render(template_id: u32) -> u32 {
    unsafe {
        let node = host_clone_template(template_id);
        host_append(0, node);
    }
    0
}

/// Handle event by ID
#[unsafe(no_mangle)]
pub extern "C" fn on_event(_id: u32) {}
