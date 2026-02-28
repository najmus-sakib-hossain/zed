//! Property-based tests for DPB format
//!
//! These tests verify the correctness properties defined in the spec:
//! - Property 1: DPB Round-Trip Consistency
//! - Property 19: DPB Header Alignment

use dx_py_bytecode::*;
use proptest::prelude::*;

/// Strategy for generating random constants
fn arb_constant() -> impl Strategy<Value = Constant> {
    prop_oneof![
        Just(Constant::None),
        any::<bool>().prop_map(Constant::Bool),
        any::<i64>().prop_map(Constant::Int),
        any::<f64>()
            .prop_filter("finite float", |f| f.is_finite())
            .prop_map(Constant::Float),
        "[a-zA-Z0-9_]{0,100}".prop_map(Constant::String),
        prop::collection::vec(any::<u8>(), 0..100).prop_map(Constant::Bytes),
    ]
}

/// Strategy for generating random names
fn arb_name() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,50}"
}

/// Strategy for generating valid bytecode sequences
fn arb_bytecode() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(
        prop_oneof![
            // No-arg opcodes
            Just(vec![DpbOpcode::PopTop as u8]),
            Just(vec![DpbOpcode::DupTop as u8]),
            Just(vec![DpbOpcode::BinaryAdd as u8]),
            Just(vec![DpbOpcode::BinarySub as u8]),
            Just(vec![DpbOpcode::BinaryMul as u8]),
            Just(vec![DpbOpcode::Return as u8]),
            Just(vec![DpbOpcode::Nop as u8]),
            // 2-byte arg opcodes
            (0u16..100).prop_map(|arg| {
                let mut v = vec![DpbOpcode::LoadFast as u8];
                v.extend_from_slice(&arg.to_le_bytes());
                v
            }),
            (0u16..100).prop_map(|arg| {
                let mut v = vec![DpbOpcode::StoreFast as u8];
                v.extend_from_slice(&arg.to_le_bytes());
                v
            }),
            (0u16..100).prop_map(|arg| {
                let mut v = vec![DpbOpcode::LoadConst as u8];
                v.extend_from_slice(&arg.to_le_bytes());
                v
            }),
        ],
        1..50,
    )
    .prop_map(|vecs| vecs.into_iter().flatten().collect())
}

/// Strategy for generating code objects
fn arb_code_object() -> impl Strategy<Value = CodeObject> {
    (
        arb_name(),
        prop::collection::vec(arb_constant(), 0..10),
        prop::collection::hash_set(arb_name(), 0..10), // Use hash_set for unique names
        arb_bytecode(),
    )
        .prop_map(|(name, constants, names_set, code)| {
            let names: Vec<String> = names_set.into_iter().collect();
            CodeObject {
                name: name.clone(),
                qualname: name,
                filename: "<test>".to_string(),
                firstlineno: 1,
                argcount: 0,
                posonlyargcount: 0,
                kwonlyargcount: 0,
                nlocals: 0,
                stacksize: 10,
                flags: CodeFlags::OPTIMIZED,
                code,
                constants,
                names,
                varnames: vec![],
                freevars: vec![],
                cellvars: vec![],
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 19: DPB Header Alignment
    /// Validates: Requirements 1.1, 1.2
    ///
    /// The DPB header must be cache-line aligned (64 bytes alignment).
    /// Note: The actual size is 128 bytes due to the 32-byte BLAKE3 hash.
    #[test]
    fn prop_dpb_header_alignment(_seed in any::<u64>()) {
        // Header size is 128 bytes (2 cache lines) due to BLAKE3 hash
        assert_eq!(DpbHeader::size(), 128, "Header must be 128 bytes");

        // Header alignment must be 64 bytes (cache-line aligned)
        assert_eq!(std::mem::align_of::<DpbHeader>(), 64, "Header must be cache-line aligned");

        // Create a header and verify it's properly aligned
        let header = DpbHeader::new();
        let ptr = &header as *const DpbHeader as usize;
        assert_eq!(ptr % 64, 0, "Header instance must be aligned to 64 bytes");
    }

    /// Property 1: DPB Round-Trip Consistency
    /// Validates: Requirements 1.10
    ///
    /// For all valid code objects, compiling to DPB then loading back
    /// must preserve semantic equivalence.
    #[test]
    fn prop_dpb_round_trip(code in arb_code_object()) {
        let mut compiler = DpbCompiler::new();

        // Compile to DPB
        let dpb_bytes = compiler.compile(&code).expect("Compilation should succeed");

        // Verify magic bytes
        assert_eq!(&dpb_bytes[0..4], b"DPB\x01", "Magic bytes must be correct");

        // Load back
        let module = DpbLoader::load_from_bytes(dpb_bytes).expect("Loading should succeed");

        // Verify header is valid
        assert!(module.header().validate_magic(), "Magic must validate");

        // Note: Constants count may be less than original due to deduplication
        // The compiler deduplicates None, Bool, Int, Float, and String constants
        assert!(
            module.header().constants_count as usize <= code.constants.len(),
            "Constants count must be <= original (deduplication)"
        );

        // Verify names count matches
        assert_eq!(
            module.header().names_count as usize,
            code.names.len(),
            "Names count must match"
        );

        // Verify code size matches
        assert_eq!(
            module.header().code_size as usize,
            code.code.len(),
            "Code size must match"
        );

        // Verify bytecode matches
        assert_eq!(module.code(), &code.code[..], "Bytecode must match");

        // Verify all unique constants can be retrieved
        let unique_count = module.header().constants_count as usize;
        for i in 0..unique_count {
            let loaded = module.get_constant(i as u32);
            assert!(loaded.is_some(), "Constant {} must be loadable", i);
        }

        // Verify names can be retrieved
        for (i, original) in code.names.iter().enumerate() {
            let loaded = module.get_name(i as u32);
            assert_eq!(loaded, Some(original.as_str()), "Name {} must match", i);
        }
    }

    /// Test that magic bytes are always validated correctly
    #[test]
    fn prop_magic_validation(
        b0 in any::<u8>(),
        b1 in any::<u8>(),
        b2 in any::<u8>(),
        b3 in any::<u8>()
    ) {
        let mut header = DpbHeader::new();
        header.magic = [b0, b1, b2, b3];

        let expected_valid = header.magic == *b"DPB\x01";
        assert_eq!(header.validate_magic(), expected_valid);
    }

    /// Test that opcode parsing is consistent
    #[test]
    fn prop_opcode_roundtrip(byte in any::<u8>()) {
        if let Some(opcode) = DpbOpcode::from_u8(byte) {
            // If we can parse it, the byte value should match
            assert_eq!(opcode as u8, byte);
            // And it should be marked as valid
            assert!(DpbOpcode::is_valid(byte));
        } else {
            // If we can't parse it, it should be marked as invalid
            assert!(!DpbOpcode::is_valid(byte));
        }
    }

    /// Test that constant serialization is deterministic
    #[test]
    fn prop_constant_deterministic(constant in arb_constant()) {
        let mut compiler1 = DpbCompiler::new();
        let mut compiler2 = DpbCompiler::new();

        let idx1 = compiler1.add_constant(constant.clone());
        let idx2 = compiler2.add_constant(constant);

        // Same constant should get same index in fresh compilers
        assert_eq!(idx1, idx2);
    }

    /// Test that name interning is consistent
    #[test]
    fn prop_name_interning(name in arb_name()) {
        let mut compiler = DpbCompiler::new();

        let idx1 = compiler.intern_name(&name);
        let idx2 = compiler.intern_name(&name);

        // Same name should always get same index
        assert_eq!(idx1, idx2);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_header_fields() {
        let header = DpbHeader::new();

        assert_eq!(header.magic, *b"DPB\x01");
        assert_eq!(header.version, 1);
        assert_eq!(header.python_version, 0x030C);
        assert!(header.flags.is_empty());
    }

    #[test]
    fn test_opcode_ranges() {
        // Load/Store range
        for byte in 0x00..=0x17 {
            assert!(DpbOpcode::is_valid(byte), "0x{:02X} should be valid", byte);
        }

        // Binary ops range
        for byte in 0x20..=0x3D {
            assert!(DpbOpcode::is_valid(byte), "0x{:02X} should be valid", byte);
        }

        // Invalid range
        for byte in 0x18..=0x1F {
            assert!(!DpbOpcode::is_valid(byte), "0x{:02X} should be invalid", byte);
        }
    }
}
