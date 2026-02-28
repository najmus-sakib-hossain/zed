//! Red List Configuration for DX Markdown LLM Format
//!
//! User-controlled content filtering with toggles for elements and sections.

use serde::{Deserialize, Serialize};

/// Red List configuration for content filtering
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedListConfig {
    /// Markdown element filters
    pub elements: ElementFilters,

    /// Section filters (by heading name)
    pub sections: SectionFilters,

    /// Preset mode
    pub preset: Option<Preset>,
}

/// Markdown element filters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElementFilters {
    /// Remove all images: ![alt](url)
    pub remove_images: bool,

    /// Remove external links (keep text): [text](url) → text
    pub remove_links: bool,

    /// Remove horizontal rules: ---
    pub remove_horizontal_rules: bool,

    /// Remove blockquotes: > quote
    pub remove_blockquotes: bool,

    /// Remove code blocks: ```lang ... ```
    pub remove_code_blocks: bool,

    /// Remove inline code: `code` → code
    pub remove_inline_code: bool,

    /// Remove emphasis: **bold** → bold, *italic* → italic
    pub remove_emphasis: bool,

    /// Remove strikethrough: ~~text~~ → text
    pub remove_strikethrough: bool,

    /// Remove task list checkboxes: - [x] task → - task
    pub remove_task_lists: bool,

    /// Remove footnotes: `[^1]` and definitions
    pub remove_footnotes: bool,

    /// Remove emojis: :smile: or 😀
    pub remove_emojis: bool,

    /// Remove HTML tags: <div>, <kbd>, etc.
    pub remove_html: bool,

    /// Remove math expressions: $E = mc^2$
    pub remove_math: bool,

    /// Remove Mermaid diagrams: ```mermaid ... ```
    pub remove_mermaid: bool,
}

/// Section filters (by heading name)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SectionFilters {
    /// Remove specific sections by exact heading match
    pub remove_sections: Vec<String>,

    /// Common section patterns to remove
    pub remove_badges: bool,
    pub remove_table_of_contents: bool,
    pub remove_license: bool,
    pub remove_contributing: bool,
    pub remove_changelog: bool,
    pub remove_acknowledgments: bool,
    pub remove_faq: bool,
    pub remove_examples: bool,
    pub remove_troubleshooting: bool,
    pub remove_installation: bool,
    pub remove_previous_updates: bool,
    pub remove_social_links: bool,
    pub remove_footnotes_section: bool,
    pub remove_alerts: bool,
    pub remove_collapsible: bool,
    pub remove_emoji_section: bool,
    pub remove_math_section: bool,
    pub remove_mermaid_section: bool,
    pub remove_ascii_art: bool,
    pub remove_html_section: bool,
    pub remove_yaml_front_matter: bool,
    pub remove_mentions: bool,
    pub remove_geojson: bool,
}

impl SectionFilters {
    /// Get list of section headings to remove based on flags
    pub fn get_sections_to_remove(&self) -> Vec<String> {
        let mut sections = self.remove_sections.clone();

        if self.remove_table_of_contents {
            sections.extend(vec![
                "Table of Contents".to_string(),
                "TOC".to_string(),
                "Contents".to_string(),
            ]);
        }

        if self.remove_license {
            sections.extend(vec![
                "License".to_string(),
                "Licensing".to_string(),
                "Copyright".to_string(),
            ]);
        }

        if self.remove_contributing {
            sections.extend(vec![
                "Contributing".to_string(),
                "Contribution".to_string(),
                "How to Contribute".to_string(),
                "Contributors".to_string(),
            ]);
        }

        if self.remove_changelog {
            sections.extend(vec![
                "Changelog".to_string(),
                "Change Log".to_string(),
                "Release Notes".to_string(),
                "Version History".to_string(),
                "What's New".to_string(),
            ]);
        }

        if self.remove_acknowledgments {
            sections.extend(vec![
                "Acknowledgments".to_string(),
                "Acknowledgements".to_string(),
                "Credits".to_string(),
                "Thanks".to_string(),
                "Special Thanks".to_string(),
            ]);
        }

        if self.remove_faq {
            sections.extend(vec![
                "FAQ".to_string(),
                "Frequently Asked Questions".to_string(),
                "Common Questions".to_string(),
                "Q&A".to_string(),
            ]);
        }

        if self.remove_examples {
            sections.extend(vec![
                "Examples".to_string(),
                "Example".to_string(),
                "Usage Examples".to_string(),
                "Sample Code".to_string(),
                "Demo".to_string(),
            ]);
        }

        if self.remove_troubleshooting {
            sections.extend(vec![
                "Troubleshooting".to_string(),
                "Common Issues".to_string(),
                "Known Issues".to_string(),
                "Debugging".to_string(),
            ]);
        }

        if self.remove_installation {
            sections.extend(vec![
                "Installation".to_string(),
                "Install".to_string(),
                "Getting Started".to_string(),
                "Setup".to_string(),
                "Quick Start".to_string(),
            ]);
        }

        if self.remove_previous_updates {
            sections.extend(vec![
                "Previous Updates".to_string(),
                "Old Versions".to_string(),
                "Archive".to_string(),
                "History".to_string(),
            ]);
        }

        if self.remove_social_links {
            sections.extend(vec![
                "Follow Us".to_string(),
                "Social Media".to_string(),
                "Connect".to_string(),
                "Community".to_string(),
            ]);
        }

        if self.remove_footnotes_section {
            sections.push("Footnotes".to_string());
        }

        if self.remove_alerts {
            sections.push("Alerts (GFM)".to_string());
        }

        if self.remove_collapsible {
            sections.push("Collapsible Sections".to_string());
        }

        if self.remove_emoji_section {
            sections.push("Emoji".to_string());
        }

        if self.remove_math_section {
            sections.push("Mathematical Expressions".to_string());
        }

        if self.remove_mermaid_section {
            sections.push("Mermaid Diagrams".to_string());
        }

        if self.remove_ascii_art {
            sections.push("ASCII Art".to_string());
        }

        if self.remove_html_section {
            sections.push("HTML in Markdown".to_string());
        }

        if self.remove_yaml_front_matter {
            sections.push("YAML Front Matter".to_string());
        }

        if self.remove_mentions {
            sections.push("Mentions and References (GFM)".to_string());
        }

        if self.remove_geojson {
            sections.push("GeoJSON Example".to_string());
        }

        sections
    }
}

/// Preset configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    /// Minimal: Remove everything possible (85% savings)
    Minimal,

    /// Code-Only: Keep code, remove prose (60% savings)
    CodeOnly,

    /// Docs-Only: Keep docs, remove code (40% savings)
    DocsOnly,

    /// API-Only: API reference only (50% savings)
    ApiOnly,
}

impl RedListConfig {
    /// Create config from preset
    pub fn from_preset(preset: Preset) -> Self {
        match preset {
            Preset::Minimal => Self::minimal(),
            Preset::CodeOnly => Self::code_only(),
            Preset::DocsOnly => Self::docs_only(),
            Preset::ApiOnly => Self::api_only(),
        }
    }

    /// Minimal preset: Remove everything possible
    pub fn minimal() -> Self {
        Self {
            elements: ElementFilters {
                remove_images: true,
                remove_links: false, // Keep for reference
                remove_horizontal_rules: true,
                remove_blockquotes: true,
                remove_code_blocks: false, // Keep code
                remove_inline_code: false,
                remove_emphasis: true,
                remove_strikethrough: true,
                remove_task_lists: true,
                remove_footnotes: true,
                remove_emojis: true,
                remove_html: true,
                remove_math: true,
                remove_mermaid: true,
            },
            sections: SectionFilters {
                remove_sections: Vec::new(),
                remove_badges: true,
                remove_table_of_contents: true,
                remove_license: true,
                remove_contributing: true,
                remove_changelog: true,
                remove_acknowledgments: true,
                remove_faq: true,
                remove_examples: true,
                remove_troubleshooting: true,
                remove_installation: true,
                remove_previous_updates: true,
                remove_social_links: true,
                remove_footnotes_section: true,
                remove_alerts: true,
                remove_collapsible: true,
                remove_emoji_section: true,
                remove_math_section: true,
                remove_mermaid_section: true,
                remove_ascii_art: true,
                remove_html_section: true,
                remove_yaml_front_matter: true,
                remove_mentions: true,
                remove_geojson: true,
            },
            preset: Some(Preset::Minimal),
        }
    }

    /// Code-Only preset: Keep code, remove prose
    pub fn code_only() -> Self {
        Self {
            elements: ElementFilters {
                remove_images: true,
                remove_links: false,
                remove_horizontal_rules: true,
                remove_blockquotes: true,
                remove_code_blocks: false, // Keep code
                remove_inline_code: false,
                remove_emphasis: true,
                remove_strikethrough: true,
                remove_task_lists: true,
                remove_footnotes: true,
                remove_emojis: true,
                remove_html: true,
                remove_math: false,    // Keep math
                remove_mermaid: false, // Keep diagrams
            },
            sections: SectionFilters {
                remove_sections: Vec::new(),
                remove_badges: true,
                remove_table_of_contents: true,
                remove_license: true,
                remove_contributing: true,
                remove_changelog: true,
                remove_acknowledgments: true,
                remove_faq: true,
                remove_examples: false, // Keep examples
                remove_troubleshooting: true,
                remove_installation: true,
                remove_previous_updates: true,
                remove_social_links: true,
                remove_footnotes_section: true,
                remove_alerts: true,
                remove_collapsible: false,
                remove_emoji_section: true,
                remove_math_section: false,
                remove_mermaid_section: false,
                remove_ascii_art: false,
                remove_html_section: true,
                remove_yaml_front_matter: true,
                remove_mentions: true,
                remove_geojson: true,
            },
            preset: Some(Preset::CodeOnly),
        }
    }

    /// Docs-Only preset: Keep docs, remove code
    pub fn docs_only() -> Self {
        Self {
            elements: ElementFilters {
                remove_images: true,
                remove_links: false,
                remove_horizontal_rules: true,
                remove_blockquotes: false, // Keep quotes
                remove_code_blocks: true,  // Remove code
                remove_inline_code: true,
                remove_emphasis: false, // Keep emphasis
                remove_strikethrough: true,
                remove_task_lists: true,
                remove_footnotes: false, // Keep footnotes
                remove_emojis: true,
                remove_html: true,
                remove_math: true,
                remove_mermaid: true,
            },
            sections: SectionFilters {
                remove_sections: Vec::new(),
                remove_badges: true,
                remove_table_of_contents: false,
                remove_license: true,
                remove_contributing: true,
                remove_changelog: true,
                remove_acknowledgments: true,
                remove_faq: false,             // Keep FAQ
                remove_examples: true,         // Remove examples
                remove_troubleshooting: false, // Keep troubleshooting
                remove_installation: true,
                remove_previous_updates: true,
                remove_social_links: true,
                remove_footnotes_section: false,
                remove_alerts: false,
                remove_collapsible: false,
                remove_emoji_section: true,
                remove_math_section: true,
                remove_mermaid_section: true,
                remove_ascii_art: true,
                remove_html_section: true,
                remove_yaml_front_matter: true,
                remove_mentions: true,
                remove_geojson: true,
            },
            preset: Some(Preset::DocsOnly),
        }
    }

    /// API-Only preset: API reference only
    pub fn api_only() -> Self {
        Self {
            elements: ElementFilters {
                remove_images: true,
                remove_links: false,
                remove_horizontal_rules: true,
                remove_blockquotes: true,
                remove_code_blocks: false, // Keep code
                remove_inline_code: false,
                remove_emphasis: false, // Keep emphasis
                remove_strikethrough: true,
                remove_task_lists: true,
                remove_footnotes: true,
                remove_emojis: true,
                remove_html: true,
                remove_math: false,
                remove_mermaid: true,
            },
            sections: SectionFilters {
                remove_sections: Vec::new(),
                remove_badges: true,
                remove_table_of_contents: false,
                remove_license: true,
                remove_contributing: true,
                remove_changelog: true,
                remove_acknowledgments: true,
                remove_faq: true,
                remove_examples: true, // Remove examples
                remove_troubleshooting: true,
                remove_installation: true,
                remove_previous_updates: true,
                remove_social_links: true,
                remove_footnotes_section: true,
                remove_alerts: true,
                remove_collapsible: false,
                remove_emoji_section: true,
                remove_math_section: false,
                remove_mermaid_section: true,
                remove_ascii_art: true,
                remove_html_section: true,
                remove_yaml_front_matter: true,
                remove_mentions: true,
                remove_geojson: true,
            },
            preset: Some(Preset::ApiOnly),
        }
    }
}
