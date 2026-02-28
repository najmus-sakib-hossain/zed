// Crystallized cache for instant warm starts
use super::format::CrystallizedCode;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct CrystalCache {
    cache_dir: PathBuf,
    memory: HashMap<[u8; 32], String>,
}

impl CrystalCache {
    pub fn new() -> std::io::Result<Self> {
        let cache_dir = std::env::temp_dir().join("dx-cache");
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            cache_dir,
            memory: HashMap::new(),
        })
    }

    /// Get cached output or None
    pub fn get(&mut self, source: &str) -> Option<String> {
        let hash = CrystallizedCode::hash_source(source);

        // Check memory cache
        if let Some(output) = self.memory.get(&hash) {
            return Some(output.clone());
        }

        // Check disk cache
        let path = self.cache_path(&hash);
        if let Ok(data) = std::fs::read(&path) {
            if let Ok(crystal) = bincode::deserialize::<CrystallizedCode>(&data) {
                if crystal.is_valid() && crystal.matches_source(source) {
                    self.memory.insert(hash, crystal.output.clone());
                    return Some(crystal.output);
                }
            }
        }

        None
    }

    /// Store output to cache
    pub fn store(&mut self, source: &str, output: String) -> std::io::Result<()> {
        let hash = CrystallizedCode::hash_source(source);
        let crystal = CrystallizedCode::new(source, output.clone());

        // Store in memory
        self.memory.insert(hash, output);

        // Store on disk
        let path = self.cache_path(&hash);
        let data =
            bincode::serialize(&crystal).map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(&path, data)?;

        Ok(())
    }

    fn cache_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex = hex::encode(&hash[..8]);
        self.cache_dir.join(format!("{}.dxb", hex))
    }
}
