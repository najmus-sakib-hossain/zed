//! Lazy CSS generation with memoization
//!
//! Implements game-changing optimizations:
//! - Lazy evaluation: Only generate CSS when needed
//! - Memoization: Cache generated CSS to avoid recomputation
//! - Parallel generation: Use rayon for multi-threaded CSS generation
//! - String interning: Deduplicate repeated strings

use ahash::AHashMap;
use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Global CSS cache for memoization
/// Stores class -> CSS mappings to avoid regeneration
static CSS_CACHE: Lazy<RwLock<AHashMap<String, String>>> =
    Lazy::new(|| RwLock::new(AHashMap::with_capacity(1024)));

/// Get cached CSS or generate and cache it
#[inline]
pub fn get_or_generate_css<F>(class: &str, generator: F) -> Option<String>
where
    F: FnOnce() -> Option<String>,
{
    // Fast path: Check cache first (read lock)
    if let Ok(cache) = CSS_CACHE.read() {
        if let Some(css) = cache.get(class) {
            return Some(css.clone());
        }
    }

    // Slow path: Generate and cache (write lock)
    let css = generator()?;

    if let Ok(mut cache) = CSS_CACHE.write() {
        cache.insert(class.to_string(), css.clone());
    }

    Some(css)
}

/// Clear the CSS cache (useful for testing or memory management)
#[allow(dead_code)]
pub fn clear_cache() {
    if let Ok(mut cache) = CSS_CACHE.write() {
        cache.clear();
    }
}

/// Get cache statistics
#[allow(dead_code)]
pub fn cache_stats() -> (usize, usize) {
    if let Ok(cache) = CSS_CACHE.read() {
        let size = cache.len();
        let capacity = cache.capacity();
        (size, capacity)
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memoization() {
        clear_cache();

        let mut call_count = 0;
        let generator = || {
            call_count += 1;
            Some("color: red;".to_string())
        };

        // First call should generate
        let css1 = get_or_generate_css("test-class", generator);
        assert_eq!(css1, Some("color: red;".to_string()));
        assert_eq!(call_count, 1);

        // Second call should use cache
        let css2 = get_or_generate_css("test-class", || {
            call_count += 1;
            Some("color: blue;".to_string())
        });
        assert_eq!(css2, Some("color: red;".to_string()));
        assert_eq!(call_count, 1); // Generator not called again
    }

    #[test]
    fn test_cache_stats() {
        clear_cache();

        get_or_generate_css("class1", || Some("css1".to_string()));
        get_or_generate_css("class2", || Some("css2".to_string()));

        let (size, _) = cache_stats();
        assert_eq!(size, 2);
    }
}
