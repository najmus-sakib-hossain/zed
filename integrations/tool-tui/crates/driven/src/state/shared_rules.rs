//! Shared Rule Storage
//!
//! Thread-safe rule storage with zero-copy access.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::DirtyBits;

/// Reference to a shared rule
#[derive(Debug, Clone)]
pub struct RuleRef {
    /// Rule ID
    pub id: u32,
    /// Rule content
    content: Arc<[u8]>,
    /// Version number
    pub version: u64,
}

impl RuleRef {
    /// Create a new rule reference
    pub fn new(id: u32, content: Vec<u8>) -> Self {
        Self {
            id,
            content: content.into(),
            version: 1,
        }
    }

    /// Get content as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.content
    }

    /// Get content as string (if valid UTF-8)
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.content).ok()
    }

    /// Get content length
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Shared rule storage with dirty tracking
#[derive(Debug)]
pub struct SharedRules {
    /// Rules by ID
    rules: RwLock<HashMap<u32, RuleRef>>,
    /// Dirty bit tracker
    dirty: DirtyBits,
    /// Next rule ID
    next_id: std::sync::atomic::AtomicU32,
    /// Global version
    version: std::sync::atomic::AtomicU64,
}

impl SharedRules {
    /// Create new shared storage
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(HashMap::new()),
            dirty: DirtyBits::new(),
            next_id: std::sync::atomic::AtomicU32::new(1),
            version: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Insert a new rule
    pub fn insert(&self, content: Vec<u8>) -> u32 {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let rule = RuleRef::new(id, content);

        self.rules.write().unwrap().insert(id, rule);
        self.dirty.dirty_standard((id % 64) as u8);
        self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        id
    }

    /// Get a rule by ID
    pub fn get(&self, id: u32) -> Option<RuleRef> {
        self.rules.read().unwrap().get(&id).cloned()
    }

    /// Update a rule
    pub fn update(&self, id: u32, content: Vec<u8>) -> bool {
        let mut rules = self.rules.write().unwrap();
        if let Some(rule) = rules.get_mut(&id) {
            *rule = RuleRef {
                id,
                content: content.into(),
                version: rule.version + 1,
            };
            self.dirty.dirty_standard((id % 64) as u8);
            self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Remove a rule
    pub fn remove(&self, id: u32) -> Option<RuleRef> {
        let removed = self.rules.write().unwrap().remove(&id);
        if removed.is_some() {
            self.dirty.dirty_standard((id % 64) as u8);
            self.version.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        removed
    }

    /// Get all rule IDs
    pub fn ids(&self) -> Vec<u32> {
        self.rules.read().unwrap().keys().copied().collect()
    }

    /// Get rule count
    pub fn len(&self) -> usize {
        self.rules.read().unwrap().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.rules.read().unwrap().is_empty()
    }

    /// Get dirty tracker
    pub fn dirty(&self) -> &DirtyBits {
        &self.dirty
    }

    /// Get current version
    pub fn version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Check if changes exist
    pub fn has_changes(&self) -> bool {
        self.dirty.has_changes()
    }

    /// Mark as synced
    pub fn mark_synced(&self) {
        self.dirty.mark_synced();
    }

    /// Get dirty rule IDs
    pub fn dirty_ids(&self) -> Vec<u32> {
        let rules = self.rules.read().unwrap();
        let dirty_indices = self.dirty.standards.dirty_indices();

        rules
            .keys()
            .filter(|&&id| dirty_indices.contains(&((id % 64) as u8)))
            .copied()
            .collect()
    }

    /// Clear all rules
    pub fn clear(&self) {
        self.rules.write().unwrap().clear();
        self.dirty.mark_synced();
    }
}

impl Default for SharedRules {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe shared rules handle
pub type SharedRulesHandle = Arc<SharedRules>;

/// Create a new shared rules handle
pub fn create_shared_rules() -> SharedRulesHandle {
    Arc::new(SharedRules::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let rules = SharedRules::new();

        let id = rules.insert(b"rule content".to_vec());
        let rule = rules.get(id).unwrap();

        assert_eq!(rule.as_bytes(), b"rule content");
        assert_eq!(rule.version, 1);
    }

    #[test]
    fn test_update() {
        let rules = SharedRules::new();

        let id = rules.insert(b"original".to_vec());
        rules.update(id, b"updated".to_vec());

        let rule = rules.get(id).unwrap();
        assert_eq!(rule.as_bytes(), b"updated");
        assert_eq!(rule.version, 2);
    }

    #[test]
    fn test_dirty_tracking() {
        let rules = SharedRules::new();

        assert!(!rules.has_changes());

        rules.insert(b"test".to_vec());
        assert!(rules.has_changes());

        rules.mark_synced();
        assert!(!rules.has_changes());
    }

    #[test]
    fn test_thread_safety() {
        let rules = Arc::new(SharedRules::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let rules = rules.clone();
                std::thread::spawn(move || {
                    rules.insert(format!("rule {}", i).into_bytes());
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(rules.len(), 10);
    }
}
