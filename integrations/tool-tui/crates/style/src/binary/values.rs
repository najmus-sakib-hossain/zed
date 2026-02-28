//! Level 5: Binary CSS Values (Nuclear Option)
//!
//! Instead of storing "display:flex" strings, store property + value as binary enums.
//! This is 6× smaller than string-based CSS.

use std::convert::TryFrom;

/// Error returned when converting an invalid byte to CssProperty
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPropertyError(pub u8);

impl std::fmt::Display for InvalidPropertyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid CSS property byte: 0x{:02X}", self.0)
    }
}

impl std::error::Error for InvalidPropertyError {}

/// CSS Property enum - u8 allows 256 properties
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssProperty {
    Display = 0x01,
    FlexDirection = 0x02,
    FlexWrap = 0x03,
    JustifyContent = 0x04,
    AlignItems = 0x05,
    AlignSelf = 0x06,
    Position = 0x07,
    Padding = 0x08,
    PaddingTop = 0x09,
    PaddingRight = 0x0A,
    PaddingBottom = 0x0B,
    PaddingLeft = 0x0C,
    Margin = 0x0D,
    MarginTop = 0x0E,
    MarginRight = 0x0F,
    MarginBottom = 0x10,
    MarginLeft = 0x11,
    Width = 0x12,
    Height = 0x13,
    Color = 0x14,
    Background = 0x15,
    Border = 0x16,
    BorderWidth = 0x17,
    BorderRadius = 0x18,
    FontSize = 0x19,
    FontWeight = 0x1A,
    LineHeight = 0x1B,
    TextAlign = 0x1C,
    BoxShadow = 0x1D,
    Overflow = 0x1E,
    OverflowX = 0x1F,
    OverflowY = 0x20,
    Top = 0x21,
    Right = 0x22,
    Bottom = 0x23,
    Left = 0x24,
    ZIndex = 0x25,
}

impl TryFrom<u8> for CssProperty {
    type Error = InvalidPropertyError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(CssProperty::Display),
            0x02 => Ok(CssProperty::FlexDirection),
            0x03 => Ok(CssProperty::FlexWrap),
            0x04 => Ok(CssProperty::JustifyContent),
            0x05 => Ok(CssProperty::AlignItems),
            0x06 => Ok(CssProperty::AlignSelf),
            0x07 => Ok(CssProperty::Position),
            0x08 => Ok(CssProperty::Padding),
            0x09 => Ok(CssProperty::PaddingTop),
            0x0A => Ok(CssProperty::PaddingRight),
            0x0B => Ok(CssProperty::PaddingBottom),
            0x0C => Ok(CssProperty::PaddingLeft),
            0x0D => Ok(CssProperty::Margin),
            0x0E => Ok(CssProperty::MarginTop),
            0x0F => Ok(CssProperty::MarginRight),
            0x10 => Ok(CssProperty::MarginBottom),
            0x11 => Ok(CssProperty::MarginLeft),
            0x12 => Ok(CssProperty::Width),
            0x13 => Ok(CssProperty::Height),
            0x14 => Ok(CssProperty::Color),
            0x15 => Ok(CssProperty::Background),
            0x16 => Ok(CssProperty::Border),
            0x17 => Ok(CssProperty::BorderWidth),
            0x18 => Ok(CssProperty::BorderRadius),
            0x19 => Ok(CssProperty::FontSize),
            0x1A => Ok(CssProperty::FontWeight),
            0x1B => Ok(CssProperty::LineHeight),
            0x1C => Ok(CssProperty::TextAlign),
            0x1D => Ok(CssProperty::BoxShadow),
            0x1E => Ok(CssProperty::Overflow),
            0x1F => Ok(CssProperty::OverflowX),
            0x20 => Ok(CssProperty::OverflowY),
            0x21 => Ok(CssProperty::Top),
            0x22 => Ok(CssProperty::Right),
            0x23 => Ok(CssProperty::Bottom),
            0x24 => Ok(CssProperty::Left),
            0x25 => Ok(CssProperty::ZIndex),
            _ => Err(InvalidPropertyError(value)),
        }
    }
}

impl From<CssProperty> for u8 {
    fn from(prop: CssProperty) -> Self {
        prop as u8
    }
}

/// Display values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayValue {
    None = 0x00,
    Block = 0x01,
    Inline = 0x02,
    InlineBlock = 0x03,
    Flex = 0x04,
    InlineFlex = 0x05,
    Grid = 0x06,
    InlineGrid = 0x07,
    Table = 0x08,
}

/// Flex Direction values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirectionValue {
    Row = 0x00,
    RowReverse = 0x01,
    Column = 0x02,
    ColumnReverse = 0x03,
}

/// Justify Content values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContentValue {
    FlexStart = 0x00,
    FlexEnd = 0x01,
    Center = 0x02,
    SpaceBetween = 0x03,
    SpaceAround = 0x04,
    SpaceEvenly = 0x05,
}

/// Align Items values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItemsValue {
    FlexStart = 0x00,
    FlexEnd = 0x01,
    Center = 0x02,
    Baseline = 0x03,
    Stretch = 0x04,
}

/// Position values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionValue {
    Static = 0x00,
    Relative = 0x01,
    Absolute = 0x02,
    Fixed = 0x03,
    Sticky = 0x04,
}

/// Color value (RGB stored as u32: 0xRRGGBB)
pub type ColorValue = u32;

/// Length value (in pixels, stored as u16)
pub type LengthValue = u16;

/// Property name lookup table
const PROP_NAMES: &[&str] = &[
    "",                // 0x00 (unused)
    "display",         // 0x01
    "flex-direction",  // 0x02
    "flex-wrap",       // 0x03
    "justify-content", // 0x04
    "align-items",     // 0x05
    "align-self",      // 0x06
    "position",        // 0x07
    "padding",         // 0x08
    "padding-top",     // 0x09
    "padding-right",   // 0x0A
    "padding-bottom",  // 0x0B
    "padding-left",    // 0x0C
    "margin",          // 0x0D
    "margin-top",      // 0x0E
    "margin-right",    // 0x0F
    "margin-bottom",   // 0x10
    "margin-left",     // 0x11
    "width",           // 0x12
    "height",          // 0x13
    "color",           // 0x14
    "background",      // 0x15
    "border",          // 0x16
    "border-width",    // 0x17
    "border-radius",   // 0x18
    "font-size",       // 0x19
    "font-weight",     // 0x1A
    "line-height",     // 0x1B
    "text-align",      // 0x1C
    "box-shadow",      // 0x1D
    "overflow",        // 0x1E
    "overflow-x",      // 0x1F
    "overflow-y",      // 0x20
    "top",             // 0x21
    "right",           // 0x22
    "bottom",          // 0x23
    "left",            // 0x24
    "z-index",         // 0x25
];

/// Display value names
const DISPLAY_VALUES: &[&str] = &[
    "none",
    "block",
    "inline",
    "inline-block",
    "flex",
    "inline-flex",
    "grid",
    "inline-grid",
    "table",
];

/// Flex direction value names
const FLEX_DIRECTION_VALUES: &[&str] = &["row", "row-reverse", "column", "column-reverse"];

/// Justify content value names
const JUSTIFY_CONTENT_VALUES: &[&str] = &[
    "flex-start",
    "flex-end",
    "center",
    "space-between",
    "space-around",
    "space-evenly",
];

/// Align items value names
const ALIGN_ITEMS_VALUES: &[&str] = &["flex-start", "flex-end", "center", "baseline", "stretch"];

/// Position value names
const POSITION_VALUES: &[&str] = &["static", "relative", "absolute", "fixed", "sticky"];

/// Get property name from property byte
#[allow(dead_code)]
fn get_property_name(prop: u8) -> &'static str {
    PROP_NAMES.get(prop as usize).unwrap_or(&"")
}

/// Get value string for a property
fn get_value_string(prop: CssProperty, val: u8) -> String {
    match prop {
        CssProperty::Display => DISPLAY_VALUES.get(val as usize).unwrap_or(&"block").to_string(),
        CssProperty::FlexDirection => {
            FLEX_DIRECTION_VALUES.get(val as usize).unwrap_or(&"row").to_string()
        }
        CssProperty::JustifyContent => {
            JUSTIFY_CONTENT_VALUES.get(val as usize).unwrap_or(&"flex-start").to_string()
        }
        CssProperty::AlignItems => {
            ALIGN_ITEMS_VALUES.get(val as usize).unwrap_or(&"stretch").to_string()
        }
        CssProperty::Position => POSITION_VALUES.get(val as usize).unwrap_or(&"static").to_string(),
        _ => format!("{}", val), // Numeric value
    }
}

/// Apply binary CSS from a byte stream
///
/// Stream format: [PROP, VAL, PROP, VAL, ...]
///
/// Returns CSS text string
pub fn apply_binary_css(stream: &[u8]) -> Result<String, &'static str> {
    if stream.len() % 2 != 0 {
        return Err("Invalid stream length (must be even)");
    }

    let mut css = String::with_capacity(stream.len() * 15); // Approx 15 chars per property

    let mut i = 0;
    while i < stream.len() {
        let prop_byte = stream[i];
        let val_byte = stream[i + 1];

        if let Some(&prop_name) = PROP_NAMES.get(prop_byte as usize) {
            if !prop_name.is_empty() {
                if !css.is_empty() {
                    css.push(';');
                }

                css.push_str(prop_name);
                css.push(':');

                // Convert property byte to enum using safe TryFrom
                // Skip invalid property bytes rather than causing undefined behavior
                if let Ok(prop) = CssProperty::try_from(prop_byte) {
                    let val_str = get_value_string(prop, val_byte);
                    css.push_str(&val_str);
                } else {
                    // Invalid property byte - use raw value as fallback
                    css.push_str(&val_byte.to_string());
                }
            }
        }

        i += 2;
    }

    Ok(css)
}

/// Encode a single CSS property-value pair to binary
pub fn encode_property(prop: CssProperty, value: u8) -> [u8; 2] {
    [prop as u8, value]
}

/// Encode multiple properties into a binary stream
pub fn encode_properties(props: &[(CssProperty, u8)]) -> Vec<u8> {
    let mut stream = Vec::with_capacity(props.len() * 2);

    for &(prop, val) in props {
        stream.push(prop as u8);
        stream.push(val);
    }

    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_single_property() {
        let encoded = encode_property(CssProperty::Display, DisplayValue::Flex as u8);
        assert_eq!(encoded, [0x01, 0x04]);
    }

    #[test]
    fn test_encode_multiple_properties() {
        let props = vec![
            (CssProperty::Display, DisplayValue::Flex as u8),
            (CssProperty::AlignItems, AlignItemsValue::Center as u8),
        ];

        let stream = encode_properties(&props);
        assert_eq!(stream, vec![0x01, 0x04, 0x05, 0x02]);
    }

    #[test]
    fn test_apply_binary_css() {
        let stream = vec![
            0x01, 0x04, // display: flex
            0x05, 0x02, // align-items: center
        ];

        let css = apply_binary_css(&stream).unwrap();
        assert_eq!(css, "display:flex;align-items:center");
    }

    #[test]
    fn test_invalid_stream_length() {
        let stream = vec![0x01]; // Odd length
        let result = apply_binary_css(&stream);
        assert!(result.is_err());
    }

    #[test]
    fn test_size_comparison() {
        // String version: "display:flex" = 12 bytes
        let string_version = "display:flex";

        // Binary version: [0x01, 0x04] = 2 bytes
        let binary_version = vec![0x01, 0x04];

        assert_eq!(string_version.len(), 12);
        assert_eq!(binary_version.len(), 2);

        // 6× smaller!
        let ratio = string_version.len() as f64 / binary_version.len() as f64;
        assert!(ratio >= 6.0);
    }

    #[test]
    fn test_complex_example() {
        // flex + items-center + p-4 (padding: 1rem = 16px)
        let props = vec![
            (CssProperty::Display, DisplayValue::Flex as u8),
            (CssProperty::AlignItems, AlignItemsValue::Center as u8),
            (CssProperty::Padding, 16), // 16px = 1rem
        ];

        let stream = encode_properties(&props);
        let css = apply_binary_css(&stream).unwrap();

        // Should contain all three properties
        assert!(css.contains("display:flex"));
        assert!(css.contains("align-items:center"));
        assert!(css.contains("padding:16"));

        // Size comparison
        let string_equivalent = "display:flex;align-items:center;padding:1rem";
        let size_ratio = string_equivalent.len() as f64 / stream.len() as f64;

        println!(
            "String: {} bytes, Binary: {} bytes, Ratio: {:.1}×",
            string_equivalent.len(),
            stream.len(),
            size_ratio
        );

        assert!(size_ratio > 5.0); // At least 5× smaller
    }

    #[test]
    fn test_property_name_lookup() {
        assert_eq!(get_property_name(0x01), "display");
        assert_eq!(get_property_name(0x05), "align-items");
        assert_eq!(get_property_name(0x14), "color");
    }

    #[test]
    fn test_roundtrip() {
        let original_props = vec![
            (CssProperty::Display, DisplayValue::Flex as u8),
            (CssProperty::JustifyContent, JustifyContentValue::Center as u8),
            (CssProperty::AlignItems, AlignItemsValue::Center as u8),
        ];

        // Encode
        let stream = encode_properties(&original_props);

        // Decode
        let css = apply_binary_css(&stream).unwrap();

        // Verify
        assert!(css.contains("display:flex"));
        assert!(css.contains("justify-content:center"));
        assert!(css.contains("align-items:center"));
    }

    #[test]
    fn test_performance() {
        use std::time::Instant;

        let props = vec![
            (CssProperty::Display, DisplayValue::Flex as u8),
            (CssProperty::FlexDirection, FlexDirectionValue::Column as u8),
            (CssProperty::AlignItems, AlignItemsValue::Center as u8),
            (CssProperty::JustifyContent, JustifyContentValue::Center as u8),
        ];

        let stream = encode_properties(&props);

        // Benchmark decoding
        let start = Instant::now();
        for _ in 0..100000 {
            let _ = apply_binary_css(&stream).unwrap();
        }
        let elapsed = start.elapsed();

        println!("100k iterations: {:?}", elapsed);
        assert!(elapsed.as_millis() < 500); // Should be very fast
    }

    /// **Property 1: CssProperty Round-Trip Conversion**
    /// **Validates: Requirements 1.4**
    /// *For any* valid CssProperty variant, converting it to u8 and back to CssProperty
    /// SHALL produce the original variant.
    #[test]
    fn test_css_property_roundtrip_all_variants() {
        // Test all valid CssProperty variants for round-trip conversion
        let all_variants = [
            CssProperty::Display,
            CssProperty::FlexDirection,
            CssProperty::FlexWrap,
            CssProperty::JustifyContent,
            CssProperty::AlignItems,
            CssProperty::AlignSelf,
            CssProperty::Position,
            CssProperty::Padding,
            CssProperty::PaddingTop,
            CssProperty::PaddingRight,
            CssProperty::PaddingBottom,
            CssProperty::PaddingLeft,
            CssProperty::Margin,
            CssProperty::MarginTop,
            CssProperty::MarginRight,
            CssProperty::MarginBottom,
            CssProperty::MarginLeft,
            CssProperty::Width,
            CssProperty::Height,
            CssProperty::Color,
            CssProperty::Background,
            CssProperty::Border,
            CssProperty::BorderWidth,
            CssProperty::BorderRadius,
            CssProperty::FontSize,
            CssProperty::FontWeight,
            CssProperty::LineHeight,
            CssProperty::TextAlign,
            CssProperty::BoxShadow,
            CssProperty::Overflow,
            CssProperty::OverflowX,
            CssProperty::OverflowY,
            CssProperty::Top,
            CssProperty::Right,
            CssProperty::Bottom,
            CssProperty::Left,
            CssProperty::ZIndex,
        ];

        for prop in all_variants {
            let byte: u8 = prop.into();
            let roundtrip = CssProperty::try_from(byte);
            assert_eq!(
                roundtrip,
                Ok(prop),
                "Round-trip failed for {:?} (byte 0x{:02X})",
                prop,
                byte
            );
        }
    }

    /// **Property 2: Invalid Property Bytes Return Errors**
    /// **Validates: Requirements 1.2**
    /// *For any* byte value that does not correspond to a valid CssProperty variant
    /// (outside 0x01-0x25), TryFrom<u8> SHALL return an InvalidPropertyError.
    #[test]
    fn test_invalid_property_bytes_return_errors() {
        // Test byte 0x00 (before valid range)
        assert!(CssProperty::try_from(0x00).is_err(), "Byte 0x00 should return error");

        // Test bytes after valid range (0x26-0xFF)
        for byte in 0x26u8..=0xFF {
            let result = CssProperty::try_from(byte);
            assert!(result.is_err(), "Byte 0x{:02X} should return error, got {:?}", byte, result);

            // Verify the error contains the invalid byte
            if let Err(InvalidPropertyError(err_byte)) = result {
                assert_eq!(err_byte, byte, "Error should contain the invalid byte");
            }
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // **Property 1: CssProperty Round-Trip Conversion (Property-Based)**
    // **Validates: Requirements 1.4**
    // Feature: dx-style-production-hardening, Property 1: CssProperty Round-Trip Conversion
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_css_property_roundtrip(byte in 0x01u8..=0x25u8) {
            // All bytes in valid range should round-trip successfully
            let prop = CssProperty::try_from(byte).expect("Valid byte should convert");
            let back: u8 = prop.into();
            prop_assert_eq!(byte, back, "Round-trip should preserve byte value");

            // Converting back to CssProperty should give same variant
            let roundtrip = CssProperty::try_from(back).expect("Should convert back");
            prop_assert_eq!(prop, roundtrip, "Round-trip should preserve variant");
        }

        // **Property 2: Invalid Property Bytes Return Errors (Property-Based)**
        // **Validates: Requirements 1.2**
        // Feature: dx-style-production-hardening, Property 2: Invalid Property Bytes Return Errors
        #[test]
        fn prop_invalid_bytes_return_errors(byte in prop_oneof![
            Just(0x00u8),
            0x26u8..=0xFFu8
        ]) {
            let result = CssProperty::try_from(byte);
            prop_assert!(result.is_err(), "Invalid byte 0x{:02X} should return error", byte);

            if let Err(InvalidPropertyError(err_byte)) = result {
                prop_assert_eq!(err_byte, byte, "Error should contain the invalid byte");
            }
        }
    }
}
