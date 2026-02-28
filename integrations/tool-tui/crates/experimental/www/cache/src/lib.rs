//! # dx-cache: The Eternal Binary Cache Engine
//!
//! **Target:** 0ms LCP on second visit - forever
//!
//! ## Architecture
//!
//! ```text
//! First Visit:
//! Network → dx-binary → WASM Runtime → Render → Cache Everything
//!
//! Second Visit (FOREVER):
//! Cache (0ms) → WASM Resume → Render (instant)
//! ```
//!
//! ## Storage Strategy
//!
//! 1. **IndexedDB** - Primary storage (templates, snapshots, state)
//! 2. **Cache API** - HTTP cache for delta updates
//! 3. **LocalStorage** - Metadata (version hashes, signatures)
//!

#![allow(clippy::collapsible_if)] // Nested if statements improve readability for async operations
//! ## Security
//!
//! - Every entry signed with Ed25519
//! - Cache keyed by origin + public key
//! - Tamper-proof - any modification = instant invalidation

#![forbid(unsafe_code)]

pub mod crypto;
pub mod preload;
pub mod storage;

use wasm_bindgen::prelude::*;
use web_sys::console;

/// Cache configuration
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Database name
    db_name: String,
    /// Cache version
    version: u32,
    /// Maximum cache size (bytes)
    max_size: usize,
    /// Cache lifetime (seconds, 0 = eternal)
    lifetime: u64,
}

#[wasm_bindgen]
impl CacheConfig {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    #[wasm_bindgen(getter)]
    pub fn db_name(&self) -> String {
        self.db_name.clone()
    }

    #[wasm_bindgen(setter)]
    pub fn set_db_name(&mut self, name: String) {
        self.db_name = name;
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 {
        self.version
    }

    #[wasm_bindgen(getter)]
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    #[wasm_bindgen(getter)]
    pub fn lifetime(&self) -> u64 {
        self.lifetime
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            db_name: "dx-cache".to_string(),
            version: 1,
            max_size: 128 * 1024 * 1024, // 128 MB
            lifetime: 0,                 // Eternal
        }
    }
}

/// Initialize dx-cache system
#[wasm_bindgen]
pub async fn init_cache(config: Option<CacheConfig>) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();

    let config = config.unwrap_or_default();

    console::log_1(&format!("🗄️  dx-cache initializing (v{})", config.version).into());
    console::log_1(&format!("📦 Max size: {} MB", config.max_size / 1024 / 1024).into());

    if config.lifetime == 0 {
        console::log_1(&"♾️  Lifetime: ETERNAL".into());
    } else {
        console::log_1(&format!("⏰ Lifetime: {} days", config.lifetime / 86400).into());
    }

    // Initialize storage systems
    storage::init_indexeddb(&config.db_name, config.version).await?;
    storage::init_cache_api().await?;

    console::log_1(&"✅ dx-cache ready - Eternal storage initialized".into());

    Ok(JsValue::TRUE)
}

/// Check if cache is available
#[wasm_bindgen]
pub fn is_cache_available() -> bool {
    // Check IndexedDB support
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };

    // Check if IndexedDB exists
    js_sys::Reflect::has(&window, &"indexedDB".into()).unwrap_or(false)
}

/// Get cache statistics
#[wasm_bindgen]
pub async fn get_cache_stats() -> Result<JsValue, JsValue> {
    let stats = storage::get_storage_stats().await?;
    Ok(serde_wasm_bindgen::to_value(&stats)?)
}

/// Clear all cache (for testing/debugging)
#[wasm_bindgen]
pub async fn clear_cache() -> Result<(), JsValue> {
    console::log_1(&"🗑️  Clearing dx-cache...".into());

    storage::clear_indexeddb().await?;
    storage::clear_cache_api().await?;

    console::log_1(&"✅ Cache cleared".into());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CacheConfig::default();
        assert_eq!(config.db_name, "dx-cache");
        assert_eq!(config.version, 1);
        assert_eq!(config.lifetime, 0); // Eternal
    }
}
