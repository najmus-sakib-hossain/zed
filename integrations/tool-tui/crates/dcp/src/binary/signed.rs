//! Ed25519 signed structures for security.

use crate::DCPError;

/// Ed25519 signed tool definition
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedToolDef {
    /// Tool identifier
    pub tool_id: u32,
    /// Blake3 hash of schema
    pub schema_hash: [u8; 32],
    /// Required capabilities bitfield
    pub capabilities: u64,
    /// Ed25519 signature
    pub signature: [u8; 64],
    /// Signer's public key
    pub public_key: [u8; 32],
}

/// Signed invocation with replay protection
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedInvocation {
    /// Tool identifier
    pub tool_id: u32,
    /// Unique nonce for replay protection
    pub nonce: u64,
    /// Timestamp for expiration
    pub timestamp: u64,
    /// Blake3 hash of arguments
    pub args_hash: [u8; 32],
    /// Ed25519 signature
    pub signature: [u8; 64],
}

impl SignedToolDef {
    /// Size of the struct in bytes
    pub const SIZE: usize = 144; // 4 + 32 + 8 + 64 + 32 + padding

    /// Parse from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least SIZE bytes
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: SignedToolDef is repr(C) with predictable layout
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Get the bytes that are signed (everything before the signature)
    pub fn signed_bytes(&self) -> &[u8] {
        // tool_id (4) + schema_hash (32) + capabilities (8) = 44 bytes
        &self.as_bytes()[..44]
    }
}

impl SignedInvocation {
    /// Size of the struct in bytes
    pub const SIZE: usize = 120; // 4 + 8 + 8 + 32 + 64 + padding

    /// Parse from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least SIZE bytes
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: SignedInvocation is repr(C) with predictable layout
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Get the bytes that are signed (everything before the signature)
    pub fn signed_bytes(&self) -> &[u8] {
        // tool_id (4) + nonce (8) + timestamp (8) + args_hash (32) = 52 bytes
        &self.as_bytes()[..52]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signed_tool_def_size() {
        assert_eq!(std::mem::size_of::<SignedToolDef>(), SignedToolDef::SIZE);
    }

    #[test]
    fn test_signed_invocation_size() {
        assert_eq!(std::mem::size_of::<SignedInvocation>(), SignedInvocation::SIZE);
    }

    #[test]
    fn test_signed_tool_def_round_trip() {
        let def = SignedToolDef {
            tool_id: 42,
            schema_hash: [0xAB; 32],
            capabilities: 0x1234567890ABCDEF,
            signature: [0xCD; 64],
            public_key: [0xEF; 32],
        };
        let bytes = def.as_bytes();
        let parsed = SignedToolDef::from_bytes(bytes).unwrap();

        assert_eq!(parsed.tool_id, 42);
        assert_eq!(parsed.schema_hash, [0xAB; 32]);
        assert_eq!(parsed.capabilities, 0x1234567890ABCDEF);
        assert_eq!(parsed.signature, [0xCD; 64]);
        assert_eq!(parsed.public_key, [0xEF; 32]);
    }

    #[test]
    fn test_signed_invocation_round_trip() {
        let inv = SignedInvocation {
            tool_id: 123,
            nonce: 0xDEADBEEF,
            timestamp: 1234567890,
            args_hash: [0x11; 32],
            signature: [0x22; 64],
        };
        let bytes = inv.as_bytes();
        let parsed = SignedInvocation::from_bytes(bytes).unwrap();

        assert_eq!(parsed.tool_id, 123);
        assert_eq!(parsed.nonce, 0xDEADBEEF);
        assert_eq!(parsed.timestamp, 1234567890);
        assert_eq!(parsed.args_hash, [0x11; 32]);
        assert_eq!(parsed.signature, [0x22; 64]);
    }

    #[test]
    fn test_signed_bytes_length() {
        let def = SignedToolDef {
            tool_id: 0,
            schema_hash: [0; 32],
            capabilities: 0,
            signature: [0; 64],
            public_key: [0; 32],
        };
        assert_eq!(def.signed_bytes().len(), 44);

        let inv = SignedInvocation {
            tool_id: 0,
            nonce: 0,
            timestamp: 0,
            args_hash: [0; 32],
            signature: [0; 64],
        };
        assert_eq!(inv.signed_bytes().len(), 52);
    }
}
