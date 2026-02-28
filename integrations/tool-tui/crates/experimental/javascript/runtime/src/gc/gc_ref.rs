//! GcRef - Smart pointer for GC-managed objects
//!
//! GcRef is a reference-counted smart pointer that tracks references to
//! heap-allocated JavaScript objects. It provides:
//! - Automatic reference counting for root tracking
//! - Safe dereferencing with lifetime tracking
//! - Write barrier support for generational GC

use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Deref;
use std::ptr::NonNull;

use super::header::{GcHeader, ObjectType};

/// Trait for types that can be managed by the garbage collector
pub trait GcObject: Sized {
    /// Get the object type tag
    fn object_type() -> ObjectType;

    /// Trace all references to other GC objects
    /// Called during the mark phase of garbage collection
    fn trace(&self, tracer: &mut dyn FnMut(GcRef<()>));

    /// Get the size of this object in bytes (including header)
    fn size(&self) -> usize {
        std::mem::size_of::<GcHeader>() + std::mem::size_of::<Self>()
    }
}

/// A reference to a garbage-collected object
///
/// GcRef is a smart pointer that holds a reference to a heap-allocated
/// JavaScript object. It is designed to work with the generational
/// garbage collector.
///
/// # Safety
///
/// GcRef assumes that the pointed-to memory is valid and properly aligned.
/// The garbage collector is responsible for ensuring this invariant.
///
/// # Example
///
/// ```ignore
/// let gc_string: GcRef<GcString> = heap.alloc(GcString::new("hello"));
/// println!("{}", gc_string.as_str());
/// ```
#[repr(transparent)]
pub struct GcRef<T> {
    /// Pointer to the GC header (object data follows immediately after)
    ptr: NonNull<GcHeader>,
    /// Phantom data for type safety
    _marker: PhantomData<T>,
}

impl<T> GcRef<T> {
    /// Create a new GcRef from a raw pointer to the header
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid GcHeader followed by a valid T.
    /// The memory must remain valid for the lifetime of this GcRef.
    #[inline]
    pub unsafe fn from_header_ptr(ptr: NonNull<GcHeader>) -> Self {
        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    /// Create a new GcRef from a raw pointer to the object data
    ///
    /// # Safety
    ///
    /// The pointer must point to valid object data that is preceded by a GcHeader.
    #[inline]
    pub unsafe fn from_data_ptr(ptr: NonNull<T>) -> Self {
        let header_ptr = (ptr.as_ptr() as *mut u8).sub(GcHeader::header_size()) as *mut GcHeader;
        Self {
            ptr: NonNull::new_unchecked(header_ptr),
            _marker: PhantomData,
        }
    }

    /// Get a reference to the GC header
    #[inline]
    pub fn header(&self) -> &GcHeader {
        // SAFETY: ptr is guaranteed to point to a valid GcHeader
        unsafe { self.ptr.as_ref() }
    }

    /// Get a raw pointer to the object data
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        // SAFETY: Object data follows immediately after the header
        unsafe { (self.ptr.as_ptr() as *const u8).add(GcHeader::header_size()) as *const T }
    }

    /// Get a mutable raw pointer to the object data
    #[inline]
    pub fn as_mut_ptr(&self) -> *mut T {
        // SAFETY: Object data follows immediately after the header at a fixed offset.
        // The caller is responsible for ensuring exclusive access when dereferencing.
        unsafe { (self.ptr.as_ptr() as *mut u8).add(GcHeader::header_size()) as *mut T }
    }

    /// Get a raw pointer to the header
    #[inline]
    pub fn header_ptr(&self) -> *const GcHeader {
        self.ptr.as_ptr()
    }

    /// Cast to a type-erased GcRef
    #[inline]
    pub fn erase(self) -> GcRef<()> {
        GcRef {
            ptr: self.ptr,
            _marker: PhantomData,
        }
    }

    /// Check if this reference points to the same object as another
    #[inline]
    pub fn ptr_eq<U>(&self, other: &GcRef<U>) -> bool {
        self.ptr == other.ptr
    }

    /// Get the address of the object (for hashing/comparison)
    #[inline]
    pub fn addr(&self) -> usize {
        self.ptr.as_ptr() as usize
    }
}

impl<T> Clone for GcRef<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for GcRef<T> {}

impl<T> Deref for GcRef<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // SAFETY: The GC guarantees the object is valid while GcRef exists
        unsafe { &*self.as_ptr() }
    }
}

impl<T> PartialEq for GcRef<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<T> Eq for GcRef<T> {}

impl<T> Hash for GcRef<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.as_ptr().hash(state);
    }
}

impl<T: fmt::Debug> fmt::Debug for GcRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GcRef")
            .field("addr", &format_args!("{:p}", self.ptr.as_ptr()))
            .field("value", unsafe { &*self.as_ptr() })
            .finish()
    }
}

impl<T> fmt::Pointer for GcRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr.as_ptr(), f)
    }
}

// ============================================================================
// GC-managed String
// ============================================================================

/// A garbage-collected string
#[repr(C)]
pub struct GcString {
    /// Length of the string in bytes
    pub(crate) len: u32,
    /// Hash of the string (cached for fast comparison)
    pub(crate) hash: u32,
    // String data follows immediately after (flexible array member pattern)
}

impl GcString {
    /// Get the string data as a byte slice
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            let data_ptr = (self as *const Self).add(1) as *const u8;
            std::slice::from_raw_parts(data_ptr, self.len as usize)
        }
    }

    /// Get the string as a str
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: We only store valid UTF-8 strings
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Get the length in bytes
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the cached hash
    #[inline]
    pub fn hash(&self) -> u32 {
        self.hash
    }

    /// Calculate the total size needed for a string of given length
    #[inline]
    pub fn total_size(str_len: usize) -> usize {
        GcHeader::header_size() + std::mem::size_of::<GcString>() + str_len
    }
}

impl GcObject for GcString {
    fn object_type() -> ObjectType {
        ObjectType::String
    }

    fn trace(&self, _tracer: &mut dyn FnMut(GcRef<()>)) {
        // Strings don't contain references to other objects
    }

    fn size(&self) -> usize {
        Self::total_size(self.len as usize)
    }
}

impl fmt::Debug for GcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GcString({:?})", self.as_str())
    }
}

impl fmt::Display for GcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// GC-managed Array
// ============================================================================

use crate::value::TaggedValue;

/// A garbage-collected array
#[repr(C)]
pub struct GcArray {
    /// Current length
    len: u32,
    /// Capacity (number of elements that can be stored)
    capacity: u32,
    // Elements follow immediately after (flexible array member pattern)
}

impl GcArray {
    /// Get the elements as a slice
    #[inline]
    pub fn as_slice(&self) -> &[TaggedValue] {
        unsafe {
            let data_ptr = (self as *const Self).add(1) as *const TaggedValue;
            std::slice::from_raw_parts(data_ptr, self.len as usize)
        }
    }

    /// Get the elements as a mutable slice - reserved for array mutation
    #[inline]
    #[allow(dead_code)]
    pub fn as_mut_slice(&mut self) -> &mut [TaggedValue] {
        unsafe {
            let data_ptr = (self as *mut Self).add(1) as *mut TaggedValue;
            std::slice::from_raw_parts_mut(data_ptr, self.len as usize)
        }
    }

    /// Get the length - reserved for array operations
    #[inline]
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if empty - reserved for array operations
    #[inline]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the capacity - reserved for array operations
    #[inline]
    #[allow(dead_code)]
    pub fn capacity(&self) -> usize {
        self.capacity as usize
    }

    /// Calculate the total size needed for an array of given capacity
    #[inline]
    pub fn total_size(capacity: usize) -> usize {
        GcHeader::header_size()
            + std::mem::size_of::<GcArray>()
            + capacity * std::mem::size_of::<TaggedValue>()
    }
}

impl GcObject for GcArray {
    fn object_type() -> ObjectType {
        ObjectType::Array
    }

    fn trace(&self, tracer: &mut dyn FnMut(GcRef<()>)) {
        // Trace all elements that are heap objects
        for elem in self.as_slice() {
            if let Some(ptr) = elem.as_non_null() {
                unsafe {
                    // Calculate header pointer from data pointer
                    let header_ptr = ptr.as_ptr().sub(GcHeader::header_size()) as *mut GcHeader;
                    if let Some(header_nn) = NonNull::new(header_ptr) {
                        let gc_ref = GcRef::<()>::from_header_ptr(header_nn);
                        tracer(gc_ref);
                    }
                }
            }
        }
    }

    fn size(&self) -> usize {
        Self::total_size(self.capacity as usize)
    }
}

impl fmt::Debug for GcArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice().iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_ref_size() {
        // GcRef should be pointer-sized
        assert_eq!(std::mem::size_of::<GcRef<()>>(), std::mem::size_of::<*const ()>());
    }

    #[test]
    fn test_gc_string_size() {
        // GcString header should be 8 bytes
        assert_eq!(std::mem::size_of::<GcString>(), 8);
    }

    #[test]
    fn test_gc_array_size() {
        // GcArray header should be 8 bytes
        assert_eq!(std::mem::size_of::<GcArray>(), 8);
    }
}
