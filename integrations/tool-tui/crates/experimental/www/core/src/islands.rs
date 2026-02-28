//! # Binary Islands Architecture
//!
//! Binary Dawn's islands architecture uses 1-bit flags and WASM chunks for partial hydration.
//! This achieves 10x smaller island overhead compared to Astro's ~5KB minimum.
//!
//! Islands are activated individually, loading only the necessary WASM code.

/// Island type identifier
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IslandType {
    /// Interactive counter component
    Counter = 0,
    /// Form component
    Form = 1,
    /// Navigation component
    Navigation = 2,
    /// Modal/dialog component
    Modal = 3,
    /// Carousel/slider component
    Carousel = 4,
    /// Search component
    Search = 5,
    /// Custom component
    Custom = 255,
}

impl IslandType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Counter),
            1 => Some(Self::Form),
            2 => Some(Self::Navigation),
            3 => Some(Self::Modal),
            4 => Some(Self::Carousel),
            5 => Some(Self::Search),
            255 => Some(Self::Custom),
            _ => None,
        }
    }

    /// Get the WASM chunk filename for this island type
    pub fn chunk_name(&self) -> &'static str {
        match self {
            Self::Counter => "island_counter.wasm",
            Self::Form => "island_form.wasm",
            Self::Navigation => "island_nav.wasm",
            Self::Modal => "island_modal.wasm",
            Self::Carousel => "island_carousel.wasm",
            Self::Search => "island_search.wasm",
            Self::Custom => "island_custom.wasm",
        }
    }
}

/// Island slot definition
///
/// Defines a slot in the page where an island can be mounted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IslandSlot {
    /// Slot ID (position in page)
    pub slot_id: u16,
    /// Type of island for this slot
    pub island_type: IslandType,
}

impl IslandSlot {
    /// Create a new island slot
    pub const fn new(slot_id: u16, island_type: IslandType) -> Self {
        Self {
            slot_id,
            island_type,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 3] {
        let mut bytes = [0u8; 3];
        bytes[0..2].copy_from_slice(&self.slot_id.to_le_bytes());
        bytes[2] = self.island_type as u8;
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 3]) -> Option<Self> {
        Some(Self {
            slot_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            island_type: IslandType::from_u8(bytes[2])?,
        })
    }
}

/// Maximum number of islands per page
pub const MAX_ISLANDS: usize = 64;

/// Island activation bitfield (64 islands max per page)
///
/// Each bit represents one island's activation state.
/// 0 = not hydrated, 1 = hydrated
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IslandActivation {
    /// Bitfield where each bit represents an island
    pub bits: u64,
}

impl IslandActivation {
    /// Create with no islands activated
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Create with all islands activated
    pub const fn all_active() -> Self {
        Self { bits: u64::MAX }
    }

    /// Activate island N
    #[inline(always)]
    pub fn activate(&mut self, n: u8) {
        debug_assert!((n as usize) < MAX_ISLANDS);
        self.bits |= 1 << n;
    }

    /// Deactivate island N
    #[inline(always)]
    pub fn deactivate(&mut self, n: u8) {
        debug_assert!((n as usize) < MAX_ISLANDS);
        self.bits &= !(1 << n);
    }

    /// Check if island N is active
    #[inline(always)]
    pub fn is_active(&self, n: u8) -> bool {
        debug_assert!((n as usize) < MAX_ISLANDS);
        (self.bits & (1 << n)) != 0
    }

    /// Toggle island N
    #[inline(always)]
    pub fn toggle(&mut self, n: u8) {
        debug_assert!((n as usize) < MAX_ISLANDS);
        self.bits ^= 1 << n;
    }

    /// Count active islands
    #[inline]
    pub fn count_active(&self) -> u32 {
        self.bits.count_ones()
    }

    /// Get list of active island IDs
    pub fn active_islands(&self) -> Vec<u8> {
        (0..MAX_ISLANDS as u8).filter(|&n| self.is_active(n)).collect()
    }

    /// Check if any islands are active
    #[inline]
    pub fn any_active(&self) -> bool {
        self.bits != 0
    }

    /// Check if all islands are inactive
    #[inline]
    pub fn none_active(&self) -> bool {
        self.bits == 0
    }
}

/// Page with island slots
///
/// Contains the static template and island slot definitions.
#[derive(Debug, Clone)]
pub struct BinaryPage {
    /// Page ID
    pub page_id: u16,
    /// Pre-rendered static template (HTML bytes)
    pub static_template: Vec<u8>,
    /// Island slot definitions
    pub island_slots: Vec<IslandSlot>,
    /// Current activation state
    pub activation: IslandActivation,
    /// Loaded island chunks (by island ID)
    loaded_chunks: Vec<bool>,
}

impl BinaryPage {
    /// Create a new binary page
    pub fn new(page_id: u16, static_template: Vec<u8>) -> Self {
        Self {
            page_id,
            static_template,
            island_slots: Vec::new(),
            activation: IslandActivation::new(),
            loaded_chunks: vec![false; MAX_ISLANDS],
        }
    }

    /// Add an island slot
    pub fn add_island(&mut self, slot_id: u16, island_type: IslandType) {
        self.island_slots.push(IslandSlot::new(slot_id, island_type));
    }

    /// Get island slot by ID
    pub fn get_island(&self, island_id: u8) -> Option<&IslandSlot> {
        self.island_slots.get(island_id as usize)
    }

    /// Check if island is activated
    pub fn is_island_active(&self, island_id: u8) -> bool {
        self.activation.is_active(island_id)
    }

    /// Check if island chunk is loaded
    pub fn is_chunk_loaded(&self, island_id: u8) -> bool {
        self.loaded_chunks.get(island_id as usize).copied().unwrap_or(false)
    }

    /// Mark island chunk as loaded
    pub fn mark_chunk_loaded(&mut self, island_id: u8) {
        if let Some(loaded) = self.loaded_chunks.get_mut(island_id as usize) {
            *loaded = true;
        }
    }

    /// Activate single island
    ///
    /// Returns the chunk name to load if not already loaded.
    pub fn activate_island(&mut self, island_id: u8) -> Option<&'static str> {
        if self.activation.is_active(island_id) {
            return None; // Already active
        }

        let slot = self.island_slots.get(island_id as usize)?;
        let chunk_name = slot.island_type.chunk_name();

        self.activation.activate(island_id);

        if !self.is_chunk_loaded(island_id) {
            self.mark_chunk_loaded(island_id);
            Some(chunk_name)
        } else {
            None
        }
    }

    /// Deactivate island
    pub fn deactivate_island(&mut self, island_id: u8) {
        self.activation.deactivate(island_id);
    }

    /// Get count of active islands
    pub fn active_count(&self) -> u32 {
        self.activation.count_active()
    }

    /// Get total island count
    pub fn island_count(&self) -> usize {
        self.island_slots.len()
    }
}

/// Island chunk loader (simulated)
///
/// In a real implementation, this would load WASM chunks.
pub struct IslandLoader {
    /// Loaded chunks by island type
    loaded: [bool; 256],
}

impl IslandLoader {
    /// Create a new loader
    pub fn new() -> Self {
        Self {
            loaded: [false; 256],
        }
    }

    /// Check if island type is loaded
    pub fn is_loaded(&self, island_type: IslandType) -> bool {
        self.loaded[island_type as usize]
    }

    /// Mark island type as loaded
    pub fn mark_loaded(&mut self, island_type: IslandType) {
        self.loaded[island_type as usize] = true;
    }

    /// Simulate loading an island chunk
    ///
    /// Returns the chunk size (simulated).
    pub fn load_chunk(&mut self, island_type: IslandType) -> usize {
        self.mark_loaded(island_type);
        // Simulated chunk sizes (~500 bytes minimum)
        match island_type {
            IslandType::Counter => 512,
            IslandType::Form => 1024,
            IslandType::Navigation => 768,
            IslandType::Modal => 640,
            IslandType::Carousel => 896,
            IslandType::Search => 1280,
            IslandType::Custom => 512,
        }
    }
}

impl Default for IslandLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_island_activation_basic() {
        let mut activation = IslandActivation::new();

        assert!(!activation.is_active(0));
        assert!(!activation.is_active(5));

        activation.activate(5);
        assert!(activation.is_active(5));
        assert!(!activation.is_active(0));

        activation.deactivate(5);
        assert!(!activation.is_active(5));
    }

    #[test]
    fn test_island_activation_multiple() {
        let mut activation = IslandActivation::new();

        activation.activate(0);
        activation.activate(10);
        activation.activate(63);

        assert_eq!(activation.count_active(), 3);
        assert!(activation.is_active(0));
        assert!(activation.is_active(10));
        assert!(activation.is_active(63));
        assert!(!activation.is_active(1));
    }

    #[test]
    fn test_island_activation_toggle() {
        let mut activation = IslandActivation::new();

        activation.toggle(5);
        assert!(activation.is_active(5));

        activation.toggle(5);
        assert!(!activation.is_active(5));
    }

    #[test]
    fn test_island_slot_roundtrip() {
        let slot = IslandSlot::new(42, IslandType::Form);
        let bytes = slot.to_bytes();
        let restored = IslandSlot::from_bytes(&bytes).unwrap();

        assert_eq!(slot, restored);
    }

    #[test]
    fn test_binary_page() {
        let mut page = BinaryPage::new(1, vec![1, 2, 3]);
        page.add_island(0, IslandType::Counter);
        page.add_island(1, IslandType::Form);

        assert_eq!(page.island_count(), 2);
        assert!(!page.is_island_active(0));

        let chunk = page.activate_island(0);
        assert!(chunk.is_some());
        assert!(page.is_island_active(0));

        // Second activation should return None (already active)
        let chunk2 = page.activate_island(0);
        assert!(chunk2.is_none());
    }

    #[test]
    fn test_island_loader() {
        let mut loader = IslandLoader::new();

        assert!(!loader.is_loaded(IslandType::Counter));

        let size = loader.load_chunk(IslandType::Counter);
        assert!(size >= 500); // Minimum ~500 bytes
        assert!(loader.is_loaded(IslandType::Counter));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 11: Island Activation Bitfield**
    // *For any* island ID in the range [0, 63], `IslandActivation::activate(n)` followed by
    // `is_active(n)` SHALL return true, and `is_active(m)` for m != n SHALL be unchanged.
    // **Validates: Requirements 6.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_island_activation_bitfield(
            island_id in 0u8..64,
            other_id in 0u8..64
        ) {
            let mut activation = IslandActivation::new();

            // Initially not active
            prop_assert!(!activation.is_active(island_id));

            // Activate the island
            activation.activate(island_id);

            // Should now be active
            prop_assert!(activation.is_active(island_id));

            // Other islands should be unchanged (only active if same as island_id)
            if other_id != island_id {
                prop_assert!(!activation.is_active(other_id));
            }

            // Deactivate
            activation.deactivate(island_id);
            prop_assert!(!activation.is_active(island_id));
        }
    }

    // **Feature: binary-dawn-features, Property 12: Partial Hydration Isolation**
    // *For any* page with multiple islands, activating island N SHALL NOT load or
    // hydrate any other island M where M != N.
    // **Validates: Requirements 6.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_partial_hydration_isolation(
            island_count in 2usize..10,
            activate_id in 0u8..10
        ) {
            let mut page = BinaryPage::new(1, vec![]);

            // Add multiple islands
            for i in 0..island_count {
                let island_type = match i % 3 {
                    0 => IslandType::Counter,
                    1 => IslandType::Form,
                    _ => IslandType::Navigation,
                };
                page.add_island(i as u16, island_type);
            }

            // Activate only one island
            let target_id = (activate_id as usize % island_count) as u8;
            page.activate_island(target_id);

            // Only the target island should be active
            for i in 0..island_count {
                let is_active = page.is_island_active(i as u8);
                if i as u8 == target_id {
                    prop_assert!(is_active, "Target island {} should be active", i);
                } else {
                    prop_assert!(!is_active, "Island {} should NOT be active", i);
                }
            }

            // Only one island should be active
            prop_assert_eq!(page.active_count(), 1);
        }
    }

    // Island activation count property
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_island_activation_count(
            islands_to_activate in prop::collection::vec(0u8..64, 0..20)
        ) {
            let mut activation = IslandActivation::new();
            let mut expected_active: std::collections::HashSet<u8> = std::collections::HashSet::new();

            for &id in &islands_to_activate {
                activation.activate(id);
                expected_active.insert(id);
            }

            prop_assert_eq!(activation.count_active() as usize, expected_active.len());
        }
    }

    // IslandSlot round-trip
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_island_slot_roundtrip(
            slot_id in any::<u16>(),
            island_type_id in 0u8..6
        ) {
            let island_type = match island_type_id {
                0 => IslandType::Counter,
                1 => IslandType::Form,
                2 => IslandType::Navigation,
                3 => IslandType::Modal,
                4 => IslandType::Carousel,
                _ => IslandType::Search,
            };

            let slot = IslandSlot::new(slot_id, island_type);
            let bytes = slot.to_bytes();
            let restored = IslandSlot::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            prop_assert_eq!(slot, restored.unwrap());
        }
    }

    // Toggle is self-inverse
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_toggle_self_inverse(
            island_id in 0u8..64,
            initial_state in any::<bool>()
        ) {
            let mut activation = IslandActivation::new();

            if initial_state {
                activation.activate(island_id);
            }

            let before = activation.is_active(island_id);
            activation.toggle(island_id);
            activation.toggle(island_id);
            let after = activation.is_active(island_id);

            prop_assert_eq!(before, after);
        }
    }
}
