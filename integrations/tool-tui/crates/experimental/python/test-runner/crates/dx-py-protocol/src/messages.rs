//! Binary message definitions for test protocol

use dx_py_core::{AssertionStats, ProtocolError, TestCase, TestId, TestResult, TestStatus};
use std::time::Duration;

/// Magic bytes for protocol messages
pub const PROTOCOL_MAGIC: u32 = 0xDEADBEEF;

/// Maximum payload size (1MB)
pub const MAX_PAYLOAD_SIZE: usize = 1024 * 1024;

/// Message types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Run = 1,
    Result = 2,
    Skip = 3,
    MsgError = 4,
}

impl TryFrom<u8> for MessageType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(MessageType::Run),
            2 => Ok(MessageType::Result),
            3 => Ok(MessageType::Skip),
            4 => Ok(MessageType::MsgError),
            _ => Err(ProtocolError::InvalidMessageType(value)),
        }
    }
}

/// Message flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MessageFlags(pub u8);

impl MessageFlags {
    pub const ASYNC: u8 = 1;
    pub const PARAMETERIZED: u8 = 2;
    pub const FIXTURE: u8 = 4;

    pub fn is_async(&self) -> bool {
        self.0 & Self::ASYNC != 0
    }

    pub fn is_parameterized(&self) -> bool {
        self.0 & Self::PARAMETERIZED != 0
    }

    pub fn has_fixture(&self) -> bool {
        self.0 & Self::FIXTURE != 0
    }
}

/// Test execution request (32-byte header)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TestMessageHeader {
    pub magic: u32,         // 0xDEADBEEF (4 bytes)
    pub msg_type: u8,       // RUN=1, RESULT=2, SKIP=3, ERROR=4 (1 byte)
    pub flags: u8,          // ASYNC=1, PARAMETERIZED=2, FIXTURE=4 (1 byte)
    pub test_id: u16,       // (2 bytes)
    pub file_hash: u64,     // (8 bytes)
    pub payload_len: u32,   // (4 bytes)
    pub reserved: [u8; 12], // (12 bytes) - total = 32 bytes
}

impl TestMessageHeader {
    pub const SIZE: usize = 32;

    pub fn new(msg_type: MessageType, test_id: u16, file_hash: u64, payload_len: u32) -> Self {
        Self {
            magic: PROTOCOL_MAGIC,
            msg_type: msg_type as u8,
            flags: 0,
            test_id,
            file_hash,
            payload_len,
            reserved: [0; 12],
        }
    }

    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.magic != PROTOCOL_MAGIC {
            return Err(ProtocolError::InvalidMagic(self.magic));
        }
        let _ = MessageType::try_from(self.msg_type)?;
        if self.payload_len as usize > MAX_PAYLOAD_SIZE {
            return Err(ProtocolError::PayloadTooLarge(
                self.payload_len as usize,
                MAX_PAYLOAD_SIZE,
            ));
        }
        Ok(())
    }

    /// Convert header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.magic.to_le_bytes());
        bytes[4] = self.msg_type;
        bytes[5] = self.flags;
        bytes[6..8].copy_from_slice(&self.test_id.to_le_bytes());
        bytes[8..16].copy_from_slice(&self.file_hash.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes[20..32].copy_from_slice(&self.reserved);
        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < Self::SIZE {
            return Err(ProtocolError::DeserializationFailed(
                "Data too short for header".to_string(),
            ));
        }

        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let msg_type = bytes[4];
        let flags = bytes[5];
        let test_id = u16::from_le_bytes([bytes[6], bytes[7]]);
        let file_hash = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        let payload_len = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let mut reserved = [0u8; 12];
        reserved.copy_from_slice(&bytes[20..32]);

        Ok(Self {
            magic,
            msg_type,
            flags,
            test_id,
            file_hash,
            payload_len,
            reserved,
        })
    }
}

/// Test result (40-byte header)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TestResultHeader {
    pub test_id: u16,           // (2 bytes)
    pub status: u8,             // PASS=0, FAIL=1, SKIP=2, ERROR=3 (1 byte)
    pub _padding: u8,           // (1 byte)
    pub duration_ns: u64,       // (8 bytes)
    pub assertions_passed: u32, // (4 bytes)
    pub assertions_failed: u32, // (4 bytes)
    pub stdout_len: u32,        // (4 bytes)
    pub stderr_len: u32,        // (4 bytes)
    pub traceback_len: u32,     // (4 bytes)
    pub _reserved: [u8; 8],     // (8 bytes) - total = 40 bytes
}

impl TestResultHeader {
    pub const SIZE: usize = 40;

    /// Convert header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..2].copy_from_slice(&self.test_id.to_le_bytes());
        bytes[2] = self.status;
        bytes[3] = self._padding;
        bytes[4..12].copy_from_slice(&self.duration_ns.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.assertions_passed.to_le_bytes());
        bytes[16..20].copy_from_slice(&self.assertions_failed.to_le_bytes());
        bytes[20..24].copy_from_slice(&self.stdout_len.to_le_bytes());
        bytes[24..28].copy_from_slice(&self.stderr_len.to_le_bytes());
        bytes[28..32].copy_from_slice(&self.traceback_len.to_le_bytes());
        bytes[32..40].copy_from_slice(&self._reserved);
        bytes
    }

    /// Parse header from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ProtocolError> {
        if bytes.len() < Self::SIZE {
            return Err(ProtocolError::DeserializationFailed(
                "Data too short for result header".to_string(),
            ));
        }

        Ok(Self {
            test_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            status: bytes[2],
            _padding: bytes[3],
            duration_ns: u64::from_le_bytes([
                bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11],
            ]),
            assertions_passed: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            assertions_failed: u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]),
            stdout_len: u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            stderr_len: u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
            traceback_len: u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            _reserved: [
                bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38],
                bytes[39],
            ],
        })
    }
}

/// Binary protocol implementation
pub struct BinaryProtocol;

impl BinaryProtocol {
    /// Serialize a TestCase to binary format
    pub fn serialize_test(test: &TestCase) -> Result<Vec<u8>, ProtocolError> {
        let payload = bincode::serialize(test)
            .map_err(|e| ProtocolError::SerializationFailed(e.to_string()))?;

        if payload.len() > MAX_PAYLOAD_SIZE {
            return Err(ProtocolError::PayloadTooLarge(payload.len(), MAX_PAYLOAD_SIZE));
        }

        let header = TestMessageHeader::new(
            MessageType::Run,
            (test.id.0 & 0xFFFF) as u16,
            test.id.0,
            payload.len() as u32,
        );

        let mut result = Vec::with_capacity(TestMessageHeader::SIZE + payload.len());
        result.extend_from_slice(&header.to_bytes());
        result.extend_from_slice(&payload);

        Ok(result)
    }

    /// Deserialize a TestCase from binary format
    pub fn deserialize_test(data: &[u8]) -> Result<TestCase, ProtocolError> {
        let header = TestMessageHeader::from_bytes(data)?;
        header.validate()?;

        let payload = &data[TestMessageHeader::SIZE..];
        bincode::deserialize(payload)
            .map_err(|e| ProtocolError::DeserializationFailed(e.to_string()))
    }

    /// Deserialize a TestResult from binary format
    pub fn deserialize_result(data: &[u8]) -> Result<TestResult, ProtocolError> {
        let header = TestResultHeader::from_bytes(data)?;

        let status = match header.status {
            0 => TestStatus::Pass,
            1 => TestStatus::Fail,
            2 => TestStatus::Skip {
                reason: String::new(),
            },
            3 => TestStatus::Error {
                message: String::new(),
            },
            _ => return Err(ProtocolError::InvalidMessageType(header.status)),
        };

        let mut offset = TestResultHeader::SIZE;

        let stdout = if header.stdout_len > 0 {
            let end = offset + header.stdout_len as usize;
            if end > data.len() {
                return Err(ProtocolError::DeserializationFailed(
                    "Stdout length exceeds data".to_string(),
                ));
            }
            let s = String::from_utf8_lossy(&data[offset..end]).to_string();
            offset = end;
            s
        } else {
            String::new()
        };

        let stderr = if header.stderr_len > 0 {
            let end = offset + header.stderr_len as usize;
            if end > data.len() {
                return Err(ProtocolError::DeserializationFailed(
                    "Stderr length exceeds data".to_string(),
                ));
            }
            let s = String::from_utf8_lossy(&data[offset..end]).to_string();
            offset = end;
            s
        } else {
            String::new()
        };

        let traceback = if header.traceback_len > 0 {
            let end = offset + header.traceback_len as usize;
            if end > data.len() {
                return Err(ProtocolError::DeserializationFailed(
                    "Traceback length exceeds data".to_string(),
                ));
            }
            Some(String::from_utf8_lossy(&data[offset..end]).to_string())
        } else {
            None
        };

        Ok(TestResult {
            test_id: TestId(header.test_id as u64),
            status,
            duration: Duration::from_nanos(header.duration_ns),
            stdout,
            stderr,
            traceback,
            assertions: AssertionStats::new(header.assertions_passed, header.assertions_failed),
            assertion_failure: None,
        })
    }
}
