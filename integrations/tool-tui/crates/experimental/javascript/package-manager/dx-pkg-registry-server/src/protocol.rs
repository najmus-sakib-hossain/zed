//! DXRP Protocol Implementation

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

/// DXRP Request (32 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DxrpRequest {
    pub magic: [u8; 4],    // "DXRP"
    pub op: DxrpOp,        // Operation (1 byte)
    pub _padding: [u8; 3], // Alignment
    pub name_hash: u64,    // blake3(package_name)
    pub version: u64,      // Encoded version
    pub checksum: u64,     // Request integrity
}

/// DXRP Operations
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxrpOp {
    Resolve = 1,  // Resolve package metadata
    Download = 2, // Download .dxp package
    Ping = 3,     // Health check
}

impl TryFrom<u8> for DxrpOp {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(DxrpOp::Resolve),
            2 => Ok(DxrpOp::Download),
            3 => Ok(DxrpOp::Ping),
            _ => Err(anyhow!("Invalid operation: {}", value)),
        }
    }
}

impl DxrpRequest {
    /// Parse request from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        // Check magic
        if &bytes[0..4] != b"DXRP" {
            return Err(anyhow!("Invalid magic bytes"));
        }

        // Parse operation
        let op = DxrpOp::try_from(bytes[4])?;

        // Parse fields
        let name_hash = u64::from_le_bytes(bytes[8..16].try_into()?);
        let version = u64::from_le_bytes(bytes[16..24].try_into()?);
        let checksum = u64::from_le_bytes(bytes[24..32].try_into()?);

        Ok(Self {
            magic: [b'D', b'X', b'R', b'P'],
            op,
            _padding: [0; 3],
            name_hash,
            version,
            checksum,
        })
    }

    /// Convert to bytes
    #[allow(dead_code)]
    #[allow(clippy::wrong_self_convention)]
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4] = self.op as u8;
        bytes[8..16].copy_from_slice(&self.name_hash.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.version.to_le_bytes());
        bytes[24..32].copy_from_slice(&self.checksum.to_le_bytes());
        bytes
    }
}

/// DXRP Response (32 bytes header + payload)
#[derive(Debug, Clone)]
pub struct DxrpResponse {
    pub status: DxrpStatus,
    pub payload_size: u64,
    pub payload_hash: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum DxrpStatus {
    Ok = 0,
    NotFound = 1,
    Error = 2,
}

impl DxrpResponse {
    /// Create success response
    pub fn ok(payload: &[u8]) -> Self {
        let hash = blake3::hash(payload);
        let hash_u64 = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap_or([0u8; 8]));

        Self {
            status: DxrpStatus::Ok,
            payload_size: payload.len() as u64,
            payload_hash: hash_u64,
        }
    }

    /// Create error response
    pub fn error(msg: &str) -> Self {
        let payload = msg.as_bytes();
        let hash = blake3::hash(payload);
        let hash_u64 = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap_or([0u8; 8]));

        Self {
            status: DxrpStatus::Error,
            payload_size: payload.len() as u64,
            payload_hash: hash_u64,
        }
    }

    /// Convert to bytes (32 byte header)
    pub fn to_bytes(&self) -> [u8; 32] {
        let mut bytes = [0u8; 32];
        bytes[0] = self.status as u8;
        bytes[8..16].copy_from_slice(&self.payload_size.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.payload_hash.to_le_bytes());
        bytes
    }
}

/// Package metadata (returned by Resolve)
#[derive(Debug, Clone, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<(String, String)>,
    pub size: u64,
    pub hash: String,
}
