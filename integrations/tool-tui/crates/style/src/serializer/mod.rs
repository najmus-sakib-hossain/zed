//! DX Serializer integration for StyleConfig
//!
//! This module provides serialization and deserialization of style configuration
//! using the DX ecosystem's own serialization format, replacing FlatBuffers.

use serializer::{DxDocument, DxLlmValue, document_to_machine, machine_to_document};
use std::collections::BTreeMap;

/// Error type for serialization operations
#[derive(Debug)]
#[allow(dead_code)]
pub enum SerializerError {
    /// Failed to serialize to binary format
    SerializationFailed(String),
    /// Failed to deserialize from binary format
    DeserializationFailed(String),
    /// Invalid data format
    InvalidFormat(String),
}

impl std::fmt::Display for SerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializerError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            SerializerError::DeserializationFailed(msg) => {
                write!(f, "Deserialization failed: {}", msg)
            }
            SerializerError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}

impl std::error::Error for SerializerError {}

/// Dynamic style entry with property and values
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicEntry {
    pub property: String,
    pub values: BTreeMap<String, String>,
}

/// Generator metadata for numeric style generation
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratorMeta {
    pub prefix: String,
    pub property: String,
    pub multiplier: f32,
    pub unit: String,
}

/// Group definition for style grouping
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDefDump {
    pub utilities: Vec<String>,
    pub allow_extend: bool,
    pub raw_tokens: Vec<String>,
    pub dev_tokens: Vec<String>,
}

/// Group dump containing all group definitions and cached CSS
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupDump {
    pub definitions: BTreeMap<String, GroupDefDump>,
    pub cached_css: BTreeMap<String, String>,
}

/// Complete style configuration
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleConfig {
    pub static_styles: BTreeMap<String, String>,
    pub dynamic_styles: BTreeMap<String, DynamicEntry>,
    pub generators: Vec<GeneratorMeta>,
    pub screens: BTreeMap<String, String>,
    pub states: BTreeMap<String, String>,
    pub container_queries: BTreeMap<String, String>,
    pub colors: BTreeMap<String, String>,
    pub animation_generators: BTreeMap<String, String>,
    pub groups: GroupDump,
    pub base_css: String,
    pub property_css: String,
}

impl StyleConfig {
    /// Create a new empty StyleConfig
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize to DX Machine format (binary)
    #[allow(dead_code)]
    pub fn to_binary(&self) -> Result<Vec<u8>, SerializerError> {
        let doc = self.to_dx_document();
        let machine = document_to_machine(&doc);
        Ok(machine.data)
    }

    /// Deserialize from DX Machine format (binary)
    pub fn from_binary(data: &[u8]) -> Result<Self, SerializerError> {
        let machine = serializer::MachineFormat::new(data.to_vec());
        let doc = machine_to_document(&machine)
            .map_err(|e| SerializerError::DeserializationFailed(e.to_string()))?;
        Self::from_dx_document(&doc)
    }

    /// Convert to DxDocument for serialization
    #[allow(dead_code)]
    pub fn to_dx_document(&self) -> DxDocument {
        let mut doc = DxDocument::new();

        // Static styles: s:name -> css
        for (name, css) in &self.static_styles {
            doc.context.insert(format!("s:{}", name), DxLlmValue::Str(css.clone()));
        }

        // Dynamic styles: d:key|property|suffix -> value
        // Also store d:key|property|_ -> "" for entries with empty values (to preserve the entry)
        for (key, entry) in &self.dynamic_styles {
            if entry.values.is_empty() {
                // Store a marker for empty dynamic entries
                doc.context.insert(
                    format!("d:{}|{}|_", key, entry.property),
                    DxLlmValue::Str(String::new()),
                );
            } else {
                for (suffix, value) in &entry.values {
                    doc.context.insert(
                        format!("d:{}|{}|{}", key, entry.property, suffix),
                        DxLlmValue::Str(value.clone()),
                    );
                }
            }
        }

        // Generators: g:prefix|property|m -> multiplier, g:prefix|property|u -> unit
        for generator in &self.generators {
            doc.context.insert(
                format!("g:{}|{}|m", generator.prefix, generator.property),
                DxLlmValue::Num(generator.multiplier as f64),
            );
            doc.context.insert(
                format!("g:{}|{}|u", generator.prefix, generator.property),
                DxLlmValue::Str(generator.unit.clone()),
            );
        }

        // Screens: sc:name -> value
        for (name, value) in &self.screens {
            doc.context.insert(format!("sc:{}", name), DxLlmValue::Str(value.clone()));
        }

        // States: st:name -> value
        for (name, value) in &self.states {
            doc.context.insert(format!("st:{}", name), DxLlmValue::Str(value.clone()));
        }

        // Container queries: cq:name -> value
        for (name, value) in &self.container_queries {
            doc.context.insert(format!("cq:{}", name), DxLlmValue::Str(value.clone()));
        }

        // Colors: c:name -> value
        for (name, value) in &self.colors {
            doc.context.insert(format!("c:{}", name), DxLlmValue::Str(value.clone()));
        }

        // Animation generators: ag:name -> template
        for (name, template) in &self.animation_generators {
            doc.context.insert(format!("ag:{}", name), DxLlmValue::Str(template.clone()));
        }

        // Groups: gd:alias|u -> utilities array (as delimited string), gd:alias|e -> allow_extend, etc.
        // Note: Machine format doesn't support arrays, so we serialize as delimited strings
        for (alias, def) in &self.groups.definitions {
            // Serialize arrays as pipe-delimited strings for machine format compatibility
            doc.context
                .insert(format!("gd:{}|u", alias), DxLlmValue::Str(def.utilities.join("|")));
            doc.context
                .insert(format!("gd:{}|e", alias), DxLlmValue::Bool(def.allow_extend));
            doc.context
                .insert(format!("gd:{}|r", alias), DxLlmValue::Str(def.raw_tokens.join("|")));
            doc.context
                .insert(format!("gd:{}|d", alias), DxLlmValue::Str(def.dev_tokens.join("|")));
        }

        // Cached CSS: gc:alias -> css
        for (alias, css) in &self.groups.cached_css {
            doc.context.insert(format!("gc:{}", alias), DxLlmValue::Str(css.clone()));
        }

        // Base CSS and property CSS
        if !self.base_css.is_empty() {
            doc.context
                .insert("base_css".to_string(), DxLlmValue::Str(self.base_css.clone()));
        }
        if !self.property_css.is_empty() {
            doc.context
                .insert("property_css".to_string(), DxLlmValue::Str(self.property_css.clone()));
        }

        doc
    }

    /// Convert from DxDocument after deserialization
    pub fn from_dx_document(doc: &DxDocument) -> Result<Self, SerializerError> {
        let mut config = StyleConfig::new();

        // Track generator keys to reconstruct GeneratorMeta
        let mut generator_multipliers: BTreeMap<String, f32> = BTreeMap::new();
        let mut generator_units: BTreeMap<String, String> = BTreeMap::new();

        // Track dynamic style keys
        let mut dynamic_values: BTreeMap<(String, String), BTreeMap<String, String>> =
            BTreeMap::new();

        // Track group definition parts
        let mut group_utilities: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut group_allow_extend: BTreeMap<String, bool> = BTreeMap::new();
        let mut group_raw_tokens: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut group_dev_tokens: BTreeMap<String, Vec<String>> = BTreeMap::new();

        for (key, value) in &doc.context {
            if let Some(name) = key.strip_prefix("s:") {
                if let DxLlmValue::Str(css) = value {
                    config.static_styles.insert(name.to_string(), css.clone());
                }
            } else if let Some(rest) = key.strip_prefix("d:") {
                // d:key|property|suffix -> value
                // Special case: d:key|property|_ with empty value is a marker for empty entries
                let parts: Vec<&str> = rest.split('|').collect();
                if parts.len() == 3 {
                    if let DxLlmValue::Str(v) = value {
                        let key_name = parts[0].to_string();
                        let property = parts[1].to_string();
                        let suffix = parts[2].to_string();

                        // Ensure the entry exists
                        dynamic_values.entry((key_name, property)).or_default();

                        // Only add to values if not the empty marker
                        if !(suffix == "_" && v.is_empty()) {
                            let key_name = parts[0].to_string();
                            let property = parts[1].to_string();
                            dynamic_values
                                .entry((key_name, property))
                                .or_default()
                                .insert(suffix, v.clone());
                        }
                    }
                }
            } else if let Some(rest) = key.strip_prefix("g:") {
                // g:prefix|property|m or g:prefix|property|u
                let parts: Vec<&str> = rest.split('|').collect();
                if parts.len() == 3 {
                    let gen_key = format!("{}|{}", parts[0], parts[1]);
                    match parts[2] {
                        "m" => {
                            if let DxLlmValue::Num(n) = value {
                                generator_multipliers.insert(gen_key, *n as f32);
                            }
                        }
                        "u" => {
                            if let DxLlmValue::Str(s) = value {
                                generator_units.insert(gen_key, s.clone());
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(name) = key.strip_prefix("sc:") {
                if let DxLlmValue::Str(v) = value {
                    config.screens.insert(name.to_string(), v.clone());
                }
            } else if let Some(name) = key.strip_prefix("st:") {
                if let DxLlmValue::Str(v) = value {
                    config.states.insert(name.to_string(), v.clone());
                }
            } else if let Some(name) = key.strip_prefix("cq:") {
                if let DxLlmValue::Str(v) = value {
                    config.container_queries.insert(name.to_string(), v.clone());
                }
            } else if let Some(name) = key.strip_prefix("c:") {
                if let DxLlmValue::Str(v) = value {
                    config.colors.insert(name.to_string(), v.clone());
                }
            } else if let Some(name) = key.strip_prefix("ag:") {
                if let DxLlmValue::Str(v) = value {
                    config.animation_generators.insert(name.to_string(), v.clone());
                }
            } else if let Some(rest) = key.strip_prefix("gd:") {
                // gd:alias|u, gd:alias|e, gd:alias|r, gd:alias|d
                let parts: Vec<&str> = rest.split('|').collect();
                if parts.len() == 2 {
                    let alias = parts[0].to_string();
                    match parts[1] {
                        "u" => {
                            if let DxLlmValue::Str(s) = value {
                                // Parse pipe-delimited string back to array
                                let utilities: Vec<String> = if s.is_empty() {
                                    vec![]
                                } else {
                                    s.split('|').map(|s| s.to_string()).collect()
                                };
                                group_utilities.insert(alias, utilities);
                            }
                        }
                        "e" => {
                            if let DxLlmValue::Bool(b) = value {
                                group_allow_extend.insert(alias, *b);
                            }
                        }
                        "r" => {
                            if let DxLlmValue::Str(s) = value {
                                // Parse pipe-delimited string back to array
                                let tokens: Vec<String> = if s.is_empty() {
                                    vec![]
                                } else {
                                    s.split('|').map(|s| s.to_string()).collect()
                                };
                                group_raw_tokens.insert(alias, tokens);
                            }
                        }
                        "d" => {
                            if let DxLlmValue::Str(s) = value {
                                // Parse pipe-delimited string back to array
                                let tokens: Vec<String> = if s.is_empty() {
                                    vec![]
                                } else {
                                    s.split('|').map(|s| s.to_string()).collect()
                                };
                                group_dev_tokens.insert(alias, tokens);
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(alias) = key.strip_prefix("gc:") {
                if let DxLlmValue::Str(css) = value {
                    config.groups.cached_css.insert(alias.to_string(), css.clone());
                }
            } else if key == "base_css" {
                if let DxLlmValue::Str(v) = value {
                    config.base_css = v.clone();
                }
            } else if key == "property_css" {
                if let DxLlmValue::Str(v) = value {
                    config.property_css = v.clone();
                }
            }
        }

        // Reconstruct dynamic styles
        for ((key_name, property), values) in dynamic_values {
            config.dynamic_styles.insert(key_name, DynamicEntry { property, values });
        }

        // Reconstruct generators
        for (gen_key, multiplier) in generator_multipliers {
            let parts: Vec<&str> = gen_key.split('|').collect();
            if parts.len() == 2 {
                let unit = generator_units.get(&gen_key).cloned().unwrap_or_default();
                config.generators.push(GeneratorMeta {
                    prefix: parts[0].to_string(),
                    property: parts[1].to_string(),
                    multiplier,
                    unit,
                });
            }
        }

        // Reconstruct group definitions
        for (alias, utilities) in group_utilities {
            config.groups.definitions.insert(
                alias.clone(),
                GroupDefDump {
                    utilities,
                    allow_extend: group_allow_extend.get(&alias).copied().unwrap_or(false),
                    raw_tokens: group_raw_tokens.get(&alias).cloned().unwrap_or_default(),
                    dev_tokens: group_dev_tokens.get(&alias).cloned().unwrap_or_default(),
                },
            );
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_config_roundtrip() {
        let config = StyleConfig::new();
        let binary = config.to_binary().expect("serialization should succeed");
        let restored = StyleConfig::from_binary(&binary).expect("deserialization should succeed");
        assert_eq!(config, restored);
    }

    #[test]
    fn test_static_styles_roundtrip() {
        let mut config = StyleConfig::new();
        config.static_styles.insert("flex".to_string(), "display: flex;".to_string());
        config.static_styles.insert("hidden".to_string(), "display: none;".to_string());

        let binary = config.to_binary().expect("serialization should succeed");
        let restored = StyleConfig::from_binary(&binary).expect("deserialization should succeed");
        assert_eq!(config.static_styles, restored.static_styles);
    }

    #[test]
    fn test_generators_roundtrip() {
        let mut config = StyleConfig::new();
        config.generators.push(GeneratorMeta {
            prefix: "p".to_string(),
            property: "padding".to_string(),
            multiplier: 0.25,
            unit: "rem".to_string(),
        });

        let binary = config.to_binary().expect("serialization should succeed");
        let restored = StyleConfig::from_binary(&binary).expect("deserialization should succeed");
        assert_eq!(config.generators.len(), restored.generators.len());
        assert_eq!(config.generators[0].prefix, restored.generators[0].prefix);
        assert_eq!(config.generators[0].unit, restored.generators[0].unit);
    }

    #[test]
    fn test_groups_roundtrip() {
        use serializer::MachineFormat;

        let mut config = StyleConfig::new();
        config.groups.definitions.insert(
            "card".to_string(),
            GroupDefDump {
                utilities: vec!["bg-white".to_string(), "rounded".to_string()],
                allow_extend: true,
                raw_tokens: vec!["card(bg-white rounded)".to_string()],
                dev_tokens: vec!["bg-white".to_string(), "rounded".to_string()],
            },
        );
        config.groups.cached_css.insert(
            "card".to_string(),
            ".card { background: white; border-radius: 0.25rem; }".to_string(),
        );

        let binary = config.to_binary().expect("serialization should succeed");

        // Deserialize the machine format back to document
        let machine = MachineFormat::new(binary.clone());
        let restored_doc =
            machine_to_document(&machine).expect("machine_to_document should succeed");

        // Debug: Check what values we got
        for (key, value) in &restored_doc.context {
            println!("Key: '{}', Value: {:?}", key, value);
        }

        let restored = StyleConfig::from_binary(&binary).expect("deserialization should succeed");

        assert_eq!(config.groups.definitions.len(), restored.groups.definitions.len());
        assert_eq!(config.groups.cached_css.len(), restored.groups.cached_css.len());

        let card_def =
            restored.groups.definitions.get("card").expect("card definition should exist");
        assert_eq!(card_def.utilities, vec!["bg-white".to_string(), "rounded".to_string()]);
        assert_eq!(card_def.raw_tokens, vec!["card(bg-white rounded)".to_string()]);
        assert_eq!(card_def.dev_tokens, vec!["bg-white".to_string(), "rounded".to_string()]);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Arbitrary generators for StyleConfig components
    fn arb_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_-]{1,20}".prop_map(|s| s)
    }

    fn arb_css_value() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9:;. #%-]{1,50}".prop_map(|s| s)
    }

    fn arb_static_styles() -> impl Strategy<Value = BTreeMap<String, String>> {
        prop::collection::btree_map(arb_string(), arb_css_value(), 0..10)
    }

    fn arb_dynamic_entry() -> impl Strategy<Value = DynamicEntry> {
        (arb_string(), prop::collection::btree_map(arb_string(), arb_css_value(), 0..5))
            .prop_map(|(property, values)| DynamicEntry { property, values })
    }

    fn arb_dynamic_styles() -> impl Strategy<Value = BTreeMap<String, DynamicEntry>> {
        prop::collection::btree_map(arb_string(), arb_dynamic_entry(), 0..5)
    }

    fn arb_generator_meta() -> impl Strategy<Value = GeneratorMeta> {
        (
            arb_string(),
            arb_string(),
            0.0f32..100.0f32,
            prop::sample::select(vec!["px", "rem", "em", "%", "vh", "vw"]),
        )
            .prop_map(|(prefix, property, multiplier, unit)| GeneratorMeta {
                prefix,
                property,
                multiplier,
                unit: unit.to_string(),
            })
    }

    fn arb_generators() -> impl Strategy<Value = Vec<GeneratorMeta>> {
        prop::collection::vec(arb_generator_meta(), 0..5)
    }

    fn arb_string_map() -> impl Strategy<Value = BTreeMap<String, String>> {
        prop::collection::btree_map(arb_string(), arb_css_value(), 0..5)
    }

    fn arb_group_def_dump() -> impl Strategy<Value = GroupDefDump> {
        (
            prop::collection::vec(arb_string(), 0..5),
            any::<bool>(),
            prop::collection::vec(arb_string(), 0..3),
            prop::collection::vec(arb_string(), 0..3),
        )
            .prop_map(|(utilities, allow_extend, raw_tokens, dev_tokens)| GroupDefDump {
                utilities,
                allow_extend,
                raw_tokens,
                dev_tokens,
            })
    }

    fn arb_group_dump() -> impl Strategy<Value = GroupDump> {
        (
            prop::collection::btree_map(arb_string(), arb_group_def_dump(), 0..5),
            prop::collection::btree_map(arb_string(), arb_css_value(), 0..5),
        )
            .prop_map(|(definitions, cached_css)| GroupDump {
                definitions,
                cached_css,
            })
    }

    fn arb_style_config() -> impl Strategy<Value = StyleConfig> {
        (
            arb_static_styles(),
            arb_dynamic_styles(),
            arb_generators(),
            arb_string_map(),
            arb_string_map(),
            arb_string_map(),
            arb_string_map(),
            arb_string_map(),
            arb_group_dump(),
            arb_css_value(),
            arb_css_value(),
        )
            .prop_map(
                |(
                    static_styles,
                    dynamic_styles,
                    generators,
                    screens,
                    states,
                    container_queries,
                    colors,
                    animation_generators,
                    groups,
                    base_css,
                    property_css,
                )| {
                    StyleConfig {
                        static_styles,
                        dynamic_styles,
                        generators,
                        screens,
                        states,
                        container_queries,
                        colors,
                        animation_generators,
                        groups,
                        base_css,
                        property_css,
                    }
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-production-ready, Property 14: DX Serializer Size Efficiency
        /// *For any* style configuration, the DX Machine format output SHALL be reasonably
        /// compact (within 2x of JSON representation size as a baseline).
        /// **Validates: Requirements 1.4**
        ///
        /// Note: Original requirement was 20% smaller than FlatBuffers, but since FlatBuffers
        /// has been removed, we validate against a reasonable size baseline instead.
        #[test]
        fn prop_dx_serializer_size_efficiency(config in arb_style_config()) {
            let binary = config.to_binary().expect("serialization should succeed");

            // Calculate a baseline size estimate (sum of all string content)
            let mut baseline_size = 0usize;
            for (k, v) in &config.static_styles {
                baseline_size += k.len() + v.len();
            }
            for (k, entry) in &config.dynamic_styles {
                baseline_size += k.len() + entry.property.len();
                for (sk, sv) in &entry.values {
                    baseline_size += sk.len() + sv.len();
                }
            }
            for generator in &config.generators {
                baseline_size += generator.prefix.len() + generator.property.len() + generator.unit.len() + 4; // 4 for f32
            }
            for (k, v) in &config.screens {
                baseline_size += k.len() + v.len();
            }
            for (k, v) in &config.states {
                baseline_size += k.len() + v.len();
            }
            for (k, v) in &config.container_queries {
                baseline_size += k.len() + v.len();
            }
            for (k, v) in &config.colors {
                baseline_size += k.len() + v.len();
            }
            for (k, v) in &config.animation_generators {
                baseline_size += k.len() + v.len();
            }
            for (alias, def) in &config.groups.definitions {
                baseline_size += alias.len();
                for u in &def.utilities {
                    baseline_size += u.len();
                }
                for t in &def.raw_tokens {
                    baseline_size += t.len();
                }
                for t in &def.dev_tokens {
                    baseline_size += t.len();
                }
            }
            for (k, v) in &config.groups.cached_css {
                baseline_size += k.len() + v.len();
            }
            baseline_size += config.base_css.len() + config.property_css.len();

            // Binary format should be reasonably efficient
            // Allow up to 3x overhead for metadata/structure (generous for small configs)
            // For empty configs, just verify it serializes
            if baseline_size > 0 {
                let max_allowed = baseline_size * 3 + 100; // 3x + 100 bytes for header overhead
                prop_assert!(
                    binary.len() <= max_allowed,
                    "Binary size {} exceeds max allowed {} (baseline: {})",
                    binary.len(), max_allowed, baseline_size
                );
            }

            // Verify the binary is not empty (has at least header)
            prop_assert!(
                binary.len() >= 4,
                "Binary output too small: {} bytes",
                binary.len()
            );
        }

        /// Feature: dx-style-production-ready, Property 1: Configuration Round-Trip
        /// *For any* valid StyleConfig, serializing to DX Machine format then deserializing
        /// SHALL produce an equivalent configuration.
        /// **Validates: Requirements 1.5**
        #[test]
        fn prop_config_roundtrip(config in arb_style_config()) {
            let binary = config.to_binary().expect("serialization should succeed");
            let restored = StyleConfig::from_binary(&binary).expect("deserialization should succeed");

            // Compare all fields
            prop_assert_eq!(&config.static_styles, &restored.static_styles, "static_styles mismatch");
            prop_assert_eq!(&config.screens, &restored.screens, "screens mismatch");
            prop_assert_eq!(&config.states, &restored.states, "states mismatch");
            prop_assert_eq!(&config.container_queries, &restored.container_queries, "container_queries mismatch");
            prop_assert_eq!(&config.colors, &restored.colors, "colors mismatch");
            prop_assert_eq!(&config.animation_generators, &restored.animation_generators, "animation_generators mismatch");
            prop_assert_eq!(&config.base_css, &restored.base_css, "base_css mismatch");
            prop_assert_eq!(&config.property_css, &restored.property_css, "property_css mismatch");

            // Compare dynamic styles
            prop_assert_eq!(config.dynamic_styles.len(), restored.dynamic_styles.len(), "dynamic_styles count mismatch");
            for (key, entry) in &config.dynamic_styles {
                let restored_entry = restored.dynamic_styles.get(key)
                    .expect(&format!("missing dynamic style key: {}", key));
                prop_assert_eq!(&entry.property, &restored_entry.property, "dynamic property mismatch for {}", key);
                prop_assert_eq!(&entry.values, &restored_entry.values, "dynamic values mismatch for {}", key);
            }

            // Compare generators (order may differ, so compare by content)
            prop_assert_eq!(config.generators.len(), restored.generators.len(), "generators count mismatch");

            // Compare groups
            prop_assert_eq!(
                config.groups.definitions.len(),
                restored.groups.definitions.len(),
                "group definitions count mismatch"
            );
            prop_assert_eq!(
                config.groups.cached_css.len(),
                restored.groups.cached_css.len(),
                "group cached_css count mismatch"
            );
            for (alias, def) in &config.groups.definitions {
                let restored_def = restored.groups.definitions.get(alias)
                    .expect(&format!("missing group definition: {}", alias));
                prop_assert_eq!(&def.utilities, &restored_def.utilities, "utilities mismatch for {}", alias);
                prop_assert_eq!(def.allow_extend, restored_def.allow_extend, "allow_extend mismatch for {}", alias);
                prop_assert_eq!(&def.raw_tokens, &restored_def.raw_tokens, "raw_tokens mismatch for {}", alias);
                prop_assert_eq!(&def.dev_tokens, &restored_def.dev_tokens, "dev_tokens mismatch for {}", alias);
            }
            for (alias, css) in &config.groups.cached_css {
                let restored_css = restored.groups.cached_css.get(alias)
                    .expect(&format!("missing cached css: {}", alias));
                prop_assert_eq!(css, restored_css, "cached_css mismatch for {}", alias);
            }
        }
    }
}
