//! Message parsing and validation for DCP protocol.

use crate::binary::{BinaryMessageEnvelope, Flags, MessageType};
use crate::DCPError;

/// Parsed DCP message with envelope and payload reference
#[derive(Debug, PartialEq)]
pub struct ParsedMessage<'a> {
    /// The message envelope
    pub envelope: &'a BinaryMessageEnvelope,
    /// The payload bytes
    pub payload: &'a [u8],
}

/// Message parser for DCP protocol
pub struct MessageParser;

impl MessageParser {
    /// Parse a complete DCP message from bytes
    pub fn parse(bytes: &[u8]) -> Result<ParsedMessage<'_>, DCPError> {
        let envelope = BinaryMessageEnvelope::from_bytes(bytes)?;
        let payload_start = BinaryMessageEnvelope::SIZE;
        let payload_end = payload_start + envelope.payload_len as usize;

        if bytes.len() < payload_end {
            return Err(DCPError::InsufficientData);
        }

        Ok(ParsedMessage {
            envelope,
            payload: &bytes[payload_start..payload_end],
        })
    }

    /// Validate message type is known
    pub fn validate_message_type(msg_type: u8) -> Result<MessageType, DCPError> {
        MessageType::from_u8(msg_type).ok_or(DCPError::UnknownMessageType)
    }

    /// Extract flags from envelope
    pub fn extract_flags(envelope: &BinaryMessageEnvelope) -> MessageFlags {
        MessageFlags {
            streaming: envelope.flags & Flags::STREAMING != 0,
            compressed: envelope.flags & Flags::COMPRESSED != 0,
            signed: envelope.flags & Flags::SIGNED != 0,
        }
    }
}

/// Extracted message flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageFlags {
    pub streaming: bool,
    pub compressed: bool,
    pub signed: bool,
}

impl MessageFlags {
    /// Convert flags back to u8
    pub fn to_u8(&self) -> u8 {
        let mut flags = 0u8;
        if self.streaming {
            flags |= Flags::STREAMING;
        }
        if self.compressed {
            flags |= Flags::COMPRESSED;
        }
        if self.signed {
            flags |= Flags::SIGNED;
        }
        flags
    }
}

/// Message dispatcher for routing by type
pub struct MessageDispatcher;

impl MessageDispatcher {
    /// Dispatch a parsed message to the appropriate handler
    pub fn dispatch<'a, H: MessageHandler>(
        message: &ParsedMessage<'a>,
        handler: &H,
    ) -> Result<(), DCPError> {
        let msg_type = MessageParser::validate_message_type(message.envelope.message_type)?;

        match msg_type {
            MessageType::Tool => handler.handle_tool(message),
            MessageType::Resource => handler.handle_resource(message),
            MessageType::Prompt => handler.handle_prompt(message),
            MessageType::Response => handler.handle_response(message),
            MessageType::Error => handler.handle_error(message),
            MessageType::Stream => handler.handle_stream(message),
        }
    }
}

/// Trait for handling different message types
pub trait MessageHandler {
    fn handle_tool(&self, message: &ParsedMessage<'_>) -> Result<(), DCPError>;
    fn handle_resource(&self, message: &ParsedMessage<'_>) -> Result<(), DCPError>;
    fn handle_prompt(&self, message: &ParsedMessage<'_>) -> Result<(), DCPError>;
    fn handle_response(&self, message: &ParsedMessage<'_>) -> Result<(), DCPError>;
    fn handle_error(&self, message: &ParsedMessage<'_>) -> Result<(), DCPError>;
    fn handle_stream(&self, message: &ParsedMessage<'_>) -> Result<(), DCPError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message() {
        let mut bytes = vec![0u8; 16];
        // Create envelope
        let envelope = BinaryMessageEnvelope::new(MessageType::Tool, 0, 8);
        bytes[..8].copy_from_slice(envelope.as_bytes());
        // Add payload
        bytes[8..16].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let parsed = MessageParser::parse(&bytes).unwrap();
        assert_eq!(parsed.envelope.message_type, MessageType::Tool as u8);
        assert_eq!(parsed.payload, &[1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_parse_insufficient_payload() {
        let mut bytes = vec![0u8; 12];
        let envelope = BinaryMessageEnvelope::new(MessageType::Tool, 0, 100);
        bytes[..8].copy_from_slice(envelope.as_bytes());

        assert_eq!(MessageParser::parse(&bytes), Err(DCPError::InsufficientData));
    }

    #[test]
    fn test_validate_message_type() {
        assert_eq!(MessageParser::validate_message_type(1), Ok(MessageType::Tool));
        assert_eq!(MessageParser::validate_message_type(6), Ok(MessageType::Stream));
        assert_eq!(MessageParser::validate_message_type(0), Err(DCPError::UnknownMessageType));
        assert_eq!(MessageParser::validate_message_type(7), Err(DCPError::UnknownMessageType));
    }

    #[test]
    fn test_extract_flags() {
        let envelope =
            BinaryMessageEnvelope::new(MessageType::Tool, Flags::STREAMING | Flags::SIGNED, 0);
        let flags = MessageParser::extract_flags(&envelope);

        assert!(flags.streaming);
        assert!(!flags.compressed);
        assert!(flags.signed);
    }

    #[test]
    fn test_flags_round_trip() {
        let flags = MessageFlags {
            streaming: true,
            compressed: false,
            signed: true,
        };
        let byte = flags.to_u8();
        assert_eq!(byte, Flags::STREAMING | Flags::SIGNED);
    }
}
