//! Style Engine module
//!
//! The core CSS generation engine that transforms utility class names into CSS rules.
//! Supports precompiled styles, dynamic generation, screens, states, container queries,
//! and color theming.
//!
//! Key components:
//! - `StyleEngine`: Main engine for CSS generation
//! - `GeneratorMeta`: Metadata for dynamic CSS generators
//! - `PropertyMeta`: CSS property definitions
//! - `ThemeDefinition`: Theme token definitions

pub mod composite;
pub mod container_queries;
pub mod dynamic;
pub mod screens;
pub mod states;

pub use composite::expand_composite;
pub use dynamic::generate_dynamic_css;
pub use screens::{build_block, sanitize_declarations, wrap_media_queries};
pub use states::apply_wrappers_and_states;

#[allow(dead_code)]
pub fn init() {}

use ahash::AHashMap;
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use crate::core::color::{color::Argb, format_argb_as_oklch, theme::ThemeBuilder};
use crate::serializer::StyleConfig;

const DX_FONT_TOKENS: &[(&str, &str)] = &[
    ("font-sans", "Geist, sans-serif"),
    ("font-serif", "Georgia, serif"),
    ("font-mono", "Geist Mono, monospace"),
];

const DX_BASE_TOKENS: &[(&str, &str)] = &[("radius", "0.5rem")];

const DEFAULT_THEME_SOURCE: u32 = 0xFF6750A4;

#[derive(Clone)]
pub struct GeneratorMeta {
    pub prefix: String,
    pub property: String,
    pub multiplier: f32,
    pub unit: String,
}

pub struct StyleEngine {
    pub(crate) precompiled: AHashMap<String, String>,
    pub(crate) _mmap: Arc<Mmap>,
    pub screens: AHashMap<String, String>,
    pub states: AHashMap<String, String>,
    pub container_queries: AHashMap<String, String>,
    pub colors: AHashMap<String, String>,
    pub generators: Option<Vec<GeneratorMeta>>,
    pub generator_map: Option<AHashMap<String, usize>>,
    #[allow(dead_code)]
    pub properties: Vec<PropertyMeta>,
    #[allow(dead_code)]
    pub themes: Vec<ThemeDefinition>,
    pub theme_lookup: AHashMap<String, usize>,
    pub property_css: String,
    pub base_layer_raw: Option<String>,
    pub property_layer_raw: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct PropertyMeta {
    pub name: String,
    pub syntax: String,
    pub inherits: bool,
    pub initial: String,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ThemeDefinition {
    pub name: String,
    pub tokens: Vec<(String, String)>,
}

impl StyleEngine {
    pub fn load_from_disk() -> Result<Self, Box<dyn std::error::Error>> {
        let override_path = std::env::var("DX_STYLE_DXM").ok();
        let path_buf;
        let path = if let Some(p) = override_path.as_deref() {
            path_buf = std::path::PathBuf::from(p);
            &path_buf
        } else {
            Path::new(".dx/style/style.dxm")
        };

        // Read the DX Machine format file
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Deserialize using DX Serializer
        let config = StyleConfig::from_binary(&mmap)
            .map_err(|e| format!("Failed to parse style.dxm: {}", e))?;

        // Build precompiled styles from static styles
        let mut precompiled = AHashMap::new();
        for (name, css) in &config.static_styles {
            if name.is_empty() || css.is_empty() {
                continue;
            }
            precompiled.insert(name.clone(), css.trim_end().trim_end_matches(';').to_string());
        }

        // Add dynamic styles to precompiled
        for (key, entry) in &config.dynamic_styles {
            for (suffix, value) in &entry.values {
                let name = if suffix.is_empty() {
                    key.clone()
                } else {
                    format!("{}-{}", key, suffix)
                };
                if !name.is_empty() {
                    let css =
                        format!("{}: {}", entry.property, value.trim_end().trim_end_matches(';'));
                    precompiled.insert(name, css);
                }
            }
        }

        // Convert screens
        let screens: AHashMap<String, String> = config.screens.into_iter().collect();

        // Convert states
        let states: AHashMap<String, String> = config.states.into_iter().collect();

        // Convert container queries
        let container_queries: AHashMap<String, String> =
            config.container_queries.into_iter().collect();

        // Convert colors
        let colors: AHashMap<String, String> = config.colors.into_iter().collect();

        // Convert generators
        let generators: Option<Vec<GeneratorMeta>> = if config.generators.is_empty() {
            None
        } else {
            Some(
                config
                    .generators
                    .iter()
                    .map(|g| GeneratorMeta {
                        prefix: g.prefix.clone(),
                        property: g.property.clone(),
                        multiplier: g.multiplier,
                        unit: g.unit.clone(),
                    })
                    .collect(),
            )
        };

        let generator_map = generators.as_ref().map(|vec| {
            let mut m = AHashMap::new();
            for (i, g) in vec.iter().enumerate() {
                m.insert(g.prefix.clone(), i);
            }
            m
        });

        // Properties and themes are not yet stored in StyleConfig
        // These will be added in a future iteration
        let properties: Vec<PropertyMeta> = Vec::new();
        let themes: Vec<ThemeDefinition> = Vec::new();
        let theme_lookup: AHashMap<String, usize> = AHashMap::new();

        // Generate property CSS (empty for now since properties aren't loaded)
        let property_css = String::new();

        let base_layer_raw = if config.base_css.is_empty() {
            None
        } else {
            Some(config.base_css.clone())
        };

        let property_layer_raw = if config.property_css.is_empty() {
            None
        } else {
            Some(config.property_css.clone())
        };

        Ok(Self {
            precompiled,
            _mmap: Arc::new(mmap),
            screens,
            states,
            container_queries,
            colors,
            generators,
            generator_map,
            properties,
            themes,
            theme_lookup,
            property_css,
            base_layer_raw,
            property_layer_raw,
        })
    }

    pub fn empty() -> Self {
        let override_path = std::env::var("DX_STYLE_DXM").ok();
        let default_path = override_path.as_deref().unwrap_or(".dx/style/style.dxm");
        let file = File::options().read(true).write(false).open(default_path).ok();
        let mmap = file.and_then(|f| unsafe { Mmap::map(&f).ok() });
        fn anon_read_only_mmap() -> Option<Mmap> {
            let anon = MmapOptions::new().len(1).map_anon().ok()?;
            anon.make_read_only().ok()
        }
        StyleEngine {
            precompiled: AHashMap::new(),
            _mmap: Arc::new(mmap.unwrap_or_else(|| {
                let file = File::options().read(true).write(false).open(default_path).ok();
                if let Some(file) = file {
                    // SAFETY: Mmap::map requires unsafe but is safe when file is valid
                    unsafe { Mmap::map(&file) }.ok().or_else(anon_read_only_mmap).unwrap_or_else(
                        || {
                            // Fallback to empty mmap if all else fails
                            MmapOptions::new()
                                .len(1)
                                .map_anon()
                                .ok()
                                .and_then(|m| m.make_read_only().ok())
                                .unwrap_or_else(|| {
                                    // Last resort: create minimal valid mmap
                                    // SAFETY: Creating zeroed Mmap is safe as fallback
                                    unsafe { std::mem::zeroed() }
                                })
                        },
                    )
                } else {
                    anon_read_only_mmap().unwrap_or_else(|| {
                        // SAFETY: Creating zeroed Mmap is safe as fallback
                        unsafe { std::mem::zeroed() }
                    })
                }
            })),
            screens: AHashMap::new(),
            states: AHashMap::new(),
            container_queries: AHashMap::new(),
            colors: AHashMap::new(),
            generators: None,
            generator_map: None,
            properties: Vec::new(),
            themes: Vec::new(),
            theme_lookup: AHashMap::new(),
            property_css: String::new(),
            base_layer_raw: None,
            property_layer_raw: None,
        }
    }

    pub fn property_at_rules(&self) -> String {
        self.property_css.clone()
    }

    pub fn theme_by_name(&self, name: &str) -> Option<&ThemeDefinition> {
        self.theme_lookup.get(name).and_then(|idx| self.themes.get(*idx))
    }

    pub fn compute_css(&self, class_name: &str) -> Option<String> {
        // MEMOIZATION: Check cache first for instant return
        crate::core::lazy_gen::get_or_generate_css(class_name, || {
            self.compute_css_uncached(class_name)
        })
    }

    fn compute_css_uncached(&self, class_name: &str) -> Option<String> {
        // FAST PATH: Check atomic class perfect hash first (<1µs lookup)
        if let Some(css) = crate::core::atomic::lookup_atomic_class(class_name) {
            let mut escaped_ident = String::with_capacity(class_name.len() + 8);
            struct Acc<'a> {
                buf: &'a mut String,
            }
            impl<'a> std::fmt::Write for Acc<'a> {
                fn write_str(&mut self, s: &str) -> std::fmt::Result {
                    self.buf.push_str(s);
                    Ok(())
                }
            }
            if cssparser::serialize_identifier(
                class_name,
                &mut Acc {
                    buf: &mut escaped_ident,
                },
            )
            .is_err()
            {
                for ch in class_name.chars() {
                    match ch {
                        ':' => escaped_ident.push_str("\\:"),
                        '@' => escaped_ident.push_str("\\@"),
                        '(' => escaped_ident.push_str("\\("),
                        ')' => escaped_ident.push_str("\\)"),
                        ' ' => escaped_ident.push_str("\\ "),
                        '/' => escaped_ident.push_str("\\/"),
                        '\\' => escaped_ident.push_str("\\\\"),
                        _ => escaped_ident.push(ch),
                    }
                }
            }
            return Some(format!(".{} {{ {} }}", escaped_ident, css));
        }

        // SLOW PATH: Full CSS generation for non-atomic classes
        if class_name.starts_with("from(")
            || class_name.starts_with("to(")
            || class_name.starts_with("via(")
        {
            return None;
        }
        let mut last_colon = None;
        for (i, b) in class_name.as_bytes().iter().enumerate() {
            if *b == b':' {
                last_colon = Some(i);
            }
        }
        let (prefix_segment, base_class) = if let Some(idx) = last_colon {
            (&class_name[..idx], &class_name[idx + 1..])
        } else {
            ("", class_name)
        };
        let (media_queries, pseudo_classes, wrappers) =
            crate::core::engine::apply_wrappers_and_states(self, prefix_segment);
        let core_css_raw = crate::core::engine::expand_composite(self, class_name)
            .or_else(|| self.precompiled.get(base_class).cloned())
            .or_else(|| crate::core::color::generate_color_css(self, base_class))
            .or_else(|| {
                if class_name.contains(' ') {
                    None
                } else {
                    crate::core::animation::generate_animation_css(class_name)
                }
            })
            .or_else(|| crate::core::engine::generate_dynamic_css(self, base_class))
            .or_else(|| crate::core::engine::expand_composite(self, base_class));
        core_css_raw.map(|mut css| {
            css = crate::core::engine::sanitize_declarations(&css);
            let mut escaped_ident = String::with_capacity(class_name.len() + 8);
            struct Acc<'a> {
                buf: &'a mut String,
            }
            impl<'a> std::fmt::Write for Acc<'a> {
                fn write_str(&mut self, s: &str) -> std::fmt::Result {
                    self.buf.push_str(s);
                    Ok(())
                }
            }
            if cssparser::serialize_identifier(
                class_name,
                &mut Acc {
                    buf: &mut escaped_ident,
                },
            )
            .is_err()
            {
                for ch in class_name.chars() {
                    match ch {
                        ':' => escaped_ident.push_str("\\:"),
                        '@' => escaped_ident.push_str("\\@"),
                        '(' => escaped_ident.push_str("\\("),
                        ')' => escaped_ident.push_str("\\)"),
                        ' ' => escaped_ident.push_str("\\ "),
                        '/' => escaped_ident.push_str("\\/"),
                        '\\' => escaped_ident.push_str("\\\\"),
                        _ => escaped_ident.push(ch),
                    }
                }
            }
            let mut selector =
                String::with_capacity(escaped_ident.len() + pseudo_classes.len() + 2);
            selector.push('.');
            selector.push_str(&escaped_ident);
            selector.push_str(&pseudo_classes);
            let blocks = self.decode_encoded_css(&css, &selector, &wrappers);
            crate::core::engine::wrap_media_queries(blocks, &media_queries)
        })
    }

    pub fn css_for_class(&self, class: &str) -> Option<String> {
        self.compute_css(class)
    }

    pub fn generate_color_vars_for<'a, I>(&self, classes: I) -> (String, String)
    where
        I: IntoIterator<Item = &'a String>,
    {
        use std::collections::BTreeSet;
        use std::fmt::Write as _;

        let mut needed: BTreeSet<&str> = BTreeSet::new();
        for c in classes.into_iter() {
            let base = c.rsplit(':').next().unwrap_or(c);
            if let Some(name) = base.strip_prefix("bg-") {
                needed.insert(name);
            }
            if let Some(name) = base.strip_prefix("text-") {
                needed.insert(name);
            }
        }

        let mut token_entries: Vec<(String, String)> = Vec::new();

        for name in &needed {
            if let Some(mut val) = crate::core::color::derive_color_value(self, name) {
                if let Some(oklch) = crate::core::color::normalize_color_to_oklch(&val) {
                    val = oklch;
                }
                token_entries.push(((*name).to_string(), val));
            }
        }

        let format_token_value = |raw: &str| -> String {
            let trimmed = raw.trim();
            let normalized = crate::core::color::normalize_color_to_oklch(trimmed)
                .unwrap_or_else(|| trimmed.to_string());

            let mut out = String::with_capacity(normalized.len() + 8);
            let mut prev: Option<char> = None;
            let mut iter = normalized.chars().peekable();
            while let Some(ch) = iter.next() {
                if ch == '.' {
                    let prev_is_digit = prev.is_some_and(|p| p.is_ascii_digit());
                    let next_is_digit = iter.peek().is_some_and(|n| n.is_ascii_digit());
                    if !prev_is_digit && next_is_digit {
                        out.push('0');
                    }
                }
                out.push(ch);
                prev = Some(ch);
            }
            out
        };

        if let (Some(light_theme), Some(dark_theme)) =
            (self.theme_by_name("dx.light"), self.theme_by_name("dx.dark"))
        {
            let mut root = String::from(":root {\n");
            let mut dark = String::from(".dark {\n");

            for (name, value) in &light_theme.tokens {
                let _ = writeln!(root, "  --{}: {};", name, value);
            }

            for (name, value) in &dark_theme.tokens {
                let _ = writeln!(dark, "  --{}: {};", name, value);
            }

            root.push_str("}\n");
            dark.push_str("}\n");
            return (root, dark);
        }

        let mut root = String::from(":root {\n");
        let mut dark = String::from(".dark {\n");

        for (name, value) in DX_FONT_TOKENS {
            let normalized = format_token_value(value);
            let _ = writeln!(root, "  --{}: {};", name, normalized);
        }
        for (name, value) in DX_BASE_TOKENS {
            let normalized = format_token_value(value);
            let _ = writeln!(root, "  --{}: {};", name, normalized);
        }
        let theme = ThemeBuilder::with_source(Argb::from_u32(DEFAULT_THEME_SOURCE)).build();
        let light = &theme.schemes.light;
        let dark_scheme = &theme.schemes.dark;

        let write_argb_token = |buffer: &mut String, name: &str, color: Argb| {
            let color_str = format_argb_as_oklch(color);
            let normalized = format_token_value(&color_str);
            let _ = writeln!(buffer, "  --{}: {};", name, normalized);
        };

        let write_raw_token = |buffer: &mut String, name: &str, value: &str| {
            let normalized = format_token_value(value);
            let _ = writeln!(buffer, "  --{}: {};", name, normalized);
        };

        // Surface & content tokens
        write_argb_token(&mut root, "background", light.background);
        write_argb_token(&mut dark, "background", dark_scheme.background);
        write_argb_token(&mut root, "foreground", light.on_background);
        write_argb_token(&mut dark, "foreground", dark_scheme.on_background);
        write_argb_token(&mut root, "card", light.surface);
        write_argb_token(&mut dark, "card", dark_scheme.surface);
        write_argb_token(&mut root, "card-foreground", light.on_surface);
        write_argb_token(&mut dark, "card-foreground", dark_scheme.on_surface);
        write_argb_token(&mut root, "popover", light.surface_bright);
        write_argb_token(&mut dark, "popover", dark_scheme.surface_dim);
        write_argb_token(&mut root, "popover-foreground", light.on_surface);
        write_argb_token(&mut dark, "popover-foreground", dark_scheme.on_surface);

        // Brand tokens
        write_argb_token(&mut root, "primary", light.primary);
        write_argb_token(&mut dark, "primary", dark_scheme.primary);
        write_argb_token(&mut root, "primary-foreground", light.on_primary);
        write_argb_token(&mut dark, "primary-foreground", dark_scheme.on_primary);
        write_argb_token(&mut root, "secondary", light.secondary);
        write_argb_token(&mut dark, "secondary", dark_scheme.secondary);
        write_argb_token(&mut root, "secondary-foreground", light.on_secondary);
        write_argb_token(&mut dark, "secondary-foreground", dark_scheme.on_secondary);
        write_argb_token(&mut root, "muted", light.surface_variant);
        write_argb_token(&mut dark, "muted", dark_scheme.surface_variant);
        write_argb_token(&mut root, "muted-foreground", light.on_surface_variant);
        write_argb_token(&mut dark, "muted-foreground", dark_scheme.on_surface_variant);
        write_argb_token(&mut root, "accent", light.tertiary);
        write_argb_token(&mut dark, "accent", dark_scheme.tertiary);
        write_argb_token(&mut root, "accent-foreground", light.on_tertiary);
        write_argb_token(&mut dark, "accent-foreground", dark_scheme.on_tertiary);
        write_argb_token(&mut root, "destructive", light.error);
        write_argb_token(&mut dark, "destructive", dark_scheme.error);
        write_argb_token(&mut root, "destructive-foreground", light.on_error);
        write_argb_token(&mut dark, "destructive-foreground", dark_scheme.on_error);

        // Interaction tokens
        write_argb_token(&mut root, "border", light.outline);
        write_argb_token(&mut dark, "border", dark_scheme.outline);
        write_argb_token(&mut root, "input", light.surface_container_high);
        write_argb_token(&mut dark, "input", dark_scheme.surface_container_high);
        write_argb_token(&mut root, "ring", light.surface_tint);
        write_argb_token(&mut dark, "ring", dark_scheme.surface_tint);

        // Chart palette (shared across themes)
        let chart_1 = theme.palettes.primary.tone(60);
        let chart_2 = theme.palettes.secondary.tone(60);
        let chart_3 = theme.palettes.tertiary.tone(60);
        let chart_4 = theme.palettes.primary.tone(80);
        let chart_5 = theme.palettes.secondary.tone(80);
        for (name, color) in [
            ("chart-1", chart_1),
            ("chart-2", chart_2),
            ("chart-3", chart_3),
            ("chart-4", chart_4),
            ("chart-5", chart_5),
        ] {
            write_argb_token(&mut root, name, color);
            write_argb_token(&mut dark, name, color);
        }

        // Sidebar tokens
        write_argb_token(&mut root, "sidebar", light.surface_container_low);
        write_argb_token(&mut dark, "sidebar", dark_scheme.surface_container_low);
        write_argb_token(&mut root, "sidebar-foreground", light.on_surface);
        write_argb_token(&mut dark, "sidebar-foreground", dark_scheme.on_surface);
        write_argb_token(&mut root, "sidebar-primary", light.primary);
        write_argb_token(&mut dark, "sidebar-primary", dark_scheme.primary);
        write_argb_token(&mut root, "sidebar-primary-foreground", light.on_primary);
        write_argb_token(&mut dark, "sidebar-primary-foreground", dark_scheme.on_primary);
        write_argb_token(&mut root, "sidebar-accent", light.secondary_container);
        write_argb_token(&mut dark, "sidebar-accent", dark_scheme.secondary_container);
        write_argb_token(&mut root, "sidebar-accent-foreground", light.on_secondary_container);
        write_argb_token(
            &mut dark,
            "sidebar-accent-foreground",
            dark_scheme.on_secondary_container,
        );
        write_argb_token(&mut root, "sidebar-border", light.outline_variant);
        write_argb_token(&mut dark, "sidebar-border", dark_scheme.outline_variant);
        write_argb_token(&mut root, "sidebar-ring", light.surface_tint);
        write_argb_token(&mut dark, "sidebar-ring", dark_scheme.surface_tint);

        // Shadows
        write_argb_token(&mut root, "shadow-color", light.shadow);
        write_argb_token(&mut dark, "shadow-color", dark_scheme.shadow);
        for target in [&mut root, &mut dark] {
            write_raw_token(target, "shadow-opacity", "0.18");
            write_raw_token(target, "shadow-blur", "2px");
            write_raw_token(target, "shadow-spread", "0px");
            write_raw_token(target, "shadow-offset-x", "0px");
            write_raw_token(target, "shadow-offset-y", "1px");
        }

        for (name, value) in &token_entries {
            let normalized = format_token_value(value);
            let _ = writeln!(root, "  --color-{}: {};", name, normalized);
            let _ = writeln!(dark, "  --color-{}: {};", name, normalized);
        }

        root.push_str("}\n");
        dark.push_str("}\n");
        (root, dark)
    }

    fn decode_encoded_css(&self, css: &str, selector: &str, wrappers: &[String]) -> String {
        use crate::core::engine::build_block;
        let is_encoded = [
            "BASE|", "STATE|", "CHILD|", "COND|", "DATA|", "RAW|", "ANIM|",
        ]
        .iter()
        .any(|p| css.contains(p));
        if !is_encoded {
            if wrappers.is_empty() {
                return build_block(selector, css);
            }
            let mut out = String::new();
            for w in wrappers {
                let sel = w.replace('&', selector);
                out.push_str(&build_block(&sel, css));
                out.push('\n');
            }
            if out.ends_with('\n') {
                out.pop();
            }
            return out;
        }
        let mut out = String::new();
        let mut pending_anim: Option<crate::core::animation::PendingAnimation> = None;
        let lines: Vec<&str> = if css.contains('\n') {
            css.lines().collect()
        } else {
            vec![css]
        };
        for line in lines {
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("BASE|") {
                if wrappers.is_empty() {
                    out.push_str(&build_block(selector, rest));
                } else {
                    for w in wrappers {
                        let sel = w.replace('&', selector);
                        out.push_str(&build_block(&sel, rest));
                        out.push('\n');
                    }
                    if out.ends_with('\n') {
                        out.pop();
                    }
                }
                out.push('\n');
            } else if let Some(rest) = line.strip_prefix("STATE|") {
                let mut parts = rest.splitn(2, '|');
                let state = parts.next().unwrap_or("");
                let decls = parts.next().unwrap_or("");
                if state == "dark" {
                    out.push_str(&build_block(&format!(".dark {}", selector), decls));
                } else if state == "light" {
                    out.push_str(&build_block(&format!(":root {}", selector), decls));
                    out.push('\n');
                    out.push_str(&build_block(&format!(".light {}", selector), decls));
                } else {
                    out.push_str(&build_block(&format!("{}:{}", selector, state), decls));
                }
                out.push('\n');
            } else if let Some(rest) = line.strip_prefix("CHILD|") {
                let mut parts = rest.splitn(2, '|');
                let child = parts.next().unwrap_or("");
                let decls = parts.next().unwrap_or("");
                out.push_str(&build_block(&format!("{} > {}", selector, child), decls));
                out.push('\n');
            } else if let Some(rest) = line.strip_prefix("DATA|") {
                let mut parts = rest.splitn(2, '|');
                let data = parts.next().unwrap_or("");
                let decls = parts.next().unwrap_or("");
                out.push_str(&build_block(&format!("{}[data-{}]", selector, data), decls));
                out.push('\n');
            } else if let Some(rest) = line.strip_prefix("COND|") {
                let mut parts = rest.splitn(2, '|');
                let cond = parts.next().unwrap_or("");
                let decls = parts.next().unwrap_or("");
                if let Some(val) = cond.strip_prefix("@container>") {
                    out.push_str(&format!("@container (min-width: {}) {{\n", val));
                    for l in build_block(selector, decls).lines() {
                        out.push_str("  ");
                        out.push_str(l);
                        out.push('\n');
                    }
                    out.push_str("}\n");
                } else if let Some(bp) = cond.strip_prefix("screen:") {
                    if let Some(v) = self.screens.get(bp) {
                        out.push_str(&format!("@media (min-width: {}) {{\n", v));
                        for l in build_block(selector, decls).lines() {
                            out.push_str("  ");
                            out.push_str(l);
                            out.push('\n');
                        }
                        out.push_str("}\n");
                    }
                }
            } else if line.starts_with("ANIM|") {
                crate::core::animation::process_anim_line(line, &mut pending_anim);
            } else if let Some(raw) = line.strip_prefix("RAW|") {
                out.push_str(raw);
                if !raw.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        crate::core::animation::decode_animation_if_pending(
            self,
            selector,
            &mut pending_anim,
            &mut out,
        );
        if out.ends_with('\n') {
            out.pop();
        }
        out
    }
}
