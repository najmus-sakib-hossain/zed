//! Dirty-Bit Template Caching - Feature #6
//!
//! Every template instance gets a 64-bit dirty mask tracking which parameters changed.
//! Enables O(1) change detection and partial regeneration.
//!
//! ## Dirty Bit Mapping
//!
//! - Bits 0-15: Simple value parameters (name, description, etc.)
//! - Bits 16-31: Structural parameters (with_state, has_tests, etc.)
//! - Bits 32-47: Array parameters (state_vars, imports, etc.)
//! - Bits 48-63: Composition parameters (parent_template, mixins, etc.)

use std::fmt;

// ============================================================================
// Dirty Mask
// ============================================================================

/// 64-bit dirty mask for O(1) change detection.
///
/// Each bit represents whether a specific parameter slot has changed
/// since the last generation. This enables:
///
/// - Skip regeneration entirely if output cached and params unchanged
/// - Partial regeneration for minor changes
/// - Full regeneration only when structural changes detected
///
/// # Bit Layout
///
/// ```text
/// 63                48 47                32 31                16 15                 0
/// ├──────────────────┼──────────────────┼──────────────────┼──────────────────┤
/// │   Composition    │      Array       │    Structural    │      Simple      │
/// │   (templates)    │   (collections)  │    (booleans)    │    (strings)     │
/// └──────────────────┴──────────────────┴──────────────────┴──────────────────┘
/// ```
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DirtyMask(u64);

impl DirtyMask {
    /// Mask for simple value parameters (bits 0-15).
    pub const SIMPLE_MASK: u64 = 0x0000_0000_0000_FFFF;

    /// Mask for structural parameters (bits 16-31).
    pub const STRUCTURAL_MASK: u64 = 0x0000_0000_FFFF_0000;

    /// Mask for array parameters (bits 32-47).
    pub const ARRAY_MASK: u64 = 0x0000_FFFF_0000_0000;

    /// Mask for composition parameters (bits 48-63).
    pub const COMPOSITION_MASK: u64 = 0xFFFF_0000_0000_0000;

    /// Create a clean (no dirty bits) mask.
    #[must_use]
    pub const fn clean() -> Self {
        Self(0)
    }

    /// Create a fully dirty mask.
    #[must_use]
    pub const fn all_dirty() -> Self {
        Self(u64::MAX)
    }

    /// Create from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Get the raw bits.
    #[must_use]
    pub const fn bits(&self) -> u64 {
        self.0
    }

    /// Check if completely clean (no changes).
    #[must_use]
    pub const fn is_clean(&self) -> bool {
        self.0 == 0
    }

    /// Check if any bit is dirty.
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.0 != 0
    }

    /// Mark a specific parameter slot as dirty.
    #[must_use]
    pub const fn mark_dirty(self, slot: u8) -> Self {
        if slot >= 64 {
            return self;
        }
        Self(self.0 | (1 << slot))
    }

    /// Mark a specific parameter slot as clean.
    #[must_use]
    pub const fn mark_clean(self, slot: u8) -> Self {
        if slot >= 64 {
            return self;
        }
        Self(self.0 & !(1 << slot))
    }

    /// Check if a specific slot is dirty.
    #[must_use]
    pub const fn is_slot_dirty(&self, slot: u8) -> bool {
        if slot >= 64 {
            return false;
        }
        (self.0 & (1 << slot)) != 0
    }

    /// Set all simple parameter bits dirty.
    #[must_use]
    pub const fn mark_simple_dirty(self) -> Self {
        Self(self.0 | Self::SIMPLE_MASK)
    }

    /// Set all structural parameter bits dirty.
    #[must_use]
    pub const fn mark_structural_dirty(self) -> Self {
        Self(self.0 | Self::STRUCTURAL_MASK)
    }

    /// Set all array parameter bits dirty.
    #[must_use]
    pub const fn mark_array_dirty(self) -> Self {
        Self(self.0 | Self::ARRAY_MASK)
    }

    /// Set all composition parameter bits dirty.
    #[must_use]
    pub const fn mark_composition_dirty(self) -> Self {
        Self(self.0 | Self::COMPOSITION_MASK)
    }

    /// Check if any simple parameters changed.
    #[must_use]
    pub const fn has_simple_changes(&self) -> bool {
        (self.0 & Self::SIMPLE_MASK) != 0
    }

    /// Check if any structural parameters changed.
    #[must_use]
    pub const fn has_structural_changes(&self) -> bool {
        (self.0 & Self::STRUCTURAL_MASK) != 0
    }

    /// Check if any array parameters changed.
    #[must_use]
    pub const fn has_array_changes(&self) -> bool {
        (self.0 & Self::ARRAY_MASK) != 0
    }

    /// Check if any composition parameters changed.
    #[must_use]
    pub const fn has_composition_changes(&self) -> bool {
        (self.0 & Self::COMPOSITION_MASK) != 0
    }

    /// Merge with another dirty mask (OR operation).
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Intersect with another dirty mask (AND operation).
    #[must_use]
    pub const fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Count the number of dirty bits.
    #[must_use]
    pub const fn count_dirty(&self) -> u32 {
        self.0.count_ones()
    }

    /// Get the index of the first dirty bit (or None if clean).
    #[must_use]
    pub const fn first_dirty(&self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0.trailing_zeros() as u8)
        }
    }

    /// Iterate over dirty slot indices.
    pub fn iter_dirty(&self) -> DirtyIterator {
        DirtyIterator {
            remaining: self.0,
            current: 0,
        }
    }
}

impl fmt::Debug for DirtyMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DirtyMask({:#018x})", self.0)
    }
}

impl fmt::Display for DirtyMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_clean() {
            write!(f, "clean")
        } else {
            write!(f, "{} dirty", self.count_dirty())
        }
    }
}

impl std::ops::BitOr for DirtyMask {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.merge(rhs)
    }
}

impl std::ops::BitOrAssign for DirtyMask {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = self.merge(rhs);
    }
}

impl std::ops::BitAnd for DirtyMask {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersect(rhs)
    }
}

impl std::ops::BitAndAssign for DirtyMask {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = self.intersect(rhs);
    }
}

impl std::ops::Not for DirtyMask {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

// ============================================================================
// Dirty Iterator
// ============================================================================

/// Iterator over dirty slot indices.
pub struct DirtyIterator {
    remaining: u64,
    current: u8,
}

impl Iterator for DirtyIterator {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        let zeros = self.remaining.trailing_zeros() as u8;
        let slot = self.current + zeros;
        self.remaining >>= zeros + 1;
        self.current = slot + 1;

        Some(slot)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.remaining.count_ones() as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for DirtyIterator {}

// ============================================================================
// Parameter Category
// ============================================================================

/// Category of a parameter for dirty bit assignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamCategory {
    /// Simple value (string, number, etc.) - bits 0-15
    Simple,
    /// Structural flag (affects code structure) - bits 16-31
    Structural,
    /// Array/collection value - bits 32-47
    Array,
    /// Template composition - bits 48-63
    Composition,
}

impl ParamCategory {
    /// Get the base bit offset for this category.
    #[must_use]
    pub const fn base_offset(&self) -> u8 {
        match self {
            Self::Simple => 0,
            Self::Structural => 16,
            Self::Array => 32,
            Self::Composition => 48,
        }
    }

    /// Get the slot index for a parameter in this category.
    #[must_use]
    pub const fn slot(&self, index: u8) -> u8 {
        self.base_offset() + (index & 0x0F)
    }

    /// Create a dirty mask with just this parameter dirty.
    #[must_use]
    pub const fn dirty_bit(&self, index: u8) -> DirtyMask {
        let slot = self.slot(index);
        DirtyMask::from_bits(1 << slot)
    }
}

// ============================================================================
// Dirty Tracker
// ============================================================================

/// Tracks dirty state across multiple parameters.
///
/// Used to compare current parameters against cached values
/// and determine what needs regeneration.
#[derive(Clone, Debug)]
pub struct DirtyTracker {
    /// Current dirty state
    mask: DirtyMask,
    /// Hash of last known parameter values (per slot)
    hashes: [u64; 64],
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self {
            mask: DirtyMask::clean(),
            hashes: [0u64; 64],
        }
    }
}

impl DirtyTracker {
    /// Create a new tracker (all clean).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current dirty mask.
    #[must_use]
    pub fn mask(&self) -> DirtyMask {
        self.mask
    }

    /// Check if any parameter is dirty.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.mask.is_dirty()
    }

    /// Update a parameter and mark dirty if changed.
    ///
    /// Returns true if the parameter changed.
    pub fn update(&mut self, slot: u8, new_hash: u64) -> bool {
        if slot >= 64 {
            return false;
        }

        let old_hash = self.hashes[slot as usize];
        if old_hash != new_hash {
            self.hashes[slot as usize] = new_hash;
            self.mask = self.mask.mark_dirty(slot);
            true
        } else {
            false
        }
    }

    /// Mark all parameters as clean (after successful generation).
    pub fn clear(&mut self) {
        self.mask = DirtyMask::clean();
    }

    /// Force a slot dirty without changing its hash.
    pub fn force_dirty(&mut self, slot: u8) {
        self.mask = self.mask.mark_dirty(slot);
    }

    /// Get the hash for a slot.
    #[must_use]
    pub fn get_hash(&self, slot: u8) -> u64 {
        if slot >= 64 {
            return 0;
        }
        self.hashes[slot as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_mask_clean() {
        let mask = DirtyMask::clean();
        assert!(mask.is_clean());
        assert!(!mask.is_dirty());
        assert_eq!(mask.count_dirty(), 0);
    }

    #[test]
    fn test_dirty_mask_mark() {
        let mask = DirtyMask::clean().mark_dirty(0).mark_dirty(5);
        assert!(mask.is_dirty());
        assert!(mask.is_slot_dirty(0));
        assert!(mask.is_slot_dirty(5));
        assert!(!mask.is_slot_dirty(1));
        assert_eq!(mask.count_dirty(), 2);
    }

    #[test]
    fn test_dirty_mask_categories() {
        let mask = DirtyMask::clean().mark_simple_dirty().mark_structural_dirty();

        assert!(mask.has_simple_changes());
        assert!(mask.has_structural_changes());
        assert!(!mask.has_array_changes());
        assert!(!mask.has_composition_changes());
    }

    #[test]
    fn test_dirty_iterator() {
        let mask = DirtyMask::clean().mark_dirty(0).mark_dirty(3).mark_dirty(7);

        let dirty: Vec<u8> = mask.iter_dirty().collect();
        assert_eq!(dirty, vec![0, 3, 7]);
    }

    #[test]
    fn test_param_category_slots() {
        assert_eq!(ParamCategory::Simple.slot(0), 0);
        assert_eq!(ParamCategory::Simple.slot(5), 5);
        assert_eq!(ParamCategory::Structural.slot(0), 16);
        assert_eq!(ParamCategory::Array.slot(3), 35);
        assert_eq!(ParamCategory::Composition.slot(1), 49);
    }

    #[test]
    fn test_dirty_tracker() {
        let mut tracker = DirtyTracker::new();

        // First update always marks dirty
        assert!(tracker.update(0, 123));
        assert!(tracker.is_dirty());

        // Same hash doesn't mark dirty
        tracker.clear();
        assert!(!tracker.update(0, 123));
        assert!(!tracker.is_dirty());

        // Different hash marks dirty
        assert!(tracker.update(0, 456));
        assert!(tracker.is_dirty());
    }

    #[test]
    fn test_dirty_mask_ops() {
        let a = DirtyMask::clean().mark_dirty(0);
        let b = DirtyMask::clean().mark_dirty(1);

        let merged = a | b;
        assert!(merged.is_slot_dirty(0));
        assert!(merged.is_slot_dirty(1));

        let intersected = merged & a;
        assert!(intersected.is_slot_dirty(0));
        assert!(!intersected.is_slot_dirty(1));
    }

    #[test]
    fn test_first_dirty() {
        let mask = DirtyMask::clean();
        assert_eq!(mask.first_dirty(), None);

        let mask = DirtyMask::clean().mark_dirty(5);
        assert_eq!(mask.first_dirty(), Some(5));

        let mask = DirtyMask::clean().mark_dirty(10).mark_dirty(5);
        assert_eq!(mask.first_dirty(), Some(5));
    }
}
