//! Escape analysis for determining stack allocation eligibility

use std::collections::{HashMap, HashSet};

/// Represents an allocation site in the bytecode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AllocSite {
    /// Bytecode offset of the allocation
    pub offset: u32,
    /// Type of allocation
    pub kind: AllocKind,
}

/// Kind of allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocKind {
    Tuple,
    List,
    Dict,
    Set,
    Object,
    Closure,
}

/// Reason why an object escapes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeReason {
    /// Returned from function
    Returned,
    /// Stored in a global variable
    StoredGlobal,
    /// Stored in an object field
    StoredField,
    /// Passed to an unknown function
    PassedToCall,
    /// Stored in a container that escapes
    StoredInEscaping,
    /// Yielded from generator
    Yielded,
}

/// Result of escape analysis for an allocation site
#[derive(Debug, Clone)]
pub struct EscapeInfo {
    /// Whether the object escapes
    pub escapes: bool,
    /// Reason for escape (if any)
    pub reason: Option<EscapeReason>,
    /// Maximum size if known (for stack allocation sizing)
    pub max_size: Option<usize>,
}

/// Escape analyzer for bytecode
pub struct EscapeAnalyzer {
    /// Allocation sites found in the bytecode
    alloc_sites: HashMap<u32, AllocSite>,
    /// Set of allocation sites that are stack candidates
    stack_candidates: HashSet<u32>,
    /// Set of allocation sites that escape
    escaped: HashSet<u32>,
    /// Escape info per allocation site
    escape_info: HashMap<u32, EscapeInfo>,
}

impl EscapeAnalyzer {
    /// Create a new escape analyzer
    pub fn new() -> Self {
        Self {
            alloc_sites: HashMap::new(),
            stack_candidates: HashSet::new(),
            escaped: HashSet::new(),
            escape_info: HashMap::new(),
        }
    }

    /// Analyze bytecode for escape information
    pub fn analyze(&mut self, bytecode: &[u8]) {
        // First pass: identify allocation sites
        self.find_allocation_sites(bytecode);

        // Initially, all allocations are stack candidates
        self.stack_candidates = self.alloc_sites.keys().copied().collect();

        // Second pass: mark escaping objects
        self.mark_escaping(bytecode);

        // Build escape info
        for (&offset, site) in &self.alloc_sites {
            let escapes = self.escaped.contains(&offset);
            self.escape_info.insert(
                offset,
                EscapeInfo {
                    escapes,
                    reason: if escapes {
                        Some(EscapeReason::Returned)
                    } else {
                        None
                    },
                    max_size: self.estimate_size(site.kind),
                },
            );
        }
    }

    /// First pass: find all allocation sites
    fn find_allocation_sites(&mut self, bytecode: &[u8]) {
        let mut offset = 0u32;

        while (offset as usize) < bytecode.len() {
            let opcode = bytecode[offset as usize];

            // Check for allocation opcodes
            let kind = match opcode {
                0x80 => Some(AllocKind::Tuple),          // BuildTuple
                0x81 => Some(AllocKind::List),           // BuildList
                0x82 => Some(AllocKind::Set),            // BuildSet
                0x83 => Some(AllocKind::Dict),           // BuildDict
                0x73 | 0x74 => Some(AllocKind::Closure), // MakeFunction/MakeClosure
                _ => None,
            };

            if let Some(kind) = kind {
                self.alloc_sites.insert(offset, AllocSite { offset, kind });
            }

            // Advance by opcode size (simplified - assumes 1-3 byte opcodes)
            offset += self.opcode_size(opcode);
        }
    }

    /// Second pass: mark objects that escape
    fn mark_escaping(&mut self, bytecode: &[u8]) {
        let mut offset = 0u32;

        // Track which local variables hold allocations
        let _local_allocs: HashMap<u16, u32> = HashMap::new();

        while (offset as usize) < bytecode.len() {
            let opcode = bytecode[offset as usize];

            // Collect candidates to mark as escaped
            let mut to_escape: Vec<(u32, EscapeReason)> = Vec::new();

            match opcode {
                // Return escapes the top of stack
                0x56 => {
                    // Return
                    // Mark any allocation on stack as escaping
                    // In a real implementation, we'd track the stack
                    for &alloc_offset in self.stack_candidates.iter() {
                        // Conservative: mark recent allocations as escaping
                        if alloc_offset < offset && offset - alloc_offset < 20 {
                            to_escape.push((alloc_offset, EscapeReason::Returned));
                        }
                    }
                }

                // Yield escapes
                0x57 | 0x58 => {
                    // Yield, YieldFrom
                    for &alloc_offset in self.stack_candidates.iter() {
                        if alloc_offset < offset && offset - alloc_offset < 20 {
                            to_escape.push((alloc_offset, EscapeReason::Yielded));
                        }
                    }
                }

                // Store to global escapes
                0x04 => {
                    // StoreGlobal
                    for &alloc_offset in self.stack_candidates.iter() {
                        if alloc_offset < offset && offset - alloc_offset < 10 {
                            to_escape.push((alloc_offset, EscapeReason::StoredGlobal));
                        }
                    }
                }

                // Store to attribute escapes
                0x06 => {
                    // StoreAttr
                    for &alloc_offset in self.stack_candidates.iter() {
                        if alloc_offset < offset && offset - alloc_offset < 10 {
                            to_escape.push((alloc_offset, EscapeReason::StoredField));
                        }
                    }
                }

                // Function calls may escape arguments
                0x70 | 0x71 | 0x72 | 0x76 => {
                    // Call variants
                    // Conservative: assume calls escape their arguments
                    for &alloc_offset in self.stack_candidates.iter() {
                        if alloc_offset < offset && offset - alloc_offset < 30 {
                            to_escape.push((alloc_offset, EscapeReason::PassedToCall));
                        }
                    }
                }

                _ => {}
            }

            // Now mark all collected escapes
            for (alloc_offset, reason) in to_escape {
                self.mark_escaped(alloc_offset, reason);
            }

            offset += self.opcode_size(opcode);
        }
    }

    /// Mark an allocation as escaped
    fn mark_escaped(&mut self, offset: u32, reason: EscapeReason) {
        self.escaped.insert(offset);
        self.stack_candidates.remove(&offset);

        if let Some(info) = self.escape_info.get_mut(&offset) {
            info.escapes = true;
            info.reason = Some(reason);
        }
    }

    /// Get the size of an opcode (simplified)
    fn opcode_size(&self, opcode: u8) -> u32 {
        match opcode {
            // No argument
            0x15
            | 0x12
            | 0x13
            | 0x16
            | 0x56
            | 0x57
            | 0x59
            | 0x5A
            | 0x20..=0x30
            | 0x40..=0x4A
            | 0xF0 => 1,
            // 1-byte argument
            0x14 | 0x17 | 0x80..=0x8F | 0x92 | 0xF1..=0xF5 => 2,
            // 2-byte argument (most common)
            _ => 3,
        }
    }

    /// Estimate the size of an allocation
    fn estimate_size(&self, kind: AllocKind) -> Option<usize> {
        match kind {
            AllocKind::Tuple => Some(64),  // Small tuple
            AllocKind::List => Some(128),  // Small list
            AllocKind::Dict => Some(256),  // Small dict
            AllocKind::Set => Some(128),   // Small set
            AllocKind::Object => Some(64), // Small object
            AllocKind::Closure => None,    // Variable size
        }
    }

    /// Check if an allocation site can be stack-allocated
    pub fn can_stack_allocate(&self, offset: u32) -> bool {
        self.stack_candidates.contains(&offset)
    }

    /// Get escape info for an allocation site
    pub fn get_escape_info(&self, offset: u32) -> Option<&EscapeInfo> {
        self.escape_info.get(&offset)
    }

    /// Get all stack-allocatable sites
    pub fn stack_candidates(&self) -> &HashSet<u32> {
        &self.stack_candidates
    }

    /// Get all allocation sites
    pub fn alloc_sites(&self) -> &HashMap<u32, AllocSite> {
        &self.alloc_sites
    }
}

impl Default for EscapeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_bytecode() {
        let mut analyzer = EscapeAnalyzer::new();
        analyzer.analyze(&[]);
        assert!(analyzer.alloc_sites().is_empty());
    }

    #[test]
    fn test_find_tuple_allocation() {
        let mut analyzer = EscapeAnalyzer::new();
        // BuildTuple with size 2, then Return
        let bytecode = [0x80, 0x02, 0x56];
        analyzer.analyze(&bytecode);

        assert!(analyzer.alloc_sites().contains_key(&0));
        assert_eq!(analyzer.alloc_sites()[&0].kind, AllocKind::Tuple);
    }

    #[test]
    fn test_return_escapes() {
        let mut analyzer = EscapeAnalyzer::new();
        // BuildTuple, Return (tuple escapes)
        let bytecode = [0x80, 0x02, 0x56];
        analyzer.analyze(&bytecode);

        // The tuple should escape because it's returned
        assert!(!analyzer.can_stack_allocate(0));
    }
}
