//! Cache entry structure

use std::time::SystemTime;

/// Compilation tier for cached bytecode
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationTier {
    /// Interpreted bytecode
    Interpreter = 0,
    /// Baseline JIT compiled
    BaselineJit = 1,
    /// Optimizing JIT compiled
    OptimizingJit = 2,
    /// AOT optimized
    AotOptimized = 3,
}

impl CompilationTier {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Interpreter),
            1 => Some(Self::BaselineJit),
            2 => Some(Self::OptimizingJit),
            3 => Some(Self::AotOptimized),
            _ => None,
        }
    }
}

/// A cache entry for compiled bytecode
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// BLAKE3 hash of the source file
    pub source_hash: [u8; 32],
    /// Offset of the cached data in the cache file
    pub data_offset: u64,
    /// Size of the cached data
    pub data_size: u32,
    /// Timestamp when the entry was validated
    pub validated_at: u64,
    /// Compilation tier
    pub tier: CompilationTier,
    /// Source file modification time (for quick validation)
    pub source_mtime: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(
        source_hash: [u8; 32],
        data_offset: u64,
        data_size: u32,
        tier: CompilationTier,
        source_mtime: u64,
    ) -> Self {
        let validated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            source_hash,
            data_offset,
            data_size,
            validated_at,
            tier,
            source_mtime,
        }
    }

    /// Quick validation using modification time
    pub fn is_valid_quick(&self, current_mtime: u64) -> bool {
        self.source_mtime == current_mtime
    }

    /// Full validation using content hash
    pub fn validate_full(&self, source_content: &[u8]) -> bool {
        let hash = blake3::hash(source_content);
        hash.as_bytes() == &self.source_hash
    }

    /// Serialize the entry to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(64);
        bytes.extend_from_slice(&self.source_hash);
        bytes.extend_from_slice(&self.data_offset.to_le_bytes());
        bytes.extend_from_slice(&self.data_size.to_le_bytes());
        bytes.extend_from_slice(&self.validated_at.to_le_bytes());
        bytes.push(self.tier as u8);
        bytes.extend_from_slice(&[0u8; 3]); // padding
        bytes.extend_from_slice(&self.source_mtime.to_le_bytes());
        bytes
    }

    /// Deserialize an entry from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 64 {
            return None;
        }

        let mut source_hash = [0u8; 32];
        source_hash.copy_from_slice(&bytes[0..32]);

        let data_offset = u64::from_le_bytes([
            bytes[32], bytes[33], bytes[34], bytes[35], bytes[36], bytes[37], bytes[38], bytes[39],
        ]);

        let data_size = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);

        let validated_at = u64::from_le_bytes([
            bytes[44], bytes[45], bytes[46], bytes[47], bytes[48], bytes[49], bytes[50], bytes[51],
        ]);

        let tier = CompilationTier::from_u8(bytes[52])?;

        let source_mtime = u64::from_le_bytes([
            bytes[56], bytes[57], bytes[58], bytes[59], bytes[60], bytes[61], bytes[62], bytes[63],
        ]);

        Some(Self {
            source_hash,
            data_offset,
            data_size,
            validated_at,
            tier,
            source_mtime,
        })
    }

    /// Get the serialized size
    pub const fn serialized_size() -> usize {
        64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_roundtrip() {
        let hash = blake3::hash(b"test source");
        let entry =
            CacheEntry::new(*hash.as_bytes(), 1024, 512, CompilationTier::BaselineJit, 1234567890);

        let bytes = entry.to_bytes();
        let restored = CacheEntry::from_bytes(&bytes).unwrap();

        assert_eq!(restored.source_hash, entry.source_hash);
        assert_eq!(restored.data_offset, 1024);
        assert_eq!(restored.data_size, 512);
        assert_eq!(restored.tier, CompilationTier::BaselineJit);
        assert_eq!(restored.source_mtime, 1234567890);
    }

    #[test]
    fn test_quick_validation() {
        let entry = CacheEntry::new([0u8; 32], 0, 0, CompilationTier::Interpreter, 100);

        assert!(entry.is_valid_quick(100));
        assert!(!entry.is_valid_quick(101));
    }

    #[test]
    fn test_full_validation() {
        let source = b"test source content";
        let hash = blake3::hash(source);
        let entry = CacheEntry::new(*hash.as_bytes(), 0, 0, CompilationTier::Interpreter, 0);

        assert!(entry.validate_full(source));
        assert!(!entry.validate_full(b"different content"));
    }
}
