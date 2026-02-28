//! Binary encoder for .drv format

use super::{
    SectionType,
    schema::{
        ContextSection, DrvHeader, PersonaSection, RuleEntry, StandardsSection, WorkflowSection,
    },
};
use crate::{DrivenError, Result, parser::UnifiedRule};
use bytes::{BufMut, BytesMut};
use std::collections::HashMap;

/// Encoder for creating .drv binary files
#[derive(Debug)]
pub struct DrvEncoder {
    /// String deduplication table
    string_table: Vec<String>,
    /// String to index mapping
    string_map: HashMap<String, u32>,
    /// Output buffer
    buffer: BytesMut,
}

impl DrvEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        Self {
            string_table: Vec::new(),
            string_map: HashMap::new(),
            buffer: BytesMut::with_capacity(4096),
        }
    }

    /// Intern a string and return its index
    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.string_map.get(s) {
            return idx;
        }
        let idx = self.string_table.len() as u32;
        self.string_table.push(s.to_string());
        self.string_map.insert(s.to_string(), idx);
        idx
    }

    /// Encode rules to binary format
    pub fn encode(&self, rules: &[UnifiedRule]) -> Result<Vec<u8>> {
        let mut encoder = DrvEncoder::new();
        encoder.encode_rules(rules)
    }

    /// Internal encoding implementation
    fn encode_rules(&mut self, rules: &[UnifiedRule]) -> Result<Vec<u8>> {
        // First pass: build string table and sections
        let mut persona: Option<PersonaSection> = None;
        let mut standards = StandardsSection::default();
        let mut context = ContextSection::default();
        let mut workflow: Option<WorkflowSection> = None;

        for rule in rules {
            match rule {
                UnifiedRule::Persona {
                    name,
                    role,
                    identity,
                    style,
                    traits,
                    principles,
                } => {
                    persona = Some(PersonaSection {
                        name_idx: self.intern(name),
                        role_idx: self.intern(role),
                        identity_idx: identity.as_ref().map(|s| self.intern(s)).unwrap_or(0),
                        style_idx: style.as_ref().map(|s| self.intern(s)).unwrap_or(0),
                        traits: traits.iter().map(|t| self.intern(t)).collect(),
                        principles: principles.iter().map(|p| self.intern(p)).collect(),
                    });
                }
                UnifiedRule::Standard {
                    category,
                    priority,
                    description,
                    pattern,
                } => {
                    standards.rules.push(RuleEntry {
                        category: *category,
                        priority: *priority,
                        description_idx: self.intern(description),
                        pattern_idx: pattern.as_ref().map(|p| self.intern(p)).unwrap_or(0),
                    });
                }
                UnifiedRule::Context {
                    includes,
                    excludes,
                    focus,
                } => {
                    context.include_patterns = includes.iter().map(|s| self.intern(s)).collect();
                    context.exclude_patterns = excludes.iter().map(|s| self.intern(s)).collect();
                    context.focus_areas = focus.iter().map(|s| self.intern(s)).collect();
                }
                UnifiedRule::Workflow { name, steps } => {
                    let mut ws = WorkflowSection {
                        name_idx: self.intern(name),
                        steps: Vec::new(),
                    };
                    for step in steps {
                        ws.steps.push(super::schema::WorkflowStep {
                            name_idx: self.intern(&step.name),
                            description_idx: self.intern(&step.description),
                            condition_idx: step
                                .condition
                                .as_ref()
                                .map(|c| self.intern(c))
                                .unwrap_or(0),
                            actions: step.actions.iter().map(|a| self.intern(a)).collect(),
                        });
                    }
                    workflow = Some(ws);
                }
                UnifiedRule::Raw { content } => {
                    // Store raw content as a standard rule
                    standards.rules.push(RuleEntry {
                        category: super::schema::RuleCategory::Other,
                        priority: 100,
                        description_idx: self.intern(content),
                        pattern_idx: 0,
                    });
                }
            }
        }

        // Count sections
        let mut section_count = 1; // String table always present
        if persona.is_some() {
            section_count += 1;
        }
        if !standards.rules.is_empty() {
            section_count += 1;
        }
        if !context.include_patterns.is_empty() || !context.exclude_patterns.is_empty() {
            section_count += 1;
        }
        if workflow.is_some() {
            section_count += 1;
        }

        // Write header (will update checksum at end)
        let header = DrvHeader::new(section_count);
        self.buffer.put_slice(bytemuck::bytes_of(&header));

        // Write string table
        self.write_string_table()?;

        // Write sections
        if let Some(ref p) = persona {
            self.write_persona_section(p)?;
        }
        if !standards.rules.is_empty() {
            self.write_standards_section(&standards)?;
        }
        if !context.include_patterns.is_empty() || !context.exclude_patterns.is_empty() {
            self.write_context_section(&context)?;
        }
        if let Some(ref w) = workflow {
            self.write_workflow_section(w)?;
        }

        // Calculate and update checksum
        let checksum = self.calculate_checksum();
        let header_bytes = &mut self.buffer[..16];
        header_bytes[12..16].copy_from_slice(&checksum.to_le_bytes());

        Ok(self.buffer.to_vec())
    }

    fn write_string_table(&mut self) -> Result<()> {
        self.buffer.put_u8(SectionType::StringTable as u8);
        self.buffer.put_u32_le(self.string_table.len() as u32);

        for s in &self.string_table {
            let bytes = s.as_bytes();
            if bytes.len() > u16::MAX as usize {
                return Err(DrivenError::Format(format!("String too long: {} bytes", bytes.len())));
            }
            self.buffer.put_u16_le(bytes.len() as u16);
            self.buffer.put_slice(bytes);
        }

        Ok(())
    }

    fn write_persona_section(&mut self, persona: &PersonaSection) -> Result<()> {
        self.buffer.put_u8(SectionType::Persona as u8);
        self.buffer.put_u32_le(persona.name_idx);
        self.buffer.put_u32_le(persona.role_idx);
        self.buffer.put_u32_le(persona.identity_idx);
        self.buffer.put_u32_le(persona.style_idx);

        self.buffer.put_u16_le(persona.traits.len() as u16);
        for &idx in &persona.traits {
            self.buffer.put_u32_le(idx);
        }

        self.buffer.put_u16_le(persona.principles.len() as u16);
        for &idx in &persona.principles {
            self.buffer.put_u32_le(idx);
        }

        Ok(())
    }

    fn write_standards_section(&mut self, standards: &StandardsSection) -> Result<()> {
        self.buffer.put_u8(SectionType::Standards as u8);
        self.buffer.put_u32_le(standards.rules.len() as u32);

        for rule in &standards.rules {
            self.buffer.put_u8(rule.category as u8);
            self.buffer.put_u8(rule.priority);
            self.buffer.put_u32_le(rule.description_idx);
            self.buffer.put_u32_le(rule.pattern_idx);
        }

        Ok(())
    }

    fn write_context_section(&mut self, context: &ContextSection) -> Result<()> {
        self.buffer.put_u8(SectionType::Context as u8);

        self.buffer.put_u16_le(context.include_patterns.len() as u16);
        for &idx in &context.include_patterns {
            self.buffer.put_u32_le(idx);
        }

        self.buffer.put_u16_le(context.exclude_patterns.len() as u16);
        for &idx in &context.exclude_patterns {
            self.buffer.put_u32_le(idx);
        }

        self.buffer.put_u16_le(context.focus_areas.len() as u16);
        for &idx in &context.focus_areas {
            self.buffer.put_u32_le(idx);
        }

        self.buffer.put_u16_le(context.dependencies.len() as u16);
        for &(name, version) in &context.dependencies {
            self.buffer.put_u32_le(name);
            self.buffer.put_u32_le(version);
        }

        Ok(())
    }

    fn write_workflow_section(&mut self, workflow: &WorkflowSection) -> Result<()> {
        self.buffer.put_u8(SectionType::Workflow as u8);
        self.buffer.put_u32_le(workflow.name_idx);
        self.buffer.put_u16_le(workflow.steps.len() as u16);

        for step in &workflow.steps {
            self.buffer.put_u32_le(step.name_idx);
            self.buffer.put_u32_le(step.description_idx);
            self.buffer.put_u32_le(step.condition_idx);
            self.buffer.put_u16_le(step.actions.len() as u16);
            for &idx in &step.actions {
                self.buffer.put_u32_le(idx);
            }
        }

        Ok(())
    }

    fn calculate_checksum(&self) -> u32 {
        let hash = blake3::hash(&self.buffer[16..]);
        let bytes = hash.as_bytes();
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}

impl Default for DrvEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_new() {
        let encoder = DrvEncoder::new();
        assert!(encoder.string_table.is_empty());
        assert!(encoder.string_map.is_empty());
    }

    #[test]
    fn test_string_interning() {
        let mut encoder = DrvEncoder::new();
        let idx1 = encoder.intern("hello");
        let idx2 = encoder.intern("world");
        let idx3 = encoder.intern("hello"); // duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first
        assert_eq!(encoder.string_table.len(), 2);
    }

    #[test]
    fn test_encode_empty() {
        let encoder = DrvEncoder::new();
        let result = encoder.encode(&[]).unwrap();

        // Should have at least header + string table section
        assert!(result.len() >= 16);
        // Check magic bytes
        assert_eq!(&result[0..4], b"DRV\0");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::format::DrvDecoder;
    use crate::format::schema::RuleCategory;
    use proptest::prelude::*;

    /// Generate arbitrary RuleCategory values
    fn arb_rule_category() -> impl Strategy<Value = RuleCategory> {
        prop_oneof![
            Just(RuleCategory::Style),
            Just(RuleCategory::Naming),
            Just(RuleCategory::ErrorHandling),
            Just(RuleCategory::Testing),
            Just(RuleCategory::Documentation),
            Just(RuleCategory::Security),
            Just(RuleCategory::Performance),
            Just(RuleCategory::Architecture),
            Just(RuleCategory::Imports),
            Just(RuleCategory::Git),
            Just(RuleCategory::Api),
            Just(RuleCategory::Other),
        ]
    }

    /// Generate arbitrary WorkflowStepData
    fn arb_workflow_step() -> impl Strategy<Value = crate::parser::WorkflowStepData> {
        (
            "[a-zA-Z][a-zA-Z0-9_]{0,20}",                              // name
            "[a-zA-Z0-9 .,!?]{1,100}",                                 // description
            proptest::option::of("[a-zA-Z0-9 .,!?]{1,50}"),            // condition
            proptest::collection::vec("[a-zA-Z0-9 .,!?]{1,50}", 0..5), // actions
        )
            .prop_map(|(name, description, condition, actions)| {
                crate::parser::WorkflowStepData {
                    name,
                    description,
                    condition,
                    actions,
                }
            })
    }

    /// Generate arbitrary UnifiedRule values
    fn arb_unified_rule() -> impl Strategy<Value = UnifiedRule> {
        prop_oneof![
            // Persona rule
            (
                "[a-zA-Z][a-zA-Z0-9_]{0,20}",                              // name
                "[a-zA-Z0-9 .,!?]{1,100}",                                 // role
                proptest::option::of("[a-zA-Z0-9 .,!?]{1,100}"),           // identity
                proptest::option::of("[a-zA-Z0-9 .,!?]{1,50}"),            // style
                proptest::collection::vec("[a-zA-Z0-9 .,!?]{1,30}", 0..5), // traits
                proptest::collection::vec("[a-zA-Z0-9 .,!?]{1,50}", 0..5), // principles
            )
                .prop_map(|(name, role, identity, style, traits, principles)| {
                    UnifiedRule::Persona {
                        name,
                        role,
                        identity,
                        style,
                        traits,
                        principles,
                    }
                }),
            // Standard rule
            (
                arb_rule_category(),
                0u8..=255u8,                                        // priority
                "[a-zA-Z0-9 .,!?]{1,200}",                          // description
                proptest::option::of("[a-zA-Z0-9 .,!?*_/]{1,100}"), // pattern
            )
                .prop_map(|(category, priority, description, pattern)| {
                    UnifiedRule::Standard {
                        category,
                        priority,
                        description,
                        pattern,
                    }
                }),
            // Context rule
            (
                proptest::collection::vec("[a-zA-Z0-9*_./]{1,50}", 0..5), // includes
                proptest::collection::vec("[a-zA-Z0-9*_./]{1,50}", 0..5), // excludes
                proptest::collection::vec("[a-zA-Z0-9 ]{1,30}", 0..5),    // focus
            )
                .prop_map(|(includes, excludes, focus)| {
                    UnifiedRule::Context {
                        includes,
                        excludes,
                        focus,
                    }
                }),
            // Workflow rule
            (
                "[a-zA-Z][a-zA-Z0-9_-]{0,30}",                        // name
                proptest::collection::vec(arb_workflow_step(), 0..5), // steps
            )
                .prop_map(|(name, steps)| { UnifiedRule::Workflow { name, steps } }),
            // Raw rule
            "[a-zA-Z0-9 .,!?\n]{1,500}".prop_map(|content| { UnifiedRule::Raw { content } }),
        ]
    }

    /// Helper to compare UnifiedRule values for equality (since it doesn't derive PartialEq)
    fn rules_equal(a: &UnifiedRule, b: &UnifiedRule) -> bool {
        match (a, b) {
            (
                UnifiedRule::Persona {
                    name: n1,
                    role: r1,
                    identity: i1,
                    style: s1,
                    traits: t1,
                    principles: p1,
                },
                UnifiedRule::Persona {
                    name: n2,
                    role: r2,
                    identity: i2,
                    style: s2,
                    traits: t2,
                    principles: p2,
                },
            ) => n1 == n2 && r1 == r2 && i1 == i2 && s1 == s2 && t1 == t2 && p1 == p2,
            (
                UnifiedRule::Standard {
                    category: c1,
                    priority: p1,
                    description: d1,
                    pattern: pt1,
                },
                UnifiedRule::Standard {
                    category: c2,
                    priority: p2,
                    description: d2,
                    pattern: pt2,
                },
            ) => c1 == c2 && p1 == p2 && d1 == d2 && pt1 == pt2,
            (
                UnifiedRule::Context {
                    includes: i1,
                    excludes: e1,
                    focus: f1,
                },
                UnifiedRule::Context {
                    includes: i2,
                    excludes: e2,
                    focus: f2,
                },
            ) => i1 == i2 && e1 == e2 && f1 == f2,
            (
                UnifiedRule::Workflow {
                    name: n1,
                    steps: s1,
                },
                UnifiedRule::Workflow {
                    name: n2,
                    steps: s2,
                },
            ) => {
                if n1 != n2 || s1.len() != s2.len() {
                    return false;
                }
                s1.iter().zip(s2.iter()).all(|(a, b)| {
                    a.name == b.name
                        && a.description == b.description
                        && a.condition == b.condition
                        && a.actions == b.actions
                })
            }
            (UnifiedRule::Raw { content: c1 }, UnifiedRule::Raw { content: c2 }) => c1 == c2,
            _ => false,
        }
    }

    proptest! {
        /// Property 2: DX Machine Format Round-Trip Consistency
        /// *For any* valid binary rule set, encoding to DX Machine format and
        /// decoding back SHALL produce an equivalent rule set.
        /// **Validates: Requirements 1.2**
        ///
        /// Note: The current binary format has these limitations:
        /// - Only one Persona per file (last one wins)
        /// - Only one Workflow per file (last one wins)
        /// - Only one Context per file (last one wins, and only written if non-empty)
        /// - Empty Context (no include/exclude patterns) is not written
        #[test]
        fn prop_drv_binary_roundtrip(rules in proptest::collection::vec(arb_unified_rule(), 0..10)) {
            let encoder = DrvEncoder::new();
            let encoded = encoder.encode(&rules).expect("Encoding should succeed");

            let decoder = DrvDecoder::new(&encoded).expect("Decoding should succeed");
            let decoded = decoder.decode_all().expect("Decode all should succeed");

            // Count expected rules after encoding (accounting for format limitations)
            // - Only one Persona is kept (last one)
            // - Only one Workflow is kept (last one)
            // - Only one Context is kept (last one, and only written if non-empty)
            // - All Standards are kept
            // - Raw rules become Standards
            let has_persona = rules.iter().any(|r| matches!(r, UnifiedRule::Persona { .. }));
            let has_workflow = rules.iter().any(|r| matches!(r, UnifiedRule::Workflow { .. }));

            // The last Context wins, and it's only written if it has patterns
            let last_context = rules.iter().rev().find(|r| matches!(r, UnifiedRule::Context { .. }));
            let has_written_context = if let Some(UnifiedRule::Context { includes, excludes, .. }) = last_context {
                !includes.is_empty() || !excludes.is_empty()
            } else {
                false
            };

            let standard_count = rules.iter().filter(|r| {
                matches!(r, UnifiedRule::Standard { .. } | UnifiedRule::Raw { .. })
            }).count();

            let expected_count = (if has_persona { 1 } else { 0 })
                + (if has_workflow { 1 } else { 0 })
                + (if has_written_context { 1 } else { 0 })
                + standard_count;

            prop_assert_eq!(decoded.len(), expected_count, "Rule count mismatch");

            // Verify the last Persona matches
            if has_persona {
                let last_persona = rules.iter().rev().find(|r| matches!(r, UnifiedRule::Persona { .. }));
                let decoded_persona = decoded.iter().find(|r| matches!(r, UnifiedRule::Persona { .. }));
                if let (Some(orig), Some(dec)) = (last_persona, decoded_persona) {
                    prop_assert!(rules_equal(orig, dec), "Persona mismatch");
                }
            }

            // Verify the last Workflow matches
            if has_workflow {
                let last_workflow = rules.iter().rev().find(|r| matches!(r, UnifiedRule::Workflow { .. }));
                let decoded_workflow = decoded.iter().find(|r| matches!(r, UnifiedRule::Workflow { .. }));
                if let (Some(orig), Some(dec)) = (last_workflow, decoded_workflow) {
                    prop_assert!(rules_equal(orig, dec), "Workflow mismatch");
                }
            }

            // Verify Standards (including converted Raw rules)
            let original_standards: Vec<_> = rules.iter().filter(|r| {
                matches!(r, UnifiedRule::Standard { .. } | UnifiedRule::Raw { .. })
            }).collect();
            let decoded_standards: Vec<_> = decoded.iter().filter(|r| {
                matches!(r, UnifiedRule::Standard { .. })
            }).collect();

            prop_assert_eq!(original_standards.len(), decoded_standards.len(), "Standard count mismatch");
        }

        /// Property test for string interning consistency
        #[test]
        fn prop_string_interning_idempotent(s in "[a-zA-Z0-9 .,!?]{1,100}") {
            let mut encoder = DrvEncoder::new();
            let idx1 = encoder.intern(&s);
            let idx2 = encoder.intern(&s);
            prop_assert_eq!(idx1, idx2, "Same string should return same index");
        }

        /// Property test for encoding determinism
        #[test]
        fn prop_encoding_deterministic(rules in proptest::collection::vec(arb_unified_rule(), 0..5)) {
            let encoder1 = DrvEncoder::new();
            let encoder2 = DrvEncoder::new();

            let encoded1 = encoder1.encode(&rules).expect("First encoding should succeed");
            let encoded2 = encoder2.encode(&rules).expect("Second encoding should succeed");

            prop_assert_eq!(encoded1, encoded2, "Same rules should produce same binary");
        }
    }
}
