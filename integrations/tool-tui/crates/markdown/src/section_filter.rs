//! Section filtering for LLM optimization
//!
//! Allows selective inclusion/exclusion of markdown sections when converting
//! from human format to LLM format.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for which sections to include in LLM format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionFilterConfig {
    /// Sections to exclude by exact header match
    pub exclude_sections: HashSet<String>,

    /// Sections to exclude by pattern (e.g., "Previous Updates")
    pub exclude_patterns: Vec<String>,

    /// Section types to exclude
    pub exclude_types: HashSet<SectionType>,

    /// Whether to preserve all content (no filtering)
    pub preserve_all: bool,
}

impl Default for SectionFilterConfig {
    fn default() -> Self {
        Self {
            exclude_sections: HashSet::new(),
            exclude_patterns: Vec::new(),
            exclude_types: HashSet::new(),
            preserve_all: true, // Safe default: keep everything
        }
    }
}

/// Types of sections that can be filtered
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SectionType {
    /// Changelog/update sections
    Updates,
    /// Contributing guidelines
    Contributing,
    /// Community/social links
    Community,
    /// Acknowledgments/credits
    Acknowledgments,
    /// License information
    License,
    /// Installation instructions
    Installation,
    /// Examples/tutorials
    Examples,
}

impl SectionFilterConfig {
    /// Create a conservative config (removes only obvious redundant sections)
    pub fn conservative() -> Self {
        let mut exclude_types = HashSet::new();
        exclude_types.insert(SectionType::Acknowledgments);

        Self {
            exclude_sections: HashSet::new(),
            exclude_patterns: vec![],
            exclude_types,
            preserve_all: false,
        }
    }

    /// Create an aggressive config (removes all non-technical sections)
    pub fn aggressive() -> Self {
        let mut exclude_types = HashSet::new();
        exclude_types.insert(SectionType::Updates);
        exclude_types.insert(SectionType::Contributing);
        exclude_types.insert(SectionType::Community);
        exclude_types.insert(SectionType::Acknowledgments);

        let exclude_patterns = vec!["Previous Updates".to_string(), "Latest Updates".to_string()];

        Self {
            exclude_sections: HashSet::new(),
            exclude_patterns,
            exclude_types,
            preserve_all: false,
        }
    }

    /// Check if a section should be excluded
    pub fn should_exclude(&self, header: &str) -> bool {
        if self.preserve_all {
            return false;
        }

        // Check exact match
        if self.exclude_sections.contains(header) {
            return true;
        }

        // Check patterns
        for pattern in &self.exclude_patterns {
            if header.contains(pattern) {
                return true;
            }
        }

        // Check section type
        if let Some(section_type) = self.detect_section_type(header)
            && self.exclude_types.contains(&section_type)
        {
            return true;
        }

        false
    }

    /// Detect the type of section from its header
    fn detect_section_type(&self, header: &str) -> Option<SectionType> {
        let lower = header.to_lowercase();

        if lower.contains("update") || lower.contains("changelog") || lower.contains("history") {
            Some(SectionType::Updates)
        } else if lower.contains("contribut") {
            Some(SectionType::Contributing)
        } else if lower.contains("community")
            || lower.contains("support")
            || lower.contains("discord")
        {
            Some(SectionType::Community)
        } else if lower.contains("acknowledgment")
            || lower.contains("credit")
            || lower.contains("thanks")
        {
            Some(SectionType::Acknowledgments)
        } else if lower.contains("license") {
            Some(SectionType::License)
        } else if lower.contains("install")
            || lower.contains("setup")
            || lower.contains("quick start")
        {
            Some(SectionType::Installation)
        } else if lower.contains("example") || lower.contains("tutorial") || lower.contains("guide")
        {
            Some(SectionType::Examples)
        } else {
            None
        }
    }
}

/// Filter markdown content based on section configuration
pub fn filter_sections(content: &str, config: &SectionFilterConfig) -> String {
    if config.preserve_all {
        return content.to_string();
    }

    let mut result = String::with_capacity(content.len());
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    let mut skip_section = false;

    while i < lines.len() {
        let line = lines[i];

        // Check if this is a header
        if line.starts_with("## ") {
            let header = line.trim_start_matches("## ").trim();
            skip_section = config.should_exclude(header);

            if !skip_section {
                result.push_str(line);
                result.push('\n');
            }
        } else if line.starts_with("# ") {
            // Top-level header - never skip
            skip_section = false;
            result.push_str(line);
            result.push('\n');
        } else if !skip_section {
            result.push_str(line);
            result.push('\n');
        }

        i += 1;
    }

    result
}

/// Analyze which sections exist in the content
pub fn analyze_sections(content: &str) -> Vec<SectionInfo> {
    let mut sections = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut current_section: Option<SectionInfo> = None;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## ") {
            // Save previous section
            if let Some(section) = current_section.take() {
                sections.push(section);
            }

            // Start new section
            let header = line.trim_start_matches("## ").trim().to_string();
            let section_type = detect_section_type_standalone(&header);

            current_section = Some(SectionInfo {
                header: header.clone(),
                line_start: i,
                line_end: i,
                section_type,
                estimated_tokens: 0,
            });
        } else if let Some(ref mut section) = current_section {
            section.line_end = i;
        }
    }

    // Save last section
    if let Some(section) = current_section {
        sections.push(section);
    }

    // Estimate tokens for each section
    for section in &mut sections {
        let section_lines = &lines[section.line_start..=section.line_end];
        let section_content = section_lines.join("\n");
        section.estimated_tokens = estimate_tokens(&section_content);
    }

    sections
}

/// Information about a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionInfo {
    pub header: String,
    pub line_start: usize,
    pub line_end: usize,
    pub section_type: Option<SectionType>,
    pub estimated_tokens: usize,
}

fn detect_section_type_standalone(header: &str) -> Option<SectionType> {
    let lower = header.to_lowercase();

    if lower.contains("update") || lower.contains("changelog") || lower.contains("history") {
        Some(SectionType::Updates)
    } else if lower.contains("contribut") {
        Some(SectionType::Contributing)
    } else if lower.contains("community") || lower.contains("support") || lower.contains("discord")
    {
        Some(SectionType::Community)
    } else if lower.contains("acknowledgment")
        || lower.contains("credit")
        || lower.contains("thanks")
    {
        Some(SectionType::Acknowledgments)
    } else if lower.contains("license") {
        Some(SectionType::License)
    } else if lower.contains("install") || lower.contains("setup") || lower.contains("quick start")
    {
        Some(SectionType::Installation)
    } else if lower.contains("example") || lower.contains("tutorial") || lower.contains("guide") {
        Some(SectionType::Examples)
    } else {
        None
    }
}

fn estimate_tokens(text: &str) -> usize {
    // Rough estimate: 4 chars per token
    (text.len() as f64 / 4.0).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preserves_all() {
        let config = SectionFilterConfig::default();
        assert!(config.preserve_all);
        assert!(!config.should_exclude("## Any Section"));
    }

    #[test]
    fn test_conservative_config() {
        let config = SectionFilterConfig::conservative();
        assert!(!config.preserve_all);
        assert!(config.should_exclude("## Acknowledgments"));
        assert!(!config.should_exclude("## Quick Start"));
    }

    #[test]
    fn test_aggressive_config() {
        let config = SectionFilterConfig::aggressive();
        assert!(config.should_exclude("## Previous Updates (Dec 2025)"));
        assert!(config.should_exclude("## Contributing"));
        assert!(config.should_exclude("## Community & Support"));
        assert!(!config.should_exclude("## Quick Start"));
    }

    #[test]
    fn test_filter_sections() {
        let content = "# Title\n\n## Keep This\nContent\n\n## Acknowledgments\nRemove this\n\n## Also Keep\nMore content";
        let config = SectionFilterConfig::conservative();
        let filtered = filter_sections(content, &config);

        assert!(filtered.contains("## Keep This"));
        assert!(filtered.contains("## Also Keep"));
        assert!(!filtered.contains("## Acknowledgments"));
    }

    #[test]
    fn test_analyze_sections() {
        let content = "# Title\n\n## Section 1\nContent\n\n## Section 2\nMore content";
        let sections = analyze_sections(content);

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].header, "Section 1");
        assert_eq!(sections[1].header, "Section 2");
    }
}
