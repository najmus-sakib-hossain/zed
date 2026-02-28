//! CSS Property Registry
//!
//! Runtime registry for CSS property lookup and CSS generation.
//! Loads from DX Serializer Machine format for fast access.
//!
//! **Validates: Requirements 1.1, 1.2, 1.4, 1.5, 1.6, 1.7, 1.8**

use super::database::{CssPropertyDef, LENGTH_UNITS};
use ahash::AHashMap;
use serializer::{DxLlmValue, MachineFormat, machine_to_document};
use std::path::Path;

/// Result of matching a class name to a CSS property
#[derive(Debug, Clone, PartialEq)]
pub enum CssMatch {
    /// Matched property with keyword value (e.g., "display-flex")
    PropertyValue { property: String, value: String },
    /// Matched property with numeric value (e.g., "width-100px")
    PropertyNumeric {
        property: String,
        value: String,
        unit: String,
    },
    /// CSS custom property (e.g., "--my-var-blue")
    CustomProperty { name: String, value: String },
}

/// Warning emitted for unknown properties
#[derive(Debug, Clone)]
pub struct PropertyWarning {
    pub class_name: String,
    pub message: String,
    pub suggestion: Option<String>,
}

/// CSS Property Registry - loaded from DX Serializer Machine format
///
/// Provides fast lookup of CSS properties and generation of CSS rules.
pub struct CssPropertyRegistry {
    /// Map of property name to property definition
    properties: AHashMap<String, CssPropertyDef>,
    /// Map of category to property names
    categories: AHashMap<String, Vec<String>>,
    /// Warnings collected during processing
    warnings: Vec<PropertyWarning>,
}

impl CssPropertyRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            properties: AHashMap::new(),
            categories: AHashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// Load registry from DX Serializer Machine format file
    ///
    /// **Validates: Requirements 1.7**
    pub fn load<P: AsRef<Path>>(machine_path: P) -> Result<Self, String> {
        let data = std::fs::read(machine_path.as_ref())
            .map_err(|e| format!("Failed to read machine file: {}", e))?;
        Self::from_machine_format(&data)
    }

    /// Load registry from machine format bytes
    pub fn from_machine_format(data: &[u8]) -> Result<Self, String> {
        let machine = MachineFormat::new(data.to_vec());
        let doc = machine_to_document(&machine)
            .map_err(|e| format!("Failed to parse machine format: {}", e))?;

        let mut registry = Self::new();

        // Extract property definitions
        for (key, value) in &doc.context {
            if let Some(rest) = key.strip_prefix("p:") {
                if let Some((name, field)) = rest.split_once('|') {
                    let prop = registry
                        .properties
                        .entry(name.to_string())
                        .or_insert_with(|| CssPropertyDef::new(name, "unknown"));

                    match field {
                        "cat" => {
                            if let DxLlmValue::Str(cat) = value {
                                prop.category = cat.clone();
                            }
                        }
                        "val" => {
                            if let DxLlmValue::Arr(arr) = value {
                                prop.values = arr
                                    .iter()
                                    .filter_map(|v| {
                                        if let DxLlmValue::Str(s) = v {
                                            Some(s.clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                            }
                        }
                        "num" => {
                            if let DxLlmValue::Bool(b) = value {
                                prop.accepts_numeric = *b;
                            }
                        }
                        "units" => {
                            if let DxLlmValue::Arr(arr) = value {
                                prop.valid_units = arr
                                    .iter()
                                    .filter_map(|v| {
                                        if let DxLlmValue::Str(s) = v {
                                            Some(s.clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect();
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(cat) = key.strip_prefix("cat:") {
                if let DxLlmValue::Arr(arr) = value {
                    let props: Vec<String> = arr
                        .iter()
                        .filter_map(|v| {
                            if let DxLlmValue::Str(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                    registry.categories.insert(cat.to_string(), props);
                }
            }
        }

        Ok(registry)
    }

    /// Load from built-in database (no file needed)
    pub fn from_builtin() -> Self {
        let mut registry = Self::new();
        for prop in super::database::get_all_css_properties() {
            registry
                .categories
                .entry(prop.category.clone())
                .or_default()
                .push(prop.name.clone());
            registry.properties.insert(prop.name.clone(), prop);
        }
        registry
    }

    /// Check if a class name matches a CSS property-value pattern
    ///
    /// **Validates: Requirements 1.2**
    pub fn matches_property(&mut self, class: &str) -> Option<CssMatch> {
        // Handle CSS custom properties (--my-var-value)
        if class.starts_with("--") {
            return self.parse_custom_property(class);
        }

        // Try "property-value" pattern
        if let Some(idx) = class.rfind('-') {
            let (prop_part, val_part) = class.split_at(idx);
            let val_part = &val_part[1..]; // Skip the '-'

            // Check if property exists
            if let Some(def) = self.properties.get(prop_part) {
                // Check keyword values
                if def.values.contains(&val_part.to_string()) {
                    return Some(CssMatch::PropertyValue {
                        property: prop_part.to_string(),
                        value: val_part.to_string(),
                    });
                }

                // Check numeric values with units
                if def.accepts_numeric {
                    if let Some((num, unit)) = self.parse_numeric_value(val_part, &def.valid_units)
                    {
                        return Some(CssMatch::PropertyNumeric {
                            property: prop_part.to_string(),
                            value: num,
                            unit,
                        });
                    }
                }
            }

            // Unknown property - emit warning but still try to generate
            self.emit_unknown_warning(class, prop_part);
            return Some(CssMatch::PropertyValue {
                property: prop_part.to_string(),
                value: val_part.to_string(),
            });
        }

        None
    }

    /// Parse a numeric value with optional unit
    ///
    /// **Validates: Requirements 1.4**
    fn parse_numeric_value(&self, value: &str, valid_units: &[String]) -> Option<(String, String)> {
        // Try to find where the number ends and unit begins
        let mut num_end = 0;
        let chars: Vec<char> = value.chars().collect();

        for (i, ch) in chars.iter().enumerate() {
            if ch.is_ascii_digit() || *ch == '.' || *ch == '-' {
                num_end = i + 1;
            } else {
                break;
            }
        }

        if num_end == 0 {
            return None;
        }

        let num_str = &value[..num_end];
        let unit_str = &value[num_end..];

        // Validate it's a valid number
        if num_str.parse::<f64>().is_err() {
            return None;
        }

        // If no unit, check if unitless is allowed
        if unit_str.is_empty() {
            if valid_units.is_empty() {
                return Some((num_str.to_string(), String::new()));
            }
            return None;
        }

        // Check if unit is valid
        if valid_units.is_empty() || valid_units.iter().any(|u| u == unit_str) {
            return Some((num_str.to_string(), unit_str.to_string()));
        }

        // Also accept standard length units
        if LENGTH_UNITS.contains(&unit_str) {
            return Some((num_str.to_string(), unit_str.to_string()));
        }

        None
    }

    /// Parse CSS custom property (--name-value)
    ///
    /// **Validates: Requirements 1.5**
    fn parse_custom_property(&self, class: &str) -> Option<CssMatch> {
        let rest = class.strip_prefix("--")?;
        if let Some(idx) = rest.rfind('-') {
            let name = &rest[..idx];
            let value = &rest[idx + 1..];
            if !name.is_empty() && !value.is_empty() {
                return Some(CssMatch::CustomProperty {
                    name: format!("--{}", name),
                    value: value.to_string(),
                });
            }
        }
        None
    }

    /// Emit warning for unknown property
    ///
    /// **Validates: Requirements 1.6**
    fn emit_unknown_warning(&mut self, class: &str, property: &str) {
        // Find similar properties for suggestion
        let suggestion = self.find_similar_property(property);

        self.warnings.push(PropertyWarning {
            class_name: class.to_string(),
            message: format!("Unknown CSS property: '{}'", property),
            suggestion,
        });
    }

    /// Find a similar property name for suggestions
    fn find_similar_property(&self, property: &str) -> Option<String> {
        let mut best_match: Option<(&str, usize)> = None;

        for name in self.properties.keys() {
            let distance = Self::levenshtein_distance(property, name);
            if distance <= 3 && (best_match.is_none() || distance < best_match.unwrap().1) {
                best_match = Some((name, distance));
            }
        }

        best_match.map(|(name, _)| format!("Did you mean '{}'?", name))
    }

    /// Simple Levenshtein distance for typo detection
    fn levenshtein_distance(a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let m = a_chars.len();
        let n = b_chars.len();

        if m == 0 {
            return n;
        }
        if n == 0 {
            return m;
        }

        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 0..=m {
            dp[i][0] = i;
        }
        for j in 0..=n {
            dp[0][j] = j;
        }

        for i in 1..=m {
            for j in 1..=n {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                dp[i][j] = (dp[i - 1][j] + 1).min(dp[i][j - 1] + 1).min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[m][n]
    }

    /// Generate CSS for a matched property
    ///
    /// **Validates: Requirements 1.2, 1.9**
    pub fn generate_css(&self, match_: &CssMatch) -> String {
        match match_ {
            CssMatch::PropertyValue { property, value } => {
                format!("{}: {}", property, value)
            }
            CssMatch::PropertyNumeric {
                property,
                value,
                unit,
            } => {
                format!("{}: {}{}", property, value, unit)
            }
            CssMatch::CustomProperty { name, value } => {
                format!("{}: {}", name, value)
            }
        }
    }

    /// Get collected warnings
    pub fn warnings(&self) -> &[PropertyWarning] {
        &self.warnings
    }

    /// Clear collected warnings
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }

    /// Get property definition by name
    pub fn get_property(&self, name: &str) -> Option<&CssPropertyDef> {
        self.properties.get(name)
    }

    /// Get all properties in a category
    pub fn get_category(&self, category: &str) -> Option<&Vec<String>> {
        self.categories.get(category)
    }

    /// Get total property count
    pub fn property_count(&self) -> usize {
        self.properties.len()
    }
}

impl Default for CssPropertyRegistry {
    fn default() -> Self {
        Self::from_builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_property_value() {
        let mut registry = CssPropertyRegistry::from_builtin();

        let result = registry.matches_property("display-flex");
        assert!(result.is_some());
        if let Some(CssMatch::PropertyValue { property, value }) = result {
            assert_eq!(property, "display");
            assert_eq!(value, "flex");
        } else {
            panic!("Expected PropertyValue match");
        }
    }

    #[test]
    fn test_matches_property_numeric() {
        let mut registry = CssPropertyRegistry::from_builtin();

        let result = registry.matches_property("width-100px");
        assert!(result.is_some());
        if let Some(CssMatch::PropertyNumeric {
            property,
            value,
            unit,
        }) = result
        {
            assert_eq!(property, "width");
            assert_eq!(value, "100");
            assert_eq!(unit, "px");
        } else {
            panic!("Expected PropertyNumeric match");
        }
    }

    #[test]
    fn test_matches_custom_property() {
        let mut registry = CssPropertyRegistry::from_builtin();

        let result = registry.matches_property("--my-var-blue");
        assert!(result.is_some());
        if let Some(CssMatch::CustomProperty { name, value }) = result {
            assert_eq!(name, "--my-var");
            assert_eq!(value, "blue");
        } else {
            panic!("Expected CustomProperty match");
        }
    }

    #[test]
    fn test_unknown_property_warning() {
        let mut registry = CssPropertyRegistry::from_builtin();

        // Unknown property should still match but emit warning
        let result = registry.matches_property("unknownprop-value");
        assert!(result.is_some());
        assert!(!registry.warnings().is_empty());
    }

    #[test]
    fn test_generate_css() {
        let registry = CssPropertyRegistry::from_builtin();

        let css = registry.generate_css(&CssMatch::PropertyValue {
            property: "display".to_string(),
            value: "flex".to_string(),
        });
        assert_eq!(css, "display: flex");

        let css = registry.generate_css(&CssMatch::PropertyNumeric {
            property: "width".to_string(),
            value: "100".to_string(),
            unit: "px".to_string(),
        });
        assert_eq!(css, "width: 100px");
    }

    #[test]
    fn test_from_builtin() {
        let registry = CssPropertyRegistry::from_builtin();
        assert!(registry.property_count() > 0);
        assert!(registry.get_property("display").is_some());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Arbitrary generators for CSS property testing
    fn arb_property_name() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "display",
            "position",
            "width",
            "height",
            "margin",
            "padding",
            "flex-direction",
            "justify-content",
            "align-items",
            "gap",
            "font-size",
            "line-height",
            "color",
            "background-color",
        ])
        .prop_map(|s| s.to_string())
    }

    fn arb_keyword_value() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "flex", "block", "grid", "none", "auto", "center", "start", "end", "row", "column",
            "nowrap", "wrap", "stretch", "baseline",
        ])
        .prop_map(|s| s.to_string())
    }

    fn arb_numeric_value() -> impl Strategy<Value = (String, String)> {
        (1i32..1000i32, prop::sample::select(vec!["px", "rem", "em", "%", "vw", "vh"]))
            .prop_map(|(n, u)| (n.to_string(), u.to_string()))
    }

    fn arb_custom_property() -> impl Strategy<Value = (String, String)> {
        ("[a-z][a-z0-9-]{1,10}".prop_map(|s| s), "[a-z0-9#]{1,10}".prop_map(|s| s))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-advanced-features, Property 1: CSS Property-Value Generation Correctness
        /// *For any* valid CSS property-value class name (e.g., "display-flex", "width-100px"),
        /// the generated CSS SHALL contain the correct property and value according to CSS spec.
        /// **Validates: Requirements 1.2, 1.3, 1.4, 1.5, 1.9**
        #[test]
        fn prop_css_property_value_generation(
            property in arb_property_name(),
            value in arb_keyword_value()
        ) {
            let mut registry = CssPropertyRegistry::from_builtin();
            let class_name = format!("{}-{}", property, value);

            if let Some(match_) = registry.matches_property(&class_name) {
                let css = registry.generate_css(&match_);

                // CSS should contain the property name
                prop_assert!(
                    css.contains(&property),
                    "Generated CSS '{}' should contain property '{}'",
                    css, property
                );

                // CSS should be in valid format "property: value"
                prop_assert!(
                    css.contains(": "),
                    "Generated CSS '{}' should be in 'property: value' format",
                    css
                );
            }
        }

        /// Property test for numeric value CSS generation
        /// **Validates: Requirements 1.4**
        #[test]
        fn prop_css_numeric_value_generation(
            (num, unit) in arb_numeric_value()
        ) {
            let mut registry = CssPropertyRegistry::from_builtin();
            let class_name = format!("width-{}{}", num, unit);

            if let Some(match_) = registry.matches_property(&class_name) {
                let css = registry.generate_css(&match_);

                // CSS should contain the numeric value
                prop_assert!(
                    css.contains(&num),
                    "Generated CSS '{}' should contain numeric value '{}'",
                    css, num
                );

                // CSS should contain the unit
                prop_assert!(
                    css.contains(&unit),
                    "Generated CSS '{}' should contain unit '{}'",
                    css, unit
                );
            }
        }

        /// Property test for CSS custom property generation
        /// **Validates: Requirements 1.5**
        #[test]
        fn prop_css_custom_property_generation(
            (name, value) in arb_custom_property()
        ) {
            let mut registry = CssPropertyRegistry::from_builtin();
            let class_name = format!("--{}-{}", name, value);

            if let Some(CssMatch::CustomProperty { name: prop_name, value: prop_value }) =
                registry.matches_property(&class_name)
            {
                let css = registry.generate_css(&CssMatch::CustomProperty {
                    name: prop_name.clone(),
                    value: prop_value.clone()
                });

                // CSS should start with --
                prop_assert!(
                    css.starts_with("--"),
                    "Custom property CSS '{}' should start with '--'",
                    css
                );

                // CSS should contain the value
                prop_assert!(
                    css.contains(&prop_value),
                    "Custom property CSS '{}' should contain value '{}'",
                    css, prop_value
                );
            }
        }

        /// Feature: dx-style-advanced-features, Property 2: Unknown Property Warning with Generation
        /// *For any* unknown CSS property-value combination, the Style_Engine SHALL emit a warning
        /// AND still generate the CSS output (fail-open behavior).
        /// **Validates: Requirements 1.6**
        #[test]
        fn prop_unknown_property_warning_with_generation(
            prop_name in "[a-z]{5,15}",
            value in "[a-z]{3,10}"
        ) {
            let mut registry = CssPropertyRegistry::from_builtin();

            // Create a class with an unknown property (prefix with 'x' to ensure it's unknown)
            let unknown_prop = format!("x{}", prop_name);
            let class_name = format!("{}-{}", unknown_prop, value);

            // Clear any previous warnings
            registry.clear_warnings();

            // Should still match (fail-open behavior)
            let result = registry.matches_property(&class_name);

            // Should produce a match (CSS is generated)
            prop_assert!(
                result.is_some(),
                "Unknown property '{}' should still produce a match (fail-open)",
                class_name
            );

            // Should emit a warning
            prop_assert!(
                !registry.warnings().is_empty(),
                "Unknown property '{}' should emit a warning",
                class_name
            );

            // Warning should mention the unknown property
            let warning = &registry.warnings()[0];
            prop_assert!(
                warning.message.contains(&unknown_prop),
                "Warning message '{}' should mention the unknown property '{}'",
                warning.message, unknown_prop
            );

            // Should still generate valid CSS
            if let Some(match_) = result {
                let css = registry.generate_css(&match_);
                prop_assert!(
                    css.contains(": "),
                    "Generated CSS '{}' should be in valid format even for unknown property",
                    css
                );
            }
        }
    }
}
