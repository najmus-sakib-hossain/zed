
# Unsafe Code Audit - DX-JS Runtime

This document provides a comprehensive audit of all `unsafe` code blocks in the DX-JS runtime. Each unsafe block is documented with its location, purpose, safety justification, and any invariants that must be maintained.

## Audit Summary

+--------------+--------+--------+----------+--------+---------+
| Module       | Unsafe | Blocks | Risk     | Level  | Status  |
+==============+========+========+==========+========+=========+
| `gc/heap.rs` | 18     | Medium | Reviewed | `gc/gc | ref.rs` |
+--------------+--------+--------+----------+--------+---------+



## Module: `gc/heap.rs`

### 1. Arena Allocation (`Arena::new`)

```rust
let ptr = unsafe { alloc(layout) };
```
Purpose: Allocate raw memory for the GC arena. Safety Justification: -Layout is validated before allocation via `Layout::from_size_align` -Returned pointer is checked for null before use -Memory is properly deallocated in `Drop` implementation Invariants: -Layout must be valid (size > 0, alignment is power of 2) -Pointer must be freed with matching layout

### 2. Arena End Pointer Calculation

```rust
end: unsafe { ptr.add(size) }, ```
Purpose: Calculate the end boundary of the arena. Safety Justification: -`ptr` is guaranteed non-null from successful allocation -`size` matches the allocated size -Result is only used for bounds checking, never dereferenced Invariants: -`size` must match the allocated size exactly


### 3. Arena Deallocation (`Arena::drop`)


```rust
unsafe { let layout = Layout::from_size_align_unchecked(self.size, 16);
dealloc(self.start.as_ptr(), layout);
}
```
Purpose: Free arena memory when dropped. Safety Justification: -`self.start` was obtained from a successful `alloc` call -`self.size` matches the original allocation size -Layout matches the original allocation layout Invariants: -Must only be called once (guaranteed by Drop semantics) -Size and alignment must match original allocation


### 4. GC Header Initialization


```rust
unsafe { header_ptr.write(GcHeader::new(T::object_type(), size as u32));
}
```
Purpose: Initialize the GC header for a newly allocated object. Safety Justification: -`header_ptr` points to freshly allocated, uninitialized memory -Memory is properly aligned for `GcHeader` -`write` is used instead of assignment to avoid reading uninitialized memory Invariants: -Pointer must be valid and properly aligned -Memory must not have been previously initialized


### 5. Object Data Initialization


```rust
let data_ptr = unsafe { ptr.as_ptr().add(GcHeader::header_size()) as *mut T };
unsafe { data_ptr.write(value);
}
```
Purpose: Initialize the object data after the GC header. Safety Justification: -Pointer arithmetic is within allocated bounds -Memory is properly aligned for type T -`write` avoids reading uninitialized memory Invariants: -Object data must immediately follow the header -Type T must match the object_type in the header


### 6. Object Tracking


```rust
self.objects.push(unsafe { NonNull::new_unchecked(header_ptr) });
```
Purpose: Track allocated objects for GC iteration. Safety Justification: -`header_ptr` is guaranteed non-null (from successful allocation) -Pointer remains valid until object is collected Invariants: -Pointer must remain valid while in the objects list


### 7. GcRef Creation


```rust
Some(unsafe { GcRef::from_header_ptr(NonNull::new_unchecked(header_ptr)) })
```
Purpose: Create a GcRef from the allocated object. Safety Justification: -Header has been properly initialized -Object data has been properly initialized -Pointer is guaranteed non-null Invariants: -Header and object must be fully initialized before creating GcRef


### 8. String Data Copy


```rust
unsafe { let len_ptr = string_ptr as *mut u32;
let hash_ptr = (string_ptr as *mut u8).add(4) as *mut u32;
std::ptr::write(len_ptr, s.len() as u32);
std::ptr::write(hash_ptr, hash_string(s));
let data_ptr = (string_ptr as *mut u8).add(std::mem::size_of::<GcString>());
std::ptr::copy_nonoverlapping(s.as_ptr(), data_ptr, s.len());
}
```
Purpose: Initialize GcString fields and copy string data. Safety Justification: -All pointer arithmetic is within allocated bounds -`total_size` calculation ensures sufficient space -`copy_nonoverlapping` is safe because source and destination don't overlap Invariants: -Allocated size must be at least `GcString::total_size(s.len())` -Source string must be valid UTF-8


### 9. Remembered Set Access


```rust
let header = unsafe { &*(addr as *const GcHeader) };
```
Purpose: Access GC header from remembered set address. Safety Justification: -Addresses in remembered set are from valid allocations -Objects are not freed while in remembered set Invariants: -Remembered set must only contain valid object addresses


## Module: `gc/gc_ref.rs`



### 10. GcRef from Header Pointer


```rust
pub unsafe fn from_header_ptr(ptr: NonNull<GcHeader>) -> Self ```
Purpose: Create a GcRef from a raw header pointer. Safety Justification: -Caller must ensure pointer points to valid GcHeader -Caller must ensure object data follows the header Invariants: -Pointer must point to valid, initialized GcHeader -Object data must be properly initialized

### 11. GcRef from Data Pointer

```rust
pub unsafe fn from_data_ptr(ptr: NonNull<T>) -> Self { let header_ptr = (ptr.as_ptr() as *mut u8).sub(GcHeader::header_size()) as *mut GcHeader;
...
}
```
Purpose: Create a GcRef from a pointer to object data. Safety Justification: -Caller must ensure pointer points to valid object data -Header must exist at the expected offset before the data Invariants: -Object must have been allocated through GcHeap -Header must be at exactly `GcHeader::header_size()` bytes before data

### 12. Header Access

```rust
pub fn header(&self) -> &GcHeader { unsafe { self.ptr.as_ref() }
}
```
Purpose: Get a reference to the GC header. Safety Justification: -GcRef is only created from valid allocations -Header remains valid while GcRef exists Invariants: -GcRef must only be created from valid GcHeap allocations

### 13. Data Pointer Access

```rust
pub fn as_ptr(&self) -> *const T { unsafe { (self.ptr.as_ptr() as *const u8).add(GcHeader::header_size()) as *const T }
}
```
Purpose: Get a raw pointer to the object data. Safety Justification: -Pointer arithmetic is within allocated bounds -Object data follows header at fixed offset Invariants: -Object layout must match expected format

### 14. Deref Implementation

```rust
fn deref(&self) -> &Self::Target { unsafe { &*self.as_ptr() }
}
```
Purpose: Allow dereferencing GcRef to access object data. Safety Justification: -GC guarantees object is valid while GcRef exists -Pointer is properly aligned for type T Invariants: -Object must not be collected while GcRef is in use

### 15. GcString Byte Access

```rust
pub fn as_bytes(&self) -> &[u8] { unsafe { let data_ptr = (self as *const Self).add(1) as *const u8;
std::slice::from_raw_parts(data_ptr, self.len as usize)
}
}
```
Purpose: Access string bytes stored after the GcString header. Safety Justification: -String data is stored immediately after GcString struct -Length field accurately reflects data size Invariants: -`self.len` must match actual stored data length

### 16. GcString UTF-8 Conversion

```rust
pub fn as_str(&self) -> &str { unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
}
```
Purpose: Convert string bytes to &str without validation. Safety Justification: -Only valid UTF-8 strings are stored in GcString -Validation is performed at allocation time Invariants: -All GcString instances must contain valid UTF-8

### 17. GcArray Element Access

```rust
pub fn as_slice(&self) -> &[TaggedValue] { unsafe { let data_ptr = (self as *const Self).add(1) as *const TaggedValue;
std::slice::from_raw_parts(data_ptr, self.len as usize)
}
}
```
Purpose: Access array elements stored after the GcArray header. Safety Justification: -Elements are stored immediately after GcArray struct -Length field accurately reflects element count Invariants: -`self.len` must match actual element count

### 18. GcArray Trace Implementation

```rust
if let Some(ptr) = elem.as_non_null() { unsafe { let header_ptr = ptr.as_ptr().sub(GcHeader::header_size()) as *mut GcHeader;
...
}
}
```
Purpose: Trace array elements during garbage collection. Safety Justification: -Only heap pointers are traced -Header is at known offset before object data Invariants: -TaggedValue heap pointers must point to valid GC objects

## Module: `runtime/memory.rs`

### 19. Arena Send/Sync Implementation

```rust
unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}
```
Purpose: Allow Arena to be used across threads. Safety Justification: -Access is controlled via atomic operations -No mutable aliasing is possible Invariants: -All access must use atomic operations

### 20. Arena Memory Allocation

```rust
let base = unsafe { alloc(layout) };
```
Purpose: Allocate memory for the arena. Safety Justification: -Layout is validated before allocation -Null check is performed after allocation Invariants: -Layout must be valid

### 21. Arena Pointer Return

```rust
return Some(unsafe { self.base.add(aligned) });
```
Purpose: Return pointer to allocated region within arena. Safety Justification: -Bounds checking ensures pointer is within arena -Atomic operations prevent race conditions Invariants: -Returned pointer must be within arena bounds

### 22. Arena Deallocation

```rust
unsafe { dealloc(self.base, self.layout);
}
```
Purpose: Free arena memory. Safety Justification: -Base pointer and layout match original allocation -Called only once via Drop Invariants: -Must match original allocation parameters

## Module: `runtime/nodejs.rs`

### 23. Buffer Unsafe Allocation

```rust
pub fn alloc_unsafe(&self, size: usize) -> Buffer { let mut data = Vec::with_capacity(size);
unsafe { data.set_len(size);
}
...
}
```
Purpose: Create uninitialized buffer matching Node.js `Buffer.allocUnsafe`. Safety Justification: -Matches Node.js behavior for performance -Caller is responsible for initializing before reading -Documented as unsafe in API Invariants: -Caller must initialize buffer before reading

## Module: `zero_copy/mod.rs`

### 24. Memory Map Creation

```rust
let mmap = unsafe { Mmap::map(file)? };
```
Purpose: Create memory-mapped file for zero-copy reading. Safety Justification: -File is open and valid -Mapping is read-only -File handle is kept alive Invariants: -File must remain open while mmap is in use

### 25. Memory Map Content Access

```rust
let content = unsafe { let ptr = mmap.as_ptr();
let len = mmap.len();
...
};
```
Purpose: Access memory-mapped content as a slice. Safety Justification: -Mmap is valid and properly initialized -Length matches actual mapped size Invariants: -Mmap must remain valid during access

### 26. Lifetime Extension

```rust
let content: &'static str = unsafe { std::mem::transmute(content) };
```
Purpose: Extend lifetime of memory-mapped content. Safety Justification: -Mmap is stored alongside the reference -Reference is only valid while struct exists Invariants: -Struct must keep mmap alive for lifetime of reference

## Recommendations

### High Priority

- Add debug assertions to verify invariants in debug builds
- Consider using `NonNull` more consistently for pointer types
- Add MIRI testing to CI for detecting undefined behavior

### Medium Priority

- Document thread-safety guarantees more explicitly
- Consider using `MaybeUninit` for uninitialized memory
- Add bounds checking in debug builds for pointer arithmetic

### Low Priority

- Reduce unsafe surface area where possible
- Consider safe abstractions for common patterns
- Add fuzzing tests for memory management code

## Audit History

Date: 2025-12-30, Auditor: Kiro, Version: 0.0.1, Notes: Initial comprehensive audit

## References

- Rustonomicon
- Unsafe Code Guidelines
- Rust API Guidelines
- Safety
- MIRI
- Undefined Behavior Detector
