//! CPython C-API compatibility layer

use std::sync::atomic::{AtomicU64, Ordering};

/// Simulated Python object header
#[repr(C)]
pub struct PyObjectHeader {
    /// Reference count
    pub refcount: AtomicU64,
    /// Type pointer (would point to PyTypeObject in real impl)
    pub ob_type: *const (),
}

impl PyObjectHeader {
    pub fn new() -> Self {
        Self {
            refcount: AtomicU64::new(1),
            ob_type: std::ptr::null(),
        }
    }

    pub fn inc_ref(&self) {
        self.refcount.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_ref(&self) -> bool {
        let old = self.refcount.fetch_sub(1, Ordering::Release);
        if old == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            true // Should deallocate
        } else {
            false
        }
    }

    pub fn ref_count(&self) -> u64 {
        self.refcount.load(Ordering::Relaxed)
    }
}

impl Default for PyObjectHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// Type tags for quick type checking
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
    Function = 10,
    Module = 11,
    Type = 12,
    Object = 13,
}

/// CPython C-API compatibility layer
///
/// Provides function pointers compatible with the CPython C-API
/// for existing C extensions.
pub struct CApiCompat {
    /// API function table
    api_table: Vec<*const ()>,
}

impl CApiCompat {
    /// Initialize C-API compatibility table
    pub fn new() -> Self {
        let mut api_table = Vec::with_capacity(100);

        // Core object functions
        api_table.push(Self::py_incref as *const ());
        api_table.push(Self::py_decref as *const ());
        api_table.push(Self::py_refcnt as *const ());

        // Type checking
        api_table.push(Self::py_type_check as *const ());
        api_table.push(Self::py_long_check as *const ());
        api_table.push(Self::py_float_check as *const ());
        api_table.push(Self::py_unicode_check as *const ());
        api_table.push(Self::py_list_check as *const ());
        api_table.push(Self::py_dict_check as *const ());
        api_table.push(Self::py_tuple_check as *const ());

        Self { api_table }
    }

    /// Get the API table pointer
    pub fn api_table(&self) -> *const *const () {
        self.api_table.as_ptr()
    }

    /// Get the number of API functions
    pub fn api_count(&self) -> usize {
        self.api_table.len()
    }

    // C-API compatible functions

    extern "C" fn py_incref(obj: *mut PyObjectHeader) {
        if !obj.is_null() {
            unsafe { (*obj).inc_ref() };
        }
    }

    extern "C" fn py_decref(obj: *mut PyObjectHeader) {
        if !obj.is_null() {
            unsafe {
                if (*obj).dec_ref() {
                    // Would deallocate here
                }
            }
        }
    }

    extern "C" fn py_refcnt(obj: *const PyObjectHeader) -> u64 {
        if obj.is_null() {
            0
        } else {
            unsafe { (*obj).ref_count() }
        }
    }

    extern "C" fn py_type_check(_obj: *const PyObjectHeader, _type_tag: u8) -> i32 {
        // Simplified type check
        1
    }

    extern "C" fn py_long_check(obj: *const PyObjectHeader) -> i32 {
        Self::py_type_check(obj, TypeTag::Int as u8)
    }

    extern "C" fn py_float_check(obj: *const PyObjectHeader) -> i32 {
        Self::py_type_check(obj, TypeTag::Float as u8)
    }

    extern "C" fn py_unicode_check(obj: *const PyObjectHeader) -> i32 {
        Self::py_type_check(obj, TypeTag::Str as u8)
    }

    extern "C" fn py_list_check(obj: *const PyObjectHeader) -> i32 {
        Self::py_type_check(obj, TypeTag::List as u8)
    }

    extern "C" fn py_dict_check(obj: *const PyObjectHeader) -> i32 {
        Self::py_type_check(obj, TypeTag::Dict as u8)
    }

    extern "C" fn py_tuple_check(obj: *const PyObjectHeader) -> i32 {
        Self::py_type_check(obj, TypeTag::Tuple as u8)
    }
}

impl Default for CApiCompat {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_object_header() {
        let header = PyObjectHeader::new();
        assert_eq!(header.ref_count(), 1);

        header.inc_ref();
        assert_eq!(header.ref_count(), 2);

        assert!(!header.dec_ref());
        assert_eq!(header.ref_count(), 1);

        assert!(header.dec_ref());
        assert_eq!(header.ref_count(), 0);
    }

    #[test]
    fn test_capi_compat() {
        let capi = CApiCompat::new();
        assert!(capi.api_count() > 0);
        assert!(!capi.api_table().is_null());
    }
}
