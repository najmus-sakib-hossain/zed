//! Format configuration for dx serializer and markdown

use anyhow::Result;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Human,
    Llm,
    Machine,
}

impl SourceFormat {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "llm" => Self::Llm,
            "machine" => Self::Machine,
            _ => Self::Human, // default
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Human => "sr",
            Self::Llm => "llm",
            Self::Machine => "machine",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormatConfig {
    pub serializer_source: SourceFormat,
    pub markdown_source: SourceFormat,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            serializer_source: SourceFormat::Human,
            markdown_source: SourceFormat::Human,
        }
    }
}

impl FormatConfig {
    /// Load format config from dx file in workspace root
    pub fn load() -> Self {
        Self::load_from_path("dx").unwrap_or_default()
    }

    fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let parser = serializer::llm::human_parser::HumanParser::new();
        let doc = parser.parse(&content)?;

        let mut config = Self::default();

        // Look for [formats] section
        if let Some((id, _)) = doc.section_names.iter().find(|(_, name)| name.as_str() == "formats")
        {
            if let Some(section) = doc.sections.get(id) {
                if let Some(row) = section.rows.first() {
                    if let Some(serializer::DxLlmValue::Obj(obj)) = row.first() {
                        if let Some(serializer::DxLlmValue::Str(val)) = obj.get("serializer_source")
                        {
                            config.serializer_source = SourceFormat::from_str(val);
                        }
                        if let Some(serializer::DxLlmValue::Str(val)) = obj.get("markdown_source") {
                            config.markdown_source = SourceFormat::from_str(val);
                        }
                    }
                }
            }
        }

        Ok(config)
    }
}
