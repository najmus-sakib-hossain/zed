//! Export table with perfect hashing for O(1) symbol lookup

use crate::format::{DpmError, ExportEntry, ExportKind};

/// Perfect hash function using FNV-1a with seed
#[inline]
fn perfect_hash(name: &str, seed: u32, table_size: usize) -> usize {
    // Use a combination of seed and FNV-1a for better distribution
    let mut hash: u64 = 14695981039346656037u64;
    hash ^= seed as u64;
    hash = hash.wrapping_mul(1099511628211);

    for byte in name.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }

    // Mix the bits more thoroughly
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51afd7ed558ccd);
    hash ^= hash >> 33;

    (hash as usize) % table_size
}

/// Export table with perfect hashing for O(1) lookup
pub struct ExportTable {
    /// The seed used for perfect hashing
    seed: u32,
    /// Export entries (indexed by perfect hash)
    entries: Vec<Option<ExportEntry>>,
}

impl ExportTable {
    /// Create a new empty export table
    pub fn new() -> Self {
        Self {
            seed: 0,
            entries: Vec::new(),
        }
    }

    /// Build an export table from a list of (name, kind, value_offset) tuples
    pub fn build(exports: &[(String, ExportKind, u32)]) -> Result<Self, DpmError> {
        if exports.is_empty() {
            return Ok(Self::new());
        }

        // Use a larger table size to reduce collision probability
        let table_size = (exports.len() * 4).next_power_of_two().max(16);

        // Try different seeds to find a perfect hash
        for seed in 0..1000000u32 {
            let mut entries: Vec<Option<ExportEntry>> = vec![None; table_size];
            let mut collision = false;

            for (name, kind, value_offset) in exports {
                let idx = perfect_hash(name, seed, table_size);
                if entries[idx].is_some() {
                    collision = true;
                    break;
                }

                let name_hash = {
                    let mut h: u64 = 14695981039346656037;
                    for b in name.bytes() {
                        h ^= b as u64;
                        h = h.wrapping_mul(1099511628211);
                    }
                    h
                };

                entries[idx] = Some(ExportEntry {
                    name_hash,
                    name_offset: 0, // Set during serialization
                    kind: *kind,
                    _reserved: [0; 3],
                    value_offset: *value_offset,
                });
            }

            if !collision {
                return Ok(Self { seed, entries });
            }
        }

        Err(DpmError::PerfectHashFailed)
    }

    /// Get an export by name with O(1) lookup
    #[inline]
    pub fn get(&self, name: &str) -> Option<&ExportEntry> {
        if self.entries.is_empty() {
            return None;
        }

        let idx = perfect_hash(name, self.seed, self.entries.len());
        let entry = self.entries.get(idx)?.as_ref()?;

        // Verify the name hash matches
        let name_hash = {
            let mut h: u64 = 14695981039346656037;
            for b in name.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(1099511628211);
            }
            h
        };

        if entry.name_hash == name_hash {
            Some(entry)
        } else {
            None
        }
    }

    /// Get the seed used for perfect hashing
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// Get all entries
    pub fn entries(&self) -> &[Option<ExportEntry>] {
        &self.entries
    }

    /// Get the number of exports
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ExportTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_table() {
        let table = ExportTable::new();
        assert!(table.is_empty());
        assert!(table.get("foo").is_none());
    }

    #[test]
    fn test_build_and_lookup() {
        let exports = vec![
            ("foo".to_string(), ExportKind::Function, 100),
            ("bar".to_string(), ExportKind::Class, 200),
            ("baz".to_string(), ExportKind::Constant, 300),
        ];

        let table = ExportTable::build(&exports).unwrap();
        assert_eq!(table.len(), 3);

        let foo = table.get("foo").unwrap();
        assert_eq!(foo.kind, ExportKind::Function);
        assert_eq!(foo.value_offset, 100);

        let bar = table.get("bar").unwrap();
        assert_eq!(bar.kind, ExportKind::Class);
        assert_eq!(bar.value_offset, 200);

        assert!(table.get("nonexistent").is_none());
    }

    #[test]
    fn test_o1_lookup() {
        // Build a larger table to verify O(1) behavior
        let exports: Vec<_> = (0..100)
            .map(|i| (format!("symbol_{}", i), ExportKind::Variable, i as u32))
            .collect();

        let table = ExportTable::build(&exports).unwrap();

        // All lookups should succeed
        for i in 0..100 {
            let name = format!("symbol_{}", i);
            let entry = table.get(&name).unwrap();
            assert_eq!(entry.value_offset, i as u32);
        }
    }
}
