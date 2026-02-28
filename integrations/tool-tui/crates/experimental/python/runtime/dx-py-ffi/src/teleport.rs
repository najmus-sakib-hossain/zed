//! Zero-copy array teleportation for NumPy integration
//!
//! This module provides the `TeleportedArray` type which implements the NumPy
//! array protocol for zero-copy data sharing between DX-Py and NumPy.
//!
//! ## NumPy Array Protocol Support
//!
//! This module implements the full NumPy array protocol:
//! - `__array_interface__` (version 3) - Dictionary-based interface
//! - `__array_struct__` - C-level interface via PyCapsule
//! - All NumPy dtypes including complex, datetime, and structured types
//!
//! ## Usage
//!
//! ```rust,ignore
//! use dx_py_ffi::teleport::{TeleportedArray, DType};
//!
//! // Create a float64 array
//! let data = vec![1.0f64, 2.0, 3.0, 4.0];
//! let array = TeleportedArray::from_vec(data, vec![2, 2]);
//!
//! // Get the array interface for NumPy
//! let interface = array.array_interface();
//! ```

use std::collections::HashMap;

/// Data type for array elements (NumPy dtype compatible)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DType {
    // Floating point types
    Float16,
    Float32,
    Float64,
    // Signed integer types
    Int8,
    Int16,
    Int32,
    Int64,
    // Unsigned integer types
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    // Boolean
    Bool,
    // Complex types
    Complex64,
    Complex128,
    // String types (fixed-length)
    String(usize),
    // Unicode types (fixed-length)
    Unicode(usize),
    // Datetime types
    DateTime64,
    TimeDelta64,
    // Object type (Python objects)
    Object,
}

impl DType {
    /// Get the size of this dtype in bytes
    pub fn size(&self) -> usize {
        match self {
            DType::Bool | DType::Int8 | DType::UInt8 => 1,
            DType::Float16 | DType::Int16 | DType::UInt16 => 2,
            DType::Float32 | DType::Int32 | DType::UInt32 => 4,
            DType::Float64 | DType::Int64 | DType::UInt64 | DType::Complex64 => 8,
            DType::Complex128 | DType::DateTime64 | DType::TimeDelta64 => 16,
            DType::String(len) => *len,
            DType::Unicode(len) => *len * 4, // UCS-4 encoding
            DType::Object => std::mem::size_of::<*mut ()>(),
        }
    }

    /// Get the alignment of this dtype
    pub fn alignment(&self) -> usize {
        match self {
            DType::Object => std::mem::size_of::<*mut ()>(),
            _ => self.size().min(8),
        }
    }

    /// Get the NumPy type character for this dtype
    pub fn type_char(&self) -> char {
        match self {
            DType::Bool => '?',
            DType::Int8 => 'b',
            DType::UInt8 => 'B',
            DType::Int16 => 'h',
            DType::UInt16 => 'H',
            DType::Int32 => 'i',
            DType::UInt32 => 'I',
            DType::Int64 => 'q',
            DType::UInt64 => 'Q',
            DType::Float16 => 'e',
            DType::Float32 => 'f',
            DType::Float64 => 'd',
            DType::Complex64 => 'F',
            DType::Complex128 => 'D',
            DType::String(_) => 'S',
            DType::Unicode(_) => 'U',
            DType::DateTime64 => 'M',
            DType::TimeDelta64 => 'm',
            DType::Object => 'O',
        }
    }

    /// Get the NumPy dtype string (e.g., "<f8" for little-endian float64)
    pub fn numpy_dtype_str(&self) -> String {
        let endian = if cfg!(target_endian = "little") {
            '<'
        } else {
            '>'
        };
        let type_char = self.type_char();
        let size = self.size();
        format!("{}{}{}", endian, type_char, size)
    }

    /// Parse a NumPy dtype string into a DType
    pub fn from_numpy_str(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }

        // Handle endianness prefix
        let (_, rest) =
            if s.starts_with('<') || s.starts_with('>') || s.starts_with('=') || s.starts_with('|')
            {
                (s.chars().next(), &s[1..])
            } else {
                (None, s)
            };

        if rest.is_empty() {
            return None;
        }

        let type_char = rest.chars().next()?;
        let size_str = &rest[1..];
        let size: usize = if size_str.is_empty() {
            0
        } else {
            size_str.parse().ok()?
        };

        match (type_char, size) {
            ('?', _) => Some(DType::Bool),
            ('b', _) => Some(DType::Int8),
            ('B', _) => Some(DType::UInt8),
            ('h', _) | ('i', 2) => Some(DType::Int16),
            ('H', _) | ('u', 2) => Some(DType::UInt16),
            ('i', 4) | ('l', 4) => Some(DType::Int32),
            ('I', 4) | ('L', 4) => Some(DType::UInt32),
            ('q', _) | ('i', 8) | ('l', 8) => Some(DType::Int64),
            ('Q', _) | ('u', 8) | ('L', 8) => Some(DType::UInt64),
            ('e', _) | ('f', 2) => Some(DType::Float16),
            ('f', 4) => Some(DType::Float32),
            ('d', _) | ('f', 8) => Some(DType::Float64),
            ('F', _) | ('c', 8) => Some(DType::Complex64),
            ('D', _) | ('c', 16) => Some(DType::Complex128),
            ('S', n) => Some(DType::String(n)),
            ('U', n) => Some(DType::Unicode(n)),
            ('M', _) => Some(DType::DateTime64),
            ('m', _) => Some(DType::TimeDelta64),
            ('O', _) => Some(DType::Object),
            _ => None,
        }
    }

    /// Check if this dtype is a numeric type
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            DType::Float16
                | DType::Float32
                | DType::Float64
                | DType::Int8
                | DType::Int16
                | DType::Int32
                | DType::Int64
                | DType::UInt8
                | DType::UInt16
                | DType::UInt32
                | DType::UInt64
                | DType::Complex64
                | DType::Complex128
        )
    }

    /// Check if this dtype is a floating point type
    pub fn is_floating(&self) -> bool {
        matches!(self, DType::Float16 | DType::Float32 | DType::Float64)
    }

    /// Check if this dtype is an integer type
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            DType::Int8
                | DType::Int16
                | DType::Int32
                | DType::Int64
                | DType::UInt8
                | DType::UInt16
                | DType::UInt32
                | DType::UInt64
        )
    }

    /// Check if this dtype is a complex type
    pub fn is_complex(&self) -> bool {
        matches!(self, DType::Complex64 | DType::Complex128)
    }

    /// Check if this dtype is a datetime type
    pub fn is_datetime(&self) -> bool {
        matches!(self, DType::DateTime64 | DType::TimeDelta64)
    }

    /// Check if this dtype is a string type
    pub fn is_string(&self) -> bool {
        matches!(self, DType::String(_) | DType::Unicode(_))
    }

    /// Check if this dtype is signed
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            DType::Int8
                | DType::Int16
                | DType::Int32
                | DType::Int64
                | DType::Float16
                | DType::Float32
                | DType::Float64
                | DType::Complex64
                | DType::Complex128
        )
    }

    /// Get the byte order character for this dtype
    pub fn byte_order_char(&self) -> char {
        match self {
            DType::Bool | DType::Int8 | DType::UInt8 => '|', // Not applicable
            _ => {
                if cfg!(target_endian = "little") {
                    '<'
                } else {
                    '>'
                }
            }
        }
    }

    /// Get the NumPy kind character (category of dtype)
    pub fn kind_char(&self) -> char {
        match self {
            DType::Bool => 'b',
            DType::Int8 | DType::Int16 | DType::Int32 | DType::Int64 => 'i',
            DType::UInt8 | DType::UInt16 | DType::UInt32 | DType::UInt64 => 'u',
            DType::Float16 | DType::Float32 | DType::Float64 => 'f',
            DType::Complex64 | DType::Complex128 => 'c',
            DType::String(_) => 'S',
            DType::Unicode(_) => 'U',
            DType::DateTime64 => 'M',
            DType::TimeDelta64 => 'm',
            DType::Object => 'O',
        }
    }

    /// Create a dtype from kind and size
    pub fn from_kind_and_size(kind: char, size: usize) -> Option<Self> {
        match (kind, size) {
            ('b', _) => Some(DType::Bool),
            ('i', 1) => Some(DType::Int8),
            ('i', 2) => Some(DType::Int16),
            ('i', 4) => Some(DType::Int32),
            ('i', 8) => Some(DType::Int64),
            ('u', 1) => Some(DType::UInt8),
            ('u', 2) => Some(DType::UInt16),
            ('u', 4) => Some(DType::UInt32),
            ('u', 8) => Some(DType::UInt64),
            ('f', 2) => Some(DType::Float16),
            ('f', 4) => Some(DType::Float32),
            ('f', 8) => Some(DType::Float64),
            ('c', 8) => Some(DType::Complex64),
            ('c', 16) => Some(DType::Complex128),
            ('S', n) => Some(DType::String(n)),
            ('U', n) => Some(DType::Unicode(n)),
            ('M', _) => Some(DType::DateTime64),
            ('m', _) => Some(DType::TimeDelta64),
            ('O', _) => Some(DType::Object),
            _ => None,
        }
    }

    /// Get the default value for this dtype as bytes
    pub fn default_bytes(&self) -> Vec<u8> {
        vec![0u8; self.size()]
    }

    /// Check if two dtypes are compatible for operations
    pub fn is_compatible_with(&self, other: &DType) -> bool {
        // Same type is always compatible
        if self == other {
            return true;
        }

        // Numeric types can be promoted
        if self.is_numeric() && other.is_numeric() {
            return true;
        }

        // String types with different lengths are compatible
        if self.is_string() && other.is_string() {
            return true;
        }

        false
    }

    /// Get the promoted dtype when combining two dtypes
    pub fn promote_with(&self, other: &DType) -> Option<DType> {
        if self == other {
            return Some(*self);
        }

        // Float promotion rules
        if self.is_floating() || other.is_floating() {
            let max_size = self.size().max(other.size());
            return match max_size {
                8 => Some(DType::Float64),
                4 => Some(DType::Float32),
                _ => Some(DType::Float64),
            };
        }

        // Complex promotion
        if self.is_complex() || other.is_complex() {
            let max_size = self.size().max(other.size());
            return match max_size {
                16 => Some(DType::Complex128),
                _ => Some(DType::Complex64),
            };
        }

        // Integer promotion
        if self.is_integer() && other.is_integer() {
            let max_size = self.size().max(other.size());
            let is_signed = self.is_signed() || other.is_signed();
            return match (max_size, is_signed) {
                (8, true) => Some(DType::Int64),
                (8, false) => Some(DType::UInt64),
                (4, true) => Some(DType::Int32),
                (4, false) => Some(DType::UInt32),
                (2, true) => Some(DType::Int16),
                (2, false) => Some(DType::UInt16),
                (1, true) => Some(DType::Int8),
                (1, false) => Some(DType::UInt8),
                _ => Some(DType::Int64),
            };
        }

        None
    }
}

/// Zero-copy array access for NumPy integration
///
/// This struct provides direct access to array data without copying,
/// enabling SIMD operations directly on NumPy memory.
///
/// Implements the NumPy array protocol via `__array_interface__` and
/// `__array_struct__` for seamless interoperability.
pub struct TeleportedArray {
    /// Pointer to array data (shared with Python)
    data: *mut u8,
    /// Shape of the array
    shape: Vec<usize>,
    /// Strides in bytes
    strides: Vec<isize>,
    /// Element type
    dtype: DType,
    /// Total byte size of data
    byte_size: usize,
    /// Whether this array is read-only
    readonly: bool,
    /// Owner reference count (to prevent deallocation)
    _owner_refcount: u64,
    /// Flags for array properties
    flags: ArrayFlags,
}

/// Flags describing array properties (compatible with NumPy)
#[derive(Debug, Clone, Copy, Default)]
pub struct ArrayFlags {
    /// Array is C-contiguous (row-major)
    pub c_contiguous: bool,
    /// Array is Fortran-contiguous (column-major)
    pub f_contiguous: bool,
    /// Array owns its data
    pub owndata: bool,
    /// Array is writeable
    pub writeable: bool,
    /// Array is aligned
    pub aligned: bool,
    /// Array can be updated in-place
    pub updateifcopy: bool,
}

impl ArrayFlags {
    /// NumPy flag constants
    pub const NPY_ARRAY_C_CONTIGUOUS: u32 = 0x0001;
    pub const NPY_ARRAY_F_CONTIGUOUS: u32 = 0x0002;
    pub const NPY_ARRAY_OWNDATA: u32 = 0x0004;
    pub const NPY_ARRAY_FORCECAST: u32 = 0x0010;
    pub const NPY_ARRAY_ENSURECOPY: u32 = 0x0020;
    pub const NPY_ARRAY_ENSUREARRAY: u32 = 0x0040;
    pub const NPY_ARRAY_ELEMENTSTRIDES: u32 = 0x0080;
    pub const NPY_ARRAY_ALIGNED: u32 = 0x0100;
    pub const NPY_ARRAY_NOTSWAPPED: u32 = 0x0200;
    pub const NPY_ARRAY_WRITEABLE: u32 = 0x0400;
    pub const NPY_ARRAY_UPDATEIFCOPY: u32 = 0x1000;

    /// Convert to NumPy flags integer
    pub fn to_numpy_flags(&self) -> u32 {
        let mut flags = 0u32;
        if self.c_contiguous {
            flags |= Self::NPY_ARRAY_C_CONTIGUOUS;
        }
        if self.f_contiguous {
            flags |= Self::NPY_ARRAY_F_CONTIGUOUS;
        }
        if self.owndata {
            flags |= Self::NPY_ARRAY_OWNDATA;
        }
        if self.writeable {
            flags |= Self::NPY_ARRAY_WRITEABLE;
        }
        if self.aligned {
            flags |= Self::NPY_ARRAY_ALIGNED;
        }
        if self.updateifcopy {
            flags |= Self::NPY_ARRAY_UPDATEIFCOPY;
        }
        flags
    }

    /// Create from NumPy flags integer
    pub fn from_numpy_flags(flags: u32) -> Self {
        Self {
            c_contiguous: (flags & Self::NPY_ARRAY_C_CONTIGUOUS) != 0,
            f_contiguous: (flags & Self::NPY_ARRAY_F_CONTIGUOUS) != 0,
            owndata: (flags & Self::NPY_ARRAY_OWNDATA) != 0,
            writeable: (flags & Self::NPY_ARRAY_WRITEABLE) != 0,
            aligned: (flags & Self::NPY_ARRAY_ALIGNED) != 0,
            updateifcopy: (flags & Self::NPY_ARRAY_UPDATEIFCOPY) != 0,
        }
    }
}

/// The __array_interface__ dictionary representation
#[derive(Debug, Clone)]
pub struct ArrayInterface {
    /// Shape tuple
    pub shape: Vec<usize>,
    /// Type string (e.g., "<f8")
    pub typestr: String,
    /// Data pointer and read-only flag
    pub data: (usize, bool),
    /// Strides tuple (None for C-contiguous)
    pub strides: Option<Vec<isize>>,
    /// Descr for structured arrays (None for simple types)
    pub descr: Option<Vec<(String, String)>>,
    /// Version (always 3)
    pub version: u32,
    /// Mask array for masked arrays (None for regular arrays)
    pub mask: Option<usize>,
    /// Offset into the data buffer (usually 0)
    pub offset: usize,
}

impl ArrayInterface {
    /// Create a new ArrayInterface with default values
    pub fn new(shape: Vec<usize>, typestr: String, data_ptr: usize, readonly: bool) -> Self {
        Self {
            shape,
            typestr,
            data: (data_ptr, readonly),
            strides: None,
            descr: None,
            version: 3,
            mask: None,
            offset: 0,
        }
    }

    /// Set strides for non-contiguous arrays
    pub fn with_strides(mut self, strides: Vec<isize>) -> Self {
        self.strides = Some(strides);
        self
    }

    /// Set descr for structured arrays
    pub fn with_descr(mut self, descr: Vec<(String, String)>) -> Self {
        self.descr = Some(descr);
        self
    }

    /// Set mask for masked arrays
    pub fn with_mask(mut self, mask_ptr: usize) -> Self {
        self.mask = Some(mask_ptr);
        self
    }

    /// Convert to a Python-compatible dictionary representation
    pub fn to_dict(&self) -> HashMap<String, ArrayInterfaceValue> {
        let mut dict = HashMap::new();

        dict.insert("shape".to_string(), ArrayInterfaceValue::Shape(self.shape.clone()));
        dict.insert("typestr".to_string(), ArrayInterfaceValue::TypeStr(self.typestr.clone()));
        dict.insert("data".to_string(), ArrayInterfaceValue::Data(self.data.0, self.data.1));
        dict.insert("version".to_string(), ArrayInterfaceValue::Version(self.version));

        if let Some(ref strides) = self.strides {
            dict.insert("strides".to_string(), ArrayInterfaceValue::Strides(strides.clone()));
        }

        if let Some(ref descr) = self.descr {
            dict.insert("descr".to_string(), ArrayInterfaceValue::Descr(descr.clone()));
        }

        if let Some(mask) = self.mask {
            dict.insert("mask".to_string(), ArrayInterfaceValue::Mask(mask));
        }

        dict
    }
}

/// Structured dtype field descriptor
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    /// Field name
    pub name: String,
    /// Field dtype
    pub dtype: DType,
    /// Byte offset within the struct
    pub offset: usize,
    /// Optional shape for array fields
    pub shape: Option<Vec<usize>>,
}

impl StructField {
    /// Create a new struct field
    pub fn new(name: impl Into<String>, dtype: DType, offset: usize) -> Self {
        Self {
            name: name.into(),
            dtype,
            offset,
            shape: None,
        }
    }

    /// Create an array field
    pub fn array(name: impl Into<String>, dtype: DType, offset: usize, shape: Vec<usize>) -> Self {
        Self {
            name: name.into(),
            dtype,
            offset,
            shape: Some(shape),
        }
    }

    /// Get the total size of this field in bytes
    pub fn size(&self) -> usize {
        let base_size = self.dtype.size();
        match &self.shape {
            Some(shape) => base_size * shape.iter().product::<usize>(),
            None => base_size,
        }
    }

    /// Get the NumPy descr tuple for this field
    pub fn to_descr(&self) -> (String, String) {
        let typestr = self.dtype.numpy_dtype_str();
        match &self.shape {
            Some(shape) => {
                let shape_str = shape.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(",");
                (self.name.clone(), format!("({},){}", shape_str, typestr))
            }
            None => (self.name.clone(), typestr),
        }
    }
}

/// Structured dtype for record arrays
#[derive(Debug, Clone)]
pub struct StructuredDType {
    /// Fields in order
    pub fields: Vec<StructField>,
    /// Total size of the struct in bytes
    pub itemsize: usize,
    /// Alignment requirement
    pub alignment: usize,
}

impl StructuredDType {
    /// Create a new structured dtype from fields
    pub fn new(fields: Vec<StructField>) -> Self {
        let itemsize = fields.iter().map(|f| f.offset + f.size()).max().unwrap_or(0);
        let alignment = fields.iter().map(|f| f.dtype.alignment()).max().unwrap_or(1);

        Self {
            fields,
            itemsize,
            alignment,
        }
    }

    /// Create a packed structured dtype (no padding)
    pub fn packed(field_specs: Vec<(String, DType)>) -> Self {
        let mut offset = 0;
        let fields: Vec<StructField> = field_specs
            .into_iter()
            .map(|(name, dtype)| {
                let field = StructField::new(name, dtype, offset);
                offset += dtype.size();
                field
            })
            .collect();

        Self::new(fields)
    }

    /// Get the NumPy descr list for this structured dtype
    pub fn to_descr(&self) -> Vec<(String, String)> {
        self.fields.iter().map(|f| f.to_descr()).collect()
    }

    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&StructField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Get field offset by name
    pub fn field_offset(&self, name: &str) -> Option<usize> {
        self.get_field(name).map(|f| f.offset)
    }
}

/// The __array_struct__ PyCapsule contents
#[repr(C)]
pub struct PyArrayInterface {
    /// Version (2 for current)
    pub two: i32,
    /// Number of dimensions
    pub nd: i32,
    /// Type character
    pub typekind: u8,
    /// Item size
    pub itemsize: i32,
    /// Flags
    pub flags: i32,
    /// Shape array
    pub shape: *mut isize,
    /// Strides array
    pub strides: *mut isize,
    /// Data pointer
    pub data: *mut u8,
    /// Object to hold reference
    pub descr: *mut std::ffi::c_void,
}

// Safety: TeleportedArray is Send because we ensure proper synchronization
// when accessing the data pointer
unsafe impl Send for TeleportedArray {}

impl TeleportedArray {
    /// Create a new teleported array from raw components
    ///
    /// # Safety
    /// - `data` must be a valid pointer to array data
    /// - The data must remain valid for the lifetime of this struct
    /// - The shape and strides must correctly describe the data layout
    pub unsafe fn new(
        data: *mut u8,
        shape: Vec<usize>,
        strides: Vec<isize>,
        dtype: DType,
        readonly: bool,
    ) -> Self {
        let byte_size = shape.iter().product::<usize>() * dtype.size();
        let c_contiguous = Self::check_c_contiguous(&shape, &strides, dtype.size());
        let f_contiguous = Self::check_f_contiguous(&shape, &strides, dtype.size());

        Self {
            data,
            shape,
            strides,
            dtype,
            byte_size,
            readonly,
            _owner_refcount: 1,
            flags: ArrayFlags {
                c_contiguous,
                f_contiguous,
                owndata: false,
                writeable: !readonly,
                aligned: true,
                updateifcopy: false,
            },
        }
    }

    /// Create a teleported array from a Vec (takes ownership)
    pub fn from_vec<T: Copy + 'static>(data: Vec<T>, shape: Vec<usize>) -> Self {
        let dtype = Self::dtype_for::<T>();
        let byte_size = data.len() * std::mem::size_of::<T>();

        // Calculate strides (row-major / C-contiguous)
        let mut strides = vec![0isize; shape.len()];
        let mut stride = std::mem::size_of::<T>() as isize;
        for i in (0..shape.len()).rev() {
            strides[i] = stride;
            stride *= shape[i] as isize;
        }

        let ptr = Box::into_raw(data.into_boxed_slice()) as *mut u8;

        Self {
            data: ptr,
            shape: shape.clone(),
            strides: strides.clone(),
            dtype,
            byte_size,
            readonly: false,
            _owner_refcount: 1,
            flags: ArrayFlags {
                c_contiguous: true,
                f_contiguous: shape.len() <= 1,
                owndata: true,
                writeable: true,
                aligned: true,
                updateifcopy: false,
            },
        }
    }

    /// Create a zeros array with the given shape and dtype
    pub fn zeros(shape: Vec<usize>, dtype: DType) -> Self {
        let total_elements: usize = shape.iter().product();
        let byte_size = total_elements * dtype.size();
        let data = vec![0u8; byte_size];

        // Calculate strides (row-major / C-contiguous)
        let mut strides = vec![0isize; shape.len()];
        let mut stride = dtype.size() as isize;
        for i in (0..shape.len()).rev() {
            strides[i] = stride;
            stride *= shape[i] as isize;
        }

        let ptr = Box::into_raw(data.into_boxed_slice()) as *mut u8;

        Self {
            data: ptr,
            shape: shape.clone(),
            strides,
            dtype,
            byte_size,
            readonly: false,
            _owner_refcount: 1,
            flags: ArrayFlags {
                c_contiguous: true,
                f_contiguous: shape.len() <= 1,
                owndata: true,
                writeable: true,
                aligned: true,
                updateifcopy: false,
            },
        }
    }

    /// Create a ones array with the given shape and dtype
    pub fn ones(shape: Vec<usize>, dtype: DType) -> Self {
        let mut arr = Self::zeros(shape, dtype);

        // Fill with ones based on dtype
        unsafe {
            match dtype {
                DType::Float32 => {
                    let slice = arr.as_mut_slice::<f32>().unwrap();
                    slice.fill(1.0);
                }
                DType::Float64 => {
                    let slice = arr.as_mut_slice::<f64>().unwrap();
                    slice.fill(1.0);
                }
                DType::Int8 => {
                    let slice = arr.as_mut_slice::<i8>().unwrap();
                    slice.fill(1);
                }
                DType::Int16 => {
                    let slice = arr.as_mut_slice::<i16>().unwrap();
                    slice.fill(1);
                }
                DType::Int32 => {
                    let slice = arr.as_mut_slice::<i32>().unwrap();
                    slice.fill(1);
                }
                DType::Int64 => {
                    let slice = arr.as_mut_slice::<i64>().unwrap();
                    slice.fill(1);
                }
                DType::UInt8 => {
                    let slice = arr.as_mut_slice::<u8>().unwrap();
                    slice.fill(1);
                }
                DType::UInt16 => {
                    let slice = arr.as_mut_slice::<u16>().unwrap();
                    slice.fill(1);
                }
                DType::UInt32 => {
                    let slice = arr.as_mut_slice::<u32>().unwrap();
                    slice.fill(1);
                }
                DType::UInt64 => {
                    let slice = arr.as_mut_slice::<u64>().unwrap();
                    slice.fill(1);
                }
                _ => {} // Other types not supported for ones
            }
        }

        arr
    }

    /// Check if strides represent C-contiguous layout
    fn check_c_contiguous(shape: &[usize], strides: &[isize], elem_size: usize) -> bool {
        if shape.is_empty() {
            return true;
        }

        let mut expected_stride = elem_size as isize;
        for i in (0..shape.len()).rev() {
            if strides[i] != expected_stride {
                return false;
            }
            expected_stride *= shape[i] as isize;
        }
        true
    }

    /// Check if strides represent Fortran-contiguous layout
    fn check_f_contiguous(shape: &[usize], strides: &[isize], elem_size: usize) -> bool {
        if shape.is_empty() {
            return true;
        }

        let mut expected_stride = elem_size as isize;
        for i in 0..shape.len() {
            if strides[i] != expected_stride {
                return false;
            }
            expected_stride *= shape[i] as isize;
        }
        true
    }

    /// Get the dtype for a Rust type
    fn dtype_for<T: 'static>() -> DType {
        use std::any::TypeId;

        let type_id = TypeId::of::<T>();

        if type_id == TypeId::of::<f32>() {
            DType::Float32
        } else if type_id == TypeId::of::<f64>() {
            DType::Float64
        } else if type_id == TypeId::of::<i32>() {
            DType::Int32
        } else if type_id == TypeId::of::<i64>() {
            DType::Int64
        } else if type_id == TypeId::of::<i16>() {
            DType::Int16
        } else if type_id == TypeId::of::<i8>() {
            DType::Int8
        } else if type_id == TypeId::of::<u32>() {
            DType::UInt32
        } else if type_id == TypeId::of::<u64>() {
            DType::UInt64
        } else if type_id == TypeId::of::<u16>() {
            DType::UInt16
        } else if type_id == TypeId::of::<u8>() {
            DType::UInt8
        } else if type_id == TypeId::of::<bool>() {
            DType::Bool
        } else {
            DType::UInt8
        } // Default fallback
    }

    /// Get raw data pointer (zero-copy)
    #[inline]
    pub fn data_ptr(&self) -> *const u8 {
        self.data
    }

    /// Get mutable data pointer (zero-copy)
    ///
    /// Returns None if the array is read-only.
    #[inline]
    pub fn data_ptr_mut(&mut self) -> Option<*mut u8> {
        if self.readonly {
            None
        } else {
            Some(self.data)
        }
    }

    /// Get the shape
    #[inline]
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Get the strides
    #[inline]
    pub fn strides(&self) -> &[isize] {
        &self.strides
    }

    /// Get the dtype
    #[inline]
    pub fn dtype(&self) -> DType {
        self.dtype
    }

    /// Get the total byte size
    #[inline]
    pub fn byte_size(&self) -> usize {
        self.byte_size
    }

    /// Get the number of dimensions
    #[inline]
    pub fn ndim(&self) -> usize {
        self.shape.len()
    }

    /// Get the total number of elements
    #[inline]
    pub fn len(&self) -> usize {
        self.shape.iter().product()
    }

    /// Check if the array is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if the array is contiguous (C-order)
    pub fn is_contiguous(&self) -> bool {
        if self.shape.is_empty() {
            return true;
        }

        let elem_size = self.dtype.size() as isize;
        let mut expected_stride = elem_size;

        for i in (0..self.shape.len()).rev() {
            if self.strides[i] != expected_stride {
                return false;
            }
            expected_stride *= self.shape[i] as isize;
        }

        true
    }

    /// Get data as a slice (zero-copy)
    ///
    /// # Safety
    /// The caller must ensure T matches the dtype.
    #[inline]
    pub unsafe fn as_slice<T>(&self) -> &[T] {
        let len = self.byte_size / std::mem::size_of::<T>();
        std::slice::from_raw_parts(self.data as *const T, len)
    }

    /// Get data as a mutable slice (zero-copy)
    ///
    /// # Safety
    /// The caller must ensure T matches the dtype.
    #[inline]
    pub unsafe fn as_mut_slice<T>(&mut self) -> Option<&mut [T]> {
        if self.readonly {
            return None;
        }
        let len = self.byte_size / std::mem::size_of::<T>();
        Some(std::slice::from_raw_parts_mut(self.data as *mut T, len))
    }

    /// Add a scalar to all elements (SIMD-accelerated for f64)
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    pub fn add_scalar_f64(&mut self, scalar: f64) -> bool {
        if self.readonly || self.dtype != DType::Float64 {
            return false;
        }

        unsafe {
            if is_x86_feature_detected!("avx2") {
                self.add_scalar_f64_avx2(scalar);
            } else {
                self.add_scalar_f64_scalar(scalar);
            }
        }

        true
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86")))]
    pub fn add_scalar_f64(&mut self, scalar: f64) -> bool {
        if self.readonly || self.dtype != DType::Float64 {
            return false;
        }

        unsafe { self.add_scalar_f64_scalar(scalar) };
        true
    }

    /// AVX2 implementation of add_scalar
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    #[target_feature(enable = "avx2")]
    unsafe fn add_scalar_f64_avx2(&mut self, scalar: f64) {
        use std::arch::x86_64::*;

        let scalar_vec = _mm256_set1_pd(scalar);
        let data = self.data as *mut f64;
        let len = self.byte_size / 8;

        let mut i = 0;
        while i + 4 <= len {
            let chunk = _mm256_loadu_pd(data.add(i));
            let result = _mm256_add_pd(chunk, scalar_vec);
            _mm256_storeu_pd(data.add(i), result);
            i += 4;
        }

        // Scalar remainder
        while i < len {
            *data.add(i) += scalar;
            i += 1;
        }
    }

    /// Scalar implementation of add_scalar
    unsafe fn add_scalar_f64_scalar(&mut self, scalar: f64) {
        let data = self.data as *mut f64;
        let len = self.byte_size / 8;

        for i in 0..len {
            *data.add(i) += scalar;
        }
    }

    /// Multiply all elements by a scalar (SIMD-accelerated for f64)
    pub fn mul_scalar_f64(&mut self, scalar: f64) -> bool {
        if self.readonly || self.dtype != DType::Float64 {
            return false;
        }

        unsafe {
            let data = self.data as *mut f64;
            let len = self.byte_size / 8;

            for i in 0..len {
                *data.add(i) *= scalar;
            }
        }

        true
    }

    // =========================================================================
    // Array Arithmetic Operations
    // =========================================================================

    /// Subtract a scalar from all elements
    pub fn sub_scalar_f64(&mut self, scalar: f64) -> bool {
        self.add_scalar_f64(-scalar)
    }

    /// Divide all elements by a scalar
    pub fn div_scalar_f64(&mut self, scalar: f64) -> bool {
        if scalar == 0.0 {
            return false;
        }
        self.mul_scalar_f64(1.0 / scalar)
    }

    /// Element-wise addition with another array
    ///
    /// Returns a new array with the result. Arrays must have the same shape.
    pub fn add(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        if self.shape != other.shape || self.dtype != other.dtype {
            return None;
        }

        let mut result = TeleportedArray::zeros(self.shape.clone(), self.dtype);

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let r: &mut [f64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] + b[i];
                    }
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let r: &mut [f32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] + b[i];
                    }
                }
                DType::Int64 => {
                    let a: &[i64] = self.as_slice();
                    let b: &[i64] = other.as_slice();
                    let r: &mut [i64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] + b[i];
                    }
                }
                DType::Int32 => {
                    let a: &[i32] = self.as_slice();
                    let b: &[i32] = other.as_slice();
                    let r: &mut [i32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] + b[i];
                    }
                }
                _ => return None,
            }
        }

        Some(result)
    }

    /// Element-wise subtraction with another array
    pub fn sub(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        if self.shape != other.shape || self.dtype != other.dtype {
            return None;
        }

        let mut result = TeleportedArray::zeros(self.shape.clone(), self.dtype);

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let r: &mut [f64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] - b[i];
                    }
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let r: &mut [f32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] - b[i];
                    }
                }
                DType::Int64 => {
                    let a: &[i64] = self.as_slice();
                    let b: &[i64] = other.as_slice();
                    let r: &mut [i64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] - b[i];
                    }
                }
                DType::Int32 => {
                    let a: &[i32] = self.as_slice();
                    let b: &[i32] = other.as_slice();
                    let r: &mut [i32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] - b[i];
                    }
                }
                _ => return None,
            }
        }

        Some(result)
    }

    /// Element-wise multiplication with another array
    pub fn mul(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        if self.shape != other.shape || self.dtype != other.dtype {
            return None;
        }

        let mut result = TeleportedArray::zeros(self.shape.clone(), self.dtype);

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let r: &mut [f64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] * b[i];
                    }
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let r: &mut [f32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] * b[i];
                    }
                }
                DType::Int64 => {
                    let a: &[i64] = self.as_slice();
                    let b: &[i64] = other.as_slice();
                    let r: &mut [i64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] * b[i];
                    }
                }
                DType::Int32 => {
                    let a: &[i32] = self.as_slice();
                    let b: &[i32] = other.as_slice();
                    let r: &mut [i32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] * b[i];
                    }
                }
                _ => return None,
            }
        }

        Some(result)
    }

    /// Element-wise division with another array
    pub fn div(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        if self.shape != other.shape || self.dtype != other.dtype {
            return None;
        }

        let mut result = TeleportedArray::zeros(self.shape.clone(), self.dtype);

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let r: &mut [f64] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] / b[i];
                    }
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let r: &mut [f32] = result.as_mut_slice()?;
                    for i in 0..a.len() {
                        r[i] = a[i] / b[i];
                    }
                }
                _ => return None, // Integer division not supported
            }
        }

        Some(result)
    }

    // =========================================================================
    // Broadcasting Support
    // =========================================================================

    /// Check if two shapes are broadcastable
    pub fn shapes_broadcastable(shape1: &[usize], shape2: &[usize]) -> bool {
        let max_len = shape1.len().max(shape2.len());

        for i in 0..max_len {
            let d1 = if i < shape1.len() {
                shape1[shape1.len() - 1 - i]
            } else {
                1
            };
            let d2 = if i < shape2.len() {
                shape2[shape2.len() - 1 - i]
            } else {
                1
            };

            if d1 != d2 && d1 != 1 && d2 != 1 {
                return false;
            }
        }

        true
    }

    /// Get the broadcast shape of two shapes
    pub fn broadcast_shape(shape1: &[usize], shape2: &[usize]) -> Option<Vec<usize>> {
        if !Self::shapes_broadcastable(shape1, shape2) {
            return None;
        }

        let max_len = shape1.len().max(shape2.len());
        let mut result = vec![0; max_len];

        for i in 0..max_len {
            let d1 = if i < shape1.len() {
                shape1[shape1.len() - 1 - i]
            } else {
                1
            };
            let d2 = if i < shape2.len() {
                shape2[shape2.len() - 1 - i]
            } else {
                1
            };
            result[max_len - 1 - i] = d1.max(d2);
        }

        Some(result)
    }

    /// Broadcast this array to a new shape
    ///
    /// Returns a view (no data copy) if possible.
    pub fn broadcast_to(&self, new_shape: &[usize]) -> Option<TeleportedArray> {
        // Check if broadcast is valid
        if new_shape.len() < self.shape.len() {
            return None;
        }

        // Check each dimension
        let offset = new_shape.len() - self.shape.len();
        for i in 0..self.shape.len() {
            let old_dim = self.shape[i];
            let new_dim = new_shape[offset + i];
            if old_dim != new_dim && old_dim != 1 {
                return None;
            }
        }

        // Calculate new strides (0 for broadcast dimensions)
        let mut new_strides = vec![0isize; new_shape.len()];
        for i in 0..self.shape.len() {
            let old_dim = self.shape[i];
            if old_dim == 1 {
                new_strides[offset + i] = 0; // Broadcast dimension
            } else {
                new_strides[offset + i] = self.strides[i];
            }
        }

        Some(TeleportedArray {
            data: self.data,
            shape: new_shape.to_vec(),
            strides: new_strides,
            dtype: self.dtype,
            byte_size: self.byte_size,
            readonly: true, // Broadcast views are read-only
            _owner_refcount: self._owner_refcount,
            flags: ArrayFlags {
                c_contiguous: false,
                f_contiguous: false,
                owndata: false,
                writeable: false,
                aligned: self.flags.aligned,
                updateifcopy: false,
            },
        })
    }

    // =========================================================================
    // Slicing and Indexing
    // =========================================================================

    /// Get a slice of the array along the first axis
    ///
    /// Returns a view into the original data.
    pub fn slice_axis0(&self, start: usize, end: usize) -> Option<TeleportedArray> {
        if self.shape.is_empty() || start >= end || end > self.shape[0] {
            return None;
        }

        let mut new_shape = self.shape.clone();
        new_shape[0] = end - start;

        let offset = start as isize * self.strides[0];
        let new_data = unsafe { self.data.offset(offset) };

        Some(TeleportedArray {
            data: new_data,
            shape: new_shape,
            strides: self.strides.clone(),
            dtype: self.dtype,
            byte_size: (end - start)
                * self.shape[1..].iter().product::<usize>()
                * self.dtype.size(),
            readonly: self.readonly,
            _owner_refcount: self._owner_refcount,
            flags: ArrayFlags {
                c_contiguous: self.flags.c_contiguous,
                f_contiguous: false,
                owndata: false,
                writeable: !self.readonly,
                aligned: self.flags.aligned,
                updateifcopy: false,
            },
        })
    }

    /// Get a single element by flat index
    ///
    /// # Safety
    /// The index must be in bounds.
    pub unsafe fn get_flat<T: Copy>(&self, index: usize) -> Option<T> {
        if index >= self.len() {
            return None;
        }

        let ptr = self.data as *const T;
        Some(*ptr.add(index))
    }

    /// Set a single element by flat index
    ///
    /// # Safety
    /// The index must be in bounds and the array must be writeable.
    pub unsafe fn set_flat<T: Copy>(&mut self, index: usize, value: T) -> bool {
        if self.readonly || index >= self.len() {
            return false;
        }

        let ptr = self.data as *mut T;
        *ptr.add(index) = value;
        true
    }

    /// Get element at multi-dimensional index
    ///
    /// # Safety
    /// All indices must be in bounds.
    pub unsafe fn get_at<T: Copy>(&self, indices: &[usize]) -> Option<T> {
        if indices.len() != self.shape.len() {
            return None;
        }

        // Check bounds
        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return None;
            }
        }

        // Calculate byte offset
        let mut offset: isize = 0;
        for (i, &idx) in indices.iter().enumerate() {
            offset += (idx as isize) * self.strides[i];
        }

        let ptr = self.data.offset(offset) as *const T;
        Some(*ptr)
    }

    /// Set element at multi-dimensional index
    ///
    /// # Safety
    /// All indices must be in bounds and the array must be writeable.
    pub unsafe fn set_at<T: Copy>(&mut self, indices: &[usize], value: T) -> bool {
        if self.readonly || indices.len() != self.shape.len() {
            return false;
        }

        // Check bounds
        for (i, &idx) in indices.iter().enumerate() {
            if idx >= self.shape[i] {
                return false;
            }
        }

        // Calculate byte offset
        let mut offset: isize = 0;
        for (i, &idx) in indices.iter().enumerate() {
            offset += (idx as isize) * self.strides[i];
        }

        let ptr = self.data.offset(offset) as *mut T;
        *ptr = value;
        true
    }

    /// Create a copy of the array
    pub fn copy(&self) -> TeleportedArray {
        let result = TeleportedArray::zeros(self.shape.clone(), self.dtype);

        unsafe {
            std::ptr::copy_nonoverlapping(self.data, result.data, self.byte_size);
        }

        result
    }

    /// Get the __array_interface__ dictionary representation
    ///
    /// This implements the NumPy array interface protocol (version 3)
    /// for zero-copy interoperability with NumPy.
    pub fn array_interface(&self) -> ArrayInterface {
        let mut interface = ArrayInterface::new(
            self.shape.clone(),
            self.dtype.numpy_dtype_str(),
            self.data as usize,
            self.readonly,
        );

        if !self.flags.c_contiguous {
            interface = interface.with_strides(self.strides.clone());
        }

        interface
    }

    /// Get the __array_interface__ as a HashMap for Python dict conversion
    pub fn array_interface_dict(&self) -> HashMap<String, ArrayInterfaceValue> {
        self.array_interface().to_dict()
    }

    /// Create a PyArrayInterface struct for __array_struct__
    ///
    /// # Safety
    /// The returned struct contains raw pointers that must remain valid
    /// for the lifetime of the array.
    pub unsafe fn array_struct(&self) -> PyArrayInterface {
        // Allocate shape and strides arrays that will be pointed to
        let shape_ptr = self.shape.as_ptr() as *mut isize;
        let strides_ptr = self.strides.as_ptr() as *mut isize;

        PyArrayInterface {
            two: 2, // Version number
            nd: self.shape.len() as i32,
            typekind: self.dtype.type_char() as u8,
            itemsize: self.dtype.size() as i32,
            flags: self.flags.to_numpy_flags() as i32,
            shape: shape_ptr,
            strides: strides_ptr,
            data: self.data,
            descr: std::ptr::null_mut(),
        }
    }

    /// Get the array flags
    pub fn flags(&self) -> ArrayFlags {
        self.flags
    }

    /// Check if the array is Fortran-contiguous
    pub fn is_f_contiguous(&self) -> bool {
        self.flags.f_contiguous
    }

    /// Check if the array is C-contiguous
    pub fn is_c_contiguous(&self) -> bool {
        self.flags.c_contiguous
    }

    /// Reshape the array (returns a new view if possible)
    pub fn reshape(&self, new_shape: Vec<usize>) -> Option<TeleportedArray> {
        // Check that total elements match
        let old_total: usize = self.shape.iter().product();
        let new_total: usize = new_shape.iter().product();

        if old_total != new_total {
            return None;
        }

        // Can only reshape contiguous arrays without copying
        if !self.is_contiguous() {
            return None;
        }

        // Calculate new strides
        let mut new_strides = vec![0isize; new_shape.len()];
        let mut stride = self.dtype.size() as isize;
        for i in (0..new_shape.len()).rev() {
            new_strides[i] = stride;
            stride *= new_shape[i] as isize;
        }

        Some(TeleportedArray {
            data: self.data,
            shape: new_shape.clone(),
            strides: new_strides,
            dtype: self.dtype,
            byte_size: self.byte_size,
            readonly: self.readonly,
            _owner_refcount: self._owner_refcount,
            flags: ArrayFlags {
                c_contiguous: true,
                f_contiguous: new_shape.len() <= 1,
                owndata: false, // View doesn't own data
                writeable: !self.readonly,
                aligned: self.flags.aligned,
                updateifcopy: false,
            },
        })
    }

    /// Transpose the array (swap axes)
    pub fn transpose(&self) -> TeleportedArray {
        let mut new_shape = self.shape.clone();
        let mut new_strides = self.strides.clone();

        new_shape.reverse();
        new_strides.reverse();

        TeleportedArray {
            data: self.data,
            shape: new_shape.clone(),
            strides: new_strides.clone(),
            dtype: self.dtype,
            byte_size: self.byte_size,
            readonly: self.readonly,
            _owner_refcount: self._owner_refcount,
            flags: ArrayFlags {
                c_contiguous: Self::check_c_contiguous(&new_shape, &new_strides, self.dtype.size()),
                f_contiguous: Self::check_f_contiguous(&new_shape, &new_strides, self.dtype.size()),
                owndata: false,
                writeable: !self.readonly,
                aligned: self.flags.aligned,
                updateifcopy: false,
            },
        }
    }

    /// Get a flat iterator over all elements
    pub fn flat_iter(&self) -> FlatIterator<'_> {
        FlatIterator {
            array: self,
            index: 0,
            total: self.len(),
        }
    }

    // =========================================================================
    // Linear Algebra Operations
    // =========================================================================

    /// Dot product of two arrays
    ///
    /// For 1-D arrays: inner product of vectors
    /// For 2-D arrays: matrix multiplication
    /// For N-D arrays: sum product over the last axis of a and second-to-last of b
    pub fn dot(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        // Both arrays must have the same dtype
        if self.dtype != other.dtype {
            return None;
        }

        match (self.ndim(), other.ndim()) {
            // 1D dot 1D: inner product
            (1, 1) => self.dot_1d_1d(other),
            // 2D dot 1D: matrix-vector product
            (2, 1) => self.dot_2d_1d(other),
            // 1D dot 2D: vector-matrix product
            (1, 2) => self.dot_1d_2d(other),
            // 2D dot 2D: matrix multiplication
            (2, 2) => self.matmul(other),
            _ => None, // Higher dimensions not yet supported
        }
    }

    /// Inner product of two 1D vectors
    fn dot_1d_1d(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        if self.shape[0] != other.shape[0] {
            return None;
        }

        let n = self.shape[0];

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let mut sum = 0.0f64;
                    for i in 0..n {
                        sum += a[i] * b[i];
                    }
                    Some(TeleportedArray::from_vec(vec![sum], vec![]))
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let mut sum = 0.0f32;
                    for i in 0..n {
                        sum += a[i] * b[i];
                    }
                    Some(TeleportedArray::from_vec(vec![sum], vec![]))
                }
                DType::Int64 => {
                    let a: &[i64] = self.as_slice();
                    let b: &[i64] = other.as_slice();
                    let mut sum = 0i64;
                    for i in 0..n {
                        sum += a[i] * b[i];
                    }
                    Some(TeleportedArray::from_vec(vec![sum], vec![]))
                }
                DType::Int32 => {
                    let a: &[i32] = self.as_slice();
                    let b: &[i32] = other.as_slice();
                    let mut sum = 0i32;
                    for i in 0..n {
                        sum += a[i] * b[i];
                    }
                    Some(TeleportedArray::from_vec(vec![sum], vec![]))
                }
                _ => None,
            }
        }
    }

    /// Matrix-vector product (2D dot 1D)
    fn dot_2d_1d(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        let m = self.shape[0]; // rows
        let n = self.shape[1]; // cols

        if n != other.shape[0] {
            return None;
        }

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let mut result = vec![0.0f64; m];

                    for i in 0..m {
                        for j in 0..n {
                            result[i] += a[i * n + j] * b[j];
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m]))
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let mut result = vec![0.0f32; m];

                    for i in 0..m {
                        for j in 0..n {
                            result[i] += a[i * n + j] * b[j];
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m]))
                }
                _ => None,
            }
        }
    }

    /// Vector-matrix product (1D dot 2D)
    fn dot_1d_2d(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        let n = self.shape[0];
        let m = other.shape[0]; // rows
        let p = other.shape[1]; // cols

        if n != m {
            return None;
        }

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let mut result = vec![0.0f64; p];

                    for j in 0..p {
                        for i in 0..n {
                            result[j] += a[i] * b[i * p + j];
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![p]))
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let mut result = vec![0.0f32; p];

                    for j in 0..p {
                        for i in 0..n {
                            result[j] += a[i] * b[i * p + j];
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![p]))
                }
                _ => None,
            }
        }
    }

    /// Matrix multiplication (2D @ 2D)
    ///
    /// Computes the matrix product of two 2D arrays.
    /// For arrays a (m x n) and b (n x p), returns result (m x p).
    pub fn matmul(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        // Both must be 2D
        if self.ndim() != 2 || other.ndim() != 2 {
            return None;
        }

        // Must have same dtype
        if self.dtype != other.dtype {
            return None;
        }

        let m = self.shape[0]; // rows of A
        let n = self.shape[1]; // cols of A = rows of B
        let p = other.shape[1]; // cols of B

        // Inner dimensions must match
        if n != other.shape[0] {
            return None;
        }

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let mut result = vec![0.0f64; m * p];

                    // Standard matrix multiplication: C[i,j] = sum(A[i,k] * B[k,j])
                    for i in 0..m {
                        for j in 0..p {
                            let mut sum = 0.0f64;
                            for k in 0..n {
                                sum += a[i * n + k] * b[k * p + j];
                            }
                            result[i * p + j] = sum;
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m, p]))
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let mut result = vec![0.0f32; m * p];

                    for i in 0..m {
                        for j in 0..p {
                            let mut sum = 0.0f32;
                            for k in 0..n {
                                sum += a[i * n + k] * b[k * p + j];
                            }
                            result[i * p + j] = sum;
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m, p]))
                }
                DType::Int64 => {
                    let a: &[i64] = self.as_slice();
                    let b: &[i64] = other.as_slice();
                    let mut result = vec![0i64; m * p];

                    for i in 0..m {
                        for j in 0..p {
                            let mut sum = 0i64;
                            for k in 0..n {
                                sum += a[i * n + k] * b[k * p + j];
                            }
                            result[i * p + j] = sum;
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m, p]))
                }
                DType::Int32 => {
                    let a: &[i32] = self.as_slice();
                    let b: &[i32] = other.as_slice();
                    let mut result = vec![0i32; m * p];

                    for i in 0..m {
                        for j in 0..p {
                            let mut sum = 0i32;
                            for k in 0..n {
                                sum += a[i * n + k] * b[k * p + j];
                            }
                            result[i * p + j] = sum;
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m, p]))
                }
                _ => None,
            }
        }
    }

    /// Outer product of two 1D vectors
    ///
    /// For vectors a (m,) and b (n,), returns matrix (m x n) where
    /// result[i,j] = a[i] * b[j]
    pub fn outer(&self, other: &TeleportedArray) -> Option<TeleportedArray> {
        // Both must be 1D
        if self.ndim() != 1 || other.ndim() != 1 {
            return None;
        }

        // Must have same dtype
        if self.dtype != other.dtype {
            return None;
        }

        let m = self.shape[0];
        let n = other.shape[0];

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let a: &[f64] = self.as_slice();
                    let b: &[f64] = other.as_slice();
                    let mut result = vec![0.0f64; m * n];

                    for i in 0..m {
                        for j in 0..n {
                            result[i * n + j] = a[i] * b[j];
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m, n]))
                }
                DType::Float32 => {
                    let a: &[f32] = self.as_slice();
                    let b: &[f32] = other.as_slice();
                    let mut result = vec![0.0f32; m * n];

                    for i in 0..m {
                        for j in 0..n {
                            result[i * n + j] = a[i] * b[j];
                        }
                    }

                    Some(TeleportedArray::from_vec(result, vec![m, n]))
                }
                _ => None,
            }
        }
    }

    /// Sum of all elements
    pub fn sum(&self) -> Option<f64> {
        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let slice: &[f64] = self.as_slice();
                    Some(slice.iter().sum())
                }
                DType::Float32 => {
                    let slice: &[f32] = self.as_slice();
                    Some(slice.iter().map(|&x| x as f64).sum())
                }
                DType::Int64 => {
                    let slice: &[i64] = self.as_slice();
                    Some(slice.iter().map(|&x| x as f64).sum())
                }
                DType::Int32 => {
                    let slice: &[i32] = self.as_slice();
                    Some(slice.iter().map(|&x| x as f64).sum())
                }
                _ => None,
            }
        }
    }

    /// Mean of all elements
    pub fn mean(&self) -> Option<f64> {
        let sum = self.sum()?;
        let len = self.len();
        if len == 0 {
            None
        } else {
            Some(sum / len as f64)
        }
    }

    /// Maximum element
    pub fn max(&self) -> Option<f64> {
        if self.is_empty() {
            return None;
        }

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let slice: &[f64] = self.as_slice();
                    slice.iter().cloned().reduce(f64::max)
                }
                DType::Float32 => {
                    let slice: &[f32] = self.as_slice();
                    slice.iter().cloned().reduce(f32::max).map(|x| x as f64)
                }
                DType::Int64 => {
                    let slice: &[i64] = self.as_slice();
                    slice.iter().cloned().max().map(|x| x as f64)
                }
                DType::Int32 => {
                    let slice: &[i32] = self.as_slice();
                    slice.iter().cloned().max().map(|x| x as f64)
                }
                _ => None,
            }
        }
    }

    /// Minimum element
    pub fn min(&self) -> Option<f64> {
        if self.is_empty() {
            return None;
        }

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let slice: &[f64] = self.as_slice();
                    slice.iter().cloned().reduce(f64::min)
                }
                DType::Float32 => {
                    let slice: &[f32] = self.as_slice();
                    slice.iter().cloned().reduce(f32::min).map(|x| x as f64)
                }
                DType::Int64 => {
                    let slice: &[i64] = self.as_slice();
                    slice.iter().cloned().min().map(|x| x as f64)
                }
                DType::Int32 => {
                    let slice: &[i32] = self.as_slice();
                    slice.iter().cloned().min().map(|x| x as f64)
                }
                _ => None,
            }
        }
    }

    /// Euclidean norm (L2 norm) of the array
    pub fn norm(&self) -> Option<f64> {
        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let slice: &[f64] = self.as_slice();
                    let sum_sq: f64 = slice.iter().map(|x| x * x).sum();
                    Some(sum_sq.sqrt())
                }
                DType::Float32 => {
                    let slice: &[f32] = self.as_slice();
                    let sum_sq: f64 = slice.iter().map(|&x| (x as f64) * (x as f64)).sum();
                    Some(sum_sq.sqrt())
                }
                _ => None,
            }
        }
    }

    /// Trace of a 2D matrix (sum of diagonal elements)
    pub fn trace(&self) -> Option<f64> {
        if self.ndim() != 2 {
            return None;
        }

        let n = self.shape[0].min(self.shape[1]);
        let cols = self.shape[1];

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let slice: &[f64] = self.as_slice();
                    let mut sum = 0.0f64;
                    for i in 0..n {
                        sum += slice[i * cols + i];
                    }
                    Some(sum)
                }
                DType::Float32 => {
                    let slice: &[f32] = self.as_slice();
                    let mut sum = 0.0f64;
                    for i in 0..n {
                        sum += slice[i * cols + i] as f64;
                    }
                    Some(sum)
                }
                DType::Int64 => {
                    let slice: &[i64] = self.as_slice();
                    let mut sum = 0i64;
                    for i in 0..n {
                        sum += slice[i * cols + i];
                    }
                    Some(sum as f64)
                }
                DType::Int32 => {
                    let slice: &[i32] = self.as_slice();
                    let mut sum = 0i32;
                    for i in 0..n {
                        sum += slice[i * cols + i];
                    }
                    Some(sum as f64)
                }
                _ => None,
            }
        }
    }

    /// Extract diagonal elements from a 2D matrix
    pub fn diag(&self) -> Option<TeleportedArray> {
        if self.ndim() != 2 {
            return None;
        }

        let n = self.shape[0].min(self.shape[1]);
        let cols = self.shape[1];

        unsafe {
            match self.dtype {
                DType::Float64 => {
                    let slice: &[f64] = self.as_slice();
                    let mut result = vec![0.0f64; n];
                    for i in 0..n {
                        result[i] = slice[i * cols + i];
                    }
                    Some(TeleportedArray::from_vec(result, vec![n]))
                }
                DType::Float32 => {
                    let slice: &[f32] = self.as_slice();
                    let mut result = vec![0.0f32; n];
                    for i in 0..n {
                        result[i] = slice[i * cols + i];
                    }
                    Some(TeleportedArray::from_vec(result, vec![n]))
                }
                DType::Int64 => {
                    let slice: &[i64] = self.as_slice();
                    let mut result = vec![0i64; n];
                    for i in 0..n {
                        result[i] = slice[i * cols + i];
                    }
                    Some(TeleportedArray::from_vec(result, vec![n]))
                }
                DType::Int32 => {
                    let slice: &[i32] = self.as_slice();
                    let mut result = vec![0i32; n];
                    for i in 0..n {
                        result[i] = slice[i * cols + i];
                    }
                    Some(TeleportedArray::from_vec(result, vec![n]))
                }
                _ => None,
            }
        }
    }

    /// Create an identity matrix of size n x n
    pub fn eye(n: usize, dtype: DType) -> TeleportedArray {
        let mut arr = TeleportedArray::zeros(vec![n, n], dtype);

        unsafe {
            match dtype {
                DType::Float64 => {
                    let slice = arr.as_mut_slice::<f64>().unwrap();
                    for i in 0..n {
                        slice[i * n + i] = 1.0;
                    }
                }
                DType::Float32 => {
                    let slice = arr.as_mut_slice::<f32>().unwrap();
                    for i in 0..n {
                        slice[i * n + i] = 1.0;
                    }
                }
                DType::Int64 => {
                    let slice = arr.as_mut_slice::<i64>().unwrap();
                    for i in 0..n {
                        slice[i * n + i] = 1;
                    }
                }
                DType::Int32 => {
                    let slice = arr.as_mut_slice::<i32>().unwrap();
                    for i in 0..n {
                        slice[i * n + i] = 1;
                    }
                }
                _ => {}
            }
        }

        arr
    }
}

/// Value types for array interface dictionary
#[derive(Debug, Clone)]
pub enum ArrayInterfaceValue {
    Shape(Vec<usize>),
    TypeStr(String),
    Data(usize, bool),
    Strides(Vec<isize>),
    Version(u32),
    Descr(Vec<(String, String)>),
    Mask(usize),
    Offset(usize),
}

/// Iterator over array elements in flat (row-major) order
pub struct FlatIterator<'a> {
    array: &'a TeleportedArray,
    index: usize,
    total: usize,
}

impl<'a> Iterator for FlatIterator<'a> {
    type Item = usize; // Returns byte offset

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.total {
            return None;
        }

        // Calculate multi-dimensional index from flat index
        let mut remaining = self.index;
        let mut offset = 0isize;

        for (dim_size, stride) in self.array.shape.iter().zip(self.array.strides.iter()).rev() {
            let dim_index = remaining % dim_size;
            remaining /= dim_size;
            offset += (dim_index as isize) * stride;
        }

        self.index += 1;
        Some(offset as usize)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.total - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for FlatIterator<'a> {}

impl Drop for TeleportedArray {
    fn drop(&mut self) {
        // Only deallocate if we own the data (refcount == 1)
        // In real implementation, this would check if data came from Python
        if self._owner_refcount == 1 && !self.data.is_null() {
            // Don't deallocate - data may be owned by Python
            // This is a simplified implementation
        }
    }
}

/// View into a teleported array (borrowed, zero-copy)
#[allow(dead_code)]
pub struct TeleportedArrayView<'a, T> {
    data: &'a [T],
    shape: &'a [usize],
    strides: &'a [isize],
}

impl<'a, T> TeleportedArrayView<'a, T> {
    /// Create a view from a teleported array
    ///
    /// # Safety
    /// T must match the array's dtype
    pub unsafe fn from_array(array: &'a TeleportedArray) -> Self {
        Self {
            data: array.as_slice(),
            shape: &array.shape,
            strides: &array.strides,
        }
    }

    /// Get the data slice
    pub fn data(&self) -> &[T] {
        self.data
    }

    /// Get the shape
    pub fn shape(&self) -> &[usize] {
        self.shape
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtype_size() {
        assert_eq!(DType::Float64.size(), 8);
        assert_eq!(DType::Float32.size(), 4);
        assert_eq!(DType::Int64.size(), 8);
        assert_eq!(DType::Int32.size(), 4);
        assert_eq!(DType::Bool.size(), 1);
        assert_eq!(DType::Float16.size(), 2);
        assert_eq!(DType::Complex128.size(), 16);
    }

    #[test]
    fn test_dtype_numpy_str() {
        let endian = if cfg!(target_endian = "little") {
            '<'
        } else {
            '>'
        };
        assert_eq!(DType::Float64.numpy_dtype_str(), format!("{}d8", endian));
        assert_eq!(DType::Float32.numpy_dtype_str(), format!("{}f4", endian));
        assert_eq!(DType::Int32.numpy_dtype_str(), format!("{}i4", endian));
    }

    #[test]
    fn test_dtype_from_numpy_str() {
        assert_eq!(DType::from_numpy_str("<f8"), Some(DType::Float64));
        assert_eq!(DType::from_numpy_str(">f4"), Some(DType::Float32));
        assert_eq!(DType::from_numpy_str("<i4"), Some(DType::Int32));
        assert_eq!(DType::from_numpy_str("|b1"), Some(DType::Int8));
        assert_eq!(DType::from_numpy_str("?"), Some(DType::Bool));
    }

    #[test]
    fn test_dtype_type_char() {
        assert_eq!(DType::Float64.type_char(), 'd');
        assert_eq!(DType::Float32.type_char(), 'f');
        assert_eq!(DType::Int32.type_char(), 'i');
        assert_eq!(DType::Bool.type_char(), '?');
    }

    #[test]
    fn test_teleported_array_from_vec() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![4]);

        assert_eq!(array.shape(), &[4]);
        assert_eq!(array.dtype(), DType::Float64);
        assert_eq!(array.len(), 4);
        assert!(array.is_contiguous());
        assert!(array.is_c_contiguous());
    }

    #[test]
    fn test_teleported_array_as_slice() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![4]);

        unsafe {
            let slice: &[f64] = array.as_slice();
            assert_eq!(slice, &[1.0, 2.0, 3.0, 4.0]);
        }
    }

    #[test]
    fn test_add_scalar() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let mut array = TeleportedArray::from_vec(data, vec![4]);

        assert!(array.add_scalar_f64(10.0));

        unsafe {
            let slice: &[f64] = array.as_slice();
            assert_eq!(slice, &[11.0, 12.0, 13.0, 14.0]);
        }
    }

    #[test]
    fn test_mul_scalar() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let mut array = TeleportedArray::from_vec(data, vec![4]);

        assert!(array.mul_scalar_f64(2.0));

        unsafe {
            let slice: &[f64] = array.as_slice();
            assert_eq!(slice, &[2.0, 4.0, 6.0, 8.0]);
        }
    }

    #[test]
    fn test_2d_array() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let array = TeleportedArray::from_vec(data, vec![2, 3]);

        assert_eq!(array.shape(), &[2, 3]);
        assert_eq!(array.ndim(), 2);
        assert_eq!(array.len(), 6);
    }

    #[test]
    fn test_zeros() {
        let array = TeleportedArray::zeros(vec![2, 3], DType::Float64);

        assert_eq!(array.shape(), &[2, 3]);
        assert_eq!(array.dtype(), DType::Float64);
        assert_eq!(array.len(), 6);

        unsafe {
            let slice: &[f64] = array.as_slice();
            assert!(slice.iter().all(|&x| x == 0.0));
        }
    }

    #[test]
    fn test_ones() {
        let array = TeleportedArray::ones(vec![2, 3], DType::Float64);

        assert_eq!(array.shape(), &[2, 3]);
        assert_eq!(array.dtype(), DType::Float64);

        unsafe {
            let slice: &[f64] = array.as_slice();
            assert!(slice.iter().all(|&x| x == 1.0));
        }
    }

    #[test]
    fn test_array_interface() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![2, 2]);

        let interface = array.array_interface();

        assert_eq!(interface.shape, vec![2, 2]);
        assert_eq!(interface.version, 3);
        assert!(!interface.data.1); // Not read-only
        assert!(interface.strides.is_none()); // C-contiguous
    }

    #[test]
    fn test_array_interface_dict() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![2, 2]);

        let dict = array.array_interface_dict();

        assert!(dict.contains_key("shape"));
        assert!(dict.contains_key("typestr"));
        assert!(dict.contains_key("data"));
        assert!(dict.contains_key("version"));
    }

    #[test]
    fn test_reshape() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let array = TeleportedArray::from_vec(data, vec![2, 3]);

        let reshaped = array.reshape(vec![3, 2]).unwrap();

        assert_eq!(reshaped.shape(), &[3, 2]);
        assert_eq!(reshaped.len(), 6);
    }

    #[test]
    fn test_reshape_invalid() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![2, 2]);

        // Wrong total elements
        assert!(array.reshape(vec![3, 2]).is_none());
    }

    #[test]
    fn test_transpose() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0];
        let array = TeleportedArray::from_vec(data, vec![2, 3]);

        let transposed = array.transpose();

        assert_eq!(transposed.shape(), &[3, 2]);
    }

    #[test]
    fn test_flat_iterator() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![2, 2]);

        let offsets: Vec<usize> = array.flat_iter().collect();

        assert_eq!(offsets.len(), 4);
    }

    #[test]
    fn test_array_flags() {
        let data = vec![1.0f64, 2.0, 3.0, 4.0];
        let array = TeleportedArray::from_vec(data, vec![2, 2]);

        let flags = array.flags();

        assert!(flags.c_contiguous);
        assert!(flags.writeable);
        assert!(flags.owndata);
    }

    #[test]
    fn test_array_flags_numpy_conversion() {
        let flags = ArrayFlags {
            c_contiguous: true,
            f_contiguous: false,
            owndata: true,
            writeable: true,
            aligned: true,
            updateifcopy: false,
        };

        let numpy_flags = flags.to_numpy_flags();
        let restored = ArrayFlags::from_numpy_flags(numpy_flags);

        assert_eq!(flags.c_contiguous, restored.c_contiguous);
        assert_eq!(flags.f_contiguous, restored.f_contiguous);
        assert_eq!(flags.owndata, restored.owndata);
        assert_eq!(flags.writeable, restored.writeable);
    }

    #[test]
    fn test_dtype_kind_char() {
        assert_eq!(DType::Bool.kind_char(), 'b');
        assert_eq!(DType::Int32.kind_char(), 'i');
        assert_eq!(DType::UInt32.kind_char(), 'u');
        assert_eq!(DType::Float64.kind_char(), 'f');
        assert_eq!(DType::Complex128.kind_char(), 'c');
        assert_eq!(DType::String(10).kind_char(), 'S');
        assert_eq!(DType::Unicode(10).kind_char(), 'U');
    }

    #[test]
    fn test_dtype_byte_order() {
        // Single-byte types have no byte order
        assert_eq!(DType::Bool.byte_order_char(), '|');
        assert_eq!(DType::Int8.byte_order_char(), '|');
        assert_eq!(DType::UInt8.byte_order_char(), '|');

        // Multi-byte types have byte order
        let expected = if cfg!(target_endian = "little") {
            '<'
        } else {
            '>'
        };
        assert_eq!(DType::Int32.byte_order_char(), expected);
        assert_eq!(DType::Float64.byte_order_char(), expected);
    }

    #[test]
    fn test_dtype_from_kind_and_size() {
        assert_eq!(DType::from_kind_and_size('i', 4), Some(DType::Int32));
        assert_eq!(DType::from_kind_and_size('f', 8), Some(DType::Float64));
        assert_eq!(DType::from_kind_and_size('c', 16), Some(DType::Complex128));
        assert_eq!(DType::from_kind_and_size('S', 20), Some(DType::String(20)));
        assert_eq!(DType::from_kind_and_size('x', 4), None);
    }

    #[test]
    fn test_dtype_promotion() {
        // Float promotion
        assert_eq!(DType::Int32.promote_with(&DType::Float32), Some(DType::Float32));
        assert_eq!(DType::Float32.promote_with(&DType::Float64), Some(DType::Float64));

        // Integer promotion
        assert_eq!(DType::Int16.promote_with(&DType::Int32), Some(DType::Int32));
        assert_eq!(DType::UInt8.promote_with(&DType::Int16), Some(DType::Int16));

        // Complex promotion (complex takes precedence)
        assert_eq!(DType::Complex64.promote_with(&DType::Complex128), Some(DType::Complex128));
        assert_eq!(DType::Complex64.promote_with(&DType::Complex64), Some(DType::Complex64));
    }

    #[test]
    fn test_dtype_compatibility() {
        // Same type
        assert!(DType::Float64.is_compatible_with(&DType::Float64));

        // Numeric types
        assert!(DType::Int32.is_compatible_with(&DType::Float64));
        assert!(DType::Float32.is_compatible_with(&DType::Int64));

        // String types
        assert!(DType::String(10).is_compatible_with(&DType::String(20)));

        // Incompatible
        assert!(!DType::String(10).is_compatible_with(&DType::Int32));
    }

    #[test]
    fn test_dtype_is_datetime() {
        assert!(DType::DateTime64.is_datetime());
        assert!(DType::TimeDelta64.is_datetime());
        assert!(!DType::Float64.is_datetime());
    }

    #[test]
    fn test_dtype_is_string() {
        assert!(DType::String(10).is_string());
        assert!(DType::Unicode(10).is_string());
        assert!(!DType::Float64.is_string());
    }

    #[test]
    fn test_struct_field() {
        let field = StructField::new("x", DType::Float64, 0);
        assert_eq!(field.name, "x");
        assert_eq!(field.dtype, DType::Float64);
        assert_eq!(field.offset, 0);
        assert_eq!(field.size(), 8);

        let (name, typestr) = field.to_descr();
        assert_eq!(name, "x");
        assert!(typestr.contains("d") || typestr.contains("f8"));
    }

    #[test]
    fn test_struct_field_array() {
        let field = StructField::array("data", DType::Float32, 0, vec![3, 3]);
        assert_eq!(field.size(), 4 * 9); // 9 float32s
    }

    #[test]
    fn test_structured_dtype() {
        let dtype = StructuredDType::packed(vec![
            ("x".to_string(), DType::Float64),
            ("y".to_string(), DType::Float64),
            ("z".to_string(), DType::Float64),
        ]);

        assert_eq!(dtype.fields.len(), 3);
        assert_eq!(dtype.itemsize, 24); // 3 * 8 bytes

        assert_eq!(dtype.field_offset("x"), Some(0));
        assert_eq!(dtype.field_offset("y"), Some(8));
        assert_eq!(dtype.field_offset("z"), Some(16));
    }

    #[test]
    fn test_array_interface_builder() {
        let interface = ArrayInterface::new(vec![2, 3], "<f8".to_string(), 0x1000, false)
            .with_strides(vec![24, 8])
            .with_descr(vec![("value".to_string(), "<f8".to_string())]);

        assert_eq!(interface.shape, vec![2, 3]);
        assert_eq!(interface.strides, Some(vec![24, 8]));
        assert!(interface.descr.is_some());
    }

    #[test]
    fn test_array_interface_to_dict() {
        let interface = ArrayInterface::new(vec![4], "<f8".to_string(), 0x1000, true);

        let dict = interface.to_dict();

        assert!(dict.contains_key("shape"));
        assert!(dict.contains_key("typestr"));
        assert!(dict.contains_key("data"));
        assert!(dict.contains_key("version"));
    }

    #[test]
    fn test_array_add() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);
        let b = TeleportedArray::from_vec(vec![10.0f64, 20.0, 30.0, 40.0], vec![4]);

        let result = a.add(&b).unwrap();

        unsafe {
            let slice: &[f64] = result.as_slice();
            assert_eq!(slice, &[11.0, 22.0, 33.0, 44.0]);
        }
    }

    #[test]
    fn test_array_sub() {
        let a = TeleportedArray::from_vec(vec![10.0f64, 20.0, 30.0, 40.0], vec![4]);
        let b = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);

        let result = a.sub(&b).unwrap();

        unsafe {
            let slice: &[f64] = result.as_slice();
            assert_eq!(slice, &[9.0, 18.0, 27.0, 36.0]);
        }
    }

    #[test]
    fn test_array_mul() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);
        let b = TeleportedArray::from_vec(vec![2.0f64, 3.0, 4.0, 5.0], vec![4]);

        let result = a.mul(&b).unwrap();

        unsafe {
            let slice: &[f64] = result.as_slice();
            assert_eq!(slice, &[2.0, 6.0, 12.0, 20.0]);
        }
    }

    #[test]
    fn test_array_div() {
        let a = TeleportedArray::from_vec(vec![10.0f64, 20.0, 30.0, 40.0], vec![4]);
        let b = TeleportedArray::from_vec(vec![2.0f64, 4.0, 5.0, 8.0], vec![4]);

        let result = a.div(&b).unwrap();

        unsafe {
            let slice: &[f64] = result.as_slice();
            assert_eq!(slice, &[5.0, 5.0, 6.0, 5.0]);
        }
    }

    #[test]
    fn test_array_ops_shape_mismatch() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0], vec![3]);
        let b = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);

        assert!(a.add(&b).is_none());
        assert!(a.sub(&b).is_none());
        assert!(a.mul(&b).is_none());
        assert!(a.div(&b).is_none());
    }

    #[test]
    fn test_shapes_broadcastable() {
        // Same shapes
        assert!(TeleportedArray::shapes_broadcastable(&[3, 4], &[3, 4]));

        // Broadcasting with 1
        assert!(TeleportedArray::shapes_broadcastable(&[3, 4], &[1, 4]));
        assert!(TeleportedArray::shapes_broadcastable(&[3, 4], &[3, 1]));
        assert!(TeleportedArray::shapes_broadcastable(&[3, 4], &[4]));

        // Not broadcastable
        assert!(!TeleportedArray::shapes_broadcastable(&[3, 4], &[2, 4]));
        assert!(!TeleportedArray::shapes_broadcastable(&[3, 4], &[3, 5]));
    }

    #[test]
    fn test_broadcast_shape() {
        assert_eq!(TeleportedArray::broadcast_shape(&[3, 4], &[1, 4]), Some(vec![3, 4]));
        assert_eq!(TeleportedArray::broadcast_shape(&[3, 1], &[1, 4]), Some(vec![3, 4]));
        assert_eq!(TeleportedArray::broadcast_shape(&[4], &[3, 4]), Some(vec![3, 4]));
        assert_eq!(TeleportedArray::broadcast_shape(&[3, 4], &[2, 4]), None);
    }

    #[test]
    fn test_broadcast_to() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0], vec![1, 3]);

        let broadcast = a.broadcast_to(&[4, 3]).unwrap();

        assert_eq!(broadcast.shape(), &[4, 3]);
        assert!(broadcast.readonly); // Broadcast views are read-only
    }

    #[test]
    fn test_slice_axis0() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0], vec![3, 2]);

        let slice = a.slice_axis0(1, 3).unwrap();

        assert_eq!(slice.shape(), &[2, 2]);

        unsafe {
            let data: &[f64] = slice.as_slice();
            // Should contain [3.0, 4.0, 5.0, 6.0]
            assert_eq!(data[0], 3.0);
            assert_eq!(data[1], 4.0);
        }
    }

    #[test]
    fn test_get_set_flat() {
        let mut a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);

        unsafe {
            assert_eq!(a.get_flat::<f64>(0), Some(1.0));
            assert_eq!(a.get_flat::<f64>(3), Some(4.0));
            assert_eq!(a.get_flat::<f64>(10), None);

            assert!(a.set_flat(1, 99.0f64));
            assert_eq!(a.get_flat::<f64>(1), Some(99.0));
        }
    }

    #[test]
    fn test_get_set_at() {
        let mut a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);

        unsafe {
            assert_eq!(a.get_at::<f64>(&[0, 0]), Some(1.0));
            assert_eq!(a.get_at::<f64>(&[0, 2]), Some(3.0));
            assert_eq!(a.get_at::<f64>(&[1, 0]), Some(4.0));
            assert_eq!(a.get_at::<f64>(&[1, 2]), Some(6.0));

            // Out of bounds
            assert_eq!(a.get_at::<f64>(&[2, 0]), None);

            // Set value
            assert!(a.set_at(&[0, 1], 99.0f64));
            assert_eq!(a.get_at::<f64>(&[0, 1]), Some(99.0));
        }
    }

    #[test]
    fn test_array_copy() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);
        let b = a.copy();

        assert_eq!(a.shape(), b.shape());
        assert_eq!(a.dtype(), b.dtype());

        unsafe {
            let a_data: &[f64] = a.as_slice();
            let b_data: &[f64] = b.as_slice();
            assert_eq!(a_data, b_data);
        }

        // Verify they're independent (different memory)
        assert_ne!(a.data_ptr(), b.data_ptr());
    }

    // =========================================================================
    // Linear Algebra Tests
    // =========================================================================

    #[test]
    fn test_dot_1d_1d() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0], vec![3]);
        let b = TeleportedArray::from_vec(vec![4.0f64, 5.0, 6.0], vec![3]);

        let result = a.dot(&b).unwrap();

        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert_eq!(result.shape(), &[] as &[usize]); // Scalar result
        unsafe {
            let slice: &[f64] = result.as_slice();
            assert!((slice[0] - 32.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_dot_2d_1d() {
        // Matrix (2x3) dot vector (3,)
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
        let b = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0], vec![3]);

        let result = a.dot(&b).unwrap();

        // [1*1 + 2*2 + 3*3, 4*1 + 5*2 + 6*3] = [14, 32]
        assert_eq!(result.shape(), &[2]);
        unsafe {
            let slice: &[f64] = result.as_slice();
            assert!((slice[0] - 14.0).abs() < 1e-10);
            assert!((slice[1] - 32.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_dot_1d_2d() {
        // Vector (3,) dot matrix (3x2)
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0], vec![3]);
        let b = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0], vec![3, 2]);

        let result = a.dot(&b).unwrap();

        // [1*1 + 2*3 + 3*5, 1*2 + 2*4 + 3*6] = [22, 28]
        assert_eq!(result.shape(), &[2]);
        unsafe {
            let slice: &[f64] = result.as_slice();
            assert!((slice[0] - 22.0).abs() < 1e-10);
            assert!((slice[1] - 28.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_matmul() {
        // Matrix (2x3) @ matrix (3x2)
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
        let b = TeleportedArray::from_vec(vec![7.0f64, 8.0, 9.0, 10.0, 11.0, 12.0], vec![3, 2]);

        let result = a.matmul(&b).unwrap();

        // Result should be (2x2)
        // [1*7+2*9+3*11, 1*8+2*10+3*12]   = [58, 64]
        // [4*7+5*9+6*11, 4*8+5*10+6*12]   = [139, 154]
        assert_eq!(result.shape(), &[2, 2]);
        unsafe {
            let slice: &[f64] = result.as_slice();
            assert!((slice[0] - 58.0).abs() < 1e-10);
            assert!((slice[1] - 64.0).abs() < 1e-10);
            assert!((slice[2] - 139.0).abs() < 1e-10);
            assert!((slice[3] - 154.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_matmul_dimension_mismatch() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![2, 2]);
        let b = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0], vec![3, 2]);

        // Inner dimensions don't match (2 != 3)
        assert!(a.matmul(&b).is_none());
    }

    #[test]
    fn test_outer() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0], vec![3]);
        let b = TeleportedArray::from_vec(vec![4.0f64, 5.0], vec![2]);

        let result = a.outer(&b).unwrap();

        // Result should be (3x2)
        // [[1*4, 1*5], [2*4, 2*5], [3*4, 3*5]] = [[4, 5], [8, 10], [12, 15]]
        assert_eq!(result.shape(), &[3, 2]);
        unsafe {
            let slice: &[f64] = result.as_slice();
            assert_eq!(slice, &[4.0, 5.0, 8.0, 10.0, 12.0, 15.0]);
        }
    }

    #[test]
    fn test_sum() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);
        assert!((a.sum().unwrap() - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_mean() {
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![4]);
        assert!((a.mean().unwrap() - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_max_min() {
        let a = TeleportedArray::from_vec(vec![3.0f64, 1.0, 4.0, 1.0, 5.0, 9.0], vec![6]);
        assert!((a.max().unwrap() - 9.0).abs() < 1e-10);
        assert!((a.min().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_norm() {
        let a = TeleportedArray::from_vec(vec![3.0f64, 4.0], vec![2]);
        // sqrt(3^2 + 4^2) = sqrt(9 + 16) = sqrt(25) = 5
        assert!((a.norm().unwrap() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_trace() {
        let a = TeleportedArray::from_vec(
            vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
            vec![3, 3],
        );
        // trace = 1 + 5 + 9 = 15
        assert!((a.trace().unwrap() - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_diag() {
        let a = TeleportedArray::from_vec(
            vec![1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
            vec![3, 3],
        );

        let diag = a.diag().unwrap();

        assert_eq!(diag.shape(), &[3]);
        unsafe {
            let slice: &[f64] = diag.as_slice();
            assert_eq!(slice, &[1.0, 5.0, 9.0]);
        }
    }

    #[test]
    fn test_eye() {
        let eye = TeleportedArray::eye(3, DType::Float64);

        assert_eq!(eye.shape(), &[3, 3]);
        unsafe {
            let slice: &[f64] = eye.as_slice();
            assert_eq!(slice, &[1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);
        }
    }

    #[test]
    fn test_matmul_identity() {
        // A @ I = A
        let a = TeleportedArray::from_vec(vec![1.0f64, 2.0, 3.0, 4.0], vec![2, 2]);
        let eye = TeleportedArray::eye(2, DType::Float64);

        let result = a.matmul(&eye).unwrap();

        unsafe {
            let a_slice: &[f64] = a.as_slice();
            let r_slice: &[f64] = result.as_slice();
            for i in 0..4 {
                assert!((a_slice[i] - r_slice[i]).abs() < 1e-10);
            }
        }
    }
}
