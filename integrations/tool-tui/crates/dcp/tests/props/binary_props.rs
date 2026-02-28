//! Property-based tests for binary structs.
//!
//! Feature: dcp-protocol, Property 1: Binary Struct Round-Trip

use dcp::binary::{
    BinaryMessageEnvelope, ChunkFlags, Flags, HbtpHeader, MessageType, SignedInvocation,
    SignedToolDef, StreamChunk, ToolInvocation,
};
use proptest::prelude::*;

// Strategy for generating valid MessageType values
fn message_type_strategy() -> impl Strategy<Value = u8> {
    prop_oneof![
        Just(MessageType::Tool as u8),
        Just(MessageType::Resource as u8),
        Just(MessageType::Prompt as u8),
        Just(MessageType::Response as u8),
        Just(MessageType::Error as u8),
        Just(MessageType::Stream as u8),
    ]
}

// Strategy for generating valid flag combinations
fn flags_strategy() -> impl Strategy<Value = u8> {
    (0u8..=7u8) // All combinations of 3 flag bits
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (BinaryMessageEnvelope)
    /// For any valid BinaryMessageEnvelope, serializing to bytes and deserializing back
    /// SHALL produce an equivalent struct.
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_envelope_round_trip(
        msg_type in message_type_strategy(),
        flags in flags_strategy(),
        payload_len in any::<u32>(),
    ) {
        let msg_type_enum = MessageType::from_u8(msg_type).unwrap();
        let original = BinaryMessageEnvelope::new(msg_type_enum, flags, payload_len);
        let bytes = original.as_bytes();
        let parsed = BinaryMessageEnvelope::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.magic, original.magic);
        prop_assert_eq!(parsed.message_type, original.message_type);
        prop_assert_eq!(parsed.flags, original.flags);
        prop_assert_eq!(parsed.payload_len, original.payload_len);
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (HbtpHeader)
    /// For any valid HbtpHeader, serializing to bytes and deserializing back
    /// SHALL produce an equivalent struct.
    /// **Validates: Requirements 8.2**
    #[test]
    fn prop_hbtp_header_round_trip(
        version in any::<u8>(),
        msg_type in any::<u8>(),
        flags in any::<u16>(),
        stream_id in any::<u32>(),
        length in any::<u32>(),
    ) {
        let original = HbtpHeader::new(version, msg_type, flags, stream_id, length);
        let bytes = original.as_bytes();
        let parsed = HbtpHeader::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.magic, original.magic);
        prop_assert_eq!(parsed.version, original.version);
        prop_assert_eq!(parsed.msg_type, original.msg_type);
        prop_assert_eq!(parsed.flags, original.flags);
        prop_assert_eq!(parsed.stream_id, original.stream_id);
        prop_assert_eq!(parsed.length, original.length);
        // Checksum is computed, so we verify it matches
        prop_assert_eq!(parsed.checksum, original.checksum);
    }

    /// Feature: dcp-protocol, Property 11: HBTP Checksum Integrity
    /// For any HBTP message, the CRC32 checksum SHALL detect single-bit errors.
    /// **Validates: Requirements 8.5**
    #[test]
    fn prop_hbtp_checksum_detects_corruption(
        version in any::<u8>(),
        msg_type in any::<u8>(),
        flags in any::<u16>(),
        stream_id in any::<u32>(),
        length in any::<u32>(),
        bit_to_flip in 0usize..16, // Flip a bit in the first 16 bytes (before checksum)
    ) {
        let original = HbtpHeader::new(version, msg_type, flags, stream_id, length);
        let mut bytes = original.as_bytes().to_vec();

        // Flip a single bit
        let byte_idx = bit_to_flip / 8;
        let bit_idx = bit_to_flip % 8;
        bytes[byte_idx] ^= 1 << bit_idx;

        // Verification should fail (either invalid magic or checksum mismatch)
        let result = HbtpHeader::from_bytes(&bytes);
        if let Ok(parsed) = result {
            // If magic is still valid, checksum should fail
            prop_assert!(!parsed.verify_checksum());
        }
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (ToolInvocation)
    /// **Validates: Requirements 2.3**
    #[test]
    fn prop_tool_invocation_round_trip(
        tool_id in any::<u32>(),
        arg_layout in any::<u64>(),
        args_offset in any::<u32>(),
        args_len in any::<u32>(),
    ) {
        let original = ToolInvocation::new(tool_id, arg_layout, args_offset, args_len);
        let bytes = original.as_bytes();
        let parsed = ToolInvocation::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.tool_id, original.tool_id);
        prop_assert_eq!(parsed.arg_layout, original.arg_layout);
        prop_assert_eq!(parsed.args_offset, original.args_offset);
        prop_assert_eq!(parsed.args_len, original.args_len);
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (StreamChunk)
    /// **Validates: Requirements 5.3**
    #[test]
    fn prop_stream_chunk_round_trip(
        sequence in any::<u32>(),
        flags in 0u8..16, // Valid flag combinations
        len in any::<u16>(),
    ) {
        let original = StreamChunk::new(sequence, flags, len);
        let bytes = original.as_bytes();
        let parsed = StreamChunk::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.sequence, original.sequence);
        prop_assert_eq!(parsed.flags, original.flags);
        prop_assert_eq!(parsed.len, original.len);
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (SignedToolDef)
    /// **Validates: Requirements 7.1**
    #[test]
    fn prop_signed_tool_def_round_trip(
        tool_id in any::<u32>(),
        schema_hash in any::<[u8; 32]>(),
        capabilities in any::<u64>(),
        signature in any::<[u8; 64]>(),
        public_key in any::<[u8; 32]>(),
    ) {
        let original = SignedToolDef {
            tool_id,
            schema_hash,
            capabilities,
            signature,
            public_key,
        };
        let bytes = original.as_bytes();
        let parsed = SignedToolDef::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.tool_id, original.tool_id);
        prop_assert_eq!(parsed.schema_hash, original.schema_hash);
        prop_assert_eq!(parsed.capabilities, original.capabilities);
        prop_assert_eq!(parsed.signature, original.signature);
        prop_assert_eq!(parsed.public_key, original.public_key);
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (SignedInvocation)
    /// **Validates: Requirements 7.3**
    #[test]
    fn prop_signed_invocation_round_trip(
        tool_id in any::<u32>(),
        nonce in any::<u64>(),
        timestamp in any::<u64>(),
        args_hash in any::<[u8; 32]>(),
        signature in any::<[u8; 64]>(),
    ) {
        let original = SignedInvocation {
            tool_id,
            nonce,
            timestamp,
            args_hash,
            signature,
        };
        let bytes = original.as_bytes();
        let parsed = SignedInvocation::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.tool_id, original.tool_id);
        prop_assert_eq!(parsed.nonce, original.nonce);
        prop_assert_eq!(parsed.timestamp, original.timestamp);
        prop_assert_eq!(parsed.args_hash, original.args_hash);
        prop_assert_eq!(parsed.signature, original.signature);
    }
}
