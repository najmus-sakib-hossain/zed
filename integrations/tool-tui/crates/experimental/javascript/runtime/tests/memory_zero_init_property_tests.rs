//! Property tests for Memory Zero-Initialization
//!
//! Feature: dx-runtime-production-ready
//! Property 1: Memory Zero-Initialization
//!
//! These tests verify that the Arena allocator:
//! - Zero-initializes all allocated memory
//! - Maintains zero-initialization after reset and reuse
//! - Prevents use of uninitialized data
//!
//! **Validates: Requirements 1.1**

use dx_js_runtime::runtime::memory::Arena;
use proptest::prelude::*;

// ============================================================================
// Property 1: Memory Zero-Initialization
// For any memory allocation from the Arena_Allocator, all bytes in the
// allocated region should be zero before first use.
// **Validates: Requirements 1.1**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: Newly allocated memory is zero-initialized
    /// Feature: dx-runtime-production-ready, Property 1: Memory Zero-Initialization
    #[test]
    fn prop_newly_allocated_memory_is_zero(
        size in 1..4096usize,
        align in prop::sample::select(vec![1usize, 2, 4, 8, 16, 32, 64])
    ) {
        let arena = Arena::new(64 * 1024).expect("Failed to create arena");
        
        if let Some(ptr) = arena.alloc(size, align) {
            // Property: All bytes in the allocated region should be zero
            let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
            for (i, &byte) in slice.iter().enumerate() {
                prop_assert_eq!(
                    byte, 0,
                    "Byte at offset {} should be zero, but was {}",
                    i, byte
                );
            }
        }
    }

    /// Property 1: Multiple allocations are all zero-initialized
    /// Feature: dx-runtime-production-ready, Property 1: Memory Zero-Initialization
    #[test]
    fn prop_multiple_allocations_are_zero(
        sizes in prop::collection::vec(1..512usize, 1..20)
    ) {
        let arena = Arena::new(64 * 1024).expect("Failed to create arena");
        
        for (alloc_idx, &size) in sizes.iter().enumerate() {
            if let Some(ptr) = arena.alloc(size, 8) {
                // Property: All bytes in each allocation should be zero
                let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
                for (byte_idx, &byte) in slice.iter().enumerate() {
                    prop_assert_eq!(
                        byte, 0,
                        "Allocation {}, byte {} should be zero, but was {}",
                        alloc_idx, byte_idx, byte
                    );
                }
            }
        }
    }

    /// Property 1: Memory remains zero after reset and reallocation
    /// Feature: dx-runtime-production-ready, Property 1: Memory Zero-Initialization
    #[test]
    fn prop_memory_zero_after_reset(
        size in 1..2048usize,
        fill_value in 1u8..=255u8
    ) {
        let arena = Arena::new(64 * 1024).expect("Failed to create arena");
        
        // First allocation
        if let Some(ptr) = arena.alloc(size, 8) {
            // Write non-zero data to the allocated memory
            unsafe {
                std::ptr::write_bytes(ptr, fill_value, size);
            }
            
            // Verify the data was written
            let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
            for &byte in slice.iter() {
                prop_assert_eq!(byte, fill_value, "Data should have been written");
            }
        }
        
        // Reset the arena
        arena.reset();
        
        // Reallocate from the same region
        if let Some(ptr) = arena.alloc(size, 8) {
            // Property: Memory should be zero-initialized again after reset
            let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
            for (i, &byte) in slice.iter().enumerate() {
                prop_assert_eq!(
                    byte, 0,
                    "Byte at offset {} should be zero after reset, but was {}",
                    i, byte
                );
            }
        }
    }

    /// Property 1: Large allocations are zero-initialized
    /// Feature: dx-runtime-production-ready, Property 1: Memory Zero-Initialization
    #[test]
    fn prop_large_allocations_are_zero(
        size_kb in 1..32usize
    ) {
        let size = size_kb * 1024;
        let arena = Arena::new(64 * 1024).expect("Failed to create arena");
        
        if let Some(ptr) = arena.alloc(size, 8) {
            // Property: All bytes in large allocation should be zero
            let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
            
            // Check every byte (for large allocations, this is important)
            for (i, &byte) in slice.iter().enumerate() {
                prop_assert_eq!(
                    byte, 0,
                    "Large allocation: byte at offset {} should be zero, but was {}",
                    i, byte
                );
            }
        }
    }

    /// Property 1: Aligned allocations are zero-initialized
    /// Feature: dx-runtime-production-ready, Property 1: Memory Zero-Initialization
    #[test]
    fn prop_aligned_allocations_are_zero(
        size in 1..1024usize,
        align_power in 0..6u32  // 1, 2, 4, 8, 16, 32
    ) {
        let align = 1usize << align_power;
        let arena = Arena::new(64 * 1024).expect("Failed to create arena");
        
        if let Some(ptr) = arena.alloc(size, align) {
            // Verify alignment
            prop_assert_eq!(
                ptr as usize % align, 0,
                "Pointer should be aligned to {} bytes",
                align
            );
            
            // Property: All bytes should be zero regardless of alignment
            let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
            for (i, &byte) in slice.iter().enumerate() {
                prop_assert_eq!(
                    byte, 0,
                    "Aligned allocation: byte at offset {} should be zero, but was {}",
                    i, byte
                );
            }
        }
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

#[test]
fn test_zero_init_single_byte() {
    let arena = Arena::new(4096).expect("Failed to create arena");
    
    if let Some(ptr) = arena.alloc(1, 1) {
        let byte = unsafe { *ptr };
        assert_eq!(byte, 0, "Single byte allocation should be zero");
    }
}

#[test]
fn test_zero_init_page_boundary() {
    let arena = Arena::new(8192).expect("Failed to create arena");
    
    // Allocate exactly one page
    if let Some(ptr) = arena.alloc(4096, 4096) {
        let slice = unsafe { std::slice::from_raw_parts(ptr, 4096) };
        for (i, &byte) in slice.iter().enumerate() {
            assert_eq!(byte, 0, "Page-aligned allocation: byte {} should be zero", i);
        }
    }
}

#[test]
fn test_zero_init_after_multiple_resets() {
    let arena = Arena::new(4096).expect("Failed to create arena");
    
    for iteration in 0..5 {
        // Allocate and fill with non-zero data
        if let Some(ptr) = arena.alloc(1024, 8) {
            unsafe {
                std::ptr::write_bytes(ptr, 0xFF, 1024);
            }
        }
        
        // Reset
        arena.reset();
        
        // Verify zero-initialization
        if let Some(ptr) = arena.alloc(1024, 8) {
            let slice = unsafe { std::slice::from_raw_parts(ptr, 1024) };
            for (i, &byte) in slice.iter().enumerate() {
                assert_eq!(
                    byte, 0,
                    "Iteration {}: byte {} should be zero after reset",
                    iteration, i
                );
            }
        }
        
        arena.reset();
    }
}

#[test]
fn test_zero_init_interleaved_allocations() {
    let arena = Arena::new(8192).expect("Failed to create arena");
    
    // Allocate several blocks
    let sizes = [64, 128, 256, 512, 1024];
    let mut ptrs = Vec::new();
    
    for &size in &sizes {
        if let Some(ptr) = arena.alloc(size, 8) {
            ptrs.push((ptr, size));
        }
    }
    
    // Verify all allocations are zero
    for (idx, &(ptr, size)) in ptrs.iter().enumerate() {
        let slice = unsafe { std::slice::from_raw_parts(ptr, size) };
        for (i, &byte) in slice.iter().enumerate() {
            assert_eq!(
                byte, 0,
                "Allocation {}, byte {} should be zero",
                idx, i
            );
        }
    }
}
