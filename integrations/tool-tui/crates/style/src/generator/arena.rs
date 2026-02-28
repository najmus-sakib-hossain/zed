//! Arena-based CSS generator for zero-allocation batch CSS generation
//!
//! This module provides high-performance CSS generation by using arena allocation
//! to eliminate per-class allocation overhead. It's optimized for batch operations
//! where many CSS rules need to be generated at once.

use bumpalo::Bump;
use cssparser::serialize_identifier;

use crate::core::{AppState, group::GroupRegistry};

/// Arena-based CSS batch generator
///
/// Uses bump allocation for extremely fast memory allocation without
/// individual deallocations. Perfect for generating many CSS rules at once.
#[allow(dead_code)]
pub struct ArenaCssGenerator<'arena> {
    arena: &'arena Bump,
    buffer: bumpalo::collections::Vec<'arena, u8>,
}

#[allow(dead_code)]
impl<'arena> ArenaCssGenerator<'arena> {
    /// Create a new arena-based generator with the given arena
    #[inline]
    pub fn new(arena: &'arena Bump) -> Self {
        Self {
            arena,
            buffer: bumpalo::collections::Vec::with_capacity_in(8192, arena),
        }
    }

    /// Create with explicit capacity hint
    #[inline]
    pub fn with_capacity(arena: &'arena Bump, capacity: usize) -> Self {
        Self {
            arena,
            buffer: bumpalo::collections::Vec::with_capacity_in(capacity, arena),
        }
    }

    /// Generate CSS for multiple classes in a single batch
    ///
    /// This is the main entry point and provides the best performance for
    /// generating multiple CSS rules at once.
    pub fn generate_batch<'a, I>(&mut self, classes: I, groups: &mut GroupRegistry) -> &[u8]
    where
        I: IntoIterator<Item = &'a String>,
    {
        self.buffer.clear();

        let engine_opt = std::panic::catch_unwind(AppState::engine).ok();
        let Some(engine) = engine_opt else {
            return self.buffer.as_slice();
        };

        let collected: Vec<&String> = classes.into_iter().collect();

        // Generate property layer if needed
        if self.buffer.is_empty() && !crate::core::properties_layer_present() {
            let props = engine.property_at_rules();
            if !props.is_empty() {
                self.buffer.extend_from_slice(props.as_bytes());
                crate::core::set_properties_layer_present();
            }

            // Generate color variables
            let (root_vars, dark_vars) = engine.generate_color_vars_for(collected.iter().copied());
            if !root_vars.is_empty() {
                self.buffer.extend_from_slice(root_vars.as_bytes());
            }
            if !dark_vars.is_empty() {
                self.buffer.extend_from_slice(dark_vars.as_bytes());
            }
        }

        // Use arena for temporary string allocations
        let mut escaped = bumpalo::collections::String::with_capacity_in(128, self.arena);

        for class in collected {
            if groups.is_internal_token(class) {
                continue;
            }

            // Handle group aliases
            if let Some(alias_css) = groups.generate_css_for(class, engine) {
                self.buffer.extend_from_slice(alias_css.as_bytes());
                if !alias_css.ends_with('\n') {
                    self.buffer.push(b'\n');
                }
                continue;
            }

            // Generate CSS from engine
            if let Some(css) = engine.css_for_class(class) {
                self.buffer.extend_from_slice(css.as_bytes());
                if !css.ends_with('\n') {
                    self.buffer.push(b'\n');
                }
            } else {
                // Fallback: generate empty rule
                self.buffer.push(b'.');
                escaped.clear();
                let _ = serialize_identifier(class, &mut escaped);
                self.buffer.extend_from_slice(escaped.as_bytes());
                self.buffer.extend_from_slice(b" {}\n");
            }
        }

        self.buffer.as_slice()
    }

    /// Generate CSS for a single class (optimized path)
    #[inline]
    pub fn generate_single(&mut self, class: &str, groups: &mut GroupRegistry) -> Option<&[u8]> {
        self.buffer.clear();

        let engine_opt = std::panic::catch_unwind(AppState::engine).ok();
        let engine = engine_opt?;

        if groups.is_internal_token(class) {
            return None;
        }

        // Handle group aliases
        if let Some(alias_css) = groups.generate_css_for(class, engine) {
            self.buffer.extend_from_slice(alias_css.as_bytes());
            if !alias_css.ends_with('\n') {
                self.buffer.push(b'\n');
            }
            return Some(self.buffer.as_slice());
        }

        // Generate from engine
        if let Some(css) = engine.css_for_class(class) {
            self.buffer.extend_from_slice(css.as_bytes());
            if !css.ends_with('\n') {
                self.buffer.push(b'\n');
            }
            Some(self.buffer.as_slice())
        } else {
            // Fallback: generate empty rule
            self.buffer.push(b'.');
            let mut escaped = bumpalo::collections::String::with_capacity_in(64, self.arena);
            let _ = serialize_identifier(class, &mut escaped);
            self.buffer.extend_from_slice(escaped.as_bytes());
            self.buffer.extend_from_slice(b" {}\n");
            Some(self.buffer.as_slice())
        }
    }

    /// Get the current buffer size
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Clear the internal buffer
    #[inline]
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get a reference to the accumulated CSS
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        self.buffer.as_slice()
    }
}

/// Optimized batch CSS generation using arena allocation
///
/// This function creates an arena and generates CSS for all provided classes
/// in a single allocation context, then copies the result to the output buffer.
#[allow(dead_code)]
pub fn generate_css_batch_arena<'a, I>(out: &mut Vec<u8>, classes: I, groups: &mut GroupRegistry)
where
    I: IntoIterator<Item = &'a String>,
{
    // Create arena for temporary allocations
    let arena = Bump::new();
    let mut generator = ArenaCssGenerator::new(&arena);

    let css = generator.generate_batch(classes, groups);
    out.extend_from_slice(css);

    // Arena is dropped here, freeing all allocations at once
}

/// Fast path for generating CSS with pre-sized buffer
#[allow(dead_code)]
pub fn generate_css_batch_presized<'a, I>(
    out: &mut Vec<u8>,
    classes: I,
    groups: &mut GroupRegistry,
    estimated_size: usize,
) where
    I: IntoIterator<Item = &'a String>,
{
    out.reserve(estimated_size);
    generate_css_batch_arena(out, classes, groups);
}

/// Estimate CSS output size based on class count
///
/// Returns a conservative estimate of the CSS size that will be generated.
/// This helps with pre-allocation to avoid reallocations.
#[allow(dead_code)]
#[inline]
pub fn estimate_css_size(class_count: usize) -> usize {
    // Average CSS rule is roughly 80-120 bytes
    // Using 128 bytes per rule as a safe estimate
    class_count.saturating_mul(128).max(1024)
}

/// Fast CSS generation for incremental updates
///
/// Optimized for adding a small number of new classes to existing CSS.
#[allow(dead_code)]
pub fn generate_incremental_css<'a, I>(
    out: &mut Vec<u8>,
    new_classes: I,
    groups: &mut GroupRegistry,
) where
    I: IntoIterator<Item = &'a String>,
{
    let new_classes_vec: Vec<&String> = new_classes.into_iter().collect();
    let count = new_classes_vec.len();

    if count == 0 {
        return;
    }

    // For small updates, just use regular generation
    if count <= 5 {
        let arena = Bump::new();
        let mut generator = ArenaCssGenerator::new(&arena);
        let css = generator.generate_batch(new_classes_vec, groups);
        out.extend_from_slice(css);
        return;
    }

    // For larger updates, pre-allocate
    let estimated = estimate_css_size(count);
    generate_css_batch_presized(out, new_classes_vec, groups, estimated);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::group::GroupRegistry;

    #[test]
    fn test_arena_generator_basic() {
        let arena = Bump::new();
        let mut generator = ArenaCssGenerator::new(&arena);
        let mut groups = GroupRegistry::new();

        let classes = vec!["flex".to_string(), "items-center".to_string()];
        let css = generator.generate_batch(&classes, &mut groups);

        assert!(!css.is_empty());
        assert!(css.len() > 10);
    }

    #[test]
    fn test_arena_generator_single() {
        let arena = Bump::new();
        let mut generator = ArenaCssGenerator::new(&arena);
        let mut groups = GroupRegistry::new();

        let result = generator.generate_single("flex", &mut groups);
        assert!(result.is_some());

        let css = result.unwrap();
        assert!(!css.is_empty());
    }

    #[test]
    fn test_estimate_css_size() {
        assert_eq!(estimate_css_size(0), 1024);
        assert_eq!(estimate_css_size(10), 1280);
        assert_eq!(estimate_css_size(100), 12800);
    }

    #[test]
    fn test_arena_reuse() {
        let arena = Bump::new();
        let mut generator = ArenaCssGenerator::new(&arena);
        let mut groups = GroupRegistry::new();

        // Generate multiple times with same generator
        let classes1 = vec!["flex".to_string()];
        let css1 = generator.generate_batch(&classes1, &mut groups);
        let len1 = css1.len();

        let classes2 = vec!["block".to_string()];
        let css2 = generator.generate_batch(&classes2, &mut groups);
        let len2 = css2.len();

        // Both should produce output
        assert!(len1 > 0);
        assert!(len2 > 0);
    }

    #[test]
    fn test_batch_generation() {
        let mut out = Vec::new();
        let mut groups = GroupRegistry::new();

        let classes = vec![
            "flex".to_string(),
            "items-center".to_string(),
            "justify-between".to_string(),
        ];

        generate_css_batch_arena(&mut out, &classes, &mut groups);

        assert!(!out.is_empty());
        assert!(out.len() > 30);
    }

    #[test]
    fn test_incremental_generation() {
        let mut out = Vec::new();
        let mut groups = GroupRegistry::new();

        let new_classes = vec!["text-red-500".to_string(), "bg-blue-200".to_string()];

        generate_incremental_css(&mut out, &new_classes, &mut groups);

        assert!(!out.is_empty());
    }
}
