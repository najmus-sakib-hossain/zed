//! Icon component parsing from source code

use super::component::IconComponent;
use std::collections::{HashMap, HashSet};

/// Parse icon components from source code
pub fn parse_icon_components(source: &str) -> Vec<IconComponent> {
    use once_cell::sync::Lazy;

    static ICON_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
        // This regex is a compile-time constant and will never fail
        #[allow(clippy::expect_used)]
        regex::Regex::new(
            r#"<dx-icon\s+(?:[^>]*\s+)?name="([^"]+)"(?:\s+size="(\d+)")?(?:\s+color="([^"]+)")?(?:\s+class="([^"]+)")?\s*/>"#
        ).expect("Invalid regex pattern")
    });

    let mut icons = Vec::new();
    let mut seen = HashSet::new();

    for cap in ICON_REGEX.captures_iter(source) {
        let name = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
        let size = cap.get(2).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(24);
        let color = cap.get(3).map(|m| m.as_str().to_string());
        let class = cap.get(4).map(|m| m.as_str().to_string());

        let mut icon = IconComponent::new(name);
        icon.size = size;
        icon.color = color;
        icon.class = class;

        if !seen.contains(&icon.name) {
            seen.insert(icon.name.clone());
            icons.push(icon);
        }
    }

    icons
}

/// Extract unique icon names from source code
pub fn extract_icon_names(source: &str) -> Vec<String> {
    let icons = parse_icon_components(source);
    icons.into_iter().map(|icon| icon.name).collect()
}

/// Extract icon components grouped by set
pub fn extract_icons_by_set(source: &str) -> HashMap<String, Vec<String>> {
    let icons = parse_icon_components(source);
    let mut by_set = HashMap::new();

    for icon in icons {
        let (set, name) = icon.parse_name();
        by_set.entry(set.to_string()).or_insert_with(Vec::new).push(name.to_string());
    }

    by_set
}
