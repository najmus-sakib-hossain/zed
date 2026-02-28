//! Opcode definitions for the interpreter
//!
//! These opcodes MUST match the DpbOpcode values from dx-py-bytecode

/// Opcode enum matching DPB format exactly
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
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
    Dup = 0x12,
    /// Duplicate top two items
    DupTwo = 0x13,
    /// Rotate top N items
    RotN = 0x14,
    /// Pop top of stack
    Pop = 0x15,
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
    /// Pop jump if true
    PopJumpIfTrue = 0x60,
    /// Pop jump if false
    PopJumpIfFalse = 0x61,
    /// Pop jump if none
    PopJumpIfNone = 0x62,
    /// Pop jump if not none
    PopJumpIfNotNone = 0x63,
    /// Import name
    ImportName = 0x5C,
    /// Import from
    ImportFrom = 0x5D,
    /// Import star
    ImportStar = 0x5E,

    // Function calls (0x70-0x7F)
    /// Call function
    Call = 0x70,
    /// Call with keyword arguments
    CallKw = 0x71,
    /// Call with *args/**kwargs
    CallEx = 0x72,
    /// Make function object
    MakeFunction = 0x73,
    /// Make closure (function with free variables)
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
    /// Build class from name, bases, and namespace
    BuildClass = 0x89,
    /// Unpack sequence
    UnpackSequence = 0x8D,

    // Exception handling (0x90-0x9F)
    /// Setup try block
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
    /// Setup finally
    SetupFinally = 0x97,
    /// Setup with
    SetupWith = 0x98,
    /// Before with
    BeforeWith = 0x99,
    /// End finally
    EndFinally = 0x9B,
    /// With except start
    WithExceptStart = 0x9C,

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

impl Opcode {
    /// Decode opcode from byte
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            // Load/Store (0x00-0x1F)
            0x00 => Some(Opcode::LoadFast),
            0x01 => Some(Opcode::StoreFast),
            0x02 => Some(Opcode::LoadConst),
            0x03 => Some(Opcode::LoadGlobal),
            0x04 => Some(Opcode::StoreGlobal),
            0x05 => Some(Opcode::LoadAttr),
            0x06 => Some(Opcode::StoreAttr),
            0x07 => Some(Opcode::LoadSubscr),
            0x08 => Some(Opcode::StoreSubscr),
            0x09 => Some(Opcode::DeleteFast),
            0x0A => Some(Opcode::DeleteGlobal),
            0x0B => Some(Opcode::DeleteAttr),
            0x0C => Some(Opcode::DeleteSubscr),
            0x0D => Some(Opcode::LoadName),
            0x0E => Some(Opcode::StoreName),
            0x0F => Some(Opcode::LoadDeref),
            0x10 => Some(Opcode::StoreDeref),
            0x11 => Some(Opcode::LoadClassDeref),
            0x12 => Some(Opcode::Dup),
            0x13 => Some(Opcode::DupTwo),
            0x14 => Some(Opcode::RotN),
            0x15 => Some(Opcode::Pop),
            0x16 => Some(Opcode::Swap),
            0x17 => Some(Opcode::Copy),

            // Binary operations (0x20-0x3F)
            0x20 => Some(Opcode::BinaryAdd),
            0x21 => Some(Opcode::BinarySub),
            0x22 => Some(Opcode::BinaryMul),
            0x23 => Some(Opcode::BinaryDiv),
            0x24 => Some(Opcode::BinaryFloorDiv),
            0x25 => Some(Opcode::BinaryMod),
            0x26 => Some(Opcode::BinaryPow),
            0x27 => Some(Opcode::BinaryAnd),
            0x28 => Some(Opcode::BinaryOr),
            0x29 => Some(Opcode::BinaryXor),
            0x2A => Some(Opcode::BinaryLshift),
            0x2B => Some(Opcode::BinaryRshift),
            0x2C => Some(Opcode::BinaryMatMul),
            0x2D => Some(Opcode::UnaryNeg),
            0x2E => Some(Opcode::UnaryPos),
            0x2F => Some(Opcode::UnaryInvert),
            0x30 => Some(Opcode::UnaryNot),
            0x31 => Some(Opcode::InplaceAdd),
            0x32 => Some(Opcode::InplaceSub),
            0x33 => Some(Opcode::InplaceMul),
            0x34 => Some(Opcode::InplaceDiv),
            0x35 => Some(Opcode::InplaceFloorDiv),
            0x36 => Some(Opcode::InplaceMod),
            0x37 => Some(Opcode::InplacePow),
            0x38 => Some(Opcode::InplaceAnd),
            0x39 => Some(Opcode::InplaceOr),
            0x3A => Some(Opcode::InplaceXor),
            0x3B => Some(Opcode::InplaceLshift),
            0x3C => Some(Opcode::InplaceRshift),
            0x3D => Some(Opcode::InplaceMatMul),

            // Comparison (0x40-0x4F)
            0x40 => Some(Opcode::CompareLt),
            0x41 => Some(Opcode::CompareLe),
            0x42 => Some(Opcode::CompareEq),
            0x43 => Some(Opcode::CompareNe),
            0x44 => Some(Opcode::CompareGt),
            0x45 => Some(Opcode::CompareGe),
            0x46 => Some(Opcode::CompareIs),
            0x47 => Some(Opcode::CompareIsNot),
            0x48 => Some(Opcode::CompareIn),
            0x49 => Some(Opcode::CompareNotIn),
            0x4A => Some(Opcode::ExceptionMatch),

            // Control flow (0x50-0x6F)
            0x50 => Some(Opcode::Jump),
            0x51 => Some(Opcode::JumpIfTrue),
            0x52 => Some(Opcode::JumpIfFalse),
            0x53 => Some(Opcode::JumpIfTrueOrPop),
            0x54 => Some(Opcode::JumpIfFalseOrPop),
            0x55 => Some(Opcode::ForIter),
            0x56 => Some(Opcode::Return),
            0x57 => Some(Opcode::Yield),
            0x58 => Some(Opcode::YieldFrom),
            0x59 => Some(Opcode::GetIter),
            0x60 => Some(Opcode::PopJumpIfTrue),
            0x61 => Some(Opcode::PopJumpIfFalse),
            0x62 => Some(Opcode::PopJumpIfNone),
            0x63 => Some(Opcode::PopJumpIfNotNone),
            0x5C => Some(Opcode::ImportName),
            0x5D => Some(Opcode::ImportFrom),
            0x5E => Some(Opcode::ImportStar),

            // Function calls (0x70-0x7F)
            0x70 => Some(Opcode::Call),
            0x71 => Some(Opcode::CallKw),
            0x72 => Some(Opcode::CallEx),
            0x73 => Some(Opcode::MakeFunction),
            0x74 => Some(Opcode::MakeClosure),
            0x75 => Some(Opcode::LoadMethod),
            0x76 => Some(Opcode::CallMethod),
            0x77 => Some(Opcode::PushNull),
            0x78 => Some(Opcode::KwNames),

            // Object creation (0x80-0x8F)
            0x80 => Some(Opcode::BuildTuple),
            0x81 => Some(Opcode::BuildList),
            0x82 => Some(Opcode::BuildSet),
            0x83 => Some(Opcode::BuildDict),
            0x84 => Some(Opcode::BuildString),
            0x85 => Some(Opcode::BuildSlice),
            0x86 => Some(Opcode::ListAppend),
            0x87 => Some(Opcode::SetAdd),
            0x88 => Some(Opcode::MapAdd),
            0x89 => Some(Opcode::BuildClass),
            0x8D => Some(Opcode::UnpackSequence),

            // Exception handling (0x90-0x9F)
            0x90 => Some(Opcode::SetupExcept),
            0x91 => Some(Opcode::PopExcept),
            0x92 => Some(Opcode::Raise),
            0x93 => Some(Opcode::Reraise),
            0x94 => Some(Opcode::PushExcInfo),
            0x95 => Some(Opcode::CheckExcMatch),
            0x97 => Some(Opcode::SetupFinally),
            0x98 => Some(Opcode::SetupWith),
            0x99 => Some(Opcode::BeforeWith),
            0x9B => Some(Opcode::EndFinally),
            0x9C => Some(Opcode::WithExceptStart),

            // Async (0xA0-0xAF)
            0xA0 => Some(Opcode::GetAwaitable),
            0xA1 => Some(Opcode::GetAiter),
            0xA2 => Some(Opcode::GetAnext),
            0xA3 => Some(Opcode::EndAsyncFor),
            0xA4 => Some(Opcode::BeforeAsyncWith),
            0xA5 => Some(Opcode::SetupAsyncWith),
            0xA6 => Some(Opcode::Send),

            // Special
            0xF0 => Some(Opcode::Nop),
            0xF1 => Some(Opcode::Resume),
            0xF2 => Some(Opcode::Cache),
            0xF3 => Some(Opcode::Precall),
            0xF4 => Some(Opcode::BinaryOp),
            0xF5 => Some(Opcode::CompareOp),
            0xFF => Some(Opcode::Extended),

            _ => None,
        }
    }

    /// Check if opcode has an argument
    pub fn has_arg(&self) -> bool {
        self.arg_size() > 0
    }

    /// Get the argument size in bytes (0, 1, or 2)
    pub fn arg_size(&self) -> usize {
        match self {
            // No argument
            Opcode::Pop
            | Opcode::Dup
            | Opcode::DupTwo
            | Opcode::Swap
            | Opcode::BinaryAdd
            | Opcode::BinarySub
            | Opcode::BinaryMul
            | Opcode::BinaryDiv
            | Opcode::BinaryFloorDiv
            | Opcode::BinaryMod
            | Opcode::BinaryPow
            | Opcode::BinaryAnd
            | Opcode::BinaryOr
            | Opcode::BinaryXor
            | Opcode::BinaryLshift
            | Opcode::BinaryRshift
            | Opcode::BinaryMatMul
            | Opcode::InplaceAdd
            | Opcode::InplaceSub
            | Opcode::InplaceMul
            | Opcode::InplaceDiv
            | Opcode::InplaceFloorDiv
            | Opcode::InplaceMod
            | Opcode::InplacePow
            | Opcode::InplaceAnd
            | Opcode::InplaceOr
            | Opcode::InplaceXor
            | Opcode::InplaceLshift
            | Opcode::InplaceRshift
            | Opcode::InplaceMatMul
            | Opcode::UnaryNeg
            | Opcode::UnaryPos
            | Opcode::UnaryInvert
            | Opcode::UnaryNot
            | Opcode::CompareLt
            | Opcode::CompareLe
            | Opcode::CompareEq
            | Opcode::CompareNe
            | Opcode::CompareGt
            | Opcode::CompareGe
            | Opcode::CompareIs
            | Opcode::CompareIsNot
            | Opcode::CompareIn
            | Opcode::CompareNotIn
            | Opcode::ExceptionMatch
            | Opcode::Return
            | Opcode::Yield
            | Opcode::GetIter
            | Opcode::LoadSubscr
            | Opcode::StoreSubscr
            | Opcode::DeleteSubscr
            | Opcode::PopExcept
            | Opcode::Reraise
            | Opcode::EndFinally
            | Opcode::PushNull
            | Opcode::Nop => 0,

            // 1-byte argument
            Opcode::RotN
            | Opcode::Copy
            | Opcode::BuildTuple
            | Opcode::BuildList
            | Opcode::BuildSet
            | Opcode::BuildDict
            | Opcode::BuildString
            | Opcode::BuildSlice
            | Opcode::BuildClass
            | Opcode::UnpackSequence
            | Opcode::Raise
            | Opcode::Resume
            | Opcode::Cache
            | Opcode::Precall
            | Opcode::BinaryOp
            | Opcode::CompareOp => 1,

            // 4-byte argument (extended)
            Opcode::Extended => 4,

            // 2-byte argument (most common)
            _ => 2,
        }
    }
}
