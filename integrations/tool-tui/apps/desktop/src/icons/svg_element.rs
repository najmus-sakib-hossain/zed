#![allow(dead_code)]

use gpui::{div, prelude::*, IntoElement, Pixels};

/// A simple SVG icon renderer that displays SVG content
/// Since GPUI doesn't support dynamic SVG loading, we render a placeholder
/// with the icon's color information extracted from the SVG
pub struct SvgIcon {
    svg_data: String,
    size: Pixels,
}

impl SvgIcon {
    pub fn new(svg_data: String, size: Pixels) -> Self {
        Self { svg_data, size }
    }

    /// Extract a representative color from the SVG data
    fn extract_color(&self) -> gpui::Hsla {
        // Try to find fill or stroke colors in the SVG
        if let Some(fill_start) = self.svg_data.find("fill=\"#") {
            if let Some(color_hex) = self.svg_data.get(fill_start + 7..fill_start + 13) {
                if let Ok(r) = u8::from_str_radix(&color_hex[0..2], 16) {
                    if let Ok(g) = u8::from_str_radix(&color_hex[2..4], 16) {
                        if let Ok(b) = u8::from_str_radix(&color_hex[4..6], 16) {
                            return gpui::rgb(((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
                                .into();
                        }
                    }
                }
            }
        }

        // Default to a nice blue color
        gpui::rgb(0x3498db).into()
    }

    pub fn render(self) -> impl IntoElement {
        let color = self.extract_color();

        // For now, render a colored circle that represents the icon
        // This is a limitation of GPUI - it doesn't support dynamic SVG rendering
        div()
            .flex()
            .items_center()
            .justify_center()
            .size(self.size)
            .rounded(self.size / 2.0) // Make it circular
            .bg(color)
            .into_any_element()
    }
}
