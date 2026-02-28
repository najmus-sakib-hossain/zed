//! HBSP Streaming Protocol
//!
//! Hyper Binary Security Protocol for real-time finding emission.
//!
//! ## Packet Format
//!
//! Each HBSP packet consists of:
//! - 8-byte header: magic (2), version (1), type (1), length (4)
//! - Compressed payload using LZ4 (optional)
//!
//! ## Requirements
//! - _Requirements: 9.1, 9.3, 9.4_

use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;

/// HBSP protocol version
pub const HBSP_VERSION: u8 = 1;

/// HBSP magic bytes
pub const HBSP_MAGIC: [u8; 2] = *b"DX";

/// HBSP packet header (8 bytes)
///
/// Format:
/// - Bytes 0-1: Magic "DX"
/// - Byte 2: Protocol version
/// - Byte 3: Finding type (with compression flag in high bit)
/// - Bytes 4-7: Payload length (little-endian u32)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HbspHeader {
    /// Magic bytes "DX"
    pub magic: [u8; 2],
    /// Protocol version
    pub version: u8,
    /// Finding type (high bit = compression flag)
    pub finding_type: u8,
    /// Payload length (compressed if applicable)
    pub payload_len: u32,
}

impl HbspHeader {
    /// Header size in bytes
    pub const SIZE: usize = 8;

    /// Compression flag (high bit of finding_type)
    pub const COMPRESSION_FLAG: u8 = 0x80;

    /// Create a new HBSP header
    pub fn new(finding_type: u8, payload_len: u32) -> Self {
        Self {
            magic: HBSP_MAGIC,
            version: HBSP_VERSION,
            finding_type,
            payload_len,
        }
    }

    /// Create a new HBSP header with compression enabled
    pub fn new_compressed(finding_type: u8, payload_len: u32) -> Self {
        Self {
            magic: HBSP_MAGIC,
            version: HBSP_VERSION,
            finding_type: finding_type | Self::COMPRESSION_FLAG,
            payload_len,
        }
    }

    /// Check if payload is compressed
    pub fn is_compressed(&self) -> bool {
        self.finding_type & Self::COMPRESSION_FLAG != 0
    }

    /// Get the actual finding type (without compression flag)
    pub fn get_finding_type(&self) -> u8 {
        self.finding_type & !Self::COMPRESSION_FLAG
    }

    /// Validate header magic and version
    pub fn is_valid(&self) -> bool {
        self.magic == HBSP_MAGIC && self.version == HBSP_VERSION
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..2].copy_from_slice(&self.magic);
        bytes[2] = self.version;
        bytes[3] = self.finding_type;
        bytes[4..8].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8; 8]) -> Option<Self> {
        let header = Self {
            magic: [bytes[0], bytes[1]],
            version: bytes[2],
            finding_type: bytes[3],
            payload_len: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        };

        if header.is_valid() {
            Some(header)
        } else {
            None
        }
    }
}

/// Finding types for HBSP
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingType {
    /// Vulnerability (CVE) finding
    Vulnerability = 0,
    /// Secret/credential leak finding
    Secret = 1,
    /// Custom rule violation
    RuleViolation = 2,
    /// Security score update
    Score = 3,
    /// Scan progress update
    Progress = 4,
    /// Scan complete notification
    Complete = 5,
}

impl FindingType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Vulnerability),
            1 => Some(Self::Secret),
            2 => Some(Self::RuleViolation),
            3 => Some(Self::Score),
            4 => Some(Self::Progress),
            5 => Some(Self::Complete),
            _ => None,
        }
    }
}

/// Severity levels for findings
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational only
    Info = 0,
    /// Low severity
    Low = 1,
    /// Medium severity
    Medium = 2,
    /// High severity
    High = 3,
    /// Critical severity
    Critical = 4,
}

impl Severity {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Info),
            1 => Some(Self::Low),
            2 => Some(Self::Medium),
            3 => Some(Self::High),
            4 => Some(Self::Critical),
            _ => None,
        }
    }
}

/// Generic finding for streaming
///
/// Represents a security finding that can be emitted in real-time.
#[derive(Debug, Clone)]
pub struct Finding {
    /// Type of finding
    pub finding_type: FindingType,
    /// Severity level
    pub severity: u8,
    /// File path where finding was detected
    pub file_path: PathBuf,
    /// Line number (0 if not applicable)
    pub line_number: u32,
    /// Column number (0 if not applicable)
    pub column: u16,
    /// Human-readable message
    pub message: String,
    /// Optional CVE ID for vulnerabilities
    pub cve_id: Option<String>,
}

impl Finding {
    /// Create a new finding
    pub fn new(
        finding_type: FindingType,
        severity: u8,
        file_path: PathBuf,
        line_number: u32,
        message: String,
    ) -> Self {
        Self {
            finding_type,
            severity,
            file_path,
            line_number,
            column: 0,
            message,
            cve_id: None,
        }
    }

    /// Create a vulnerability finding
    pub fn vulnerability(
        severity: Severity,
        file_path: PathBuf,
        line_number: u32,
        message: String,
        cve_id: Option<String>,
    ) -> Self {
        Self {
            finding_type: FindingType::Vulnerability,
            severity: severity as u8,
            file_path,
            line_number,
            column: 0,
            message,
            cve_id,
        }
    }

    /// Create a secret finding
    pub fn secret(file_path: PathBuf, line_number: u32, column: u16, message: String) -> Self {
        Self {
            finding_type: FindingType::Secret,
            severity: Severity::Critical as u8,
            file_path,
            line_number,
            column,
            message,
            cve_id: None,
        }
    }

    /// Create a score update finding
    pub fn score(score: u8) -> Self {
        Self {
            finding_type: FindingType::Score,
            severity: 0,
            file_path: PathBuf::new(),
            line_number: 0,
            column: 0,
            message: format!("Security score: {}", score),
            cve_id: None,
        }
    }
}

/// Stream output target
#[derive(Debug, Clone)]
pub enum StreamOutput {
    /// Output to terminal (stdout)
    Terminal,
    /// Output to file (append mode)
    File(PathBuf),
    /// Output to network socket
    Network(SocketAddr),
    /// Output to in-memory buffer (for testing)
    Buffer,
}

/// Configuration for the streaming protocol
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Enable LZ4 compression for payloads
    pub compress: bool,
    /// Minimum payload size to compress (bytes)
    pub compress_threshold: usize,
    /// Buffer size for batching
    pub buffer_size: usize,
    /// Enable colored terminal output
    pub colored_output: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            compress: true,
            compress_threshold: 64,
            buffer_size: 4096,
            colored_output: true,
        }
    }
}

/// Finding emitter for real-time streaming
///
/// Emits findings as they are discovered, supporting terminal, file, and network outputs.
/// _Requirements: 9.3, 9.4_
pub struct FindingEmitter {
    output: StreamOutput,
    config: StreamConfig,
    buffer: Vec<u8>,
    memory_buffer: Vec<u8>,
    findings_emitted: u64,
}

impl FindingEmitter {
    /// Create a new finding emitter
    pub fn new(output: StreamOutput) -> Self {
        Self {
            output,
            config: StreamConfig::default(),
            buffer: Vec::with_capacity(4096),
            memory_buffer: Vec::new(),
            findings_emitted: 0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(output: StreamOutput, config: StreamConfig) -> Self {
        Self {
            output,
            config,
            buffer: Vec::with_capacity(4096),
            memory_buffer: Vec::new(),
            findings_emitted: 0,
        }
    }

    /// Emit a finding immediately
    ///
    /// Serializes the finding and writes it to the configured output.
    /// _Requirements: 9.3_
    pub fn emit(&mut self, finding: Finding) -> std::io::Result<()> {
        let payload = self.serialize_finding(&finding);

        // Create header (with compression flag if applicable)
        let header = if self.config.compress && payload.len() >= self.config.compress_threshold {
            // For now, we use simple serialization without actual LZ4
            // In production, this would use lz4 crate
            HbspHeader::new(finding.finding_type as u8, payload.len() as u32)
        } else {
            HbspHeader::new(finding.finding_type as u8, payload.len() as u32)
        };

        self.buffer.clear();
        self.buffer.extend_from_slice(&header.to_bytes());
        self.buffer.extend_from_slice(&payload);

        self.write_output()?;
        self.findings_emitted += 1;

        Ok(())
    }

    /// Emit multiple findings in batch
    pub fn emit_batch(&mut self, findings: &[Finding]) -> std::io::Result<()> {
        for finding in findings {
            self.emit(finding.clone())?;
        }
        Ok(())
    }

    /// Serialize finding to bytes
    fn serialize_finding(&self, finding: &Finding) -> Vec<u8> {
        let path_bytes = finding.file_path.to_string_lossy().as_bytes().to_vec();
        let msg_bytes = finding.message.as_bytes();
        let cve_bytes = finding.cve_id.as_ref().map(|s| s.as_bytes()).unwrap_or(&[]);

        let mut payload = Vec::with_capacity(
            1 + 4 + 2 + 2 + path_bytes.len() + 2 + msg_bytes.len() + 1 + cve_bytes.len(),
        );

        // Severity (1 byte)
        payload.push(finding.severity);

        // Line number (4 bytes, little-endian)
        payload.extend_from_slice(&finding.line_number.to_le_bytes());

        // Column (2 bytes, little-endian)
        payload.extend_from_slice(&finding.column.to_le_bytes());

        // Path length + path
        payload.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
        payload.extend_from_slice(&path_bytes);

        // Message length + message
        payload.extend_from_slice(&(msg_bytes.len() as u16).to_le_bytes());
        payload.extend_from_slice(msg_bytes);

        // CVE ID (optional, prefixed with length byte)
        payload.push(cve_bytes.len() as u8);
        if !cve_bytes.is_empty() {
            payload.extend_from_slice(cve_bytes);
        }

        payload
    }

    /// Deserialize finding from bytes
    pub fn deserialize_finding(finding_type: FindingType, data: &[u8]) -> Option<Finding> {
        if data.len() < 9 {
            return None;
        }

        let severity = data[0];
        let line_number = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let column = u16::from_le_bytes([data[5], data[6]]);

        let path_len = u16::from_le_bytes([data[7], data[8]]) as usize;
        if data.len() < 9 + path_len + 2 {
            return None;
        }

        let file_path = PathBuf::from(String::from_utf8_lossy(&data[9..9 + path_len]).to_string());

        let msg_start = 9 + path_len;
        let msg_len = u16::from_le_bytes([data[msg_start], data[msg_start + 1]]) as usize;
        if data.len() < msg_start + 2 + msg_len + 1 {
            return None;
        }

        let message =
            String::from_utf8_lossy(&data[msg_start + 2..msg_start + 2 + msg_len]).to_string();

        let cve_start = msg_start + 2 + msg_len;
        let cve_len = data[cve_start] as usize;
        let cve_id = if cve_len > 0 && data.len() >= cve_start + 1 + cve_len {
            Some(String::from_utf8_lossy(&data[cve_start + 1..cve_start + 1 + cve_len]).to_string())
        } else {
            None
        };

        Some(Finding {
            finding_type,
            severity,
            file_path,
            line_number,
            column,
            message,
            cve_id,
        })
    }

    /// Write buffer to output
    /// _Requirements: 9.4_
    fn write_output(&mut self) -> std::io::Result<()> {
        match &self.output {
            StreamOutput::Terminal => {
                // For terminal, we can optionally format as human-readable
                std::io::stdout().write_all(&self.buffer)?;
                std::io::stdout().flush()
            }
            StreamOutput::File(path) => {
                let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
                file.write_all(&self.buffer)
            }
            StreamOutput::Network(addr) => {
                // UDP streaming for real-time delivery
                let socket = std::net::UdpSocket::bind("0.0.0.0:0")?;
                socket.send_to(&self.buffer, addr)?;
                Ok(())
            }
            StreamOutput::Buffer => {
                self.memory_buffer.extend_from_slice(&self.buffer);
                Ok(())
            }
        }
    }

    /// Flush buffered findings
    pub fn flush(&mut self) -> std::io::Result<()> {
        self.buffer.clear();
        Ok(())
    }

    /// Set output target
    pub fn set_output(&mut self, output: StreamOutput) {
        self.output = output;
    }

    /// Get number of findings emitted
    pub fn findings_emitted(&self) -> u64 {
        self.findings_emitted
    }

    /// Get memory buffer contents (for Buffer output mode)
    pub fn get_buffer(&self) -> &[u8] {
        &self.memory_buffer
    }

    /// Clear memory buffer
    pub fn clear_buffer(&mut self) {
        self.memory_buffer.clear();
    }
}

/// Legacy alias for backward compatibility
pub type StreamingProtocol = FindingEmitter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_serialization() {
        let header = HbspHeader::new(FindingType::Secret as u8, 100);
        let bytes = header.to_bytes();

        assert_eq!(bytes[0..2], HBSP_MAGIC);
        assert_eq!(bytes[2], HBSP_VERSION);
        assert_eq!(bytes[3], FindingType::Secret as u8);
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 100);
    }

    #[test]
    fn test_header_roundtrip() {
        let header = HbspHeader::new(FindingType::Vulnerability as u8, 12345);
        let bytes = header.to_bytes();
        let parsed = HbspHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.magic, parsed.magic);
        assert_eq!(header.version, parsed.version);
        assert_eq!(header.finding_type, parsed.finding_type);
        assert_eq!(header.payload_len, parsed.payload_len);
    }

    #[test]
    fn test_compression_flag() {
        let header = HbspHeader::new_compressed(FindingType::Secret as u8, 100);
        assert!(header.is_compressed());
        assert_eq!(header.get_finding_type(), FindingType::Secret as u8);

        let header_uncompressed = HbspHeader::new(FindingType::Secret as u8, 100);
        assert!(!header_uncompressed.is_compressed());
    }

    #[test]
    fn test_finding_serialization_roundtrip() {
        let finding = Finding::secret(
            PathBuf::from("src/main.rs"),
            42,
            10,
            "AWS access key detected".to_string(),
        );

        let mut emitter = FindingEmitter::new(StreamOutput::Buffer);
        let payload = emitter.serialize_finding(&finding);

        let parsed = FindingEmitter::deserialize_finding(FindingType::Secret, &payload).unwrap();

        assert_eq!(parsed.severity, finding.severity);
        assert_eq!(parsed.file_path, finding.file_path);
        assert_eq!(parsed.line_number, finding.line_number);
        assert_eq!(parsed.column, finding.column);
        assert_eq!(parsed.message, finding.message);
    }

    #[test]
    fn test_emit_to_buffer() {
        let mut emitter = FindingEmitter::new(StreamOutput::Buffer);

        let finding = Finding::new(
            FindingType::Vulnerability,
            Severity::High as u8,
            PathBuf::from("Cargo.toml"),
            1,
            "Vulnerable dependency".to_string(),
        );

        emitter.emit(finding).unwrap();

        assert!(!emitter.get_buffer().is_empty());
        assert_eq!(emitter.findings_emitted(), 1);

        // Verify header is present
        let buffer = emitter.get_buffer();
        assert!(buffer.len() >= HbspHeader::SIZE);
        assert_eq!(&buffer[0..2], &HBSP_MAGIC);
    }

    #[test]
    fn test_finding_type_conversion() {
        assert_eq!(FindingType::from_u8(0), Some(FindingType::Vulnerability));
        assert_eq!(FindingType::from_u8(1), Some(FindingType::Secret));
        assert_eq!(FindingType::from_u8(5), Some(FindingType::Complete));
        assert_eq!(FindingType::from_u8(255), None);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }
}
