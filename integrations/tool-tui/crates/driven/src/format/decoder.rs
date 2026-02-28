//! Binary decoder for .drv format

use super::{
    SectionType,
    schema::{
        ContextSection, DrvHeader, PersonaSection, RuleCategory, RuleEntry, StandardsSection,
        WorkflowSection, WorkflowStep,
    },
};
use crate::{DrivenError, Result, parser::UnifiedRule};
use bytes::Buf;
use std::io::Cursor;

/// Decoder for reading .drv binary files
#[derive(Debug)]
pub struct DrvDecoder<'a> {
    /// Raw data
    data: &'a [u8],
    /// Decoded string table
    string_table: Vec<&'a str>,
    /// Header
    header: DrvHeader,
}

impl<'a> DrvDecoder<'a> {
    /// Create a new decoder from binary data
    pub fn new(data: &'a [u8]) -> Result<Self> {
        if data.len() < 16 {
            return Err(DrivenError::InvalidBinary("Data too short for header".to_string()));
        }

        // Parse header
        let header: DrvHeader = *bytemuck::from_bytes(&data[..16]);
        header.validate()?;

        // Verify checksum
        let stored_checksum = header.checksum;
        let computed = {
            let hash = blake3::hash(&data[16..]);
            let bytes = hash.as_bytes();
            u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
        };

        if stored_checksum != computed {
            return Err(DrivenError::InvalidBinary(format!(
                "Checksum mismatch: stored={:08x}, computed={:08x}",
                stored_checksum, computed
            )));
        }

        let mut decoder = Self {
            data,
            string_table: Vec::new(),
            header,
        };

        // Parse string table (always first section after header)
        decoder.parse_string_table()?;

        Ok(decoder)
    }

    /// Parse the string table section
    fn parse_string_table(&mut self) -> Result<()> {
        let mut cursor = Cursor::new(&self.data[16..]);

        // Read section type
        if cursor.remaining() < 1 {
            return Err(DrivenError::InvalidBinary("Missing string table section".to_string()));
        }

        let section_type = cursor.get_u8();
        if section_type != SectionType::StringTable as u8 {
            return Err(DrivenError::InvalidBinary(format!(
                "Expected string table (0x01), got 0x{:02x}",
                section_type
            )));
        }

        // Read count
        if cursor.remaining() < 4 {
            return Err(DrivenError::InvalidBinary("Missing string table count".to_string()));
        }
        let count = cursor.get_u32_le() as usize;

        // Read strings
        self.string_table.reserve(count);
        let base_offset = 16 + 5; // header + section_type + count
        let mut offset = base_offset;

        for _ in 0..count {
            if offset + 2 > self.data.len() {
                return Err(DrivenError::InvalidBinary(
                    "Unexpected end of string table".to_string(),
                ));
            }

            let len = u16::from_le_bytes([self.data[offset], self.data[offset + 1]]) as usize;
            offset += 2;

            if offset + len > self.data.len() {
                return Err(DrivenError::InvalidBinary(
                    "String extends past end of data".to_string(),
                ));
            }

            let s = std::str::from_utf8(&self.data[offset..offset + len])
                .map_err(|e| DrivenError::InvalidBinary(format!("Invalid UTF-8: {}", e)))?;
            self.string_table.push(s);
            offset += len;
        }

        Ok(())
    }

    /// Get a string by index
    pub fn get_string(&self, idx: u32) -> Result<&'a str> {
        self.string_table
            .get(idx as usize)
            .copied()
            .ok_or_else(|| DrivenError::InvalidBinary(format!("Invalid string index: {}", idx)))
    }

    /// Decode all rules from the binary
    pub fn decode_all(&self) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();
        let mut offset = 16 + 5; // Skip header and string table header

        // Skip past string table data
        for s in &self.string_table {
            offset += 2 + s.len();
        }

        // Read remaining sections
        while offset < self.data.len() {
            let section_type = SectionType::try_from(self.data[offset])?;
            offset += 1;

            match section_type {
                SectionType::StringTable => {
                    // Already parsed
                    unreachable!("String table should be first");
                }
                SectionType::Persona => {
                    let (persona, new_offset) = self.decode_persona(offset)?;
                    offset = new_offset;
                    rules.push(UnifiedRule::Persona {
                        name: self.get_string(persona.name_idx)?.to_string(),
                        role: self.get_string(persona.role_idx)?.to_string(),
                        identity: if persona.identity_idx > 0 {
                            Some(self.get_string(persona.identity_idx)?.to_string())
                        } else {
                            None
                        },
                        style: if persona.style_idx > 0 {
                            Some(self.get_string(persona.style_idx)?.to_string())
                        } else {
                            None
                        },
                        traits: persona
                            .traits
                            .iter()
                            .map(|&idx| self.get_string(idx).map(String::from))
                            .collect::<Result<Vec<_>>>()?,
                        principles: persona
                            .principles
                            .iter()
                            .map(|&idx| self.get_string(idx).map(String::from))
                            .collect::<Result<Vec<_>>>()?,
                    });
                }
                SectionType::Standards => {
                    let (standards, new_offset) = self.decode_standards(offset)?;
                    offset = new_offset;
                    for rule in standards.rules {
                        rules.push(UnifiedRule::Standard {
                            category: rule.category,
                            priority: rule.priority,
                            description: self.get_string(rule.description_idx)?.to_string(),
                            pattern: if rule.pattern_idx > 0 {
                                Some(self.get_string(rule.pattern_idx)?.to_string())
                            } else {
                                None
                            },
                        });
                    }
                }
                SectionType::Context => {
                    let (context, new_offset) = self.decode_context(offset)?;
                    offset = new_offset;
                    rules.push(UnifiedRule::Context {
                        includes: context
                            .include_patterns
                            .iter()
                            .map(|&idx| self.get_string(idx).map(String::from))
                            .collect::<Result<Vec<_>>>()?,
                        excludes: context
                            .exclude_patterns
                            .iter()
                            .map(|&idx| self.get_string(idx).map(String::from))
                            .collect::<Result<Vec<_>>>()?,
                        focus: context
                            .focus_areas
                            .iter()
                            .map(|&idx| self.get_string(idx).map(String::from))
                            .collect::<Result<Vec<_>>>()?,
                    });
                }
                SectionType::Workflow => {
                    let (workflow, new_offset) = self.decode_workflow(offset)?;
                    offset = new_offset;
                    let steps = workflow
                        .steps
                        .iter()
                        .map(|step| {
                            Ok(crate::parser::WorkflowStepData {
                                name: self.get_string(step.name_idx)?.to_string(),
                                description: self.get_string(step.description_idx)?.to_string(),
                                condition: if step.condition_idx > 0 {
                                    Some(self.get_string(step.condition_idx)?.to_string())
                                } else {
                                    None
                                },
                                actions: step
                                    .actions
                                    .iter()
                                    .map(|&idx| self.get_string(idx).map(String::from))
                                    .collect::<Result<Vec<_>>>()?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    rules.push(UnifiedRule::Workflow {
                        name: self.get_string(workflow.name_idx)?.to_string(),
                        steps,
                    });
                }
            }
        }

        Ok(rules)
    }

    fn decode_persona(&self, offset: usize) -> Result<(PersonaSection, usize)> {
        let mut cursor = Cursor::new(&self.data[offset..]);

        let name_idx = cursor.get_u32_le();
        let role_idx = cursor.get_u32_le();
        let identity_idx = cursor.get_u32_le();
        let style_idx = cursor.get_u32_le();

        let traits_count = cursor.get_u16_le() as usize;
        let mut traits = Vec::with_capacity(traits_count);
        for _ in 0..traits_count {
            traits.push(cursor.get_u32_le());
        }

        let principles_count = cursor.get_u16_le() as usize;
        let mut principles = Vec::with_capacity(principles_count);
        for _ in 0..principles_count {
            principles.push(cursor.get_u32_le());
        }

        let persona = PersonaSection {
            name_idx,
            role_idx,
            identity_idx,
            style_idx,
            traits,
            principles,
        };

        Ok((persona, offset + cursor.position() as usize))
    }

    fn decode_standards(&self, offset: usize) -> Result<(StandardsSection, usize)> {
        let mut cursor = Cursor::new(&self.data[offset..]);

        let count = cursor.get_u32_le() as usize;
        let mut rules = Vec::with_capacity(count);

        for _ in 0..count {
            let category = RuleCategory::from(cursor.get_u8());
            let priority = cursor.get_u8();
            let description_idx = cursor.get_u32_le();
            let pattern_idx = cursor.get_u32_le();

            rules.push(RuleEntry {
                category,
                priority,
                description_idx,
                pattern_idx,
            });
        }

        Ok((StandardsSection { rules }, offset + cursor.position() as usize))
    }

    fn decode_context(&self, offset: usize) -> Result<(ContextSection, usize)> {
        let mut cursor = Cursor::new(&self.data[offset..]);

        let include_count = cursor.get_u16_le() as usize;
        let mut include_patterns = Vec::with_capacity(include_count);
        for _ in 0..include_count {
            include_patterns.push(cursor.get_u32_le());
        }

        let exclude_count = cursor.get_u16_le() as usize;
        let mut exclude_patterns = Vec::with_capacity(exclude_count);
        for _ in 0..exclude_count {
            exclude_patterns.push(cursor.get_u32_le());
        }

        let focus_count = cursor.get_u16_le() as usize;
        let mut focus_areas = Vec::with_capacity(focus_count);
        for _ in 0..focus_count {
            focus_areas.push(cursor.get_u32_le());
        }

        let dep_count = cursor.get_u16_le() as usize;
        let mut dependencies = Vec::with_capacity(dep_count);
        for _ in 0..dep_count {
            let name = cursor.get_u32_le();
            let version = cursor.get_u32_le();
            dependencies.push((name, version));
        }

        Ok((
            ContextSection {
                include_patterns,
                exclude_patterns,
                focus_areas,
                dependencies,
            },
            offset + cursor.position() as usize,
        ))
    }

    fn decode_workflow(&self, offset: usize) -> Result<(WorkflowSection, usize)> {
        let mut cursor = Cursor::new(&self.data[offset..]);

        let name_idx = cursor.get_u32_le();
        let step_count = cursor.get_u16_le() as usize;

        let mut steps = Vec::with_capacity(step_count);
        for _ in 0..step_count {
            let step_name_idx = cursor.get_u32_le();
            let description_idx = cursor.get_u32_le();
            let condition_idx = cursor.get_u32_le();

            let action_count = cursor.get_u16_le() as usize;
            let mut actions = Vec::with_capacity(action_count);
            for _ in 0..action_count {
                actions.push(cursor.get_u32_le());
            }

            steps.push(WorkflowStep {
                name_idx: step_name_idx,
                description_idx,
                condition_idx,
                actions,
            });
        }

        Ok((WorkflowSection { name_idx, steps }, offset + cursor.position() as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::DrvEncoder;

    #[test]
    fn test_roundtrip_empty() {
        let encoder = DrvEncoder::new();
        let data = encoder.encode(&[]).unwrap();

        let decoder = DrvDecoder::new(&data).unwrap();
        let rules = decoder.decode_all().unwrap();

        assert!(rules.is_empty());
    }

    #[test]
    fn test_invalid_magic() {
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(b"BAD\0");

        let result = DrvDecoder::new(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_short() {
        let data = vec![0u8; 10];
        let result = DrvDecoder::new(&data);
        assert!(result.is_err());
    }
}
