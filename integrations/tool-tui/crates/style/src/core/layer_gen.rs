//! Optimized CSS layer generation with caching and parallelization
//!
//! This module provides high-performance layer generation by:
//! - Caching static layers (theme, base, properties)
//! - Parallel generation using rayon
//! - Incremental updates (only regenerate changed layers)

use super::{AppState, LayerCache};
use ahash::{AHashMap, AHasher};
use std::hash::Hasher;

/// Check if a class affects the theme layer (colors)
#[inline]
pub fn is_color_class(class: &str) -> bool {
    let base = class.rsplit(':').next().unwrap_or(class);
    base.starts_with("bg-")
        || base.starts_with("text-")
        || base.starts_with("border-")
        || base.starts_with("ring-")
        || base.starts_with("from-")
        || base.starts_with("to-")
        || base.starts_with("via-")
}

/// Calculate hash of color classes for cache invalidation
pub fn calculate_theme_hash(classes: &[String]) -> u64 {
    let mut hasher = AHasher::default();
    let mut color_classes: Vec<&str> =
        classes.iter().filter(|c| is_color_class(c)).map(|s| s.as_str()).collect();
    color_classes.sort_unstable();
    for class in color_classes {
        hasher.write(class.as_bytes());
    }
    hasher.finish()
}

/// Write a CSS layer with proper formatting
fn write_layer(buf: &mut Vec<u8>, name: &str, body: &str) {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        buf.extend_from_slice(format!("@layer {} {{}}\n", name).as_bytes());
    } else {
        buf.extend_from_slice(format!("@layer {} {{\n", name).as_bytes());
        for line in trimmed.lines() {
            if line.is_empty() {
                continue;
            }
            buf.extend_from_slice(b"  ");
            buf.extend_from_slice(line.as_bytes());
            buf.push(b'\n');
        }
        buf.extend_from_slice(b"}\n");
    }
}

/// Generate theme layer (colors)
fn generate_theme_layer(classes: &[String]) -> Vec<u8> {
    let (root_vars, dark_vars) = {
        let engine = AppState::engine();
        engine.generate_color_vars_for(classes.iter().collect::<Vec<_>>().iter().copied())
    };

    let mut theme_body = String::new();
    if !root_vars.is_empty() {
        theme_body.push_str(root_vars.trim_end());
        theme_body.push('\n');
    }
    if !dark_vars.is_empty() {
        theme_body.push_str(dark_vars.trim_end());
        theme_body.push('\n');
    }

    let mut buf = Vec::new();
    write_layer(&mut buf, "theme", &theme_body);
    buf
}

/// Generate base layer (CSS reset)
fn generate_base_layer() -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(base_raw) = AppState::engine().base_layer_raw.as_ref() {
        if !base_raw.is_empty() {
            let mut base_body = String::new();
            for line in base_raw.trim_end().lines() {
                if line.trim().is_empty() {
                    continue;
                }
                base_body.push_str(line);
                base_body.push('\n');
            }
            write_layer(&mut buf, "base", &base_body);
        } else {
            write_layer(&mut buf, "base", "");
        }
    } else {
        write_layer(&mut buf, "base", "");
    }
    buf
}

/// Generate properties layer (@property rules)
fn generate_properties_layer() -> Vec<u8> {
    let mut buf = Vec::new();
    let engine = AppState::engine();
    let mut prop_body = if let Some(prop_raw) = engine.property_layer_raw.as_ref() {
        if prop_raw.trim().is_empty() {
            String::new()
        } else {
            let mut b = String::new();
            for line in prop_raw.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                b.push_str(line);
                b.push('\n');
            }
            b
        }
    } else {
        String::new()
    };

    if prop_body.is_empty() {
        let at_rules = engine.property_at_rules();
        if !at_rules.trim().is_empty() {
            prop_body.push_str(at_rules.trim_end());
            prop_body.push('\n');
        }
    }

    write_layer(&mut buf, "properties", &prop_body);
    buf
}

/// Generate all layers with caching and parallelization
pub fn generate_layers_cached(
    classes: &[String],
    cache: &mut LayerCache,
    force_regenerate: bool,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let theme_hash = calculate_theme_hash(classes);
    let needs_theme_update = force_regenerate || !cache.valid || cache.theme_hash != theme_hash;

    if needs_theme_update {
        // Generate theme and base in parallel (properties is fast, do inline)
        let (theme, base) = rayon::join(|| generate_theme_layer(classes), generate_base_layer);
        let props = generate_properties_layer();

        // Update cache
        cache.theme_bytes = theme.clone();
        cache.base_bytes = base.clone();
        cache.properties_bytes = props.clone();
        cache.theme_hash = theme_hash;
        cache.valid = true;

        (theme, base, props)
    } else {
        // Reuse cached layers
        (
            cache.theme_bytes.clone(),
            cache.base_bytes.clone(),
            cache.properties_bytes.clone(),
        )
    }
}

/// Generate utilities layer with group registry
pub fn generate_utilities_layer(
    classes: &[String],
    group_registry: &mut crate::core::group::GroupRegistry,
    html_bytes: &[u8],
) -> Vec<u8> {
    use super::find_grouped_calls_in_text;
    use crate::generator;

    // Update group registry dev selectors
    let mut devs: AHashMap<String, String> = AHashMap::default();
    let html_string = String::from_utf8_lossy(html_bytes).to_string();
    for (name, _def) in group_registry.definitions() {
        let grouped_calls = find_grouped_calls_in_text(&html_string);
        if let Some((_, inner)) = grouped_calls.into_iter().find(|(n, _)| n == name) {
            let raw = format!("@{}({})", name, inner);
            devs.insert(name.clone(), raw);
        }
    }
    if !devs.is_empty() {
        group_registry.set_dev_selectors(devs);
    }

    // Generate utility CSS
    let mut util_buf = Vec::new();
    generator::generate_class_rules_only(&mut util_buf, classes.iter(), group_registry);

    let mut util_body = String::new();
    for line in String::from_utf8_lossy(&util_buf).lines() {
        if line.trim().is_empty() {
            continue;
        }
        util_body.push_str(line);
        util_body.push('\n');
    }

    let mut buf = Vec::new();
    buf.extend_from_slice(b"@layer utilities {\n");
    for line in util_body.lines() {
        if line.is_empty() {
            continue;
        }
        buf.extend_from_slice(b"  ");
        buf.extend_from_slice(line.as_bytes());
        buf.push(b'\n');
    }
    buf.extend_from_slice(b"}\n");

    buf
}
