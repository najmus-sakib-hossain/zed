//! Property-based tests for protocol layer.
//!
//! Feature: dcp-protocol, Property 2: Message Parsing Correctness
//! Feature: dcp-protocol, Property 5: Schema Validation Correctness

use dcp::binary::{BinaryMessageEnvelope, Flags, MessageType};
use dcp::protocol::parser::{MessageFlags, MessageParser};
use dcp::protocol::schema::{FieldDef, InputSchema, SchemaValidator};
use dcp::DCPError;
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

    /// Feature: dcp-protocol, Property 2: Message Parsing Correctness
    /// For any valid BinaryMessageEnvelope with any combination of message type
    /// and flags, parsing the envelope SHALL correctly extract all fields
    /// and the magic number SHALL be 0xDC01.
    /// **Validates: Requirements 1.2, 1.3, 1.4**
    #[test]
    fn prop_message_parsing_correctness(
        msg_type in message_type_strategy(),
        flags in flags_strategy(),
        payload_len in 0u32..1024,
        payload in prop::collection::vec(any::<u8>(), 0..1024usize),
    ) {
        let msg_type_enum = MessageType::from_u8(msg_type).unwrap();
        let envelope = BinaryMessageEnvelope::new(msg_type_enum, flags, payload_len);

        // Build complete message
        let mut message = envelope.as_bytes().to_vec();
        message.extend(&payload[..payload_len.min(payload.len() as u32) as usize]);

        // Parse the message
        let parsed = MessageParser::parse(&message);

        if payload_len as usize <= payload.len() {
            let parsed = parsed.unwrap();

            // Verify magic number
            prop_assert_eq!(parsed.envelope.magic, 0xDC01);

            // Verify message type is correctly extracted
            prop_assert_eq!(parsed.envelope.message_type, msg_type);
            prop_assert!(MessageParser::validate_message_type(msg_type).is_ok());

            // Verify flags are correctly extracted
            let extracted_flags = MessageParser::extract_flags(parsed.envelope);
            prop_assert_eq!(extracted_flags.streaming, flags & Flags::STREAMING != 0);
            prop_assert_eq!(extracted_flags.compressed, flags & Flags::COMPRESSED != 0);
            prop_assert_eq!(extracted_flags.signed, flags & Flags::SIGNED != 0);

            // Verify payload length
            prop_assert_eq!(parsed.envelope.payload_len, payload_len);
        }
    }

    /// Feature: dcp-protocol, Property 2: Message Parsing Correctness (flags round-trip)
    /// For any combination of flags, converting to u8 and back SHALL preserve values.
    /// **Validates: Requirements 1.4**
    #[test]
    fn prop_flags_round_trip(
        streaming in any::<bool>(),
        compressed in any::<bool>(),
        signed in any::<bool>(),
    ) {
        let flags = MessageFlags {
            streaming,
            compressed,
            signed,
        };

        let byte = flags.to_u8();
        let envelope = BinaryMessageEnvelope::new(MessageType::Tool, byte, 0);
        let extracted = MessageParser::extract_flags(&envelope);

        prop_assert_eq!(extracted.streaming, streaming);
        prop_assert_eq!(extracted.compressed, compressed);
        prop_assert_eq!(extracted.signed, signed);
    }

    /// Feature: dcp-protocol, Property 5: Schema Validation Correctness (required fields)
    /// For any schema with required fields, validation SHALL reject inputs
    /// missing required fields.
    /// **Validates: Requirements 3.2**
    #[test]
    fn prop_schema_required_validation(
        required_mask in any::<u64>(),
        present_mask in any::<u64>(),
    ) {
        let mut schema = InputSchema::new();
        schema.required = required_mask;

        let result = SchemaValidator::validate_required(&schema, present_mask);

        // Validation should pass iff all required bits are present
        let missing = required_mask & !present_mask;
        if missing == 0 {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::ValidationFailed));
        }
    }

    /// Feature: dcp-protocol, Property 5: Schema Validation Correctness (enum range)
    /// For any schema with enum constraints, validation SHALL reject
    /// out-of-range enum values.
    /// **Validates: Requirements 3.3**
    #[test]
    fn prop_schema_enum_validation(
        min in 0u8..128,
        max_offset in 1u8..128,
        value in any::<u8>(),
    ) {
        let max = min.saturating_add(max_offset);
        let field = FieldDef::new_enum("test_enum", 0, 1, min, max);

        let result = SchemaValidator::validate_enum(&field, value);

        if value >= min && value <= max {
            prop_assert!(result.is_ok());
        } else {
            prop_assert_eq!(result, Err(DCPError::ValidationFailed));
        }
    }

    /// Feature: dcp-protocol, Property 5: Schema Validation Correctness (complete)
    /// For any schema, complete validation SHALL check both required fields
    /// and enum constraints.
    /// **Validates: Requirements 3.2, 3.3**
    #[test]
    fn prop_schema_complete_validation(
        required_bits in 0u8..4, // Which of first 4 fields are required
        present_bits in 0u8..16, // Which of first 4 fields are present
        enum_min in 1u8..10,
        enum_max_offset in 1u8..10,
        enum_value in any::<u8>(),
    ) {
        let enum_max = enum_min.saturating_add(enum_max_offset);

        let mut schema = InputSchema::new();
        schema.required = required_bits as u64;
        schema.add_field(FieldDef::new_enum("enum_field", 0, 1, enum_min, enum_max));

        let field_values = vec![(0, enum_value)];
        let result = SchemaValidator::validate_input(
            &schema,
            present_bits as u64,
            &field_values,
        );

        let missing_required = (required_bits as u64) & !(present_bits as u64);
        let enum_valid = enum_value >= enum_min && enum_value <= enum_max;

        if missing_required != 0 || !enum_valid {
            prop_assert_eq!(result, Err(DCPError::ValidationFailed));
        } else {
            prop_assert!(result.is_ok());
        }
    }
}
