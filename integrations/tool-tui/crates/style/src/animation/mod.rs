//! Animation Engine
//!
//! Provides programmatic animation generation via class syntax.
//! Supports presets, timing parameters, composition, and custom keyframes.
//!
//! **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9**

use ahash::AHashMap;

/// Animation preset definition
#[derive(Debug, Clone)]
pub struct AnimationPreset {
    /// Preset name
    pub name: String,
    /// Keyframes for this animation
    pub keyframes: Vec<Keyframe>,
    /// Default duration
    pub default_duration: String,
    /// Default easing function
    pub default_easing: String,
}

/// Single keyframe in an animation
#[derive(Debug, Clone)]
pub struct Keyframe {
    /// Percentage (0-100)
    pub percent: u8,
    /// CSS properties at this keyframe
    pub properties: Vec<(String, String)>,
}

/// Parsed animation class
#[derive(Debug, Clone, Default)]
pub struct AnimationClass {
    /// Base animation name (fade-in, slide-up, etc.)
    pub name: String,
    /// Duration (e.g., "500ms", "1s")
    pub duration: Option<String>,
    /// Delay (e.g., "200ms")
    pub delay: Option<String>,
    /// Easing function (ease-out, ease-in-out, etc.)
    pub easing: Option<String>,
    /// Iteration count (1, 2, infinite)
    pub iteration: Option<String>,
    /// Direction (normal, reverse, alternate)
    pub direction: Option<String>,
    /// Fill mode (none, forwards, backwards, both)
    pub fill_mode: Option<String>,
    /// Composed animations (for animate-fade-in+slide-up)
    pub composed: Vec<String>,
}

/// Programmatic animation engine
///
/// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9**
pub struct AnimationEngine {
    /// Built-in animation presets
    presets: AHashMap<String, AnimationPreset>,
    /// Custom keyframes from config
    custom_keyframes: AHashMap<String, Vec<Keyframe>>,
}

impl AnimationEngine {
    /// Create a new animation engine with built-in presets
    pub fn new() -> Self {
        let mut engine = Self {
            presets: AHashMap::new(),
            custom_keyframes: AHashMap::new(),
        };
        engine.load_builtin_presets();
        engine
    }

    /// Load built-in animation presets
    ///
    /// **Validates: Requirements 4.7**
    fn load_builtin_presets(&mut self) {
        // Fade animations
        self.presets.insert(
            "fade-in".to_string(),
            AnimationPreset {
                name: "fade-in".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![("opacity".to_string(), "0".to_string())],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![("opacity".to_string(), "1".to_string())],
                    },
                ],
                default_duration: "300ms".to_string(),
                default_easing: "ease-out".to_string(),
            },
        );

        self.presets.insert(
            "fade-out".to_string(),
            AnimationPreset {
                name: "fade-out".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![("opacity".to_string(), "1".to_string())],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![("opacity".to_string(), "0".to_string())],
                    },
                ],
                default_duration: "300ms".to_string(),
                default_easing: "ease-in".to_string(),
            },
        );

        // Slide animations
        self.presets.insert(
            "slide-up".to_string(),
            AnimationPreset {
                name: "slide-up".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "translateY(20px)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "translateY(0)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                ],
                default_duration: "400ms".to_string(),
                default_easing: "ease-out".to_string(),
            },
        );

        self.presets.insert(
            "slide-down".to_string(),
            AnimationPreset {
                name: "slide-down".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "translateY(-20px)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "translateY(0)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                ],
                default_duration: "400ms".to_string(),
                default_easing: "ease-out".to_string(),
            },
        );

        self.presets.insert(
            "slide-left".to_string(),
            AnimationPreset {
                name: "slide-left".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "translateX(20px)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "translateX(0)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                ],
                default_duration: "400ms".to_string(),
                default_easing: "ease-out".to_string(),
            },
        );

        self.presets.insert(
            "slide-right".to_string(),
            AnimationPreset {
                name: "slide-right".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "translateX(-20px)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "translateX(0)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                ],
                default_duration: "400ms".to_string(),
                default_easing: "ease-out".to_string(),
            },
        );

        // Scale animations
        self.presets.insert(
            "scale-in".to_string(),
            AnimationPreset {
                name: "scale-in".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "scale(0.9)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "scale(1)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                ],
                default_duration: "300ms".to_string(),
                default_easing: "ease-out".to_string(),
            },
        );

        self.presets.insert(
            "scale-out".to_string(),
            AnimationPreset {
                name: "scale-out".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "scale(1)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "scale(0.9)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                ],
                default_duration: "300ms".to_string(),
                default_easing: "ease-in".to_string(),
            },
        );

        // Bounce animation
        self.presets.insert(
            "bounce".to_string(),
            AnimationPreset {
                name: "bounce".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![("transform".to_string(), "scale(1)".to_string())],
                    },
                    Keyframe {
                        percent: 50,
                        properties: vec![("transform".to_string(), "scale(1.1)".to_string())],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![("transform".to_string(), "scale(1)".to_string())],
                    },
                ],
                default_duration: "600ms".to_string(),
                default_easing: "ease-in-out".to_string(),
            },
        );

        // Pulse animation
        self.presets.insert(
            "pulse".to_string(),
            AnimationPreset {
                name: "pulse".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![("opacity".to_string(), "1".to_string())],
                    },
                    Keyframe {
                        percent: 50,
                        properties: vec![("opacity".to_string(), "0.5".to_string())],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![("opacity".to_string(), "1".to_string())],
                    },
                ],
                default_duration: "1000ms".to_string(),
                default_easing: "ease-in-out".to_string(),
            },
        );

        // Spin animation
        self.presets.insert(
            "spin".to_string(),
            AnimationPreset {
                name: "spin".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![("transform".to_string(), "rotate(0deg)".to_string())],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![("transform".to_string(), "rotate(360deg)".to_string())],
                    },
                ],
                default_duration: "1000ms".to_string(),
                default_easing: "linear".to_string(),
            },
        );

        // Shake animation
        self.presets.insert(
            "shake".to_string(),
            AnimationPreset {
                name: "shake".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![("transform".to_string(), "translateX(0)".to_string())],
                    },
                    Keyframe {
                        percent: 25,
                        properties: vec![("transform".to_string(), "translateX(-5px)".to_string())],
                    },
                    Keyframe {
                        percent: 50,
                        properties: vec![("transform".to_string(), "translateX(5px)".to_string())],
                    },
                    Keyframe {
                        percent: 75,
                        properties: vec![("transform".to_string(), "translateX(-5px)".to_string())],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![("transform".to_string(), "translateX(0)".to_string())],
                    },
                ],
                default_duration: "500ms".to_string(),
                default_easing: "ease-in-out".to_string(),
            },
        );

        // Ping animation (like Tailwind's ping)
        self.presets.insert(
            "ping".to_string(),
            AnimationPreset {
                name: "ping".to_string(),
                keyframes: vec![
                    Keyframe {
                        percent: 0,
                        properties: vec![
                            ("transform".to_string(), "scale(1)".to_string()),
                            ("opacity".to_string(), "1".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 75,
                        properties: vec![
                            ("transform".to_string(), "scale(2)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                    Keyframe {
                        percent: 100,
                        properties: vec![
                            ("transform".to_string(), "scale(2)".to_string()),
                            ("opacity".to_string(), "0".to_string()),
                        ],
                    },
                ],
                default_duration: "1000ms".to_string(),
                default_easing: "cubic-bezier(0, 0, 0.2, 1)".to_string(),
            },
        );
    }

    /// Parse an animation class name
    ///
    /// **Validates: Requirements 4.1**
    pub fn parse(&self, class: &str) -> Option<AnimationClass> {
        if !class.starts_with("animate-") {
            return None;
        }

        let rest = &class[8..]; // Skip "animate-"
        let mut anim = AnimationClass::default();

        // Handle composition (animate-fade-in+slide-up)
        if rest.contains('+') {
            anim.composed = rest.split('+').map(String::from).collect();
            // Use first animation's name for the combined animation
            if let Some(first) = anim.composed.first() {
                anim.name = format!("composed-{}", first);
            }
            return Some(anim);
        }

        // Parse parts separated by dashes
        self.parse_animation_parts(&mut anim, rest);

        Some(anim)
    }

    /// Parse animation parts from the class name
    ///
    /// **Validates: Requirements 4.3, 4.4, 4.5, 4.6**
    fn parse_animation_parts(&self, anim: &mut AnimationClass, input: &str) {
        // Try to match known preset names first
        for preset_name in self.presets.keys() {
            if input.starts_with(preset_name) {
                anim.name = preset_name.clone();
                let rest = &input[preset_name.len()..];
                if !rest.is_empty() && rest.starts_with('-') {
                    self.parse_modifiers(anim, &rest[1..]);
                }
                return;
            }
        }

        // Check custom keyframes
        for custom_name in self.custom_keyframes.keys() {
            if input.starts_with(custom_name) {
                anim.name = custom_name.clone();
                let rest = &input[custom_name.len()..];
                if !rest.is_empty() && rest.starts_with('-') {
                    self.parse_modifiers(anim, &rest[1..]);
                }
                return;
            }
        }

        // Fallback: use the whole input as name
        anim.name = input.to_string();
    }

    /// Parse modifier parts (duration, delay, easing, etc.)
    fn parse_modifiers(&self, anim: &mut AnimationClass, modifiers: &str) {
        // We need to handle multi-word modifiers like "ease-in-out" and "alternate-reverse"
        let mut remaining = modifiers;

        while !remaining.is_empty() {
            let mut matched = false;

            // Try multi-word easing functions first
            for easing in &["ease-in-out", "ease-out", "ease-in"] {
                if remaining.starts_with(easing) {
                    anim.easing = Some(easing.to_string());
                    remaining = remaining[easing.len()..].trim_start_matches('-');
                    matched = true;
                    break;
                }
            }
            if matched {
                continue;
            }

            // Try multi-word directions
            if remaining.starts_with("alternate-reverse") {
                anim.direction = Some("alternate-reverse".to_string());
                remaining = remaining[17..].trim_start_matches('-');
                continue;
            }

            // Try delay prefix
            if remaining.starts_with("delay") {
                let rest = &remaining[5..];
                // Find the end of the delay value
                let end = rest
                    .find(|c: char| {
                        c == '-'
                            && !rest[..rest.find(c).unwrap_or(0)]
                                .ends_with(|c: char| c.is_ascii_digit())
                    })
                    .unwrap_or(rest.len());
                let delay_val = &rest[..end].trim_start_matches('-');
                if !delay_val.is_empty() {
                    anim.delay = Some(Self::normalize_duration(delay_val));
                }
                remaining = if end < rest.len() {
                    &rest[end + 1..]
                } else {
                    ""
                };
                continue;
            }

            // Find next part (up to next dash or end)
            let end = remaining.find('-').unwrap_or(remaining.len());
            let part = &remaining[..end];
            remaining = if end < remaining.len() {
                &remaining[end + 1..]
            } else {
                ""
            };

            if part.is_empty() {
                continue;
            }

            // Duration (e.g., "500ms", "1s", "2000")
            if Self::is_duration(part) {
                anim.duration = Some(Self::normalize_duration(part));
                continue;
            }

            // Simple easing functions
            if matches!(part, "linear" | "ease") {
                anim.easing = Some(part.to_string());
                continue;
            }

            // Iteration count
            if part == "infinite" || part.parse::<u32>().is_ok() {
                anim.iteration = Some(part.to_string());
                continue;
            }

            // Direction
            if matches!(part, "normal" | "reverse" | "alternate") {
                anim.direction = Some(part.to_string());
                continue;
            }

            // Fill mode
            if matches!(part, "forwards" | "backwards" | "both" | "none") {
                anim.fill_mode = Some(part.to_string());
                continue;
            }
        }
    }

    /// Check if a string is a duration value
    fn is_duration(s: &str) -> bool {
        s.ends_with("ms") || s.ends_with('s') || s.parse::<u32>().is_ok()
    }

    /// Normalize duration to include unit
    fn normalize_duration(s: &str) -> String {
        if s.ends_with("ms") || s.ends_with('s') {
            s.to_string()
        } else if let Ok(n) = s.parse::<u32>() {
            format!("{}ms", n)
        } else {
            s.to_string()
        }
    }

    /// Check if a string is an easing function
    #[allow(dead_code)]
    fn is_easing(s: &str) -> bool {
        matches!(
            s,
            "linear" | "ease" | "ease-in" | "ease-out" | "ease-in-out" | "step-start" | "step-end"
        ) || s.starts_with("cubic-bezier")
            || s.starts_with("steps")
    }

    /// Get keyframes for an animation (handles composition)
    ///
    /// **Validates: Requirements 4.2, 4.8**
    pub fn get_keyframes(&self, anim: &AnimationClass) -> Vec<Keyframe> {
        if !anim.composed.is_empty() {
            return self.merge_keyframes(&anim.composed);
        }

        // Try presets first
        if let Some(preset) = self.presets.get(&anim.name) {
            return preset.keyframes.clone();
        }

        // Try custom keyframes
        if let Some(custom) = self.custom_keyframes.get(&anim.name) {
            return custom.clone();
        }

        Vec::new()
    }

    /// Merge keyframes from multiple animations
    ///
    /// **Validates: Requirements 4.8**
    fn merge_keyframes(&self, names: &[String]) -> Vec<Keyframe> {
        let mut merged: AHashMap<u8, Vec<(String, String)>> = AHashMap::new();

        for name in names {
            let keyframes = if let Some(preset) = self.presets.get(name) {
                &preset.keyframes
            } else if let Some(custom) = self.custom_keyframes.get(name) {
                custom
            } else {
                continue;
            };

            for kf in keyframes {
                let entry = merged.entry(kf.percent).or_default();
                for prop in &kf.properties {
                    // Only add if not already present (first animation wins for conflicts)
                    if !entry.iter().any(|(p, _)| p == &prop.0) {
                        entry.push(prop.clone());
                    }
                }
            }
        }

        let mut result: Vec<Keyframe> = merged
            .into_iter()
            .map(|(percent, properties)| Keyframe {
                percent,
                properties,
            })
            .collect();

        result.sort_by_key(|kf| kf.percent);
        result
    }

    /// Generate CSS for an animation class
    ///
    /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**
    pub fn generate_css(&self, anim: &AnimationClass) -> String {
        let mut output = String::new();

        // Get keyframes
        let keyframes = self.get_keyframes(anim);
        if keyframes.is_empty() {
            return output;
        }

        // Generate unique animation name
        let anim_name = if !anim.composed.is_empty() {
            format!("dx-{}", anim.composed.join("-"))
        } else {
            format!("dx-{}", anim.name)
        };

        // Generate @keyframes
        output.push_str(&format!("@keyframes {} {{\n", anim_name));
        for kf in &keyframes {
            output.push_str(&format!("  {}% {{\n", kf.percent));
            for (prop, val) in &kf.properties {
                output.push_str(&format!("    {}: {};\n", prop, val));
            }
            output.push_str("  }\n");
        }
        output.push_str("}\n\n");

        // Get defaults from preset if available
        let (default_duration, default_easing) = if let Some(preset) = self.presets.get(&anim.name)
        {
            (preset.default_duration.clone(), preset.default_easing.clone())
        } else {
            ("300ms".to_string(), "ease-out".to_string())
        };

        // Generate animation property
        let duration = anim.duration.as_deref().unwrap_or(&default_duration);
        let easing = anim.easing.as_deref().unwrap_or(&default_easing);
        let delay = anim.delay.as_deref().unwrap_or("0ms");
        let iteration = anim.iteration.as_deref().unwrap_or("1");
        let direction = anim.direction.as_deref().unwrap_or("normal");
        let fill_mode = anim.fill_mode.as_deref().unwrap_or("none");

        output.push_str(&format!(
            "animation: {} {} {} {} {} {} {};\n",
            anim_name, duration, easing, delay, iteration, direction, fill_mode
        ));

        output
    }

    /// Add custom keyframes
    ///
    /// **Validates: Requirements 4.9**
    pub fn add_custom_keyframes(&mut self, name: &str, keyframes: Vec<Keyframe>) {
        self.custom_keyframes.insert(name.to_string(), keyframes);
    }

    /// Get preset by name
    pub fn get_preset(&self, name: &str) -> Option<&AnimationPreset> {
        self.presets.get(name)
    }

    /// List all available preset names
    pub fn preset_names(&self) -> Vec<&str> {
        self.presets.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for AnimationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_animation() {
        let engine = AnimationEngine::new();
        let result = engine.parse("animate-fade-in");
        assert!(result.is_some());
        let anim = result.unwrap();
        assert_eq!(anim.name, "fade-in");
    }

    #[test]
    fn test_parse_animation_with_duration() {
        let engine = AnimationEngine::new();
        let result = engine.parse("animate-fade-in-500ms");
        assert!(result.is_some());
        let anim = result.unwrap();
        assert_eq!(anim.name, "fade-in");
        assert_eq!(anim.duration, Some("500ms".to_string()));
    }

    #[test]
    fn test_parse_animation_with_easing() {
        let engine = AnimationEngine::new();
        let result = engine.parse("animate-slide-up-ease-in-out");
        assert!(result.is_some());
        let anim = result.unwrap();
        assert_eq!(anim.name, "slide-up");
        assert_eq!(anim.easing, Some("ease-in-out".to_string()));
    }

    #[test]
    fn test_parse_composed_animation() {
        let engine = AnimationEngine::new();
        let result = engine.parse("animate-fade-in+slide-up");
        assert!(result.is_some());
        let anim = result.unwrap();
        assert_eq!(anim.composed.len(), 2);
        assert!(anim.composed.contains(&"fade-in".to_string()));
        assert!(anim.composed.contains(&"slide-up".to_string()));
    }

    #[test]
    fn test_parse_animation_with_iteration() {
        let engine = AnimationEngine::new();
        let result = engine.parse("animate-spin-infinite");
        assert!(result.is_some());
        let anim = result.unwrap();
        assert_eq!(anim.name, "spin");
        assert_eq!(anim.iteration, Some("infinite".to_string()));
    }

    #[test]
    fn test_parse_animation_with_direction() {
        let engine = AnimationEngine::new();
        let result = engine.parse("animate-bounce-alternate");
        assert!(result.is_some());
        let anim = result.unwrap();
        assert_eq!(anim.name, "bounce");
        assert_eq!(anim.direction, Some("alternate".to_string()));
    }

    #[test]
    fn test_get_keyframes() {
        let engine = AnimationEngine::new();
        let anim = AnimationClass {
            name: "fade-in".to_string(),
            ..Default::default()
        };
        let keyframes = engine.get_keyframes(&anim);
        assert_eq!(keyframes.len(), 2);
        assert_eq!(keyframes[0].percent, 0);
        assert_eq!(keyframes[1].percent, 100);
    }

    #[test]
    fn test_merge_keyframes() {
        let engine = AnimationEngine::new();
        let anim = AnimationClass {
            composed: vec!["fade-in".to_string(), "slide-up".to_string()],
            ..Default::default()
        };
        let keyframes = engine.get_keyframes(&anim);

        // Should have merged keyframes
        assert!(!keyframes.is_empty());

        // Check that both opacity and transform are present
        let has_opacity =
            keyframes.iter().any(|kf| kf.properties.iter().any(|(p, _)| p == "opacity"));
        let has_transform =
            keyframes.iter().any(|kf| kf.properties.iter().any(|(p, _)| p == "transform"));
        assert!(has_opacity);
        assert!(has_transform);
    }

    #[test]
    fn test_generate_css() {
        let engine = AnimationEngine::new();
        let anim = AnimationClass {
            name: "fade-in".to_string(),
            duration: Some("500ms".to_string()),
            ..Default::default()
        };
        let css = engine.generate_css(&anim);

        assert!(css.contains("@keyframes dx-fade-in"));
        assert!(css.contains("opacity"));
        assert!(css.contains("500ms"));
    }

    #[test]
    fn test_generate_css_composed() {
        let engine = AnimationEngine::new();
        let anim = AnimationClass {
            name: "composed-fade-in".to_string(),
            composed: vec!["fade-in".to_string(), "slide-up".to_string()],
            ..Default::default()
        };
        let css = engine.generate_css(&anim);

        assert!(css.contains("@keyframes dx-fade-in-slide-up"));
        assert!(css.contains("opacity"));
        assert!(css.contains("transform"));
    }

    #[test]
    fn test_custom_keyframes() {
        let mut engine = AnimationEngine::new();
        engine.add_custom_keyframes(
            "my-anim",
            vec![
                Keyframe {
                    percent: 0,
                    properties: vec![("color".to_string(), "red".to_string())],
                },
                Keyframe {
                    percent: 100,
                    properties: vec![("color".to_string(), "blue".to_string())],
                },
            ],
        );

        let anim = AnimationClass {
            name: "my-anim".to_string(),
            ..Default::default()
        };
        let keyframes = engine.get_keyframes(&anim);
        assert_eq!(keyframes.len(), 2);
    }

    #[test]
    fn test_preset_names() {
        let engine = AnimationEngine::new();
        let names = engine.preset_names();
        assert!(names.contains(&"fade-in"));
        assert!(names.contains(&"slide-up"));
        assert!(names.contains(&"bounce"));
        assert!(names.contains(&"spin"));
    }

    #[test]
    fn test_not_animation_class() {
        let engine = AnimationEngine::new();
        assert!(engine.parse("not-animate").is_none());
        assert!(engine.parse("bg-red").is_none());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_preset_name() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "fade-in",
            "fade-out",
            "slide-up",
            "slide-down",
            "slide-left",
            "slide-right",
            "scale-in",
            "scale-out",
            "bounce",
            "pulse",
            "spin",
            "shake",
            "ping",
        ])
        .prop_map(|s| s.to_string())
    }

    fn arb_duration() -> impl Strategy<Value = String> {
        prop_oneof![
            (100u32..2000u32).prop_map(|n| format!("{}ms", n)),
            (1u32..5u32).prop_map(|n| format!("{}s", n)),
        ]
    }

    fn arb_easing() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["linear", "ease", "ease-in", "ease-out", "ease-in-out"])
            .prop_map(|s| s.to_string())
    }

    fn arb_iteration() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("infinite".to_string()),
            (1u32..10u32).prop_map(|n| n.to_string()),
        ]
    }

    fn arb_direction() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["normal", "reverse", "alternate", "alternate-reverse"])
            .prop_map(|s| s.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-advanced-features, Property 7: Animation Generation Correctness
        /// *For any* animation class (e.g., "animate-fade-in-500ms-ease-out"), the generated CSS
        /// SHALL contain valid @keyframes and animation property with the specified parameters.
        /// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**
        #[test]
        fn prop_animation_generation_correctness(
            preset in arb_preset_name(),
            duration in arb_duration(),
            easing in arb_easing()
        ) {
            let engine = AnimationEngine::new();
            let class_name = format!("animate-{}-{}-{}", preset, duration, easing);

            let result = engine.parse(&class_name);
            prop_assert!(
                result.is_some(),
                "Should parse animation class: {}",
                class_name
            );

            let anim = result.unwrap();
            let css = engine.generate_css(&anim);

            // Should contain @keyframes
            prop_assert!(
                css.contains("@keyframes"),
                "Generated CSS should contain @keyframes: {}",
                css
            );

            // Should contain animation property
            prop_assert!(
                css.contains("animation:"),
                "Generated CSS should contain animation property: {}",
                css
            );

            // Should contain the duration
            prop_assert!(
                css.contains(&duration),
                "Generated CSS should contain duration '{}': {}",
                duration, css
            );

            // Should contain the easing
            prop_assert!(
                css.contains(&easing),
                "Generated CSS should contain easing '{}': {}",
                easing, css
            );
        }

        /// Property test for animation with iteration and direction
        /// **Validates: Requirements 4.5, 4.6**
        #[test]
        fn prop_animation_iteration_direction(
            preset in arb_preset_name(),
            iteration in arb_iteration(),
            direction in arb_direction()
        ) {
            let engine = AnimationEngine::new();
            let class_name = format!("animate-{}-{}-{}", preset, iteration, direction);

            let result = engine.parse(&class_name);
            prop_assert!(result.is_some());

            let anim = result.unwrap();
            let css = engine.generate_css(&anim);

            // Should contain iteration count
            prop_assert!(
                css.contains(&iteration),
                "Generated CSS should contain iteration '{}': {}",
                iteration, css
            );

            // Should contain direction
            prop_assert!(
                css.contains(&direction),
                "Generated CSS should contain direction '{}': {}",
                direction, css
            );
        }

        /// Feature: dx-style-advanced-features, Property 8: Animation Composition Merging
        /// *For any* composed animation class (e.g., "animate-fade-in+slide-up"), the Animation_Engine
        /// SHALL merge all component animations into a single animation property.
        /// **Validates: Requirements 4.8**
        #[test]
        fn prop_animation_composition_merging(
            preset1 in arb_preset_name(),
            preset2 in arb_preset_name()
        ) {
            let engine = AnimationEngine::new();
            let class_name = format!("animate-{}+{}", preset1, preset2);

            let result = engine.parse(&class_name);
            prop_assert!(
                result.is_some(),
                "Should parse composed animation: {}",
                class_name
            );

            let anim = result.unwrap();

            // Should have composed animations
            prop_assert!(
                anim.composed.len() >= 2,
                "Should have at least 2 composed animations"
            );

            // Get merged keyframes
            let keyframes = engine.get_keyframes(&anim);

            // Should have keyframes
            prop_assert!(
                !keyframes.is_empty(),
                "Merged animation should have keyframes"
            );

            // Generate CSS
            let css = engine.generate_css(&anim);

            // Should have single @keyframes block
            let keyframes_count = css.matches("@keyframes").count();
            prop_assert_eq!(
                keyframes_count, 1,
                "Should have exactly one @keyframes block, got {}: {}",
                keyframes_count, css
            );

            // Should have single animation property
            let animation_count = css.matches("animation:").count();
            prop_assert_eq!(
                animation_count, 1,
                "Should have exactly one animation property, got {}: {}",
                animation_count, css
            );
        }

        /// Property test for keyframe percentage ordering
        #[test]
        fn prop_keyframe_ordering(
            preset in arb_preset_name()
        ) {
            let engine = AnimationEngine::new();
            let anim = AnimationClass {
                name: preset.clone(),
                ..Default::default()
            };

            let keyframes = engine.get_keyframes(&anim);

            // Keyframes should be sorted by percentage
            for i in 1..keyframes.len() {
                prop_assert!(
                    keyframes[i].percent >= keyframes[i-1].percent,
                    "Keyframes should be sorted by percentage"
                );
            }
        }

        /// Property test for valid CSS output
        #[test]
        fn prop_valid_css_output(
            preset in arb_preset_name()
        ) {
            let engine = AnimationEngine::new();
            let anim = AnimationClass {
                name: preset.clone(),
                ..Default::default()
            };

            let css = engine.generate_css(&anim);

            // Should have balanced braces
            let open_braces = css.chars().filter(|&c| c == '{').count();
            let close_braces = css.chars().filter(|&c| c == '}').count();
            prop_assert_eq!(
                open_braces, close_braces,
                "CSS should have balanced braces: {}",
                css
            );

            // Should not have empty keyframes
            prop_assert!(
                !css.contains("{}"),
                "CSS should not have empty blocks: {}",
                css
            );
        }
    }
}
