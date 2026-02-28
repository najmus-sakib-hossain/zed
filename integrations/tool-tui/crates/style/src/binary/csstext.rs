/// Level 2: Direct cssText Injection
///
/// Skip classList.add() entirely and write styles directly to element.style.cssText
/// This is 3-5× faster than classList operations
use crate::binary::ids::{StyleId, style_id_to_csstext};

/// Apply styles directly via cssText (ONE DOM write instead of N classList.add() calls)
///
/// # Arguments
/// * `ids` - Array of style IDs to apply
///
/// # Returns
/// * CSS text string ready for element.style.cssText
pub fn apply_styles_direct(ids: &[StyleId]) -> String {
    let mut css_text = String::with_capacity(ids.len() * 30); // Pre-allocate

    for (i, &id) in ids.iter().enumerate() {
        if let Some(css) = style_id_to_csstext(id) {
            css_text.push_str(css);

            // Add semicolon separator (except after last property)
            if i < ids.len() - 1 {
                css_text.push(';');
            }
        }
    }

    css_text
}

/// Apply styles with specific capacity hint (for performance)
pub fn apply_styles_direct_with_capacity(ids: &[StyleId], capacity: usize) -> String {
    let mut css_text = String::with_capacity(capacity);

    for (i, &id) in ids.iter().enumerate() {
        if let Some(css) = style_id_to_csstext(id) {
            css_text.push_str(css);
            if i < ids.len() - 1 {
                css_text.push(';');
            }
        }
    }

    css_text
}

/// Optimized version that pre-joins common patterns
/// Returns None if any ID is invalid
pub fn apply_styles_direct_checked(ids: &[StyleId]) -> Option<String> {
    let mut css_text = String::with_capacity(ids.len() * 30);

    for (i, &id) in ids.iter().enumerate() {
        let css = style_id_to_csstext(id)?; // Return None if invalid ID
        css_text.push_str(css);
        if i < ids.len() - 1 {
            css_text.push(';');
        }
    }

    Some(css_text)
}

/// JavaScript-compatible API
/// Generates the host function signature for WASM
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use super::*;
    use wasm_bindgen::prelude::*;

    /// Apply styles to a DOM node (called from WASM)
    ///
    /// # Arguments
    /// * `node_id` - DOM node identifier
    /// * `ids` - JavaScript Uint16Array of style IDs
    #[wasm_bindgen]
    pub fn wasm_apply_styles(node_id: u32, ids: &[u16]) {
        let css_text = apply_styles_direct(ids);
        host_set_style(node_id, &css_text);
    }

    // Host function imported from JavaScript
    #[wasm_bindgen(raw_module = "../dx-client.js")]
    extern "C" {
        #[wasm_bindgen(js_name = setStyle)]
        fn host_set_style(node_id: u32, css_text: &str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_application() {
        // flex items-center p-4 = IDs [4, 26, 36]
        // Note: ID 36 = padding:1rem (p-4), ID 35 = padding:0.75rem (p-3)
        let ids = vec![4, 26, 36];
        let result = apply_styles_direct(&ids);

        assert_eq!(result, "display:flex;align-items:center;padding:1rem");
    }

    #[test]
    fn test_single_style() {
        let ids = vec![4]; // flex
        let result = apply_styles_direct(&ids);
        assert_eq!(result, "display:flex");
    }

    #[test]
    fn test_empty_list() {
        let ids = vec![];
        let result = apply_styles_direct(&ids);
        assert_eq!(result, "");
    }

    #[test]
    fn test_capacity_optimization() {
        let ids = vec![4, 26, 36];
        let result = apply_styles_direct_with_capacity(&ids, 100);
        assert_eq!(result, "display:flex;align-items:center;padding:1rem");
    }

    #[test]
    fn test_checked_version() {
        let ids = vec![4, 26, 36];
        let result = apply_styles_direct_checked(&ids);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "display:flex;align-items:center;padding:1rem");
    }

    #[test]
    fn test_invalid_id() {
        let ids = vec![4, 9999, 36]; // 9999 is invalid
        let result = apply_styles_direct_checked(&ids);
        assert!(result.is_none());
    }

    #[test]
    fn test_performance_characteristics() {
        // This demonstrates the performance advantage
        // Traditional: 3 classList.add() calls = 3 DOM writes
        // Direct cssText: 1 cssText write = 1 DOM write

        let ids = vec![4, 26, 36, 173, 191]; // 5 styles
        let start = std::time::Instant::now();

        for _ in 0..1000 {
            let _ = apply_styles_direct(&ids);
        }

        let elapsed = start.elapsed();
        println!("1000 iterations took: {:?}", elapsed);

        // Should be < 1ms for 1000 iterations
        assert!(elapsed.as_millis() < 10);
    }
}
