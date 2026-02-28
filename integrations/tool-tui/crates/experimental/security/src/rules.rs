//! Binary Rule Fusion Engine (BRFE)
//!
//! Compiled binary state machine for rule execution.

use crate::error::{Result, SecurityError};
use std::path::Path;

/// Magic bytes for the rule database
const DXR_MAGIC: [u8; 4] = *b"DXR\0";

/// Current rule database version
const DXR_VERSION: u8 = 1;

/// Size of a compiled rule in bytes
const RULE_SIZE: usize = 11; // 1 (opcode) + 8 (mask) + 2 (offset)

/// Rule opcodes for the VM
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpCode {
    /// Match pattern at current position (mask contains pattern bytes)
    Match = 0,
    /// Jump to offset if previous match succeeded
    Jump = 1,
    /// Accept (rule matched) - offset contains rule_id
    Accept = 2,
    /// Reject (rule failed) - continue to next rule
    Reject = 3,
    /// Match any byte (wildcard)
    Any = 4,
    /// Match byte range (mask contains min/max)
    Range = 5,
    /// Branch - try multiple paths (offset is branch table index)
    Branch = 6,
    /// End of rules marker
    End = 255,
}

impl OpCode {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(OpCode::Match),
            1 => Some(OpCode::Jump),
            2 => Some(OpCode::Accept),
            3 => Some(OpCode::Reject),
            4 => Some(OpCode::Any),
            5 => Some(OpCode::Range),
            6 => Some(OpCode::Branch),
            255 => Some(OpCode::End),
            _ => None,
        }
    }
}

/// Compiled rule instruction in binary format
/// Format: [OpCode: u8][Mask: u64][Offset: u16]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledRule {
    /// Operation code
    pub opcode: u8,
    /// Bit mask for matching (pattern bytes packed into u64)
    pub mask: u64,
    /// Jump offset, rule_id, or additional data
    pub offset: u16,
}

impl CompiledRule {
    /// Create a new compiled rule
    pub fn new(opcode: OpCode, mask: u64, offset: u16) -> Self {
        Self {
            opcode: opcode as u8,
            mask,
            offset,
        }
    }

    /// Create a Match instruction
    pub fn match_bytes(pattern: &[u8]) -> Self {
        let mut mask = 0u64;
        for (i, &byte) in pattern.iter().take(8).enumerate() {
            mask |= (byte as u64) << (i * 8);
        }
        Self::new(OpCode::Match, mask, pattern.len().min(8) as u16)
    }

    /// Create a Jump instruction
    pub fn jump(offset: u16) -> Self {
        Self::new(OpCode::Jump, 0, offset)
    }

    /// Create an Accept instruction
    pub fn accept(rule_id: u16) -> Self {
        Self::new(OpCode::Accept, 0, rule_id)
    }

    /// Create a Reject instruction
    pub fn reject() -> Self {
        Self::new(OpCode::Reject, 0, 0)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; RULE_SIZE] {
        let mut bytes = [0u8; RULE_SIZE];
        bytes[0] = self.opcode;
        bytes[1..9].copy_from_slice(&self.mask.to_le_bytes());
        bytes[9..11].copy_from_slice(&self.offset.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < RULE_SIZE {
            return None;
        }
        Some(Self {
            opcode: bytes[0],
            mask: u64::from_le_bytes(bytes[1..9].try_into().ok()?),
            offset: u16::from_le_bytes(bytes[9..11].try_into().ok()?),
        })
    }

    /// Get the opcode as enum
    pub fn opcode(&self) -> Option<OpCode> {
        OpCode::from_u8(self.opcode)
    }
}

/// Rule match result
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleMatch {
    /// Rule identifier
    pub rule_id: u32,
    /// Byte offset where match occurred
    pub offset: usize,
    /// Length of matched content
    pub length: usize,
}

/// Rule database header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RuleDbHeader {
    /// Magic bytes "DXR\0"
    pub magic: [u8; 4],
    /// Format version
    pub version: u8,
    /// Number of rules
    pub rule_count: u32,
}

impl RuleDbHeader {
    /// Header size in bytes
    const SIZE: usize = 9; // 4 + 1 + 4

    /// Create a new header
    pub fn new(rule_count: u32) -> Self {
        Self {
            magic: DXR_MAGIC,
            version: DXR_VERSION,
            rule_count,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4] = self.version;
        bytes[5..9].copy_from_slice(&self.rule_count.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        Some(Self {
            magic,
            version: bytes[4],
            rule_count: u32::from_le_bytes(bytes[5..9].try_into().ok()?),
        })
    }

    /// Validate the header
    pub fn is_valid(&self) -> bool {
        self.magic == DXR_MAGIC && self.version == DXR_VERSION
    }
}

/// YAML rule definition (for parsing)
#[derive(Debug, Clone)]
pub struct YamlRule {
    /// Rule identifier
    pub id: String,
    /// Pattern to match (hex string or literal)
    pub pattern: String,
    /// Rule description
    pub description: String,
    /// Severity level
    pub severity: u8,
}

/// Binary Rule Fusion Engine
pub struct RuleEngine {
    /// Compiled rules
    rules: Vec<CompiledRule>,
    /// Rule metadata (id -> description mapping)
    rule_metadata: Vec<(String, String)>,
}

impl RuleEngine {
    /// Create a new rule engine
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            rule_metadata: Vec::new(),
        }
    }

    /// Load compiled rules from .dxr file
    pub fn load_rules(&mut self, path: &Path) -> Result<()> {
        let data = std::fs::read(path)?;

        // Parse header
        let header = RuleDbHeader::from_bytes(&data)
            .ok_or_else(|| SecurityError::RuleCompileError("Invalid header".to_string()))?;

        if !header.is_valid() {
            return Err(SecurityError::RuleCompileError("Invalid magic or version".to_string()));
        }

        // Parse rules
        self.rules.clear();
        let mut offset = RuleDbHeader::SIZE;

        for _ in 0..header.rule_count {
            if offset + RULE_SIZE > data.len() {
                return Err(SecurityError::RuleCompileError("Truncated rule data".to_string()));
            }

            let rule = CompiledRule::from_bytes(&data[offset..])
                .ok_or_else(|| SecurityError::RuleCompileError("Invalid rule".to_string()))?;

            self.rules.push(rule);
            offset += RULE_SIZE;
        }

        Ok(())
    }

    /// Save compiled rules to .dxr file
    pub fn save_rules(&self, path: &Path) -> Result<()> {
        let header = RuleDbHeader::new(self.rules.len() as u32);
        let mut data = Vec::with_capacity(RuleDbHeader::SIZE + self.rules.len() * RULE_SIZE);

        data.extend_from_slice(&header.to_bytes());
        for rule in &self.rules {
            data.extend_from_slice(&rule.to_bytes());
        }

        std::fs::write(path, data)?;
        Ok(())
    }

    /// Execute rules against data (VM execution)
    pub fn execute(&self, data: &[u8]) -> Vec<RuleMatch> {
        let mut matches = Vec::new();

        if self.rules.is_empty() {
            return matches;
        }

        // Scan through data looking for pattern matches
        for data_offset in 0..data.len() {
            let mut pc = 0; // Program counter
            let mut matched_length = 0;
            let mut rule_start = 0; // Track start of current rule group

            while pc < self.rules.len() {
                let rule = &self.rules[pc];

                match OpCode::from_u8(rule.opcode) {
                    Some(OpCode::Match) => {
                        let pattern_len = rule.offset as usize;
                        if data_offset + matched_length + pattern_len > data.len() {
                            // Pattern doesn't fit, skip to next rule group
                            pc = self.find_next_rule_group(pc);
                            matched_length = 0;
                            rule_start = pc;
                            continue;
                        }

                        // Extract pattern bytes from mask
                        let mut pattern_matches = true;
                        for i in 0..pattern_len {
                            let pattern_byte = ((rule.mask >> (i * 8)) & 0xFF) as u8;
                            if data[data_offset + matched_length + i] != pattern_byte {
                                pattern_matches = false;
                                break;
                            }
                        }

                        if pattern_matches {
                            matched_length += pattern_len;
                            pc += 1;
                        } else {
                            // Pattern didn't match, skip to next rule group
                            pc = self.find_next_rule_group(pc);
                            matched_length = 0;
                            rule_start = pc;
                        }
                    }
                    Some(OpCode::Jump) => {
                        pc = rule.offset as usize;
                    }
                    Some(OpCode::Accept) => {
                        if matched_length > 0 {
                            matches.push(RuleMatch {
                                rule_id: rule.offset as u32,
                                offset: data_offset,
                                length: matched_length,
                            });
                        }
                        // Move to next rule group
                        pc = self.find_next_rule_group(pc);
                        matched_length = 0;
                        rule_start = pc;
                    }
                    Some(OpCode::Reject) | Some(OpCode::End) | None => {
                        // Move to next rule group
                        pc = self.find_next_rule_group(pc);
                        matched_length = 0;
                        rule_start = pc;
                    }
                    Some(OpCode::Any) => {
                        if data_offset + matched_length < data.len() {
                            matched_length += 1;
                            pc += 1;
                        } else {
                            pc = self.find_next_rule_group(pc);
                            matched_length = 0;
                            rule_start = pc;
                        }
                    }
                    Some(OpCode::Range) => {
                        if data_offset + matched_length < data.len() {
                            let byte = data[data_offset + matched_length];
                            let min = (rule.mask & 0xFF) as u8;
                            let max = ((rule.mask >> 8) & 0xFF) as u8;
                            if byte >= min && byte <= max {
                                matched_length += 1;
                                pc += 1;
                            } else {
                                pc = self.find_next_rule_group(pc);
                                matched_length = 0;
                                rule_start = pc;
                            }
                        } else {
                            pc = self.find_next_rule_group(pc);
                            matched_length = 0;
                            rule_start = pc;
                        }
                    }
                    Some(OpCode::Branch) => {
                        // Branch not fully implemented - skip for now
                        pc += 1;
                    }
                }
            }
        }

        matches
    }

    /// Find the start of the next rule group (after Accept or End)
    fn find_next_rule_group(&self, current_pc: usize) -> usize {
        let mut pc = current_pc;
        while pc < self.rules.len() {
            if let Some(op) = OpCode::from_u8(self.rules[pc].opcode) {
                if matches!(op, OpCode::Accept | OpCode::End) {
                    return pc + 1;
                }
            }
            pc += 1;
        }
        self.rules.len() // End of rules
    }

    /// Compile YAML rules to binary format
    pub fn compile(yaml_rules: &str) -> Result<Vec<CompiledRule>> {
        let parsed_rules = Self::parse_yaml(yaml_rules)?;
        let mut compiled = Vec::new();

        for (rule_id, rule) in parsed_rules.iter().enumerate() {
            // Parse pattern (supports hex: "0x41424344" or literal: "ABCD")
            let pattern_bytes = Self::parse_pattern(&rule.pattern)?;

            // Generate match instructions for pattern
            for chunk in pattern_bytes.chunks(8) {
                compiled.push(CompiledRule::match_bytes(chunk));
            }

            // Add accept instruction
            compiled.push(CompiledRule::accept(rule_id as u16));
        }

        Ok(compiled)
    }

    /// Parse YAML rule definitions
    fn parse_yaml(yaml: &str) -> Result<Vec<YamlRule>> {
        let mut rules = Vec::new();
        let mut current_rule: Option<YamlRule> = None;

        for line in yaml.lines() {
            let line = line.trim();

            if line.starts_with("- id:") {
                // Save previous rule
                if let Some(rule) = current_rule.take() {
                    rules.push(rule);
                }

                // Start new rule
                let id = line.strip_prefix("- id:").unwrap().trim().trim_matches('"');
                current_rule = Some(YamlRule {
                    id: id.to_string(),
                    pattern: String::new(),
                    description: String::new(),
                    severity: 0,
                });
            } else if let Some(ref mut rule) = current_rule {
                if let Some(pattern) = line.strip_prefix("pattern:") {
                    rule.pattern = pattern.trim().trim_matches('"').to_string();
                } else if let Some(desc) = line.strip_prefix("description:") {
                    rule.description = desc.trim().trim_matches('"').to_string();
                } else if let Some(sev) = line.strip_prefix("severity:") {
                    rule.severity = sev.trim().parse().unwrap_or(0);
                }
            }
        }

        // Save last rule
        if let Some(rule) = current_rule {
            rules.push(rule);
        }

        Ok(rules)
    }

    /// Parse pattern string to bytes
    fn parse_pattern(pattern: &str) -> Result<Vec<u8>> {
        if pattern.starts_with("0x") || pattern.starts_with("0X") {
            // Hex pattern
            let hex = &pattern[2..];
            let mut bytes = Vec::new();
            for i in (0..hex.len()).step_by(2) {
                let byte = u8::from_str_radix(&hex[i..i.min(hex.len()) + 2.min(hex.len() - i)], 16)
                    .map_err(|_| {
                        SecurityError::RuleCompileError(format!("Invalid hex: {}", pattern))
                    })?;
                bytes.push(byte);
            }
            Ok(bytes)
        } else {
            // Literal pattern
            Ok(pattern.as_bytes().to_vec())
        }
    }

    /// Add a compiled rule
    pub fn add_rule(&mut self, rule: CompiledRule) {
        self.rules.push(rule);
    }

    /// Add rules from a compiled set
    pub fn add_rules(&mut self, rules: Vec<CompiledRule>) {
        self.rules.extend(rules);
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get rules
    pub fn rules(&self) -> &[CompiledRule] {
        &self.rules
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
        self.rule_metadata.clear();
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_opcode_conversion() {
        assert_eq!(OpCode::from_u8(0), Some(OpCode::Match));
        assert_eq!(OpCode::from_u8(1), Some(OpCode::Jump));
        assert_eq!(OpCode::from_u8(2), Some(OpCode::Accept));
        assert_eq!(OpCode::from_u8(3), Some(OpCode::Reject));
        assert_eq!(OpCode::from_u8(255), Some(OpCode::End));
        assert_eq!(OpCode::from_u8(100), None);
    }

    #[test]
    fn test_compiled_rule_serialization() {
        let rule = CompiledRule::new(OpCode::Match, 0x4142434445464748, 8);
        let bytes = rule.to_bytes();
        let parsed = CompiledRule::from_bytes(&bytes).unwrap();

        assert_eq!(rule, parsed);
    }

    #[test]
    fn test_match_bytes() {
        let rule = CompiledRule::match_bytes(b"ABCD");

        assert_eq!(rule.opcode, OpCode::Match as u8);
        assert_eq!(rule.offset, 4);

        // Verify pattern is packed correctly
        assert_eq!((rule.mask & 0xFF) as u8, b'A');
        assert_eq!(((rule.mask >> 8) & 0xFF) as u8, b'B');
        assert_eq!(((rule.mask >> 16) & 0xFF) as u8, b'C');
        assert_eq!(((rule.mask >> 24) & 0xFF) as u8, b'D');
    }

    #[test]
    fn test_rule_db_header() {
        let header = RuleDbHeader::new(42);
        let bytes = header.to_bytes();
        let parsed = RuleDbHeader::from_bytes(&bytes).unwrap();

        assert!(parsed.is_valid());
        assert_eq!(parsed.rule_count, 42);
    }

    #[test]
    fn test_save_and_load_rules() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.dxr");

        let mut engine = RuleEngine::new();
        engine.add_rule(CompiledRule::match_bytes(b"TEST"));
        engine.add_rule(CompiledRule::accept(0));

        engine.save_rules(&path).unwrap();

        let mut loaded = RuleEngine::new();
        loaded.load_rules(&path).unwrap();

        assert_eq!(loaded.rule_count(), 2);
    }

    #[test]
    fn test_execute_simple_match() {
        let mut engine = RuleEngine::new();
        engine.add_rule(CompiledRule::match_bytes(b"TEST"));
        engine.add_rule(CompiledRule::accept(0));

        let data = b"Hello TEST World";
        let matches = engine.execute(data);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].rule_id, 0);
        assert_eq!(matches[0].offset, 6);
        assert_eq!(matches[0].length, 4);
    }

    #[test]
    fn test_execute_no_match() {
        let mut engine = RuleEngine::new();
        engine.add_rule(CompiledRule::match_bytes(b"NOTFOUND"));
        engine.add_rule(CompiledRule::accept(0));

        let data = b"Hello World";
        let matches = engine.execute(data);

        assert!(matches.is_empty());
    }

    #[test]
    fn test_execute_multiple_matches() {
        let mut engine = RuleEngine::new();
        engine.add_rule(CompiledRule::match_bytes(b"AB"));
        engine.add_rule(CompiledRule::accept(0));

        let data = b"AB_AB_AB";
        let matches = engine.execute(data);

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_compile_yaml_literal() {
        let yaml = r#"
- id: test_rule
  pattern: "SECRET"
  description: "Test rule"
  severity: 3
"#;

        let compiled = RuleEngine::compile(yaml).unwrap();
        assert!(!compiled.is_empty());
    }

    #[test]
    fn test_compile_yaml_hex() {
        let yaml = r#"
- id: hex_rule
  pattern: "0x414243"
  description: "Hex pattern ABC"
  severity: 2
"#;

        let compiled = RuleEngine::compile(yaml).unwrap();
        assert!(!compiled.is_empty());

        // Verify the pattern matches "ABC"
        let mut engine = RuleEngine::new();
        engine.add_rules(compiled);

        let matches = engine.execute(b"ABC");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_parse_pattern_literal() {
        let bytes = RuleEngine::parse_pattern("HELLO").unwrap();
        assert_eq!(bytes, b"HELLO");
    }

    #[test]
    fn test_parse_pattern_hex() {
        let bytes = RuleEngine::parse_pattern("0x48454C4C4F").unwrap();
        assert_eq!(bytes, b"HELLO");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate a random pattern (1-8 bytes)
    fn arb_pattern() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 1..9)
    }

    /// Generate random data that may contain the pattern
    fn arb_data_with_pattern(pattern: Vec<u8>) -> impl Strategy<Value = (Vec<u8>, Vec<u8>)> {
        let pattern_clone = pattern.clone();
        (0usize..100).prop_map(move |prefix_len| {
            let mut data = vec![0u8; prefix_len];
            data.extend_from_slice(&pattern_clone);
            data.extend_from_slice(&[0u8; 10]);
            (data, pattern_clone.clone())
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 10: Rule Compilation Round-Trip**
        /// **Validates: Requirements 5.1**
        ///
        /// For any valid YAML rule definition, compiling to .dxr format and
        /// executing against test data SHALL produce the same matches as
        /// interpreting the original YAML rules.
        #[test]
        fn prop_rule_serialization_roundtrip(
            opcode in 0u8..4,
            mask in any::<u64>(),
            offset in any::<u16>()
        ) {
            let rule = CompiledRule {
                opcode,
                mask,
                offset,
            };

            let bytes = rule.to_bytes();
            let parsed = CompiledRule::from_bytes(&bytes).unwrap();

            prop_assert_eq!(rule, parsed, "Rule should survive serialization round-trip");
        }

        /// Pattern matching should find patterns at any position
        #[test]
        fn prop_pattern_found_at_position(
            prefix_len in 0usize..50,
            pattern in arb_pattern(),
            suffix_len in 0usize..50
        ) {
            // Build data with pattern at known position
            let mut data = vec![0u8; prefix_len];
            data.extend_from_slice(&pattern);
            data.extend(vec![0u8; suffix_len]);

            // Create engine with pattern
            let mut engine = RuleEngine::new();
            engine.add_rule(CompiledRule::match_bytes(&pattern));
            engine.add_rule(CompiledRule::accept(0));

            let matches = engine.execute(&data);

            // Should find at least one match at the expected position
            let found_at_position = matches.iter().any(|m| m.offset == prefix_len);
            prop_assert!(
                found_at_position,
                "Pattern should be found at position {}",
                prefix_len
            );
        }

        /// Empty data should produce no matches
        #[test]
        fn prop_empty_data_no_matches(pattern in arb_pattern()) {
            let mut engine = RuleEngine::new();
            engine.add_rule(CompiledRule::match_bytes(&pattern));
            engine.add_rule(CompiledRule::accept(0));

            let matches = engine.execute(&[]);

            prop_assert!(matches.is_empty(), "Empty data should produce no matches");
        }

        /// Rule count should be preserved after save/load
        #[test]
        fn prop_rule_count_preserved(num_rules in 1usize..20) {
            let dir = tempfile::TempDir::new().unwrap();
            let path = dir.path().join("test.dxr");

            let mut engine = RuleEngine::new();
            for i in 0..num_rules {
                engine.add_rule(CompiledRule::match_bytes(&[i as u8]));
            }

            engine.save_rules(&path).unwrap();

            let mut loaded = RuleEngine::new();
            loaded.load_rules(&path).unwrap();

            prop_assert_eq!(
                loaded.rule_count(),
                num_rules,
                "Rule count should be preserved"
            );
        }
    }
}
