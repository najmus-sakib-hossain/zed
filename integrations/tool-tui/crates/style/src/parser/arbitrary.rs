//! Arbitrary Value Parser
//!
//! Parses bracket syntax for arbitrary CSS values (e.g., `w-[calc(100%-20px)]`).
//! Supports arbitrary colors, spacing, and CSS functions.
//!
//! **Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5, 7.6**

#![allow(dead_code)]

/// Parsed arbitrary value from bracket syntax
#[derive(Debug, Clone, PartialEq)]
pub struct ArbitraryValue {
    /// CSS property name
    pub property: String,
    /// CSS value (with underscores converted to spaces)
    pub value: String,
    /// Original class name
    pub original: String,
}

/// Warning for invalid arbitrary values
#[derive(Debug, Clone)]
pub struct ArbitraryWarning {
    pub class_name: String,
    pub message: String,
}

/// Parser for arbitrary CSS values in brackets
///
/// Handles syntax like:
/// - `w-[calc(100%-20px)]` → `width: calc(100% - 20px)`
/// - `bg-[#ff5500]` → `background: #ff5500`
/// - `p-[17px]` → `padding: 17px`
/// - `bg-[linear-gradient(to_right,red,blue)]` → `background: linear-gradient(to right,red,blue)`
pub struct ArbitraryValueParser {
    warnings: Vec<ArbitraryWarning>,
}

impl ArbitraryValueParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    /// Parse a class with arbitrary value (e.g., "w-[calc(100%-20px)]")
    ///
    /// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
    pub fn parse(&mut self, class: &str) -> Option<ArbitraryValue> {
        let bracket_start = class.find('[')?;
        let bracket_end = class.rfind(']')?;

        if bracket_end <= bracket_start {
            return None;
        }

        let prefix = &class[..bracket_start];
        let value = &class[bracket_start + 1..bracket_end];

        // Determine property from prefix
        let property = Self::prefix_to_property(prefix)?;

        // Unescape underscores to spaces (Tailwind convention)
        let css_value = value.replace('_', " ");

        // Validate the CSS value
        if !self.is_valid_css_value(&css_value) {
            self.warnings.push(ArbitraryWarning {
                class_name: class.to_string(),
                message: format!("Invalid CSS value: '{}'", css_value),
            });
            return None;
        }

        Some(ArbitraryValue {
            property,
            value: css_value,
            original: class.to_string(),
        })
    }

    /// Map prefix to CSS property
    ///
    /// **Validates: Requirements 7.1**
    fn prefix_to_property(prefix: &str) -> Option<String> {
        // Remove trailing dash if present
        let prefix = prefix.trim_end_matches('-');

        match prefix {
            // Sizing
            "w" => Some("width".into()),
            "h" => Some("height".into()),
            "min-w" => Some("min-width".into()),
            "min-h" => Some("min-height".into()),
            "max-w" => Some("max-width".into()),
            "max-h" => Some("max-height".into()),
            "size" => Some("width".into()), // Also sets height via aspect-ratio

            // Spacing
            "p" => Some("padding".into()),
            "px" => Some("padding-inline".into()),
            "py" => Some("padding-block".into()),
            "pt" => Some("padding-top".into()),
            "pr" => Some("padding-right".into()),
            "pb" => Some("padding-bottom".into()),
            "pl" => Some("padding-left".into()),
            "m" => Some("margin".into()),
            "mx" => Some("margin-inline".into()),
            "my" => Some("margin-block".into()),
            "mt" => Some("margin-top".into()),
            "mr" => Some("margin-right".into()),
            "mb" => Some("margin-bottom".into()),
            "ml" => Some("margin-left".into()),

            // Position
            "top" => Some("top".into()),
            "right" => Some("right".into()),
            "bottom" => Some("bottom".into()),
            "left" => Some("left".into()),
            "inset" => Some("inset".into()),
            "inset-x" => Some("inset-inline".into()),
            "inset-y" => Some("inset-block".into()),

            // Colors
            "bg" => Some("background".into()),
            "text" => Some("color".into()),
            "border" => Some("border-color".into()),
            "outline" => Some("outline-color".into()),
            "ring" => Some("--tw-ring-color".into()),
            "shadow" => Some("--tw-shadow-color".into()),
            "accent" => Some("accent-color".into()),
            "caret" => Some("caret-color".into()),
            "fill" => Some("fill".into()),
            "stroke" => Some("stroke".into()),

            // Typography
            "font" => Some("font-family".into()),
            "text-size" | "leading" => Some("line-height".into()),
            "tracking" => Some("letter-spacing".into()),

            // Layout
            "gap" => Some("gap".into()),
            "gap-x" => Some("column-gap".into()),
            "gap-y" => Some("row-gap".into()),
            "basis" => Some("flex-basis".into()),
            "grow" => Some("flex-grow".into()),
            "shrink" => Some("flex-shrink".into()),
            "order" => Some("order".into()),
            "z" => Some("z-index".into()),

            // Border
            "rounded" => Some("border-radius".into()),
            "border-w" => Some("border-width".into()),

            // Effects
            "opacity" => Some("opacity".into()),
            "blur" => Some("filter".into()),
            "brightness" => Some("filter".into()),
            "contrast" => Some("filter".into()),
            "grayscale" => Some("filter".into()),
            "saturate" => Some("filter".into()),
            "sepia" => Some("filter".into()),

            // Transform
            "rotate" => Some("rotate".into()),
            "scale" => Some("scale".into()),
            "translate-x" => Some("--tw-translate-x".into()),
            "translate-y" => Some("--tw-translate-y".into()),
            "skew-x" => Some("--tw-skew-x".into()),
            "skew-y" => Some("--tw-skew-y".into()),

            // Transition
            "duration" => Some("transition-duration".into()),
            "delay" => Some("transition-delay".into()),
            "ease" => Some("transition-timing-function".into()),

            // Grid
            "cols" => Some("grid-template-columns".into()),
            "rows" => Some("grid-template-rows".into()),
            "col" => Some("grid-column".into()),
            "row" => Some("grid-row".into()),
            "col-start" => Some("grid-column-start".into()),
            "col-end" => Some("grid-column-end".into()),
            "row-start" => Some("grid-row-start".into()),
            "row-end" => Some("grid-row-end".into()),

            _ => None,
        }
    }

    /// Check if a CSS value is valid (basic validation)
    ///
    /// **Validates: Requirements 7.6**
    fn is_valid_css_value(&self, value: &str) -> bool {
        if value.is_empty() {
            return false;
        }

        // Check for balanced parentheses
        let mut paren_depth = 0i32;
        for ch in value.chars() {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    paren_depth -= 1;
                    if paren_depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        if paren_depth != 0 {
            return false;
        }

        // Check for balanced brackets
        let mut bracket_depth = 0i32;
        for ch in value.chars() {
            match ch {
                '[' => bracket_depth += 1,
                ']' => {
                    bracket_depth -= 1;
                    if bracket_depth < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        if bracket_depth != 0 {
            return false;
        }

        true
    }

    /// Generate CSS for an arbitrary value
    pub fn generate_css(arb: &ArbitraryValue) -> String {
        format!("{}: {}", arb.property, arb.value)
    }

    /// Escape class name for CSS selector
    ///
    /// **Validates: Requirements 7.5**
    pub fn escape_selector(class: &str) -> String {
        let mut escaped = String::with_capacity(class.len() * 2);
        for ch in class.chars() {
            match ch {
                '[' => escaped.push_str("\\["),
                ']' => escaped.push_str("\\]"),
                '(' => escaped.push_str("\\("),
                ')' => escaped.push_str("\\)"),
                '%' => escaped.push_str("\\%"),
                '/' => escaped.push_str("\\/"),
                ':' => escaped.push_str("\\:"),
                '#' => escaped.push_str("\\#"),
                ',' => escaped.push_str("\\,"),
                '+' => escaped.push_str("\\+"),
                '>' => escaped.push_str("\\>"),
                '~' => escaped.push_str("\\~"),
                '.' => escaped.push_str("\\."),
                ' ' => escaped.push_str("\\ "),
                _ => escaped.push(ch),
            }
        }
        escaped
    }

    /// Get collected warnings
    pub fn warnings(&self) -> &[ArbitraryWarning] {
        &self.warnings
    }

    /// Clear collected warnings
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }
}

impl Default for ArbitraryValueParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_width() {
        let mut parser = ArbitraryValueParser::new();
        let result = parser.parse("w-[100px]");
        assert!(result.is_some());
        let arb = result.unwrap();
        assert_eq!(arb.property, "width");
        assert_eq!(arb.value, "100px");
    }

    #[test]
    fn test_parse_calc() {
        let mut parser = ArbitraryValueParser::new();
        let result = parser.parse("w-[calc(100%-20px)]");
        assert!(result.is_some());
        let arb = result.unwrap();
        assert_eq!(arb.property, "width");
        assert_eq!(arb.value, "calc(100%-20px)");
    }

    #[test]
    fn test_parse_underscore_to_space() {
        let mut parser = ArbitraryValueParser::new();
        let result = parser.parse("bg-[linear-gradient(to_right,red,blue)]");
        assert!(result.is_some());
        let arb = result.unwrap();
        assert_eq!(arb.property, "background");
        assert_eq!(arb.value, "linear-gradient(to right,red,blue)");
    }

    #[test]
    fn test_parse_color() {
        let mut parser = ArbitraryValueParser::new();
        let result = parser.parse("bg-[#ff5500]");
        assert!(result.is_some());
        let arb = result.unwrap();
        assert_eq!(arb.property, "background");
        assert_eq!(arb.value, "#ff5500");
    }

    #[test]
    fn test_escape_selector() {
        let escaped = ArbitraryValueParser::escape_selector("w-[calc(100%-20px)]");
        assert!(escaped.contains("\\["));
        assert!(escaped.contains("\\]"));
        assert!(escaped.contains("\\("));
        assert!(escaped.contains("\\)"));
        assert!(escaped.contains("\\%"));
    }

    #[test]
    fn test_generate_css() {
        let arb = ArbitraryValue {
            property: "width".to_string(),
            value: "calc(100% - 20px)".to_string(),
            original: "w-[calc(100%_-_20px)]".to_string(),
        };
        let css = ArbitraryValueParser::generate_css(&arb);
        assert_eq!(css, "width: calc(100% - 20px)");
    }

    #[test]
    fn test_invalid_unbalanced_parens() {
        let mut parser = ArbitraryValueParser::new();
        let result = parser.parse("w-[calc(100%]");
        assert!(result.is_none());
        assert!(!parser.warnings().is_empty());
    }

    #[test]
    fn test_unknown_prefix() {
        let mut parser = ArbitraryValueParser::new();
        let result = parser.parse("unknown-[value]");
        assert!(result.is_none());
    }

    #[test]
    fn test_spacing_prefixes() {
        let mut parser = ArbitraryValueParser::new();

        let result = parser.parse("pt-[10px]");
        assert!(result.is_some());
        assert_eq!(result.unwrap().property, "padding-top");

        let result = parser.parse("mx-[auto]");
        assert!(result.is_some());
        assert_eq!(result.unwrap().property, "margin-inline");
    }

    #[test]
    fn test_grid_prefixes() {
        let mut parser = ArbitraryValueParser::new();

        let result = parser.parse("cols-[repeat(3,1fr)]");
        assert!(result.is_some());
        let arb = result.unwrap();
        assert_eq!(arb.property, "grid-template-columns");
        assert_eq!(arb.value, "repeat(3,1fr)");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Arbitrary generators for testing
    fn arb_prefix() -> impl Strategy<Value = &'static str> {
        prop::sample::select(vec![
            "w", "h", "p", "m", "bg", "text", "border", "top", "right", "bottom", "left", "gap",
            "z", "opacity", "rounded", "pt", "pr", "pb", "pl", "mt", "mr", "mb", "ml",
        ])
    }

    fn arb_simple_value() -> impl Strategy<Value = String> {
        prop_oneof![
            // Pixel values
            (1i32..1000i32).prop_map(|n| format!("{}px", n)),
            // Rem values
            (1i32..100i32).prop_map(|n| format!("{}rem", n)),
            // Percentage values
            (1i32..100i32).prop_map(|n| format!("{}%", n)),
            // Hex colors
            "[0-9a-f]{6}".prop_map(|s| format!("#{}", s)),
            // Named values
            prop::sample::select(vec!["auto", "inherit", "initial", "unset"])
                .prop_map(|s| s.to_string()),
        ]
    }

    fn arb_calc_value() -> impl Strategy<Value = String> {
        (1i32..100i32, 1i32..50i32).prop_map(|(a, b)| format!("calc({}%-{}px)", a, b))
    }

    fn arb_special_char() -> impl Strategy<Value = char> {
        prop::sample::select(vec!['[', ']', '(', ')', '%', '/', ':', '#', '+', '>', '~', '.'])
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-advanced-features, Property 11: Arbitrary Value Parsing Correctness
        /// *For any* arbitrary value class (e.g., "w-[calc(100%-20px)]"), the parser SHALL
        /// correctly extract the property and value, and the generated CSS SHALL be valid.
        /// **Validates: Requirements 7.1, 7.2, 7.3, 7.4**
        #[test]
        fn prop_arbitrary_value_parsing_correctness(
            prefix in arb_prefix(),
            value in arb_simple_value()
        ) {
            let mut parser = ArbitraryValueParser::new();
            let class_name = format!("{}-[{}]", prefix, value);

            let result = parser.parse(&class_name);

            // Should successfully parse
            prop_assert!(
                result.is_some(),
                "Class '{}' should parse successfully",
                class_name
            );

            let arb = result.unwrap();

            // Property should be non-empty
            prop_assert!(
                !arb.property.is_empty(),
                "Property should not be empty for class '{}'",
                class_name
            );

            // Value should match (with underscore conversion)
            let expected_value = value.replace('_', " ");
            prop_assert!(
                arb.value == expected_value,
                "Value '{}' should match expected '{}' for class '{}'",
                arb.value, expected_value, class_name
            );

            // Original should be preserved
            prop_assert!(
                arb.original == class_name,
                "Original '{}' should match class name '{}'",
                arb.original, class_name
            );

            // Generated CSS should be valid format
            let css = ArbitraryValueParser::generate_css(&arb);
            prop_assert!(
                css.contains(": "),
                "Generated CSS '{}' should be in 'property: value' format",
                css
            );
        }

        /// Property test for calc() expressions
        /// **Validates: Requirements 7.2, 7.3**
        #[test]
        fn prop_arbitrary_calc_parsing(
            prefix in arb_prefix(),
            value in arb_calc_value()
        ) {
            let mut parser = ArbitraryValueParser::new();
            let class_name = format!("{}-[{}]", prefix, value);

            let result = parser.parse(&class_name);

            prop_assert!(
                result.is_some(),
                "Calc expression '{}' should parse successfully",
                class_name
            );

            let arb = result.unwrap();

            // Value should contain calc
            prop_assert!(
                arb.value.starts_with("calc("),
                "Value '{}' should start with 'calc('",
                arb.value
            );

            // Parentheses should be balanced
            let open_count = arb.value.chars().filter(|&c| c == '(').count();
            let close_count = arb.value.chars().filter(|&c| c == ')').count();
            prop_assert_eq!(
                open_count, close_count,
                "Parentheses should be balanced in '{}'",
                arb.value
            );
        }

        /// Property test for underscore to space conversion
        /// **Validates: Requirements 7.4**
        #[test]
        fn prop_underscore_to_space_conversion(
            prefix in arb_prefix(),
            word1 in "[a-z]{3,8}",
            word2 in "[a-z]{3,8}"
        ) {
            let mut parser = ArbitraryValueParser::new();
            let value_with_underscore = format!("{}_{}", word1, word2);
            let class_name = format!("{}-[{}]", prefix, value_with_underscore);

            let result = parser.parse(&class_name);

            prop_assert!(
                result.is_some(),
                "Class '{}' should parse successfully",
                class_name
            );

            let arb = result.unwrap();

            // Underscores should be converted to spaces
            prop_assert!(
                arb.value.contains(' '),
                "Value '{}' should contain space (converted from underscore)",
                arb.value
            );

            prop_assert!(
                !arb.value.contains('_'),
                "Value '{}' should not contain underscore after conversion",
                arb.value
            );
        }

        /// Feature: dx-style-advanced-features, Property 12: Arbitrary Value Selector Escaping
        /// *For any* arbitrary value class with special characters, the generated CSS selector
        /// SHALL properly escape all special characters.
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_arbitrary_selector_escaping(
            prefix in arb_prefix(),
            value in arb_simple_value()
        ) {
            let class_name = format!("{}-[{}]", prefix, value);
            let escaped = ArbitraryValueParser::escape_selector(&class_name);

            // Brackets should be escaped
            prop_assert!(
                !escaped.contains("[") || escaped.contains("\\["),
                "Open bracket should be escaped in '{}'",
                escaped
            );
            prop_assert!(
                !escaped.contains("]") || escaped.contains("\\]"),
                "Close bracket should be escaped in '{}'",
                escaped
            );

            // The escaped selector should be longer or equal (escaping adds backslashes)
            prop_assert!(
                escaped.len() >= class_name.len(),
                "Escaped selector '{}' should be at least as long as original '{}'",
                escaped, class_name
            );
        }

        /// Property test for special character escaping
        /// **Validates: Requirements 7.5**
        #[test]
        fn prop_special_char_escaping(
            ch in arb_special_char()
        ) {
            let class_name = format!("test{}class", ch);
            let escaped = ArbitraryValueParser::escape_selector(&class_name);

            // Special character should be escaped with backslash
            let escaped_char = format!("\\{}", ch);
            prop_assert!(
                escaped.contains(&escaped_char),
                "Character '{}' should be escaped as '{}' in '{}'",
                ch, escaped_char, escaped
            );
        }

        /// Property test for invalid values (unbalanced parentheses)
        /// **Validates: Requirements 7.6**
        #[test]
        fn prop_invalid_unbalanced_parens(
            prefix in arb_prefix(),
            num in 1i32..10i32
        ) {
            let mut parser = ArbitraryValueParser::new();

            // Create unbalanced parentheses
            let open_parens: String = (0..num).map(|_| '(').collect();
            let class_name = format!("{}-[calc{}100%]", prefix, open_parens);

            let result = parser.parse(&class_name);

            // Should fail to parse or emit warning
            if result.is_some() {
                // If it parsed, there should be a warning
                prop_assert!(
                    !parser.warnings().is_empty(),
                    "Unbalanced parens '{}' should emit warning",
                    class_name
                );
            }
        }
    }
}
