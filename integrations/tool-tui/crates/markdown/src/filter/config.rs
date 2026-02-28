// Configuration for content filtering

/// Master configuration for content filtering
#[derive(Debug, Clone)]
pub struct FilterConfig {
    /// Preset mode (overrides individual settings if set)
    pub preset: Option<Preset>,

    /// Individual category toggles
    pub categories: CategoryToggles,

    /// Context-aware settings
    pub context: ContextSettings,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Preset {
    /// Keep everything (no filtering)
    Full,

    /// Balanced filtering for general use
    #[default]
    Balanced,

    /// Aggressive filtering for minimal tokens
    Minimal,

    /// Code-focused (keep code, remove prose)
    CodeOnly,

    /// Docs-focused (keep explanations, remove examples)
    DocsOnly,

    /// API reference only
    ApiOnly,
}

#[derive(Debug, Clone, Default)]
pub struct CategoryToggles {
    pub badges: bool,
    pub images: bool,
    pub promotional: bool,
    pub decorative: bool,
    pub verbose: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ContextSettings {
    /// Maximum output tokens
    pub max_tokens: Option<usize>,

    /// Target audience
    pub audience: Audience,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Audience {
    /// AI coding agent (aggressive filtering)
    #[default]
    Agent,

    /// Human developer (balanced)
    Developer,

    /// Mixed/unknown
    Mixed,
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            preset: Some(Preset::Balanced),
            categories: CategoryToggles::default(),
            context: ContextSettings::default(),
        }
    }
}
