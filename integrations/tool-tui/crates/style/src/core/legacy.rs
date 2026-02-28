//! Legacy Global State
//!
//! This module contains legacy global state that should be migrated to AppState.
//! New code should use AppState fields instead of these global statics.

use std::sync::atomic::{AtomicBool, Ordering};

// Legacy global state - new code should use AppState.base_layer_present instead
static BASE_LAYER_PRESENT: AtomicBool = AtomicBool::new(false);

/// Set the base layer present flag (legacy global state)
/// Note: New code should use AppState.base_layer_present instead
pub fn set_base_layer_present() {
    BASE_LAYER_PRESENT.store(true, Ordering::Relaxed);
}

#[allow(dead_code)]
pub(crate) fn base_layer_present() -> bool {
    BASE_LAYER_PRESENT.load(Ordering::Relaxed);
}

// Legacy global state - new code should use AppState.properties_layer_present instead
static PROPERTIES_LAYER_PRESENT: AtomicBool = AtomicBool::new(false);

/// Set the properties layer present flag (legacy global state)
/// Note: New code should use AppState.properties_layer_present instead
pub fn set_properties_layer_present() {
    PROPERTIES_LAYER_PRESENT.store(true, Ordering::Relaxed);
}

#[allow(dead_code)]
pub fn properties_layer_present() -> bool {
    PROPERTIES_LAYER_PRESENT.load(Ordering::Relaxed);
}

// Legacy global state - new code should use RebuildResult.html_modified instead
pub(crate) static FIRST_LOG_DONE: AtomicBool = AtomicBool::new(false);

// Flag to suppress logging after HTML grouping rewrite
// When set, the next rebuild_styles() call will skip all logging
// Note: New code should use RebuildResult.html_modified instead
pub(crate) static SUPPRESS_NEXT_LOG: AtomicBool = AtomicBool::new(false);
