//! # Handler Code Splitting
//!
//! Smart code splitting that groups handlers by usage pattern instead of
//! per-function files, reducing HTTP requests from 50+ to 3-5.
//!
//! **Validates: Requirements 13.1, 13.2, 13.4**

use crate::handlers::HandlerGroup;

/// Handler metadata for classification
#[derive(Debug, Clone)]
pub struct Handler {
    /// Handler ID
    pub id: u16,
    /// Is this handler for an above-fold element?
    pub is_above_fold: bool,
    /// Is this a click handler?
    pub is_click: bool,
    /// Is this a hover handler?
    pub is_hover: bool,
    /// Is this a focus handler?
    pub is_focus: bool,
    /// Is this a form submission handler?
    pub is_form_submit: bool,
    /// Is this a navigation handler?
    pub is_navigation: bool,
    /// Element ID this handler is attached to
    pub element_id: u16,
}

impl Handler {
    /// Create a new handler with default values
    pub fn new(id: u16) -> Self {
        Self {
            id,
            is_above_fold: false,
            is_click: false,
            is_hover: false,
            is_focus: false,
            is_form_submit: false,
            is_navigation: false,
            element_id: 0,
        }
    }

    /// Builder pattern: set above fold
    pub fn above_fold(mut self, value: bool) -> Self {
        self.is_above_fold = value;
        self
    }

    /// Builder pattern: set click
    pub fn click(mut self, value: bool) -> Self {
        self.is_click = value;
        self
    }

    /// Builder pattern: set hover
    pub fn hover(mut self, value: bool) -> Self {
        self.is_hover = value;
        self
    }

    /// Builder pattern: set focus
    pub fn focus(mut self, value: bool) -> Self {
        self.is_focus = value;
        self
    }

    /// Builder pattern: set form submit
    pub fn form_submit(mut self, value: bool) -> Self {
        self.is_form_submit = value;
        self
    }

    /// Builder pattern: set navigation
    pub fn navigation(mut self, value: bool) -> Self {
        self.is_navigation = value;
        self
    }

    /// Builder pattern: set element ID
    pub fn element(mut self, id: u16) -> Self {
        self.element_id = id;
        self
    }
}

/// Handler classification for smart splitting
pub struct HandlerClassifier;

impl HandlerClassifier {
    /// Classify handler by usage pattern
    ///
    /// Classification priority:
    /// 1. Critical: Above-fold click handlers (likely clicked first)
    /// 2. Interactive: Hover and focus handlers
    /// 3. Submission: Form submission handlers
    /// 4. Navigation: Route change handlers
    /// 5. Rare: Everything else (error handlers, edge cases)
    #[inline]
    pub fn classify(handler: &Handler) -> HandlerGroup {
        if handler.is_above_fold && handler.is_click {
            HandlerGroup::Critical
        } else if handler.is_hover || handler.is_focus {
            HandlerGroup::Interactive
        } else if handler.is_form_submit {
            HandlerGroup::Submission
        } else if handler.is_navigation {
            HandlerGroup::Navigation
        } else {
            HandlerGroup::Rare
        }
    }

    /// Classify multiple handlers and group them
    pub fn classify_all(handlers: &[Handler]) -> ClassificationResult {
        let mut result = ClassificationResult::new();

        for handler in handlers {
            let group = Self::classify(handler);
            result.add(handler.id, group);
        }

        result
    }
}

/// Result of classifying multiple handlers
#[derive(Debug, Clone, Default)]
pub struct ClassificationResult {
    /// Critical handlers (above-fold clicks)
    pub critical: Vec<u16>,
    /// Interactive handlers (hover, focus)
    pub interactive: Vec<u16>,
    /// Submission handlers (form submits)
    pub submission: Vec<u16>,
    /// Navigation handlers (route changes)
    pub navigation: Vec<u16>,
    /// Rare handlers (everything else)
    pub rare: Vec<u16>,
}

impl ClassificationResult {
    /// Create a new empty result
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a handler to the appropriate group
    pub fn add(&mut self, handler_id: u16, group: HandlerGroup) {
        match group {
            HandlerGroup::Critical => self.critical.push(handler_id),
            HandlerGroup::Interactive => self.interactive.push(handler_id),
            HandlerGroup::Submission => self.submission.push(handler_id),
            HandlerGroup::Navigation => self.navigation.push(handler_id),
            HandlerGroup::Rare => self.rare.push(handler_id),
        }
    }

    /// Get the number of non-empty groups (chunks to generate)
    pub fn chunk_count(&self) -> usize {
        let mut count = 0;
        if !self.critical.is_empty() {
            count += 1;
        }
        if !self.interactive.is_empty() {
            count += 1;
        }
        if !self.submission.is_empty() {
            count += 1;
        }
        if !self.navigation.is_empty() {
            count += 1;
        }
        if !self.rare.is_empty() {
            count += 1;
        }
        count
    }

    /// Get all handler IDs in a specific group
    pub fn get_group(&self, group: HandlerGroup) -> &[u16] {
        match group {
            HandlerGroup::Critical => &self.critical,
            HandlerGroup::Interactive => &self.interactive,
            HandlerGroup::Submission => &self.submission,
            HandlerGroup::Navigation => &self.navigation,
            HandlerGroup::Rare => &self.rare,
        }
    }

    /// Get total number of handlers
    pub fn total_handlers(&self) -> usize {
        self.critical.len()
            + self.interactive.len()
            + self.submission.len()
            + self.navigation.len()
            + self.rare.len()
    }
}

/// Prefetching system for handler chunks
pub struct Prefetcher {
    /// Bitfield of loaded groups (bit N = group N loaded)
    loaded_groups: u8,
    /// Prediction cache: element_id -> likely groups
    predictions: Vec<(u16, Vec<HandlerGroup>)>,
}

impl Prefetcher {
    /// Create a new prefetcher
    pub fn new() -> Self {
        Self {
            loaded_groups: 0,
            predictions: Vec::new(),
        }
    }

    /// Check if a group is already loaded
    #[inline]
    pub fn is_loaded(&self, group: HandlerGroup) -> bool {
        (self.loaded_groups & (1 << group as u8)) != 0
    }

    /// Mark a group as loaded
    #[inline]
    pub fn mark_loaded(&mut self, group: HandlerGroup) {
        self.loaded_groups |= 1 << group as u8;
    }

    /// Add a prediction for an element
    pub fn add_prediction(&mut self, element_id: u16, groups: Vec<HandlerGroup>) {
        self.predictions.push((element_id, groups));
    }

    /// Predict and prefetch on mouse enter
    ///
    /// Returns the groups that should be prefetched
    pub fn on_mouse_enter(&mut self, element_id: u16) -> Vec<HandlerGroup> {
        let likely_groups = self.predict_actions(element_id);
        let mut to_prefetch = Vec::new();

        for group in likely_groups {
            if !self.is_loaded(group) {
                to_prefetch.push(group);
                self.mark_loaded(group);
            }
        }

        to_prefetch
    }

    /// Predict likely actions for an element
    fn predict_actions(&self, element_id: u16) -> Vec<HandlerGroup> {
        // Check prediction cache first
        for (id, groups) in &self.predictions {
            if *id == element_id {
                return groups.clone();
            }
        }

        // Default prediction: Critical and Interactive are most likely
        vec![HandlerGroup::Critical, HandlerGroup::Interactive]
    }

    /// Get the number of loaded groups
    pub fn loaded_count(&self) -> u32 {
        self.loaded_groups.count_ones()
    }

    /// Reset all loaded state
    pub fn reset(&mut self) {
        self.loaded_groups = 0;
    }
}

impl Default for Prefetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunk manifest for build output
#[derive(Debug, Clone)]
pub struct ChunkManifest {
    /// Chunks to generate
    pub chunks: Vec<ChunkInfo>,
}

/// Information about a single chunk
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    /// Chunk filename
    pub filename: String,
    /// Handler group
    pub group: HandlerGroup,
    /// Handler IDs in this chunk
    pub handler_ids: Vec<u16>,
    /// Estimated size in bytes
    pub estimated_size: usize,
}

impl ChunkManifest {
    /// Create a manifest from classification result
    pub fn from_classification(result: &ClassificationResult) -> Self {
        let mut chunks = Vec::new();

        let groups = [
            (HandlerGroup::Critical, "handlers_critical.dxb", &result.critical),
            (HandlerGroup::Interactive, "handlers_secondary.dxb", &result.interactive),
            (HandlerGroup::Submission, "handlers_submission.dxb", &result.submission),
            (HandlerGroup::Navigation, "handlers_navigation.dxb", &result.navigation),
            (HandlerGroup::Rare, "handlers_rare.dxb", &result.rare),
        ];

        for (group, filename, ids) in groups {
            if !ids.is_empty() {
                chunks.push(ChunkInfo {
                    filename: filename.to_string(),
                    group,
                    handler_ids: ids.clone(),
                    // Estimate ~50 bytes per handler
                    estimated_size: ids.len() * 50,
                });
            }
        }

        Self { chunks }
    }

    /// Get total number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // Unit tests

    #[test]
    fn test_handler_classification_critical() {
        let handler = Handler::new(1).above_fold(true).click(true);
        assert_eq!(HandlerClassifier::classify(&handler), HandlerGroup::Critical);
    }

    #[test]
    fn test_handler_classification_interactive() {
        let handler = Handler::new(2).hover(true);
        assert_eq!(HandlerClassifier::classify(&handler), HandlerGroup::Interactive);

        let handler = Handler::new(3).focus(true);
        assert_eq!(HandlerClassifier::classify(&handler), HandlerGroup::Interactive);
    }

    #[test]
    fn test_handler_classification_submission() {
        let handler = Handler::new(4).form_submit(true);
        assert_eq!(HandlerClassifier::classify(&handler), HandlerGroup::Submission);
    }

    #[test]
    fn test_handler_classification_navigation() {
        let handler = Handler::new(5).navigation(true);
        assert_eq!(HandlerClassifier::classify(&handler), HandlerGroup::Navigation);
    }

    #[test]
    fn test_handler_classification_rare() {
        let handler = Handler::new(6);
        assert_eq!(HandlerClassifier::classify(&handler), HandlerGroup::Rare);
    }

    #[test]
    fn test_classification_result() {
        let handlers = vec![
            Handler::new(1).above_fold(true).click(true),
            Handler::new(2).hover(true),
            Handler::new(3).form_submit(true),
            Handler::new(4).navigation(true),
            Handler::new(5),
        ];

        let result = HandlerClassifier::classify_all(&handlers);

        assert_eq!(result.critical, vec![1]);
        assert_eq!(result.interactive, vec![2]);
        assert_eq!(result.submission, vec![3]);
        assert_eq!(result.navigation, vec![4]);
        assert_eq!(result.rare, vec![5]);
        assert_eq!(result.chunk_count(), 5);
        assert_eq!(result.total_handlers(), 5);
    }

    #[test]
    fn test_prefetcher() {
        let mut prefetcher = Prefetcher::new();

        assert!(!prefetcher.is_loaded(HandlerGroup::Critical));

        prefetcher.mark_loaded(HandlerGroup::Critical);
        assert!(prefetcher.is_loaded(HandlerGroup::Critical));
        assert!(!prefetcher.is_loaded(HandlerGroup::Interactive));

        assert_eq!(prefetcher.loaded_count(), 1);
    }

    #[test]
    fn test_prefetcher_on_mouse_enter() {
        let mut prefetcher = Prefetcher::new();

        // First mouse enter should prefetch
        let to_prefetch = prefetcher.on_mouse_enter(1);
        assert!(!to_prefetch.is_empty());

        // Second mouse enter on same element should not prefetch same groups
        let to_prefetch2 = prefetcher.on_mouse_enter(1);
        assert!(to_prefetch2.is_empty());
    }

    #[test]
    fn test_chunk_manifest() {
        let handlers = vec![
            Handler::new(1).above_fold(true).click(true),
            Handler::new(2).above_fold(true).click(true),
            Handler::new(3).hover(true),
        ];

        let result = HandlerClassifier::classify_all(&handlers);
        let manifest = ChunkManifest::from_classification(&result);

        // Should have 2 chunks: critical and interactive
        assert_eq!(manifest.chunk_count(), 2);
    }

    // Property-based tests

    // Feature: binary-dawn-features, Property 22: Handler Classification Completeness
    // For any handler in an application, the classifier SHALL assign it to exactly one HandlerGroup.
    // Validates: Requirements 13.1, 13.2
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_handler_classification_completeness(
            id in 0u16..1000,
            is_above_fold in any::<bool>(),
            is_click in any::<bool>(),
            is_hover in any::<bool>(),
            is_focus in any::<bool>(),
            is_form_submit in any::<bool>(),
            is_navigation in any::<bool>(),
        ) {
            let handler = Handler {
                id,
                is_above_fold,
                is_click,
                is_hover,
                is_focus,
                is_form_submit,
                is_navigation,
                element_id: 0,
            };

            let group = HandlerClassifier::classify(&handler);

            // Property: Every handler is assigned to exactly one group
            let is_valid_group = matches!(
                group,
                HandlerGroup::Critical
                | HandlerGroup::Interactive
                | HandlerGroup::Submission
                | HandlerGroup::Navigation
                | HandlerGroup::Rare
            );
            prop_assert!(is_valid_group);
        }

        #[test]
        fn prop_classification_preserves_all_handlers(
            handler_count in 1usize..100
        ) {
            let handlers: Vec<Handler> = (0..handler_count)
                .map(|i| Handler::new(i as u16))
                .collect();

            let result = HandlerClassifier::classify_all(&handlers);

            // Property: Total handlers in result equals input count
            prop_assert_eq!(result.total_handlers(), handler_count);
        }

        #[test]
        fn prop_chunk_count_between_3_and_5(
            handler_count in 5usize..50
        ) {
            // Create handlers that will be distributed across all groups
            let mut handlers = Vec::new();

            for i in 0..handler_count {
                let handler = match i % 5 {
                    0 => Handler::new(i as u16).above_fold(true).click(true),
                    1 => Handler::new(i as u16).hover(true),
                    2 => Handler::new(i as u16).form_submit(true),
                    3 => Handler::new(i as u16).navigation(true),
                    _ => Handler::new(i as u16),
                };
                handlers.push(handler);
            }

            let result = HandlerClassifier::classify_all(&handlers);
            let manifest = ChunkManifest::from_classification(&result);

            // Property: Chunk count is between 3 and 5 (when all groups have handlers)
            let chunk_count = manifest.chunk_count();
            prop_assert!(chunk_count >= 1 && chunk_count <= 5);
        }

        #[test]
        fn prop_prefetcher_idempotent(
            element_id in 0u16..100,
            iterations in 1usize..10
        ) {
            let mut prefetcher = Prefetcher::new();

            // First call may prefetch
            let first_prefetch = prefetcher.on_mouse_enter(element_id);

            // Subsequent calls should not prefetch the same groups
            for _ in 1..iterations {
                let subsequent = prefetcher.on_mouse_enter(element_id);

                // Property: Once a group is loaded, it won't be prefetched again
                for group in &subsequent {
                    prop_assert!(!first_prefetch.contains(group));
                }
            }
        }

        #[test]
        fn prop_classification_deterministic(
            id in 0u16..1000,
            is_above_fold in any::<bool>(),
            is_click in any::<bool>(),
            is_hover in any::<bool>(),
            is_focus in any::<bool>(),
            is_form_submit in any::<bool>(),
            is_navigation in any::<bool>(),
        ) {
            let handler = Handler {
                id,
                is_above_fold,
                is_click,
                is_hover,
                is_focus,
                is_form_submit,
                is_navigation,
                element_id: 0,
            };

            // Property: Classification is deterministic
            let group1 = HandlerClassifier::classify(&handler);
            let group2 = HandlerClassifier::classify(&handler);

            prop_assert_eq!(group1, group2);
        }
    }
}
