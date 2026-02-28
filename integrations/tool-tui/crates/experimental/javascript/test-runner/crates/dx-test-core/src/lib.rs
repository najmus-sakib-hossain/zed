//! DX Test Core - Binary formats and types
//!
//! Core data structures for the DX test runner.
//! All structures are #[repr(C, packed)] for zero-copy binary serialization.

use bytemuck::{Pod, Zeroable};
use std::time::Duration;

pub mod coverage;
pub mod mock;
pub mod snapshot;

pub use coverage::{
    BranchCoverage, BranchType, CodeInstrumenter, CoverageCollector, CoverageReporter,
    CoverageSummary, CoverageThresholds, FileCoverage, FileCoverageSummary, FunctionCoverage,
    StatementCoverage, ThresholdFailure, ThresholdResult,
};
pub use mock::{FakeTimers, Jest, MockCall, MockFunction, MockValue, ModuleMockRegistry, Spy};
pub use snapshot::{
    generate_diff, InlineSnapshot, Snapshot, SnapshotFile, SnapshotManager, SnapshotResult,
    SnapshotSerializer, SnapshotValue,
};

/// Magic bytes for DX Test Layout format
pub const DXTL_MAGIC: [u8; 4] = *b"DXTL";

/// Current format version
pub const DXTL_VERSION: u32 = 1;

/// Test layout cache header
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TestLayoutHeader {
    /// Magic: "DXTL"
    pub magic: [u8; 4],
    /// Version
    pub version: u32,
    /// Project source hash (for invalidation)
    pub source_hash: u128,
    /// Number of test files
    pub file_count: u32,
    /// Total test count
    pub test_count: u32,
    /// Total suite count
    pub suite_count: u32,
    /// Offset to file entries
    pub files_offset: u64,
    /// Offset to test entries
    pub tests_offset: u64,
    /// Offset to suite entries
    pub suites_offset: u64,
    /// Created timestamp
    pub created_at: u64,
}

impl TestLayoutHeader {
    pub fn new(source_hash: u128, file_count: u32, test_count: u32, suite_count: u32) -> Self {
        Self {
            magic: DXTL_MAGIC,
            version: DXTL_VERSION,
            source_hash,
            file_count,
            test_count,
            suite_count,
            files_offset: 0,
            tests_offset: 0,
            suites_offset: 0,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.magic == DXTL_MAGIC && self.version == DXTL_VERSION
    }
}

/// Pre-compiled test file entry
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct TestFileEntry {
    /// File path hash
    pub path_hash: u64,
    /// DXT file hash (content-addressed)
    pub dxt_hash: u128,
    /// Offset in DXT pool file
    pub dxt_offset: u64,
    /// DXT size
    pub dxt_size: u32,
    /// Number of tests in file
    pub test_count: u32,
    /// First test index
    pub first_test: u32,
}

/// Flattened test entry (no tree traversal needed!)
#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct FlatTestEntry {
    /// Test name hash
    pub name_hash: u64,
    /// Full name offset (including suite path)
    pub full_name_offset: u32,
    /// Full name length
    pub full_name_len: u16,
    /// Parent file index
    pub file_idx: u32,
    /// Bytecode offset in DXT pool
    pub bytecode_offset: u64,
    /// Bytecode length
    pub bytecode_len: u32,
    /// Flags (skip, only, concurrent, etc.)
    pub flags: u16,
    /// Timeout in ms
    pub timeout_ms: u32,
    /// Expected assertions
    pub assertion_count: u16,
    /// Dependencies (other tests that must run first)
    pub deps_bitmap: u64,
}

/// Test execution status
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed = 0,
    Failed = 1,
    Skipped = 2,
    Todo = 3,
    Running = 4,
}

/// Test execution result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub status: TestStatus,
    pub duration: Duration,
    pub assertions: u16,
    pub first_failure: Option<u16>,
    pub error_message: Option<String>,
}

impl TestResult {
    pub fn passed(duration: Duration, assertions: u16) -> Self {
        Self {
            status: TestStatus::Passed,
            duration,
            assertions,
            first_failure: None,
            error_message: None,
        }
    }

    pub fn failed(duration: Duration, assertions: u16, failure_idx: u16, message: String) -> Self {
        Self {
            status: TestStatus::Failed,
            duration,
            assertions,
            first_failure: Some(failure_idx),
            error_message: Some(message),
        }
    }
}

/// Assertion result
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub passed: bool,
    pub opcode: u8,
    pub index: u16,
    pub message: Option<String>,
}

/// Test bytecode opcodes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOpcode {
    // Stack operations
    Nop = 0x00,
    Push = 0x01,
    Pop = 0x02,
    Dup = 0x03,
    Swap = 0x04,

    // Local variables
    LoadLocal = 0x10,
    StoreLocal = 0x11,
    LoadConst = 0x12,

    // Fast paths for common types
    PushInt = 0x20,
    PushFloat = 0x21,
    PushTrue = 0x22,
    PushFalse = 0x23,
    PushNull = 0x24,
    PushUndefined = 0x25,
    PushString = 0x26,

    // Arithmetic
    Add = 0x30,
    Sub = 0x31,
    Mul = 0x32,
    Div = 0x33,
    Mod = 0x34,
    Neg = 0x35,

    // Comparison
    Eq = 0x40,
    Ne = 0x41,
    Lt = 0x42,
    Le = 0x43,
    Gt = 0x44,
    Ge = 0x45,
    StrictEq = 0x46,
    StrictNe = 0x47,

    // Assertions (the magic!)
    AssertEq = 0x50,
    AssertDeepEq = 0x51,
    AssertTruthy = 0x52,
    AssertFalsy = 0x53,
    AssertNull = 0x54,
    AssertDefined = 0x55,
    AssertContains = 0x56,
    AssertLength = 0x57,
    AssertMatch = 0x58,
    AssertThrows = 0x59,
    AssertSnapshot = 0x5A,
    AssertType = 0x5B,
    AssertInstanceOf = 0x5C,
    AssertCloseTo = 0x5D,
    AssertArrayEq = 0x5E,
    AssertStringEq = 0x5F,

    // Negation
    Not = 0x60,

    // Control flow
    Jump = 0x70,
    JumpIf = 0x71,
    JumpIfNot = 0x72,
    Call = 0x73,
    Return = 0x74,

    // Objects and arrays
    NewObject = 0x80,
    NewArray = 0x81,
    GetProp = 0x82,
    SetProp = 0x83,
    GetIndex = 0x84,
    SetIndex = 0x85,

    // Test result
    TestPass = 0xF0,
    TestFail = 0xF1,
    TestSkip = 0xF2,

    // End
    End = 0xFF,
}

/// NaN-boxed value (compatible with dx-js-runtime)
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Value(pub u64);

impl Value {
    const NAN_BITS: u64 = 0x7FF8_0000_0000_0000;
    const TAG_INT: u64 = 0x0001_0000_0000_0000;
    const TAG_BOOL: u64 = 0x0002_0000_0000_0000;
    const TAG_NULL: u64 = 0x0003_0000_0000_0000;
    const TAG_UNDEFINED: u64 = 0x0004_0000_0000_0000;
    #[allow(dead_code)]
    const TAG_STRING: u64 = 0x0005_0000_0000_0000;
    #[allow(dead_code)]
    const TAG_OBJECT: u64 = 0x0006_0000_0000_0000;
    #[allow(dead_code)]
    const TAG_ARRAY: u64 = 0x0007_0000_0000_0000;

    #[inline(always)]
    pub fn int(n: i32) -> Self {
        Self(Self::NAN_BITS | Self::TAG_INT | (n as u32 as u64))
    }

    #[inline(always)]
    pub fn float(f: f64) -> Self {
        Self(f.to_bits())
    }

    #[inline(always)]
    pub fn bool(b: bool) -> Self {
        Self(Self::NAN_BITS | Self::TAG_BOOL | (b as u64))
    }

    #[inline(always)]
    pub fn null() -> Self {
        Self(Self::NAN_BITS | Self::TAG_NULL)
    }

    #[inline(always)]
    pub fn undefined() -> Self {
        Self(Self::NAN_BITS | Self::TAG_UNDEFINED)
    }

    #[inline(always)]
    pub fn is_int(&self) -> bool {
        (self.0 & 0xFFFF_0000_0000_0000) == (Self::NAN_BITS | Self::TAG_INT)
    }

    #[inline(always)]
    pub fn is_bool(&self) -> bool {
        (self.0 & 0xFFFF_0000_0000_0000) == (Self::NAN_BITS | Self::TAG_BOOL)
    }

    #[inline(always)]
    pub fn is_null(&self) -> bool {
        (self.0 & 0xFFFF_0000_0000_0000) == (Self::NAN_BITS | Self::TAG_NULL)
    }

    #[inline(always)]
    pub fn is_undefined(&self) -> bool {
        (self.0 & 0xFFFF_0000_0000_0000) == (Self::NAN_BITS | Self::TAG_UNDEFINED)
    }

    #[inline(always)]
    pub fn as_int(&self) -> Option<i32> {
        if self.is_int() {
            Some((self.0 & 0xFFFF_FFFF) as i32)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_bool(&self) -> Option<bool> {
        if self.is_bool() {
            Some((self.0 & 1) != 0)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn as_float(&self) -> Option<f64> {
        let f = f64::from_bits(self.0);
        if !f.is_nan() {
            Some(f)
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn is_truthy(&self) -> bool {
        match self.0 & 0xFFFF_0000_0000_0000 {
            x if x == Self::NAN_BITS | Self::TAG_NULL => false,
            x if x == Self::NAN_BITS | Self::TAG_UNDEFINED => false,
            x if x == Self::NAN_BITS | Self::TAG_BOOL => (self.0 & 1) != 0,
            x if x == Self::NAN_BITS | Self::TAG_INT => (self.0 as i32) != 0,
            _ => {
                let f = f64::from_bits(self.0);
                f != 0.0 && !f.is_nan()
            }
        }
    }
}
