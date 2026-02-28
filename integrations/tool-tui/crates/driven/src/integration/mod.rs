//! Cross-Crate Integration Module
//!
//! This module provides integration between Driven, Generator, and DCP crates,
//! enabling seamless cross-crate operations and shared binary format compatibility.
//!
//! ## Features
//!
//! - Driven tool registration in DCP
//! - Spec scaffolding with Generator templates
//! - Hook message passing via DCP
//! - DX ∞ binary format compatibility
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integration::{DrivenDcpBridge, SpecScaffolder, HookMessenger};
//!
//! // Register driven tools in DCP
//! let mut bridge = DrivenDcpBridge::new(dcp_client);
//! bridge.register_driven_tools()?;
//!
//! // Generate spec scaffolding
//! let scaffolder = SpecScaffolder::new(generator_bridge);
//! scaffolder.create_spec("feature-001", &params)?;
//!
//! // Send hook messages via DCP
//! let messenger = HookMessenger::new(dcp_client);
//! messenger.send_hook_message(&hook, &context)?;
//! ```

use crate::dcp::{DcpClient, DcpMessage, MessageType, ZctiBuilder};
use crate::generator_integration::{GenerateParams, GeneratorBridge};
use crate::hooks::{AgentHook, HookContext};
use crate::{DrivenError, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ==================== Driven-DCP Bridge ====================

/// Bridge between Driven and DCP for tool registration and communication
///
/// Provides APIs for registering Driven tools in DCP and invoking them
/// via the DCP protocol.
#[derive(Debug)]
pub struct DrivenDcpBridge {
    /// Registered tool IDs mapped by name
    tool_ids: HashMap<String, u32>,
    /// Whether the bridge is initialized
    initialized: bool,
}

/// Driven tool definitions for DCP registration
#[derive(Debug, Clone)]
pub struct DrivenTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool category
    pub category: DrivenToolCategory,
    /// Required capabilities (bitset)
    pub capabilities: u64,
}

/// Categories of Driven tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrivenToolCategory {
    /// Rule management tools
    Rules,
    /// Spec-driven development tools
    Specs,
    /// Hook management tools
    Hooks,
    /// Steering file tools
    Steering,
    /// Template tools
    Templates,
    /// Sync tools
    Sync,
}

impl DrivenDcpBridge {
    /// Create a new Driven-DCP bridge
    pub fn new() -> Self {
        Self {
            tool_ids: HashMap::new(),
            initialized: false,
        }
    }

    /// Register all Driven tools in the DCP client
    pub fn register_driven_tools(&mut self, client: &mut DcpClient) -> Result<()> {
        let tools = self.get_driven_tools();

        for tool in tools {
            let schema_hash = self.compute_schema_hash(&tool);
            let tool_id =
                client.register_tool(&tool.name, &tool.description, schema_hash, tool.capabilities);
            self.tool_ids.insert(tool.name.clone(), tool_id);

            // Also register with MCP adapter for compatibility
            client.register_tool_mcp(&tool.name, tool_id);
        }

        self.initialized = true;
        Ok(())
    }

    /// Get all Driven tool definitions
    fn get_driven_tools(&self) -> Vec<DrivenTool> {
        vec![
            // Rule management tools
            DrivenTool {
                name: "driven.rules.sync".to_string(),
                description: "Sync rules to all configured editors".to_string(),
                category: DrivenToolCategory::Rules,
                capabilities: 0x0001,
            },
            DrivenTool {
                name: "driven.rules.convert".to_string(),
                description: "Convert rules between formats".to_string(),
                category: DrivenToolCategory::Rules,
                capabilities: 0x0001,
            },
            DrivenTool {
                name: "driven.rules.validate".to_string(),
                description: "Validate rule syntax and semantics".to_string(),
                category: DrivenToolCategory::Rules,
                capabilities: 0x0001,
            },
            // Spec tools
            DrivenTool {
                name: "driven.spec.init".to_string(),
                description: "Initialize a new specification".to_string(),
                category: DrivenToolCategory::Specs,
                capabilities: 0x0002,
            },
            DrivenTool {
                name: "driven.spec.generate".to_string(),
                description: "Generate spec artifacts from description".to_string(),
                category: DrivenToolCategory::Specs,
                capabilities: 0x0002,
            },
            DrivenTool {
                name: "driven.spec.analyze".to_string(),
                description: "Analyze spec for consistency".to_string(),
                category: DrivenToolCategory::Specs,
                capabilities: 0x0002,
            },
            // Hook tools
            DrivenTool {
                name: "driven.hooks.list".to_string(),
                description: "List all registered hooks".to_string(),
                category: DrivenToolCategory::Hooks,
                capabilities: 0x0004,
            },
            DrivenTool {
                name: "driven.hooks.trigger".to_string(),
                description: "Manually trigger a hook".to_string(),
                category: DrivenToolCategory::Hooks,
                capabilities: 0x0004,
            },
            DrivenTool {
                name: "driven.hooks.enable".to_string(),
                description: "Enable a hook".to_string(),
                category: DrivenToolCategory::Hooks,
                capabilities: 0x0004,
            },
            DrivenTool {
                name: "driven.hooks.disable".to_string(),
                description: "Disable a hook".to_string(),
                category: DrivenToolCategory::Hooks,
                capabilities: 0x0004,
            },
            // Steering tools
            DrivenTool {
                name: "driven.steering.list".to_string(),
                description: "List steering files".to_string(),
                category: DrivenToolCategory::Steering,
                capabilities: 0x0008,
            },
            DrivenTool {
                name: "driven.steering.get".to_string(),
                description: "Get steering content for context".to_string(),
                category: DrivenToolCategory::Steering,
                capabilities: 0x0008,
            },
            // Template tools
            DrivenTool {
                name: "driven.templates.list".to_string(),
                description: "List available templates".to_string(),
                category: DrivenToolCategory::Templates,
                capabilities: 0x0010,
            },
            DrivenTool {
                name: "driven.templates.apply".to_string(),
                description: "Apply a template".to_string(),
                category: DrivenToolCategory::Templates,
                capabilities: 0x0010,
            },
        ]
    }

    /// Compute a schema hash for a tool (using Blake3)
    fn compute_schema_hash(&self, tool: &DrivenTool) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(tool.name.as_bytes());
        hasher.update(tool.description.as_bytes());
        hasher.update(&[tool.category as u8]);
        *hasher.finalize().as_bytes()
    }

    /// Get tool ID by name
    pub fn get_tool_id(&self, name: &str) -> Option<u32> {
        self.tool_ids.get(name).copied()
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, name: &str) -> bool {
        self.tool_ids.contains_key(name)
    }

    /// List all registered tool names
    pub fn list_tools(&self) -> Vec<&str> {
        self.tool_ids.keys().map(|s| s.as_str()).collect()
    }

    /// Create a ZCTI builder for a Driven tool
    pub fn create_invocation(&self, tool_name: &str, client: &DcpClient) -> Result<ZctiBuilder> {
        let tool_id = self
            .get_tool_id(tool_name)
            .ok_or_else(|| DrivenError::Config(format!("Tool not found: {}", tool_name)))?;
        client.create_zcti_builder(tool_id)
    }
}

impl Default for DrivenDcpBridge {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Spec Scaffolder ====================

/// Spec scaffolder that integrates with Generator for template-based scaffolding
#[derive(Debug)]
pub struct SpecScaffolder {
    /// Generator bridge for template rendering
    generator: GeneratorBridge,
    /// Base directory for specs
    spec_dir: PathBuf,
    /// Auto-numbering counter
    next_spec_number: u32,
}

impl SpecScaffolder {
    /// Create a new spec scaffolder
    pub fn new() -> Result<Self> {
        Ok(Self {
            generator: GeneratorBridge::new()?,
            spec_dir: PathBuf::from(".driven/specs"),
            next_spec_number: 1,
        })
    }

    /// Create with custom spec directory
    pub fn with_spec_dir(spec_dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            generator: GeneratorBridge::new()?,
            spec_dir: spec_dir.as_ref().to_path_buf(),
            next_spec_number: 1,
        })
    }

    /// Set the generator bridge
    pub fn with_generator(mut self, generator: GeneratorBridge) -> Self {
        self.generator = generator;
        self
    }

    /// Initialize the scaffolder by scanning existing specs
    pub fn initialize(&mut self) -> Result<()> {
        self.generator.initialize()?;

        // Scan for existing specs to determine next number
        if self.spec_dir.exists() {
            let mut max_num = 0u32;
            for entry in std::fs::read_dir(&self.spec_dir).map_err(DrivenError::Io)? {
                let entry = entry.map_err(DrivenError::Io)?;
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Try to parse leading number (e.g., "001-feature" -> 1)
                        if let Some(num_str) = name.split('-').next() {
                            if let Ok(num) = num_str.parse::<u32>() {
                                max_num = max_num.max(num);
                            }
                        }
                    }
                }
            }
            self.next_spec_number = max_num + 1;
        }

        Ok(())
    }

    /// Create a new spec with auto-numbering
    pub fn create_spec(
        &mut self,
        name: &str,
        params: &GenerateParams,
    ) -> Result<SpecScaffoldResult> {
        let spec_id = format!("{:03}-{}", self.next_spec_number, name);
        self.next_spec_number += 1;

        self.create_spec_with_id(&spec_id, params)
    }

    /// Create a spec with a specific ID
    pub fn create_spec_with_id(
        &self,
        spec_id: &str,
        params: &GenerateParams,
    ) -> Result<SpecScaffoldResult> {
        // Generate scaffold files
        let files = self.generator.generate_spec_scaffold(spec_id, params)?;

        // Write files to disk
        let written = self.generator.write_files(&files)?;
        let written_count = written.len();
        let files_skipped = files.len() - written_count;

        Ok(SpecScaffoldResult {
            spec_id: spec_id.to_string(),
            spec_dir: self.spec_dir.join(spec_id),
            files_created: written,
            files_skipped,
        })
    }

    /// Get the next spec number
    pub fn next_spec_number(&self) -> u32 {
        self.next_spec_number
    }

    /// Get the spec directory
    pub fn spec_dir(&self) -> &Path {
        &self.spec_dir
    }
}

impl Default for SpecScaffolder {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            generator: GeneratorBridge::default(),
            spec_dir: PathBuf::from(".driven/specs"),
            next_spec_number: 1,
        })
    }
}

/// Result of spec scaffolding
#[derive(Debug, Clone)]
pub struct SpecScaffoldResult {
    /// Spec ID
    pub spec_id: String,
    /// Spec directory path
    pub spec_dir: PathBuf,
    /// Files that were created
    pub files_created: Vec<PathBuf>,
    /// Number of files skipped (already existed)
    pub files_skipped: usize,
}

// ==================== Hook Messenger ====================

/// Hook messenger for sending hook messages via DCP
#[derive(Debug)]
pub struct HookMessenger {
    /// Message queue for pending messages
    message_queue: Vec<HookMessage>,
    /// Whether to use DCP binary protocol
    use_dcp: bool,
}

/// A hook message to be sent via DCP
#[derive(Debug, Clone)]
pub struct HookMessage {
    /// Hook ID that triggered this message
    pub hook_id: String,
    /// Message content
    pub content: String,
    /// Context data
    pub context: HashMap<String, String>,
    /// Priority (lower = higher priority)
    pub priority: u8,
    /// Timestamp when the message was created
    pub timestamp: u64,
}

impl HookMessenger {
    /// Create a new hook messenger
    pub fn new() -> Self {
        Self {
            message_queue: Vec::new(),
            use_dcp: true,
        }
    }

    /// Set whether to use DCP binary protocol
    pub fn with_dcp(mut self, use_dcp: bool) -> Self {
        self.use_dcp = use_dcp;
        self
    }

    /// Create a message from a hook and context
    pub fn create_message(&self, hook: &AgentHook, context: &HookContext) -> HookMessage {
        let mut ctx_map = HashMap::new();

        // Add context data
        if let Some(file) = &context.file_path {
            ctx_map.insert("file".to_string(), file.to_string_lossy().to_string());
        }
        if let Some(name) = &context.file_name {
            ctx_map.insert("file_name".to_string(), name.clone());
        }
        if let Some(ext) = &context.file_ext {
            ctx_map.insert("file_ext".to_string(), ext.clone());
        }

        // Add custom variables
        for (k, v) in &context.variables {
            ctx_map.insert(k.clone(), v.clone());
        }

        HookMessage {
            hook_id: hook.id.clone(),
            content: hook.action.message.clone(),
            context: ctx_map,
            priority: hook.priority,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Queue a message for sending
    pub fn queue_message(&mut self, message: HookMessage) {
        self.message_queue.push(message);
        // Sort by priority (lower = higher priority)
        self.message_queue.sort_by_key(|m| m.priority);
    }

    /// Send a hook message via DCP
    pub fn send_message(&self, _client: &DcpClient, message: &HookMessage) -> Result<DcpMessage> {
        if self.use_dcp {
            self.send_dcp_message(message)
        } else {
            self.send_mcp_message(message)
        }
    }

    /// Send message using DCP binary protocol
    fn send_dcp_message(&self, message: &HookMessage) -> Result<DcpMessage> {
        // Serialize message to binary format
        let payload = self.serialize_message(message)?;

        // Create DCP message
        Ok(DcpMessage::new(MessageType::Prompt, 0, payload))
    }

    /// Send message using MCP JSON-RPC protocol
    fn send_mcp_message(&self, message: &HookMessage) -> Result<DcpMessage> {
        // Create JSON payload
        let json = serde_json::json!({
            "hook_id": message.hook_id,
            "content": message.content,
            "context": message.context,
            "priority": message.priority,
            "timestamp": message.timestamp,
        });

        let payload = serde_json::to_vec(&json)
            .map_err(|e| DrivenError::Format(format!("Failed to serialize: {}", e)))?;

        Ok(DcpMessage::new(MessageType::Prompt, 0, payload))
    }

    /// Serialize a message to binary format
    fn serialize_message(&self, message: &HookMessage) -> Result<Vec<u8>> {
        // Simple binary format:
        // - 4 bytes: hook_id length
        // - N bytes: hook_id
        // - 4 bytes: content length
        // - N bytes: content
        // - 4 bytes: context count
        // - For each context entry:
        //   - 4 bytes: key length
        //   - N bytes: key
        //   - 4 bytes: value length
        //   - N bytes: value
        // - 1 byte: priority
        // - 8 bytes: timestamp

        let mut buf = Vec::new();

        // Hook ID
        buf.extend_from_slice(&(message.hook_id.len() as u32).to_le_bytes());
        buf.extend_from_slice(message.hook_id.as_bytes());

        // Content
        buf.extend_from_slice(&(message.content.len() as u32).to_le_bytes());
        buf.extend_from_slice(message.content.as_bytes());

        // Context
        buf.extend_from_slice(&(message.context.len() as u32).to_le_bytes());
        for (k, v) in &message.context {
            buf.extend_from_slice(&(k.len() as u32).to_le_bytes());
            buf.extend_from_slice(k.as_bytes());
            buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
            buf.extend_from_slice(v.as_bytes());
        }

        // Priority and timestamp
        buf.push(message.priority);
        buf.extend_from_slice(&message.timestamp.to_le_bytes());

        Ok(buf)
    }

    /// Deserialize a message from binary format
    pub fn deserialize_message(&self, data: &[u8]) -> Result<HookMessage> {
        let mut offset = 0;

        // Helper to read u32
        let read_u32 = |data: &[u8], offset: &mut usize| -> Result<u32> {
            if *offset + 4 > data.len() {
                return Err(DrivenError::InvalidBinary("Insufficient data".to_string()));
            }
            let bytes: [u8; 4] = data[*offset..*offset + 4].try_into().unwrap();
            *offset += 4;
            Ok(u32::from_le_bytes(bytes))
        };

        // Helper to read string
        let read_string = |data: &[u8], offset: &mut usize| -> Result<String> {
            let len = read_u32(data, offset)? as usize;
            if *offset + len > data.len() {
                return Err(DrivenError::InvalidBinary("Insufficient data".to_string()));
            }
            let s = std::str::from_utf8(&data[*offset..*offset + len])
                .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8: {}", e)))?
                .to_string();
            *offset += len;
            Ok(s)
        };

        // Read hook_id
        let hook_id = read_string(data, &mut offset)?;

        // Read content
        let content = read_string(data, &mut offset)?;

        // Read context
        let ctx_count = read_u32(data, &mut offset)? as usize;
        let mut context = HashMap::new();
        for _ in 0..ctx_count {
            let key = read_string(data, &mut offset)?;
            let value = read_string(data, &mut offset)?;
            context.insert(key, value);
        }

        // Read priority
        if offset >= data.len() {
            return Err(DrivenError::InvalidBinary("Insufficient data for priority".to_string()));
        }
        let priority = data[offset];
        offset += 1;

        // Read timestamp
        if offset + 8 > data.len() {
            return Err(DrivenError::InvalidBinary("Insufficient data for timestamp".to_string()));
        }
        let timestamp_bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
        let timestamp = u64::from_le_bytes(timestamp_bytes);

        Ok(HookMessage {
            hook_id,
            content,
            context,
            priority,
            timestamp,
        })
    }

    /// Flush all queued messages
    pub fn flush(&mut self, client: &DcpClient) -> Result<Vec<DcpMessage>> {
        let messages: Vec<_> = self.message_queue.drain(..).collect();
        let mut results = Vec::new();

        for message in messages {
            results.push(self.send_message(client, &message)?);
        }

        Ok(results)
    }

    /// Get the number of queued messages
    pub fn queue_len(&self) -> usize {
        self.message_queue.len()
    }

    /// Clear the message queue
    pub fn clear_queue(&mut self) {
        self.message_queue.clear();
    }
}

impl Default for HookMessenger {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== Binary Format Compatibility ====================

/// Binary format compatibility checker for DX ∞ format
///
/// Ensures that binary data can be correctly serialized and deserialized
/// across different crates (Driven, Generator, DCP).
#[derive(Debug)]
pub struct BinaryFormatChecker {
    /// Supported format versions
    supported_versions: Vec<u16>,
}

/// Binary format header for cross-crate compatibility
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CrossCrateHeader {
    /// Magic bytes: "DX∞C" (DX Infinity Cross-crate)
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
    /// Source crate identifier
    pub source_crate: u8,
    /// Flags
    pub flags: u8,
    /// Payload length
    pub payload_len: u32,
    /// Checksum (Blake3, first 8 bytes)
    pub checksum: [u8; 8],
}

/// Source crate identifiers
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceCrate {
    /// dx-driven crate
    Driven = 1,
    /// dx-generator crate
    Generator = 2,
    /// dx-dcp crate
    Dcp = 3,
    /// Unknown source
    Unknown = 0,
}

impl CrossCrateHeader {
    /// Magic bytes for cross-crate format
    pub const MAGIC: [u8; 4] = [0x44, 0x58, 0xE2, 0x43]; // "DX∞C" (with infinity symbol byte)

    /// Header size in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Current format version
    pub const CURRENT_VERSION: u16 = 1;

    /// Create a new header
    pub fn new(source: SourceCrate, payload_len: u32) -> Self {
        Self {
            magic: Self::MAGIC,
            version: Self::CURRENT_VERSION,
            source_crate: source as u8,
            flags: 0,
            payload_len,
            checksum: [0; 8],
        }
    }

    /// Create header with checksum
    pub fn with_checksum(source: SourceCrate, payload: &[u8]) -> Self {
        let mut header = Self::new(source, payload.len() as u32);
        header.checksum = Self::compute_checksum(payload);
        header
    }

    /// Compute checksum for payload
    pub fn compute_checksum(payload: &[u8]) -> [u8; 8] {
        let hash = blake3::hash(payload);
        let mut checksum = [0u8; 8];
        checksum.copy_from_slice(&hash.as_bytes()[..8]);
        checksum
    }

    /// Verify the checksum
    pub fn verify_checksum(&self, payload: &[u8]) -> bool {
        let expected = Self::compute_checksum(payload);
        self.checksum == expected
    }

    /// Check if magic bytes are valid
    pub fn is_valid_magic(&self) -> bool {
        self.magic == Self::MAGIC
    }

    /// Get the source crate
    pub fn source(&self) -> SourceCrate {
        match self.source_crate {
            1 => SourceCrate::Driven,
            2 => SourceCrate::Generator,
            3 => SourceCrate::Dcp,
            _ => SourceCrate::Unknown,
        }
    }

    /// Convert to bytes
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self> {
        if bytes.len() < Self::SIZE {
            return Err(DrivenError::InvalidBinary(format!(
                "Insufficient data for header: {} < {}",
                bytes.len(),
                Self::SIZE
            )));
        }

        let header = unsafe { &*(bytes.as_ptr() as *const Self) };

        if !header.is_valid_magic() {
            return Err(DrivenError::InvalidBinary("Invalid magic bytes".to_string()));
        }

        Ok(header)
    }
}

impl BinaryFormatChecker {
    /// Create a new binary format checker
    pub fn new() -> Self {
        Self {
            supported_versions: vec![1],
        }
    }

    /// Check if a version is supported
    pub fn is_version_supported(&self, version: u16) -> bool {
        self.supported_versions.contains(&version)
    }

    /// Validate a cross-crate binary blob
    pub fn validate(&self, data: &[u8]) -> Result<ValidationResult> {
        // Parse header
        let header = CrossCrateHeader::from_bytes(data)?;

        // Check version
        if !self.is_version_supported(header.version) {
            return Ok(ValidationResult {
                valid: false,
                source: header.source(),
                version: header.version,
                payload_len: header.payload_len,
                checksum_valid: false,
                error: Some(format!("Unsupported version: {}", header.version)),
            });
        }

        // Check payload length
        let expected_len = CrossCrateHeader::SIZE + header.payload_len as usize;
        if data.len() < expected_len {
            return Ok(ValidationResult {
                valid: false,
                source: header.source(),
                version: header.version,
                payload_len: header.payload_len,
                checksum_valid: false,
                error: Some(format!(
                    "Insufficient data: expected {}, got {}",
                    expected_len,
                    data.len()
                )),
            });
        }

        // Verify checksum
        let payload = &data[CrossCrateHeader::SIZE..expected_len];
        let checksum_valid = header.verify_checksum(payload);

        Ok(ValidationResult {
            valid: checksum_valid,
            source: header.source(),
            version: header.version,
            payload_len: header.payload_len,
            checksum_valid,
            error: if checksum_valid {
                None
            } else {
                Some("Checksum mismatch".to_string())
            },
        })
    }

    /// Wrap payload with cross-crate header
    pub fn wrap(&self, source: SourceCrate, payload: &[u8]) -> Vec<u8> {
        let header = CrossCrateHeader::with_checksum(source, payload);
        let mut result = Vec::with_capacity(CrossCrateHeader::SIZE + payload.len());
        result.extend_from_slice(header.as_bytes());
        result.extend_from_slice(payload);
        result
    }

    /// Unwrap payload from cross-crate format
    pub fn unwrap(&self, data: &[u8]) -> Result<(SourceCrate, Vec<u8>)> {
        let result = self.validate(data)?;

        if !result.valid {
            return Err(DrivenError::InvalidBinary(
                result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let payload = data
            [CrossCrateHeader::SIZE..CrossCrateHeader::SIZE + result.payload_len as usize]
            .to_vec();
        Ok((result.source, payload))
    }
}

impl Default for BinaryFormatChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of binary format validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the data is valid
    pub valid: bool,
    /// Source crate
    pub source: SourceCrate,
    /// Format version
    pub version: u16,
    /// Payload length
    pub payload_len: u32,
    /// Whether checksum is valid
    pub checksum_valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
}

// ==================== Integration Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== DrivenDcpBridge Tests ====================

    #[test]
    fn test_driven_dcp_bridge_creation() {
        let bridge = DrivenDcpBridge::new();
        assert!(!bridge.initialized);
        assert!(bridge.tool_ids.is_empty());
    }

    #[test]
    fn test_driven_tools_list() {
        let bridge = DrivenDcpBridge::new();
        let tools = bridge.get_driven_tools();

        // Should have tools for each category
        assert!(tools.iter().any(|t| t.category == DrivenToolCategory::Rules));
        assert!(tools.iter().any(|t| t.category == DrivenToolCategory::Specs));
        assert!(tools.iter().any(|t| t.category == DrivenToolCategory::Hooks));
        assert!(tools.iter().any(|t| t.category == DrivenToolCategory::Steering));
        assert!(tools.iter().any(|t| t.category == DrivenToolCategory::Templates));
    }

    #[test]
    fn test_schema_hash_determinism() {
        let bridge = DrivenDcpBridge::new();
        let tool = DrivenTool {
            name: "test.tool".to_string(),
            description: "A test tool".to_string(),
            category: DrivenToolCategory::Rules,
            capabilities: 0x1234,
        };

        let hash1 = bridge.compute_schema_hash(&tool);
        let hash2 = bridge.compute_schema_hash(&tool);

        assert_eq!(hash1, hash2);
    }

    // ==================== SpecScaffolder Tests ====================

    #[test]
    fn test_spec_scaffolder_creation() {
        let scaffolder = SpecScaffolder::new().unwrap();
        assert_eq!(scaffolder.next_spec_number(), 1);
    }

    #[test]
    fn test_spec_scaffolder_custom_dir() {
        let scaffolder = SpecScaffolder::with_spec_dir("custom/specs").unwrap();
        assert_eq!(scaffolder.spec_dir(), Path::new("custom/specs"));
    }

    // ==================== HookMessenger Tests ====================

    #[test]
    fn test_hook_messenger_creation() {
        let messenger = HookMessenger::new();
        assert!(messenger.use_dcp);
        assert_eq!(messenger.queue_len(), 0);
    }

    #[test]
    fn test_hook_message_serialization_roundtrip() {
        let messenger = HookMessenger::new();

        let mut context = HashMap::new();
        context.insert("file".to_string(), "test.rs".to_string());
        context.insert("branch".to_string(), "main".to_string());

        let message = HookMessage {
            hook_id: "test-hook".to_string(),
            content: "Test message content".to_string(),
            context,
            priority: 50,
            timestamp: 1234567890,
        };

        let serialized = messenger.serialize_message(&message).unwrap();
        let deserialized = messenger.deserialize_message(&serialized).unwrap();

        assert_eq!(deserialized.hook_id, message.hook_id);
        assert_eq!(deserialized.content, message.content);
        assert_eq!(deserialized.priority, message.priority);
        assert_eq!(deserialized.timestamp, message.timestamp);
        assert_eq!(deserialized.context.len(), message.context.len());
    }

    #[test]
    fn test_hook_message_queue() {
        let mut messenger = HookMessenger::new();

        // Queue messages with different priorities
        messenger.queue_message(HookMessage {
            hook_id: "low".to_string(),
            content: "Low priority".to_string(),
            context: HashMap::new(),
            priority: 100,
            timestamp: 0,
        });

        messenger.queue_message(HookMessage {
            hook_id: "high".to_string(),
            content: "High priority".to_string(),
            context: HashMap::new(),
            priority: 10,
            timestamp: 0,
        });

        messenger.queue_message(HookMessage {
            hook_id: "medium".to_string(),
            content: "Medium priority".to_string(),
            context: HashMap::new(),
            priority: 50,
            timestamp: 0,
        });

        assert_eq!(messenger.queue_len(), 3);

        // Messages should be sorted by priority
        assert_eq!(messenger.message_queue[0].hook_id, "high");
        assert_eq!(messenger.message_queue[1].hook_id, "medium");
        assert_eq!(messenger.message_queue[2].hook_id, "low");
    }

    // ==================== BinaryFormatChecker Tests ====================

    #[test]
    fn test_cross_crate_header_creation() {
        let header = CrossCrateHeader::new(SourceCrate::Driven, 100);

        assert!(header.is_valid_magic());
        assert_eq!(header.version, CrossCrateHeader::CURRENT_VERSION);
        assert_eq!(header.source(), SourceCrate::Driven);
        assert_eq!(header.payload_len, 100);
    }

    #[test]
    fn test_cross_crate_header_checksum() {
        let payload = b"test payload data";
        let header = CrossCrateHeader::with_checksum(SourceCrate::Generator, payload);

        assert!(header.verify_checksum(payload));
        assert!(!header.verify_checksum(b"different data"));
    }

    #[test]
    fn test_binary_format_wrap_unwrap() {
        let checker = BinaryFormatChecker::new();
        let payload = b"test payload for cross-crate transfer";

        let wrapped = checker.wrap(SourceCrate::Dcp, payload);
        let (source, unwrapped) = checker.unwrap(&wrapped).unwrap();

        assert_eq!(source, SourceCrate::Dcp);
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_binary_format_validation() {
        let checker = BinaryFormatChecker::new();
        let payload = b"validation test payload";

        let wrapped = checker.wrap(SourceCrate::Driven, payload);
        let result = checker.validate(&wrapped).unwrap();

        assert!(result.valid);
        assert!(result.checksum_valid);
        assert_eq!(result.source, SourceCrate::Driven);
        assert_eq!(result.payload_len, payload.len() as u32);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_binary_format_invalid_magic() {
        let checker = BinaryFormatChecker::new();
        let invalid_data = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00];

        let result = checker.validate(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_binary_format_corrupted_checksum() {
        let checker = BinaryFormatChecker::new();
        let payload = b"test payload";

        let mut wrapped = checker.wrap(SourceCrate::Driven, payload);
        // Corrupt the payload
        if wrapped.len() > CrossCrateHeader::SIZE {
            wrapped[CrossCrateHeader::SIZE] ^= 0xFF;
        }

        let result = checker.validate(&wrapped).unwrap();
        assert!(!result.valid);
        assert!(!result.checksum_valid);
    }
}

// ==================== Property-Based Tests ====================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Property 13: Cross-Crate Integration
    // For any operation that spans multiple crates, the operation SHALL complete
    // successfully with correct data passing between crates.

    proptest! {
        /// Property: Hook message serialization is lossless
        #[test]
        fn prop_hook_message_roundtrip(
            hook_id in "[a-z][a-z0-9-]{0,30}",
            content in ".{0,1000}",
            priority in 0u8..=255u8,
            timestamp in 0u64..=u64::MAX,
        ) {
            let messenger = HookMessenger::new();

            let message = HookMessage {
                hook_id: hook_id.clone(),
                content: content.clone(),
                context: HashMap::new(),
                priority,
                timestamp,
            };

            let serialized = messenger.serialize_message(&message).unwrap();
            let deserialized = messenger.deserialize_message(&serialized).unwrap();

            prop_assert_eq!(deserialized.hook_id, hook_id);
            prop_assert_eq!(deserialized.content, content);
            prop_assert_eq!(deserialized.priority, priority);
            prop_assert_eq!(deserialized.timestamp, timestamp);
        }

        /// Property: Binary format wrap/unwrap is lossless
        #[test]
        fn prop_binary_format_roundtrip(
            payload in prop::collection::vec(any::<u8>(), 0..10000),
            source in prop_oneof![
                Just(SourceCrate::Driven),
                Just(SourceCrate::Generator),
                Just(SourceCrate::Dcp),
            ],
        ) {
            let checker = BinaryFormatChecker::new();

            let wrapped = checker.wrap(source, &payload);
            let (unwrapped_source, unwrapped_payload) = checker.unwrap(&wrapped).unwrap();

            prop_assert_eq!(unwrapped_source, source);
            prop_assert_eq!(unwrapped_payload, payload);
        }

        /// Property: Cross-crate header checksum is deterministic
        #[test]
        fn prop_checksum_deterministic(
            payload in prop::collection::vec(any::<u8>(), 0..10000),
        ) {
            let checksum1 = CrossCrateHeader::compute_checksum(&payload);
            let checksum2 = CrossCrateHeader::compute_checksum(&payload);

            prop_assert_eq!(checksum1, checksum2);
        }

        /// Property: Schema hash is deterministic for same tool
        #[test]
        fn prop_schema_hash_deterministic(
            name in "[a-z][a-z0-9.]{0,50}",
            description in ".{0,200}",
            capabilities in 0u64..=u64::MAX,
        ) {
            let bridge = DrivenDcpBridge::new();

            let tool = DrivenTool {
                name: name.clone(),
                description: description.clone(),
                category: DrivenToolCategory::Rules,
                capabilities,
            };

            let hash1 = bridge.compute_schema_hash(&tool);
            let hash2 = bridge.compute_schema_hash(&tool);

            prop_assert_eq!(hash1, hash2);
        }

        /// Property: Message queue maintains priority ordering
        #[test]
        fn prop_message_queue_priority_order(
            priorities in prop::collection::vec(0u8..=255u8, 1..20),
        ) {
            let mut messenger = HookMessenger::new();

            for (i, priority) in priorities.iter().enumerate() {
                messenger.queue_message(HookMessage {
                    hook_id: format!("hook-{}", i),
                    content: String::new(),
                    context: HashMap::new(),
                    priority: *priority,
                    timestamp: 0,
                });
            }

            // Verify messages are sorted by priority
            for i in 1..messenger.message_queue.len() {
                prop_assert!(
                    messenger.message_queue[i - 1].priority <= messenger.message_queue[i].priority,
                    "Messages not sorted by priority"
                );
            }
        }

        /// Property: Binary format validation detects corruption
        #[test]
        fn prop_corruption_detection(
            payload in prop::collection::vec(any::<u8>(), 1..1000),
            corrupt_offset in 0usize..1000usize,
        ) {
            let checker = BinaryFormatChecker::new();
            let mut wrapped = checker.wrap(SourceCrate::Driven, &payload);

            // Only corrupt if offset is within payload area
            let payload_start = CrossCrateHeader::SIZE;
            if corrupt_offset < payload.len() {
                let actual_offset = payload_start + corrupt_offset;
                if actual_offset < wrapped.len() {
                    wrapped[actual_offset] ^= 0xFF;

                    let result = checker.validate(&wrapped).unwrap();
                    prop_assert!(!result.checksum_valid, "Corruption not detected");
                }
            }
        }
    }
}
