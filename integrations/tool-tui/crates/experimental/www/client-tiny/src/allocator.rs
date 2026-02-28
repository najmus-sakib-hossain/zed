//! Ultra-tiny bump allocator for NO_STD WASM
//!
//! Zero malloc/free overhead - just a pointer increment

use core::alloc::{GlobalAlloc, Layout};

/// Bump allocator with 64KB heap (one WASM page)
pub struct BumpAlloc;

// 64KB = 1 WASM page (minimal footprint)
const HEAP_SIZE: usize = 65536;
static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
static mut PTR: usize = 0;

unsafe impl GlobalAlloc for BumpAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let start = PTR;
        let end = start + layout.size();

        // Crash on OOM to prevent memory corruption
        if end > HEAP_SIZE {
            #[cfg(target_arch = "wasm32")]
            core::arch::wasm32::unreachable();
            #[cfg(not(target_arch = "wasm32"))]
            panic!("OOM");
        }

        PTR = end;
        HEAP.as_mut_ptr().add(start)
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // No-op. We reset the entire heap in one shot.
    }
}

/// Reset heap pointer (called per render frame)
#[no_mangle]
pub unsafe fn reset_heap() {
    PTR = 0;
}
