//! Ultra-minimal bump allocator for no_std WASM
//!
//! Zero malloc/free overhead - just a pointer increment

#![allow(static_mut_refs)]

use core::alloc::{GlobalAlloc, Layout};

pub struct BumpAlloc;

const HEAP_SIZE: usize = 65536; // 64KB = 1 WASM page
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut PTR: usize = 0;

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe {
            let align = layout.align();
            let start = (PTR + align - 1) & !(align - 1);
            let end = start + layout.size();

            if end > HEAP_SIZE {
                #[cfg(target_arch = "wasm32")]
                core::arch::wasm32::unreachable();
                #[cfg(not(target_arch = "wasm32"))]
                return core::ptr::null_mut();
            }

            PTR = end;
            HEAP.as_mut_ptr().add(start)
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op - bump allocator doesn't free
    }
}

/// Reset heap pointer
#[unsafe(no_mangle)]
pub unsafe fn reset_heap() {
    unsafe {
        PTR = 0;
    }
}

/// Allocate a byte buffer of the given size
///
/// Returns null if allocation fails (out of memory)
pub fn alloc_bytes(size: usize) -> *mut u8 {
    if size == 0 {
        return core::ptr::null_mut();
    }

    unsafe {
        let start = PTR;
        let end = start + size;

        if end > HEAP_SIZE {
            return core::ptr::null_mut();
        }

        PTR = end;
        HEAP.as_mut_ptr().add(start)
    }
}
