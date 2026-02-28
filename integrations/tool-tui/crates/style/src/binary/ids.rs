use once_cell::sync::Lazy;
/// Level 1: Binary Style ID System
///
/// Maps CSS utility class names to u16 binary IDs
/// Stores pre-computed CSS strings in static arrays
use std::collections::HashMap;

/// Style ID type - u16 allows 65,536 unique utilities
pub type StyleId = u16;

/// Static dictionary mapping style IDs to CSS text
/// This is pre-computed at compile time
pub static STYLE_DICT: Lazy<Vec<&'static str>> = Lazy::new(|| {
    vec![
        // Display utilities (0-10)
        "display:none",
        "display:block",
        "display:inline",
        "display:inline-block",
        "display:flex",
        "display:inline-flex",
        "display:grid",
        "display:inline-grid",
        "display:table",
        "display:table-row",
        "display:table-cell",
        // Flexbox utilities (11-30)
        "flex-direction:row",
        "flex-direction:row-reverse",
        "flex-direction:column",
        "flex-direction:column-reverse",
        "flex-wrap:nowrap",
        "flex-wrap:wrap",
        "flex-wrap:wrap-reverse",
        "justify-content:flex-start",
        "justify-content:flex-end",
        "justify-content:center",
        "justify-content:space-between",
        "justify-content:space-around",
        "justify-content:space-evenly",
        "align-items:flex-start",
        "align-items:flex-end",
        "align-items:center",
        "align-items:baseline",
        "align-items:stretch",
        "align-self:auto",
        "align-self:flex-start",
        "align-self:flex-end",
        // Spacing - Padding (31-100)
        "padding:0",
        "padding:0.25rem",
        "padding:0.5rem",
        "padding:0.75rem",
        "padding:1rem",
        "padding:1.25rem",
        "padding:1.5rem",
        "padding:1.75rem",
        "padding:2rem",
        "padding:2.5rem",
        "padding:3rem",
        "padding:4rem",
        "padding-left:0",
        "padding-left:0.25rem",
        "padding-left:0.5rem",
        "padding-left:0.75rem",
        "padding-left:1rem",
        "padding-left:1.5rem",
        "padding-left:2rem",
        "padding-right:0",
        "padding-right:0.25rem",
        "padding-right:0.5rem",
        "padding-right:0.75rem",
        "padding-right:1rem",
        "padding-right:1.5rem",
        "padding-right:2rem",
        "padding-top:0",
        "padding-top:0.25rem",
        "padding-top:0.5rem",
        "padding-top:0.75rem",
        "padding-top:1rem",
        "padding-top:1.5rem",
        "padding-top:2rem",
        "padding-bottom:0",
        "padding-bottom:0.25rem",
        "padding-bottom:0.5rem",
        "padding-bottom:0.75rem",
        "padding-bottom:1rem",
        "padding-bottom:1.5rem",
        "padding-bottom:2rem",
        // Spacing - Margin (101-170)
        "margin:0",
        "margin:0.25rem",
        "margin:0.5rem",
        "margin:0.75rem",
        "margin:1rem",
        "margin:1.5rem",
        "margin:2rem",
        "margin:auto",
        "margin-left:0",
        "margin-left:0.25rem",
        "margin-left:0.5rem",
        "margin-left:0.75rem",
        "margin-left:1rem",
        "margin-left:1.5rem",
        "margin-left:2rem",
        "margin-left:auto",
        "margin-right:0",
        "margin-right:0.25rem",
        "margin-right:0.5rem",
        "margin-right:0.75rem",
        "margin-right:1rem",
        "margin-right:1.5rem",
        "margin-right:2rem",
        "margin-right:auto",
        "margin-top:0",
        "margin-top:0.25rem",
        "margin-top:0.5rem",
        "margin-top:0.75rem",
        "margin-top:1rem",
        "margin-top:1.5rem",
        "margin-top:2rem",
        "margin-top:auto",
        "margin-bottom:0",
        "margin-bottom:0.25rem",
        "margin-bottom:0.5rem",
        "margin-bottom:0.75rem",
        "margin-bottom:1rem",
        "margin-bottom:1.5rem",
        "margin-bottom:2rem",
        "margin-bottom:auto",
        // Colors (171-250)
        "color:#000",
        "color:#fff",
        "color:#ef4444", // red-500
        "color:#f97316", // orange-500
        "color:#f59e0b", // amber-500
        "color:#eab308", // yellow-500
        "color:#84cc16", // lime-500
        "color:#22c55e", // green-500
        "color:#10b981", // emerald-500
        "color:#14b8a6", // teal-500
        "color:#06b6d4", // cyan-500
        "color:#0ea5e9", // sky-500
        "color:#3b82f6", // blue-500
        "color:#6366f1", // indigo-500
        "color:#8b5cf6", // violet-500
        "color:#a855f7", // purple-500
        "color:#d946ef", // fuchsia-500
        "color:#ec4899", // pink-500
        "color:#f43f5e", // rose-500
        "background:#000",
        "background:#fff",
        "background:#ef4444",
        "background:#f97316",
        "background:#f59e0b",
        "background:#eab308",
        "background:#84cc16",
        "background:#22c55e",
        "background:#10b981",
        "background:#14b8a6",
        "background:#06b6d4",
        "background:#0ea5e9",
        "background:#3b82f6",
        "background:#6366f1",
        "background:#8b5cf6",
        "background:#a855f7",
        "background:#d946ef",
        "background:#ec4899",
        "background:#f43f5e",
        // Border (251-300)
        "border:1px solid #e5e7eb",
        "border:2px solid #e5e7eb",
        "border-width:0",
        "border-width:1px",
        "border-width:2px",
        "border-width:4px",
        "border-radius:0",
        "border-radius:0.125rem",
        "border-radius:0.25rem",
        "border-radius:0.375rem",
        "border-radius:0.5rem",
        "border-radius:0.75rem",
        "border-radius:1rem",
        "border-radius:9999px",
        // Typography (301-350)
        "font-size:0.75rem;line-height:1rem",
        "font-size:0.875rem;line-height:1.25rem",
        "font-size:1rem;line-height:1.5rem",
        "font-size:1.125rem;line-height:1.75rem",
        "font-size:1.25rem;line-height:1.75rem",
        "font-size:1.5rem;line-height:2rem",
        "font-size:1.875rem;line-height:2.25rem",
        "font-size:2.25rem;line-height:2.5rem",
        "font-size:3rem;line-height:1",
        "font-weight:100",
        "font-weight:200",
        "font-weight:300",
        "font-weight:400",
        "font-weight:500",
        "font-weight:600",
        "font-weight:700",
        "font-weight:800",
        "font-weight:900",
        "text-align:left",
        "text-align:center",
        "text-align:right",
        "text-align:justify",
        // Shadow (351-370)
        "box-shadow:0 1px 2px 0 rgb(0 0 0 / 0.05)",
        "box-shadow:0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)",
        "box-shadow:0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)",
        "box-shadow:0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)",
        "box-shadow:0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)",
        "box-shadow:0 25px 50px -12px rgb(0 0 0 / 0.25)",
        // Width/Height (371-420)
        "width:auto",
        "width:50%",
        "width:100%",
        "width:100vw",
        "width:min-content",
        "width:max-content",
        "height:auto",
        "height:50%",
        "height:100%",
        "height:100vh",
        "height:min-content",
        "height:max-content",
        // Position (421-450)
        "position:static",
        "position:relative",
        "position:absolute",
        "position:fixed",
        "position:sticky",
        "top:0",
        "right:0",
        "bottom:0",
        "left:0",
        "z-index:0",
        "z-index:10",
        "z-index:20",
        "z-index:30",
        "z-index:40",
        "z-index:50",
        // Overflow (451-460)
        "overflow:visible",
        "overflow:hidden",
        "overflow:scroll",
        "overflow:auto",
        "overflow-x:visible",
        "overflow-x:hidden",
        "overflow-x:scroll",
        "overflow-x:auto",
        "overflow-y:visible",
        "overflow-y:hidden",
        "overflow-y:scroll",
        "overflow-y:auto",
    ]
});

/// Reverse mapping: class name → style ID
pub static CLASS_TO_ID: Lazy<HashMap<&'static str, StyleId>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Display
    map.insert("hidden", 0);
    map.insert("block", 1);
    map.insert("inline", 2);
    map.insert("inline-block", 3);
    map.insert("flex", 4);
    map.insert("inline-flex", 5);
    map.insert("grid", 6);
    map.insert("inline-grid", 7);
    map.insert("table", 8);
    map.insert("table-row", 9);
    map.insert("table-cell", 10);

    // Flexbox
    map.insert("flex-row", 11);
    map.insert("flex-row-reverse", 12);
    map.insert("flex-col", 13);
    map.insert("flex-col-reverse", 14);
    map.insert("flex-nowrap", 15);
    map.insert("flex-wrap", 16);
    map.insert("flex-wrap-reverse", 17);
    map.insert("justify-start", 18);
    map.insert("justify-end", 19);
    map.insert("justify-center", 20);
    map.insert("justify-between", 21);
    map.insert("justify-around", 22);
    map.insert("justify-evenly", 23);
    map.insert("items-start", 24);
    map.insert("items-end", 25);
    map.insert("items-center", 26);
    map.insert("items-baseline", 27);
    map.insert("items-stretch", 28);

    // Padding
    // Note: STYLE_DICT indices for padding:
    // 32 = padding:0, 33 = padding:0.25rem, 34 = padding:0.5rem,
    // 35 = padding:0.75rem, 36 = padding:1rem, 37 = padding:1.25rem, etc.
    map.insert("p-0", 32);
    map.insert("p-1", 33);
    map.insert("p-2", 34);
    map.insert("p-3", 35);
    map.insert("p-4", 36); // padding:1rem is at index 36
    map.insert("p-5", 37);
    map.insert("p-6", 38);
    map.insert("p-8", 40);

    // Colors
    map.insert("text-black", 171);
    map.insert("text-white", 172);
    map.insert("text-red-500", 173);
    map.insert("text-blue-500", 183);
    map.insert("bg-black", 189);
    map.insert("bg-white", 190);
    map.insert("bg-red-500", 191);
    map.insert("bg-blue-500", 203);

    // Border
    map.insert("border", 251);
    map.insert("border-2", 252);
    map.insert("rounded", 258);
    map.insert("rounded-md", 259);
    map.insert("rounded-lg", 261);
    map.insert("rounded-full", 263);

    // Typography
    map.insert("text-xs", 301);
    map.insert("text-sm", 302);
    map.insert("text-base", 303);
    map.insert("text-lg", 304);
    map.insert("text-xl", 305);
    map.insert("text-2xl", 306);
    map.insert("font-bold", 316);
    map.insert("text-center", 320);

    // Shadow
    map.insert("shadow", 352);
    map.insert("shadow-md", 353);
    map.insert("shadow-lg", 354);

    // Width/Height
    map.insert("w-full", 373);
    map.insert("h-full", 379);

    // Position
    map.insert("relative", 422);
    map.insert("absolute", 423);
    map.insert("fixed", 424);

    map
});

/// Convert a class name to a style ID
pub fn style_name_to_id(class: &str) -> Option<StyleId> {
    CLASS_TO_ID.get(class).copied()
}

/// Convert a style ID to CSS text
pub fn style_id_to_csstext(id: StyleId) -> Option<&'static str> {
    STYLE_DICT.get(id as usize).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_lookup() {
        assert_eq!(style_name_to_id("flex"), Some(4));
        assert_eq!(style_id_to_csstext(4), Some("display:flex"));
    }

    #[test]
    fn test_reverse_lookup() {
        let id = style_name_to_id("items-center").unwrap();
        let css = style_id_to_csstext(id).unwrap();
        assert_eq!(css, "align-items:center");
    }

    #[test]
    fn test_invalid_lookup() {
        assert_eq!(style_name_to_id("invalid-class-name"), None);
        assert_eq!(style_id_to_csstext(9999), None);
    }
}
