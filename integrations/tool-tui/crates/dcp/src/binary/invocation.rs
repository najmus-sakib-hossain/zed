//! Tool invocation structures for zero-copy argument passing.

use crate::DCPError;

/// Zero-copy tool invocation
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolInvocation {
    /// Pre-resolved tool ID (compile-time lookup)
    pub tool_id: u32,
    /// Bitfield describing argument positions and types
    pub arg_layout: u64,
    /// Offset into shared memory for arguments
    pub args_offset: u32,
    /// Length of arguments in shared memory
    pub args_len: u32,
}

/// Argument types encoded in arg_layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ArgType {
    Null = 0,
    Bool = 1,
    I32 = 2,
    I64 = 3,
    F64 = 4,
    String = 5,
    Bytes = 6,
    Array = 7,
    Object = 8,
}

impl ArgType {
    /// Convert from u8 to ArgType
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Null),
            1 => Some(Self::Bool),
            2 => Some(Self::I32),
            3 => Some(Self::I64),
            4 => Some(Self::F64),
            5 => Some(Self::String),
            6 => Some(Self::Bytes),
            7 => Some(Self::Array),
            8 => Some(Self::Object),
            _ => None,
        }
    }
}

impl ToolInvocation {
    /// Size of the struct in bytes
    pub const SIZE: usize = 24; // 4 + 8 + 4 + 4 + padding

    /// Maximum number of arguments that can be encoded in arg_layout
    /// Each argument uses 4 bits for type, so 64 bits / 4 = 16 args
    pub const MAX_ARGS: usize = 16;

    /// Create a new tool invocation
    pub fn new(tool_id: u32, arg_layout: u64, args_offset: u32, args_len: u32) -> Self {
        Self {
            tool_id,
            arg_layout,
            args_offset,
            args_len,
        }
    }

    /// Parse from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least SIZE bytes
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: ToolInvocation is repr(C) with no padding
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Get the type of argument at the given index (0-15)
    pub fn get_arg_type(&self, index: usize) -> Option<ArgType> {
        if index >= Self::MAX_ARGS {
            return None;
        }
        let shift = index * 4;
        let type_bits = ((self.arg_layout >> shift) & 0xF) as u8;
        ArgType::from_u8(type_bits)
    }

    /// Set the type of argument at the given index (0-15)
    pub fn set_arg_type(&mut self, index: usize, arg_type: ArgType) {
        if index >= Self::MAX_ARGS {
            return;
        }
        let shift = index * 4;
        // Clear the 4 bits at this position
        self.arg_layout &= !(0xF << shift);
        // Set the new type
        self.arg_layout |= (arg_type as u64) << shift;
    }

    /// Count the number of non-null arguments
    pub fn arg_count(&self) -> usize {
        let mut count = 0;
        for i in 0..Self::MAX_ARGS {
            if let Some(arg_type) = self.get_arg_type(i) {
                if arg_type != ArgType::Null {
                    count += 1;
                } else {
                    break; // Null terminates the argument list
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invocation_size() {
        assert_eq!(std::mem::size_of::<ToolInvocation>(), ToolInvocation::SIZE);
    }

    #[test]
    fn test_invocation_round_trip() {
        let inv = ToolInvocation::new(42, 0x12345678, 100, 200);
        let bytes = inv.as_bytes();
        let parsed = ToolInvocation::from_bytes(bytes).unwrap();

        assert_eq!(parsed.tool_id, 42);
        assert_eq!(parsed.arg_layout, 0x12345678);
        assert_eq!(parsed.args_offset, 100);
        assert_eq!(parsed.args_len, 200);
    }

    #[test]
    fn test_arg_type_encoding() {
        let mut inv = ToolInvocation::new(1, 0, 0, 0);

        inv.set_arg_type(0, ArgType::String);
        inv.set_arg_type(1, ArgType::I32);
        inv.set_arg_type(2, ArgType::Bool);

        assert_eq!(inv.get_arg_type(0), Some(ArgType::String));
        assert_eq!(inv.get_arg_type(1), Some(ArgType::I32));
        assert_eq!(inv.get_arg_type(2), Some(ArgType::Bool));
        assert_eq!(inv.get_arg_type(3), Some(ArgType::Null));
    }

    #[test]
    fn test_arg_count() {
        let mut inv = ToolInvocation::new(1, 0, 0, 0);
        assert_eq!(inv.arg_count(), 0);

        inv.set_arg_type(0, ArgType::String);
        inv.set_arg_type(1, ArgType::I32);
        assert_eq!(inv.arg_count(), 2);
    }
}
