//! DPB Format definitions - Header and Opcodes
//!
//! The DPB format uses a 64-byte cache-line aligned header for O(1) section access.

use bitflags::bitflags;

/// Magic bytes for DPB format identification
pub const DPB_MAGIC: [u8; 4] = *b"DPB\x01";

/// Current DPB format version
pub const DPB_VERSION: u32 = 1;

/// DPB Header - 64 bytes, cache-line aligned
///
/// This header is designed for zero-copy memory-mapped access.
/// All offsets are relative to the start of the file.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct DpbHeader {
    /// Magic bytes "DPB\x01" for format identification
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// Target Python version (e.g., 3.12 = 0x030C)
    pub python_version: u32,
    /// Optimization flags
    pub flags: DpbFlags,

    // Section offsets (all u32 for zero-copy access)
    /// Bytecode section offset
    pub code_offset: u32,
    /// Constants pool offset
    pub constants_offset: u32,
    /// Name table offset (interned strings)
    pub names_offset: u32,
    /// Pre-resolved symbols offset
    pub symbols_offset: u32,
    /// Type annotations offset for JIT hints
    pub types_offset: u32,
    /// Debug info offset (line numbers, etc.)
    pub debug_offset: u32,

    // Sizes
    /// Size of bytecode section in bytes
    pub code_size: u32,
    /// Number of constants
    pub constants_count: u32,
    /// Number of names
    pub names_count: u32,

    /// BLAKE3 content hash for integrity verification
    pub content_hash: [u8; 32],
}

impl DpbHeader {
    /// Create a new DPB header with default values
    pub fn new() -> Self {
        Self {
            magic: DPB_MAGIC,
            version: DPB_VERSION,
            python_version: 0x030C, // Python 3.12
            flags: DpbFlags::empty(),
            code_offset: 0,
            constants_offset: 0,
            names_offset: 0,
            symbols_offset: 0,
            types_offset: 0,
            debug_offset: 0,
            code_size: 0,
            constants_count: 0,
            names_count: 0,
            content_hash: [0u8; 32],
        }
    }

    /// Validate the magic bytes
    #[inline]
    pub fn validate_magic(&self) -> bool {
        self.magic == DPB_MAGIC
    }

    /// Validate the header
    pub fn validate(&self) -> Result<(), DpbError> {
        if !self.validate_magic() {
            return Err(DpbError::InvalidMagic);
        }
        if self.version > DPB_VERSION {
            return Err(DpbError::UnsupportedVersion(self.version));
        }
        Ok(())
    }

    /// Get the size of the header in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

impl Default for DpbHeader {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    /// DPB optimization flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DpbFlags: u32 {
        /// Code has been optimized
        const OPTIMIZED = 0x0001;
        /// Contains type annotations
        const HAS_TYPES = 0x0002;
        /// Contains debug info
        const HAS_DEBUG = 0x0004;
        /// Uses extended opcodes
        const EXTENDED_OPCODES = 0x0008;
        /// Pre-resolved all symbols
        const SYMBOLS_RESOLVED = 0x0010;
        /// Contains async/await code
        const HAS_ASYNC = 0x0020;
        /// Contains generator code
        const HAS_GENERATORS = 0x0040;
    }
}

/// DPB Opcode - 256 opcodes with fixed sizes for computed goto dispatch
///
/// Opcodes are organized into ranges:
/// - 0x00-0x1F: Load/Store operations
/// - 0x20-0x3F: Binary operations
/// - 0x40-0x4F: Comparison operations
/// - 0x50-0x6F: Control flow
/// - 0x70-0x7F: Function calls
/// - 0x80-0x8F: Object creation
/// - 0x90-0x9F: Exception handling
/// - 0xA0-0xAF: Async operations
/// - 0xF0-0xFF: Special/Extended
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DpbOpcode {
    // Load/Store (0x00-0x1F)
    /// Load local variable by index
    LoadFast = 0x00,
    /// Store to local variable by index
    StoreFast = 0x01,
    /// Load constant from pool
    LoadConst = 0x02,
    /// Load global (pre-resolved)
    LoadGlobal = 0x03,
    /// Store to global
    StoreGlobal = 0x04,
    /// Load attribute
    LoadAttr = 0x05,
    /// Store attribute
    StoreAttr = 0x06,
    /// Load subscript (a[b])
    LoadSubscr = 0x07,
    /// Store subscript (a[b] = c)
    StoreSubscr = 0x08,
    /// Delete local variable
    DeleteFast = 0x09,
    /// Delete global
    DeleteGlobal = 0x0A,
    /// Delete attribute
    DeleteAttr = 0x0B,
    /// Delete subscript
    DeleteSubscr = 0x0C,
    /// Load name (fallback)
    LoadName = 0x0D,
    /// Store name (fallback)
    StoreName = 0x0E,
    /// Load closure variable
    LoadDeref = 0x0F,
    /// Store closure variable
    StoreDeref = 0x10,
    /// Load from enclosing scope
    LoadClassDeref = 0x11,
    /// Duplicate top of stack
    DupTop = 0x12,
    /// Duplicate top two items
    DupTopTwo = 0x13,
    /// Rotate top N items
    RotN = 0x14,
    /// Pop top of stack
    PopTop = 0x15,
    /// Swap top two items
    Swap = 0x16,
    /// Copy item at offset to top
    Copy = 0x17,

    // Binary operations (0x20-0x3F)
    /// a + b
    BinaryAdd = 0x20,
    /// a - b
    BinarySub = 0x21,
    /// a * b
    BinaryMul = 0x22,
    /// a / b
    BinaryDiv = 0x23,
    /// a // b
    BinaryFloorDiv = 0x24,
    /// a % b
    BinaryMod = 0x25,
    /// a ** b
    BinaryPow = 0x26,
    /// a & b
    BinaryAnd = 0x27,
    /// a | b
    BinaryOr = 0x28,
    /// a ^ b
    BinaryXor = 0x29,
    /// a << b
    BinaryLshift = 0x2A,
    /// a >> b
    BinaryRshift = 0x2B,
    /// a @ b (matrix multiply)
    BinaryMatMul = 0x2C,
    /// -a
    UnaryNeg = 0x2D,
    /// +a
    UnaryPos = 0x2E,
    /// ~a
    UnaryInvert = 0x2F,
    /// not a
    UnaryNot = 0x30,
    /// In-place add
    InplaceAdd = 0x31,
    /// In-place subtract
    InplaceSub = 0x32,
    /// In-place multiply
    InplaceMul = 0x33,
    /// In-place divide
    InplaceDiv = 0x34,
    /// In-place floor divide
    InplaceFloorDiv = 0x35,
    /// In-place modulo
    InplaceMod = 0x36,
    /// In-place power
    InplacePow = 0x37,
    /// In-place and
    InplaceAnd = 0x38,
    /// In-place or
    InplaceOr = 0x39,
    /// In-place xor
    InplaceXor = 0x3A,
    /// In-place left shift
    InplaceLshift = 0x3B,
    /// In-place right shift
    InplaceRshift = 0x3C,
    /// In-place matrix multiply
    InplaceMatMul = 0x3D,

    // Comparison (0x40-0x4F)
    /// a < b
    CompareLt = 0x40,
    /// a <= b
    CompareLe = 0x41,
    /// a == b
    CompareEq = 0x42,
    /// a != b
    CompareNe = 0x43,
    /// a > b
    CompareGt = 0x44,
    /// a >= b
    CompareGe = 0x45,
    /// a is b
    CompareIs = 0x46,
    /// a is not b
    CompareIsNot = 0x47,
    /// a in b
    CompareIn = 0x48,
    /// a not in b
    CompareNotIn = 0x49,
    /// Exception match
    ExceptionMatch = 0x4A,

    // Control flow (0x50-0x6F)
    /// Unconditional jump
    Jump = 0x50,
    /// Jump if true
    JumpIfTrue = 0x51,
    /// Jump if false
    JumpIfFalse = 0x52,
    /// Jump if true or pop
    JumpIfTrueOrPop = 0x53,
    /// Jump if false or pop
    JumpIfFalseOrPop = 0x54,
    /// For loop iteration
    ForIter = 0x55,
    /// Return from function
    Return = 0x56,
    /// Yield value (generator)
    Yield = 0x57,
    /// Yield from (generator delegation)
    YieldFrom = 0x58,
    /// Get iterator
    GetIter = 0x59,
    /// Get length
    GetLen = 0x5A,
    /// Contains check
    ContainsOp = 0x5B,
    /// Import name
    ImportName = 0x5C,
    /// Import from
    ImportFrom = 0x5D,
    /// Import star
    ImportStar = 0x5E,
    /// Setup annotations
    SetupAnnotations = 0x5F,
    /// Pop jump if true
    PopJumpIfTrue = 0x60,
    /// Pop jump if false
    PopJumpIfFalse = 0x61,
    /// Pop jump if none
    PopJumpIfNone = 0x62,
    /// Pop jump if not none
    PopJumpIfNotNone = 0x63,

    // Function calls (0x70-0x7F)
    /// Call function
    Call = 0x70,
    /// Call with keyword arguments
    CallKw = 0x71,
    /// Call with *args/**kwargs
    CallEx = 0x72,
    /// Make function object
    MakeFunction = 0x73,
    /// Make closure
    MakeClosure = 0x74,
    /// Load method for call
    LoadMethod = 0x75,
    /// Call method
    CallMethod = 0x76,
    /// Push null for method call
    PushNull = 0x77,
    /// Keyword argument names
    KwNames = 0x78,

    // Object creation (0x80-0x8F)
    /// Build tuple
    BuildTuple = 0x80,
    /// Build list
    BuildList = 0x81,
    /// Build set
    BuildSet = 0x82,
    /// Build dict
    BuildDict = 0x83,
    /// Build string (f-string)
    BuildString = 0x84,
    /// Build slice
    BuildSlice = 0x85,
    /// List append
    ListAppend = 0x86,
    /// Set add
    SetAdd = 0x87,
    /// Map add
    MapAdd = 0x88,
    /// List extend
    ListExtend = 0x89,
    /// Set update
    SetUpdate = 0x8A,
    /// Dict update
    DictUpdate = 0x8B,
    /// Dict merge
    DictMerge = 0x8C,
    /// Unpack sequence
    UnpackSequence = 0x8D,
    /// Unpack ex (with star)
    UnpackEx = 0x8E,
    /// Format value
    FormatValue = 0x8F,

    // Exception handling (0x90-0x9F)
    /// Setup except handler - push exception handler for try block
    SetupExcept = 0x90,
    /// Pop exception handler
    PopExcept = 0x91,
    /// Raise exception
    Raise = 0x92,
    /// Re-raise current exception
    Reraise = 0x93,
    /// Push exception info
    PushExcInfo = 0x94,
    /// Check exception match
    CheckExcMatch = 0x95,
    /// Cleanup throw
    CleanupThrow = 0x96,
    /// Setup finally
    SetupFinally = 0x97,
    /// Setup with
    SetupWith = 0x98,
    /// Before with
    BeforeWith = 0x99,
    /// End send
    EndSend = 0x9A,
    /// End finally block
    EndFinally = 0x9B,

    // Async (0xA0-0xAF)
    /// Get awaitable
    GetAwaitable = 0xA0,
    /// Get async iterator
    GetAiter = 0xA1,
    /// Get async next
    GetAnext = 0xA2,
    /// End async for
    EndAsyncFor = 0xA3,
    /// Before async with
    BeforeAsyncWith = 0xA4,
    /// Setup async with
    SetupAsyncWith = 0xA5,
    /// Send (coroutine)
    Send = 0xA6,
    /// Async gen wrap
    AsyncGenWrap = 0xA7,

    // Special (0xF0-0xFF)
    /// No operation
    Nop = 0xF0,
    /// Resume (generator/coroutine)
    Resume = 0xF1,
    /// Cache (inline cache slot)
    Cache = 0xF2,
    /// Precall (call preparation)
    Precall = 0xF3,
    /// Binary op (generic)
    BinaryOp = 0xF4,
    /// Compare op (generic)
    CompareOp = 0xF5,
    /// Extended opcode (next byte is extension)
    Extended = 0xFF,
}

impl DpbOpcode {
    /// Get the opcode from a byte value
    pub fn from_u8(value: u8) -> Option<Self> {
        // Safety: We validate the value is a valid opcode
        if Self::is_valid(value) {
            Some(unsafe { std::mem::transmute::<u8, DpbOpcode>(value) })
        } else {
            None
        }
    }

    /// Check if a byte value is a valid opcode
    pub fn is_valid(value: u8) -> bool {
        matches!(value,
            0x00..=0x17 |  // Load/Store
            0x20..=0x3D |  // Binary ops
            0x40..=0x4A |  // Comparison
            0x50..=0x63 |  // Control flow
            0x70..=0x78 |  // Function calls
            0x80..=0x8F |  // Object creation
            0x90..=0x9B |  // Exception handling (including EndFinally)
            0xA0..=0xA7 |  // Async
            0xF0..=0xF5 |  // Special
            0xFF           // Extended
        )
    }

    /// Get the argument size for this opcode (0, 1, 2, or 4 bytes)
    pub fn arg_size(&self) -> usize {
        match self {
            // No argument
            Self::PopTop
            | Self::DupTop
            | Self::DupTopTwo
            | Self::Swap
            | Self::Return
            | Self::Yield
            | Self::GetIter
            | Self::GetLen
            | Self::UnaryNeg
            | Self::UnaryPos
            | Self::UnaryInvert
            | Self::UnaryNot
            | Self::BinaryAdd
            | Self::BinarySub
            | Self::BinaryMul
            | Self::BinaryDiv
            | Self::BinaryFloorDiv
            | Self::BinaryMod
            | Self::BinaryPow
            | Self::BinaryAnd
            | Self::BinaryOr
            | Self::BinaryXor
            | Self::BinaryLshift
            | Self::BinaryRshift
            | Self::BinaryMatMul
            | Self::InplaceAdd
            | Self::InplaceSub
            | Self::InplaceMul
            | Self::InplaceDiv
            | Self::InplaceFloorDiv
            | Self::InplaceMod
            | Self::InplacePow
            | Self::InplaceAnd
            | Self::InplaceOr
            | Self::InplaceXor
            | Self::InplaceLshift
            | Self::InplaceRshift
            | Self::InplaceMatMul
            | Self::CompareLt
            | Self::CompareLe
            | Self::CompareEq
            | Self::CompareNe
            | Self::CompareGt
            | Self::CompareGe
            | Self::CompareIs
            | Self::CompareIsNot
            | Self::CompareIn
            | Self::CompareNotIn
            | Self::ExceptionMatch
            | Self::Reraise
            | Self::Nop
            | Self::PushNull
            | Self::EndSend
            | Self::EndFinally
            | Self::LoadSubscr
            | Self::StoreSubscr
            | Self::DeleteSubscr => 0,

            // 1-byte argument
            Self::RotN
            | Self::Copy
            | Self::UnpackSequence
            | Self::UnpackEx
            | Self::BuildTuple
            | Self::BuildList
            | Self::BuildSet
            | Self::BuildDict
            | Self::BuildString
            | Self::BuildSlice
            | Self::FormatValue
            | Self::Raise
            | Self::Resume
            | Self::Cache
            | Self::Precall
            | Self::BinaryOp
            | Self::CompareOp
            | Self::ContainsOp => 1,

            // 2-byte argument (most common)
            Self::LoadFast
            | Self::StoreFast
            | Self::LoadConst
            | Self::LoadGlobal
            | Self::StoreGlobal
            | Self::LoadAttr
            | Self::StoreAttr
            | Self::DeleteFast
            | Self::DeleteGlobal
            | Self::DeleteAttr
            | Self::LoadName
            | Self::StoreName
            | Self::LoadDeref
            | Self::StoreDeref
            | Self::LoadClassDeref
            | Self::LoadMethod
            | Self::Jump
            | Self::JumpIfTrue
            | Self::JumpIfFalse
            | Self::JumpIfTrueOrPop
            | Self::JumpIfFalseOrPop
            | Self::PopJumpIfTrue
            | Self::PopJumpIfFalse
            | Self::PopJumpIfNone
            | Self::PopJumpIfNotNone
            | Self::ForIter
            | Self::YieldFrom
            | Self::Call
            | Self::CallKw
            | Self::CallEx
            | Self::CallMethod
            | Self::MakeFunction
            | Self::MakeClosure
            | Self::KwNames
            | Self::ListAppend
            | Self::SetAdd
            | Self::MapAdd
            | Self::ListExtend
            | Self::SetUpdate
            | Self::DictUpdate
            | Self::DictMerge
            | Self::SetupExcept
            | Self::PopExcept
            | Self::PushExcInfo
            | Self::CheckExcMatch
            | Self::CleanupThrow
            | Self::SetupFinally
            | Self::SetupWith
            | Self::BeforeWith
            | Self::ImportName
            | Self::ImportFrom
            | Self::ImportStar
            | Self::SetupAnnotations
            | Self::GetAwaitable
            | Self::GetAiter
            | Self::GetAnext
            | Self::EndAsyncFor
            | Self::BeforeAsyncWith
            | Self::SetupAsyncWith
            | Self::Send
            | Self::AsyncGenWrap => 2,

            // 4-byte argument (extended)
            Self::Extended => 4,
        }
    }

    /// Get the stack effect of this opcode (positive = push, negative = pop)
    pub fn stack_effect(&self, arg: u32) -> i32 {
        match self {
            Self::LoadFast
            | Self::LoadConst
            | Self::LoadGlobal
            | Self::LoadAttr
            | Self::LoadName
            | Self::LoadDeref
            | Self::LoadClassDeref
            | Self::DupTop
            | Self::PushNull
            | Self::LoadMethod => 1,

            Self::DupTopTwo => 2,

            Self::StoreFast
            | Self::StoreGlobal
            | Self::StoreAttr
            | Self::StoreName
            | Self::StoreDeref
            | Self::PopTop
            | Self::DeleteFast
            | Self::DeleteGlobal
            | Self::DeleteAttr => -1,

            Self::BinaryAdd
            | Self::BinarySub
            | Self::BinaryMul
            | Self::BinaryDiv
            | Self::BinaryFloorDiv
            | Self::BinaryMod
            | Self::BinaryPow
            | Self::BinaryAnd
            | Self::BinaryOr
            | Self::BinaryXor
            | Self::BinaryLshift
            | Self::BinaryRshift
            | Self::BinaryMatMul
            | Self::CompareLt
            | Self::CompareLe
            | Self::CompareEq
            | Self::CompareNe
            | Self::CompareGt
            | Self::CompareGe
            | Self::CompareIs
            | Self::CompareIsNot
            | Self::CompareIn
            | Self::CompareNotIn
            | Self::LoadSubscr
            | Self::StoreSubscr => -1,

            Self::BuildTuple | Self::BuildList | Self::BuildSet => 1 - arg as i32,
            Self::BuildDict => 1 - (2 * arg) as i32,

            Self::Call => -(arg as i32),
            Self::CallKw => -(arg as i32) - 1,

            Self::Return | Self::Yield => -1,

            _ => 0,
        }
    }
}

/// DPB Error types
#[derive(Debug, thiserror::Error)]
pub enum DpbError {
    #[error("Invalid magic bytes - not a DPB file")]
    InvalidMagic,

    #[error("Unsupported DPB version: {0}")]
    UnsupportedVersion(u32),

    #[error("Invalid opcode: 0x{0:02X}")]
    InvalidOpcode(u8),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hash mismatch - file may be corrupted")]
    HashMismatch,

    #[error("Invalid section offset")]
    InvalidOffset,

    #[error("Compilation error: {0}")]
    CompileError(String),
}

/// Constant value types in the constant pool
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantType {
    None = 0,
    Bool = 1,
    Int = 2,
    Float = 3,
    Complex = 4,
    String = 5,
    Bytes = 6,
    Tuple = 7,
    FrozenSet = 8,
    Code = 9,
    Ellipsis = 10,
}

/// A constant value in the constant pool
#[derive(Debug, Clone)]
pub enum Constant {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Complex(f64, f64),
    String(String),
    Bytes(Vec<u8>),
    Tuple(Vec<Constant>),
    FrozenSet(Vec<Constant>),
    Code(Box<CodeObject>),
    Ellipsis,
}

/// A code object containing bytecode and metadata
#[derive(Debug, Clone)]
pub struct CodeObject {
    /// Function name
    pub name: String,
    /// Qualified name
    pub qualname: String,
    /// Filename
    pub filename: String,
    /// First line number
    pub firstlineno: u32,
    /// Argument count
    pub argcount: u32,
    /// Positional-only argument count
    pub posonlyargcount: u32,
    /// Keyword-only argument count
    pub kwonlyargcount: u32,
    /// Number of local variables
    pub nlocals: u32,
    /// Stack size needed
    pub stacksize: u32,
    /// Code flags
    pub flags: CodeFlags,
    /// Bytecode
    pub code: Vec<u8>,
    /// Constants
    pub constants: Vec<Constant>,
    /// Names used
    pub names: Vec<String>,
    /// Local variable names
    pub varnames: Vec<String>,
    /// Free variable names
    pub freevars: Vec<String>,
    /// Cell variable names
    pub cellvars: Vec<String>,
}

bitflags! {
    /// Code object flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CodeFlags: u32 {
        const OPTIMIZED = 0x0001;
        const NEWLOCALS = 0x0002;
        const VARARGS = 0x0004;
        const VARKEYWORDS = 0x0008;
        const NESTED = 0x0010;
        const GENERATOR = 0x0020;
        const NOFREE = 0x0040;
        const COROUTINE = 0x0080;
        const ITERABLE_COROUTINE = 0x0100;
        const ASYNC_GENERATOR = 0x0200;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        // Header is cache-line aligned (64 bytes) but actual size is 128 bytes
        // due to the 32-byte BLAKE3 hash and other fields
        assert_eq!(DpbHeader::size(), 128);
    }

    #[test]
    fn test_header_alignment() {
        assert_eq!(std::mem::align_of::<DpbHeader>(), 64);
    }

    #[test]
    fn test_magic_validation() {
        let header = DpbHeader::new();
        assert!(header.validate_magic());

        let mut bad_header = header;
        bad_header.magic = *b"BAD\x00";
        assert!(!bad_header.validate_magic());
    }

    #[test]
    fn test_opcode_from_u8() {
        assert_eq!(DpbOpcode::from_u8(0x00), Some(DpbOpcode::LoadFast));
        assert_eq!(DpbOpcode::from_u8(0x20), Some(DpbOpcode::BinaryAdd));
        assert_eq!(DpbOpcode::from_u8(0xFF), Some(DpbOpcode::Extended));
        assert_eq!(DpbOpcode::from_u8(0xFE), None); // Invalid
    }

    #[test]
    fn test_opcode_arg_size() {
        assert_eq!(DpbOpcode::PopTop.arg_size(), 0);
        assert_eq!(DpbOpcode::LoadFast.arg_size(), 2);
        assert_eq!(DpbOpcode::BuildTuple.arg_size(), 1);
        assert_eq!(DpbOpcode::Extended.arg_size(), 4);
    }
}
