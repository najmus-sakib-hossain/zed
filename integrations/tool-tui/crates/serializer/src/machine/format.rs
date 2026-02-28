//! Format detection and dual-mode support
//!
//! Automatically detects DX-Machine vs DX Serializer format based on magic bytes.

use crate::types::DxValue;

/// Binary format type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxFormat {
    /// DX-Machine: Ultra-fast binary (0x5A 0x44)
    Zero,
    /// DX Serializer: Human-optimized text (0x44 0x58)
    Text,
    /// Unknown format
    Unknown,
}

/// Detect format from magic bytes
#[inline]
pub fn detect_format(bytes: &[u8]) -> DxFormat {
    if bytes.len() < 2 {
        return DxFormat::Unknown;
    }

    match &bytes[0..2] {
        [0x5A, 0x44] => DxFormat::Zero, // "ZD" little-endian
        [0x44, 0x58] => DxFormat::Text, // "DX" (hypothetical)
        _ => DxFormat::Unknown,
    }
}

/// Parse DX format (auto-detect)
///
/// This function automatically detects whether the input is DX-Machine binary
/// or DX Serializer text format and parses accordingly.
pub fn parse_auto(bytes: &[u8]) -> Result<DxValue, String> {
    match detect_format(bytes) {
        DxFormat::Zero => {
            // Parse as DX-Machine binary
            Err(
                "DX-Machine to DxValue conversion not yet implemented (use direct struct access)"
                    .to_string(),
            )
        }
        DxFormat::Text | DxFormat::Unknown => {
            // Parse as DX Serializer text (fallback)
            crate::parse(bytes).map_err(|e| format!("Parse error: {:?}", e))
        }
    }
}

/// Configuration for format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatMode {
    /// Always use DX-Machine binary
    Zero,
    /// Always use Dx Serializer text
    Text,
    /// Auto-detect based on input
    #[default]
    Auto,
}

impl FormatMode {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "zero" | "binary" => Some(Self::Zero),
            "text" | "dsr" => Some(Self::Text),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }

    /// Get format name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::Text => "text",
            Self::Auto => "auto",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_zero_format() {
        let bytes = [0x5A, 0x44, 0x01, 0x04]; // DX-Machine header
        assert_eq!(detect_format(&bytes), DxFormat::Zero);
    }

    #[test]
    fn test_detect_text_format() {
        let bytes = [0x44, 0x58, b'_', b'I']; // Dx Serializer text format
        assert_eq!(detect_format(&bytes), DxFormat::Text);
    }

    #[test]
    fn test_detect_unknown() {
        let bytes = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(detect_format(&bytes), DxFormat::Unknown);
    }

    #[test]
    fn test_detect_too_small() {
        let bytes = [0x5A];
        assert_eq!(detect_format(&bytes), DxFormat::Unknown);
    }

    #[test]
    fn test_format_mode_from_str() {
        assert_eq!(FormatMode::from_str("zero"), Some(FormatMode::Zero));
        assert_eq!(FormatMode::from_str("text"), Some(FormatMode::Text));
        assert_eq!(FormatMode::from_str("dsr"), Some(FormatMode::Text));
        assert_eq!(FormatMode::from_str("auto"), Some(FormatMode::Auto));
        assert_eq!(FormatMode::from_str("binary"), Some(FormatMode::Zero));
        assert_eq!(FormatMode::from_str("invalid"), None);
    }

    #[test]
    fn test_format_mode_name() {
        assert_eq!(FormatMode::Zero.name(), "zero");
        assert_eq!(FormatMode::Text.name(), "text");
        assert_eq!(FormatMode::Auto.name(), "auto");
    }
}
