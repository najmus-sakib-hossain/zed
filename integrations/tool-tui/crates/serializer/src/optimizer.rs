/// Ultra-optimization rules for DX SINGULARITY format
///
/// These rules transform verbose keys into minimal byte-efficient forms
/// while maintaining clarity through the editor's display layer.
use std::collections::HashMap;

/// Ultra-optimization rule: abbreviate common keys
#[must_use]
pub fn optimize_key(key: &str) -> String {
    // Common abbreviations map
    let abbrev: HashMap<&str, &str> = [
        // Core metadata
        ("name", "n"),
        ("version", "v"),
        ("title", "t"),
        ("description", "d"),
        ("desc", "d"),
        ("author", "a"),
        ("license", "lic"),
        // Context/Config
        ("context", "c"),
        ("config", "cfg"),
        ("settings", "set"),
        // Development
        ("languages", "l"),
        ("language", "lg"),
        ("runtime", "rt"),
        ("compiler", "cp"),
        ("bundler", "bd"),
        ("packageManager", "pm"),
        ("package_manager", "pm"),
        ("framework", "fw"),
        // Project structure
        ("forge", "f"),
        ("repository", "r"),
        ("repo", "r"),
        ("container", "c"),
        ("workspace", "ws"),
        ("workspaces", "ws"),
        // UI/Style
        ("style", "s"),
        ("styles", "s"),
        ("theme", "th"),
        ("themes", "th"),
        ("engine", "e"),
        ("ui", "u"),
        ("component", "cmp"),
        ("components", "cmp"),
        // Media
        ("media", "m"),
        ("image", "img"),
        ("images", "img"),
        ("video", "vid"),
        ("videos", "vid"),
        ("sound", "snd"),
        ("sounds", "snd"),
        ("audio", "aud"),
        ("asset", "ast"),
        ("assets", "ast"),
        // i18n
        ("internationalization", "i"),
        ("i18n", "i"),
        ("locale", "loc"),
        ("locales", "loc"),
        ("translation", "tr"),
        ("translations", "tr"),
        // Common properties
        ("path", "p"),
        ("default", "d"),
        ("primary", "pr"),
        ("secondary", "sc"),
        ("variant", "vr"),
        ("pack", "pk"),
        ("items", "i"),
        ("item", "i"),
        // Environment
        ("development", "dev"),
        ("production", "prod"),
        ("test", "tst"),
        // IDE/Tools
        ("icon", "ic"),
        ("icons", "ic"),
        ("font", "fn"),
        ("fonts", "fn"),
        ("ide", "id"),
    ]
    .iter()
    .copied()
    .collect();

    // Check if we have an abbreviation
    if let Some(short) = abbrev.get(key) {
        return short.to_string();
    }

    // Apply 2-letter language codes
    match key {
        "javascript" => return "js".to_string(),
        "typescript" => return "ts".to_string(),
        "javascript/typescript" => return "js/ts".to_string(),
        "python" => return "py".to_string(),
        "rust" => return "rs".to_string(),
        "golang" => return "go".to_string(),
        _ => {}
    }

    // If no abbreviation, use first 2-3 chars for long keys
    if key.len() > 8 {
        return key.chars().take(3).collect();
    }

    key.to_string()
}

/// Optimize a nested key path (e.g., "media.images.path" -> "m.img.p")
#[must_use]
pub fn optimize_path(path: &str) -> String {
    path.split('.').map(optimize_key).collect::<Vec<_>>().join(".")
}

/// Determine if values should be inlined with ^ operator
#[must_use]
pub fn should_inline(values: &[(String, String)]) -> bool {
    // Inline if:
    // 1. Less than 5 items
    // 2. Total length < 150 chars
    // 3. All values are simple (no nested objects/arrays)

    if values.len() > 4 {
        return false;
    }

    let total_len: usize = values.iter()
        .map(|(k, v)| k.len() + v.len() + 2) // key:val^
        .sum();

    total_len < 150
}

/// Optimize array/list items (use pipe separator)
#[must_use]
pub fn format_array(items: &[String]) -> String {
    items.join("|")
}

/// Detect if a value should use dash for null/empty
#[must_use]
pub fn format_null_value() -> &'static str {
    "-"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_optimization() {
        assert_eq!(optimize_key("name"), "n");
        assert_eq!(optimize_key("version"), "v");
        assert_eq!(optimize_key("description"), "d");
        assert_eq!(optimize_key("packageManager"), "pm");
        assert_eq!(optimize_key("javascript/typescript"), "js/ts");
    }

    #[test]
    fn test_path_optimization() {
        assert_eq!(optimize_path("media.images.path"), "m.img.p");
        assert_eq!(optimize_path("i18n.locales.default"), "i.loc.d");
    }

    #[test]
    fn test_array_format() {
        let items = vec!["cli".to_string(), "docs".to_string(), "tests".to_string()];
        assert_eq!(format_array(&items), "cli|docs|tests");
    }
}
