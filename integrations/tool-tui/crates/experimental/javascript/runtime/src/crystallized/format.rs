// Crystallized binary format (.dxb)
use serde::{Deserialize, Serialize};

pub const DXB_MAGIC: [u8; 4] = *b"DXB\x00";
pub const DXB_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
pub struct CrystallizedCode {
    pub magic: [u8; 4],
    pub version: u32,
    pub source_hash: [u8; 32],
    pub output: String,
}

impl CrystallizedCode {
    pub fn new(source: &str, output: String) -> Self {
        Self {
            magic: DXB_MAGIC,
            version: DXB_VERSION,
            source_hash: Self::hash_source(source),
            output,
        }
    }

    pub fn hash_source(source: &str) -> [u8; 32] {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = hasher.finish();

        let mut result = [0u8; 32];
        result[..8].copy_from_slice(&hash.to_le_bytes());
        result
    }

    pub fn is_valid(&self) -> bool {
        self.magic == DXB_MAGIC && self.version == DXB_VERSION
    }

    pub fn matches_source(&self, source: &str) -> bool {
        self.source_hash == Self::hash_source(source)
    }
}
