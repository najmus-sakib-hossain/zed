//! Binary schema definitions for .drv format

use bytemuck::{Pod, Zeroable};

/// Header for .drv files (16 bytes, aligned)
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct DrvHeader {
    /// Magic bytes: "DRV\0"
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
    /// Feature flags
    pub flags: u16,
    /// Number of sections
    pub section_count: u32,
    /// Blake3 checksum (truncated to 32 bits)
    pub checksum: u32,
}

impl DrvHeader {
    /// Create a new header with default values
    pub fn new(section_count: u32) -> Self {
        Self {
            magic: *b"DRV\0",
            version: super::DRV_VERSION,
            flags: 0,
            section_count,
            checksum: 0,
        }
    }

    /// Validate the header
    pub fn validate(&self) -> crate::Result<()> {
        if &self.magic != b"DRV\0" {
            return Err(crate::DrivenError::InvalidBinary("Invalid magic bytes".to_string()));
        }
        if self.version > super::DRV_VERSION {
            return Err(crate::DrivenError::InvalidBinary(format!(
                "Unsupported version: {} (max supported: {})",
                self.version,
                super::DRV_VERSION
            )));
        }
        Ok(())
    }
}

/// Rule category enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RuleCategory {
    /// General coding style
    Style = 0,
    /// Naming conventions
    Naming = 1,
    /// Error handling patterns
    ErrorHandling = 2,
    /// Testing requirements
    Testing = 3,
    /// Documentation standards
    Documentation = 4,
    /// Security practices
    Security = 5,
    /// Performance guidelines
    Performance = 6,
    /// Architecture patterns
    Architecture = 7,
    /// Import organization
    Imports = 8,
    /// Git conventions
    Git = 9,
    /// API design
    Api = 10,
    /// Other/custom
    Other = 255,
}

impl From<u8> for RuleCategory {
    fn from(value: u8) -> Self {
        match value {
            0 => RuleCategory::Style,
            1 => RuleCategory::Naming,
            2 => RuleCategory::ErrorHandling,
            3 => RuleCategory::Testing,
            4 => RuleCategory::Documentation,
            5 => RuleCategory::Security,
            6 => RuleCategory::Performance,
            7 => RuleCategory::Architecture,
            8 => RuleCategory::Imports,
            9 => RuleCategory::Git,
            10 => RuleCategory::Api,
            _ => RuleCategory::Other,
        }
    }
}

/// A single rule entry in the standards section
#[derive(Debug, Clone)]
pub struct RuleEntry {
    /// Category of this rule
    pub category: RuleCategory,
    /// Priority (0 = highest)
    pub priority: u8,
    /// Description string index
    pub description_idx: u32,
    /// Pattern/example string index (optional, 0 = none)
    pub pattern_idx: u32,
}

/// Persona section defining AI agent behavior
#[derive(Debug, Clone, Default)]
pub struct PersonaSection {
    /// Name string index
    pub name_idx: u32,
    /// Role description string index
    pub role_idx: u32,
    /// Identity/expertise string index
    pub identity_idx: u32,
    /// Communication style string index
    pub style_idx: u32,
    /// Trait string indices
    pub traits: Vec<u32>,
    /// Principle string indices
    pub principles: Vec<u32>,
}

/// Standards section containing coding rules
#[derive(Debug, Clone, Default)]
pub struct StandardsSection {
    /// All rule entries
    pub rules: Vec<RuleEntry>,
}

/// Context section for project-specific settings
#[derive(Debug, Clone, Default)]
pub struct ContextSection {
    /// Include patterns (string indices)
    pub include_patterns: Vec<u32>,
    /// Exclude patterns (string indices)
    pub exclude_patterns: Vec<u32>,
    /// Focus areas (string indices)
    pub focus_areas: Vec<u32>,
    /// Dependencies (name, version pairs as string indices)
    pub dependencies: Vec<(u32, u32)>,
}

/// A single step in a workflow
#[derive(Debug, Clone)]
pub struct WorkflowStep {
    /// Step name string index
    pub name_idx: u32,
    /// Description string index
    pub description_idx: u32,
    /// Condition string index (0 = always)
    pub condition_idx: u32,
    /// Action string indices
    pub actions: Vec<u32>,
}

/// Workflow section defining development processes
#[derive(Debug, Clone, Default)]
pub struct WorkflowSection {
    /// Workflow name string index
    pub name_idx: u32,
    /// Workflow steps
    pub steps: Vec<WorkflowStep>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<DrvHeader>(), 16);
    }

    #[test]
    fn test_header_validation() {
        let valid = DrvHeader::new(3);
        assert!(valid.validate().is_ok());

        let invalid = DrvHeader {
            magic: *b"BAD\0",
            ..valid
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_rule_category_roundtrip() {
        for i in 0..=10 {
            let cat = RuleCategory::from(i);
            assert_eq!(cat as u8, i);
        }
        // Unknown values become Other
        assert_eq!(RuleCategory::from(100), RuleCategory::Other);
    }
}
