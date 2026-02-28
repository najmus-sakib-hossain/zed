// Element detection and categorization

use super::config::FilterConfig;

/// Categories of markdown elements for filtering
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementCategory {
    /// CI/CD badges, version badges, etc.
    Badge,

    /// Images (screenshots, logos, diagrams)
    Image,

    /// Code examples
    Example,

    /// Documentation sections
    Section,

    /// Promotional content (star prompts, social links)
    Promotional,

    /// Decorative elements (horizontal rules, emojis)
    Decorative,

    /// Redundant information
    Redundant,

    /// Verbose content that can be summarized
    Verbose,

    /// Essential content (always keep)
    Essential,

    /// Unknown/uncategorized
    Unknown,
}

/// Categorize a markdown element
pub fn categorize(text: &str, _config: &FilterConfig) -> ElementCategory {
    let lower = text.to_lowercase();

    // Badge detection
    if is_badge(text) {
        return ElementCategory::Badge;
    }

    // Image detection
    if text.starts_with("![") {
        return ElementCategory::Image;
    }

    // Promotional detection
    if is_promotional(&lower) {
        return ElementCategory::Promotional;
    }

    // Decorative detection
    if is_decorative(text) {
        return ElementCategory::Decorative;
    }

    ElementCategory::Unknown
}

/// Check if text is a badge
fn is_badge(text: &str) -> bool {
    text.contains("img.shields.io")
        || text.contains("badge.fury.io")
        || text.contains("badgen.net")
        || text.contains("shields.io")
        || text.contains("travis-ci.org")
        || text.contains("codecov.io")
        || text.contains("coveralls.io")
        || (text.starts_with("[![") && text.contains("badge"))
}

/// Check if text is promotional
fn is_promotional(text: &str) -> bool {
    text.contains("star this")
        || text.contains("give us a star")
        || text.contains("please star")
        || text.contains("buy me a coffee")
        || text.contains("sponsor")
        || text.contains("donate")
        || text.contains("patreon")
        || text.contains("ko-fi")
}

/// Check if text is decorative
fn is_decorative(text: &str) -> bool {
    // Horizontal rules
    if text.trim() == "---" || text.trim() == "***" {
        return true;
    }

    // Emoji-only lines
    if text.chars().all(|c| is_emoji(c) || c.is_whitespace()) {
        return true;
    }

    false
}

/// Check if character is an emoji
fn is_emoji(c: char) -> bool {
    matches!(c,
        '\u{1F300}'..='\u{1F9FF}' | // Misc Symbols and Pictographs
        '\u{2600}'..='\u{26FF}' |   // Misc symbols
        '\u{2700}'..='\u{27BF}'     // Dingbats
    )
}
