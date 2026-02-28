//! CPython C API compatibility layer
//!
//! This module provides compatibility structures and functions to support
//! loading and using CPython C extensions.

#![allow(non_camel_case_types)]

use crate::pylist::PyValue;
use std::ffi::{c_char, c_int, c_ulong, c_void};
use std::sync::Arc;

/// CPython PyObject structure (C-compatible)
#[repr(C)]
pub struct PyObject_C {
    /// Reference count
    pub ob_refcnt: isize,
    /// Type object pointer
    pub ob_type: *mut PyTypeObject_C,
}

/// CPython PyTypeObject structure (C-compatible)
#[repr(C)]
pub struct PyTypeObject_C {
    /// Base PyObject
    pub ob_base: PyObject_C,
    /// Type name
    pub tp_name: *const c_char,
    /// Basic size of instances
    pub tp_basicsize: isize,
    /// Size of variable-length items
    pub tp_itemsize: isize,
    /// Destructor function
    pub tp_dealloc: Option<destructor>,
    /// Print function (deprecated)
    pub tp_print: *mut c_void,
    /// Get attribute function
    pub tp_getattr: Option<getattrfunc>,
    /// Set attribute function
    pub tp_setattr: Option<setattrfunc>,
    /// Async methods
    pub tp_as_async: *mut PyAsyncMethods,
    /// Repr function
    pub tp_repr: Option<reprfunc>,
    /// Number methods
    pub tp_as_number: *mut PyNumberMethods,
    /// Sequence methods
    pub tp_as_sequence: *mut PySequenceMethods,
    /// Mapping methods
    pub tp_as_mapping: *mut PyMappingMethods,
    /// Hash function
    pub tp_hash: Option<hashfunc>,
    /// Call function
    pub tp_call: Option<ternaryfunc>,
    /// String representation function
    pub tp_str: Option<reprfunc>,
    /// Get attribute with string name
    pub tp_getattro: Option<getattrofunc>,
    /// Set attribute with string name
    pub tp_setattro: Option<setattrofunc>,
    /// Buffer methods
    pub tp_as_buffer: *mut PyBufferProcs,
    /// Type flags
    pub tp_flags: c_ulong,
    /// Documentation string
    pub tp_doc: *const c_char,
    /// Traverse function for GC
    pub tp_traverse: Option<traverseproc>,
    /// Clear function for GC
    pub tp_clear: Option<inquiry>,
    /// Rich comparison function
    pub tp_richcompare: Option<richcmpfunc>,
    /// Weak reference list offset
    pub tp_weaklistoffset: isize,
    /// Iterator function
    pub tp_iter: Option<getiterfunc>,
    /// Iterator next function
    pub tp_iternext: Option<iternextfunc>,
    /// Methods table
    pub tp_methods: *mut PyMethodDef,
    /// Members table
    pub tp_members: *mut PyMemberDef,
    /// Getset table
    pub tp_getset: *mut PyGetSetDef,
    /// Base type
    pub tp_base: *mut PyTypeObject_C,
    /// Dictionary
    pub tp_dict: *mut PyObject_C,
    /// Descriptor get function
    pub tp_descr_get: Option<descrgetfunc>,
    /// Descriptor set function
    pub tp_descr_set: Option<descrsetfunc>,
    /// Dictionary offset
    pub tp_dictoffset: isize,
    /// Init function
    pub tp_init: Option<initproc>,
    /// Alloc function
    pub tp_alloc: Option<allocfunc>,
    /// New function
    pub tp_new: Option<newfunc>,
    /// Free function
    pub tp_free: Option<freefunc>,
    /// Is GC function
    pub tp_is_gc: Option<inquiry>,
    /// Bases tuple
    pub tp_bases: *mut PyObject_C,
    /// MRO tuple
    pub tp_mro: *mut PyObject_C,
    /// Cache
    pub tp_cache: *mut PyObject_C,
    /// Subclasses
    pub tp_subclasses: *mut PyObject_C,
    /// Weaklist
    pub tp_weaklist: *mut PyObject_C,
    /// Delete function
    pub tp_del: Option<destructor>,
    /// Version tag
    pub tp_version_tag: c_uint,
    /// Finalize function
    pub tp_finalize: Option<destructor>,
}

/// Number methods structure
#[repr(C)]
pub struct PyNumberMethods {
    pub nb_add: Option<binaryfunc>,
    pub nb_subtract: Option<binaryfunc>,
    pub nb_multiply: Option<binaryfunc>,
    pub nb_remainder: Option<binaryfunc>,
    pub nb_divmod: Option<binaryfunc>,
    pub nb_power: Option<ternaryfunc>,
    pub nb_negative: Option<unaryfunc>,
    pub nb_positive: Option<unaryfunc>,
    pub nb_absolute: Option<unaryfunc>,
    pub nb_bool: Option<inquiry>,
    pub nb_invert: Option<unaryfunc>,
    pub nb_lshift: Option<binaryfunc>,
    pub nb_rshift: Option<binaryfunc>,
    pub nb_and: Option<binaryfunc>,
    pub nb_xor: Option<binaryfunc>,
    pub nb_or: Option<binaryfunc>,
    pub nb_int: Option<unaryfunc>,
    pub nb_reserved: *mut c_void,
    pub nb_float: Option<unaryfunc>,
    pub nb_inplace_add: Option<binaryfunc>,
    pub nb_inplace_subtract: Option<binaryfunc>,
    pub nb_inplace_multiply: Option<binaryfunc>,
    pub nb_inplace_remainder: Option<binaryfunc>,
    pub nb_inplace_power: Option<ternaryfunc>,
    pub nb_inplace_lshift: Option<binaryfunc>,
    pub nb_inplace_rshift: Option<binaryfunc>,
    pub nb_inplace_and: Option<binaryfunc>,
    pub nb_inplace_xor: Option<binaryfunc>,
    pub nb_inplace_or: Option<binaryfunc>,
    pub nb_floor_divide: Option<binaryfunc>,
    pub nb_true_divide: Option<binaryfunc>,
    pub nb_inplace_floor_divide: Option<binaryfunc>,
    pub nb_inplace_true_divide: Option<binaryfunc>,
    pub nb_index: Option<unaryfunc>,
    pub nb_matrix_multiply: Option<binaryfunc>,
    pub nb_inplace_matrix_multiply: Option<binaryfunc>,
}

/// Sequence methods structure
#[repr(C)]
pub struct PySequenceMethods {
    pub sq_length: Option<lenfunc>,
    pub sq_concat: Option<binaryfunc>,
    pub sq_repeat: Option<ssizeargfunc>,
    pub sq_item: Option<ssizeargfunc>,
    pub was_sq_slice: *mut c_void,
    pub sq_ass_item: Option<ssizeobjargproc>,
    pub was_sq_ass_slice: *mut c_void,
    pub sq_contains: Option<objobjproc>,
    pub sq_inplace_concat: Option<binaryfunc>,
    pub sq_inplace_repeat: Option<ssizeargfunc>,
}

/// Mapping methods structure
#[repr(C)]
pub struct PyMappingMethods {
    pub mp_length: Option<lenfunc>,
    pub mp_subscript: Option<binaryfunc>,
    pub mp_ass_subscript: Option<objobjargproc>,
}

/// Async methods structure
#[repr(C)]
pub struct PyAsyncMethods {
    pub am_await: Option<unaryfunc>,
    pub am_aiter: Option<unaryfunc>,
    pub am_anext: Option<unaryfunc>,
}

/// Buffer protocol structure
#[repr(C)]
pub struct PyBufferProcs {
    pub bf_getbuffer: Option<getbufferproc>,
    pub bf_releasebuffer: Option<releasebufferproc>,
}

/// Buffer structure for buffer protocol
#[repr(C)]
pub struct Py_buffer {
    pub buf: *mut c_void,
    pub obj: *mut PyObject_C,
    pub len: isize,
    pub itemsize: isize,
    pub readonly: c_int,
    pub ndim: c_int,
    pub format: *mut c_char,
    pub shape: *mut isize,
    pub strides: *mut isize,
    pub suboffsets: *mut isize,
    pub internal: *mut c_void,
}

/// Method definition structure
#[repr(C)]
pub struct PyMethodDef {
    pub ml_name: *const c_char,
    pub ml_meth: *mut c_void,
    pub ml_flags: c_int,
    pub ml_doc: *const c_char,
}

/// Member definition structure
#[repr(C)]
pub struct PyMemberDef {
    pub name: *const c_char,
    pub type_: c_int,
    pub offset: isize,
    pub flags: c_int,
    pub doc: *const c_char,
}

/// Getset definition structure
#[repr(C)]
pub struct PyGetSetDef {
    pub name: *const c_char,
    pub get: Option<getter>,
    pub set: Option<setter>,
    pub doc: *const c_char,
    pub closure: *mut c_void,
}

/// Module definition structure
#[repr(C)]
pub struct PyModuleDef {
    pub m_base: PyModuleDef_Base,
    pub m_name: *const c_char,
    pub m_doc: *const c_char,
    pub m_size: isize,
    pub m_methods: *mut PyMethodDef,
    pub m_slots: *mut PyModuleDef_Slot,
    pub m_traverse: Option<traverseproc>,
    pub m_clear: Option<inquiry>,
    pub m_free: Option<freefunc>,
}

/// Module definition base
#[repr(C)]
pub struct PyModuleDef_Base {
    pub ob_base: PyObject_C,
    pub m_init: Option<extern "C" fn() -> *mut PyObject_C>,
    pub m_index: isize,
    pub m_copy: *mut PyObject_C,
}

/// Module definition slot
#[repr(C)]
pub struct PyModuleDef_Slot {
    pub slot: c_int,
    pub value: *mut c_void,
}

// Function pointer types
pub type destructor = extern "C" fn(*mut PyObject_C);
pub type getattrfunc = extern "C" fn(*mut PyObject_C, *mut c_char) -> *mut PyObject_C;
pub type setattrfunc = extern "C" fn(*mut PyObject_C, *mut c_char, *mut PyObject_C) -> c_int;
pub type reprfunc = extern "C" fn(*mut PyObject_C) -> *mut PyObject_C;
pub type hashfunc = extern "C" fn(*mut PyObject_C) -> isize;
pub type ternaryfunc =
    extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut PyObject_C) -> *mut PyObject_C;
pub type getattrofunc = extern "C" fn(*mut PyObject_C, *mut PyObject_C) -> *mut PyObject_C;
pub type setattrofunc = extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut PyObject_C) -> c_int;
pub type traverseproc = extern "C" fn(*mut PyObject_C, visitproc, *mut c_void) -> c_int;
pub type visitproc = extern "C" fn(*mut PyObject_C, *mut c_void) -> c_int;
pub type inquiry = extern "C" fn(*mut PyObject_C) -> c_int;
pub type richcmpfunc = extern "C" fn(*mut PyObject_C, *mut PyObject_C, c_int) -> *mut PyObject_C;
pub type getiterfunc = extern "C" fn(*mut PyObject_C) -> *mut PyObject_C;
pub type iternextfunc = extern "C" fn(*mut PyObject_C) -> *mut PyObject_C;
pub type descrgetfunc =
    extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut PyObject_C) -> *mut PyObject_C;
pub type descrsetfunc = extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut PyObject_C) -> c_int;
pub type initproc = extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut PyObject_C) -> c_int;
pub type allocfunc = extern "C" fn(*mut PyTypeObject_C, isize) -> *mut PyObject_C;
pub type newfunc =
    extern "C" fn(*mut PyTypeObject_C, *mut PyObject_C, *mut PyObject_C) -> *mut PyObject_C;
pub type freefunc = extern "C" fn(*mut c_void);
pub type binaryfunc = extern "C" fn(*mut PyObject_C, *mut PyObject_C) -> *mut PyObject_C;
pub type unaryfunc = extern "C" fn(*mut PyObject_C) -> *mut PyObject_C;
pub type lenfunc = extern "C" fn(*mut PyObject_C) -> isize;
pub type ssizeargfunc = extern "C" fn(*mut PyObject_C, isize) -> *mut PyObject_C;
pub type ssizeobjargproc = extern "C" fn(*mut PyObject_C, isize, *mut PyObject_C) -> c_int;
pub type objobjproc = extern "C" fn(*mut PyObject_C, *mut PyObject_C) -> c_int;
pub type objobjargproc = extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut PyObject_C) -> c_int;
pub type getbufferproc = extern "C" fn(*mut PyObject_C, *mut Py_buffer, c_int) -> c_int;
pub type releasebufferproc = extern "C" fn(*mut PyObject_C, *mut Py_buffer);
pub type getter = extern "C" fn(*mut PyObject_C, *mut c_void) -> *mut PyObject_C;
pub type setter = extern "C" fn(*mut PyObject_C, *mut PyObject_C, *mut c_void) -> c_int;

// Additional types
pub type c_uint = u32;

/// Extension module loader
pub struct ExtensionLoader {
    /// Loaded extension handles
    handles: std::collections::HashMap<String, libloading::Library>,
}

impl ExtensionLoader {
    /// Create a new extension loader
    pub fn new() -> Self {
        Self {
            handles: std::collections::HashMap::new(),
        }
    }

    /// Load a C extension module
    pub fn load_extension(
        &mut self,
        path: &std::path::Path,
    ) -> Result<Arc<crate::types::PyType>, String> {
        // Get the module name from the filename
        let module_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "Invalid module filename".to_string())?;

        // Load the dynamic library
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|e| format!("Failed to load library {}: {}", path.display(), e))?;

        // Find the PyInit_<modname> function
        let init_name = format!("PyInit_{}", module_name);
        let init_func: libloading::Symbol<unsafe extern "C" fn() -> *mut PyObject_C> =
            unsafe { lib.get(init_name.as_bytes()) }
                .map_err(|_| format!("No {} function found in {}", init_name, path.display()))?;

        // Call the init function
        let module_ptr = unsafe { init_func() };
        if module_ptr.is_null() {
            return Err("Module init function returned NULL".to_string());
        }

        // Convert the C module to a Rust type
        let module_type = self.convert_c_module_to_rust(module_ptr)?;

        // Store the library handle to keep it loaded
        self.handles.insert(module_name.to_string(), lib);

        Ok(module_type)
    }

    /// Convert a C module object to a Rust PyType
    fn convert_c_module_to_rust(
        &self,
        _module_ptr: *mut PyObject_C,
    ) -> Result<Arc<crate::types::PyType>, String> {
        // In a full implementation, this would:
        // 1. Extract the module definition from the C object
        // 2. Create a Rust PyType with the same methods and attributes
        // 3. Set up proper bridging between C and Rust function calls

        // For now, create a basic module type
        let module_type = crate::types::PyType::new("CExtensionModule");
        Ok(Arc::new(module_type))
    }

    /// Check if a module is loaded
    pub fn is_loaded(&self, module_name: &str) -> bool {
        self.handles.contains_key(module_name)
    }

    /// Unload a module
    pub fn unload(&mut self, module_name: &str) -> Result<(), String> {
        self.handles.remove(module_name);
        Ok(())
    }
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Global extension loader instance
static EXTENSION_LOADER: once_cell::sync::Lazy<std::sync::Mutex<ExtensionLoader>> =
    once_cell::sync::Lazy::new(|| std::sync::Mutex::new(ExtensionLoader::new()));

/// Load a C extension module
pub fn load_c_extension(path: &std::path::Path) -> Result<Arc<crate::types::PyType>, String> {
    let mut loader = EXTENSION_LOADER.lock().unwrap();
    loader.load_extension(path)
}

/// Check if a C extension is loaded
pub fn is_c_extension_loaded(module_name: &str) -> bool {
    let loader = EXTENSION_LOADER.lock().unwrap();
    loader.is_loaded(module_name)
}

/// Conversion utilities between Rust and C objects
pub mod conversion {
    use super::*;

    /// Convert a Rust PyValue to a C PyObject
    pub fn pyvalue_to_c_object(_value: &PyValue) -> *mut PyObject_C {
        // In a full implementation, this would:
        // 1. Allocate a C-compatible PyObject structure
        // 2. Set up the reference count and type pointer
        // 3. Copy or reference the data appropriately

        // For now, return null (stub implementation)
        std::ptr::null_mut()
    }

    /// Convert a C PyObject to a Rust PyValue
    pub unsafe fn c_object_to_pyvalue(_obj: *mut PyObject_C) -> Option<PyValue> {
        // In a full implementation, this would:
        // 1. Check the type of the C object
        // 2. Extract the data and convert to appropriate Rust type
        // 3. Handle reference counting properly

        // For now, return None (stub implementation)
        None
    }

    /// Create a C-compatible PyObject from a Rust value
    pub fn create_c_object_from_rust(_value: &PyValue) -> *mut PyObject_C {
        // Stub implementation
        std::ptr::null_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_loader_creation() {
        let loader = ExtensionLoader::new();
        assert!(!loader.is_loaded("test_module"));
    }

    #[test]
    fn test_c_api_structure_sizes() {
        // Ensure structures have reasonable sizes
        assert!(std::mem::size_of::<PyObject_C>() > 0);
        assert!(std::mem::size_of::<PyTypeObject_C>() > 0);
        assert!(std::mem::size_of::<Py_buffer>() > 0);
    }

    #[test]
    fn test_conversion_stubs() {
        let value = PyValue::Int(42);
        let c_obj = conversion::pyvalue_to_c_object(&value);
        assert!(c_obj.is_null()); // Stub implementation returns null

        let rust_val = unsafe { conversion::c_object_to_pyvalue(std::ptr::null_mut()) };
        assert!(rust_val.is_none()); // Stub implementation returns None
    }

    #[test]
    fn test_global_extension_loader() {
        assert!(!is_c_extension_loaded("nonexistent_module"));
    }
}
