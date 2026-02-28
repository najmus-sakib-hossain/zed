//! Color module for dx-style
//!
//! Provides comprehensive color manipulation, theming, and CSS generation.
//! Includes Material Design 3 color science, OKLCH color space support,
//! and automatic theme generation from source colors.
//!
//! Key components:
//! - `Argb`: ARGB color representation
//! - `Oklch`: OKLCH color space for perceptual uniformity
//! - `ThemeBuilder`: Generate complete themes from source colors
//! - Color CSS generation for utility classes

#![allow(dead_code)] // Color module API surface is broader than current in-crate usage.

use crate::core::color::color::{Argb, Oklch};
use crate::core::engine::StyleEngine;

#[cfg(all(feature = "image", not(feature = "std")))]
compile_error!("\"image\" feature requires \"std\" feature");

#[cfg(all(feature = "std", feature = "libm"))]
compile_error!("features \"std\" and \"libm\" cannot be enabled simultaneously");

#[cfg(all(not(feature = "std"), not(feature = "libm")))]
compile_error!("\"no-std\" requires \"libm\" feature");

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
pub(crate) use ahash::HashMap as Map;
#[cfg(not(feature = "std"))]
pub(crate) use alloc::collections::BTreeMap as Map;

#[allow(dead_code)]
pub(crate) type IndexMap<K, V> =
    indexmap::IndexMap<K, V, core::hash::BuildHasherDefault<ahash::AHasher>>;

pub mod blend;
pub mod color;
pub mod contrast;
pub mod dislike;
pub mod dynamic_color;
pub mod error;
pub mod hct;
#[cfg(feature = "image")]
pub mod image;
pub mod palette;
pub mod quantize;
pub mod scheme;
pub mod score;
pub mod temperature;
pub mod theme;
pub mod utils;

pub use error::Error;

pub fn generate_color_css(engine: &StyleEngine, class_name: &str) -> Option<String> {
    if let Some(name) = class_name.strip_prefix("bg-") {
        if derive_color_value(engine, name).is_some() {
            return Some(format!("background-color: var(--color-{})", name));
        }
    }
    if let Some(name) = class_name.strip_prefix("text-") {
        if derive_color_value(engine, name).is_some() {
            return Some(format!("color: var(--color-{})", name));
        }
    }
    None
}

// --- Dynamic color token support -----------------------------------------------------------
// Any class "bg-<token>" or "text-<token>" will now produce a CSS variable --color-<token>.
// 1. If <token> exists in colors.toml it uses that value.
// 2. Else if <token> matches a CSS color keyword we emit that keyword.
// 3. Else if <token> is a 3/4/6/8 length hex sequence we emit #<token>.
// Otherwise we skip generation (invalid color).

// Sorted for binary_search
const CSS_COLOR_KEYWORDS: &[&str] = &[
    "aliceblue",
    "antiquewhite",
    "aqua",
    "aquamarine",
    "azure",
    "beige",
    "bisque",
    "black",
    "blanchedalmond",
    "blue",
    "blueviolet",
    "brown",
    "burlywood",
    "cadetblue",
    "chartreuse",
    "chocolate",
    "coral",
    "cornflowerblue",
    "cornsilk",
    "crimson",
    "cyan",
    "darkblue",
    "darkcyan",
    "darkgoldenrod",
    "darkgray",
    "darkgreen",
    "darkgrey",
    "darkkhaki",
    "darkmagenta",
    "darkolivegreen",
    "darkorange",
    "darkorchid",
    "darkred",
    "darksalmon",
    "darkseagreen",
    "darkslateblue",
    "darkslategray",
    "darkslategrey",
    "darkturquoise",
    "darkviolet",
    "deeppink",
    "deepskyblue",
    "dimgray",
    "dimgrey",
    "dodgerblue",
    "firebrick",
    "floralwhite",
    "forestgreen",
    "fuchsia",
    "gainsboro",
    "ghostwhite",
    "gold",
    "goldenrod",
    "gray",
    "green",
    "greenyellow",
    "grey",
    "honeydew",
    "hotpink",
    "indianred",
    "indigo",
    "ivory",
    "khaki",
    "lavender",
    "lavenderblush",
    "lawngreen",
    "lemonchiffon",
    "lightblue",
    "lightcoral",
    "lightcyan",
    "lightgoldenrodyellow",
    "lightgray",
    "lightgreen",
    "lightgrey",
    "lightpink",
    "lightsalmon",
    "lightseagreen",
    "lightskyblue",
    "lightslategray",
    "lightslategrey",
    "lightsteelblue",
    "lightyellow",
    "lime",
    "limegreen",
    "linen",
    "magenta",
    "maroon",
    "mediumaquamarine",
    "mediumblue",
    "mediumorchid",
    "mediumpurple",
    "mediumseagreen",
    "mediumslateblue",
    "mediumspringgreen",
    "mediumturquoise",
    "mediumvioletred",
    "midnightblue",
    "mintcream",
    "mistyrose",
    "moccasin",
    "navajowhite",
    "navy",
    "oldlace",
    "olive",
    "olivedrab",
    "orange",
    "orangered",
    "orchid",
    "palegoldenrod",
    "palegreen",
    "paleturquoise",
    "palevioletred",
    "papayawhip",
    "peachpuff",
    "peru",
    "pink",
    "plum",
    "powderblue",
    "purple",
    "rebeccapurple",
    "red",
    "rosybrown",
    "royalblue",
    "saddlebrown",
    "salmon",
    "sandybrown",
    "seagreen",
    "seashell",
    "sienna",
    "silver",
    "skyblue",
    "slateblue",
    "slategray",
    "slategrey",
    "snow",
    "springgreen",
    "steelblue",
    "tan",
    "teal",
    "thistle",
    "tomato",
    "turquoise",
    "violet",
    "wheat",
    "white",
    "whitesmoke",
    "yellow",
    "yellowgreen",
    "transparent",
    "currentcolor",
];

pub fn derive_color_value(engine: &StyleEngine, name: &str) -> Option<String> {
    if let Some(v) = engine.colors.get(name) {
        return Some(v.clone());
    }
    let lower = name.to_ascii_lowercase();
    if CSS_COLOR_KEYWORDS.binary_search(&lower.as_str()).is_ok() {
        return Some(lower);
    }
    let len = lower.len();
    if matches!(len, 3 | 4 | 6 | 8) && lower.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("#{}", lower));
    }
    None
}

fn parse_oklch_value(value: &str) -> Option<Oklch> {
    let trimmed = value.trim();
    let inner = trimmed.strip_prefix("oklch(")?.strip_suffix(')')?.replace('/', " ");
    let mut parts = inner.split_whitespace().filter(|segment| !segment.is_empty());
    let l_raw = parts.next()?;
    let c_raw = parts.next()?;
    let h_raw = parts.next()?;

    fn parse_component(component: &str) -> Option<f64> {
        let cleaned = component.trim_end_matches(|ch: char| {
            ch == '%' || ch == '°' || ch == 'd' || ch == 'e' || ch == 'g'
        });
        if cleaned.is_empty() {
            return None;
        }
        cleaned.parse::<f64>().ok()
    }

    let mut l_value = parse_component(l_raw)?;
    if l_value > 1.0 {
        l_value /= 100.0;
    }
    let c_value = parse_component(c_raw)?;
    let h_value = parse_component(h_raw)?;

    Some(Oklch {
        l: l_value,
        c: c_value,
        h: h_value,
    })
}

fn split_components(input: &str) -> Vec<&str> {
    if input.contains(',') {
        input
            .split(',')
            .map(|segment| segment.trim())
            .filter(|segment| !segment.is_empty())
            .collect()
    } else {
        input.split_whitespace().filter(|segment| !segment.is_empty()).collect()
    }
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

fn parse_alpha_component(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.ends_with('%') {
        let number = trimmed.trim_end_matches('%').trim().parse::<f64>().ok()?;
        Some(clamp01(number / 100.0))
    } else {
        let number = trimmed.parse::<f64>().ok()?;
        Some(clamp01(number))
    }
}

fn parse_rgb_component(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.ends_with('%') {
        let number = trimmed.trim_end_matches('%').trim().parse::<f64>().ok()?;
        Some((clamp01(number / 100.0)) * 255.0)
    } else {
        let number = trimmed.parse::<f64>().ok()?;
        Some(number.clamp(0.0, 255.0))
    }
}

fn parse_hue(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let (number_str, unit) = if let Some(stripped) = trimmed.strip_suffix("deg") {
        (stripped, "deg")
    } else if let Some(stripped) = trimmed.strip_suffix("rad") {
        (stripped, "rad")
    } else if let Some(stripped) = trimmed.strip_suffix("turn") {
        (stripped, "turn")
    } else {
        (trimmed, "deg")
    };

    let raw = number_str.trim().parse::<f64>().ok()?;
    let degrees = match unit {
        "deg" => raw,
        "rad" => raw.to_degrees(),
        "turn" => raw * 360.0,
        _ => raw,
    };

    let mut normalized = degrees % 360.0;
    if normalized < 0.0 {
        normalized += 360.0;
    }
    Some(normalized)
}

fn parse_percentage_component(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.ends_with('%') {
        let number = trimmed.trim_end_matches('%').trim().parse::<f64>().ok()?;
        Some(clamp01(number / 100.0))
    } else {
        let number = trimmed.parse::<f64>().ok()?;
        if number > 1.0 {
            Some(clamp01(number / 100.0))
        } else {
            Some(clamp01(number))
        }
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 1.0 / 2.0 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        (l, l, l)
    } else {
        let h_fraction = (h % 360.0) / 360.0;
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        (
            hue_to_rgb(p, q, h_fraction + 1.0 / 3.0),
            hue_to_rgb(p, q, h_fraction),
            hue_to_rgb(p, q, h_fraction - 1.0 / 3.0),
        )
    }
}

fn parse_hsl_function(value: &str) -> Option<Argb> {
    let start = value.find('(')?;
    let end = value.rfind(')')?;
    if end <= start + 1 {
        return None;
    }
    let args = &value[start + 1..end];
    let (components_part, alpha_part) = if let Some((left, right)) = args.split_once('/') {
        (left.trim(), Some(right.trim()))
    } else {
        (args.trim(), None)
    };

    let mut components = split_components(components_part);
    let inline_alpha = if components.len() == 4 {
        components.pop()
    } else {
        None
    };
    if components.len() != 3 {
        return None;
    }

    let h = parse_hue(components[0])?;
    let s = parse_percentage_component(components[1])?;
    let l = parse_percentage_component(components[2])?;
    let alpha = if let Some(alpha_raw) = alpha_part {
        parse_alpha_component(alpha_raw)?
    } else if let Some(inline) = inline_alpha {
        parse_alpha_component(inline)?
    } else {
        1.0
    };

    let (r, g, b) = hsl_to_rgb(h, s, l);
    let red = (r * 255.0).round().clamp(0.0, 255.0) as u8;
    let green = (g * 255.0).round().clamp(0.0, 255.0) as u8;
    let blue = (b * 255.0).round().clamp(0.0, 255.0) as u8;
    let alpha_byte = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Argb::new(alpha_byte, red, green, blue))
}

fn parse_rgb_function(value: &str) -> Option<Argb> {
    let start = value.find('(')?;
    let end = value.rfind(')')?;
    if end <= start + 1 {
        return None;
    }
    let args = &value[start + 1..end];
    let (components_part, alpha_from_slash) = if let Some((left, right)) = args.split_once('/') {
        (left.trim(), Some(right.trim()))
    } else {
        (args.trim(), None)
    };

    let mut components = split_components(components_part);
    if components.len() < 3 {
        return None;
    }

    let mut alpha = if let Some(alpha_raw) = alpha_from_slash {
        parse_alpha_component(alpha_raw)?
    } else if components.len() == 4 {
        parse_alpha_component(components.pop().unwrap())?
    } else {
        1.0
    };

    let red = parse_rgb_component(components.first()?.trim())?;
    let green = parse_rgb_component(components.get(1)?.trim())?;
    let blue = parse_rgb_component(components.get(2)?.trim())?;

    alpha = clamp01(alpha);

    Some(Argb::new(
        (alpha * 255.0).round().clamp(0.0, 255.0) as u8,
        red.round().clamp(0.0, 255.0) as u8,
        green.round().clamp(0.0, 255.0) as u8,
        blue.round().clamp(0.0, 255.0) as u8,
    ))
}

pub(crate) fn parse_color_to_argb(value: &str) -> Option<Argb> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower == "transparent" || lower == "currentcolor" || lower == "inherit" {
        return None;
    }
    if trimmed.starts_with("oklch(") {
        return parse_oklch_value(trimmed).map(Argb::from);
    }
    if lower.starts_with("hsl(") || lower.starts_with("hsla(") {
        if let Some(color) = parse_hsl_function(trimmed) {
            return Some(color);
        }
    }
    if lower.starts_with("rgb(") || lower.starts_with("rgba(") {
        if let Some(color) = parse_rgb_function(trimmed) {
            return Some(color);
        }
    }
    trimmed.parse::<Argb>().ok()
}

pub(crate) fn format_argb_as_oklch(color: Argb) -> String {
    let oklch = Oklch::from(color);
    format!("oklch({:.2} {:.3} {:.2})", oklch.l, oklch.c, oklch.h)
}

pub(crate) fn normalize_color_to_oklch(value: &str) -> Option<String> {
    parse_color_to_argb(value).map(format_argb_as_oklch)
}
