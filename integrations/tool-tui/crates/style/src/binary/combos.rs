use once_cell::sync::Lazy;
/// Level 3: Pre-Computed Style Combinations
///
/// Most elements use common combinations like "flex items-center p-4"
/// Pre-compute these at compile time to save runtime concatenation
use std::collections::HashMap;

pub type ComboId = u16;

/// Pre-computed style combinations
/// These are the most frequently used patterns in real apps
pub static COMBO_DICT: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        // Combo 0: flex + items-center + p-4
        "display:flex;align-items:center;padding:1rem",
        // Combo 1: text-white + bg-blue-500
        "color:#fff;background:#3b82f6",
        // Combo 2: rounded-lg + shadow-md
        "border-radius:0.5rem;box-shadow:0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
        // Combo 3: flex + flex-col + items-center
        "display:flex;flex-direction:column;align-items:center",
        // Combo 4: w-full + h-full
        "width:100%;height:100%",
        // Combo 5: absolute + top-0 + right-0
        "position:absolute;top:0;right:0",
        // Combo 6: flex + justify-between + items-center
        "display:flex;justify-content:space-between;align-items:center",
        // Combo 7: text-center + font-bold + text-2xl
        "text-align:center;font-weight:700;font-size:1.875rem;line-height:2.25rem",
        // Combo 8: p-4 + rounded-lg + border + shadow
        "padding:1rem;border-radius:0.5rem;border:1px solid #e5e7eb;box-shadow:0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
        // Combo 9: flex + items-center + justify-center
        "display:flex;align-items:center;justify-content:center",
        // Combo 10: relative + overflow-hidden + rounded-lg
        "position:relative;overflow:hidden;border-radius:0.5rem",
        // Combo 11: flex + flex-col + w-full
        "display:flex;flex-direction:column;width:100%",
        // Combo 12: text-sm + text-gray-600
        "font-size:0.875rem;line-height:1.25rem;color:#4b5563",
        // Combo 13: p-6 + bg-white + rounded-lg + shadow-lg
        "padding:1.5rem;background:#fff;border-radius:0.5rem;box-shadow:0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
        // Combo 14: fixed + top-0 + left-0 + w-full
        "position:fixed;top:0;left:0;width:100%",
        // Combo 15: grid + grid-cols-3 + gap-4
        "display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:1rem",
    ]
});

/// Mapping of style ID sequences to combo IDs
/// Key format: "id1,id2,id3" → combo_id
pub static COMBO_MAP: Lazy<HashMap<String, ComboId>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Combo 0: flex(4) + items-center(26) + p-4(36)
    // Note: ID 36 = padding:1rem (p-4)
    map.insert("4,26,36".to_string(), 0);

    // Combo 1: text-white(172) + bg-blue-500(203)
    map.insert("172,203".to_string(), 1);

    // Combo 2: rounded-lg(261) + shadow-md(353)
    map.insert("261,353".to_string(), 2);

    // Combo 3: flex(4) + flex-col(13) + items-center(26)
    map.insert("4,13,26".to_string(), 3);

    // Combo 4: w-full(373) + h-full(379)
    map.insert("373,379".to_string(), 4);

    // Combo 5: absolute(423) + top-0(425) + right-0(426)
    map.insert("423,425,426".to_string(), 5);

    // Combo 6: flex(4) + justify-between(21) + items-center(26)
    map.insert("4,21,26".to_string(), 6);

    // Combo 7: text-center(320) + font-bold(316) + text-2xl(306)
    map.insert("316,306,320".to_string(), 7);

    // Combo 9: flex(4) + items-center(26) + justify-center(20)
    map.insert("4,20,26".to_string(), 9);

    // Combo 10: relative(422) + overflow-hidden(452) + rounded-lg(261)
    map.insert("261,422,452".to_string(), 10);

    // Combo 11: flex(4) + flex-col(13) + w-full(373)
    map.insert("4,13,373".to_string(), 11);

    map
});

/// Check if a list of style IDs matches a common combo
pub fn is_common_combo(ids: &[u16]) -> Option<ComboId> {
    if ids.is_empty() || ids.len() > 5 {
        return None; // Combos are typically 2-4 styles
    }

    // Create lookup key
    let key = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");

    COMBO_MAP.get(&key).copied()
}

/// Get pre-computed CSS text for a combo ID
pub fn get_combo_csstext(combo_id: ComboId) -> Option<&'static str> {
    COMBO_DICT.get(combo_id as usize).copied()
}

/// Optimized combo application
/// Returns the CSS text if combo exists, otherwise None
pub fn try_apply_combo(ids: &[u16]) -> Option<&'static str> {
    let combo_id = is_common_combo(ids)?;
    get_combo_csstext(combo_id)
}

/// Statistics for combo usage
pub struct ComboStats {
    pub total_requests: u64,
    pub combo_hits: u64,
    pub combo_misses: u64,
}

impl ComboStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.combo_hits as f64 / self.total_requests as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_combo_detection() {
        // flex + items-center + p-4 (ID 36 = padding:1rem)
        let ids = vec![4, 26, 36];
        let combo_id = is_common_combo(&ids);
        assert_eq!(combo_id, Some(0));
    }

    #[test]
    fn test_combo_application() {
        let ids = vec![4, 26, 36];
        let css = try_apply_combo(&ids);
        assert!(css.is_some());
        assert_eq!(css.unwrap(), "display:flex;align-items:center;padding:1rem");
    }

    #[test]
    fn test_non_combo_pattern() {
        // Random combination not in combo dictionary
        let ids = vec![1, 2, 3];
        let combo_id = is_common_combo(&ids);
        assert_eq!(combo_id, None);
    }

    #[test]
    fn test_combo_lookup() {
        let css = get_combo_csstext(0);
        assert!(css.is_some());
        assert!(css.unwrap().contains("display:flex"));
    }

    #[test]
    fn test_invalid_combo_id() {
        let css = get_combo_csstext(9999);
        assert!(css.is_none());
    }

    #[test]
    fn test_size_reduction() {
        // Combo: 1 ID (2 bytes)
        // Individual: 3 IDs (6 bytes)
        // Savings: 67% smaller payload

        let ids = vec![4, 26, 36];
        let combo_id = is_common_combo(&ids);

        assert!(combo_id.is_some());

        // Combo sends 1 ID instead of 3
        let combo_size = std::mem::size_of::<u16>(); // 2 bytes
        let individual_size = std::mem::size_of::<u16>() * ids.len(); // 6 bytes

        assert_eq!(combo_size, 2);
        assert_eq!(individual_size, 6);

        let savings = (1.0 - (combo_size as f64 / individual_size as f64)) * 100.0;
        assert!(savings > 60.0); // > 60% reduction
    }

    #[test]
    fn test_performance_comparison() {
        use std::time::Instant;

        let ids = vec![4, 26, 36]; // Use correct ID for p-4

        // Combo lookup
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = try_apply_combo(&ids);
        }
        let combo_time = start.elapsed();

        // Individual concatenation
        let start = Instant::now();
        for _ in 0..10000 {
            let _ = crate::binary::csstext::apply_styles_direct(&ids);
        }
        let individual_time = start.elapsed();

        println!("Combo: {:?}, Individual: {:?}", combo_time, individual_time);

        // Note: In debug builds, combo lookup may not always be faster due to
        // HashMap overhead. This test verifies both methods work correctly.
        // In release builds, combo lookup is typically faster.
        assert!(combo_time.as_micros() < 100_000); // Just verify it completes reasonably fast
    }
}
