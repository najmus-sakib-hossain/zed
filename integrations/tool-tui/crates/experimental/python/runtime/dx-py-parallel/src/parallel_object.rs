//! Parallel-safe Python object implementation

use dx_py_gc::LockFreeRefCount;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// Type tag for Python objects
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyTypeTag {
    None = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    Str = 4,
    Bytes = 5,
    List = 6,
    Dict = 7,
    Tuple = 8,
    Set = 9,
    Function = 10,
    Class = 11,
    Instance = 12,
    Module = 13,
    Custom = 255,
}

impl PyTypeTag {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Bool),
            2 => Some(Self::Int),
            3 => Some(Self::Float),
            4 => Some(Self::Str),
            5 => Some(Self::Bytes),
            6 => Some(Self::List),
            7 => Some(Self::Dict),
            8 => Some(Self::Tuple),
            9 => Some(Self::Set),
            10 => Some(Self::Function),
            11 => Some(Self::Class),
            12 => Some(Self::Instance),
            13 => Some(Self::Module),
            255 => Some(Self::Custom),
            _ => None,
        }
    }
}

/// A parallel-safe Python object header
#[repr(C)]
pub struct ParallelPyObject {
    /// Lock-free reference count
    refcount: LockFreeRefCount,
    /// Type tag (atomic for safe reads)
    type_tag: AtomicU8,
    /// Object flags
    flags: AtomicU8,
    /// Reserved for alignment
    _reserved: [u8; 6],
    /// Object hash (cached, 0 = not computed)
    hash: AtomicU64,
}

impl ParallelPyObject {
    /// Create a new parallel Python object
    pub fn new(type_tag: PyTypeTag) -> Self {
        Self {
            refcount: LockFreeRefCount::new(),
            type_tag: AtomicU8::new(type_tag as u8),
            flags: AtomicU8::new(0),
            _reserved: [0; 6],
            hash: AtomicU64::new(0),
        }
    }

    /// Get the type tag
    #[inline]
    pub fn type_tag(&self) -> PyTypeTag {
        PyTypeTag::from_u8(self.type_tag.load(Ordering::Acquire)).unwrap_or(PyTypeTag::Custom)
    }

    /// Increment reference count
    #[inline]
    pub fn inc_ref(&self) {
        self.refcount.inc_strong();
    }

    /// Decrement reference count, returns true if object should be freed
    #[inline]
    pub fn dec_ref(&self) -> bool {
        self.refcount.dec_strong()
    }

    /// Get the current reference count
    #[inline]
    pub fn ref_count(&self) -> u32 {
        self.refcount.strong_count()
    }

    /// Get the cached hash value
    #[inline]
    pub fn get_hash(&self) -> Option<u64> {
        let h = self.hash.load(Ordering::Acquire);
        if h == 0 {
            None
        } else {
            Some(h)
        }
    }

    /// Set the cached hash value (only if not already set)
    #[inline]
    pub fn set_hash(&self, hash: u64) -> bool {
        // Ensure hash is never 0 (reserved for "not computed")
        let hash = if hash == 0 { 1 } else { hash };
        self.hash.compare_exchange(0, hash, Ordering::AcqRel, Ordering::Acquire).is_ok()
    }

    /// Atomically compare and swap a field value
    /// This is used for lock-free updates to object fields
    ///
    /// Returns Ok(expected) if the swap succeeded, or Err(actual) if it failed.
    /// If the actual value cannot be converted back to T, returns Err with the expected value.
    pub fn cas_field<T>(field: &AtomicU64, expected: T, new: T) -> Result<T, T>
    where
        T: Copy + PartialEq + Into<u64> + TryFrom<u64>,
    {
        let expected_u64: u64 = expected.into();
        let new_u64: u64 = new.into();

        match field.compare_exchange(expected_u64, new_u64, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => Ok(expected),
            Err(actual) => {
                // Try to convert the actual value back to T
                // If conversion fails, return the expected value as the error
                // (this indicates a type mismatch or invalid state)
                match T::try_from(actual) {
                    Ok(actual_t) => Err(actual_t),
                    Err(_) => Err(expected), // Fallback to expected on conversion failure
                }
            }
        }
    }
}

/// Object flags
pub mod flags {
    pub const IMMUTABLE: u8 = 0x01;
    pub const HASHABLE: u8 = 0x02;
    pub const ITERABLE: u8 = 0x04;
    pub const CALLABLE: u8 = 0x08;
    pub const AWAITABLE: u8 = 0x10;
    pub const GC_TRACKED: u8 = 0x20;
}

impl ParallelPyObject {
    /// Check if the object has a flag set
    #[inline]
    pub fn has_flag(&self, flag: u8) -> bool {
        self.flags.load(Ordering::Acquire) & flag != 0
    }

    /// Set a flag on the object
    #[inline]
    pub fn set_flag(&self, flag: u8) {
        self.flags.fetch_or(flag, Ordering::AcqRel);
    }

    /// Clear a flag on the object
    #[inline]
    pub fn clear_flag(&self, flag: u8) {
        self.flags.fetch_and(!flag, Ordering::AcqRel);
    }

    /// Check if the object is immutable
    #[inline]
    pub fn is_immutable(&self) -> bool {
        self.has_flag(flags::IMMUTABLE)
    }

    /// Check if the object is hashable
    #[inline]
    pub fn is_hashable(&self) -> bool {
        self.has_flag(flags::HASHABLE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_creation() {
        let obj = ParallelPyObject::new(PyTypeTag::Int);
        assert_eq!(obj.type_tag(), PyTypeTag::Int);
        assert_eq!(obj.ref_count(), 1);
    }

    #[test]
    fn test_refcount() {
        let obj = ParallelPyObject::new(PyTypeTag::Str);
        assert_eq!(obj.ref_count(), 1);

        obj.inc_ref();
        assert_eq!(obj.ref_count(), 2);

        assert!(!obj.dec_ref()); // Still has refs
        assert_eq!(obj.ref_count(), 1);

        assert!(obj.dec_ref()); // Should be freed
    }

    #[test]
    fn test_hash_caching() {
        let obj = ParallelPyObject::new(PyTypeTag::Str);
        assert!(obj.get_hash().is_none());

        assert!(obj.set_hash(12345));
        assert_eq!(obj.get_hash(), Some(12345));

        // Second set should fail
        assert!(!obj.set_hash(67890));
        assert_eq!(obj.get_hash(), Some(12345));
    }

    #[test]
    fn test_flags() {
        let obj = ParallelPyObject::new(PyTypeTag::Tuple);

        assert!(!obj.is_immutable());
        obj.set_flag(flags::IMMUTABLE | flags::HASHABLE);
        assert!(obj.is_immutable());
        assert!(obj.is_hashable());

        obj.clear_flag(flags::IMMUTABLE);
        assert!(!obj.is_immutable());
        assert!(obj.is_hashable());
    }
}
