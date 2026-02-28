//! FFI type definitions and marshaling.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// FFI type definitions for C ABI compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiType {
    /// Void type
    Void,
    /// Boolean type (C99 _Bool)
    Bool,
    /// Signed 8-bit integer (char/int8_t)
    I8,
    /// Signed 16-bit integer (short/int16_t)
    I16,
    /// Signed 32-bit integer (int/int32_t)
    I32,
    /// Signed 64-bit integer (long long/int64_t)
    I64,
    /// Unsigned 8-bit integer (unsigned char/uint8_t)
    U8,
    /// Unsigned 16-bit integer (unsigned short/uint16_t)
    U16,
    /// Unsigned 32-bit integer (unsigned int/uint32_t)
    U32,
    /// Unsigned 64-bit integer (unsigned long long/uint64_t)
    U64,
    /// 32-bit float
    F32,
    /// 64-bit float
    F64,
    /// Pointer type (void*)
    Pointer,
    /// C string type (char*)
    CString,
    /// Size type (size_t)
    USize,
    /// Signed size type (ssize_t)
    ISize,
}

impl FfiType {
    /// Get the size of this type in bytes.
    pub fn size(&self) -> usize {
        match self {
            FfiType::Void => 0,
            FfiType::Bool => 1,
            FfiType::I8 | FfiType::U8 => 1,
            FfiType::I16 | FfiType::U16 => 2,
            FfiType::I32 | FfiType::U32 | FfiType::F32 => 4,
            FfiType::I64 | FfiType::U64 | FfiType::F64 => 8,
            FfiType::Pointer | FfiType::CString | FfiType::USize | FfiType::ISize => {
                std::mem::size_of::<usize>()
            }
        }
    }

    /// Get the alignment of this type in bytes.
    pub fn alignment(&self) -> usize {
        self.size().max(1)
    }
}

/// FFI value that can be passed to/from C functions.
#[derive(Debug, Clone)]
pub enum FfiValue {
    /// Void (no value)
    Void,
    /// Boolean
    Bool(bool),
    /// Signed 8-bit integer
    I8(i8),
    /// Signed 16-bit integer
    I16(i16),
    /// Signed 32-bit integer
    I32(i32),
    /// Signed 64-bit integer
    I64(i64),
    /// Unsigned 8-bit integer
    U8(u8),
    /// Unsigned 16-bit integer
    U16(u16),
    /// Unsigned 32-bit integer
    U32(u32),
    /// Unsigned 64-bit integer
    U64(u64),
    /// 32-bit float
    F32(f32),
    /// 64-bit float
    F64(f64),
    /// Pointer
    Pointer(usize),
    /// String (owned)
    String(String),
    /// Buffer (owned bytes)
    Buffer(Vec<u8>),
}

impl FfiValue {
    /// Get the FFI type of this value.
    pub fn ffi_type(&self) -> FfiType {
        match self {
            FfiValue::Void => FfiType::Void,
            FfiValue::Bool(_) => FfiType::Bool,
            FfiValue::I8(_) => FfiType::I8,
            FfiValue::I16(_) => FfiType::I16,
            FfiValue::I32(_) => FfiType::I32,
            FfiValue::I64(_) => FfiType::I64,
            FfiValue::U8(_) => FfiType::U8,
            FfiValue::U16(_) => FfiType::U16,
            FfiValue::U32(_) => FfiType::U32,
            FfiValue::U64(_) => FfiType::U64,
            FfiValue::F32(_) => FfiType::F32,
            FfiValue::F64(_) => FfiType::F64,
            FfiValue::Pointer(_) => FfiType::Pointer,
            FfiValue::String(_) => FfiType::CString,
            FfiValue::Buffer(_) => FfiType::Pointer,
        }
    }

    /// Convert to raw pointer value.
    pub fn as_raw(&self) -> u64 {
        match self {
            FfiValue::Void => 0,
            FfiValue::Bool(v) => *v as u64,
            FfiValue::I8(v) => *v as u64,
            FfiValue::I16(v) => *v as u64,
            FfiValue::I32(v) => *v as u64,
            FfiValue::I64(v) => *v as u64,
            FfiValue::U8(v) => *v as u64,
            FfiValue::U16(v) => *v as u64,
            FfiValue::U32(v) => *v as u64,
            FfiValue::U64(v) => *v,
            FfiValue::F32(v) => v.to_bits() as u64,
            FfiValue::F64(v) => v.to_bits(),
            FfiValue::Pointer(v) => *v as u64,
            FfiValue::String(_) | FfiValue::Buffer(_) => 0, // Need special handling
        }
    }
}

/// C string wrapper for safe FFI string handling.
pub struct CStringWrapper {
    inner: CString,
}

impl CStringWrapper {
    /// Create a new C string from a Rust string.
    pub fn new(s: &str) -> Result<Self, std::ffi::NulError> {
        Ok(Self {
            inner: CString::new(s)?,
        })
    }

    /// Get the raw pointer to the C string.
    pub fn as_ptr(&self) -> *const c_char {
        self.inner.as_ptr()
    }

    /// Convert to owned String.
    pub fn into_string(self) -> String {
        self.inner.into_string().unwrap_or_default()
    }
}

/// Read a C string from a pointer.
///
/// # Safety
/// The pointer must point to a valid null-terminated C string.
pub unsafe fn read_cstring(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

/// Struct layout definition for FFI.
#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Field definitions
    pub fields: Vec<StructField>,
    /// Total size in bytes
    pub size: usize,
    /// Alignment in bytes
    pub alignment: usize,
}

/// Struct field definition.
#[derive(Debug, Clone)]
pub struct StructField {
    /// Field name
    pub name: String,
    /// Field type
    pub ffi_type: FfiType,
    /// Offset in bytes from struct start
    pub offset: usize,
}

impl StructLayout {
    /// Create a new struct layout builder.
    pub fn builder() -> StructLayoutBuilder {
        StructLayoutBuilder::new()
    }

    /// Get a field by name.
    pub fn field(&self, name: &str) -> Option<&StructField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// Builder for struct layouts.
pub struct StructLayoutBuilder {
    fields: Vec<StructField>,
    current_offset: usize,
    max_alignment: usize,
}

impl StructLayoutBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            current_offset: 0,
            max_alignment: 1,
        }
    }

    /// Add a field to the struct.
    pub fn field(mut self, name: impl Into<String>, ffi_type: FfiType) -> Self {
        let alignment = ffi_type.alignment();
        let size = ffi_type.size();

        // Align the current offset
        let padding = (alignment - (self.current_offset % alignment)) % alignment;
        let offset = self.current_offset + padding;

        self.fields.push(StructField {
            name: name.into(),
            ffi_type,
            offset,
        });

        self.current_offset = offset + size;
        self.max_alignment = self.max_alignment.max(alignment);
        self
    }

    /// Build the struct layout.
    pub fn build(self) -> StructLayout {
        // Add trailing padding for alignment
        let padding =
            (self.max_alignment - (self.current_offset % self.max_alignment)) % self.max_alignment;
        let size = self.current_offset + padding;

        StructLayout {
            fields: self.fields,
            size,
            alignment: self.max_alignment,
        }
    }
}

impl Default for StructLayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_type_sizes() {
        assert_eq!(FfiType::I8.size(), 1);
        assert_eq!(FfiType::I16.size(), 2);
        assert_eq!(FfiType::I32.size(), 4);
        assert_eq!(FfiType::I64.size(), 8);
        assert_eq!(FfiType::F32.size(), 4);
        assert_eq!(FfiType::F64.size(), 8);
    }

    #[test]
    fn test_struct_layout() {
        // Simulate a C struct: { int32_t a; int8_t b; int32_t c; }
        let layout = StructLayout::builder()
            .field("a", FfiType::I32)
            .field("b", FfiType::I8)
            .field("c", FfiType::I32)
            .build();

        assert_eq!(layout.field("a").unwrap().offset, 0);
        assert_eq!(layout.field("b").unwrap().offset, 4);
        assert_eq!(layout.field("c").unwrap().offset, 8); // Aligned to 4 bytes
        assert_eq!(layout.size, 12);
        assert_eq!(layout.alignment, 4);
    }

    #[test]
    fn test_cstring_wrapper() {
        let s = CStringWrapper::new("hello").unwrap();
        assert!(!s.as_ptr().is_null());
    }
}
