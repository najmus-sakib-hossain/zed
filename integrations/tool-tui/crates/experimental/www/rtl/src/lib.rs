//! # dx-rtl — Right-to-Left Detection
//!
//! Automatic detection and CSS flipping for RTL languages.
//!
//! ## Features
//! - Language detection
//! - Automatic dir attribute
//! - CSS property flipping
//! - Bidirectional text support

#![forbid(unsafe_code)]
#![allow(clippy::collapsible_if)] // Nested if statements improve readability for CSS parsing

use unic_langid::LanguageIdentifier;

/// RTL languages (ISO 639-1 codes)
const RTL_LANGUAGES: &[&str] = &[
    "ar", // Arabic
    "he", // Hebrew
    "fa", // Persian/Farsi
    "ur", // Urdu
    "yi", // Yiddish
    "ji", // Yiddish (alternative)
    "iw", // Hebrew (old code)
    "ps", // Pashto
    "sd", // Sindhi
    "ug", // Uyghur
];

/// Text direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDirection {
    LeftToRight,
    RightToLeft,
}

impl TextDirection {
    /// Convert to HTML dir attribute
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LeftToRight => "ltr",
            Self::RightToLeft => "rtl",
        }
    }
}

/// Language detector
pub struct LanguageDetector;

impl LanguageDetector {
    /// Detect if language is RTL
    pub fn is_rtl(lang_code: &str) -> bool {
        // Try to parse as language identifier
        if let Ok(lang_id) = lang_code.parse::<LanguageIdentifier>() {
            let lang_str = lang_id.language.as_str();
            RTL_LANGUAGES.contains(&lang_str)
        } else {
            // Fallback: check if first 2 chars match
            let code = lang_code.get(0..2).unwrap_or("");
            RTL_LANGUAGES.contains(&code)
        }
    }

    /// Get text direction for language
    pub fn get_direction(lang_code: &str) -> TextDirection {
        if Self::is_rtl(lang_code) {
            TextDirection::RightToLeft
        } else {
            TextDirection::LeftToRight
        }
    }

    /// Detect from Accept-Language header
    pub fn from_accept_language(header: &str) -> TextDirection {
        // Parse first language from header
        let first_lang = header.split(',').next().and_then(|s| s.split(';').next()).unwrap_or("en");

        Self::get_direction(first_lang.trim())
    }
}

/// CSS property flipper
pub struct CSSFlipper;

impl CSSFlipper {
    /// Flip CSS property for RTL
    pub fn flip_property(property: &str, value: &str) -> Option<(String, String)> {
        // Flip directional properties
        let flipped_prop = match property {
            "margin-left" => "margin-right",
            "margin-right" => "margin-left",
            "padding-left" => "padding-right",
            "padding-right" => "padding-left",
            "left" => "right",
            "right" => "left",
            "border-left" => "border-right",
            "border-right" => "border-left",
            "border-left-width" => "border-right-width",
            "border-right-width" => "border-left-width",
            "border-top-left-radius" => "border-top-right-radius",
            "border-top-right-radius" => "border-top-left-radius",
            "border-bottom-left-radius" => "border-bottom-right-radius",
            "border-bottom-right-radius" => "border-bottom-left-radius",
            _ => return None,
        };

        Some((flipped_prop.to_string(), value.to_string()))
    }

    /// Generate RTL version of CSS
    pub fn generate_rtl_css(ltr_css: &str) -> String {
        let mut rtl_css = String::new();
        rtl_css.push_str("[dir=\"rtl\"] {\n");

        // Simple property flipping (in production, would use CSS parser)
        for line in ltr_css.lines() {
            if let Some((prop, val)) = Self::parse_css_line(line) {
                if let Some((flipped_prop, flipped_val)) = Self::flip_property(&prop, &val) {
                    rtl_css.push_str(&format!("  {}: {};\n", flipped_prop, flipped_val));
                }
            }
        }

        rtl_css.push_str("}\n");
        rtl_css
    }

    /// Parse CSS line (simplified)
    fn parse_css_line(line: &str) -> Option<(String, String)> {
        let trimmed = line.trim();
        if let Some(colon_pos) = trimmed.find(':') {
            let prop = trimmed[..colon_pos].trim().to_string();
            let val = trimmed[colon_pos + 1..].trim().trim_end_matches(';').to_string();
            Some((prop, val))
        } else {
            None
        }
    }
}

/// RTL configuration
#[derive(Debug, Clone)]
pub struct RTLConfig {
    pub auto_detect: bool,
    pub default_direction: TextDirection,
    pub flip_css: bool,
}

impl Default for RTLConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            default_direction: TextDirection::LeftToRight,
            flip_css: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtl_detection() {
        assert!(LanguageDetector::is_rtl("ar"));
        assert!(LanguageDetector::is_rtl("he"));
        assert!(LanguageDetector::is_rtl("fa"));
        assert!(!LanguageDetector::is_rtl("en"));
        assert!(!LanguageDetector::is_rtl("fr"));
    }

    #[test]
    fn test_direction() {
        assert_eq!(LanguageDetector::get_direction("ar"), TextDirection::RightToLeft);
        assert_eq!(LanguageDetector::get_direction("en"), TextDirection::LeftToRight);
    }

    #[test]
    fn test_accept_language() {
        assert_eq!(
            LanguageDetector::from_accept_language("ar-SA,ar;q=0.9"),
            TextDirection::RightToLeft
        );
        assert_eq!(
            LanguageDetector::from_accept_language("en-US,en;q=0.9"),
            TextDirection::LeftToRight
        );
    }

    #[test]
    fn test_css_flip() {
        let (prop, val) = CSSFlipper::flip_property("margin-left", "10px").unwrap();
        assert_eq!(prop, "margin-right");
        assert_eq!(val, "10px");

        let (prop, val) = CSSFlipper::flip_property("padding-right", "5px").unwrap();
        assert_eq!(prop, "padding-left");
        assert_eq!(val, "5px");
    }

    #[test]
    fn test_no_flip() {
        assert!(CSSFlipper::flip_property("color", "red").is_none());
        assert!(CSSFlipper::flip_property("font-size", "14px").is_none());
    }

    #[test]
    fn test_text_direction_str() {
        assert_eq!(TextDirection::LeftToRight.as_str(), "ltr");
        assert_eq!(TextDirection::RightToLeft.as_str(), "rtl");
    }
}
