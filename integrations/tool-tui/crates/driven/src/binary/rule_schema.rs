//! Zero-Copy Rule Schema
//!
//! Binary rule structures using bytemuck for zero-copy access.

use bytemuck::{Pod, Zeroable};

pub use super::infinity_format::{RuleFlags, SectionOffsets};

/// Binary rule entry (12 bytes, cache-line friendly)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryRule {
    /// Unique rule ID (like StyleId in B-CSS)
    pub rule_id: u16,
    /// Category (u8 enum)
    pub category: u8,
    /// Priority (0-255)
    pub priority: u8,
    /// Pattern string index
    pub pattern: u32,
    /// Description string index
    pub description: u32,
}

impl BinaryRule {
    /// Create a new binary rule
    pub fn new(rule_id: u16, category: u8, priority: u8, pattern: u32, description: u32) -> Self {
        Self {
            rule_id,
            category,
            priority,
            pattern,
            description,
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Binary workflow step (20 bytes, no padding)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryStep {
    /// Step ID (2 bytes)
    pub step_id: u16,
    /// Number of actions (2 bytes)
    pub action_count: u16,
    /// Name string index (4 bytes)
    pub name: u32,
    /// Description string index (4 bytes)
    pub description: u32,
    /// Condition string index (4 bytes, 0 = no condition)
    pub condition: u32,
    /// Reserved (4 bytes)
    pub _reserved: u32,
}

impl BinaryStep {
    /// Create a new binary step
    pub fn new(step_id: u16, name: u32, description: u32) -> Self {
        Self {
            step_id,
            action_count: 0,
            name,
            description,
            condition: 0,
            _reserved: 0,
        }
    }

    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Binary persona section header
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryPersona {
    /// Name string index
    pub name: u32,
    /// Role string index
    pub role: u32,
    /// Identity string index (0 = none)
    pub identity: u32,
    /// Style string index (0 = none)
    pub style: u32,
    /// Expertise level (0-10)
    pub expertise_level: u8,
    /// Number of traits
    pub trait_count: u8,
    /// Number of principles
    pub principle_count: u8,
    /// Behavior flags
    pub behavior_flags: u8,
}

impl BinaryPersona {
    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Binary standards section header
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryStandardsHeader {
    /// Number of rules
    pub rule_count: u16,
    /// Category table offset (relative)
    pub category_table_offset: u16,
    /// Priority index offset (relative)
    pub priority_index_offset: u32,
}

impl BinaryStandardsHeader {
    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Binary context section
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinaryContext {
    /// Number of include patterns
    pub include_count: u16,
    /// Number of exclude patterns
    pub exclude_count: u16,
    /// Number of focus areas
    pub focus_count: u16,
    /// Reserved for alignment
    pub _reserved: u16,
}

impl BinaryContext {
    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }
}

/// Binary signature (100 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BinarySignature {
    /// Algorithm (1 = Ed25519)
    pub algorithm: u8,
    /// Reserved (3 bytes for alignment)
    pub _reserved: [u8; 3],
    /// Public key (32 bytes)
    pub public_key: [u8; 32],
    /// Signature (64 bytes)
    pub signature: [u8; 64],
}

impl BinarySignature {
    /// Ed25519 algorithm ID
    pub const ED25519: u8 = 1;

    /// Size in bytes
    pub const fn size() -> usize {
        std::mem::size_of::<Self>()
    }

    /// Create an empty signature
    pub fn empty() -> Self {
        Self {
            algorithm: 0,
            _reserved: [0; 3],
            public_key: [0; 32],
            signature: [0; 64],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_rule_size() {
        assert_eq!(BinaryRule::size(), 12);
    }

    #[test]
    fn test_binary_step_size() {
        assert_eq!(BinaryStep::size(), 20);
    }

    #[test]
    fn test_binary_persona_size() {
        assert_eq!(BinaryPersona::size(), 20);
    }

    #[test]
    fn test_binary_signature_size() {
        // 1 + 3 + 32 + 64 = 100, but might be padded
        assert!(BinarySignature::size() >= 100);
    }
}
