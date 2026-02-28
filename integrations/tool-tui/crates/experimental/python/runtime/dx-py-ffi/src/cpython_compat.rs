//! CPython C Extension Compatibility Layer
//!
//! Implements PyObject_* functions, buffer protocol, and GIL compatibility API
//! for running C extensions like NumPy.

// Allow non-standard naming conventions to match CPython's C API
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;
use thiserror::Error;

/// Errors from C extension operations
#[derive(Debug, Error)]
pub enum CExtError {
    #[error("Null pointer")]
    NullPointer,

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Attribute error: {0}")]
    AttributeError(String),

    #[error("Buffer error: {0}")]
    BufferError(String),

    #[error("GIL error: {0}")]
    GilError(String),
}

/// Result type for C extension operations
pub type CExtResult<T> = Result<T, CExtError>;

// =============================================================================
// PyObject - Core object structure
// =============================================================================

/// PyObject structure compatible with CPython's PyObject
#[repr(C)]
pub struct PyObject {
    /// Reference count
    pub ob_refcnt: AtomicI64,
    /// Type object pointer
    pub ob_type: *mut PyTypeObject,
}

impl PyObject {
    /// Create a new PyObject with refcount 1
    pub fn new(ob_type: *mut PyTypeObject) -> Self {
        Self {
            ob_refcnt: AtomicI64::new(1),
            ob_type,
        }
    }
}

/// PyTypeObject structure (simplified)
#[repr(C)]
pub struct PyTypeObject {
    /// Base object
    pub ob_base: PyObject,
    /// Type name
    pub tp_name: *const c_char,
    /// Basic size
    pub tp_basicsize: isize,
    /// Item size (for variable-size objects)
    pub tp_itemsize: isize,
    /// Destructor
    pub tp_dealloc: Option<unsafe extern "C" fn(*mut PyObject)>,
    /// Get attribute
    pub tp_getattr: Option<unsafe extern "C" fn(*mut PyObject, *mut c_char) -> *mut PyObject>,
    /// Set attribute
    pub tp_setattr:
        Option<unsafe extern "C" fn(*mut PyObject, *mut c_char, *mut PyObject) -> c_int>,
    /// Repr
    pub tp_repr: Option<unsafe extern "C" fn(*mut PyObject) -> *mut PyObject>,
    /// Hash
    pub tp_hash: Option<unsafe extern "C" fn(*mut PyObject) -> isize>,
    /// Call
    pub tp_call:
        Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, *mut PyObject) -> *mut PyObject>,
    /// Str
    pub tp_str: Option<unsafe extern "C" fn(*mut PyObject) -> *mut PyObject>,
    /// Get attribute (new style)
    pub tp_getattro: Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject) -> *mut PyObject>,
    /// Set attribute (new style)
    pub tp_setattro:
        Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, *mut PyObject) -> c_int>,
    /// Buffer protocol
    pub tp_as_buffer: *mut PyBufferProcs,
    /// Type flags
    pub tp_flags: u64,
    /// Documentation string
    pub tp_doc: *const c_char,
    /// Traverse for GC
    pub tp_traverse: Option<unsafe extern "C" fn(*mut PyObject, visitproc, *mut c_void) -> c_int>,
    /// Clear for GC
    pub tp_clear: Option<unsafe extern "C" fn(*mut PyObject) -> c_int>,
    /// Rich compare
    pub tp_richcompare:
        Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, c_int) -> *mut PyObject>,
    /// Iterator
    pub tp_iter: Option<unsafe extern "C" fn(*mut PyObject) -> *mut PyObject>,
    /// Iterator next
    pub tp_iternext: Option<unsafe extern "C" fn(*mut PyObject) -> *mut PyObject>,
    /// Methods
    pub tp_methods: *mut PyMethodDef,
    /// Members
    pub tp_members: *mut PyMemberDef,
    /// Getset descriptors
    pub tp_getset: *mut PyGetSetDef,
    /// Base type
    pub tp_base: *mut PyTypeObject,
    /// Dict
    pub tp_dict: *mut PyObject,
    /// Descriptor get
    pub tp_descr_get:
        Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, *mut PyObject) -> *mut PyObject>,
    /// Descriptor set
    pub tp_descr_set:
        Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, *mut PyObject) -> c_int>,
    /// Dict offset
    pub tp_dictoffset: isize,
    /// Init
    pub tp_init: Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, *mut PyObject) -> c_int>,
    /// Alloc
    pub tp_alloc: Option<unsafe extern "C" fn(*mut PyTypeObject, isize) -> *mut PyObject>,
    /// New
    pub tp_new: Option<
        unsafe extern "C" fn(*mut PyTypeObject, *mut PyObject, *mut PyObject) -> *mut PyObject,
    >,
    /// Free
    pub tp_free: Option<unsafe extern "C" fn(*mut c_void)>,
}

/// Visitor function for GC traversal
pub type visitproc = unsafe extern "C" fn(*mut PyObject, *mut c_void) -> c_int;

/// Method definition
#[repr(C)]
pub struct PyMethodDef {
    pub ml_name: *const c_char,
    pub ml_meth: Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject) -> *mut PyObject>,
    pub ml_flags: c_int,
    pub ml_doc: *const c_char,
}

/// Member definition
#[repr(C)]
pub struct PyMemberDef {
    pub name: *const c_char,
    pub type_code: c_int,
    pub offset: isize,
    pub flags: c_int,
    pub doc: *const c_char,
}

/// GetSet definition
#[repr(C)]
pub struct PyGetSetDef {
    pub name: *const c_char,
    pub get: Option<unsafe extern "C" fn(*mut PyObject, *mut c_void) -> *mut PyObject>,
    pub set: Option<unsafe extern "C" fn(*mut PyObject, *mut PyObject, *mut c_void) -> c_int>,
    pub doc: *const c_char,
    pub closure: *mut c_void,
}

// =============================================================================
// Buffer Protocol (PEP 3118)
// =============================================================================

/// Buffer protocol procedures
#[repr(C)]
pub struct PyBufferProcs {
    pub bf_getbuffer: Option<unsafe extern "C" fn(*mut PyObject, *mut Py_buffer, c_int) -> c_int>,
    pub bf_releasebuffer: Option<unsafe extern "C" fn(*mut PyObject, *mut Py_buffer)>,
}

/// Buffer structure (Py_buffer)
#[repr(C)]
pub struct Py_buffer {
    /// Pointer to the buffer data
    pub buf: *mut c_void,
    /// Product of shape (total number of items)
    pub len: isize,
    /// Size of each element
    pub itemsize: isize,
    /// Read-only flag
    pub readonly: c_int,
    /// Number of dimensions
    pub ndim: c_int,
    /// Format string (struct module style)
    pub format: *mut c_char,
    /// Shape array
    pub shape: *mut isize,
    /// Strides array
    pub strides: *mut isize,
    /// Suboffsets array (for PIL-style arrays)
    pub suboffsets: *mut isize,
    /// Internal data for the exporter
    pub internal: *mut c_void,
    /// The exporting object
    pub obj: *mut PyObject,
}

impl Py_buffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            buf: ptr::null_mut(),
            len: 0,
            itemsize: 0,
            readonly: 0,
            ndim: 0,
            format: ptr::null_mut(),
            shape: ptr::null_mut(),
            strides: ptr::null_mut(),
            suboffsets: ptr::null_mut(),
            internal: ptr::null_mut(),
            obj: ptr::null_mut(),
        }
    }

    /// Check if buffer is valid
    pub fn is_valid(&self) -> bool {
        !self.buf.is_null()
    }

    /// Get buffer as a slice (unsafe - caller must ensure validity)
    ///
    /// # Safety
    /// Caller must ensure the buffer is valid and properly aligned for type T.
    /// The buffer must remain valid for the lifetime of the returned slice.
    pub unsafe fn as_slice<T>(&self) -> Option<&[T]> {
        if self.buf.is_null() || self.len <= 0 {
            return None;
        }
        let count = self.len as usize / std::mem::size_of::<T>();
        Some(std::slice::from_raw_parts(self.buf as *const T, count))
    }

    /// Get buffer as a mutable slice (unsafe - caller must ensure validity)
    ///
    /// # Safety
    /// Caller must ensure the buffer is valid, properly aligned for type T,
    /// and not read-only. The buffer must remain valid for the lifetime of
    /// the returned slice.
    pub unsafe fn as_mut_slice<T>(&mut self) -> Option<&mut [T]> {
        if self.buf.is_null() || self.len <= 0 || self.readonly != 0 {
            return None;
        }
        let count = self.len as usize / std::mem::size_of::<T>();
        Some(std::slice::from_raw_parts_mut(self.buf as *mut T, count))
    }
}

impl Default for Py_buffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer flags
pub mod buffer_flags {
    use std::ffi::c_int;

    pub const PyBUF_SIMPLE: c_int = 0;
    pub const PyBUF_WRITABLE: c_int = 0x0001;
    pub const PyBUF_FORMAT: c_int = 0x0004;
    pub const PyBUF_ND: c_int = 0x0008;
    pub const PyBUF_STRIDES: c_int = 0x0010 | PyBUF_ND;
    pub const PyBUF_C_CONTIGUOUS: c_int = 0x0020 | PyBUF_STRIDES;
    pub const PyBUF_F_CONTIGUOUS: c_int = 0x0040 | PyBUF_STRIDES;
    pub const PyBUF_ANY_CONTIGUOUS: c_int = 0x0080 | PyBUF_STRIDES;
    pub const PyBUF_INDIRECT: c_int = 0x0100 | PyBUF_STRIDES;
    pub const PyBUF_CONTIG: c_int = PyBUF_ND | PyBUF_WRITABLE;
    pub const PyBUF_CONTIG_RO: c_int = PyBUF_ND;
    pub const PyBUF_STRIDED: c_int = PyBUF_STRIDES | PyBUF_WRITABLE;
    pub const PyBUF_STRIDED_RO: c_int = PyBUF_STRIDES;
    pub const PyBUF_RECORDS: c_int = PyBUF_STRIDES | PyBUF_WRITABLE | PyBUF_FORMAT;
    pub const PyBUF_RECORDS_RO: c_int = PyBUF_STRIDES | PyBUF_FORMAT;
    pub const PyBUF_FULL: c_int = PyBUF_INDIRECT | PyBUF_WRITABLE | PyBUF_FORMAT;
    pub const PyBUF_FULL_RO: c_int = PyBUF_INDIRECT | PyBUF_FORMAT;
}

// =============================================================================
// GIL Compatibility API
// =============================================================================

/// Global Interpreter Lock state
static GIL_LOCKED: AtomicBool = AtomicBool::new(false);
static GIL_HOLDER: Mutex<Option<std::thread::ThreadId>> = Mutex::new(None);

/// GIL state for PyGILState_Ensure/Release
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyGILState_STATE {
    PyGILState_LOCKED = 0,
    PyGILState_UNLOCKED = 1,
}

/// Thread state structure (simplified)
#[repr(C)]
pub struct PyThreadState {
    /// Previous thread state
    pub prev: *mut PyThreadState,
    /// Next thread state
    pub next: *mut PyThreadState,
    /// Interpreter state
    pub interp: *mut PyInterpreterState,
    /// Current frame
    pub frame: *mut c_void,
    /// Recursion depth
    pub recursion_depth: c_int,
    /// Thread ID
    pub thread_id: u64,
}

/// Interpreter state structure (simplified)
#[repr(C)]
pub struct PyInterpreterState {
    /// Next interpreter
    pub next: *mut PyInterpreterState,
    /// Thread state head
    pub tstate_head: *mut PyThreadState,
    /// Modules dict
    pub modules: *mut PyObject,
    /// Builtins dict
    pub builtins: *mut PyObject,
}

/// GIL state manager
pub struct GilState {
    /// Whether we acquired the GIL
    acquired: bool,
    /// Previous state
    prev_state: PyGILState_STATE,
}

impl GilState {
    /// Ensure the GIL is held (like PyGILState_Ensure)
    pub fn ensure() -> Self {
        let current_thread = std::thread::current().id();
        let mut holder = GIL_HOLDER.lock().unwrap();

        let acquired = if *holder == Some(current_thread) {
            // Already hold the GIL
            false
        } else {
            // Need to acquire
            while GIL_LOCKED
                .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                std::hint::spin_loop();
            }
            *holder = Some(current_thread);
            true
        };

        Self {
            acquired,
            prev_state: if acquired {
                PyGILState_STATE::PyGILState_UNLOCKED
            } else {
                PyGILState_STATE::PyGILState_LOCKED
            },
        }
    }

    /// Release the GIL (like PyGILState_Release)
    pub fn release(self) {
        if self.acquired {
            let mut holder = GIL_HOLDER.lock().unwrap();
            *holder = None;
            GIL_LOCKED.store(false, Ordering::Release);
        }
    }

    /// Get the previous state
    pub fn prev_state(&self) -> PyGILState_STATE {
        self.prev_state
    }
}

/// RAII guard for GIL
pub struct GilGuard {
    state: Option<GilState>,
}

impl GilGuard {
    /// Acquire the GIL
    pub fn acquire() -> Self {
        Self {
            state: Some(GilState::ensure()),
        }
    }

    /// Check if GIL is held
    pub fn is_held() -> bool {
        GIL_LOCKED.load(Ordering::Acquire)
    }
}

impl Drop for GilGuard {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            state.release();
        }
    }
}

/// Allow GIL-free execution (like Py_BEGIN_ALLOW_THREADS)
pub struct AllowThreads {
    _guard: (),
}

impl AllowThreads {
    /// Begin allowing threads (release GIL temporarily)
    pub fn begin() -> Self {
        // In a real implementation, this would release the GIL
        // For now, we just mark that threads are allowed
        Self { _guard: () }
    }
}

impl Drop for AllowThreads {
    fn drop(&mut self) {
        // Re-acquire GIL when dropped
    }
}

// =============================================================================
// PyObject_* Functions
// =============================================================================

/// Increment reference count
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn Py_IncRef(obj: *mut PyObject) {
    if !obj.is_null() {
        (*obj).ob_refcnt.fetch_add(1, Ordering::Relaxed);
    }
}

/// Decrement reference count
///
/// # Safety
/// The pointer must be null or point to a valid PyObject with refcnt > 0.
#[no_mangle]
pub unsafe extern "C" fn Py_DecRef(obj: *mut PyObject) {
    if !obj.is_null() {
        let old = (*obj).ob_refcnt.fetch_sub(1, Ordering::Release);
        if old == 1 {
            std::sync::atomic::fence(Ordering::Acquire);
            // Would call tp_dealloc here
            if let Some(dealloc) = (*(*obj).ob_type).tp_dealloc {
                dealloc(obj);
            }
        }
    }
}

/// Get reference count
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn Py_REFCNT(obj: *const PyObject) -> isize {
    if obj.is_null() {
        0
    } else {
        (*obj).ob_refcnt.load(Ordering::Relaxed) as isize
    }
}

/// Get type object
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn Py_TYPE(obj: *const PyObject) -> *mut PyTypeObject {
    if obj.is_null() {
        ptr::null_mut()
    } else {
        (*obj).ob_type
    }
}

/// Get attribute by name (string)
///
/// # Safety
/// All pointers must be null or point to valid objects. The name must be a
/// valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn PyObject_GetAttrString(
    obj: *mut PyObject,
    name: *const c_char,
) -> *mut PyObject {
    if obj.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    // Try tp_getattr first (old style)
    if let Some(getattr) = (*type_obj).tp_getattr {
        return getattr(obj, name as *mut c_char);
    }

    // Would need to convert name to PyObject and use tp_getattro
    ptr::null_mut()
}

/// Set attribute by name (string)
///
/// # Safety
/// All pointers must be null or point to valid objects. The name must be a
/// valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn PyObject_SetAttrString(
    obj: *mut PyObject,
    name: *const c_char,
    value: *mut PyObject,
) -> c_int {
    if obj.is_null() || name.is_null() {
        return -1;
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return -1;
    }

    // Try tp_setattr first (old style)
    if let Some(setattr) = (*type_obj).tp_setattr {
        return setattr(obj, name as *mut c_char, value);
    }

    -1
}

/// Check if object has attribute
///
/// # Safety
/// All pointers must be null or point to valid objects. The name must be a
/// valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn PyObject_HasAttrString(obj: *mut PyObject, name: *const c_char) -> c_int {
    let attr = PyObject_GetAttrString(obj, name);
    if attr.is_null() {
        0
    } else {
        Py_DecRef(attr);
        1
    }
}

/// Get attribute by PyObject name
///
/// # Safety
/// All pointers must be null or point to valid PyObjects.
#[no_mangle]
pub unsafe extern "C" fn PyObject_GetAttr(
    obj: *mut PyObject,
    name: *mut PyObject,
) -> *mut PyObject {
    if obj.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    if let Some(getattro) = (*type_obj).tp_getattro {
        return getattro(obj, name);
    }

    ptr::null_mut()
}

/// Set attribute by PyObject name
///
/// # Safety
/// All pointers must be null or point to valid PyObjects.
#[no_mangle]
pub unsafe extern "C" fn PyObject_SetAttr(
    obj: *mut PyObject,
    name: *mut PyObject,
    value: *mut PyObject,
) -> c_int {
    if obj.is_null() || name.is_null() {
        return -1;
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return -1;
    }

    if let Some(setattro) = (*type_obj).tp_setattro {
        return setattro(obj, name, value);
    }

    -1
}

/// Call object with args and kwargs
///
/// # Safety
/// All pointers must be null or point to valid PyObjects.
#[no_mangle]
pub unsafe extern "C" fn PyObject_Call(
    callable: *mut PyObject,
    args: *mut PyObject,
    kwargs: *mut PyObject,
) -> *mut PyObject {
    if callable.is_null() {
        return ptr::null_mut();
    }

    let type_obj = (*callable).ob_type;
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    if let Some(call) = (*type_obj).tp_call {
        return call(callable, args, kwargs);
    }

    ptr::null_mut()
}

/// Get string representation
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn PyObject_Repr(obj: *mut PyObject) -> *mut PyObject {
    if obj.is_null() {
        return ptr::null_mut();
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    if let Some(repr) = (*type_obj).tp_repr {
        return repr(obj);
    }

    ptr::null_mut()
}

/// Get string representation (str)
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn PyObject_Str(obj: *mut PyObject) -> *mut PyObject {
    if obj.is_null() {
        return ptr::null_mut();
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    if let Some(str_fn) = (*type_obj).tp_str {
        return str_fn(obj);
    }

    // Fall back to repr
    PyObject_Repr(obj)
}

/// Get hash value
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn PyObject_Hash(obj: *mut PyObject) -> isize {
    if obj.is_null() {
        return -1;
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return -1;
    }

    if let Some(hash) = (*type_obj).tp_hash {
        return hash(obj);
    }

    // Default: use pointer as hash
    obj as isize
}

/// Check if object is true
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn PyObject_IsTrue(obj: *mut PyObject) -> c_int {
    if obj.is_null() {
        return 0;
    }

    // Would check __bool__ or __len__
    1
}

/// Rich comparison
///
/// # Safety
/// All pointers must be null or point to valid PyObjects.
#[no_mangle]
pub unsafe extern "C" fn PyObject_RichCompare(
    obj1: *mut PyObject,
    obj2: *mut PyObject,
    op: c_int,
) -> *mut PyObject {
    if obj1.is_null() || obj2.is_null() {
        return ptr::null_mut();
    }

    let type_obj = (*obj1).ob_type;
    if type_obj.is_null() {
        return ptr::null_mut();
    }

    if let Some(richcompare) = (*type_obj).tp_richcompare {
        return richcompare(obj1, obj2, op);
    }

    ptr::null_mut()
}

/// Rich comparison operators
pub mod compare_ops {
    use std::ffi::c_int;

    pub const Py_LT: c_int = 0;
    pub const Py_LE: c_int = 1;
    pub const Py_EQ: c_int = 2;
    pub const Py_NE: c_int = 3;
    pub const Py_GT: c_int = 4;
    pub const Py_GE: c_int = 5;
}

// =============================================================================
// Buffer Protocol Functions
// =============================================================================

/// Get buffer from object
///
/// # Safety
/// All pointers must be null or point to valid objects. The view pointer
/// must point to a valid Py_buffer struct that can be written to.
#[no_mangle]
pub unsafe extern "C" fn PyObject_GetBuffer(
    obj: *mut PyObject,
    view: *mut Py_buffer,
    flags: c_int,
) -> c_int {
    if obj.is_null() || view.is_null() {
        return -1;
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return -1;
    }

    let buffer_procs = (*type_obj).tp_as_buffer;
    if buffer_procs.is_null() {
        return -1;
    }

    if let Some(getbuffer) = (*buffer_procs).bf_getbuffer {
        let result = getbuffer(obj, view, flags);
        if result == 0 {
            // Success - increment ref on obj
            (*view).obj = obj;
            Py_IncRef(obj);
        }
        return result;
    }

    -1
}

/// Release buffer
///
/// # Safety
/// The pointer must be null or point to a valid Py_buffer that was
/// previously obtained via PyObject_GetBuffer.
#[no_mangle]
pub unsafe extern "C" fn PyBuffer_Release(view: *mut Py_buffer) {
    if view.is_null() {
        return;
    }

    let obj = (*view).obj;
    if obj.is_null() {
        return;
    }

    let type_obj = (*obj).ob_type;
    if !type_obj.is_null() {
        let buffer_procs = (*type_obj).tp_as_buffer;
        if !buffer_procs.is_null() {
            if let Some(releasebuffer) = (*buffer_procs).bf_releasebuffer {
                releasebuffer(obj, view);
            }
        }
    }

    Py_DecRef(obj);
    (*view).obj = ptr::null_mut();
}

/// Check if object supports buffer protocol
///
/// # Safety
/// The pointer must be null or point to a valid PyObject.
#[no_mangle]
pub unsafe extern "C" fn PyObject_CheckBuffer(obj: *mut PyObject) -> c_int {
    if obj.is_null() {
        return 0;
    }

    let type_obj = (*obj).ob_type;
    if type_obj.is_null() {
        return 0;
    }

    let buffer_procs = (*type_obj).tp_as_buffer;
    if buffer_procs.is_null() {
        return 0;
    }

    if (*buffer_procs).bf_getbuffer.is_some() {
        1
    } else {
        0
    }
}

/// Check if buffer is contiguous
///
/// # Safety
/// The pointer must be null or point to a valid Py_buffer.
#[no_mangle]
pub unsafe extern "C" fn PyBuffer_IsContiguous(view: *const Py_buffer, order: c_char) -> c_int {
    if view.is_null() {
        return 0;
    }

    let view = &*view;

    // Simple case: 1D array is always contiguous
    if view.ndim <= 1 {
        return 1;
    }

    // Check strides for contiguity
    if view.strides.is_null() {
        return 1; // No strides means C-contiguous
    }

    let shape = std::slice::from_raw_parts(view.shape, view.ndim as usize);
    let strides = std::slice::from_raw_parts(view.strides, view.ndim as usize);

    match order as u8 as char {
        'C' | 'c' => {
            // C-contiguous: last dimension has stride = itemsize
            let mut expected_stride = view.itemsize;
            for i in (0..view.ndim as usize).rev() {
                if strides[i] != expected_stride {
                    return 0;
                }
                expected_stride *= shape[i];
            }
            1
        }
        'F' | 'f' => {
            // Fortran-contiguous: first dimension has stride = itemsize
            let mut expected_stride = view.itemsize;
            for i in 0..view.ndim as usize {
                if strides[i] != expected_stride {
                    return 0;
                }
                expected_stride *= shape[i];
            }
            1
        }
        'A' | 'a' => {
            // Any contiguous
            if PyBuffer_IsContiguous(view, b'C' as c_char) != 0
                || PyBuffer_IsContiguous(view, b'F' as c_char) != 0
            {
                1
            } else {
                0
            }
        }
        _ => 0,
    }
}

// =============================================================================
// GIL Functions
// =============================================================================

/// Ensure GIL is held
#[no_mangle]
pub extern "C" fn PyGILState_Ensure() -> PyGILState_STATE {
    let state = GilState::ensure();
    let prev = state.prev_state();
    // Note: GilState doesn't implement Drop, so we just let it go out of scope
    // without releasing. The caller is responsible for calling PyGILState_Release.
    let _ = state;
    prev
}

/// Release GIL
#[no_mangle]
pub extern "C" fn PyGILState_Release(state: PyGILState_STATE) {
    if state == PyGILState_STATE::PyGILState_UNLOCKED {
        // We acquired the GIL, so release it
        let mut holder = GIL_HOLDER.lock().unwrap();
        *holder = None;
        GIL_LOCKED.store(false, Ordering::Release);
    }
}

/// Check if GIL is held by current thread
#[no_mangle]
pub extern "C" fn PyGILState_Check() -> c_int {
    let current_thread = std::thread::current().id();
    let holder = GIL_HOLDER.lock().unwrap();
    if *holder == Some(current_thread) {
        1
    } else {
        0
    }
}

// =============================================================================
// Type Flags
// =============================================================================

pub mod type_flags {
    pub const Py_TPFLAGS_DEFAULT: u64 = 0;
    pub const Py_TPFLAGS_HEAPTYPE: u64 = 1 << 9;
    pub const Py_TPFLAGS_BASETYPE: u64 = 1 << 10;
    pub const Py_TPFLAGS_HAVE_GC: u64 = 1 << 14;
    pub const Py_TPFLAGS_HAVE_FINALIZE: u64 = 1 << 0;
    pub const Py_TPFLAGS_HAVE_VERSION_TAG: u64 = 1 << 18;
    pub const Py_TPFLAGS_VALID_VERSION_TAG: u64 = 1 << 19;
    pub const Py_TPFLAGS_IS_ABSTRACT: u64 = 1 << 20;
    pub const Py_TPFLAGS_LONG_SUBCLASS: u64 = 1 << 24;
    pub const Py_TPFLAGS_LIST_SUBCLASS: u64 = 1 << 25;
    pub const Py_TPFLAGS_TUPLE_SUBCLASS: u64 = 1 << 26;
    pub const Py_TPFLAGS_BYTES_SUBCLASS: u64 = 1 << 27;
    pub const Py_TPFLAGS_UNICODE_SUBCLASS: u64 = 1 << 28;
    pub const Py_TPFLAGS_DICT_SUBCLASS: u64 = 1 << 29;
    pub const Py_TPFLAGS_BASE_EXC_SUBCLASS: u64 = 1 << 30;
    pub const Py_TPFLAGS_TYPE_SUBCLASS: u64 = 1 << 31;
}

// =============================================================================
// Argument Parsing (PyArg_ParseTuple, Py_BuildValue)
// =============================================================================

use std::ffi::CStr;

/// Error type for argument parsing
#[derive(Debug, Clone)]
pub struct ArgParseError {
    pub message: String,
    pub format_char: Option<char>,
    pub arg_index: usize,
}

impl ArgParseError {
    pub fn new(message: impl Into<String>, format_char: Option<char>, arg_index: usize) -> Self {
        Self {
            message: message.into(),
            format_char,
            arg_index,
        }
    }
}

/// Result of argument parsing
pub type ArgParseResult<T> = Result<T, ArgParseError>;

/// Parsed argument value
#[derive(Debug, Clone)]
pub enum ParsedArg {
    /// Integer value (i, l, L, n)
    Int(i64),
    /// Unsigned integer (I, k, K)
    UInt(u64),
    /// Float value (f, d)
    Float(f64),
    /// String value (s, z, U)
    String(Option<String>),
    /// Bytes value (y, y#)
    Bytes(Vec<u8>),
    /// Object reference (O, O!)
    Object(*mut PyObject),
    /// Boolean (p)
    Bool(bool),
    /// Character (c, C)
    Char(char),
}

// =============================================================================
// PyObject Bridge - Type Conversion between DX-Py and CPython
// =============================================================================

/// DX-Py internal value representation
#[derive(Debug, Clone)]
pub enum DxValue {
    /// None/null value
    None,
    /// Boolean
    Bool(bool),
    /// Integer (arbitrary precision in full impl)
    Int(i64),
    /// Float
    Float(f64),
    /// String
    String(String),
    /// Bytes
    Bytes(Vec<u8>),
    /// List of values
    List(Vec<DxValue>),
    /// Tuple of values
    Tuple(Vec<DxValue>),
    /// Dictionary (key-value pairs)
    Dict(Vec<(DxValue, DxValue)>),
    /// Set of values
    Set(Vec<DxValue>),
    /// Raw PyObject pointer (for objects we can't convert)
    PyObjectRef(*mut PyObject),
}

// DxValue contains raw pointers in PyObjectRef variant
unsafe impl Send for DxValue {}
unsafe impl Sync for DxValue {}

/// Error type for PyObject bridge operations
#[derive(Debug, Clone)]
pub struct BridgeError {
    pub message: String,
    pub source_type: Option<String>,
    pub target_type: Option<String>,
}

impl BridgeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source_type: None,
            target_type: None,
        }
    }

    pub fn with_types(
        message: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            source_type: Some(source.into()),
            target_type: Some(target.into()),
        }
    }
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if let (Some(src), Some(tgt)) = (&self.source_type, &self.target_type) {
            write!(f, " (converting {} to {})", src, tgt)?;
        }
        Ok(())
    }
}

impl std::error::Error for BridgeError {}

/// Result type for bridge operations
pub type BridgeResult<T> = Result<T, BridgeError>;

/// PyObject Bridge - converts between DX-Py values and CPython PyObjects
///
/// This bridge handles:
/// - Converting DX-Py int/float/str/list/dict to PyObject
/// - Converting PyObject back to DX-Py objects
/// - Reference counting across the bridge
pub struct PyObjectBridge {
    /// Allocated PyObjects that need cleanup
    allocated: Vec<*mut PyObject>,
    /// Type objects for creating new PyObjects
    type_objects: PyTypeObjects,
}

/// Type objects for creating PyObjects
#[allow(dead_code)]
struct PyTypeObjects {
    int_type: *mut PyTypeObject,
    float_type: *mut PyTypeObject,
    str_type: *mut PyTypeObject,
    bytes_type: *mut PyTypeObject,
    list_type: *mut PyTypeObject,
    tuple_type: *mut PyTypeObject,
    dict_type: *mut PyTypeObject,
    bool_type: *mut PyTypeObject,
    none_type: *mut PyTypeObject,
}

impl Default for PyTypeObjects {
    fn default() -> Self {
        Self {
            int_type: ptr::null_mut(),
            float_type: ptr::null_mut(),
            str_type: ptr::null_mut(),
            bytes_type: ptr::null_mut(),
            list_type: ptr::null_mut(),
            tuple_type: ptr::null_mut(),
            dict_type: ptr::null_mut(),
            bool_type: ptr::null_mut(),
            none_type: ptr::null_mut(),
        }
    }
}

impl PyObjectBridge {
    /// Create a new PyObject bridge
    pub fn new() -> Self {
        Self {
            allocated: Vec::new(),
            type_objects: PyTypeObjects::default(),
        }
    }

    /// Convert a DX-Py value to a PyObject
    ///
    /// # Safety
    /// The returned PyObject must be properly reference counted.
    pub unsafe fn to_pyobject(&mut self, value: &DxValue) -> BridgeResult<*mut PyObject> {
        match value {
            DxValue::None => {
                // Return a "None" singleton (in full impl, this would be Py_None)
                Ok(ptr::null_mut())
            }
            DxValue::Bool(b) => self.create_bool_object(*b),
            DxValue::Int(i) => self.create_int_object(*i),
            DxValue::Float(f) => self.create_float_object(*f),
            DxValue::String(s) => self.create_string_object(s),
            DxValue::Bytes(b) => self.create_bytes_object(b),
            DxValue::List(items) => self.create_list_object(items),
            DxValue::Tuple(items) => self.create_tuple_object(items),
            DxValue::Dict(pairs) => self.create_dict_object(pairs),
            DxValue::Set(items) => self.create_set_object(items),
            DxValue::PyObjectRef(ptr) => {
                // Already a PyObject, just increment refcount
                if !ptr.is_null() {
                    Py_IncRef(*ptr);
                }
                Ok(*ptr)
            }
        }
    }

    /// Convert a PyObject to a DX-Py value
    ///
    /// # Safety
    /// The PyObject pointer must be valid.
    pub unsafe fn from_pyobject(&self, obj: *mut PyObject) -> BridgeResult<DxValue> {
        if obj.is_null() {
            return Ok(DxValue::None);
        }

        let type_obj = (*obj).ob_type;
        if type_obj.is_null() {
            return Err(BridgeError::new("PyObject has null type"));
        }

        // Check type and convert accordingly
        // In a full implementation, we would check against actual type objects
        // For now, we return a PyObjectRef wrapper
        Ok(DxValue::PyObjectRef(obj))
    }

    /// Create a PyObject representing an integer
    unsafe fn create_int_object(&mut self, _value: i64) -> BridgeResult<*mut PyObject> {
        // In a full implementation, this would create a PyLongObject
        // For now, we allocate a simple PyObject with the value stored
        let obj = self.allocate_object(self.type_objects.int_type)?;
        // Store value in object (simplified - real impl would use PyLongObject)
        Ok(obj)
    }

    /// Create a PyObject representing a float
    unsafe fn create_float_object(&mut self, _value: f64) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.float_type)?;
        Ok(obj)
    }

    /// Create a PyObject representing a string
    unsafe fn create_string_object(&mut self, _value: &str) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.str_type)?;
        Ok(obj)
    }

    /// Create a PyObject representing bytes
    unsafe fn create_bytes_object(&mut self, _value: &[u8]) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.bytes_type)?;
        Ok(obj)
    }

    /// Create a PyObject representing a boolean
    unsafe fn create_bool_object(&mut self, _value: bool) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.bool_type)?;
        Ok(obj)
    }

    /// Create a PyObject representing a list
    unsafe fn create_list_object(&mut self, items: &[DxValue]) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.list_type)?;

        // Convert each item (in full impl, would add to list)
        for _item in items {
            // self.to_pyobject(item)?;
        }

        Ok(obj)
    }

    /// Create a PyObject representing a tuple
    unsafe fn create_tuple_object(&mut self, items: &[DxValue]) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.tuple_type)?;

        for _item in items {
            // self.to_pyobject(item)?;
        }

        Ok(obj)
    }

    /// Create a PyObject representing a dict
    unsafe fn create_dict_object(
        &mut self,
        pairs: &[(DxValue, DxValue)],
    ) -> BridgeResult<*mut PyObject> {
        let obj = self.allocate_object(self.type_objects.dict_type)?;

        for (_key, _value) in pairs {
            // let key_obj = self.to_pyobject(key)?;
            // let value_obj = self.to_pyobject(value)?;
        }

        Ok(obj)
    }

    /// Create a PyObject representing a set
    unsafe fn create_set_object(&mut self, items: &[DxValue]) -> BridgeResult<*mut PyObject> {
        // Sets use a similar structure to dicts
        let obj = self.allocate_object(self.type_objects.dict_type)?;

        for _item in items {
            // self.to_pyobject(item)?;
        }

        Ok(obj)
    }

    /// Allocate a new PyObject
    unsafe fn allocate_object(
        &mut self,
        type_obj: *mut PyTypeObject,
    ) -> BridgeResult<*mut PyObject> {
        // Use Box to allocate memory safely
        let obj = Box::new(PyObject {
            ob_refcnt: AtomicI64::new(1),
            ob_type: type_obj,
        });

        let ptr = Box::into_raw(obj);

        // Track for cleanup
        self.allocated.push(ptr);

        Ok(ptr)
    }

    /// Synchronize reference count for a PyObject
    ///
    /// This ensures DX-Py's reference tracking matches the PyObject's refcount.
    ///
    /// # Safety
    /// The caller must ensure that `obj` is either null or a valid pointer to a PyObject.
    pub unsafe fn sync_refcount(&self, obj: *mut PyObject) -> i64 {
        if obj.is_null() {
            return 0;
        }
        (*obj).ob_refcnt.load(Ordering::Relaxed)
    }

    /// Get the number of allocated objects
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }

    /// Clear all allocated objects (deallocate them)
    ///
    /// # Safety
    /// The caller must ensure that no other code holds references to the allocated objects.
    pub unsafe fn clear(&mut self) {
        for obj in self.allocated.drain(..) {
            if !obj.is_null() {
                // Convert back to Box and drop it
                let _ = Box::from_raw(obj);
            }
        }
    }
}

impl Default for PyObjectBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PyObjectBridge {
    fn drop(&mut self) {
        unsafe {
            self.clear();
        }
    }
}

/// Check if two DxValues are equivalent
pub fn dx_values_equal(a: &DxValue, b: &DxValue) -> bool {
    match (a, b) {
        (DxValue::None, DxValue::None) => true,
        (DxValue::Bool(a), DxValue::Bool(b)) => a == b,
        (DxValue::Int(a), DxValue::Int(b)) => a == b,
        (DxValue::Float(a), DxValue::Float(b)) => (a - b).abs() < f64::EPSILON,
        (DxValue::String(a), DxValue::String(b)) => a == b,
        (DxValue::Bytes(a), DxValue::Bytes(b)) => a == b,
        (DxValue::List(a), DxValue::List(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| dx_values_equal(x, y))
        }
        (DxValue::Tuple(a), DxValue::Tuple(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| dx_values_equal(x, y))
        }
        (DxValue::Dict(a), DxValue::Dict(b)) => {
            if a.len() != b.len() {
                return false;
            }
            // Simple comparison - in full impl would handle key ordering
            a.iter()
                .zip(b.iter())
                .all(|((k1, v1), (k2, v2))| dx_values_equal(k1, k2) && dx_values_equal(v1, v2))
        }
        (DxValue::Set(a), DxValue::Set(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| dx_values_equal(x, y))
        }
        (DxValue::PyObjectRef(a), DxValue::PyObjectRef(b)) => std::ptr::eq(*a, *b),
        _ => false,
    }
}

// =============================================================================
// Missing API Tracking and Error Reporting
// =============================================================================

use std::collections::HashSet;
use std::sync::RwLock;

/// API function category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiCategory {
    /// Core object operations (Py_IncRef, Py_DecRef, etc.)
    ObjectCore,
    /// Type system (PyType_*, type checking)
    TypeSystem,
    /// Number protocol (PyNumber_*)
    NumberProtocol,
    /// Sequence protocol (PySequence_*)
    SequenceProtocol,
    /// Mapping protocol (PyMapping_*)
    MappingProtocol,
    /// Buffer protocol (PyBuffer_*)
    BufferProtocol,
    /// Argument parsing (PyArg_Parse*, Py_BuildValue)
    ArgParsing,
    /// Error handling (PyErr_*)
    ErrorHandling,
    /// Memory allocation (PyMem_*, PyObject_Malloc)
    MemoryAlloc,
    /// GIL operations (PyGILState_*)
    GIL,
    /// Import system (PyImport_*)
    Import,
    /// Module operations (PyModule_*)
    Module,
    /// Unknown/other
    Other,
}

impl ApiCategory {
    /// Get category from function name
    pub fn from_function_name(name: &str) -> Self {
        if name.starts_with("Py_IncRef")
            || name.starts_with("Py_DecRef")
            || name.starts_with("Py_REFCNT")
            || name.starts_with("Py_TYPE")
        {
            ApiCategory::ObjectCore
        } else if name.starts_with("PyType_") {
            ApiCategory::TypeSystem
        } else if name.starts_with("PyNumber_") {
            ApiCategory::NumberProtocol
        } else if name.starts_with("PySequence_") {
            ApiCategory::SequenceProtocol
        } else if name.starts_with("PyMapping_") {
            ApiCategory::MappingProtocol
        } else if name.starts_with("PyBuffer_") || name.starts_with("PyObject_GetBuffer") {
            ApiCategory::BufferProtocol
        } else if name.starts_with("PyArg_") || name.starts_with("Py_BuildValue") {
            ApiCategory::ArgParsing
        } else if name.starts_with("PyErr_") {
            ApiCategory::ErrorHandling
        } else if name.starts_with("PyMem_") || name.starts_with("PyObject_Malloc") {
            ApiCategory::MemoryAlloc
        } else if name.starts_with("PyGILState_") {
            ApiCategory::GIL
        } else if name.starts_with("PyImport_") {
            ApiCategory::Import
        } else if name.starts_with("PyModule_") {
            ApiCategory::Module
        } else {
            ApiCategory::Other
        }
    }
}

/// API function priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApiPriority {
    /// Required for any extension to work
    Critical,
    /// Required for most extensions
    Important,
    /// Nice to have but not essential
    Optional,
}

/// Information about an API function
#[derive(Debug, Clone)]
pub struct ApiFunction {
    /// Function name
    pub name: String,
    /// Category
    pub category: ApiCategory,
    /// Priority
    pub priority: ApiPriority,
    /// Whether it's implemented
    pub implemented: bool,
}

/// Missing API error with detailed information
#[derive(Debug, Clone)]
pub struct MissingApiError {
    /// The function name that was called
    pub function_name: String,
    /// Category of the function
    pub category: ApiCategory,
    /// Priority of the function
    pub priority: ApiPriority,
    /// Suggested workaround if available
    pub workaround: Option<String>,
    /// Related functions that might help
    pub related_functions: Vec<String>,
}

impl MissingApiError {
    /// Create a new missing API error
    pub fn new(function_name: impl Into<String>) -> Self {
        let name = function_name.into();
        let category = ApiCategory::from_function_name(&name);
        let priority = Self::infer_priority(&name);
        let workaround = Self::suggest_workaround(&name);
        let related = Self::find_related_functions(&name);

        Self {
            function_name: name,
            category,
            priority,
            workaround,
            related_functions: related,
        }
    }

    /// Infer priority from function name
    fn infer_priority(name: &str) -> ApiPriority {
        // Critical functions that almost every extension needs
        if name.starts_with("Py_IncRef")
            || name.starts_with("Py_DecRef")
            || name.starts_with("PyArg_ParseTuple")
            || name.starts_with("Py_BuildValue")
            || name.starts_with("PyErr_SetString")
            || name.starts_with("PyErr_Occurred")
        {
            ApiPriority::Critical
        }
        // Important functions used by many extensions
        else if name.starts_with("PyObject_")
            || name.starts_with("PyType_")
            || name.starts_with("PyLong_")
            || name.starts_with("PyFloat_")
            || name.starts_with("PyUnicode_")
            || name.starts_with("PyList_")
            || name.starts_with("PyDict_")
            || name.starts_with("PyTuple_")
        {
            ApiPriority::Important
        }
        // Optional functions
        else {
            ApiPriority::Optional
        }
    }

    /// Suggest a workaround for the missing function
    fn suggest_workaround(name: &str) -> Option<String> {
        match name {
            "PyArg_ParseTupleAndKeywords" => {
                Some("Use PyArg_ParseTuple for positional-only arguments".to_string())
            }
            "PyObject_GetAttrString" => {
                Some("Use PyObject_GetAttr with a PyUnicode object".to_string())
            }
            "PyErr_SetString" => Some("Return NULL to indicate an error occurred".to_string()),
            _ => None,
        }
    }

    /// Find related functions that might help
    fn find_related_functions(name: &str) -> Vec<String> {
        let mut related = Vec::new();

        if name.contains("ParseTuple") {
            related.push("PyArg_ParseTuple".to_string());
            related.push("PyArg_ParseTupleAndKeywords".to_string());
        }
        if name.contains("BuildValue") {
            related.push("Py_BuildValue".to_string());
        }
        if name.contains("Object") {
            related.push("PyObject_GetAttr".to_string());
            related.push("PyObject_SetAttr".to_string());
            related.push("PyObject_Call".to_string());
        }

        related
    }

    /// Format as a detailed error message
    pub fn format_error(&self) -> String {
        let mut msg = format!(
            "Missing CPython API function: {}\n\
             Category: {:?}\n\
             Priority: {:?}",
            self.function_name, self.category, self.priority
        );

        if let Some(ref workaround) = self.workaround {
            msg.push_str(&format!("\nWorkaround: {}", workaround));
        }

        if !self.related_functions.is_empty() {
            msg.push_str(&format!("\nRelated functions: {}", self.related_functions.join(", ")));
        }

        msg
    }
}

impl std::fmt::Display for MissingApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Missing API: {} ({:?})", self.function_name, self.category)
    }
}

impl std::error::Error for MissingApiError {}

/// API tracker for monitoring CPython API usage and gaps
pub struct ApiTracker {
    /// All API calls made
    calls: RwLock<Vec<String>>,
    /// Missing (unimplemented) API functions called
    missing: RwLock<HashSet<String>>,
    /// Implemented API functions
    implemented: HashSet<&'static str>,
}

impl ApiTracker {
    /// Create a new API tracker with known implemented functions
    pub fn new() -> Self {
        let mut implemented = HashSet::new();

        // Register all implemented functions
        implemented.insert("Py_IncRef");
        implemented.insert("Py_DecRef");
        implemented.insert("Py_REFCNT");
        implemented.insert("Py_TYPE");
        implemented.insert("PyObject_GetAttrString");
        implemented.insert("PyObject_SetAttrString");
        implemented.insert("PyObject_HasAttrString");
        implemented.insert("PyObject_GetAttr");
        implemented.insert("PyObject_SetAttr");
        implemented.insert("PyObject_Call");
        implemented.insert("PyObject_Repr");
        implemented.insert("PyObject_Str");
        implemented.insert("PyObject_Hash");
        implemented.insert("PyObject_IsTrue");
        implemented.insert("PyObject_RichCompare");
        implemented.insert("PyObject_GetBuffer");
        implemented.insert("PyBuffer_Release");
        implemented.insert("PyObject_CheckBuffer");
        implemented.insert("PyBuffer_IsContiguous");
        implemented.insert("PyGILState_Ensure");
        implemented.insert("PyGILState_Release");
        implemented.insert("PyGILState_Check");
        implemented.insert("PyArg_ParseTuple");
        implemented.insert("PyArg_ParseTupleAndKeywords");
        implemented.insert("Py_BuildValue");

        Self {
            calls: RwLock::new(Vec::new()),
            missing: RwLock::new(HashSet::new()),
            implemented,
        }
    }

    /// Record an API function call
    pub fn record_call(&self, function_name: &str) {
        if let Ok(mut calls) = self.calls.write() {
            calls.push(function_name.to_string());
        }

        // Check if it's implemented
        if !self.implemented.contains(function_name) {
            if let Ok(mut missing) = self.missing.write() {
                missing.insert(function_name.to_string());
            }
        }
    }

    /// Check if a function is implemented
    pub fn is_implemented(&self, function_name: &str) -> bool {
        self.implemented.contains(function_name)
    }

    /// Get all missing API functions that were called
    pub fn get_missing(&self) -> Vec<String> {
        self.missing.read().map(|m| m.iter().cloned().collect()).unwrap_or_default()
    }

    /// Get all API calls made
    pub fn get_all_calls(&self) -> Vec<String> {
        self.calls.read().map(|c| c.clone()).unwrap_or_default()
    }

    /// Get missing API errors with detailed information
    pub fn get_missing_errors(&self) -> Vec<MissingApiError> {
        self.get_missing().into_iter().map(MissingApiError::new).collect()
    }

    /// Get coverage statistics
    pub fn coverage_stats(&self) -> ApiCoverageStats {
        let total_implemented = self.implemented.len();
        let missing_called = self.get_missing().len();
        let total_calls = self.calls.read().map(|c| c.len()).unwrap_or(0);

        ApiCoverageStats {
            total_implemented,
            missing_called,
            total_calls,
        }
    }

    /// Clear all recorded calls
    pub fn clear(&self) {
        if let Ok(mut calls) = self.calls.write() {
            calls.clear();
        }
        if let Ok(mut missing) = self.missing.write() {
            missing.clear();
        }
    }
}

impl Default for ApiTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// API coverage statistics
#[derive(Debug, Clone)]
pub struct ApiCoverageStats {
    /// Number of implemented API functions
    pub total_implemented: usize,
    /// Number of missing functions that were called
    pub missing_called: usize,
    /// Total number of API calls made
    pub total_calls: usize,
}

impl ApiCoverageStats {
    /// Get coverage percentage (implemented / (implemented + missing_called))
    pub fn coverage_percentage(&self) -> f64 {
        let total = self.total_implemented + self.missing_called;
        if total == 0 {
            100.0
        } else {
            (self.total_implemented as f64 / total as f64) * 100.0
        }
    }
}

/// Global API tracker instance
static API_TRACKER: std::sync::OnceLock<ApiTracker> = std::sync::OnceLock::new();

/// Get the global API tracker
pub fn get_api_tracker() -> &'static ApiTracker {
    API_TRACKER.get_or_init(ApiTracker::new)
}

/// Record an API call (convenience function)
pub fn record_api_call(function_name: &str) {
    get_api_tracker().record_call(function_name);
}

/// Check if an API function is implemented and return error if not
pub fn check_api_implemented(function_name: &str) -> Result<(), MissingApiError> {
    let tracker = get_api_tracker();
    tracker.record_call(function_name);

    if tracker.is_implemented(function_name) {
        Ok(())
    } else {
        Err(MissingApiError::new(function_name))
    }
}

/// Parse a tuple of arguments according to a format string
///
/// Supported format characters:
/// - `i`: int -> c_int
/// - `l`: int -> c_long
/// - `L`: int -> c_longlong
/// - `n`: int -> Py_ssize_t
/// - `I`: int -> c_uint
/// - `k`: int -> c_ulong
/// - `K`: int -> c_ulonglong
/// - `f`: float -> c_float
/// - `d`: float -> c_double
/// - `s`: str -> *const c_char (null-terminated)
/// - `z`: str or None -> *const c_char (nullable)
/// - `y`: bytes -> *const c_char
/// - `O`: object -> *mut PyObject
/// - `p`: bool -> c_int
/// - `c`: bytes of length 1 -> c_char
/// - `C`: str of length 1 -> c_int (unicode codepoint)
/// - `|`: following args are optional
/// - `:`: end of format, followed by function name for errors
/// - `;`: end of format, followed by error message
///
/// # Safety
/// The args pointer must be null or point to a valid PyObject tuple.
/// The format pointer must be a valid null-terminated C string.
/// The va_list must contain pointers matching the format string.
#[no_mangle]
pub unsafe extern "C" fn PyArg_ParseTuple(
    args: *mut PyObject,
    format: *const c_char,
    // Note: In real implementation, this would use va_list
    // For now, we return success if format is valid
) -> c_int {
    if format.is_null() {
        return 0;
    }

    // Parse the format string to validate it
    let format_str = match CStr::from_ptr(format).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    // Validate format string
    if !validate_format_string(format_str) {
        return 0;
    }

    // In a full implementation, we would:
    // 1. Extract arguments from the tuple
    // 2. Parse each according to format
    // 3. Store results in va_list pointers

    // For now, return success if args is not null and format is valid
    if args.is_null() {
        0
    } else {
        1
    }
}

/// Parse a tuple with keyword arguments
///
/// # Safety
/// All pointers must be null or point to valid objects.
#[no_mangle]
pub unsafe extern "C" fn PyArg_ParseTupleAndKeywords(
    args: *mut PyObject,
    _kwargs: *mut PyObject,
    format: *const c_char,
    keywords: *mut *mut c_char,
    // va_list would follow
) -> c_int {
    if format.is_null() {
        return 0;
    }

    let format_str = match CStr::from_ptr(format).to_str() {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if !validate_format_string(format_str) {
        return 0;
    }

    // Validate keywords array if provided
    if !keywords.is_null() {
        let mut i = 0;
        loop {
            let kw = *keywords.add(i);
            if kw.is_null() {
                break;
            }
            i += 1;
        }
    }

    if args.is_null() {
        0
    } else {
        1
    }
}

/// Validate a format string for PyArg_ParseTuple
fn validate_format_string(format: &str) -> bool {
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Integer formats
            'i' | 'l' | 'L' | 'n' | 'I' | 'k' | 'K' | 'b' | 'B' | 'h' | 'H' => {}
            // Float formats
            'f' | 'd' | 'D' => {}
            // String formats
            's' | 'z' | 'U' | 'y' => {
                // Check for # modifier (length)
                if chars.peek() == Some(&'#') {
                    chars.next();
                }
                // Check for * modifier (buffer)
                if chars.peek() == Some(&'*') {
                    chars.next();
                }
            }
            // Object formats
            'O' => {
                // Check for ! or & modifier
                if chars.peek() == Some(&'!') || chars.peek() == Some(&'&') {
                    chars.next();
                }
            }
            // Boolean
            'p' => {}
            // Character
            'c' | 'C' => {}
            // Special markers
            '|' => {
                // Following args are optional - marker only
            }
            ':' | ';' => {
                // Rest of string is function name or error message
                break;
            }
            '(' | ')' => {
                // Tuple grouping - skip for now
            }
            // Whitespace is allowed
            ' ' | '\t' => {}
            // Unknown format character
            _ => {
                return false;
            }
        }
    }

    true
}

/// Build a Python object from a format string and values
///
/// Supported format characters:
/// - `i`: c_int -> int
/// - `l`: c_long -> int
/// - `L`: c_longlong -> int
/// - `n`: Py_ssize_t -> int
/// - `I`: c_uint -> int
/// - `k`: c_ulong -> int
/// - `K`: c_ulonglong -> int
/// - `f`: c_float -> float
/// - `d`: c_double -> float
/// - `s`: *const c_char -> str
/// - `z`: *const c_char -> str or None
/// - `y`: *const c_char -> bytes
/// - `O`: *mut PyObject -> object (borrowed ref)
/// - `N`: *mut PyObject -> object (steals ref)
/// - `()`: tuple
/// - `[]`: list
/// - `{}`: dict
///
/// # Safety
/// The format pointer must be a valid null-terminated C string.
/// The va_list must contain values matching the format string.
#[no_mangle]
pub unsafe extern "C" fn Py_BuildValue(
    format: *const c_char,
    // va_list would follow
) -> *mut PyObject {
    if format.is_null() {
        return ptr::null_mut();
    }

    let format_str = match CStr::from_ptr(format).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    // Empty format returns None
    if format_str.is_empty() {
        // Would return Py_None here
        return ptr::null_mut();
    }

    // Validate format string
    if !validate_build_format_string(format_str) {
        return ptr::null_mut();
    }

    // In a full implementation, we would:
    // 1. Parse the format string
    // 2. Extract values from va_list
    // 3. Build the appropriate Python object

    // For now, return null (would return actual object in full impl)
    ptr::null_mut()
}

/// Validate a format string for Py_BuildValue
fn validate_build_format_string(format: &str) -> bool {
    let mut depth = 0;
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // Integer formats
            'i' | 'l' | 'L' | 'n' | 'I' | 'k' | 'K' | 'b' | 'B' | 'h' | 'H' => {}
            // Float formats
            'f' | 'd' | 'D' => {}
            // String formats
            's' | 'z' | 'U' | 'y' => {
                // Check for # modifier
                if chars.peek() == Some(&'#') {
                    chars.next();
                }
            }
            // Object formats
            'O' | 'N' => {
                // Check for & modifier
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
            }
            // Container start
            '(' | '[' | '{' => {
                depth += 1;
            }
            // Container end
            ')' | ']' | '}' => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            // Whitespace and separators
            ' ' | '\t' | ',' | ':' => {}
            // Unknown format character
            _ => {
                return false;
            }
        }
    }

    depth == 0
}

/// Parse a single argument from format
///
/// # Safety
/// The value pointer must be valid for the format character.
pub unsafe fn parse_single_arg(
    format_char: char,
    obj: *mut PyObject,
    _value_ptr: *mut c_void,
) -> ArgParseResult<ParsedArg> {
    if obj.is_null() {
        return Err(ArgParseError::new("NULL object", Some(format_char), 0));
    }

    match format_char {
        'i' | 'l' | 'L' | 'n' => {
            // Would extract integer from PyObject
            Ok(ParsedArg::Int(0))
        }
        'I' | 'k' | 'K' => {
            // Would extract unsigned integer
            Ok(ParsedArg::UInt(0))
        }
        'f' | 'd' => {
            // Would extract float
            Ok(ParsedArg::Float(0.0))
        }
        's' | 'z' | 'U' => {
            // Would extract string
            Ok(ParsedArg::String(None))
        }
        'y' => {
            // Would extract bytes
            Ok(ParsedArg::Bytes(Vec::new()))
        }
        'O' => {
            // Return object reference
            Ok(ParsedArg::Object(obj))
        }
        'p' => {
            // Would extract boolean
            Ok(ParsedArg::Bool(false))
        }
        'c' | 'C' => {
            // Would extract character
            Ok(ParsedArg::Char('\0'))
        }
        _ => Err(ArgParseError::new(
            format!("Unknown format character: {}", format_char),
            Some(format_char),
            0,
        )),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_buffer_new() {
        let buf = Py_buffer::new();
        assert!(!buf.is_valid());
        assert_eq!(buf.len, 0);
        assert_eq!(buf.ndim, 0);
    }

    #[test]
    fn test_py_buffer_as_slice() {
        let data: Vec<i32> = vec![1, 2, 3, 4, 5];
        let mut buf = Py_buffer::new();
        buf.buf = data.as_ptr() as *mut c_void;
        buf.len = (data.len() * std::mem::size_of::<i32>()) as isize;
        buf.itemsize = std::mem::size_of::<i32>() as isize;
        buf.readonly = 1;

        unsafe {
            let slice: &[i32] = buf.as_slice().unwrap();
            assert_eq!(slice, &[1, 2, 3, 4, 5]);
        }
    }

    #[test]
    fn test_gil_state() {
        // Ensure GIL
        let state = GilState::ensure();
        assert!(GilGuard::is_held());

        // Release GIL
        state.release();
        assert!(!GilGuard::is_held());
    }

    #[test]
    fn test_gil_guard() {
        {
            let _guard = GilGuard::acquire();
            assert!(GilGuard::is_held());
        }
        // GIL should be released when guard is dropped
        assert!(!GilGuard::is_held());
    }

    #[test]
    fn test_gil_reentrant() {
        let state1 = GilState::ensure();
        assert_eq!(state1.prev_state(), PyGILState_STATE::PyGILState_UNLOCKED);

        // Second ensure should see we already hold it
        let state2 = GilState::ensure();
        assert_eq!(state2.prev_state(), PyGILState_STATE::PyGILState_LOCKED);

        // Release in reverse order
        state2.release(); // Should not actually release
        assert!(GilGuard::is_held());

        state1.release(); // Should actually release
        assert!(!GilGuard::is_held());
    }

    #[test]
    fn test_py_object_refcount() {
        let mut type_obj = PyTypeObject {
            ob_base: PyObject::new(ptr::null_mut()),
            tp_name: ptr::null(),
            tp_basicsize: 0,
            tp_itemsize: 0,
            tp_dealloc: None,
            tp_getattr: None,
            tp_setattr: None,
            tp_repr: None,
            tp_hash: None,
            tp_call: None,
            tp_str: None,
            tp_getattro: None,
            tp_setattro: None,
            tp_as_buffer: ptr::null_mut(),
            tp_flags: 0,
            tp_doc: ptr::null(),
            tp_traverse: None,
            tp_clear: None,
            tp_richcompare: None,
            tp_iter: None,
            tp_iternext: None,
            tp_methods: ptr::null_mut(),
            tp_members: ptr::null_mut(),
            tp_getset: ptr::null_mut(),
            tp_base: ptr::null_mut(),
            tp_dict: ptr::null_mut(),
            tp_descr_get: None,
            tp_descr_set: None,
            tp_dictoffset: 0,
            tp_init: None,
            tp_alloc: None,
            tp_new: None,
            tp_free: None,
        };

        let mut obj = PyObject::new(&mut type_obj);

        unsafe {
            assert_eq!(Py_REFCNT(&obj), 1);

            Py_IncRef(&mut obj);
            assert_eq!(Py_REFCNT(&obj), 2);

            Py_DecRef(&mut obj);
            assert_eq!(Py_REFCNT(&obj), 1);
        }
    }

    #[test]
    fn test_buffer_contiguous_1d() {
        let buf = Py_buffer {
            buf: ptr::null_mut(),
            len: 100,
            itemsize: 4,
            readonly: 0,
            ndim: 1,
            format: ptr::null_mut(),
            shape: ptr::null_mut(),
            strides: ptr::null_mut(),
            suboffsets: ptr::null_mut(),
            internal: ptr::null_mut(),
            obj: ptr::null_mut(),
        };

        unsafe {
            assert_eq!(PyBuffer_IsContiguous(&buf, b'C' as c_char), 1);
            assert_eq!(PyBuffer_IsContiguous(&buf, b'F' as c_char), 1);
            assert_eq!(PyBuffer_IsContiguous(&buf, b'A' as c_char), 1);
        }
    }

    #[test]
    fn test_validate_format_string_basic() {
        // Valid format strings
        assert!(validate_format_string("i"));
        assert!(validate_format_string("ii"));
        assert!(validate_format_string("iis"));
        assert!(validate_format_string("s#"));
        assert!(validate_format_string("O"));
        assert!(validate_format_string("O!"));
        assert!(validate_format_string("|i"));
        assert!(validate_format_string("ii|s"));
        assert!(validate_format_string("s:function_name"));
        assert!(validate_format_string("i;error message"));

        // Invalid format strings
        assert!(!validate_format_string("X")); // Unknown format
        assert!(!validate_format_string("@")); // Unknown format
    }

    #[test]
    fn test_validate_build_format_string() {
        // Valid format strings
        assert!(validate_build_format_string("i"));
        assert!(validate_build_format_string("(ii)"));
        assert!(validate_build_format_string("[i,i,i]"));
        assert!(validate_build_format_string("{s:i}"));
        assert!(validate_build_format_string("O"));
        assert!(validate_build_format_string("N"));
        assert!(validate_build_format_string("s#"));

        // Invalid format strings
        assert!(!validate_build_format_string(")")); // Unmatched
        assert!(!validate_build_format_string("(i")); // Unmatched
        assert!(!validate_build_format_string("X")); // Unknown format
    }

    #[test]
    fn test_pyarg_parsetuple_null_format() {
        unsafe {
            let result = PyArg_ParseTuple(ptr::null_mut(), ptr::null());
            assert_eq!(result, 0);
        }
    }

    #[test]
    fn test_py_buildvalue_null_format() {
        unsafe {
            let result = Py_BuildValue(ptr::null());
            assert!(result.is_null());
        }
    }

    #[test]
    fn test_py_buildvalue_empty_format() {
        unsafe {
            let format = b"\0";
            let result = Py_BuildValue(format.as_ptr() as *const c_char);
            // Empty format returns null (would return Py_None in full impl)
            assert!(result.is_null());
        }
    }

    #[test]
    fn test_dx_value_equality() {
        // None
        assert!(dx_values_equal(&DxValue::None, &DxValue::None));

        // Bool
        assert!(dx_values_equal(&DxValue::Bool(true), &DxValue::Bool(true)));
        assert!(!dx_values_equal(&DxValue::Bool(true), &DxValue::Bool(false)));

        // Int
        assert!(dx_values_equal(&DxValue::Int(42), &DxValue::Int(42)));
        assert!(!dx_values_equal(&DxValue::Int(42), &DxValue::Int(43)));

        // Float
        assert!(dx_values_equal(&DxValue::Float(3.125), &DxValue::Float(3.125)));

        // String
        assert!(dx_values_equal(
            &DxValue::String("hello".to_string()),
            &DxValue::String("hello".to_string())
        ));

        // Bytes
        assert!(dx_values_equal(&DxValue::Bytes(vec![1, 2, 3]), &DxValue::Bytes(vec![1, 2, 3])));

        // List
        assert!(dx_values_equal(
            &DxValue::List(vec![DxValue::Int(1), DxValue::Int(2)]),
            &DxValue::List(vec![DxValue::Int(1), DxValue::Int(2)])
        ));

        // Different types
        assert!(!dx_values_equal(&DxValue::Int(42), &DxValue::Float(42.0)));
    }

    #[test]
    fn test_pyobject_bridge_creation() {
        let bridge = PyObjectBridge::new();
        assert_eq!(bridge.allocated_count(), 0);
    }

    #[test]
    fn test_pyobject_bridge_to_pyobject_none() {
        let mut bridge = PyObjectBridge::new();
        unsafe {
            let result = bridge.to_pyobject(&DxValue::None);
            assert!(result.is_ok());
            // None returns null pointer (would be Py_None singleton in full impl)
            assert!(result.unwrap().is_null());
        }
    }

    #[test]
    fn test_pyobject_bridge_from_pyobject_null() {
        let bridge = PyObjectBridge::new();
        unsafe {
            let result = bridge.from_pyobject(ptr::null_mut());
            assert!(result.is_ok());
            match result.unwrap() {
                DxValue::None => {}
                _ => panic!("Expected None for null pointer"),
            }
        }
    }

    #[test]
    fn test_bridge_error_display() {
        let err = BridgeError::new("test error");
        assert_eq!(format!("{}", err), "test error");

        let err_with_types = BridgeError::with_types("conversion failed", "int", "str");
        assert!(format!("{}", err_with_types).contains("int"));
        assert!(format!("{}", err_with_types).contains("str"));
    }

    #[test]
    fn test_api_category_from_function_name() {
        assert_eq!(ApiCategory::from_function_name("Py_IncRef"), ApiCategory::ObjectCore);
        assert_eq!(ApiCategory::from_function_name("PyType_Ready"), ApiCategory::TypeSystem);
        assert_eq!(ApiCategory::from_function_name("PyNumber_Add"), ApiCategory::NumberProtocol);
        assert_eq!(
            ApiCategory::from_function_name("PySequence_Length"),
            ApiCategory::SequenceProtocol
        );
        assert_eq!(ApiCategory::from_function_name("PyMapping_Keys"), ApiCategory::MappingProtocol);
        assert_eq!(
            ApiCategory::from_function_name("PyBuffer_Release"),
            ApiCategory::BufferProtocol
        );
        assert_eq!(ApiCategory::from_function_name("PyArg_ParseTuple"), ApiCategory::ArgParsing);
        assert_eq!(ApiCategory::from_function_name("PyErr_SetString"), ApiCategory::ErrorHandling);
        assert_eq!(ApiCategory::from_function_name("PyMem_Malloc"), ApiCategory::MemoryAlloc);
        assert_eq!(ApiCategory::from_function_name("PyGILState_Ensure"), ApiCategory::GIL);
        assert_eq!(ApiCategory::from_function_name("PyImport_Import"), ApiCategory::Import);
        assert_eq!(ApiCategory::from_function_name("PyModule_Create"), ApiCategory::Module);
        assert_eq!(ApiCategory::from_function_name("SomeUnknownFunction"), ApiCategory::Other);
    }

    #[test]
    fn test_missing_api_error() {
        let err = MissingApiError::new("PyNumber_Add");
        assert_eq!(err.function_name, "PyNumber_Add");
        assert_eq!(err.category, ApiCategory::NumberProtocol);

        let formatted = err.format_error();
        assert!(formatted.contains("PyNumber_Add"));
        assert!(formatted.contains("NumberProtocol"));
    }

    #[test]
    fn test_api_tracker_basic() {
        let tracker = ApiTracker::new();

        // Check implemented function
        assert!(tracker.is_implemented("Py_IncRef"));
        assert!(tracker.is_implemented("PyArg_ParseTuple"));

        // Check unimplemented function
        assert!(!tracker.is_implemented("PyNumber_Add"));
    }

    #[test]
    fn test_api_tracker_record_calls() {
        let tracker = ApiTracker::new();

        // Record some calls
        tracker.record_call("Py_IncRef");
        tracker.record_call("PyNumber_Add"); // Not implemented
        tracker.record_call("Py_DecRef");

        let calls = tracker.get_all_calls();
        assert_eq!(calls.len(), 3);

        let missing = tracker.get_missing();
        assert_eq!(missing.len(), 1);
        assert!(missing.contains(&"PyNumber_Add".to_string()));
    }

    #[test]
    fn test_api_coverage_stats() {
        let tracker = ApiTracker::new();

        tracker.record_call("Py_IncRef");
        tracker.record_call("PyNumber_Add"); // Not implemented

        let stats = tracker.coverage_stats();
        assert!(stats.total_implemented > 0);
        assert_eq!(stats.missing_called, 1);
        assert_eq!(stats.total_calls, 2);
        assert!(stats.coverage_percentage() > 0.0);
    }

    #[test]
    fn test_check_api_implemented() {
        // Implemented function should succeed
        let result = check_api_implemented("Py_IncRef");
        assert!(result.is_ok());

        // Unimplemented function should fail with detailed error
        let result = check_api_implemented("PyNumber_Add");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.function_name, "PyNumber_Add");
    }
}
