//! Function signature for cache lookup

use std::hash::{Hash, Hasher};

/// Function signature for cache lookup
/// Combines source hash, bytecode hash, and type profile hash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSignature {
    /// BLAKE3 hash of the source code
    pub source_hash: [u8; 32],
    /// BLAKE3 hash of the compiled bytecode
    pub bytecode_hash: [u8; 32],
    /// Hash of the type profile (for specialized code)
    pub type_profile_hash: u64,
    /// Function name for debugging
    pub name: String,
    /// Module path
    pub module: String,
}

impl FunctionSignature {
    /// Create a new function signature
    pub fn new(
        source: &[u8],
        bytecode: &[u8],
        type_profile: &[u8],
        name: String,
        module: String,
    ) -> Self {
        Self {
            source_hash: blake3::hash(source).into(),
            bytecode_hash: blake3::hash(bytecode).into(),
            type_profile_hash: Self::hash_type_profile(type_profile),
            name,
            module,
        }
    }

    /// Create signature from pre-computed hashes
    pub fn from_hashes(
        source_hash: [u8; 32],
        bytecode_hash: [u8; 32],
        type_profile_hash: u64,
        name: String,
        module: String,
    ) -> Self {
        Self {
            source_hash,
            bytecode_hash,
            type_profile_hash,
            name,
            module,
        }
    }

    /// Hash the type profile
    fn hash_type_profile(profile: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        profile.hash(&mut hasher);
        hasher.finish()
    }

    /// Get a compact key for cache lookup
    pub fn cache_key(&self) -> u128 {
        let mut key: u128 = 0;
        // Combine first 8 bytes of each hash
        for i in 0..8 {
            key |= (self.source_hash[i] as u128) << (i * 8);
        }
        key |= (self.type_profile_hash as u128) << 64;
        key
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(128);
        bytes.extend_from_slice(&self.source_hash);
        bytes.extend_from_slice(&self.bytecode_hash);
        bytes.extend_from_slice(&self.type_profile_hash.to_le_bytes());

        // Name length and data
        let name_bytes = self.name.as_bytes();
        bytes.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(name_bytes);

        // Module length and data
        let module_bytes = self.module.as_bytes();
        bytes.extend_from_slice(&(module_bytes.len() as u32).to_le_bytes());
        bytes.extend_from_slice(module_bytes);

        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 72 {
            return None;
        }

        let mut source_hash = [0u8; 32];
        source_hash.copy_from_slice(&bytes[0..32]);

        let mut bytecode_hash = [0u8; 32];
        bytecode_hash.copy_from_slice(&bytes[32..64]);

        let type_profile_hash = u64::from_le_bytes(bytes[64..72].try_into().ok()?);

        let name_len = u32::from_le_bytes(bytes[72..76].try_into().ok()?) as usize;
        if bytes.len() < 76 + name_len + 4 {
            return None;
        }
        let name = String::from_utf8(bytes[76..76 + name_len].to_vec()).ok()?;

        let module_offset = 76 + name_len;
        let module_len =
            u32::from_le_bytes(bytes[module_offset..module_offset + 4].try_into().ok()?) as usize;
        if bytes.len() < module_offset + 4 + module_len {
            return None;
        }
        let module =
            String::from_utf8(bytes[module_offset + 4..module_offset + 4 + module_len].to_vec())
                .ok()?;

        Some(Self {
            source_hash,
            bytecode_hash,
            type_profile_hash,
            name,
            module,
        })
    }
}

impl Hash for FunctionSignature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source_hash.hash(state);
        self.bytecode_hash.hash(state);
        self.type_profile_hash.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_creation() {
        let sig = FunctionSignature::new(
            b"def foo(): pass",
            b"\x00\x01\x02",
            b"int,int",
            "foo".to_string(),
            "test".to_string(),
        );

        assert_eq!(sig.name, "foo");
        assert_eq!(sig.module, "test");
    }

    #[test]
    fn test_signature_roundtrip() {
        let sig = FunctionSignature::new(
            b"def bar(x): return x",
            b"\x10\x20\x30",
            b"str",
            "bar".to_string(),
            "mymodule".to_string(),
        );

        let bytes = sig.to_bytes();
        let restored = FunctionSignature::from_bytes(&bytes).unwrap();

        assert_eq!(sig.source_hash, restored.source_hash);
        assert_eq!(sig.bytecode_hash, restored.bytecode_hash);
        assert_eq!(sig.type_profile_hash, restored.type_profile_hash);
        assert_eq!(sig.name, restored.name);
        assert_eq!(sig.module, restored.module);
    }

    #[test]
    fn test_cache_key() {
        let sig1 = FunctionSignature::new(
            b"source1",
            b"bytecode1",
            b"profile1",
            "f".to_string(),
            "m".to_string(),
        );

        let sig2 = FunctionSignature::new(
            b"source2",
            b"bytecode2",
            b"profile2",
            "f".to_string(),
            "m".to_string(),
        );

        assert_ne!(sig1.cache_key(), sig2.cache_key());
    }
}
