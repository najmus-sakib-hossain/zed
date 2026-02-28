//! Cached compilation artifacts

use std::time::{SystemTime, UNIX_EPOCH};

/// Compilation tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CompilationTier {
    /// Interpreted (no compilation)
    Interpreter = 0,
    /// Baseline JIT (1:1 bytecode to machine code)
    Baseline = 1,
    /// Optimizing JIT (type-specialized)
    Optimized = 2,
    /// AOT optimized (full optimization)
    AotOptimized = 3,
}

impl CompilationTier {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Interpreter),
            1 => Some(Self::Baseline),
            2 => Some(Self::Optimized),
            3 => Some(Self::AotOptimized),
            _ => None,
        }
    }
}

/// Relocation entry for position-independent code
#[derive(Debug, Clone)]
pub struct Relocation {
    /// Offset in the code where relocation is needed
    pub offset: u32,
    /// Type of relocation
    pub kind: RelocationType,
    /// Symbol or address to relocate to
    pub target: u64,
}

/// Types of relocations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RelocationType {
    /// Absolute 64-bit address
    Abs64 = 0,
    /// PC-relative 32-bit offset
    Rel32 = 1,
    /// GOT entry
    GotEntry = 2,
    /// PLT entry
    PltEntry = 3,
}

impl RelocationType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Abs64),
            1 => Some(Self::Rel32),
            2 => Some(Self::GotEntry),
            3 => Some(Self::PltEntry),
            _ => None,
        }
    }
}

impl Relocation {
    pub fn to_bytes(&self) -> [u8; 13] {
        let mut bytes = [0u8; 13];
        bytes[0..4].copy_from_slice(&self.offset.to_le_bytes());
        bytes[4] = self.kind as u8;
        bytes[5..13].copy_from_slice(&self.target.to_le_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8; 13]) -> Option<Self> {
        Some(Self {
            offset: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            kind: RelocationType::from_u8(bytes[4])?,
            target: u64::from_le_bytes(bytes[5..13].try_into().ok()?),
        })
    }
}

/// Cached compilation artifact
#[derive(Debug, Clone)]
pub struct CachedArtifact {
    /// Compilation tier
    pub tier: CompilationTier,
    /// Offset in the code cache
    pub code_offset: u64,
    /// Size of the compiled code
    pub code_size: u32,
    /// Relocations needed when loading
    pub relocations: Vec<Relocation>,
    /// Profile data for further optimization
    pub profile_data: Vec<u8>,
    /// Creation timestamp (seconds since UNIX epoch)
    pub created_at: u64,
    /// Last access timestamp
    pub last_accessed: u64,
    /// Access count for LRU
    pub access_count: u32,
}

impl CachedArtifact {
    /// Create a new cached artifact
    pub fn new(
        tier: CompilationTier,
        code_offset: u64,
        code_size: u32,
        relocations: Vec<Relocation>,
        profile_data: Vec<u8>,
    ) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0); // Use 0 if system time is before UNIX epoch (shouldn't happen)

        Self {
            tier,
            code_offset,
            code_size,
            relocations,
            profile_data,
            created_at: now,
            last_accessed: now,
            access_count: 0,
        }
    }

    /// Record an access
    pub fn record_access(&mut self) {
        self.last_accessed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(self.last_accessed); // Keep previous value on error
        self.access_count = self.access_count.saturating_add(1);
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Fixed header
        bytes.push(self.tier as u8);
        bytes.extend_from_slice(&self.code_offset.to_le_bytes());
        bytes.extend_from_slice(&self.code_size.to_le_bytes());
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        bytes.extend_from_slice(&self.last_accessed.to_le_bytes());
        bytes.extend_from_slice(&self.access_count.to_le_bytes());

        // Relocations
        bytes.extend_from_slice(&(self.relocations.len() as u32).to_le_bytes());
        for reloc in &self.relocations {
            bytes.extend_from_slice(&reloc.to_bytes());
        }

        // Profile data
        bytes.extend_from_slice(&(self.profile_data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.profile_data);

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 37 {
            return None;
        }

        let tier = CompilationTier::from_u8(bytes[0])?;
        let code_offset = u64::from_le_bytes(bytes[1..9].try_into().ok()?);
        let code_size = u32::from_le_bytes(bytes[9..13].try_into().ok()?);
        let created_at = u64::from_le_bytes(bytes[13..21].try_into().ok()?);
        let last_accessed = u64::from_le_bytes(bytes[21..29].try_into().ok()?);
        let access_count = u32::from_le_bytes(bytes[29..33].try_into().ok()?);

        let reloc_count = u32::from_le_bytes(bytes[33..37].try_into().ok()?) as usize;
        let mut offset = 37;

        let mut relocations = Vec::with_capacity(reloc_count);
        for _ in 0..reloc_count {
            if offset + 13 > bytes.len() {
                return None;
            }
            let reloc_bytes: [u8; 13] = bytes[offset..offset + 13].try_into().ok()?;
            relocations.push(Relocation::from_bytes(&reloc_bytes)?);
            offset += 13;
        }

        if offset + 4 > bytes.len() {
            return None;
        }
        let profile_len = u32::from_le_bytes(bytes[offset..offset + 4].try_into().ok()?) as usize;
        offset += 4;

        if offset + profile_len > bytes.len() {
            return None;
        }
        let profile_data = bytes[offset..offset + profile_len].to_vec();

        Some(Self {
            tier,
            code_offset,
            code_size,
            relocations,
            profile_data,
            created_at,
            last_accessed,
            access_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_roundtrip() {
        for i in 0..4u8 {
            let tier = CompilationTier::from_u8(i).unwrap();
            assert_eq!(tier as u8, i);
        }
    }

    #[test]
    fn test_relocation_roundtrip() {
        let reloc = Relocation {
            offset: 0x1234,
            kind: RelocationType::Rel32,
            target: 0xDEADBEEF,
        };

        let bytes = reloc.to_bytes();
        let restored = Relocation::from_bytes(&bytes).unwrap();

        assert_eq!(reloc.offset, restored.offset);
        assert_eq!(reloc.kind, restored.kind);
        assert_eq!(reloc.target, restored.target);
    }

    #[test]
    fn test_artifact_roundtrip() {
        let artifact = CachedArtifact::new(
            CompilationTier::Optimized,
            0x1000,
            256,
            vec![Relocation {
                offset: 10,
                kind: RelocationType::Abs64,
                target: 0x12345678,
            }],
            vec![1, 2, 3, 4],
        );

        let bytes = artifact.to_bytes();
        let restored = CachedArtifact::from_bytes(&bytes).unwrap();

        assert_eq!(artifact.tier, restored.tier);
        assert_eq!(artifact.code_offset, restored.code_offset);
        assert_eq!(artifact.code_size, restored.code_size);
        assert_eq!(artifact.relocations.len(), restored.relocations.len());
        assert_eq!(artifact.profile_data, restored.profile_data);
    }

    #[test]
    fn test_access_recording() {
        let mut artifact = CachedArtifact::new(CompilationTier::Baseline, 0, 100, vec![], vec![]);

        assert_eq!(artifact.access_count, 0);
        artifact.record_access();
        assert_eq!(artifact.access_count, 1);
        artifact.record_access();
        assert_eq!(artifact.access_count, 2);
    }
}
