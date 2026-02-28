//! DCP (Development Context Protocol) Integration
//!
//! This module provides integration with the DCP crate for high-performance
//! AI tool communication. DCP is our MCP competitor with 10-1000x performance
//! improvements while maintaining backward compatibility.
//!
//! ## Features
//!
//! - Binary Message Envelope (BME) protocol support
//! - Zero-Copy Tool Invocation (ZCTI)
//! - MCP compatibility via McpAdapter
//! - Capability manifest support
//! - Ed25519 signing for secure tool definitions
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::dcp::{DcpClient, DcpConfig};
//!
//! let config = DcpConfig::default();
//! let mut client = DcpClient::new(config);
//!
//! // Connect to a DCP server
//! client.connect("tcp://localhost:9000")?;
//!
//! // Invoke a tool
//! let result = client.invoke_tool(tool_id, &args)?;
//!
//! // Or use MCP fallback
//! let result = client.invoke_tool_mcp(&json_rpc_request)?;
//! ```

use crate::{DrivenError, Result};
use std::collections::HashMap;

// Re-export DCP types for convenience
pub use dcp::binary::{
    ArgType, BinaryMessageEnvelope, ChunkFlags, Flags, MessageType, SignedInvocation,
    SignedToolDef, StreamChunk, ToolInvocation,
};
pub use dcp::capability::CapabilityManifest;
pub use dcp::compat::adapter::AdapterError;
pub use dcp::compat::json_rpc::RequestId;
pub use dcp::compat::{JsonRpcError, JsonRpcParser, JsonRpcRequest, JsonRpcResponse, McpAdapter};
pub use dcp::security::{Signer, Verifier};

/// Zero-Copy Tool Invocation (ZCTI) builder
///
/// Provides a high-level API for building tool invocations with typed arguments.
/// Arguments are encoded in a compact binary format for zero-copy passing.
#[derive(Debug, Clone)]
pub struct ZctiBuilder {
    /// Tool ID
    tool_id: u32,
    /// Argument layout (types encoded in bitfield)
    arg_layout: u64,
    /// Argument data buffer
    args_data: Vec<u8>,
    /// Current argument index
    arg_index: usize,
}

impl ZctiBuilder {
    /// Create a new ZCTI builder for the given tool
    pub fn new(tool_id: u32) -> Self {
        Self {
            tool_id,
            arg_layout: 0,
            args_data: Vec::new(),
            arg_index: 0,
        }
    }

    /// Add a null argument
    pub fn add_null(mut self) -> Self {
        self.set_arg_type(ArgType::Null);
        self
    }

    /// Add a boolean argument
    pub fn add_bool(mut self, value: bool) -> Self {
        self.set_arg_type(ArgType::Bool);
        self.args_data.push(if value { 1 } else { 0 });
        self
    }

    /// Add an i32 argument
    pub fn add_i32(mut self, value: i32) -> Self {
        self.set_arg_type(ArgType::I32);
        self.args_data.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Add an i64 argument
    pub fn add_i64(mut self, value: i64) -> Self {
        self.set_arg_type(ArgType::I64);
        self.args_data.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Add an f64 argument
    pub fn add_f64(mut self, value: f64) -> Self {
        self.set_arg_type(ArgType::F64);
        self.args_data.extend_from_slice(&value.to_le_bytes());
        self
    }

    /// Add a string argument
    pub fn add_string(mut self, value: &str) -> Self {
        self.set_arg_type(ArgType::String);
        // Length-prefixed string (4 bytes length + data)
        let len = value.len() as u32;
        self.args_data.extend_from_slice(&len.to_le_bytes());
        self.args_data.extend_from_slice(value.as_bytes());
        self
    }

    /// Add a bytes argument
    pub fn add_bytes(mut self, value: &[u8]) -> Self {
        self.set_arg_type(ArgType::Bytes);
        // Length-prefixed bytes (4 bytes length + data)
        let len = value.len() as u32;
        self.args_data.extend_from_slice(&len.to_le_bytes());
        self.args_data.extend_from_slice(value);
        self
    }

    /// Add raw bytes without type encoding (for pre-encoded data)
    pub fn add_raw(mut self, data: &[u8]) -> Self {
        self.args_data.extend_from_slice(data);
        self
    }

    /// Set the argument type at the current index
    fn set_arg_type(&mut self, arg_type: ArgType) {
        if self.arg_index < ToolInvocation::MAX_ARGS {
            let shift = self.arg_index * 4;
            self.arg_layout |= (arg_type as u64) << shift;
            self.arg_index += 1;
        }
    }

    /// Build the tool invocation
    pub fn build(self) -> ToolInvocation {
        ToolInvocation::new(
            self.tool_id,
            self.arg_layout,
            0, // Offset is set when placed in shared memory
            self.args_data.len() as u32,
        )
    }

    /// Build and return both the invocation and the argument data
    pub fn build_with_data(self) -> (ToolInvocation, Vec<u8>) {
        let invocation =
            ToolInvocation::new(self.tool_id, self.arg_layout, 0, self.args_data.len() as u32);
        (invocation, self.args_data)
    }

    /// Get the current argument count
    pub fn arg_count(&self) -> usize {
        self.arg_index
    }

    /// Get the current data size
    pub fn data_size(&self) -> usize {
        self.args_data.len()
    }
}

/// Zero-Copy Tool Invocation (ZCTI) reader
///
/// Provides a high-level API for reading arguments from a tool invocation.
#[derive(Debug)]
pub struct ZctiReader<'a> {
    /// The tool invocation header
    invocation: &'a ToolInvocation,
    /// The argument data
    data: &'a [u8],
    /// Current read offset
    offset: usize,
    /// Current argument index
    arg_index: usize,
}

impl<'a> ZctiReader<'a> {
    /// Create a new ZCTI reader
    pub fn new(invocation: &'a ToolInvocation, data: &'a [u8]) -> Self {
        Self {
            invocation,
            data,
            offset: 0,
            arg_index: 0,
        }
    }

    /// Get the tool ID
    pub fn tool_id(&self) -> u32 {
        self.invocation.tool_id
    }

    /// Get the total argument count
    pub fn arg_count(&self) -> usize {
        self.invocation.arg_count()
    }

    /// Get the type of the next argument
    pub fn peek_type(&self) -> Option<ArgType> {
        self.invocation.get_arg_type(self.arg_index)
    }

    /// Read a boolean argument
    pub fn read_bool(&mut self) -> Result<bool> {
        self.check_type(ArgType::Bool)?;
        if self.offset >= self.data.len() {
            return Err(DrivenError::InvalidBinary("Insufficient data for bool".to_string()));
        }
        let value = self.data[self.offset] != 0;
        self.offset += 1;
        self.arg_index += 1;
        Ok(value)
    }

    /// Read an i32 argument
    pub fn read_i32(&mut self) -> Result<i32> {
        self.check_type(ArgType::I32)?;
        if self.offset + 4 > self.data.len() {
            return Err(DrivenError::InvalidBinary("Insufficient data for i32".to_string()));
        }
        let bytes: [u8; 4] = self.data[self.offset..self.offset + 4].try_into().unwrap();
        let value = i32::from_le_bytes(bytes);
        self.offset += 4;
        self.arg_index += 1;
        Ok(value)
    }

    /// Read an i64 argument
    pub fn read_i64(&mut self) -> Result<i64> {
        self.check_type(ArgType::I64)?;
        if self.offset + 8 > self.data.len() {
            return Err(DrivenError::InvalidBinary("Insufficient data for i64".to_string()));
        }
        let bytes: [u8; 8] = self.data[self.offset..self.offset + 8].try_into().unwrap();
        let value = i64::from_le_bytes(bytes);
        self.offset += 8;
        self.arg_index += 1;
        Ok(value)
    }

    /// Read an f64 argument
    pub fn read_f64(&mut self) -> Result<f64> {
        self.check_type(ArgType::F64)?;
        if self.offset + 8 > self.data.len() {
            return Err(DrivenError::InvalidBinary("Insufficient data for f64".to_string()));
        }
        let bytes: [u8; 8] = self.data[self.offset..self.offset + 8].try_into().unwrap();
        let value = f64::from_le_bytes(bytes);
        self.offset += 8;
        self.arg_index += 1;
        Ok(value)
    }

    /// Read a string argument
    pub fn read_string(&mut self) -> Result<&'a str> {
        self.check_type(ArgType::String)?;
        if self.offset + 4 > self.data.len() {
            return Err(DrivenError::InvalidBinary(
                "Insufficient data for string length".to_string(),
            ));
        }
        let len_bytes: [u8; 4] = self.data[self.offset..self.offset + 4].try_into().unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;
        self.offset += 4;

        if self.offset + len > self.data.len() {
            return Err(DrivenError::InvalidBinary(
                "Insufficient data for string content".to_string(),
            ));
        }
        let value = std::str::from_utf8(&self.data[self.offset..self.offset + len])
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8: {}", e)))?;
        self.offset += len;
        self.arg_index += 1;
        Ok(value)
    }

    /// Read a bytes argument
    pub fn read_bytes(&mut self) -> Result<&'a [u8]> {
        self.check_type(ArgType::Bytes)?;
        if self.offset + 4 > self.data.len() {
            return Err(DrivenError::InvalidBinary(
                "Insufficient data for bytes length".to_string(),
            ));
        }
        let len_bytes: [u8; 4] = self.data[self.offset..self.offset + 4].try_into().unwrap();
        let len = u32::from_le_bytes(len_bytes) as usize;
        self.offset += 4;

        if self.offset + len > self.data.len() {
            return Err(DrivenError::InvalidBinary(
                "Insufficient data for bytes content".to_string(),
            ));
        }
        let value = &self.data[self.offset..self.offset + len];
        self.offset += len;
        self.arg_index += 1;
        Ok(value)
    }

    /// Skip the current argument
    pub fn skip(&mut self) -> Result<()> {
        let arg_type = self
            .peek_type()
            .ok_or_else(|| DrivenError::InvalidBinary("No more arguments".to_string()))?;

        match arg_type {
            ArgType::Null => {
                self.arg_index += 1;
            }
            ArgType::Bool => {
                self.offset += 1;
                self.arg_index += 1;
            }
            ArgType::I32 => {
                self.offset += 4;
                self.arg_index += 1;
            }
            ArgType::I64 | ArgType::F64 => {
                self.offset += 8;
                self.arg_index += 1;
            }
            ArgType::String | ArgType::Bytes => {
                if self.offset + 4 > self.data.len() {
                    return Err(DrivenError::InvalidBinary(
                        "Insufficient data for length".to_string(),
                    ));
                }
                let len_bytes: [u8; 4] =
                    self.data[self.offset..self.offset + 4].try_into().unwrap();
                let len = u32::from_le_bytes(len_bytes) as usize;
                self.offset += 4 + len;
                self.arg_index += 1;
            }
            ArgType::Array | ArgType::Object => {
                // For complex types, we'd need additional metadata
                return Err(DrivenError::InvalidBinary(
                    "Complex types not yet supported".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Check if there are more arguments to read
    pub fn has_more(&self) -> bool {
        self.arg_index < self.arg_count()
    }

    /// Get the remaining data slice
    pub fn remaining_data(&self) -> &'a [u8] {
        &self.data[self.offset..]
    }

    /// Check that the next argument has the expected type
    fn check_type(&self, expected: ArgType) -> Result<()> {
        let actual = self
            .peek_type()
            .ok_or_else(|| DrivenError::InvalidBinary("No more arguments".to_string()))?;
        if actual != expected {
            return Err(DrivenError::InvalidBinary(format!(
                "Type mismatch: expected {:?}, got {:?}",
                expected, actual
            )));
        }
        Ok(())
    }
}

/// Shared memory buffer for zero-copy argument passing
///
/// This is a simple implementation that uses a Vec<u8> as the backing store.
/// In a real implementation, this would use SharedArrayBuffer or mmap.
#[derive(Debug)]
pub struct SharedArgBuffer {
    /// The backing buffer
    data: Vec<u8>,
    /// Current write offset
    write_offset: usize,
}

impl SharedArgBuffer {
    /// Create a new shared argument buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            write_offset: 0,
        }
    }

    /// Create with default capacity (64KB)
    pub fn with_default_capacity() -> Self {
        Self::new(65536)
    }

    /// Write data to the buffer and return the offset
    pub fn write(&mut self, data: &[u8]) -> Result<u32> {
        if self.write_offset + data.len() > self.data.len() {
            return Err(DrivenError::InvalidBinary("Buffer overflow".to_string()));
        }
        let offset = self.write_offset as u32;
        self.data[self.write_offset..self.write_offset + data.len()].copy_from_slice(data);
        self.write_offset += data.len();
        Ok(offset)
    }

    /// Read data from the buffer at the given offset
    pub fn read(&self, offset: u32, len: u32) -> Result<&[u8]> {
        let start = offset as usize;
        let end = start + len as usize;
        if end > self.data.len() {
            return Err(DrivenError::InvalidBinary("Read out of bounds".to_string()));
        }
        Ok(&self.data[start..end])
    }

    /// Get the current write offset
    pub fn offset(&self) -> u32 {
        self.write_offset as u32
    }

    /// Reset the buffer
    pub fn reset(&mut self) {
        self.write_offset = 0;
    }

    /// Get the underlying data
    pub fn data(&self) -> &[u8] {
        &self.data[..self.write_offset]
    }

    /// Get the capacity
    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    /// Get the remaining capacity
    pub fn remaining(&self) -> usize {
        self.data.len() - self.write_offset
    }
}

/// DCP client configuration
#[derive(Debug, Clone)]
pub struct DcpConfig {
    /// Whether DCP is enabled
    pub enabled: bool,
    /// Whether to prefer DCP over MCP when available
    pub prefer_dcp: bool,
    /// DCP server endpoint (e.g., "tcp://localhost:9000")
    pub endpoint: Option<String>,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
    /// Whether to enable signing
    pub signing_enabled: bool,
    /// Seed for Ed25519 signing (32 bytes)
    pub signing_seed: Option<[u8; 32]>,
}

impl Default for DcpConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_dcp: true,
            endpoint: None,
            timeout_ms: 5000,
            signing_enabled: false,
            signing_seed: None,
        }
    }
}

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected
    Disconnected,
    /// Connecting to server
    Connecting,
    /// Connected via DCP protocol
    ConnectedDcp,
    /// Connected via MCP fallback
    ConnectedMcp,
    /// Connection failed
    Failed,
}

/// Protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// DCP binary protocol
    Dcp,
    /// MCP JSON-RPC protocol
    Mcp,
}

/// Result of a tool invocation
#[derive(Debug)]
pub enum InvocationResult {
    /// DCP binary response
    Dcp(Vec<u8>),
    /// MCP JSON-RPC response
    Mcp(String),
}

impl InvocationResult {
    /// Check if this is a DCP result
    pub fn is_dcp(&self) -> bool {
        matches!(self, InvocationResult::Dcp(_))
    }

    /// Check if this is an MCP result
    pub fn is_mcp(&self) -> bool {
        matches!(self, InvocationResult::Mcp(_))
    }

    /// Get the DCP bytes if this is a DCP result
    pub fn as_dcp(&self) -> Option<&[u8]> {
        match self {
            InvocationResult::Dcp(bytes) => Some(bytes),
            _ => None,
        }
    }

    /// Get the MCP JSON string if this is an MCP result
    pub fn as_mcp(&self) -> Option<&str> {
        match self {
            InvocationResult::Mcp(json) => Some(json),
            _ => None,
        }
    }
}

/// DCP client for AI tool communication
///
/// Provides high-performance binary protocol communication with DCP servers,
/// with automatic fallback to MCP when DCP is unavailable.
pub struct DcpClient {
    /// Client configuration
    config: DcpConfig,
    /// Current connection state
    state: ConnectionState,
    /// MCP adapter for fallback
    mcp_adapter: Option<McpAdapter>,
    /// Capability manifest
    capabilities: CapabilityManifest,
    /// Ed25519 signer for secure tool definitions
    signer: Option<Signer>,
    /// Registered tools
    tools: HashMap<u32, ToolDefinition>,
    /// Next tool ID
    next_tool_id: u32,
}

/// Tool definition
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    /// Tool ID
    pub id: u32,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Schema hash (Blake3)
    pub schema_hash: [u8; 32],
    /// Required capabilities
    pub capabilities: u64,
}

/// Statistics about the current capability manifest
#[derive(Debug, Clone, Copy)]
pub struct CapabilityStats {
    /// Number of registered tools
    pub tool_count: u32,
    /// Number of registered resources
    pub resource_count: u32,
    /// Number of registered prompts
    pub prompt_count: u32,
    /// Number of enabled extensions
    pub extension_count: u32,
    /// Protocol version
    pub version: u16,
}

/// DCP message representing a complete binary message with envelope and payload
#[derive(Debug, Clone)]
pub struct DcpMessage {
    /// Message type
    pub message_type: MessageType,
    /// Flags (streaming, compressed, signed)
    pub flags: u8,
    /// Payload data
    pub payload: Vec<u8>,
}

impl DcpMessage {
    /// Create a new DCP message
    pub fn new(message_type: MessageType, flags: u8, payload: Vec<u8>) -> Self {
        Self {
            message_type,
            flags,
            payload,
        }
    }

    /// Create a tool invocation message
    pub fn tool(payload: Vec<u8>) -> Self {
        Self::new(MessageType::Tool, 0, payload)
    }

    /// Create a resource request message
    pub fn resource(payload: Vec<u8>) -> Self {
        Self::new(MessageType::Resource, 0, payload)
    }

    /// Create a prompt message
    pub fn prompt(payload: Vec<u8>) -> Self {
        Self::new(MessageType::Prompt, 0, payload)
    }

    /// Create a response message
    pub fn response(payload: Vec<u8>) -> Self {
        Self::new(MessageType::Response, 0, payload)
    }

    /// Create an error message
    pub fn error(payload: Vec<u8>) -> Self {
        Self::new(MessageType::Error, 0, payload)
    }

    /// Create a streaming message
    pub fn stream(payload: Vec<u8>) -> Self {
        Self::new(MessageType::Stream, Flags::STREAMING, payload)
    }

    /// Check if this is a streaming message
    pub fn is_streaming(&self) -> bool {
        self.flags & Flags::STREAMING != 0
    }

    /// Check if this message is compressed
    pub fn is_compressed(&self) -> bool {
        self.flags & Flags::COMPRESSED != 0
    }

    /// Check if this message is signed
    pub fn is_signed(&self) -> bool {
        self.flags & Flags::SIGNED != 0
    }

    /// Set the streaming flag
    pub fn with_streaming(mut self) -> Self {
        self.flags |= Flags::STREAMING;
        self
    }

    /// Set the compressed flag
    pub fn with_compressed(mut self) -> Self {
        self.flags |= Flags::COMPRESSED;
        self
    }

    /// Set the signed flag
    pub fn with_signed(mut self) -> Self {
        self.flags |= Flags::SIGNED;
        self
    }

    /// Encode the message to bytes (envelope + payload)
    pub fn encode(&self) -> Vec<u8> {
        let envelope =
            BinaryMessageEnvelope::new(self.message_type, self.flags, self.payload.len() as u32);

        let mut result = Vec::with_capacity(BinaryMessageEnvelope::SIZE + self.payload.len());
        result.extend_from_slice(envelope.as_bytes());
        result.extend_from_slice(&self.payload);
        result
    }

    /// Decode a message from bytes
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < BinaryMessageEnvelope::SIZE {
            return Err(DrivenError::InvalidBinary("Insufficient data for BME header".to_string()));
        }

        let envelope = BinaryMessageEnvelope::from_bytes(bytes)
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid BME: {:?}", e)))?;

        let message_type = envelope.get_message_type().ok_or_else(|| {
            DrivenError::InvalidBinary(format!("Unknown message type: {}", envelope.message_type))
        })?;

        let payload_len = envelope.payload_len as usize;
        let total_len = BinaryMessageEnvelope::SIZE + payload_len;

        if bytes.len() < total_len {
            return Err(DrivenError::InvalidBinary(format!(
                "Insufficient data: expected {} bytes, got {}",
                total_len,
                bytes.len()
            )));
        }

        let payload = bytes[BinaryMessageEnvelope::SIZE..total_len].to_vec();

        Ok(Self {
            message_type,
            flags: envelope.flags,
            payload,
        })
    }
}

/// Stream assembler for handling chunked streaming messages
#[derive(Debug)]
pub struct StreamAssembler {
    /// Accumulated chunks
    chunks: Vec<StreamChunk>,
    /// Accumulated payload data
    data: Vec<u8>,
    /// Expected next sequence number
    next_sequence: u32,
    /// Whether the stream is complete
    complete: bool,
    /// Whether an error occurred
    error: bool,
}

impl StreamAssembler {
    /// Create a new stream assembler
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            data: Vec::new(),
            next_sequence: 0,
            complete: false,
            error: false,
        }
    }

    /// Add a chunk to the stream
    pub fn add_chunk(&mut self, chunk_bytes: &[u8], payload: &[u8]) -> Result<()> {
        if self.complete {
            return Err(DrivenError::InvalidBinary("Stream already complete".to_string()));
        }

        let chunk = StreamChunk::from_bytes(chunk_bytes)
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid chunk: {:?}", e)))?;

        // Copy values from packed struct
        let sequence = chunk.sequence;
        let flags = chunk.flags;
        let len = chunk.len;

        // Validate sequence
        if sequence != self.next_sequence {
            return Err(DrivenError::InvalidBinary(format!(
                "Out of order chunk: expected {}, got {}",
                self.next_sequence, sequence
            )));
        }

        // Check for error
        if flags & ChunkFlags::ERROR != 0 {
            self.error = true;
            self.complete = true;
            return Err(DrivenError::InvalidBinary("Stream error".to_string()));
        }

        // Validate payload length
        if payload.len() != len as usize {
            return Err(DrivenError::InvalidBinary(format!(
                "Payload length mismatch: header says {}, got {}",
                len,
                payload.len()
            )));
        }

        // Accumulate data
        self.data.extend_from_slice(payload);
        self.chunks.push(*chunk);
        self.next_sequence += 1;

        // Check if complete
        if flags & ChunkFlags::LAST != 0 {
            self.complete = true;
        }

        Ok(())
    }

    /// Check if the stream is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Check if an error occurred
    pub fn has_error(&self) -> bool {
        self.error
    }

    /// Get the assembled data (only valid when complete)
    pub fn data(&self) -> Option<&[u8]> {
        if self.complete && !self.error {
            Some(&self.data)
        } else {
            None
        }
    }

    /// Take the assembled data (consumes the assembler)
    pub fn take_data(self) -> Option<Vec<u8>> {
        if self.complete && !self.error {
            Some(self.data)
        } else {
            None
        }
    }

    /// Get the number of chunks received
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

impl Default for StreamAssembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream builder for creating chunked streaming messages
#[derive(Debug)]
pub struct StreamBuilder {
    /// Chunk size
    chunk_size: usize,
    /// Current sequence number
    sequence: u32,
}

impl StreamBuilder {
    /// Create a new stream builder with the given chunk size
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            sequence: 0,
        }
    }

    /// Create a stream builder with default chunk size (64KB)
    pub fn with_default_chunk_size() -> Self {
        Self::new(65536)
    }

    /// Build chunks from data
    pub fn build_chunks(&mut self, data: &[u8]) -> Vec<(StreamChunk, Vec<u8>)> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let remaining = data.len() - offset;
            let chunk_len = remaining.min(self.chunk_size);
            let is_first = offset == 0;
            let is_last = offset + chunk_len >= data.len();

            let flags = if is_first && is_last {
                ChunkFlags::FIRST | ChunkFlags::LAST
            } else if is_first {
                ChunkFlags::FIRST
            } else if is_last {
                ChunkFlags::LAST
            } else {
                ChunkFlags::CONTINUE
            };

            let chunk = StreamChunk::new(self.sequence, flags, chunk_len as u16);
            let payload = data[offset..offset + chunk_len].to_vec();

            chunks.push((chunk, payload));
            self.sequence += 1;
            offset += chunk_len;
        }

        chunks
    }

    /// Reset the sequence counter
    pub fn reset(&mut self) {
        self.sequence = 0;
    }
}

impl DcpClient {
    /// Create a new DCP client with the given configuration
    pub fn new(config: DcpConfig) -> Self {
        let signer = if config.signing_enabled {
            config.signing_seed.map(|seed| Signer::from_seed(&seed))
        } else {
            None
        };

        Self {
            config,
            state: ConnectionState::Disconnected,
            mcp_adapter: None,
            capabilities: CapabilityManifest::new(1),
            signer,
            tools: HashMap::new(),
            next_tool_id: 1,
        }
    }

    /// Create a new DCP client with default configuration
    pub fn with_defaults() -> Self {
        Self::new(DcpConfig::default())
    }

    /// Get the current connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Check if connected (either DCP or MCP)
    pub fn is_connected(&self) -> bool {
        matches!(self.state, ConnectionState::ConnectedDcp | ConnectionState::ConnectedMcp)
    }

    /// Check if connected via DCP
    pub fn is_dcp_connected(&self) -> bool {
        self.state == ConnectionState::ConnectedDcp
    }

    /// Check if connected via MCP
    pub fn is_mcp_connected(&self) -> bool {
        self.state == ConnectionState::ConnectedMcp
    }

    /// Check if DCP should be preferred over MCP
    pub fn prefer_dcp(&self) -> bool {
        self.config.prefer_dcp && self.config.enabled
    }

    /// Set DCP preference
    pub fn set_prefer_dcp(&mut self, prefer: bool) {
        self.config.prefer_dcp = prefer;
    }

    /// Enable DCP protocol
    pub fn enable_dcp(&mut self) {
        self.config.enabled = true;
    }

    /// Disable DCP protocol (will use MCP only)
    pub fn disable_dcp(&mut self) {
        self.config.enabled = false;
    }

    /// Check if DCP is enabled
    pub fn is_dcp_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the current protocol in use
    pub fn current_protocol(&self) -> Option<Protocol> {
        match self.state {
            ConnectionState::ConnectedDcp => Some(Protocol::Dcp),
            ConnectionState::ConnectedMcp => Some(Protocol::Mcp),
            _ => None,
        }
    }

    /// Select the best protocol for a given operation
    ///
    /// Returns DCP if connected via DCP and DCP is preferred,
    /// otherwise returns MCP if available.
    pub fn select_protocol(&self) -> Option<Protocol> {
        if self.is_dcp_connected() && self.prefer_dcp() {
            Some(Protocol::Dcp)
        } else if self.is_mcp_connected() || self.has_mcp_fallback() {
            Some(Protocol::Mcp)
        } else if self.is_dcp_connected() {
            Some(Protocol::Dcp)
        } else {
            None
        }
    }

    /// Invoke a tool using the best available protocol
    ///
    /// Automatically selects DCP or MCP based on connection state and preferences.
    pub fn invoke_tool_auto(&self, tool_id: u32, args: &[u8]) -> Result<InvocationResult> {
        match self.select_protocol() {
            Some(Protocol::Dcp) => {
                let encoded = self.invoke_tool(tool_id, args)?;
                Ok(InvocationResult::Dcp(encoded))
            }
            Some(Protocol::Mcp) => {
                // Convert to MCP format and invoke
                let tool = self
                    .tools
                    .get(&tool_id)
                    .ok_or_else(|| DrivenError::Config(format!("Tool {} not found", tool_id)))?;

                let request = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": tool.name,
                        "arguments": serde_json::from_slice::<serde_json::Value>(args)
                            .unwrap_or(serde_json::Value::Null)
                    },
                    "id": 1
                });

                let response = self.handle_mcp_request(&request.to_string())?;
                Ok(InvocationResult::Mcp(response))
            }
            None => Err(DrivenError::Config("No protocol available".to_string())),
        }
    }

    /// Get the capability manifest
    pub fn capabilities(&self) -> &CapabilityManifest {
        &self.capabilities
    }

    /// Get a mutable reference to the capability manifest
    pub fn capabilities_mut(&mut self) -> &mut CapabilityManifest {
        &mut self.capabilities
    }

    /// Connect to a DCP server
    ///
    /// If DCP connection fails and MCP fallback is available, will fall back to MCP.
    pub fn connect(&mut self, endpoint: &str) -> Result<()> {
        self.state = ConnectionState::Connecting;

        // Try DCP connection first if enabled
        if self.config.enabled {
            match self.try_dcp_connect(endpoint) {
                Ok(()) => {
                    self.state = ConnectionState::ConnectedDcp;
                    return Ok(());
                }
                Err(e) => {
                    // Log the error but continue to MCP fallback
                    tracing::warn!("DCP connection failed, falling back to MCP: {}", e);
                }
            }
        }

        // Fall back to MCP
        match self.try_mcp_connect(endpoint) {
            Ok(()) => {
                self.state = ConnectionState::ConnectedMcp;
                Ok(())
            }
            Err(e) => {
                self.state = ConnectionState::Failed;
                Err(e)
            }
        }
    }

    /// Try to connect via DCP protocol
    fn try_dcp_connect(&mut self, _endpoint: &str) -> Result<()> {
        // TODO: Implement actual DCP connection
        // For now, this is a placeholder that will be implemented
        // when we have the full DCP server infrastructure
        Err(DrivenError::Config("DCP connection not yet implemented".to_string()))
    }

    /// Try to connect via MCP protocol
    fn try_mcp_connect(&mut self, _endpoint: &str) -> Result<()> {
        // Create MCP adapter for fallback
        self.mcp_adapter = Some(McpAdapter::new());
        Ok(())
    }

    /// Disconnect from the server
    pub fn disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.mcp_adapter = None;
    }

    /// Register a tool
    pub fn register_tool(
        &mut self,
        name: &str,
        description: &str,
        schema_hash: [u8; 32],
        capabilities: u64,
    ) -> u32 {
        let id = self.next_tool_id;
        self.next_tool_id += 1;

        let tool = ToolDefinition {
            id,
            name: name.to_string(),
            description: description.to_string(),
            schema_hash,
            capabilities,
        };

        self.tools.insert(id, tool);

        // Update capability manifest
        if id < CapabilityManifest::MAX_TOOLS as u32 {
            self.capabilities.set_tool(id as u16);
        }

        id
    }

    /// Get a registered tool by ID
    pub fn get_tool(&self, id: u32) -> Option<&ToolDefinition> {
        self.tools.get(&id)
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Invoke a tool via DCP binary protocol
    ///
    /// Uses Zero-Copy Tool Invocation (ZCTI) for maximum performance.
    pub fn invoke_tool(&self, tool_id: u32, args: &[u8]) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(DrivenError::Config("Not connected".to_string()));
        }

        // Check if tool exists
        if !self.tools.contains_key(&tool_id) {
            return Err(DrivenError::Config(format!("Tool {} not found", tool_id)));
        }

        // Create tool invocation
        let invocation = ToolInvocation::new(tool_id, 0, 0, args.len() as u32);

        // Build payload: invocation header + args
        let mut payload = Vec::with_capacity(ToolInvocation::SIZE + args.len());
        payload.extend_from_slice(invocation.as_bytes());
        payload.extend_from_slice(args);

        // Create BME message
        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        let message = DcpMessage::new(MessageType::Tool, flags, payload);

        // Encode the message
        let encoded = message.encode();

        // TODO: Actually send the message when DCP transport is implemented
        // For now, return the encoded message as a placeholder
        Ok(encoded)
    }

    /// Encode a DCP message with BME envelope
    pub fn encode_message(&self, message_type: MessageType, payload: &[u8]) -> Vec<u8> {
        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        let message = DcpMessage::new(message_type, flags, payload.to_vec());
        message.encode()
    }

    /// Decode a DCP message from bytes
    pub fn decode_message(&self, bytes: &[u8]) -> Result<DcpMessage> {
        DcpMessage::decode(bytes)
    }

    /// Create a tool invocation message
    pub fn create_tool_message(&self, tool_id: u32, args: &[u8]) -> Result<DcpMessage> {
        if !self.tools.contains_key(&tool_id) {
            return Err(DrivenError::Config(format!("Tool {} not found", tool_id)));
        }

        let invocation = ToolInvocation::new(tool_id, 0, 0, args.len() as u32);

        let mut payload = Vec::with_capacity(ToolInvocation::SIZE + args.len());
        payload.extend_from_slice(invocation.as_bytes());
        payload.extend_from_slice(args);

        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        Ok(DcpMessage::new(MessageType::Tool, flags, payload))
    }

    /// Create a resource request message
    pub fn create_resource_message(&self, resource_uri: &str) -> DcpMessage {
        let payload = resource_uri.as_bytes().to_vec();
        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        DcpMessage::new(MessageType::Resource, flags, payload)
    }

    /// Create a prompt message
    pub fn create_prompt_message(&self, prompt: &str) -> DcpMessage {
        let payload = prompt.as_bytes().to_vec();
        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        DcpMessage::new(MessageType::Prompt, flags, payload)
    }

    /// Create a response message
    pub fn create_response_message(&self, response: &[u8]) -> DcpMessage {
        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        DcpMessage::new(MessageType::Response, flags, response.to_vec())
    }

    /// Create an error message
    pub fn create_error_message(&self, error_code: u32, error_msg: &str) -> DcpMessage {
        let mut payload = Vec::with_capacity(4 + error_msg.len());
        payload.extend_from_slice(&error_code.to_le_bytes());
        payload.extend_from_slice(error_msg.as_bytes());
        DcpMessage::new(MessageType::Error, 0, payload)
    }

    /// Create a streaming message builder
    pub fn create_stream_builder(&self, chunk_size: usize) -> StreamBuilder {
        StreamBuilder::new(chunk_size)
    }

    /// Create a stream assembler for receiving chunked data
    pub fn create_stream_assembler(&self) -> StreamAssembler {
        StreamAssembler::new()
    }

    /// Create a ZCTI builder for the given tool
    pub fn create_zcti_builder(&self, tool_id: u32) -> Result<ZctiBuilder> {
        if !self.tools.contains_key(&tool_id) {
            return Err(DrivenError::Config(format!("Tool {} not found", tool_id)));
        }
        Ok(ZctiBuilder::new(tool_id))
    }

    /// Invoke a tool using ZCTI (Zero-Copy Tool Invocation)
    pub fn invoke_tool_zcti(&self, builder: ZctiBuilder) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(DrivenError::Config("Not connected".to_string()));
        }

        let tool_id = builder.tool_id;
        if !self.tools.contains_key(&tool_id) {
            return Err(DrivenError::Config(format!("Tool {} not found", tool_id)));
        }

        let (invocation, args_data) = builder.build_with_data();

        // Build payload: invocation header + args
        let mut payload = Vec::with_capacity(ToolInvocation::SIZE + args_data.len());
        payload.extend_from_slice(invocation.as_bytes());
        payload.extend_from_slice(&args_data);

        // Create BME message
        let flags = if self.signer.is_some() {
            Flags::SIGNED
        } else {
            0
        };
        let message = DcpMessage::new(MessageType::Tool, flags, payload);

        // Encode and return
        Ok(message.encode())
    }

    /// Parse a tool invocation from a received message
    pub fn parse_tool_invocation<'a>(&self, message: &'a DcpMessage) -> Result<ZctiReader<'a>> {
        if message.message_type != MessageType::Tool {
            return Err(DrivenError::InvalidBinary("Message is not a tool invocation".to_string()));
        }

        if message.payload.len() < ToolInvocation::SIZE {
            return Err(DrivenError::InvalidBinary(
                "Payload too small for tool invocation".to_string(),
            ));
        }

        let invocation = ToolInvocation::from_bytes(&message.payload)
            .map_err(|e| DrivenError::InvalidBinary(format!("Invalid invocation: {:?}", e)))?;

        let args_data = &message.payload[ToolInvocation::SIZE..];
        Ok(ZctiReader::new(invocation, args_data))
    }

    /// Create a shared argument buffer
    pub fn create_shared_buffer(&self, capacity: usize) -> SharedArgBuffer {
        SharedArgBuffer::new(capacity)
    }

    /// Get the MCP adapter for JSON-RPC compatibility
    pub fn mcp_adapter(&self) -> Option<&McpAdapter> {
        self.mcp_adapter.as_ref()
    }

    /// Get a mutable reference to the MCP adapter
    pub fn mcp_adapter_mut(&mut self) -> Option<&mut McpAdapter> {
        self.mcp_adapter.as_mut()
    }

    /// Register a tool with the MCP adapter for JSON-RPC compatibility
    pub fn register_tool_mcp(&mut self, name: &str, tool_id: u32) {
        if let Some(ref mut adapter) = self.mcp_adapter {
            adapter.register_tool(name, tool_id as u16);
        }
    }

    /// Handle an MCP JSON-RPC request and return a JSON-RPC response
    ///
    /// This provides seamless fallback to MCP when DCP is unavailable.
    pub fn handle_mcp_request(&self, json: &str) -> Result<String> {
        let adapter = self
            .mcp_adapter
            .as_ref()
            .ok_or_else(|| DrivenError::Config("MCP adapter not available".to_string()))?;

        // Parse the request
        let request = adapter
            .parse_request(json)
            .map_err(|e| DrivenError::Parse(format!("Invalid JSON-RPC request: {:?}", e)))?;

        // Handle based on method
        match request.method.as_str() {
            "initialize" => adapter
                .handle_initialize(&request)
                .map_err(|e| DrivenError::Format(format!("Failed to handle initialize: {:?}", e))),
            "tools/list" => adapter
                .handle_tools_list(&request)
                .map_err(|e| DrivenError::Format(format!("Failed to handle tools/list: {:?}", e))),
            "tools/call" => {
                // For tools/call, we need to handle it ourselves since we don't have a router
                self.handle_mcp_tools_call(&request)
            }
            "resources/list" => {
                // Return empty resources list
                let result = serde_json::json!({ "resources": [] });
                adapter
                    .format_success_response(request.id, result)
                    .map_err(|e| DrivenError::Format(format!("Failed to format response: {:?}", e)))
            }
            "prompts/list" => {
                // Return empty prompts list
                let result = serde_json::json!({ "prompts": [] });
                adapter
                    .format_success_response(request.id, result)
                    .map_err(|e| DrivenError::Format(format!("Failed to format response: {:?}", e)))
            }
            _ => {
                // Unknown method
                adapter
                    .format_error_response(request.id, JsonRpcError::method_not_found())
                    .map_err(|e| DrivenError::Format(format!("Failed to format error: {:?}", e)))
            }
        }
    }

    /// Handle an MCP tools/call request
    fn handle_mcp_tools_call(&self, request: &JsonRpcRequest) -> Result<String> {
        let adapter = self
            .mcp_adapter
            .as_ref()
            .ok_or_else(|| DrivenError::Config("MCP adapter not available".to_string()))?;

        // Extract tool name and arguments from params
        let params = request
            .params
            .as_ref()
            .ok_or_else(|| DrivenError::Parse("Missing params".to_string()))?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DrivenError::Parse("Missing tool name".to_string()))?;

        let arguments = params.get("arguments").cloned();

        // Resolve tool name to ID
        let tool_id = adapter
            .resolve_tool_name(tool_name)
            .ok_or_else(|| DrivenError::Config(format!("Unknown tool: {}", tool_name)))?;

        // Check if tool exists in our registry
        if !self.tools.contains_key(&(tool_id as u32)) {
            return adapter
                .format_error_response(
                    request.id.clone(),
                    JsonRpcError::new(-32602, format!("Tool not found: {}", tool_name)),
                )
                .map_err(|e| DrivenError::Format(format!("Failed to format error: {:?}", e)));
        }

        // Translate params to bytes
        let args_bytes = adapter.translate_params(&arguments);

        // For now, return a placeholder response since we don't have actual tool execution
        // In a full implementation, this would execute the tool and return the result
        let response_result = serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("Tool {} called with {} bytes of arguments", tool_name, args_bytes.len())
            }]
        });

        adapter
            .format_success_response(request.id.clone(), response_result)
            .map_err(|e| DrivenError::Format(format!("Failed to format response: {:?}", e)))
    }

    /// Check if MCP fallback is available
    pub fn has_mcp_fallback(&self) -> bool {
        self.mcp_adapter.is_some()
    }

    /// Convert a DCP message to MCP JSON-RPC format
    pub fn dcp_to_mcp(&self, message: &DcpMessage) -> Result<String> {
        match message.message_type {
            MessageType::Response => {
                // Convert response payload to JSON
                let result: serde_json::Value = serde_json::from_slice(&message.payload)
                    .unwrap_or_else(|_| {
                        serde_json::Value::String(
                            String::from_utf8_lossy(&message.payload).to_string(),
                        )
                    });

                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "result": result,
                    "id": null
                });

                serde_json::to_string(&response)
                    .map_err(|e| DrivenError::Format(format!("Failed to serialize: {}", e)))
            }
            MessageType::Error => {
                // Parse error code and message from payload
                let (code, msg) = if message.payload.len() >= 4 {
                    let code_bytes: [u8; 4] = message.payload[0..4].try_into().unwrap();
                    let code = i32::from_le_bytes(code_bytes);
                    let msg = String::from_utf8_lossy(&message.payload[4..]).to_string();
                    (code, msg)
                } else {
                    (-32000, "Unknown error".to_string())
                };

                let response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": code,
                        "message": msg
                    },
                    "id": null
                });

                serde_json::to_string(&response)
                    .map_err(|e| DrivenError::Format(format!("Failed to serialize: {}", e)))
            }
            _ => Err(DrivenError::Format(format!(
                "Cannot convert {:?} message to MCP format",
                message.message_type
            ))),
        }
    }

    /// Convert an MCP JSON-RPC request to DCP message
    pub fn mcp_to_dcp(&self, json: &str) -> Result<DcpMessage> {
        let adapter = self
            .mcp_adapter
            .as_ref()
            .ok_or_else(|| DrivenError::Config("MCP adapter not available".to_string()))?;

        let request = adapter
            .parse_request(json)
            .map_err(|e| DrivenError::Parse(format!("Invalid JSON-RPC: {:?}", e)))?;

        // Determine message type based on method
        let message_type = match request.method.as_str() {
            "tools/call" => MessageType::Tool,
            "resources/read" => MessageType::Resource,
            "prompts/get" => MessageType::Prompt,
            _ => MessageType::Tool, // Default to tool
        };

        // Serialize the request as payload
        let payload = serde_json::to_vec(&request)
            .map_err(|e| DrivenError::Format(format!("Failed to serialize: {}", e)))?;

        Ok(DcpMessage::new(message_type, 0, payload))
    }

    /// Invoke a tool via MCP JSON-RPC protocol
    ///
    /// Used as fallback when DCP is unavailable.
    pub fn invoke_tool_mcp(&self, request: &str) -> Result<String> {
        if !self.is_connected() {
            return Err(DrivenError::Config("Not connected".to_string()));
        }

        // Parse the JSON-RPC request
        let _parsed = JsonRpcParser::parse_request(request)
            .map_err(|e| DrivenError::Parse(format!("Invalid JSON-RPC request: {:?}", e)))?;

        // Use MCP adapter if available
        if let Some(ref adapter) = self.mcp_adapter {
            // For now, return a method not found error since we don't have a router
            // In a full implementation, this would route to the appropriate handler
            let response = adapter
                .format_error_response(RequestId::Null, JsonRpcError::method_not_found())
                .map_err(|e| DrivenError::Format(format!("Failed to format response: {:?}", e)))?;
            Ok(response)
        } else {
            Err(DrivenError::Config("MCP adapter not available".to_string()))
        }
    }

    /// Create a signed tool definition
    pub fn sign_tool_def(&self, tool_id: u32) -> Result<dcp::binary::SignedToolDef> {
        let signer = self
            .signer
            .as_ref()
            .ok_or_else(|| DrivenError::Security("Signing not enabled".to_string()))?;

        let tool = self
            .tools
            .get(&tool_id)
            .ok_or_else(|| DrivenError::Config(format!("Tool {} not found", tool_id)))?;

        Ok(signer.sign_tool_def(tool_id, tool.schema_hash, tool.capabilities))
    }

    /// Verify a signed tool definition
    pub fn verify_tool_def(&self, def: &dcp::binary::SignedToolDef) -> Result<()> {
        Verifier::verify_tool_def(def)
            .map_err(|e| DrivenError::Security(format!("Signature verification failed: {:?}", e)))
    }

    /// Sign a tool invocation
    ///
    /// Creates a signed invocation that can be verified by the server.
    /// Requires signing to be enabled.
    pub fn sign_invocation(
        &self,
        tool_id: u32,
        nonce: u64,
        timestamp: u64,
        args: &[u8],
    ) -> Result<SignedInvocation> {
        let signer = self
            .signer
            .as_ref()
            .ok_or_else(|| DrivenError::Security("Signing not enabled".to_string()))?;

        if !self.tools.contains_key(&tool_id) {
            return Err(DrivenError::Config(format!("Tool {} not found", tool_id)));
        }

        Ok(signer.sign_invocation(tool_id, nonce, timestamp, args))
    }

    /// Verify a signed invocation
    ///
    /// Verifies that the invocation was signed by the holder of the given public key.
    pub fn verify_invocation(&self, inv: &SignedInvocation, public_key: &[u8; 32]) -> Result<()> {
        Verifier::verify_invocation(inv, public_key)
            .map_err(|e| DrivenError::Security(format!("Invocation verification failed: {:?}", e)))
    }

    /// Verify that the args hash in a signed invocation matches the provided arguments
    pub fn verify_args_hash(&self, inv: &SignedInvocation, args: &[u8]) -> bool {
        Verifier::verify_args_hash(inv, args)
    }

    /// Generate a random signer (for testing or ephemeral keys)
    pub fn generate_signer() -> Signer {
        Signer::generate()
    }

    // ==================== Capability Manifest Support ====================

    /// Parse a capability manifest from bytes
    ///
    /// This is used when receiving a capability manifest from a remote server
    /// during capability negotiation.
    pub fn parse_capability_manifest(bytes: &[u8]) -> Result<&CapabilityManifest> {
        CapabilityManifest::from_bytes(bytes).map_err(|e| {
            DrivenError::InvalidBinary(format!("Failed to parse capability manifest: {:?}", e))
        })
    }

    /// Serialize the current capability manifest to bytes
    pub fn serialize_capabilities(&self) -> Vec<u8> {
        self.capabilities.as_bytes().to_vec()
    }

    /// Register a resource in the capability manifest
    pub fn register_resource(&mut self, resource_id: u16) {
        self.capabilities.set_resource(resource_id);
    }

    /// Unregister a resource from the capability manifest
    pub fn unregister_resource(&mut self, resource_id: u16) {
        self.capabilities.clear_resource(resource_id);
    }

    /// Check if a resource is registered
    pub fn has_resource(&self, resource_id: u16) -> bool {
        self.capabilities.has_resource(resource_id)
    }

    /// Register a prompt in the capability manifest
    pub fn register_prompt(&mut self, prompt_id: u16) {
        self.capabilities.set_prompt(prompt_id);
    }

    /// Unregister a prompt from the capability manifest
    pub fn unregister_prompt(&mut self, prompt_id: u16) {
        self.capabilities.clear_prompt(prompt_id);
    }

    /// Check if a prompt is registered
    pub fn has_prompt(&self, prompt_id: u16) -> bool {
        self.capabilities.has_prompt(prompt_id)
    }

    /// Set an extension flag in the capability manifest
    pub fn set_extension(&mut self, bit: u8) {
        self.capabilities.set_extension(bit);
    }

    /// Clear an extension flag from the capability manifest
    pub fn clear_extension(&mut self, bit: u8) {
        self.capabilities.clear_extension(bit);
    }

    /// Check if an extension is enabled
    pub fn has_extension(&self, bit: u8) -> bool {
        self.capabilities.has_extension(bit)
    }

    /// Enforce that a tool is available before invocation
    ///
    /// Returns an error if the tool is not registered in the capability manifest.
    pub fn enforce_tool(&self, tool_id: u32) -> Result<()> {
        if tool_id >= CapabilityManifest::MAX_TOOLS as u32 {
            return Err(DrivenError::Config(format!(
                "Tool ID {} exceeds maximum ({})",
                tool_id,
                CapabilityManifest::MAX_TOOLS
            )));
        }
        if !self.capabilities.has_tool(tool_id as u16) {
            return Err(DrivenError::Config(format!(
                "Tool {} is not available in capability manifest",
                tool_id
            )));
        }
        Ok(())
    }

    /// Enforce that a resource is available before access
    ///
    /// Returns an error if the resource is not registered in the capability manifest.
    pub fn enforce_resource(&self, resource_id: u16) -> Result<()> {
        if !self.capabilities.has_resource(resource_id) {
            return Err(DrivenError::Config(format!(
                "Resource {} is not available in capability manifest",
                resource_id
            )));
        }
        Ok(())
    }

    /// Enforce that a prompt is available before use
    ///
    /// Returns an error if the prompt is not registered in the capability manifest.
    pub fn enforce_prompt(&self, prompt_id: u16) -> Result<()> {
        if !self.capabilities.has_prompt(prompt_id) {
            return Err(DrivenError::Config(format!(
                "Prompt {} is not available in capability manifest",
                prompt_id
            )));
        }
        Ok(())
    }

    /// Negotiate capabilities with a remote server
    ///
    /// Takes the server's capability manifest and computes the intersection,
    /// updating the local capabilities to only include mutually supported features.
    pub fn negotiate_capabilities(
        &mut self,
        server_manifest: &CapabilityManifest,
    ) -> CapabilityManifest {
        let negotiated = self.capabilities.intersect(server_manifest);
        self.capabilities = negotiated.clone();
        negotiated
    }

    /// Get statistics about the current capabilities
    pub fn capability_stats(&self) -> CapabilityStats {
        CapabilityStats {
            tool_count: self.capabilities.tool_count(),
            resource_count: self.capabilities.resource_count(),
            prompt_count: self.capabilities.prompt_count(),
            extension_count: self.capabilities.extension_count(),
            version: self.capabilities.version,
        }
    }

    /// Get an iterator over all registered tool IDs
    pub fn tool_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.capabilities.tool_ids()
    }

    /// Get an iterator over all registered resource IDs
    pub fn resource_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.capabilities.resource_ids()
    }

    /// Get an iterator over all registered prompt IDs
    pub fn prompt_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.capabilities.prompt_ids()
    }

    /// Compute capability intersection with another manifest
    pub fn intersect_capabilities(&self, other: &CapabilityManifest) -> CapabilityManifest {
        self.capabilities.intersect(other)
    }

    /// Enable signing with a seed
    pub fn enable_signing(&mut self, seed: [u8; 32]) {
        self.signer = Some(Signer::from_seed(&seed));
        self.config.signing_enabled = true;
        self.config.signing_seed = Some(seed);
    }

    /// Disable signing
    pub fn disable_signing(&mut self) {
        self.signer = None;
        self.config.signing_enabled = false;
        self.config.signing_seed = None;
    }

    /// Get the public key if signing is enabled
    pub fn public_key(&self) -> Option<[u8; 32]> {
        self.signer.as_ref().map(|s| s.public_key_bytes())
    }
}

/// Create a Binary Message Envelope
pub fn create_envelope(
    message_type: MessageType,
    flags: u8,
    payload_len: u32,
) -> BinaryMessageEnvelope {
    BinaryMessageEnvelope::new(message_type, flags, payload_len)
}

/// Parse a Binary Message Envelope from bytes
pub fn parse_envelope(bytes: &[u8]) -> Result<&BinaryMessageEnvelope> {
    BinaryMessageEnvelope::from_bytes(bytes)
        .map_err(|e| DrivenError::InvalidBinary(format!("Failed to parse BME: {:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_dcp_client_creation() {
        let client = DcpClient::with_defaults();
        assert_eq!(client.state(), ConnectionState::Disconnected);
        assert!(!client.is_connected());
        assert!(client.prefer_dcp());
    }

    #[test]
    fn test_dcp_client_config() {
        let config = DcpConfig {
            enabled: false,
            prefer_dcp: false,
            endpoint: Some("tcp://localhost:9000".to_string()),
            timeout_ms: 10000,
            signing_enabled: true,
            signing_seed: Some([42u8; 32]),
        };

        let client = DcpClient::new(config);
        assert!(!client.prefer_dcp());
        assert!(client.signer.is_some());
    }

    #[test]
    fn test_tool_registration() {
        let mut client = DcpClient::with_defaults();

        let tool_id = client.register_tool("test_tool", "A test tool", [0xAB; 32], 0x1234);

        assert_eq!(tool_id, 1);
        assert!(client.get_tool(tool_id).is_some());
        assert!(client.capabilities().has_tool(tool_id as u16));

        let tool = client.get_tool(tool_id).unwrap();
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description, "A test tool");
    }

    #[test]
    fn test_signing() {
        let mut client = DcpClient::with_defaults();
        client.enable_signing([42u8; 32]);

        let tool_id = client.register_tool("signed_tool", "A signed tool", [0xCD; 32], 0x5678);

        let signed_def = client.sign_tool_def(tool_id).unwrap();
        assert!(client.verify_tool_def(&signed_def).is_ok());
    }

    #[test]
    fn test_sign_invocation() {
        let mut client = DcpClient::with_defaults();
        client.enable_signing([42u8; 32]);

        let tool_id = client.register_tool("test_tool", "Test", [0; 32], 0);
        let args = b"test arguments";

        let signed_inv = client.sign_invocation(tool_id, 12345, 1234567890, args).unwrap();

        let public_key = client.public_key().unwrap();
        assert!(client.verify_invocation(&signed_inv, &public_key).is_ok());
        assert!(client.verify_args_hash(&signed_inv, args));
        assert!(!client.verify_args_hash(&signed_inv, b"wrong args"));
    }

    #[test]
    fn test_sign_invocation_without_signing_enabled() {
        let mut client = DcpClient::with_defaults();
        let tool_id = client.register_tool("test_tool", "Test", [0; 32], 0);

        // Should fail because signing is not enabled
        let result = client.sign_invocation(tool_id, 12345, 1234567890, b"args");
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_invocation_unknown_tool() {
        let mut client = DcpClient::with_defaults();
        client.enable_signing([42u8; 32]);

        // Should fail because tool doesn't exist
        let result = client.sign_invocation(999, 12345, 1234567890, b"args");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_signer() {
        let signer1 = DcpClient::generate_signer();
        let signer2 = DcpClient::generate_signer();

        // Different signers should have different public keys
        assert_ne!(signer1.public_key_bytes(), signer2.public_key_bytes());
    }

    #[test]
    fn test_capability_intersection() {
        let mut client = DcpClient::with_defaults();
        client.register_tool("tool1", "Tool 1", [0; 32], 0);
        client.register_tool("tool2", "Tool 2", [0; 32], 0);
        client.register_tool("tool3", "Tool 3", [0; 32], 0);

        let mut other = CapabilityManifest::new(1);
        other.set_tool(2);
        other.set_tool(3);
        other.set_tool(4);

        let intersection = client.intersect_capabilities(&other);
        assert!(!intersection.has_tool(1));
        assert!(intersection.has_tool(2));
        assert!(intersection.has_tool(3));
        assert!(!intersection.has_tool(4));
    }

    #[test]
    fn test_envelope_creation() {
        let envelope = create_envelope(MessageType::Tool, Flags::STREAMING, 1024);
        assert!(envelope.is_streaming());
        assert!(!envelope.is_compressed());
        assert!(!envelope.is_signed());

        let bytes = envelope.as_bytes();
        let parsed = parse_envelope(bytes).unwrap();
        assert_eq!(parsed.get_message_type(), Some(MessageType::Tool));
    }

    #[test]
    fn test_mcp_fallback() {
        let mut client = DcpClient::with_defaults();

        // Connect should fall back to MCP since DCP is not implemented
        let result = client.connect("tcp://localhost:9000");
        assert!(result.is_ok());
        assert_eq!(client.state(), ConnectionState::ConnectedMcp);
    }

    #[test]
    fn test_protocol_preference() {
        let mut client = DcpClient::with_defaults();

        // Default should prefer DCP
        assert!(client.prefer_dcp());
        assert!(client.is_dcp_enabled());

        // Disable DCP preference
        client.set_prefer_dcp(false);
        assert!(!client.prefer_dcp());

        // Re-enable
        client.set_prefer_dcp(true);
        assert!(client.prefer_dcp());
    }

    #[test]
    fn test_enable_disable_dcp() {
        let mut client = DcpClient::with_defaults();

        assert!(client.is_dcp_enabled());

        client.disable_dcp();
        assert!(!client.is_dcp_enabled());
        assert!(!client.prefer_dcp()); // prefer_dcp requires enabled

        client.enable_dcp();
        assert!(client.is_dcp_enabled());
    }

    #[test]
    fn test_current_protocol() {
        let mut client = DcpClient::with_defaults();

        // Not connected yet
        assert!(client.current_protocol().is_none());

        // Connect (falls back to MCP)
        client.connect("tcp://localhost:9000").unwrap();
        assert_eq!(client.current_protocol(), Some(Protocol::Mcp));
    }

    #[test]
    fn test_select_protocol() {
        let mut client = DcpClient::with_defaults();

        // Not connected - no protocol available
        assert!(client.select_protocol().is_none());

        // Connect (falls back to MCP)
        client.connect("tcp://localhost:9000").unwrap();

        // Should select MCP since that's what we're connected with
        assert_eq!(client.select_protocol(), Some(Protocol::Mcp));
    }

    #[test]
    fn test_is_mcp_connected() {
        let mut client = DcpClient::with_defaults();

        assert!(!client.is_mcp_connected());

        client.connect("tcp://localhost:9000").unwrap();
        assert!(client.is_mcp_connected());
        assert!(!client.is_dcp_connected());
    }

    #[test]
    fn test_invoke_tool_auto() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let tool_id = client.register_tool("test_tool", "Test", [0; 32], 0);
        client.register_tool_mcp("test_tool", tool_id);

        let args = b"{}";
        let result = client.invoke_tool_auto(tool_id, args);

        // Should succeed and return MCP result (since we're connected via MCP)
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.is_mcp());
    }

    #[test]
    fn test_invocation_result() {
        let dcp_result = InvocationResult::Dcp(vec![1, 2, 3]);
        assert!(dcp_result.is_dcp());
        assert!(!dcp_result.is_mcp());
        assert_eq!(dcp_result.as_dcp(), Some(&[1, 2, 3][..]));
        assert!(dcp_result.as_mcp().is_none());

        let mcp_result = InvocationResult::Mcp("{}".to_string());
        assert!(!mcp_result.is_dcp());
        assert!(mcp_result.is_mcp());
        assert!(mcp_result.as_dcp().is_none());
        assert_eq!(mcp_result.as_mcp(), Some("{}"));
    }

    // BME Message Tests

    #[test]
    fn test_dcp_message_tool() {
        let payload = vec![1, 2, 3, 4, 5];
        let message = DcpMessage::tool(payload.clone());

        assert_eq!(message.message_type, MessageType::Tool);
        assert_eq!(message.flags, 0);
        assert_eq!(message.payload, payload);
        assert!(!message.is_streaming());
        assert!(!message.is_compressed());
        assert!(!message.is_signed());
    }

    #[test]
    fn test_dcp_message_resource() {
        let payload = b"resource://test".to_vec();
        let message = DcpMessage::resource(payload.clone());

        assert_eq!(message.message_type, MessageType::Resource);
        assert_eq!(message.payload, payload);
    }

    #[test]
    fn test_dcp_message_prompt() {
        let payload = b"Hello, AI!".to_vec();
        let message = DcpMessage::prompt(payload.clone());

        assert_eq!(message.message_type, MessageType::Prompt);
        assert_eq!(message.payload, payload);
    }

    #[test]
    fn test_dcp_message_response() {
        let payload = b"Response data".to_vec();
        let message = DcpMessage::response(payload.clone());

        assert_eq!(message.message_type, MessageType::Response);
        assert_eq!(message.payload, payload);
    }

    #[test]
    fn test_dcp_message_error() {
        let payload = b"Error occurred".to_vec();
        let message = DcpMessage::error(payload.clone());

        assert_eq!(message.message_type, MessageType::Error);
        assert_eq!(message.payload, payload);
    }

    #[test]
    fn test_dcp_message_stream() {
        let payload = b"Streaming data".to_vec();
        let message = DcpMessage::stream(payload.clone());

        assert_eq!(message.message_type, MessageType::Stream);
        assert!(message.is_streaming());
        assert_eq!(message.payload, payload);
    }

    #[test]
    fn test_dcp_message_flags() {
        let message = DcpMessage::tool(vec![]).with_streaming().with_compressed().with_signed();

        assert!(message.is_streaming());
        assert!(message.is_compressed());
        assert!(message.is_signed());
    }

    #[test]
    fn test_dcp_message_encode_decode_roundtrip() {
        let original = DcpMessage::new(
            MessageType::Tool,
            Flags::STREAMING | Flags::SIGNED,
            vec![0xDE, 0xAD, 0xBE, 0xEF],
        );

        let encoded = original.encode();
        let decoded = DcpMessage::decode(&encoded).unwrap();

        assert_eq!(decoded.message_type, original.message_type);
        assert_eq!(decoded.flags, original.flags);
        assert_eq!(decoded.payload, original.payload);
    }

    #[test]
    fn test_dcp_message_decode_insufficient_data() {
        let bytes = vec![0u8; 4]; // Less than BME header size
        let result = DcpMessage::decode(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_dcp_message_decode_invalid_magic() {
        let mut bytes = vec![0u8; 16];
        bytes[0] = 0xFF;
        bytes[1] = 0xFF;
        let result = DcpMessage::decode(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_dcp_message_all_types_roundtrip() {
        let test_cases = vec![
            DcpMessage::tool(vec![1, 2, 3]),
            DcpMessage::resource(b"uri://test".to_vec()),
            DcpMessage::prompt(b"test prompt".to_vec()),
            DcpMessage::response(b"test response".to_vec()),
            DcpMessage::error(b"test error".to_vec()),
            DcpMessage::stream(b"streaming".to_vec()),
        ];

        for original in test_cases {
            let encoded = original.encode();
            let decoded = DcpMessage::decode(&encoded).unwrap();
            assert_eq!(decoded.message_type, original.message_type);
            assert_eq!(decoded.payload, original.payload);
        }
    }

    // Stream Assembler Tests

    #[test]
    fn test_stream_assembler_single_chunk() {
        let mut assembler = StreamAssembler::new();

        let chunk = StreamChunk::new(0, ChunkFlags::FIRST | ChunkFlags::LAST, 5);
        let payload = vec![1, 2, 3, 4, 5];

        assembler.add_chunk(chunk.as_bytes(), &payload).unwrap();

        assert!(assembler.is_complete());
        assert!(!assembler.has_error());
        assert_eq!(assembler.data(), Some(payload.as_slice()));
        assert_eq!(assembler.chunk_count(), 1);
    }

    #[test]
    fn test_stream_assembler_multiple_chunks() {
        let mut assembler = StreamAssembler::new();

        // First chunk
        let chunk1 = StreamChunk::first(0, 3);
        assembler.add_chunk(chunk1.as_bytes(), &[1, 2, 3]).unwrap();
        assert!(!assembler.is_complete());

        // Continuation chunk
        let chunk2 = StreamChunk::continuation(1, 3);
        assembler.add_chunk(chunk2.as_bytes(), &[4, 5, 6]).unwrap();
        assert!(!assembler.is_complete());

        // Last chunk
        let chunk3 = StreamChunk::last(2, 2);
        assembler.add_chunk(chunk3.as_bytes(), &[7, 8]).unwrap();
        assert!(assembler.is_complete());

        assert_eq!(assembler.data(), Some(&[1, 2, 3, 4, 5, 6, 7, 8][..]));
        assert_eq!(assembler.chunk_count(), 3);
    }

    #[test]
    fn test_stream_assembler_out_of_order() {
        let mut assembler = StreamAssembler::new();

        let chunk1 = StreamChunk::first(0, 3);
        assembler.add_chunk(chunk1.as_bytes(), &[1, 2, 3]).unwrap();

        // Try to add chunk with wrong sequence
        let chunk_wrong = StreamChunk::continuation(5, 3);
        let result = assembler.add_chunk(chunk_wrong.as_bytes(), &[4, 5, 6]);
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_assembler_error_chunk() {
        let mut assembler = StreamAssembler::new();

        let chunk = StreamChunk::error(0, 0);
        let result = assembler.add_chunk(chunk.as_bytes(), &[]);

        assert!(result.is_err());
        assert!(assembler.has_error());
        assert!(assembler.is_complete());
        assert!(assembler.data().is_none());
    }

    #[test]
    fn test_stream_assembler_take_data() {
        let mut assembler = StreamAssembler::new();

        let chunk = StreamChunk::new(0, ChunkFlags::FIRST | ChunkFlags::LAST, 3);
        assembler.add_chunk(chunk.as_bytes(), &[1, 2, 3]).unwrap();

        let data = assembler.take_data();
        assert_eq!(data, Some(vec![1, 2, 3]));
    }

    // Stream Builder Tests

    #[test]
    fn test_stream_builder_single_chunk() {
        let mut builder = StreamBuilder::new(100);
        let data = vec![1, 2, 3, 4, 5];

        let chunks = builder.build_chunks(&data);

        assert_eq!(chunks.len(), 1);
        let (chunk, payload) = &chunks[0];
        assert!(chunk.is_first());
        assert!(chunk.is_last());
        assert_eq!(payload, &data);
    }

    #[test]
    fn test_stream_builder_multiple_chunks() {
        let mut builder = StreamBuilder::new(3);
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let chunks = builder.build_chunks(&data);

        assert_eq!(chunks.len(), 3);

        // First chunk
        assert!(chunks[0].0.is_first());
        assert!(!chunks[0].0.is_last());
        assert_eq!(chunks[0].1, vec![1, 2, 3]);

        // Middle chunk
        assert!(chunks[1].0.is_continuation());
        assert!(!chunks[1].0.is_last());
        assert_eq!(chunks[1].1, vec![4, 5, 6]);

        // Last chunk
        assert!(!chunks[2].0.is_first());
        assert!(chunks[2].0.is_last());
        assert_eq!(chunks[2].1, vec![7, 8]);
    }

    #[test]
    fn test_stream_builder_reset() {
        let mut builder = StreamBuilder::new(10);

        let _ = builder.build_chunks(&[1, 2, 3]);
        builder.reset();

        let chunks = builder.build_chunks(&[4, 5, 6]);
        // Copy sequence from packed struct
        let sequence = chunks[0].0.sequence;
        assert_eq!(sequence, 0);
    }

    #[test]
    fn test_stream_builder_assembler_roundtrip() {
        let mut builder = StreamBuilder::new(5);
        let original_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        let chunks = builder.build_chunks(&original_data);

        let mut assembler = StreamAssembler::new();
        for (chunk, payload) in chunks {
            assembler.add_chunk(chunk.as_bytes(), &payload).unwrap();
        }

        assert!(assembler.is_complete());
        assert_eq!(assembler.take_data(), Some(original_data));
    }

    // Client Message Creation Tests

    #[test]
    fn test_client_create_tool_message() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let tool_id = client.register_tool("test", "Test tool", [0; 32], 0);
        let args = vec![1, 2, 3, 4];

        let message = client.create_tool_message(tool_id, &args).unwrap();

        assert_eq!(message.message_type, MessageType::Tool);
        assert!(message.payload.len() >= ToolInvocation::SIZE + args.len());
    }

    #[test]
    fn test_client_create_resource_message() {
        let client = DcpClient::with_defaults();
        let message = client.create_resource_message("file:///test.txt");

        assert_eq!(message.message_type, MessageType::Resource);
        assert_eq!(message.payload, b"file:///test.txt");
    }

    #[test]
    fn test_client_create_prompt_message() {
        let client = DcpClient::with_defaults();
        let message = client.create_prompt_message("Hello, AI!");

        assert_eq!(message.message_type, MessageType::Prompt);
        assert_eq!(message.payload, b"Hello, AI!");
    }

    #[test]
    fn test_client_create_error_message() {
        let client = DcpClient::with_defaults();
        let message = client.create_error_message(404, "Not found");

        assert_eq!(message.message_type, MessageType::Error);
        // First 4 bytes are error code
        assert_eq!(&message.payload[0..4], &404u32.to_le_bytes());
        assert_eq!(&message.payload[4..], b"Not found");
    }

    #[test]
    fn test_client_encode_decode_message() {
        let client = DcpClient::with_defaults();

        let payload = b"test payload".to_vec();
        let encoded = client.encode_message(MessageType::Response, &payload);
        let decoded = client.decode_message(&encoded).unwrap();

        assert_eq!(decoded.message_type, MessageType::Response);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_client_stream_builder_assembler() {
        let client = DcpClient::with_defaults();

        let mut builder = client.create_stream_builder(10);
        let assembler = client.create_stream_assembler();

        // Verify they work
        let chunks = builder.build_chunks(&[1, 2, 3]);
        assert_eq!(chunks.len(), 1);
        assert!(!assembler.is_complete());
    }

    // ZCTI (Zero-Copy Tool Invocation) Tests

    #[test]
    fn test_zcti_builder_basic() {
        let builder = ZctiBuilder::new(42).add_bool(true).add_i32(123).add_string("hello");

        assert_eq!(builder.arg_count(), 3);

        let (invocation, data) = builder.build_with_data();
        assert_eq!(invocation.tool_id, 42);
        assert_eq!(invocation.arg_count(), 3);
        assert!(!data.is_empty());
    }

    #[test]
    fn test_zcti_builder_all_types() {
        let builder = ZctiBuilder::new(1)
            .add_null()
            .add_bool(false)
            .add_i32(-42)
            .add_i64(i64::MAX)
            .add_f64(3.14159)
            .add_string("test")
            .add_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]);

        assert_eq!(builder.arg_count(), 7);
    }

    #[test]
    fn test_zcti_reader_basic() {
        let builder = ZctiBuilder::new(42).add_bool(true).add_i32(123).add_string("hello");

        let (invocation, data) = builder.build_with_data();
        let mut reader = ZctiReader::new(&invocation, &data);

        assert_eq!(reader.tool_id(), 42);
        assert_eq!(reader.arg_count(), 3);
        assert!(reader.has_more());

        assert_eq!(reader.read_bool().unwrap(), true);
        assert_eq!(reader.read_i32().unwrap(), 123);
        assert_eq!(reader.read_string().unwrap(), "hello");
        assert!(!reader.has_more());
    }

    #[test]
    fn test_zcti_reader_all_types() {
        let builder = ZctiBuilder::new(1)
            .add_bool(false)
            .add_i32(-42)
            .add_i64(i64::MAX)
            .add_f64(3.14159)
            .add_string("test")
            .add_bytes(&[0xDE, 0xAD, 0xBE, 0xEF]);

        let (invocation, data) = builder.build_with_data();
        let mut reader = ZctiReader::new(&invocation, &data);

        assert_eq!(reader.read_bool().unwrap(), false);
        assert_eq!(reader.read_i32().unwrap(), -42);
        assert_eq!(reader.read_i64().unwrap(), i64::MAX);
        assert!((reader.read_f64().unwrap() - 3.14159).abs() < 0.00001);
        assert_eq!(reader.read_string().unwrap(), "test");
        assert_eq!(reader.read_bytes().unwrap(), &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_zcti_reader_skip() {
        let builder = ZctiBuilder::new(1)
            .add_bool(true)
            .add_i32(123)
            .add_string("skip me")
            .add_i64(456);

        let (invocation, data) = builder.build_with_data();
        let mut reader = ZctiReader::new(&invocation, &data);

        reader.skip().unwrap(); // Skip bool
        reader.skip().unwrap(); // Skip i32
        reader.skip().unwrap(); // Skip string
        assert_eq!(reader.read_i64().unwrap(), 456);
    }

    #[test]
    fn test_zcti_type_mismatch() {
        let builder = ZctiBuilder::new(1).add_bool(true);
        let (invocation, data) = builder.build_with_data();
        let mut reader = ZctiReader::new(&invocation, &data);

        // Try to read as wrong type
        let result = reader.read_i32();
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_arg_buffer() {
        let mut buffer = SharedArgBuffer::new(1024);

        let data1 = b"hello";
        let offset1 = buffer.write(data1).unwrap();
        assert_eq!(offset1, 0);

        let data2 = b"world";
        let offset2 = buffer.write(data2).unwrap();
        assert_eq!(offset2, 5);

        assert_eq!(buffer.read(offset1, 5).unwrap(), b"hello");
        assert_eq!(buffer.read(offset2, 5).unwrap(), b"world");
    }

    #[test]
    fn test_shared_arg_buffer_overflow() {
        let mut buffer = SharedArgBuffer::new(10);

        let result = buffer.write(&[0u8; 20]);
        assert!(result.is_err());
    }

    #[test]
    fn test_shared_arg_buffer_reset() {
        let mut buffer = SharedArgBuffer::new(100);

        buffer.write(b"test").unwrap();
        assert_eq!(buffer.offset(), 4);

        buffer.reset();
        assert_eq!(buffer.offset(), 0);
    }

    #[test]
    fn test_client_zcti_integration() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let tool_id = client.register_tool("test_tool", "Test", [0; 32], 0);

        let builder = client.create_zcti_builder(tool_id).unwrap().add_string("arg1").add_i32(42);

        let encoded = client.invoke_tool_zcti(builder).unwrap();

        // Decode and verify
        let message = client.decode_message(&encoded).unwrap();
        assert_eq!(message.message_type, MessageType::Tool);

        let mut reader = client.parse_tool_invocation(&message).unwrap();
        assert_eq!(reader.tool_id(), tool_id);
        assert_eq!(reader.read_string().unwrap(), "arg1");
        assert_eq!(reader.read_i32().unwrap(), 42);
    }

    // MCP Compatibility Tests

    #[test]
    fn test_mcp_adapter_available() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        assert!(client.has_mcp_fallback());
        assert!(client.mcp_adapter().is_some());
    }

    #[test]
    fn test_mcp_register_tool() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let tool_id = client.register_tool("read_file", "Read a file", [0; 32], 0);
        client.register_tool_mcp("read_file", tool_id);

        let adapter = client.mcp_adapter().unwrap();
        assert_eq!(adapter.resolve_tool_name("read_file"), Some(tool_id as u16));
    }

    #[test]
    fn test_mcp_handle_initialize() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let request = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;
        let response = client.handle_mcp_request(request).unwrap();

        assert!(response.contains("protocolVersion"));
        assert!(response.contains("capabilities"));
    }

    #[test]
    fn test_mcp_handle_tools_list() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let tool_id = client.register_tool("test_tool", "Test tool", [0; 32], 0);
        client.register_tool_mcp("test_tool", tool_id);

        let request = r#"{"jsonrpc":"2.0","method":"tools/list","id":2}"#;
        let response = client.handle_mcp_request(request).unwrap();

        assert!(response.contains("tools"));
        assert!(response.contains("test_tool"));
    }

    #[test]
    fn test_mcp_handle_tools_call() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let tool_id = client.register_tool("echo", "Echo tool", [0; 32], 0);
        client.register_tool_mcp("echo", tool_id);

        let request = r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"text":"hello"}},"id":3}"#;
        let response = client.handle_mcp_request(request).unwrap();

        assert!(response.contains("content"));
        assert!(response.contains("echo"));
    }

    #[test]
    fn test_mcp_handle_unknown_method() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let request = r#"{"jsonrpc":"2.0","method":"unknown/method","id":4}"#;
        let response = client.handle_mcp_request(request).unwrap();

        assert!(response.contains("error"));
        assert!(response.contains("-32601")); // Method not found
    }

    #[test]
    fn test_mcp_handle_resources_list() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let request = r#"{"jsonrpc":"2.0","method":"resources/list","id":5}"#;
        let response = client.handle_mcp_request(request).unwrap();

        assert!(response.contains("resources"));
        assert!(response.contains("[]")); // Empty list
    }

    #[test]
    fn test_mcp_handle_prompts_list() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let request = r#"{"jsonrpc":"2.0","method":"prompts/list","id":6}"#;
        let response = client.handle_mcp_request(request).unwrap();

        assert!(response.contains("prompts"));
        assert!(response.contains("[]")); // Empty list
    }

    #[test]
    fn test_dcp_to_mcp_response() {
        let client = DcpClient::with_defaults();

        let payload = serde_json::to_vec(&serde_json::json!({"result": "success"})).unwrap();
        let message = DcpMessage::response(payload);

        let mcp_json = client.dcp_to_mcp(&message).unwrap();
        assert!(mcp_json.contains("jsonrpc"));
        assert!(mcp_json.contains("result"));
    }

    #[test]
    fn test_dcp_to_mcp_error() {
        let client = DcpClient::with_defaults();

        let message = client.create_error_message(404, "Not found");
        let mcp_json = client.dcp_to_mcp(&message).unwrap();

        assert!(mcp_json.contains("error"));
        assert!(mcp_json.contains("404"));
        assert!(mcp_json.contains("Not found"));
    }

    #[test]
    fn test_mcp_to_dcp() {
        let mut client = DcpClient::with_defaults();
        client.connect("tcp://localhost:9000").unwrap();

        let mcp_request =
            r#"{"jsonrpc":"2.0","method":"tools/call","params":{"name":"test"},"id":1}"#;
        let dcp_message = client.mcp_to_dcp(mcp_request).unwrap();

        assert_eq!(dcp_message.message_type, MessageType::Tool);
        assert!(!dcp_message.payload.is_empty());
    }

    // Property-Based Test Generators

    /// Generate an arbitrary MessageType
    fn arb_message_type() -> impl Strategy<Value = MessageType> {
        prop_oneof![
            Just(MessageType::Tool),
            Just(MessageType::Resource),
            Just(MessageType::Prompt),
            Just(MessageType::Response),
            Just(MessageType::Error),
            Just(MessageType::Stream),
        ]
    }

    /// Generate arbitrary flags (valid combinations)
    fn arb_flags() -> impl Strategy<Value = u8> {
        prop_oneof![
            Just(0u8),
            Just(Flags::STREAMING),
            Just(Flags::COMPRESSED),
            Just(Flags::SIGNED),
            Just(Flags::STREAMING | Flags::COMPRESSED),
            Just(Flags::STREAMING | Flags::SIGNED),
            Just(Flags::COMPRESSED | Flags::SIGNED),
            Just(Flags::STREAMING | Flags::COMPRESSED | Flags::SIGNED),
        ]
    }

    /// Generate arbitrary payload (limited size for testing)
    fn arb_payload() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..1024)
    }

    /// Generate an arbitrary DcpMessage
    fn arb_dcp_message() -> impl Strategy<Value = DcpMessage> {
        (arb_message_type(), arb_flags(), arb_payload()).prop_map(
            |(message_type, flags, payload)| DcpMessage::new(message_type, flags, payload),
        )
    }

    /// Generate arbitrary chunk size (reasonable range)
    fn arb_chunk_size() -> impl Strategy<Value = usize> {
        1usize..256
    }

    /// Generate arbitrary data for streaming
    fn arb_stream_data() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 1..2048)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 4: DCP Binary Message Envelope Round-Trip
        /// *For any* valid DCP message, encoding to BME format and decoding back
        /// SHALL produce an equivalent message.
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_bme_roundtrip(message in arb_dcp_message()) {
            let encoded = message.encode();
            let decoded = DcpMessage::decode(&encoded).expect("Decoding should succeed");

            prop_assert_eq!(decoded.message_type, message.message_type);
            prop_assert_eq!(decoded.flags, message.flags);
            prop_assert_eq!(decoded.payload, message.payload);
        }

        /// Property 4b: BME envelope header is correctly preserved
        /// *For any* valid message type, flags, and payload length, the envelope
        /// header SHALL be correctly encoded and decoded.
        #[test]
        fn prop_bme_envelope_header(
            message_type in arb_message_type(),
            flags in arb_flags(),
            payload_len in 0u32..65536
        ) {
            let envelope = BinaryMessageEnvelope::new(message_type, flags, payload_len);
            let bytes = envelope.as_bytes();
            let parsed = BinaryMessageEnvelope::from_bytes(bytes).expect("Parsing should succeed");

            // Copy values from packed struct
            let parsed_type = parsed.message_type;
            let parsed_flags = parsed.flags;
            let parsed_len = parsed.payload_len;

            prop_assert_eq!(parsed_type, message_type as u8);
            prop_assert_eq!(parsed_flags, flags);
            prop_assert_eq!(parsed_len, payload_len);
        }

        /// Property 4c: Stream chunking and reassembly round-trip
        /// *For any* data and chunk size, chunking and reassembling SHALL produce
        /// the original data.
        #[test]
        fn prop_stream_roundtrip(
            data in arb_stream_data(),
            chunk_size in arb_chunk_size()
        ) {
            let mut builder = StreamBuilder::new(chunk_size);
            let chunks = builder.build_chunks(&data);

            let mut assembler = StreamAssembler::new();
            for (chunk, payload) in chunks {
                assembler.add_chunk(chunk.as_bytes(), &payload).expect("Adding chunk should succeed");
            }

            prop_assert!(assembler.is_complete());
            prop_assert!(!assembler.has_error());
            prop_assert_eq!(assembler.take_data(), Some(data));
        }

        /// Property 4d: All message types round-trip correctly
        /// *For any* message type and payload, encoding and decoding SHALL preserve
        /// the message type and payload.
        #[test]
        fn prop_all_message_types_roundtrip(
            message_type in arb_message_type(),
            payload in arb_payload()
        ) {
            let message = DcpMessage::new(message_type, 0, payload.clone());
            let encoded = message.encode();
            let decoded = DcpMessage::decode(&encoded).expect("Decoding should succeed");

            prop_assert_eq!(decoded.message_type, message_type);
            prop_assert_eq!(decoded.payload, payload);
        }

        /// Property 4e: Flags are correctly preserved through round-trip
        /// *For any* valid flag combination, encoding and decoding SHALL preserve
        /// all flag bits.
        #[test]
        fn prop_flags_preserved(flags in arb_flags()) {
            let message = DcpMessage::new(MessageType::Tool, flags, vec![1, 2, 3]);
            let encoded = message.encode();
            let decoded = DcpMessage::decode(&encoded).expect("Decoding should succeed");

            prop_assert_eq!(decoded.flags, flags);
            prop_assert_eq!(decoded.is_streaming(), flags & Flags::STREAMING != 0);
            prop_assert_eq!(decoded.is_compressed(), flags & Flags::COMPRESSED != 0);
            prop_assert_eq!(decoded.is_signed(), flags & Flags::SIGNED != 0);
        }

        /// Property 5: DCP MCP Compatibility
        /// *For any* valid MCP JSON-RPC request, the McpAdapter SHALL produce a valid
        /// response that matches MCP protocol semantics.
        /// **Validates: Requirements 2.4**
        #[test]
        fn prop_mcp_initialize_response_valid(id in 1i64..1000) {
            let request = format!(r#"{{"jsonrpc":"2.0","method":"initialize","id":{}}}"#, id);

            let mut client = DcpClient::with_defaults();
            client.connect("tcp://localhost:9000").expect("Connection should succeed");

            let response = client.handle_mcp_request(&request).expect("Should handle initialize");

            // Verify response is valid JSON-RPC
            let parsed: serde_json::Value = serde_json::from_str(&response).expect("Should be valid JSON");
            prop_assert_eq!(parsed["jsonrpc"].as_str(), Some("2.0"));
            prop_assert!(parsed.get("result").is_some() || parsed.get("error").is_some());

            // If success, verify capabilities
            if let Some(result) = parsed.get("result") {
                prop_assert!(result.get("capabilities").is_some());
                prop_assert!(result.get("protocolVersion").is_some());
            }
        }

        /// Property 5b: MCP tools/list returns registered tools
        /// *For any* set of registered tools, tools/list SHALL return all of them.
        #[test]
        fn prop_mcp_tools_list_contains_registered(
            tool_names in prop::collection::vec("[a-z_]+", 1..5)
        ) {
            let mut client = DcpClient::with_defaults();
            client.connect("tcp://localhost:9000").expect("Connection should succeed");

            // Register tools
            for name in &tool_names {
                let tool_id = client.register_tool(name, "Test tool", [0; 32], 0);
                client.register_tool_mcp(name, tool_id);
            }

            let request = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
            let response = client.handle_mcp_request(request).expect("Should handle tools/list");

            let parsed: serde_json::Value = serde_json::from_str(&response).expect("Should be valid JSON");
            let tools = parsed["result"]["tools"].as_array().expect("Should have tools array");

            // Verify all registered tools are in the response
            for name in &tool_names {
                let found = tools.iter().any(|t| t["name"].as_str() == Some(name.as_str()));
                prop_assert!(found, "Tool {} should be in response", name);
            }
        }

        /// Property 6: DCP Capability Manifest Intersection
        /// *For any* two capability manifests, their intersection SHALL contain only
        /// capabilities present in both.
        /// **Validates: Requirements 2.6**
        #[test]
        fn prop_capability_intersection(
            tools1 in prop::collection::vec(0u16..1000, 0..20),
            tools2 in prop::collection::vec(0u16..1000, 0..20),
            resources1 in prop::collection::vec(0u16..500, 0..10),
            resources2 in prop::collection::vec(0u16..500, 0..10),
            prompts1 in prop::collection::vec(0u16..200, 0..5),
            prompts2 in prop::collection::vec(0u16..200, 0..5),
        ) {
            let mut m1 = CapabilityManifest::new(1);
            let mut m2 = CapabilityManifest::new(1);

            // Set capabilities in m1
            for &t in &tools1 { m1.set_tool(t); }
            for &r in &resources1 { m1.set_resource(r); }
            for &p in &prompts1 { m1.set_prompt(p); }

            // Set capabilities in m2
            for &t in &tools2 { m2.set_tool(t); }
            for &r in &resources2 { m2.set_resource(r); }
            for &p in &prompts2 { m2.set_prompt(p); }

            // Compute intersection
            let intersection = m1.intersect(&m2);

            // Verify: intersection only contains capabilities in BOTH manifests
            for t in 0u16..1000 {
                let in_both = m1.has_tool(t) && m2.has_tool(t);
                prop_assert_eq!(
                    intersection.has_tool(t), in_both,
                    "Tool {} should be in intersection iff in both manifests", t
                );
            }

            for r in 0u16..500 {
                let in_both = m1.has_resource(r) && m2.has_resource(r);
                prop_assert_eq!(
                    intersection.has_resource(r), in_both,
                    "Resource {} should be in intersection iff in both manifests", r
                );
            }

            for p in 0u16..200 {
                let in_both = m1.has_prompt(p) && m2.has_prompt(p);
                prop_assert_eq!(
                    intersection.has_prompt(p), in_both,
                    "Prompt {} should be in intersection iff in both manifests", p
                );
            }
        }

        /// Property 6b: Capability manifest round-trip through bytes
        /// *For any* capability manifest, serializing and parsing SHALL produce
        /// an equivalent manifest.
        #[test]
        fn prop_capability_manifest_roundtrip(
            tools in prop::collection::vec(0u16..8000, 0..50),
            resources in prop::collection::vec(0u16..1000, 0..20),
            prompts in prop::collection::vec(0u16..500, 0..10),
            extensions in 0u64..u64::MAX,
        ) {
            let mut manifest = CapabilityManifest::new(1);

            for &t in &tools { manifest.set_tool(t); }
            for &r in &resources { manifest.set_resource(r); }
            for &p in &prompts { manifest.set_prompt(p); }
            manifest.extensions = extensions;

            // Serialize and parse
            let bytes = manifest.as_bytes();
            let parsed = CapabilityManifest::from_bytes(bytes).expect("Should parse");

            // Verify all capabilities preserved
            for &t in &tools {
                prop_assert!(parsed.has_tool(t), "Tool {} should be preserved", t);
            }
            for &r in &resources {
                prop_assert!(parsed.has_resource(r), "Resource {} should be preserved", r);
            }
            for &p in &prompts {
                prop_assert!(parsed.has_prompt(p), "Prompt {} should be preserved", p);
            }
            prop_assert_eq!(parsed.extensions, extensions);
        }

        /// Property 7: Ed25519 Signature Verification
        /// *For any* signed tool definition, verification with the correct public key
        /// SHALL succeed, and verification with any other key SHALL fail.
        /// **Validates: Requirements 2.7**
        #[test]
        fn prop_signature_verification_tool_def(
            seed in prop::array::uniform32(any::<u8>()),
            tool_id in 1u32..1000,
            schema_hash in prop::array::uniform32(any::<u8>()),
            capabilities in any::<u64>(),
        ) {
            let signer = Signer::from_seed(&seed);
            let signed_def = signer.sign_tool_def(tool_id, schema_hash, capabilities);

            // Verification with correct key should succeed
            prop_assert!(Verifier::verify_tool_def(&signed_def).is_ok());

            // Verification should fail if any field is tampered
            let mut tampered = signed_def.clone();
            tampered.tool_id = tool_id.wrapping_add(1);
            prop_assert!(Verifier::verify_tool_def(&tampered).is_err());

            let mut tampered = signed_def.clone();
            tampered.schema_hash[0] ^= 0xFF;
            prop_assert!(Verifier::verify_tool_def(&tampered).is_err());

            let mut tampered = signed_def.clone();
            tampered.capabilities ^= 0xFFFFFFFF;
            prop_assert!(Verifier::verify_tool_def(&tampered).is_err());
        }

        /// Property 7b: Ed25519 Signature Verification for Invocations
        /// *For any* signed invocation, verification with the correct public key
        /// SHALL succeed, and verification with any other key SHALL fail.
        #[test]
        fn prop_signature_verification_invocation(
            seed in prop::array::uniform32(any::<u8>()),
            wrong_seed in prop::array::uniform32(any::<u8>()),
            tool_id in 1u32..1000,
            nonce in any::<u64>(),
            timestamp in any::<u64>(),
            args in prop::collection::vec(any::<u8>(), 0..256),
        ) {
            let signer = Signer::from_seed(&seed);
            let public_key = signer.public_key_bytes();
            let signed_inv = signer.sign_invocation(tool_id, nonce, timestamp, &args);

            // Verification with correct key should succeed
            prop_assert!(Verifier::verify_invocation(&signed_inv, &public_key).is_ok());

            // Args hash should match
            prop_assert!(Verifier::verify_args_hash(&signed_inv, &args));

            // Verification with wrong key should fail (unless seeds happen to be equal)
            if seed != wrong_seed {
                let wrong_signer = Signer::from_seed(&wrong_seed);
                let wrong_key = wrong_signer.public_key_bytes();
                prop_assert!(Verifier::verify_invocation(&signed_inv, &wrong_key).is_err());
            }

            // Verification should fail if any field is tampered
            let mut tampered = signed_inv.clone();
            tampered.tool_id = tool_id.wrapping_add(1);
            prop_assert!(Verifier::verify_invocation(&tampered, &public_key).is_err());

            let mut tampered = signed_inv.clone();
            tampered.nonce ^= 0xFFFFFFFF;
            prop_assert!(Verifier::verify_invocation(&tampered, &public_key).is_err());
        }
    }

    // Capability Manifest Unit Tests

    #[test]
    fn test_capability_manifest_parsing() {
        let mut manifest = CapabilityManifest::new(1);
        manifest.set_tool(42);
        manifest.set_resource(10);
        manifest.set_prompt(5);

        let bytes = manifest.as_bytes();
        let parsed = DcpClient::parse_capability_manifest(bytes).unwrap();

        assert!(parsed.has_tool(42));
        assert!(parsed.has_resource(10));
        assert!(parsed.has_prompt(5));
    }

    #[test]
    fn test_capability_manifest_serialization() {
        let mut client = DcpClient::with_defaults();
        client.register_tool("tool1", "Tool 1", [0; 32], 0);
        client.register_resource(10);
        client.register_prompt(5);

        let bytes = client.serialize_capabilities();
        let parsed = CapabilityManifest::from_bytes(&bytes).unwrap();

        assert!(parsed.has_tool(1)); // First tool gets ID 1
        assert!(parsed.has_resource(10));
        assert!(parsed.has_prompt(5));
    }

    #[test]
    fn test_resource_registration() {
        let mut client = DcpClient::with_defaults();

        assert!(!client.has_resource(42));
        client.register_resource(42);
        assert!(client.has_resource(42));
        client.unregister_resource(42);
        assert!(!client.has_resource(42));
    }

    #[test]
    fn test_prompt_registration() {
        let mut client = DcpClient::with_defaults();

        assert!(!client.has_prompt(10));
        client.register_prompt(10);
        assert!(client.has_prompt(10));
        client.unregister_prompt(10);
        assert!(!client.has_prompt(10));
    }

    #[test]
    fn test_extension_flags() {
        let mut client = DcpClient::with_defaults();

        assert!(!client.has_extension(5));
        client.set_extension(5);
        assert!(client.has_extension(5));
        client.clear_extension(5);
        assert!(!client.has_extension(5));
    }

    #[test]
    fn test_enforce_tool_success() {
        let mut client = DcpClient::with_defaults();
        let tool_id = client.register_tool("test", "Test", [0; 32], 0);

        assert!(client.enforce_tool(tool_id).is_ok());
    }

    #[test]
    fn test_enforce_tool_failure() {
        let client = DcpClient::with_defaults();

        // Tool 999 is not registered
        assert!(client.enforce_tool(999).is_err());
    }

    #[test]
    fn test_enforce_resource_success() {
        let mut client = DcpClient::with_defaults();
        client.register_resource(42);

        assert!(client.enforce_resource(42).is_ok());
    }

    #[test]
    fn test_enforce_resource_failure() {
        let client = DcpClient::with_defaults();

        assert!(client.enforce_resource(999).is_err());
    }

    #[test]
    fn test_enforce_prompt_success() {
        let mut client = DcpClient::with_defaults();
        client.register_prompt(10);

        assert!(client.enforce_prompt(10).is_ok());
    }

    #[test]
    fn test_enforce_prompt_failure() {
        let client = DcpClient::with_defaults();

        assert!(client.enforce_prompt(999).is_err());
    }

    #[test]
    fn test_negotiate_capabilities() {
        let mut client = DcpClient::with_defaults();
        client.register_tool("tool1", "Tool 1", [0; 32], 0);
        client.register_tool("tool2", "Tool 2", [0; 32], 0);
        client.register_tool("tool3", "Tool 3", [0; 32], 0);
        client.register_resource(1);
        client.register_resource(2);

        // Server only supports tools 2, 3, 4 and resource 2, 3
        let mut server_manifest = CapabilityManifest::new(1);
        server_manifest.set_tool(2);
        server_manifest.set_tool(3);
        server_manifest.set_tool(4);
        server_manifest.set_resource(2);
        server_manifest.set_resource(3);

        let negotiated = client.negotiate_capabilities(&server_manifest);

        // After negotiation, only common capabilities remain
        assert!(!negotiated.has_tool(1)); // Only client had this
        assert!(negotiated.has_tool(2)); // Both had this
        assert!(negotiated.has_tool(3)); // Both had this
        assert!(!negotiated.has_tool(4)); // Only server had this

        assert!(!negotiated.has_resource(1)); // Only client had this
        assert!(negotiated.has_resource(2)); // Both had this
        assert!(!negotiated.has_resource(3)); // Only server had this
    }

    #[test]
    fn test_capability_stats() {
        let mut client = DcpClient::with_defaults();
        client.register_tool("tool1", "Tool 1", [0; 32], 0);
        client.register_tool("tool2", "Tool 2", [0; 32], 0);
        client.register_resource(1);
        client.register_prompt(1);
        client.register_prompt(2);
        client.set_extension(0);

        let stats = client.capability_stats();

        assert_eq!(stats.tool_count, 2);
        assert_eq!(stats.resource_count, 1);
        assert_eq!(stats.prompt_count, 2);
        assert_eq!(stats.extension_count, 1);
        assert_eq!(stats.version, 1);
    }

    #[test]
    fn test_capability_iterators() {
        let mut client = DcpClient::with_defaults();
        client.register_tool("tool1", "Tool 1", [0; 32], 0);
        client.register_tool("tool2", "Tool 2", [0; 32], 0);
        client.register_resource(10);
        client.register_resource(20);
        client.register_prompt(5);

        let tool_ids: Vec<_> = client.tool_ids().collect();
        assert_eq!(tool_ids, vec![1, 2]);

        let resource_ids: Vec<_> = client.resource_ids().collect();
        assert_eq!(resource_ids, vec![10, 20]);

        let prompt_ids: Vec<_> = client.prompt_ids().collect();
        assert_eq!(prompt_ids, vec![5]);
    }
}
