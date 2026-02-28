//! PyObjectHeader - Common header for all Python objects

use dx_py_gc::LockFreeRefCount;
use std::sync::atomic::{AtomicU8, Ordering};

/// Type tag for Python objects (fits in 8 bits)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeTag {
    None = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    Str = 4,
    Bytes = 5,
    List = 6,
    Tuple = 7,
    Dict = 8,
    Set = 9,
    FrozenSet = 10,
    Function = 11,
    Method = 12,
    Class = 13,
    Instance = 14,
    Module = 15,
    Code = 16,
    Frame = 17,
    Generator = 18,
    Coroutine = 19,
    Iterator = 20,
    Range = 21,
    Slice = 22,
    Property = 23,
    StaticMethod = 24,
    ClassMethod = 25,
    Super = 26,
    Type = 27,
    Object = 28,
    Exception = 29,
    // Reserved for future use
    Custom = 255,
}

impl From<u8> for TypeTag {
    fn from(v: u8) -> Self {
        match v {
            0 => TypeTag::None,
            1 => TypeTag::Bool,
            2 => TypeTag::Int,
            3 => TypeTag::Float,
            4 => TypeTag::Str,
            5 => TypeTag::Bytes,
            6 => TypeTag::List,
            7 => TypeTag::Tuple,
            8 => TypeTag::Dict,
            9 => TypeTag::Set,
            10 => TypeTag::FrozenSet,
            11 => TypeTag::Function,
            12 => TypeTag::Method,
            13 => TypeTag::Class,
            14 => TypeTag::Instance,
            15 => TypeTag::Module,
            16 => TypeTag::Code,
            17 => TypeTag::Frame,
            18 => TypeTag::Generator,
            19 => TypeTag::Coroutine,
            20 => TypeTag::Iterator,
            21 => TypeTag::Range,
            22 => TypeTag::Slice,
            23 => TypeTag::Property,
            24 => TypeTag::StaticMethod,
            25 => TypeTag::ClassMethod,
            26 => TypeTag::Super,
            27 => TypeTag::Type,
            28 => TypeTag::Object,
            29 => TypeTag::Exception,
            _ => TypeTag::Custom,
        }
    }
}

/// Object flags
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct ObjectFlags(u8);

impl ObjectFlags {
    pub const NONE: Self = Self(0);
    pub const IMMUTABLE: Self = Self(1 << 0);
    pub const HASHABLE: Self = Self(1 << 1);
    pub const ITERABLE: Self = Self(1 << 2);
    pub const CALLABLE: Self = Self(1 << 3);
    pub const AWAITABLE: Self = Self(1 << 4);
    pub const FINALIZED: Self = Self(1 << 5); // Object has been finalized (__del__ called)
    pub const GC_TRACKED: Self = Self(1 << 6);
    pub const GC_MARKED: Self = Self(1 << 7);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn set(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn clear(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

/// Common header for all Python objects
///
/// Layout (16 bytes total):
/// - refcount: 8 bytes (LockFreeRefCount)
/// - type_tag: 1 byte
/// - flags: 1 byte
/// - reserved: 6 bytes (for future use / alignment)
#[repr(C)]
#[derive(Debug)]
pub struct PyObjectHeader {
    /// Reference count with lock-free operations
    pub refcount: LockFreeRefCount,
    /// Type tag for fast type checking
    type_tag: AtomicU8,
    /// Object flags
    flags: AtomicU8,
    /// Reserved for future use
    _reserved: [u8; 6],
}

impl Clone for PyObjectHeader {
    fn clone(&self) -> Self {
        Self {
            refcount: LockFreeRefCount::new(),
            type_tag: AtomicU8::new(self.type_tag.load(Ordering::Acquire)),
            flags: AtomicU8::new(self.flags.load(Ordering::Acquire)),
            _reserved: [0; 6],
        }
    }
}

impl PyObjectHeader {
    /// Create a new object header
    pub fn new(type_tag: TypeTag, flags: ObjectFlags) -> Self {
        Self {
            refcount: LockFreeRefCount::new(),
            type_tag: AtomicU8::new(type_tag as u8),
            flags: AtomicU8::new(flags.0),
            _reserved: [0; 6],
        }
    }

    /// Get the type tag
    #[inline]
    pub fn type_tag(&self) -> TypeTag {
        TypeTag::from(self.type_tag.load(Ordering::Relaxed))
    }

    /// Get the flags
    #[inline]
    pub fn flags(&self) -> ObjectFlags {
        ObjectFlags(self.flags.load(Ordering::Relaxed))
    }

    /// Set a flag
    #[inline]
    pub fn set_flag(&self, flag: ObjectFlags) {
        self.flags.fetch_or(flag.0, Ordering::Relaxed);
    }

    /// Clear a flag
    #[inline]
    pub fn clear_flag(&self, flag: ObjectFlags) {
        self.flags.fetch_and(!flag.0, Ordering::Relaxed);
    }

    /// Check if a flag is set
    #[inline]
    pub fn has_flag(&self, flag: ObjectFlags) -> bool {
        (self.flags.load(Ordering::Relaxed) & flag.0) == flag.0
    }

    /// Increment reference count
    #[inline]
    pub fn incref(&self) {
        self.refcount.inc_strong();
    }

    /// Decrement reference count, returns true if object should be freed
    #[inline]
    pub fn decref(&self) -> bool {
        self.refcount.dec_strong()
    }

    /// Decrement reference count and handle cleanup if needed
    /// This is the preferred method for decrementing references as it handles __del__ calls
    #[inline]
    pub fn decref_with_cleanup<F>(&self, cleanup_fn: F) -> bool
    where
        F: FnOnce(),
    {
        if self.refcount.dec_strong() {
            // Object should be deallocated - call cleanup first
            cleanup_fn();
            true
        } else {
            false
        }
    }

    /// Get current reference count
    #[inline]
    pub fn refcount(&self) -> u32 {
        self.refcount.strong_count()
    }

    /// Mark object for GC tracing
    #[inline]
    pub fn gc_mark(&self) {
        self.set_flag(ObjectFlags::GC_MARKED);
    }

    /// Unmark object after GC tracing
    #[inline]
    pub fn gc_unmark(&self) {
        self.clear_flag(ObjectFlags::GC_MARKED);
    }

    /// Check if object is marked
    #[inline]
    pub fn is_gc_marked(&self) -> bool {
        self.has_flag(ObjectFlags::GC_MARKED)
    }

    /// Track object in GC
    #[inline]
    pub fn gc_track(&self) {
        self.set_flag(ObjectFlags::GC_TRACKED);
    }

    /// Untrack object from GC
    #[inline]
    pub fn gc_untrack(&self) {
        self.clear_flag(ObjectFlags::GC_TRACKED);
    }

    /// Check if object is tracked by GC
    #[inline]
    pub fn is_gc_tracked(&self) -> bool {
        self.has_flag(ObjectFlags::GC_TRACKED)
    }
}

impl Default for PyObjectHeader {
    fn default() -> Self {
        Self::new(TypeTag::Object, ObjectFlags::NONE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<PyObjectHeader>(), 16);
    }

    #[test]
    fn test_type_tag() {
        let header = PyObjectHeader::new(TypeTag::Int, ObjectFlags::IMMUTABLE);
        assert_eq!(header.type_tag(), TypeTag::Int);
    }

    #[test]
    fn test_flags() {
        let header = PyObjectHeader::new(TypeTag::List, ObjectFlags::NONE);
        assert!(!header.has_flag(ObjectFlags::IMMUTABLE));

        header.set_flag(ObjectFlags::ITERABLE);
        assert!(header.has_flag(ObjectFlags::ITERABLE));

        header.clear_flag(ObjectFlags::ITERABLE);
        assert!(!header.has_flag(ObjectFlags::ITERABLE));
    }

    #[test]
    fn test_refcount() {
        let header = PyObjectHeader::new(TypeTag::Str, ObjectFlags::IMMUTABLE);
        assert_eq!(header.refcount(), 1);

        header.incref();
        assert_eq!(header.refcount(), 2);

        assert!(!header.decref());
        assert_eq!(header.refcount(), 1);

        assert!(header.decref());
    }
}
