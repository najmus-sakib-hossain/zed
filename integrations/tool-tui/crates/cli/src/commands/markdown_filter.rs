//! DX Markdown Filter Command - Interactive content filtering with red list (Revertable)

use anyhow::Result;
use clap::Args;
use console::style;
use dialoguer::{MultiSelect, theme::ColorfulTheme};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use markdown::red_list_config::{ElementFilters, Preset, RedListConfig, SectionFilters};

#[derive(Args)]
pub struct MarkdownFilterArgs {
    /// Input file to filter
    #[arg(value_name = "FILE")]
    pub input: PathBuf,

    /// Output file (defaults to input file with .filtered.md extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Use a preset configuration
    #[arg(short, long, value_enum)]
    pub preset: Option<PresetArg>,

    /// Skip interactive mode and use preset only
    #[arg(short, long)]
    pub yes: bool,

    /// Revert a filtered file back to original
    #[arg(short, long)]
    pub revert: bool,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum PresetArg {
    Minimal,
    CodeOnly,
    DocsOnly,
    ApiOnly,
}

impl From<PresetArg> for Preset {
    fn from(arg: PresetArg) -> Self {
        match arg {
            PresetArg::Minimal => Preset::Minimal,
            PresetArg::CodeOnly => Preset::CodeOnly,
            PresetArg::DocsOnly => Preset::DocsOnly,
            PresetArg::ApiOnly => Preset::ApiOnly,
        }
    }
}

/// Metadata for reverting filtered content
#[derive(Debug, Serialize, Deserialize)]
struct FilterMetadata {
    /// Original file path
    original_file: String,
    /// Filtered file path
    filtered_file: String,
    /// Configuration used for filtering
    config: RedListConfig,
    /// Removed sections with their content
    removed_sections: HashMap<String, String>,
    /// Original content hash for verification
    original_hash: String,
    /// Timestamp
    timestamp: String,
}

impl MarkdownFilterArgs {
    pub async fn execute(self) -> Result<()> {
        if self.revert {
            return self.revert_filter().await;
        }

        // Read input file
        let content = std::fs::read_to_string(&self.input)?;

        // Determine output path
        let output_path = self.output.clone().unwrap_or_else(|| {
            let mut path = self.input.clone();
            let stem = path.file_stem().unwrap().to_string_lossy();
            path.set_file_name(format!("{}.filtered.md", stem));
            path
        });

        // Metadata path (use .sr extension for DX serializer format)
        let metadata_path = {
            let mut path = output_path.clone();
            let stem = path.file_stem().unwrap().to_string_lossy();
            path.set_file_name(format!("{}.meta.sr", stem));
            path
        };

        println!();
        println!("  {} Markdown Content Filter", style("[*]").cyan().bold());
        println!("  {} Input:  {}", style("[>]").dim(), style(self.input.display()).cyan());
        println!("  {} Output: {}", style("[>]").dim(), style(output_path.display()).cyan());
        println!("  {} Meta:   {}", style("[>]").dim(), style(metadata_path.display()).dim());
        println!();

        // Build configuration
        let config = if let (true, Some(preset)) = (self.yes, self.preset) {
            // Use preset without interaction
            RedListConfig::from_preset(preset.into())
        } else {
            // Interactive mode
            build_interactive_config(self.preset)?
        };

        // Apply filters and collect metadata
        let (filtered, metadata) =
            apply_red_list_filters_with_metadata(&content, &config, &self.input, &output_path);

        // Write output
        std::fs::write(&output_path, &filtered)?;

        // Write metadata using DX serializer format
        let metadata_sr = serialize_metadata_to_sr(&metadata)?;
        std::fs::write(&metadata_path, metadata_sr)?;

        // Calculate savings
        let original_len = content.len();
        let filtered_len = filtered.len();
        let savings = ((original_len - filtered_len) as f64 / original_len as f64) * 100.0;

        println!();
        println!("  {} Filtering Complete", style("[✓]").green().bold());
        println!("  {} Original: {} bytes", style("[>]").dim(), style(original_len).yellow());
        println!("  {} Filtered: {} bytes", style("[>]").dim(), style(filtered_len).green());
        println!("  {} Saved:    {:.1}%", style("[>]").dim(), style(savings).cyan().bold());
        println!(
            "  {} Metadata: {} sections removed",
            style("[>]").dim(),
            style(metadata.removed_sections.len()).magenta()
        );
        println!();
        println!(
            "  {} To revert: dx markdown filter {} --revert",
            style("[i]").yellow(),
            style(output_path.display()).cyan()
        );
        println!();

        Ok(())
    }

    async fn revert_filter(&self) -> Result<()> {
        // Determine metadata path from input (use .sr extension)
        let metadata_path = {
            let mut path = self.input.clone();
            let stem = path.file_stem().unwrap().to_string_lossy();
            path.set_file_name(format!("{}.meta.sr", stem));
            path
        };

        if !metadata_path.exists() {
            anyhow::bail!(
                "Metadata file not found: {}. Cannot revert without metadata.",
                metadata_path.display()
            );
        }

        println!();
        println!("  {} Reverting Filtered Content", style("[*]").cyan().bold());
        println!("  {} Filtered: {}", style("[>]").dim(), style(self.input.display()).cyan());
        println!("  {} Meta:     {}", style("[>]").dim(), style(metadata_path.display()).dim());
        println!();

        // Read metadata from DX serializer format
        let metadata_sr = std::fs::read_to_string(&metadata_path)?;
        let metadata = deserialize_metadata_from_sr(&metadata_sr)?;

        // Read filtered content
        let filtered_content = std::fs::read_to_string(&self.input)?;

        // Reconstruct original
        let original = reconstruct_original(&filtered_content, &metadata)?;

        // Determine output path
        let output_path =
            self.output.clone().unwrap_or_else(|| PathBuf::from(&metadata.original_file));

        // Write reconstructed original
        std::fs::write(&output_path, &original)?;

        println!("  {} Revert Complete", style("[✓]").green().bold());
        println!("  {} Restored: {}", style("[>]").dim(), style(output_path.display()).green());
        println!(
            "  {} Sections restored: {}",
            style("[>]").dim(),
            style(metadata.removed_sections.len()).magenta()
        );
        println!();

        Ok(())
    }
}

fn build_interactive_config(preset: Option<PresetArg>) -> Result<RedListConfig> {
    let theme = ColorfulTheme::default();

    // Step 1: Choose preset or custom
    let preset_options = vec![
        "Custom (select individual options)",
        "Minimal (remove everything possible - 85% savings)",
        "Code-Only (keep code, remove prose - 60% savings)",
        "Docs-Only (keep docs, remove code - 40% savings)",
        "API-Only (API reference only - 50% savings)",
    ];

    let preset_choice = if let Some(p) = preset {
        match p {
            PresetArg::Minimal => 1,
            PresetArg::CodeOnly => 2,
            PresetArg::DocsOnly => 3,
            PresetArg::ApiOnly => 4,
        }
    } else {
        dialoguer::Select::with_theme(&theme)
            .with_prompt("Select filtering mode")
            .items(&preset_options)
            .default(0)
            .interact()?
    };

    if preset_choice > 0 {
        // Use preset
        let preset = match preset_choice {
            1 => Preset::Minimal,
            2 => Preset::CodeOnly,
            3 => Preset::DocsOnly,
            4 => Preset::ApiOnly,
            _ => unreachable!(),
        };
        return Ok(RedListConfig::from_preset(preset));
    }

    // Custom configuration
    println!();
    println!("  {} Custom Configuration", style("[*]").cyan().bold());
    println!(
        "  {} Use Space to toggle, 'a' to select all, Enter to confirm",
        style("[i]").yellow()
    );
    println!();

    // Element filters
    let element_items = vec![
        "Remove images",
        "Remove external links",
        "Remove horizontal rules",
        "Remove blockquotes",
        "Remove code blocks",
        "Remove inline code",
        "Remove emphasis (bold/italic)",
        "Remove strikethrough",
        "Remove task list checkboxes",
        "Remove footnotes",
        "Remove emojis",
        "Remove HTML tags",
        "Remove math expressions",
        "Remove Mermaid diagrams",
    ];

    let element_selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select elements to remove")
        .items(&element_items)
        .interact()?;

    // Section filters
    let section_items = vec![
        "Remove badges",
        "Remove table of contents",
        "Remove license section",
        "Remove contributing section",
        "Remove changelog",
        "Remove acknowledgments",
        "Remove FAQ",
        "Remove examples",
        "Remove troubleshooting",
        "Remove installation",
        "Remove previous updates",
        "Remove social links",
        "Remove footnotes section",
        "Remove alerts (GFM)",
        "Remove collapsible sections",
        "Remove emoji section",
        "Remove math section",
        "Remove Mermaid section",
        "Remove ASCII art",
        "Remove HTML section",
        "Remove YAML front matter",
        "Remove mentions",
        "Remove GeoJSON",
    ];

    let section_selections = MultiSelect::with_theme(&theme)
        .with_prompt("Select sections to remove")
        .items(&section_items)
        .interact()?;

    // Build config from selections
    let elements = ElementFilters {
        remove_images: element_selections.contains(&0),
        remove_links: element_selections.contains(&1),
        remove_horizontal_rules: element_selections.contains(&2),
        remove_blockquotes: element_selections.contains(&3),
        remove_code_blocks: element_selections.contains(&4),
        remove_inline_code: element_selections.contains(&5),
        remove_emphasis: element_selections.contains(&6),
        remove_strikethrough: element_selections.contains(&7),
        remove_task_lists: element_selections.contains(&8),
        remove_footnotes: element_selections.contains(&9),
        remove_emojis: element_selections.contains(&10),
        remove_html: element_selections.contains(&11),
        remove_math: element_selections.contains(&12),
        remove_mermaid: element_selections.contains(&13),
    };

    let sections = SectionFilters {
        remove_sections: Vec::new(),
        remove_badges: section_selections.contains(&0),
        remove_table_of_contents: section_selections.contains(&1),
        remove_license: section_selections.contains(&2),
        remove_contributing: section_selections.contains(&3),
        remove_changelog: section_selections.contains(&4),
        remove_acknowledgments: section_selections.contains(&5),
        remove_faq: section_selections.contains(&6),
        remove_examples: section_selections.contains(&7),
        remove_troubleshooting: section_selections.contains(&8),
        remove_installation: section_selections.contains(&9),
        remove_previous_updates: section_selections.contains(&10),
        remove_social_links: section_selections.contains(&11),
        remove_footnotes_section: section_selections.contains(&12),
        remove_alerts: section_selections.contains(&13),
        remove_collapsible: section_selections.contains(&14),
        remove_emoji_section: section_selections.contains(&15),
        remove_math_section: section_selections.contains(&16),
        remove_mermaid_section: section_selections.contains(&17),
        remove_ascii_art: section_selections.contains(&18),
        remove_html_section: section_selections.contains(&19),
        remove_yaml_front_matter: section_selections.contains(&20),
        remove_mentions: section_selections.contains(&21),
        remove_geojson: section_selections.contains(&22),
    };

    Ok(RedListConfig {
        elements,
        sections,
        preset: None,
    })
}

fn apply_red_list_filters_with_metadata(
    content: &str,
    config: &RedListConfig,
    input_path: &Path,
    output_path: &Path,
) -> (String, FilterMetadata) {
    let sections_to_remove = config.sections.get_sections_to_remove();
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut removed_sections = HashMap::new();
    let mut i = 0;

    // Remove sections and store them
    while i < lines.len() {
        let line = &lines[i];

        // Check if this is a heading
        if line.starts_with("## ") || line.starts_with("### ") || line.starts_with("# ") {
            let heading_text = line.trim_start_matches('#').trim();

            // Check if this section should be removed
            if sections_to_remove.iter().any(|s| heading_text.eq_ignore_ascii_case(s)) {
                // Find the next heading at the same or higher level
                let current_level = line.chars().take_while(|&c| c == '#').count();
                let mut j = i + 1;

                while j < lines.len() {
                    let next_line = &lines[j];
                    if next_line.starts_with('#') {
                        let next_level = next_line.chars().take_while(|&c| c == '#').count();
                        if next_level <= current_level {
                            break;
                        }
                    }
                    j += 1;
                }

                // Store removed section
                let section_content = lines[i..j].join("\n");
                removed_sections.insert(format!("section_{}", i), section_content);

                // Remove lines from i to j
                lines.drain(i..j);
                continue;
            }
        }

        i += 1;
    }

    let result = lines.join("\n");

    // NOTE: Element-level filtering removed - not revertable
    // Only section-level filtering is supported for revertability

    // Create metadata
    let metadata = FilterMetadata {
        original_file: input_path.display().to_string(),
        filtered_file: output_path.display().to_string(),
        config: config.clone(),
        removed_sections,
        original_hash: format!("{:x}", md5::compute(content)),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    (result, metadata)
}

fn reconstruct_original(_filtered: &str, metadata: &FilterMetadata) -> Result<String> {
    // Parse the original file to get all sections
    let original_content = std::fs::read_to_string(&metadata.original_file)?;
    let original_lines: Vec<&str> = original_content.lines().collect();

    // Build a map of section indices to their content
    let mut section_map: HashMap<usize, String> = HashMap::new();
    for (key, content) in &metadata.removed_sections {
        if let Some(index_str) = key.strip_prefix("section_")
            && let Ok(index) = index_str.parse::<usize>()
        {
            section_map.insert(index, content.clone());
        }
    }

    // Reconstruct by going through original line by line
    let mut result = String::new();
    let mut i = 0;

    while i < original_lines.len() {
        // Check if this line starts a removed section
        if let Some(section_content) = section_map.get(&i) {
            // Add the removed section
            result.push_str(section_content);
            result.push('\n');

            // Skip past the section in the original (find where it ends)
            let section_line_count = section_content.lines().count();
            i += section_line_count;
        } else {
            // This line was kept in filtered version, add it
            result.push_str(original_lines[i]);
            result.push('\n');
            i += 1;
        }
    }

    Ok(result.trim_end().to_string())
}

/// Serialize metadata to DX serializer format (.sr)
fn serialize_metadata_to_sr(metadata: &FilterMetadata) -> Result<String> {
    // Use a simple key-value format that's more token-efficient than JSON
    let mut output = String::new();
    output.push_str(&format!("original_file:{}\n", metadata.original_file));
    output.push_str(&format!("filtered_file:{}\n", metadata.filtered_file));
    output.push_str(&format!("original_hash:{}\n", metadata.original_hash));
    output.push_str(&format!("timestamp:{}\n", metadata.timestamp));
    output.push_str(&format!("sections_removed:{}\n", metadata.removed_sections.len()));

    // Store removed sections
    for (key, content) in &metadata.removed_sections {
        output.push_str(&format!("\n[{}]\n", key));
        output.push_str(content);
        output.push_str(&format!("\n[/{}]\n", key));
    }

    // Store config as JSON (compact)
    let config_json = serde_json::to_string(&metadata.config)?;
    output.push_str(&format!("\nconfig:{}\n", config_json));

    Ok(output)
}

/// Deserialize metadata from DX serializer format (.sr)
fn deserialize_metadata_from_sr(content: &str) -> Result<FilterMetadata> {
    let mut original_file = String::new();
    let mut filtered_file = String::new();
    let mut original_hash = String::new();
    let mut timestamp = String::new();
    let mut removed_sections = HashMap::new();
    let mut config: Option<RedListConfig> = None;

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("original_file:") {
            original_file = line.strip_prefix("original_file:").unwrap().to_string();
        } else if line.starts_with("filtered_file:") {
            filtered_file = line.strip_prefix("filtered_file:").unwrap().to_string();
        } else if line.starts_with("original_hash:") {
            original_hash = line.strip_prefix("original_hash:").unwrap().to_string();
        } else if line.starts_with("timestamp:") {
            timestamp = line.strip_prefix("timestamp:").unwrap().to_string();
        } else if line.starts_with("[section_") {
            // Parse section
            let key = line.trim_start_matches('[').trim_end_matches(']').to_string();
            let mut section_content = String::new();
            i += 1;

            while i < lines.len() && !lines[i].starts_with(&format!("[/{}]", key)) {
                section_content.push_str(lines[i]);
                section_content.push('\n');
                i += 1;
            }

            removed_sections.insert(key, section_content.trim_end().to_string());
        } else if line.starts_with("config:") {
            let config_json = line.strip_prefix("config:").unwrap();
            config = Some(serde_json::from_str(config_json)?);
        }

        i += 1;
    }

    Ok(FilterMetadata {
        original_file,
        filtered_file,
        config: config.unwrap_or_default(),
        removed_sections,
        original_hash,
        timestamp,
    })
}
