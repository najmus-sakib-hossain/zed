//! Application State Management
//!
//! Manages the runtime state of the CSS generation pipeline, including:
//! - HTML content hashing for change detection
//! - CSS class caching for incremental updates
//! - Output buffer management
//! - Group registry for auto-grouping
//! - Incremental parser state

use crate::core::{engine, group, output::CssOutput};
use crate::parser::IncrementalParser;
use ahash::{AHashMap, AHashSet};

/// Metadata for a CSS rule in the output buffer.
#[derive(Clone, Copy, Debug)]
pub struct RuleMeta {
    /// Byte offset in the CSS buffer where this rule starts.
    pub off: usize,
    /// Length of the rule in bytes.
    pub len: usize,
}

/// Application state for the dx-style engine.
///
/// `AppState` manages the runtime state of the CSS generation pipeline, including:
/// - HTML content hashing for change detection
/// - CSS class caching for incremental updates
/// - Output buffer management
/// - Group registry for auto-grouping
/// - Incremental parser state
///
/// # Thread Safety
///
/// `AppState` is designed to be wrapped in `Arc<Mutex<AppState>>` for thread-safe access.
/// The `engine()` method provides a static reference to the style engine using `OnceLock`.
///
/// # Example
///
/// ```rust,ignore
/// use std::sync::{Arc, Mutex};
/// use style::core::AppState;
///
/// // Get the global style engine (lazily initialized)
/// let engine = AppState::engine();
///
/// // Generate CSS for a class
/// if let Some(css) = engine.css_for_class("flex") {
///     println!("CSS: {}", css);
/// }
/// ```
///
/// # Initialization
///
/// The style engine is lazily initialized on first access via `AppState::engine()`.
/// It attempts to load configuration from disk, falling back to an empty engine if loading fails.
///
/// # Environment Variables
///
/// - `DX_FORCE_FULL`: Set to "1" to force full rebuild (skip incremental updates)
/// - `DX_FORCE_FORMAT`: Set to "1" to force CSS formatting
/// - `DX_DEBUG`: Set to "1" to enable debug logging
/// - `DX_DISABLE_INCREMENTAL`: Set to "1" to disable incremental parsing
pub struct AppState {
    /// Hash of the current HTML content for change detection.
    pub html_hash: u64,
    /// Set of CSS classes that have been processed.
    pub class_cache: AHashSet<String>,
    /// CSS output writer for generating the final CSS file.
    pub css_out: CssOutput,
    /// Hash of the last generated CSS for change detection.
    pub last_css_hash: u64,
    /// Buffer containing the generated CSS bytes.
    pub css_buffer: Vec<u8>,
    /// Checksum of the class list for validation.
    pub class_list_checksum: u64,
    /// Index mapping class names to their positions in the CSS buffer.
    pub css_index: AHashMap<String, RuleMeta>,
    /// Byte offset where utility classes start in the CSS buffer.
    pub utilities_offset: usize,
    /// Registry for grouped classnames (auto-grouping feature).
    pub group_registry: group::GroupRegistry,
    /// Hash of the group log for change detection.
    pub group_log_hash: u64,
    /// Incremental parser for efficient re-parsing of changed content.
    pub incremental_parser: IncrementalParser,
    /// Whether the base layer has been written (moved from global static)
    #[allow(dead_code)]
    pub base_layer_present: bool,
    /// Whether the properties layer has been written (moved from global static)
    #[allow(dead_code)]
    pub properties_layer_present: bool,
    /// Whether the first log message has been printed (moved from global static)
    #[allow(dead_code)]
    pub first_log_done: bool,
}

impl AppState {
    /// Get a reference to the global style engine.
    ///
    /// The engine is lazily initialized on first access using `OnceLock`.
    /// It attempts to load configuration from disk (`.dx/style/config.toml`),
    /// falling back to an empty engine if loading fails.
    ///
    /// # Returns
    ///
    /// A static reference to the `StyleEngine` instance.
    ///
    /// # Thread Safety
    ///
    /// This method is thread-safe and can be called from multiple threads.
    /// The engine is initialized exactly once.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use style::core::AppState;
    ///
    /// let engine = AppState::engine();
    ///
    /// // Check if a class is known
    /// if let Some(css) = engine.css_for_class("p-4") {
    ///     println!("padding-4 CSS: {}", css);
    /// }
    /// ```
    pub fn engine() -> &'static engine::StyleEngine {
        use std::sync::OnceLock;
        static INSTANCE: OnceLock<engine::StyleEngine> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            engine::StyleEngine::load_from_disk().unwrap_or_else(|_| engine::StyleEngine::empty())
        })
    }
}
