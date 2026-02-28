//! Build script for dx-style
//!
//! This script compiles style configuration from DX Serializer files into DX Machine format
//! for fast runtime loading. It also supports legacy TOML files by using dx-serializer's
//! toml_to_dx converter (which is part of dx-serializer, not a direct toml dependency).

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug, Clone)]
struct GeneratorConfig {
    multiplier: f32,
    unit: String,
}

#[derive(Deserialize, Debug)]
struct StaticConfig {
    #[serde(rename = "static")]
    static_styles: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct DynamicConfig {
    dynamic: HashMap<String, HashMap<String, String>>,
}

#[derive(Deserialize, Debug)]
struct GeneratorsConfig {
    generators: HashMap<String, GeneratorConfig>,
}

#[derive(Deserialize, Debug)]
struct ScreensConfig {
    screens: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct StatesConfig {
    states: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct ContainerQueriesConfig {
    container_queries: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct ColorsConfig {
    colors: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct AnimationGeneratorsConfig {
    animation_generators: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct PropertyMetaConfig {
    syntax: String,
    #[serde(default)]
    inherits: Option<bool>,
    #[serde(default, rename = "initial")]
    initial_value: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PropertiesConfig {
    properties: HashMap<String, PropertyMetaConfig>,
}

/// Read theme tokens from a DX Serializer format file
fn read_theme_tokens(path: &Path) -> Vec<(String, Vec<(String, String)>)> {
    if !path.exists() {
        return Vec::new();
    }
    let content = match fs::read_to_string(path) {
        Ok(data) => data,
        Err(_) => return Vec::new(),
    };

    let mut themes: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut current_theme: Option<usize> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let inner = line[1..line.len() - 1].trim();
            let name = if inner.starts_with('"') && inner.ends_with('"') && inner.len() >= 2 {
                inner[1..inner.len() - 1].to_string()
            } else {
                inner.to_string()
            };
            themes.push((name, Vec::new()));
            current_theme = Some(themes.len() - 1);
            continue;
        }

        let Some(theme_index) = current_theme else {
            continue;
        };

        let Some(eq_pos) = line.find('=') else {
            continue;
        };

        let key = line[..eq_pos].trim().trim_matches('"').to_string();
        let mut value_part = line[eq_pos + 1..].trim();
        if value_part.is_empty() {
            continue;
        }

        if value_part.starts_with('"') && value_part.ends_with('"') && value_part.len() >= 2 {
            value_part = &value_part[1..value_part.len() - 1];
        }

        themes[theme_index].1.push((key, value_part.to_string()));
    }

    themes
}

/// Read a DX Serializer format file and parse it using serde_json
/// DX Serializer files can be converted to JSON for parsing
fn read_dxs_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(path).ok()?;

    // Try to parse as DX format first by converting to JSON
    // DX format uses key=value syntax, we need to convert to JSON
    let json_content = dxs_to_json(&content);
    serde_json::from_str(&json_content).ok()
}

/// Convert DX Serializer format to JSON for parsing
/// This is a simple converter for build-time use
fn dxs_to_json(sr: &str) -> String {
    let mut result = String::from("{");
    let mut first = true;
    let mut in_section = false;
    let mut section_name = String::new();
    let mut section_content = String::new();

    for line in sr.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle section headers like [section]
        if line.starts_with('[') && line.ends_with(']') {
            // Close previous section if any
            if in_section && !section_name.is_empty() {
                if !first {
                    result.push(',');
                }
                result.push_str(&format!("\"{}\":{{{}}}", section_name, section_content));
                first = false;
                section_content.clear();
            }

            section_name = line[1..line.len() - 1].trim().trim_matches('"').to_string();
            in_section = true;
            continue;
        }

        // Handle key=value pairs
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().trim_matches('"');
            let value = line[eq_pos + 1..].trim();

            // Determine if value needs quoting - only quote plain strings
            let needs_quoting = !(value.starts_with('"') && value.ends_with('"'))
                && value != "true"
                && value != "false"
                && value.parse::<f64>().is_err()
                && !(value.starts_with('[') && value.ends_with(']'));

            let json_value = if needs_quoting {
                format!("\"{}\"", value)
            } else {
                value.to_string()
            };

            if in_section {
                if !section_content.is_empty() {
                    section_content.push(',');
                }
                section_content.push_str(&format!("\"{}\":{}", key, json_value));
            } else {
                if !first {
                    result.push(',');
                }
                result.push_str(&format!("\"{}\":{}", key, json_value));
                first = false;
            }
        }
    }

    // Close last section if any
    if in_section && !section_name.is_empty() {
        if !first {
            result.push(',');
        }
        result.push_str(&format!("\"{}\":{{{}}}", section_name, section_content));
    }

    result.push('}');
    result
}

/// Read configuration from either .sr or legacy .toml file
/// Prefers .sr if it exists, falls back to .toml
fn read_config_file<T: for<'de> Deserialize<'de>>(style_dir: &Path, base_name: &str) -> Option<T> {
    // Try .sr first (new DX Serializer format)
    let dxs_path = style_dir.join(format!("{}.sr", base_name));
    if dxs_path.exists()
        && let Some(config) = read_dxs_file(&dxs_path)
    {
        return Some(config);
    }

    // Fall back to .toml (legacy format) - use dx-serializer's converter
    let toml_path = style_dir.join(format!("{}.toml", base_name));
    if toml_path.exists()
        && let Ok(content) = fs::read_to_string(&toml_path)
    {
        // Use dx-serializer's toml_to_dx converter
        if let Ok(dxs_content) = serializer::toml_to_dx(&content) {
            let json_content = dxs_to_json(&dxs_content);
            if let Ok(config) = serde_json::from_str(&json_content) {
                return Some(config);
            }
        }
        // If conversion fails, try direct JSON parsing of TOML-like content
        // This handles simple key=value TOML files
        let json_content = dxs_to_json(&content);
        if let Ok(config) = serde_json::from_str(&json_content) {
            return Some(config);
        }
    }

    None
}

/// Get the style directory from config
fn get_style_dir() -> String {
    // Try .sr config first
    let dxs_config_path = Path::new(".dx/config.sr");
    if dxs_config_path.exists()
        && let Ok(content) = fs::read_to_string(dxs_config_path)
    {
        // Parse DX format to find paths.style_dir
        for line in content.lines() {
            let line = line.trim();
            if (line.starts_with("style_dir") || line.contains("style_dir"))
                && let Some(eq_pos) = line.find('=')
            {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if !value.is_empty() {
                    return value.replace('\\', "/");
                }
            }
        }
    }

    // Fall back to .toml config using dx-serializer's converter
    let toml_config_path = Path::new(".dx/config.toml");
    if toml_config_path.exists()
        && let Ok(content) = fs::read_to_string(toml_config_path)
    {
        // Use dx-serializer's toml_to_dx converter
        if let Ok(dxs_content) = serializer::toml_to_dx(&content) {
            for line in dxs_content.lines() {
                let line = line.trim();
                if (line.starts_with("style_dir") || line.contains("style_dir"))
                    && let Some(eq_pos) = line.find('=')
                {
                    let value = line[eq_pos + 1..].trim().trim_matches('"');
                    if !value.is_empty() {
                        return value.replace('\\', "/");
                    }
                }
            }
        }
        // Try simple parsing for legacy TOML
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("style_dir")
                && let Some(eq_pos) = line.find('=')
            {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if !value.is_empty() {
                    return value.replace('\\', "/");
                }
            }
        }
    }

    ".dx/style".to_string()
}

fn main() {
    let style_dir_str = get_style_dir();
    let style_dir = Path::new(&style_dir_str);

    println!("cargo:rerun-if-changed={}", style_dir.display());

    // Generate perfect hash for atomic classes
    generate_atomic_class_hash();

    // Read all configuration files (supports both .sr and legacy .toml)
    let static_styles = read_config_file::<StaticConfig>(style_dir, "static")
        .map(|c| c.static_styles)
        .unwrap_or_default();
    let dynamic = read_config_file::<DynamicConfig>(style_dir, "dynamic")
        .map(|c| c.dynamic)
        .unwrap_or_default();
    let generators = read_config_file::<GeneratorsConfig>(style_dir, "generators")
        .map(|c| c.generators)
        .unwrap_or_default();
    let screens = read_config_file::<ScreensConfig>(style_dir, "screens")
        .map(|c| c.screens)
        .unwrap_or_default();
    let states = read_config_file::<StatesConfig>(style_dir, "states")
        .map(|c| c.states)
        .unwrap_or_default();
    let container_queries =
        read_config_file::<ContainerQueriesConfig>(style_dir, "container_queries")
            .map(|c| c.container_queries)
            .unwrap_or_default();
    let colors = read_config_file::<ColorsConfig>(style_dir, "colors")
        .map(|c| c.colors)
        .unwrap_or_default();
    let animation_generators =
        read_config_file::<AnimationGeneratorsConfig>(style_dir, "animation_generators")
            .map(|c| c.animation_generators)
            .unwrap_or_default();
    let properties = read_config_file::<PropertiesConfig>(style_dir, "property")
        .map(|c| c.properties)
        .unwrap_or_default();

    // Read theme tokens (supports both .sr and legacy .toml)
    let themes = {
        let sr_path = style_dir.join("themes.sr");
        let toml_path = style_dir.join("themes.toml");
        if sr_path.exists() {
            read_theme_tokens(&sr_path)
        } else {
            read_theme_tokens(&toml_path)
        }
    };

    let base_css = fs::read_to_string(style_dir.join("base.css")).unwrap_or_default();
    let property_css = fs::read_to_string(style_dir.join("property.css")).unwrap_or_default();

    // Build DX Document using the LLM format
    use serializer::{DxDocument, DxLlmValue, document_to_machine};

    let mut doc = DxDocument::new();

    // Static styles: s:name -> css
    for (name, css) in &static_styles {
        doc.context.insert(format!("s:{}", name), DxLlmValue::Str(css.clone()));
    }

    // Dynamic styles: d:key|property|suffix -> value
    for (key, values) in &dynamic {
        let parts: Vec<&str> = key.split('|').collect();
        if parts.len() != 2 {
            println!(
                "cargo:warning=Invalid dynamic key format in dynamic config: '{}'. Skipping.",
                key
            );
            continue;
        }
        let key_name = parts[0];
        let property = parts[1];

        for (suffix, value) in values {
            doc.context.insert(
                format!("d:{}|{}|{}", key_name, property, suffix),
                DxLlmValue::Str(value.clone()),
            );
        }
    }

    // Generators: g:prefix|property|m -> multiplier, g:prefix|property|u -> unit
    for (key, config) in &generators {
        let parts: Vec<&str> = key.split('|').collect();
        if parts.len() != 2 {
            println!(
                "cargo:warning=Invalid generator key format in generators config: '{}'. Skipping.",
                key
            );
            continue;
        }
        let prefix = parts[0];
        let property = parts[1];

        doc.context.insert(
            format!("g:{}|{}|m", prefix, property),
            DxLlmValue::Num(config.multiplier as f64),
        );
        doc.context
            .insert(format!("g:{}|{}|u", prefix, property), DxLlmValue::Str(config.unit.clone()));
    }

    // Screens: sc:name -> value
    for (name, value) in &screens {
        doc.context.insert(format!("sc:{}", name), DxLlmValue::Str(value.clone()));
    }

    // States: st:name -> value
    for (name, value) in &states {
        doc.context.insert(format!("st:{}", name), DxLlmValue::Str(value.clone()));
    }

    // Container queries: cq:name -> value
    for (name, value) in &container_queries {
        doc.context.insert(format!("cq:{}", name), DxLlmValue::Str(value.clone()));
    }

    // Colors: c:name -> value
    for (name, value) in &colors {
        doc.context.insert(format!("c:{}", name), DxLlmValue::Str(value.clone()));
    }

    // Animation generators: ag:name -> template
    for (name, template) in &animation_generators {
        doc.context.insert(format!("ag:{}", name), DxLlmValue::Str(template.clone()));
    }

    // Properties: p:name|syntax -> syntax, p:name|inherits -> bool, p:name|initial -> value
    for (name, meta) in &properties {
        doc.context
            .insert(format!("p:{}|syntax", name), DxLlmValue::Str(meta.syntax.clone()));
        doc.context.insert(
            format!("p:{}|inherits", name),
            DxLlmValue::Bool(meta.inherits.unwrap_or(false)),
        );
        if let Some(ref initial) = meta.initial_value {
            doc.context
                .insert(format!("p:{}|initial", name), DxLlmValue::Str(initial.clone()));
        }
    }

    // Themes: t:theme_name|token_name -> token_value
    for (theme_name, tokens) in &themes {
        for (token_name, token_value) in tokens {
            doc.context.insert(
                format!("t:{}|{}", theme_name, token_name),
                DxLlmValue::Str(token_value.clone()),
            );
        }
    }

    // Base CSS and property CSS
    if !base_css.is_empty() {
        doc.context.insert("base_css".to_string(), DxLlmValue::Str(base_css));
    }
    if !property_css.is_empty() {
        doc.context.insert("property_css".to_string(), DxLlmValue::Str(property_css));
    }

    // Serialize to DX Machine format
    let machine = document_to_machine(&doc);
    let buf = machine.data;

    // Write to .dxm file
    let styles_dxm_path = style_dir.join("style.dxm");
    fs::create_dir_all(styles_dxm_path.parent().unwrap()).expect("Failed to create .dx directory");

    let needs_write = match fs::read(&styles_dxm_path) {
        Ok(existing) => existing.as_slice() != buf.as_slice(),
        Err(_) => true,
    };

    if needs_write {
        match fs::write(&styles_dxm_path, &buf) {
            Ok(_) => {}
            Err(e) => {
                if let Some(code) = e.raw_os_error() {
                    if code == 1224 {
                        let tmp = styles_dxm_path.with_extension("dxm.new");
                        if fs::write(&tmp, &buf).is_ok() {
                            let _ = fs::rename(&tmp, &styles_dxm_path);
                        }
                    } else {
                        panic!("Failed to write style.dxm: {:?}", e);
                    }
                } else {
                    panic!("Failed to write style.dxm: {:?}", e);
                }
            }
        }
    }

    println!("cargo:rustc-env=DX_STYLE_DXM={}", style_dir.join("style.dxm").to_string_lossy());
}

/// Generate perfect hash function for atomic CSS classes at compile time
/// This enables O(1) lookups with zero collisions for known classes
fn generate_atomic_class_hash() {
    use std::env;
    use std::io::Write;

    // Common atomic classes that should have perfect hash lookups
    // These are the most frequently used utility classes
    let atomic_classes = vec![
        // Display
        "block",
        "inline-block",
        "inline",
        "flex",
        "inline-flex",
        "grid",
        "inline-grid",
        "hidden",
        // Position
        "static",
        "fixed",
        "absolute",
        "relative",
        "sticky",
        // Flexbox
        "flex-row",
        "flex-col",
        "flex-wrap",
        "flex-nowrap",
        "items-start",
        "items-center",
        "items-end",
        "justify-start",
        "justify-center",
        "justify-end",
        "justify-between",
        // Spacing (common values)
        "m-0",
        "m-1",
        "m-2",
        "m-3",
        "m-4",
        "m-5",
        "m-6",
        "m-8",
        "m-10",
        "m-12",
        "m-16",
        "p-0",
        "p-1",
        "p-2",
        "p-3",
        "p-4",
        "p-5",
        "p-6",
        "p-8",
        "p-10",
        "p-12",
        "p-16",
        "mt-0",
        "mt-1",
        "mt-2",
        "mt-4",
        "mt-8",
        "mb-0",
        "mb-1",
        "mb-2",
        "mb-4",
        "mb-8",
        "ml-0",
        "ml-1",
        "ml-2",
        "ml-4",
        "ml-8",
        "mr-0",
        "mr-1",
        "mr-2",
        "mr-4",
        "mr-8",
        "pt-0",
        "pt-1",
        "pt-2",
        "pt-4",
        "pt-8",
        "pb-0",
        "pb-1",
        "pb-2",
        "pb-4",
        "pb-8",
        "pl-0",
        "pl-1",
        "pl-2",
        "pl-4",
        "pl-8",
        "pr-0",
        "pr-1",
        "pr-2",
        "pr-4",
        "pr-8",
        // Width/Height
        "w-full",
        "w-auto",
        "w-screen",
        "h-full",
        "h-auto",
        "h-screen",
        // Text
        "text-left",
        "text-center",
        "text-right",
        "text-sm",
        "text-base",
        "text-lg",
        "text-xl",
        "font-normal",
        "font-medium",
        "font-semibold",
        "font-bold",
        // Colors (common)
        "text-white",
        "text-black",
        "bg-white",
        "bg-black",
        "bg-transparent",
        // Border
        "border",
        "border-0",
        "border-2",
        "border-4",
        "rounded",
        "rounded-lg",
        "rounded-full",
        // Overflow
        "overflow-hidden",
        "overflow-auto",
        "overflow-scroll",
        // Cursor
        "cursor-pointer",
        "cursor-default",
        // Opacity
        "opacity-0",
        "opacity-50",
        "opacity-100",
        // Z-index
        "z-0",
        "z-10",
        "z-20",
        "z-30",
        "z-40",
        "z-50",
    ];

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("atomic_hash.rs");

    let mut file = std::fs::File::create(&dest_path).expect("Failed to create atomic_hash.rs");

    // Generate perfect hash map using phf
    writeln!(file, "// Auto-generated perfect hash for atomic CSS classes").unwrap();
    writeln!(file, "// DO NOT EDIT - generated by build.rs\n").unwrap();
    writeln!(file, "use phf::{{phf_map, Map}};\n").unwrap();

    writeln!(file, "/// Perfect hash map for atomic CSS class lookups").unwrap();
    writeln!(file, "/// Provides O(1) constant-time lookups with zero collisions").unwrap();
    writeln!(file, "pub static ATOMIC_CLASS_IDS: Map<&'static str, u16> = phf_map! {{").unwrap();

    for (idx, class) in atomic_classes.iter().enumerate() {
        writeln!(file, "    \"{}\" => {},", class, idx).unwrap();
    }

    writeln!(file, "}};").unwrap();
    writeln!(file).unwrap();
    writeln!(file, "/// Total number of atomic classes with perfect hash").unwrap();
    writeln!(file, "pub const ATOMIC_CLASS_COUNT: usize = {};", atomic_classes.len()).unwrap();

    println!("cargo:rerun-if-changed=build.rs");
}
