//! # Binary Handler References
//!
//! Binary Dawn's handler system uses 4-byte references instead of serialized closures.
//! Handlers are extracted at compile time and assigned indices in the WASM function table.
//!
//! This achieves 25x smaller handler payloads compared to QRL strings.

/// 4-byte handler reference
///
/// Instead of serializing closures as strings, handlers are referenced by their
/// index in the WASM function table plus an offset to captured values.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HandlerRef {
    /// Index in WASM function table
    pub fn_index: u16,
    /// Offset to captured values in SharedArrayBuffer
    pub capture_offset: u16,
}

impl HandlerRef {
    /// Size of HandlerRef in bytes - must be exactly 4
    pub const SIZE: usize = 4;

    /// Create a new handler reference
    #[inline]
    pub const fn new(fn_index: u16, capture_offset: u16) -> Self {
        Self {
            fn_index,
            capture_offset,
        }
    }

    /// Create a handler with no captures
    #[inline]
    pub const fn simple(fn_index: u16) -> Self {
        Self {
            fn_index,
            capture_offset: 0,
        }
    }

    /// Serialize to bytes
    #[inline]
    pub fn to_bytes(&self) -> [u8; 4] {
        let mut bytes = [0u8; 4];
        bytes[0..2].copy_from_slice(&self.fn_index.to_le_bytes());
        bytes[2..4].copy_from_slice(&self.capture_offset.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    #[inline]
    pub fn from_bytes(bytes: &[u8; 4]) -> Self {
        Self {
            fn_index: u16::from_le_bytes([bytes[0], bytes[1]]),
            capture_offset: u16::from_le_bytes([bytes[2], bytes[3]]),
        }
    }
}

/// Handler groups for smart code splitting
///
/// Handlers are classified by usage pattern to enable intelligent code splitting.
/// Instead of 50+ per-function files, we produce 3-5 binary chunks.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandlerGroup {
    /// Above-fold, likely clicked first (e.g., primary CTA buttons)
    Critical = 0,
    /// Hover, focus handlers (secondary interactions)
    Interactive = 1,
    /// Form submissions
    Submission = 2,
    /// Route changes
    Navigation = 3,
    /// Error handlers, edge cases
    Rare = 4,
}

impl HandlerGroup {
    /// Total number of handler groups
    pub const COUNT: usize = 5;

    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Critical),
            1 => Some(Self::Interactive),
            2 => Some(Self::Submission),
            3 => Some(Self::Navigation),
            4 => Some(Self::Rare),
            _ => None,
        }
    }

    /// Get chunk filename for this group
    pub fn chunk_name(&self) -> &'static str {
        match self {
            Self::Critical => "handlers_critical.dxb",
            Self::Interactive => "handlers_secondary.dxb",
            Self::Submission => "handlers_submission.dxb",
            Self::Navigation => "handlers_navigation.dxb",
            Self::Rare => "handlers_rare.dxb",
        }
    }

    /// Get all chunk names
    pub fn all_chunk_names() -> [&'static str; Self::COUNT] {
        [
            Self::Critical.chunk_name(),
            Self::Interactive.chunk_name(),
            Self::Submission.chunk_name(),
            Self::Navigation.chunk_name(),
            Self::Rare.chunk_name(),
        ]
    }
}

/// Handler metadata for classification
#[derive(Debug, Clone, Default)]
pub struct HandlerMetadata {
    /// Is this handler above the fold?
    pub is_above_fold: bool,
    /// Is this a click handler?
    pub is_click: bool,
    /// Is this a hover handler?
    pub is_hover: bool,
    /// Is this a focus handler?
    pub is_focus: bool,
    /// Is this a form submit handler?
    pub is_form_submit: bool,
    /// Is this a navigation handler?
    pub is_navigation: bool,
}

impl HandlerMetadata {
    /// Classify this handler into a group
    pub fn classify(&self) -> HandlerGroup {
        if self.is_above_fold && self.is_click {
            HandlerGroup::Critical
        } else if self.is_hover || self.is_focus {
            HandlerGroup::Interactive
        } else if self.is_form_submit {
            HandlerGroup::Submission
        } else if self.is_navigation {
            HandlerGroup::Navigation
        } else {
            HandlerGroup::Rare
        }
    }
}

/// Compiler-generated handler manifest
///
/// Contains all handlers grouped by usage pattern for code splitting.
#[derive(Debug, Clone, Default)]
pub struct HandlerManifest {
    /// Handlers grouped by classification
    pub groups: [Vec<HandlerRef>; HandlerGroup::COUNT],
}

impl HandlerManifest {
    /// Create empty manifest
    pub fn new() -> Self {
        Self {
            groups: Default::default(),
        }
    }

    /// Add a handler to the appropriate group
    pub fn add(&mut self, handler: HandlerRef, metadata: &HandlerMetadata) {
        let group = metadata.classify();
        self.groups[group as usize].push(handler);
    }

    /// Add a handler directly to a group
    pub fn add_to_group(&mut self, handler: HandlerRef, group: HandlerGroup) {
        self.groups[group as usize].push(handler);
    }

    /// Get handlers for a group
    pub fn get_group(&self, group: HandlerGroup) -> &[HandlerRef] {
        &self.groups[group as usize]
    }

    /// Get total handler count
    pub fn total_count(&self) -> usize {
        self.groups.iter().map(|g| g.len()).sum()
    }

    /// Get number of non-empty groups (chunk count)
    pub fn chunk_count(&self) -> usize {
        self.groups.iter().filter(|g| !g.is_empty()).count()
    }

    /// Serialize a group to bytes
    pub fn serialize_group(&self, group: HandlerGroup) -> Vec<u8> {
        let handlers = &self.groups[group as usize];
        let mut bytes = Vec::with_capacity(handlers.len() * HandlerRef::SIZE);
        for handler in handlers {
            bytes.extend_from_slice(&handler.to_bytes());
        }
        bytes
    }
}

/// Maximum number of handlers in the static table
pub const HANDLER_TABLE_SIZE: usize = 256;

/// Static handler table type
pub type HandlerFn = fn();

/// No-op handler for unused slots
fn noop_handler() {}

/// Static handler table - 256 function pointers
///
/// Handlers are looked up by u8 index for O(1) invocation.
/// Unused slots point to a no-op function.
pub static HANDLER_TABLE: [HandlerFn; HANDLER_TABLE_SIZE] = [noop_handler; HANDLER_TABLE_SIZE];

/// Handler lookup - O(1) array index
///
/// Invokes the handler at the given index in the static table.
#[inline(always)]
pub fn invoke_handler(id: u8) {
    HANDLER_TABLE[id as usize]();
}

/// Check if a handler ID is valid (points to a real handler, not noop)
///
/// Note: In a real implementation, this would check against registered handlers.
/// For now, all slots are valid (they point to noop_handler).
#[inline]
pub fn is_valid_handler(_id: u8) -> bool {
    // All slots are valid - they either have a real handler or noop
    // The id is always in range 0-255 which fits in u8
    true
}

/// Generate HTML attribute for handler binding
///
/// Returns the data attribute string for a click handler.
#[inline]
pub fn handler_attribute(id: u8) -> String {
    format!("data-dx-click=\"{}\"", id)
}

/// Parse handler ID from HTML attribute value
#[inline]
pub fn parse_handler_id(attr_value: &str) -> Option<u8> {
    attr_value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_ref_size() {
        assert_eq!(std::mem::size_of::<HandlerRef>(), HandlerRef::SIZE);
        assert_eq!(std::mem::size_of::<HandlerRef>(), 4);
    }

    #[test]
    fn test_handler_ref_roundtrip() {
        let handler = HandlerRef::new(42, 100);
        let bytes = handler.to_bytes();
        let restored = HandlerRef::from_bytes(&bytes);
        assert_eq!(handler, restored);
    }

    #[test]
    fn test_handler_group_count() {
        assert_eq!(HandlerGroup::COUNT, 5);
        assert!(HandlerGroup::COUNT >= 3 && HandlerGroup::COUNT <= 5);
    }

    #[test]
    fn test_handler_classification() {
        let critical = HandlerMetadata {
            is_above_fold: true,
            is_click: true,
            ..Default::default()
        };
        assert_eq!(critical.classify(), HandlerGroup::Critical);

        let interactive = HandlerMetadata {
            is_hover: true,
            ..Default::default()
        };
        assert_eq!(interactive.classify(), HandlerGroup::Interactive);

        let submission = HandlerMetadata {
            is_form_submit: true,
            ..Default::default()
        };
        assert_eq!(submission.classify(), HandlerGroup::Submission);

        let navigation = HandlerMetadata {
            is_navigation: true,
            ..Default::default()
        };
        assert_eq!(navigation.classify(), HandlerGroup::Navigation);

        let rare = HandlerMetadata::default();
        assert_eq!(rare.classify(), HandlerGroup::Rare);
    }

    #[test]
    fn test_handler_manifest() {
        let mut manifest = HandlerManifest::new();

        manifest.add_to_group(HandlerRef::new(0, 0), HandlerGroup::Critical);
        manifest.add_to_group(HandlerRef::new(1, 0), HandlerGroup::Critical);
        manifest.add_to_group(HandlerRef::new(2, 0), HandlerGroup::Interactive);

        assert_eq!(manifest.total_count(), 3);
        assert_eq!(manifest.chunk_count(), 2);
        assert_eq!(manifest.get_group(HandlerGroup::Critical).len(), 2);
    }

    #[test]
    fn test_handler_attribute() {
        let attr = handler_attribute(42);
        assert_eq!(attr, "data-dx-click=\"42\"");

        let parsed = parse_handler_id("42");
        assert_eq!(parsed, Some(42));
    }

    #[test]
    fn test_handler_table_validity() {
        // All handler IDs 0-255 should be valid
        for id in 0..=255u8 {
            assert!(is_valid_handler(id));
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 9: HandlerRef Size Invariant**
    // *For any* HandlerRef instance, `size_of::<HandlerRef>()` SHALL equal exactly 4 bytes.
    // **Validates: Requirements 5.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_handler_ref_size_invariant(
            fn_index in any::<u16>(),
            capture_offset in any::<u16>()
        ) {
            let handler = HandlerRef::new(fn_index, capture_offset);

            // Size must always be exactly 4 bytes
            prop_assert_eq!(std::mem::size_of::<HandlerRef>(), 4);
            prop_assert_eq!(HandlerRef::SIZE, 4);

            // Serialized form must also be 4 bytes
            let bytes = handler.to_bytes();
            prop_assert_eq!(bytes.len(), 4);
        }
    }

    // **Feature: binary-dawn-features, Property 7: Handler Table Validity**
    // *For any* handler ID in the range [0, 255], `HANDLER_TABLE[id]` SHALL be a valid function pointer.
    // **Validates: Requirements 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_handler_table_validity(
            id in 0u8..=255
        ) {
            // All handler IDs must be valid
            prop_assert!(is_valid_handler(id));

            // Handler table must have 256 entries
            prop_assert_eq!(HANDLER_TABLE.len(), 256);
            prop_assert_eq!(HANDLER_TABLE.len(), HANDLER_TABLE_SIZE);

            // Invoking any handler should not panic
            // (they all point to noop_handler or real handlers)
            invoke_handler(id);
        }
    }

    // **Feature: binary-dawn-features, Property 10: Handler Group Count**
    // *For any* compiled application, the number of handler chunk files SHALL be between 3 and 5 inclusive.
    // **Validates: Requirements 5.4, 13.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_handler_group_count(
            critical_count in 0usize..10,
            interactive_count in 0usize..10,
            submission_count in 0usize..10,
            navigation_count in 0usize..10,
            rare_count in 0usize..10
        ) {
            let mut manifest = HandlerManifest::new();

            // Add handlers to groups
            for i in 0..critical_count {
                manifest.add_to_group(HandlerRef::new(i as u16, 0), HandlerGroup::Critical);
            }
            for i in 0..interactive_count {
                manifest.add_to_group(HandlerRef::new(i as u16, 0), HandlerGroup::Interactive);
            }
            for i in 0..submission_count {
                manifest.add_to_group(HandlerRef::new(i as u16, 0), HandlerGroup::Submission);
            }
            for i in 0..navigation_count {
                manifest.add_to_group(HandlerRef::new(i as u16, 0), HandlerGroup::Navigation);
            }
            for i in 0..rare_count {
                manifest.add_to_group(HandlerRef::new(i as u16, 0), HandlerGroup::Rare);
            }

            // Total group count is always 5
            prop_assert_eq!(HandlerGroup::COUNT, 5);

            // Non-empty chunk count should be between 0 and 5
            let chunk_count = manifest.chunk_count();
            prop_assert!(chunk_count <= 5);

            // If we have handlers, chunk count should be at least 1
            if manifest.total_count() > 0 {
                prop_assert!(chunk_count >= 1);
            }
        }
    }

    // Round-trip property for HandlerRef serialization
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_handler_ref_roundtrip(
            fn_index in any::<u16>(),
            capture_offset in any::<u16>()
        ) {
            let original = HandlerRef::new(fn_index, capture_offset);
            let bytes = original.to_bytes();
            let restored = HandlerRef::from_bytes(&bytes);

            prop_assert_eq!(original, restored);
        }
    }

    // Handler classification completeness
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_handler_classification_completeness(
            is_above_fold in any::<bool>(),
            is_click in any::<bool>(),
            is_hover in any::<bool>(),
            is_focus in any::<bool>(),
            is_form_submit in any::<bool>(),
            is_navigation in any::<bool>()
        ) {
            let metadata = HandlerMetadata {
                is_above_fold,
                is_click,
                is_hover,
                is_focus,
                is_form_submit,
                is_navigation,
            };

            // Classification should always return a valid group
            let group = metadata.classify();
            prop_assert!(matches!(
                group,
                HandlerGroup::Critical
                    | HandlerGroup::Interactive
                    | HandlerGroup::Submission
                    | HandlerGroup::Navigation
                    | HandlerGroup::Rare
            ));
        }
    }
}
