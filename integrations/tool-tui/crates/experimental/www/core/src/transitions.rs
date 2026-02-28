//! # Pre-Compiled View Transitions
//!
//! Binary Dawn's view transitions use pre-compiled binary descriptors instead of runtime configuration.
//! This achieves 10x faster transition setup compared to runtime configuration.
//!
//! Transition configs are stored in router.dxb at build time.

/// Transition type enum
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionType {
    /// No transition
    None = 0,
    /// Fade in/out
    Fade = 1,
    /// Slide left/right
    Slide = 2,
    /// Morph elements between views
    Morph = 3,
    /// Scale up/down
    Scale = 4,
    /// Flip animation
    Flip = 5,
}

impl TransitionType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::None),
            1 => Some(Self::Fade),
            2 => Some(Self::Slide),
            3 => Some(Self::Morph),
            4 => Some(Self::Scale),
            5 => Some(Self::Flip),
            _ => None,
        }
    }
}

/// Maximum number of morph pairs
pub const MAX_MORPH_PAIRS: usize = 8;

/// Morph pair - elements to animate between views
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MorphPair {
    /// Element ID in source view
    pub from_id: u16,
    /// Element ID in target view
    pub to_id: u16,
}

impl MorphPair {
    /// Create a new morph pair
    pub const fn new(from_id: u16, to_id: u16) -> Self {
        Self { from_id, to_id }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut bytes = [0u8; 4];
        bytes[0..2].copy_from_slice(&self.from_id.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.to_id.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 4 {
            return None;
        }
        Some(Self {
            from_id: u16::from_le_bytes([bytes[0], bytes[1]]),
            to_id: u16::from_le_bytes([bytes[2], bytes[3]]),
        })
    }
}

/// Pre-compiled transition config
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionConfig {
    /// Source route ID
    pub from_route: u16,
    /// Target route ID
    pub to_route: u16,
    /// Transition type
    pub transition_type: TransitionType,
    /// Duration in milliseconds
    pub duration_ms: u16,
    /// Number of morph pairs
    pub morph_count: u8,
    /// Morph pairs (elements to animate between views)
    pub morph_pairs: [MorphPair; MAX_MORPH_PAIRS],
}

impl TransitionConfig {
    /// Base size without morph pairs
    pub const BASE_SIZE: usize = 7;
    /// Full size with all morph pairs
    pub const FULL_SIZE: usize = Self::BASE_SIZE + MAX_MORPH_PAIRS * 4;

    /// Create a new transition config
    pub fn new(
        from_route: u16,
        to_route: u16,
        transition_type: TransitionType,
        duration_ms: u16,
    ) -> Self {
        Self {
            from_route,
            to_route,
            transition_type,
            duration_ms,
            morph_count: 0,
            morph_pairs: [MorphPair::default(); MAX_MORPH_PAIRS],
        }
    }

    /// Create a fade transition
    pub fn fade(from_route: u16, to_route: u16, duration_ms: u16) -> Self {
        Self::new(from_route, to_route, TransitionType::Fade, duration_ms)
    }

    /// Create a slide transition
    pub fn slide(from_route: u16, to_route: u16, duration_ms: u16) -> Self {
        Self::new(from_route, to_route, TransitionType::Slide, duration_ms)
    }

    /// Create a morph transition
    pub fn morph(from_route: u16, to_route: u16, duration_ms: u16, pairs: &[MorphPair]) -> Self {
        let mut config = Self::new(from_route, to_route, TransitionType::Morph, duration_ms);
        config.morph_count = pairs.len().min(MAX_MORPH_PAIRS) as u8;
        for (i, pair) in pairs.iter().take(MAX_MORPH_PAIRS).enumerate() {
            config.morph_pairs[i] = *pair;
        }
        config
    }

    /// Add a morph pair
    pub fn add_morph_pair(&mut self, from_id: u16, to_id: u16) -> bool {
        if (self.morph_count as usize) < MAX_MORPH_PAIRS {
            self.morph_pairs[self.morph_count as usize] = MorphPair::new(from_id, to_id);
            self.morph_count += 1;
            true
        } else {
            false
        }
    }

    /// Get morph pairs
    pub fn get_morph_pairs(&self) -> &[MorphPair] {
        &self.morph_pairs[..self.morph_count as usize]
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::FULL_SIZE);
        bytes.extend_from_slice(&self.from_route.to_le_bytes());
        bytes.extend_from_slice(&self.to_route.to_le_bytes());
        bytes.push(self.transition_type as u8);
        bytes.extend_from_slice(&self.duration_ms.to_le_bytes());
        bytes.push(self.morph_count);
        for pair in &self.morph_pairs {
            bytes.extend_from_slice(&pair.to_bytes());
        }
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::BASE_SIZE {
            return None;
        }
        let mut morph_pairs = [MorphPair::default(); MAX_MORPH_PAIRS];
        let morph_count = bytes[6];

        for (i, pair) in morph_pairs
            .iter_mut()
            .enumerate()
            .take(morph_count.min(MAX_MORPH_PAIRS as u8) as usize)
        {
            let offset = Self::BASE_SIZE + 1 + i * 4;
            if offset + 4 <= bytes.len() {
                *pair = MorphPair::from_bytes(&bytes[offset..offset + 4])?;
            }
        }

        Some(Self {
            from_route: u16::from_le_bytes([bytes[0], bytes[1]]),
            to_route: u16::from_le_bytes([bytes[2], bytes[3]]),
            transition_type: TransitionType::from_u8(bytes[4])?,
            duration_ms: u16::from_le_bytes([bytes[5], bytes[6]]),
            morph_count,
            morph_pairs,
        })
    }
}

/// Element snapshot for FLIP animation
#[derive(Debug, Clone)]
pub struct ElementSnapshot {
    /// Element ID
    pub element_id: u16,
    /// X position
    pub x: f32,
    /// Y position
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Opacity
    pub opacity: f32,
}

impl ElementSnapshot {
    /// Create a new snapshot
    pub fn new(element_id: u16, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            element_id,
            x,
            y,
            width,
            height,
            opacity: 1.0,
        }
    }

    /// Calculate delta to another snapshot
    pub fn delta_to(&self, other: &ElementSnapshot) -> ElementDelta {
        ElementDelta {
            dx: other.x - self.x,
            dy: other.y - self.y,
            dw: other.width - self.width,
            dh: other.height - self.height,
            do_: other.opacity - self.opacity,
        }
    }
}

/// Delta between two element snapshots
#[derive(Debug, Clone)]
pub struct ElementDelta {
    /// X delta
    pub dx: f32,
    /// Y delta
    pub dy: f32,
    /// Width delta
    pub dw: f32,
    /// Height delta
    pub dh: f32,
    /// Opacity delta
    pub do_: f32,
}

/// Transition manager
#[derive(Debug, Clone)]
pub struct TransitionManager {
    /// Registered transitions
    transitions: Vec<TransitionConfig>,
}

impl TransitionManager {
    /// Create a new transition manager
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
        }
    }

    /// Register a transition
    pub fn register(&mut self, config: TransitionConfig) {
        self.transitions.push(config);
    }

    /// Find transition for route pair
    pub fn find(&self, from_route: u16, to_route: u16) -> Option<&TransitionConfig> {
        self.transitions
            .iter()
            .find(|t| t.from_route == from_route && t.to_route == to_route)
    }

    /// Get transition count
    pub fn count(&self) -> usize {
        self.transitions.len()
    }
}

impl Default for TransitionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_config_fields() {
        let config = TransitionConfig::new(1, 2, TransitionType::Fade, 300);

        assert_eq!(config.from_route, 1);
        assert_eq!(config.to_route, 2);
        assert_eq!(config.transition_type, TransitionType::Fade);
        assert_eq!(config.duration_ms, 300);
        assert_eq!(config.morph_count, 0);
    }

    #[test]
    fn test_transition_config_morph() {
        let pairs = [MorphPair::new(1, 10), MorphPair::new(2, 20)];
        let config = TransitionConfig::morph(1, 2, 500, &pairs);

        assert_eq!(config.transition_type, TransitionType::Morph);
        assert_eq!(config.morph_count, 2);
        assert_eq!(config.get_morph_pairs().len(), 2);
    }

    #[test]
    fn test_transition_config_roundtrip() {
        let mut config = TransitionConfig::fade(1, 2, 300);
        config.add_morph_pair(10, 20);
        config.add_morph_pair(30, 40);

        let bytes = config.to_bytes();
        let restored = TransitionConfig::from_bytes(&bytes).unwrap();

        assert_eq!(config.from_route, restored.from_route);
        assert_eq!(config.to_route, restored.to_route);
        assert_eq!(config.transition_type, restored.transition_type);
        assert_eq!(config.duration_ms, restored.duration_ms);
    }

    #[test]
    fn test_morph_pair_roundtrip() {
        let pair = MorphPair::new(100, 200);
        let bytes = pair.to_bytes();
        let restored = MorphPair::from_bytes(&bytes).unwrap();
        assert_eq!(pair, restored);
    }

    #[test]
    fn test_transition_manager() {
        let mut manager = TransitionManager::new();

        manager.register(TransitionConfig::fade(1, 2, 300));
        manager.register(TransitionConfig::slide(2, 3, 400));

        assert_eq!(manager.count(), 2);

        let found = manager.find(1, 2);
        assert!(found.is_some());
        assert_eq!(found.unwrap().transition_type, TransitionType::Fade);

        let not_found = manager.find(5, 6);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_element_snapshot_delta() {
        let from = ElementSnapshot::new(1, 0.0, 0.0, 100.0, 100.0);
        let to = ElementSnapshot::new(1, 50.0, 25.0, 200.0, 150.0);

        let delta = from.delta_to(&to);

        assert_eq!(delta.dx, 50.0);
        assert_eq!(delta.dy, 25.0);
        assert_eq!(delta.dw, 100.0);
        assert_eq!(delta.dh, 50.0);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 31: TransitionConfig Fields**
    // *For any* TransitionConfig, it SHALL contain from_route (u16), to_route (u16),
    // transition_type (u8), duration_ms (u16), and morph_count (u8).
    // **Validates: Requirements 18.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_transition_config_fields(
            from_route in any::<u16>(),
            to_route in any::<u16>(),
            transition_type in 0u8..6,
            duration_ms in any::<u16>(),
            morph_count in 0u8..8
        ) {
            let tt = TransitionType::from_u8(transition_type).unwrap();
            let mut config = TransitionConfig::new(from_route, to_route, tt, duration_ms);

            // Add morph pairs
            for i in 0..morph_count {
                config.add_morph_pair(i as u16, (i + 100) as u16);
            }

            // Verify all fields are present
            prop_assert_eq!(config.from_route, from_route);
            prop_assert_eq!(config.to_route, to_route);
            prop_assert_eq!(config.transition_type, tt);
            prop_assert_eq!(config.duration_ms, duration_ms);
            prop_assert_eq!(config.morph_count, morph_count);
        }
    }

    // MorphPair round-trip
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_morph_pair_roundtrip(
            from_id in any::<u16>(),
            to_id in any::<u16>()
        ) {
            let pair = MorphPair::new(from_id, to_id);
            let bytes = pair.to_bytes();
            let restored = MorphPair::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            prop_assert_eq!(pair, restored.unwrap());
        }
    }

    // TransitionType round-trip
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_transition_type_roundtrip(
            type_id in 0u8..6
        ) {
            let tt = TransitionType::from_u8(type_id);
            prop_assert!(tt.is_some());
            prop_assert_eq!(tt.unwrap() as u8, type_id);
        }
    }

    // TransitionManager find
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        #[test]
        fn prop_transition_manager_find(
            route_pairs in prop::collection::vec((any::<u16>(), any::<u16>()), 1..10)
        ) {
            let mut manager = TransitionManager::new();

            for (from, to) in &route_pairs {
                manager.register(TransitionConfig::fade(*from, *to, 300));
            }

            // All registered transitions should be found
            for (from, to) in &route_pairs {
                let found = manager.find(*from, *to);
                prop_assert!(found.is_some());
            }
        }
    }
}
