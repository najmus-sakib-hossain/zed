//! Dirty-Bit Change Tracking
//!
//! O(1) change detection using bitmask tracking.

use std::sync::atomic::{AtomicU64, Ordering};

/// 64-bit dirty mask for tracking up to 64 rule sections
#[derive(Debug)]
pub struct DirtyMask(AtomicU64);

impl DirtyMask {
    /// Create a clean mask
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Create a fully dirty mask
    pub fn all_dirty() -> Self {
        Self(AtomicU64::new(u64::MAX))
    }

    /// Mark a bit as dirty
    pub fn mark_dirty(&self, bit: u8) {
        if bit < 64 {
            self.0.fetch_or(1 << bit, Ordering::SeqCst);
        }
    }

    /// Clear a bit
    pub fn clear(&self, bit: u8) {
        if bit < 64 {
            self.0.fetch_and(!(1 << bit), Ordering::SeqCst);
        }
    }

    /// Clear all bits
    pub fn clear_all(&self) {
        self.0.store(0, Ordering::SeqCst);
    }

    /// Check if a bit is dirty
    pub fn is_dirty(&self, bit: u8) -> bool {
        if bit >= 64 {
            return false;
        }
        (self.0.load(Ordering::SeqCst) & (1 << bit)) != 0
    }

    /// Check if any bits are dirty
    pub fn any_dirty(&self) -> bool {
        self.0.load(Ordering::SeqCst) != 0
    }

    /// Get raw value
    pub fn raw(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }

    /// Count dirty bits
    pub fn count(&self) -> u32 {
        self.0.load(Ordering::SeqCst).count_ones()
    }

    /// Get dirty bit indices
    pub fn dirty_indices(&self) -> Vec<u8> {
        let raw = self.raw();
        (0..64).filter(|&i| (raw & (1 << i)) != 0).collect()
    }

    /// Swap and get previous value
    pub fn swap(&self, new_value: u64) -> u64 {
        self.0.swap(new_value, Ordering::SeqCst)
    }
}

impl Default for DirtyMask {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DirtyMask {
    fn clone(&self) -> Self {
        Self(AtomicU64::new(self.0.load(Ordering::SeqCst)))
    }
}

/// Dirty-bit tracker for rule sections
#[derive(Debug)]
pub struct DirtyBits {
    /// Mask for persona changes
    pub persona: DirtyMask,
    /// Mask for standards changes
    pub standards: DirtyMask,
    /// Mask for workflow changes
    pub workflow: DirtyMask,
    /// Mask for context changes
    pub context: DirtyMask,
    /// Global change counter
    change_counter: AtomicU64,
    /// Last sync counter
    last_sync: AtomicU64,
}

impl DirtyBits {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            persona: DirtyMask::new(),
            standards: DirtyMask::new(),
            workflow: DirtyMask::new(),
            context: DirtyMask::new(),
            change_counter: AtomicU64::new(0),
            last_sync: AtomicU64::new(0),
        }
    }

    /// Mark persona as dirty
    pub fn dirty_persona(&self, index: u8) {
        self.persona.mark_dirty(index);
        self.change_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark standard as dirty
    pub fn dirty_standard(&self, index: u8) {
        self.standards.mark_dirty(index);
        self.change_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark workflow step as dirty
    pub fn dirty_workflow(&self, index: u8) {
        self.workflow.mark_dirty(index);
        self.change_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Mark context as dirty
    pub fn dirty_context(&self, index: u8) {
        self.context.mark_dirty(index);
        self.change_counter.fetch_add(1, Ordering::SeqCst);
    }

    /// Check if any changes exist since last sync
    pub fn has_changes(&self) -> bool {
        self.change_counter.load(Ordering::SeqCst) > self.last_sync.load(Ordering::SeqCst)
    }

    /// Clear all dirty bits and mark as synced
    pub fn mark_synced(&self) {
        self.persona.clear_all();
        self.standards.clear_all();
        self.workflow.clear_all();
        self.context.clear_all();
        self.last_sync
            .store(self.change_counter.load(Ordering::SeqCst), Ordering::SeqCst);
    }

    /// Get change count
    pub fn change_count(&self) -> u64 {
        self.change_counter.load(Ordering::SeqCst)
    }

    /// Get changes since last sync
    pub fn pending_changes(&self) -> u64 {
        self.change_counter.load(Ordering::SeqCst) - self.last_sync.load(Ordering::SeqCst)
    }

    /// Get summary of dirty sections
    pub fn summary(&self) -> DirtySummary {
        DirtySummary {
            persona_dirty: self.persona.any_dirty(),
            standards_dirty: self.standards.any_dirty(),
            workflow_dirty: self.workflow.any_dirty(),
            context_dirty: self.context.any_dirty(),
            total_dirty: self.persona.count()
                + self.standards.count()
                + self.workflow.count()
                + self.context.count(),
        }
    }
}

impl Default for DirtyBits {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of dirty state
#[derive(Debug, Clone)]
pub struct DirtySummary {
    /// Persona section has changes
    pub persona_dirty: bool,
    /// Standards section has changes
    pub standards_dirty: bool,
    /// Workflow section has changes
    pub workflow_dirty: bool,
    /// Context section has changes
    pub context_dirty: bool,
    /// Total dirty bit count
    pub total_dirty: u32,
}

impl DirtySummary {
    /// Check if any section is dirty
    pub fn any_dirty(&self) -> bool {
        self.persona_dirty || self.standards_dirty || self.workflow_dirty || self.context_dirty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_mask() {
        let mask = DirtyMask::new();
        assert!(!mask.any_dirty());

        mask.mark_dirty(0);
        assert!(mask.is_dirty(0));
        assert!(mask.any_dirty());

        mask.mark_dirty(5);
        assert!(mask.is_dirty(5));
        assert_eq!(mask.count(), 2);

        mask.clear(0);
        assert!(!mask.is_dirty(0));
        assert!(mask.is_dirty(5));
    }

    #[test]
    fn test_dirty_bits() {
        let bits = DirtyBits::new();
        assert!(!bits.has_changes());

        bits.dirty_standard(0);
        assert!(bits.has_changes());
        assert_eq!(bits.pending_changes(), 1);

        bits.dirty_standard(1);
        assert_eq!(bits.pending_changes(), 2);

        bits.mark_synced();
        assert!(!bits.has_changes());
        assert_eq!(bits.pending_changes(), 0);
    }

    #[test]
    fn test_dirty_summary() {
        let bits = DirtyBits::new();
        bits.dirty_persona(0);
        bits.dirty_workflow(5);

        let summary = bits.summary();
        assert!(summary.persona_dirty);
        assert!(!summary.standards_dirty);
        assert!(summary.workflow_dirty);
        assert!(summary.any_dirty());
    }
}
