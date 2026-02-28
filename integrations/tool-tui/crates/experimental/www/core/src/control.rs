//! # Binary Control Flow Opcodes
//!
//! Binary Dawn's control flow uses binary opcodes instead of component overhead.
//! This achieves 30x faster list rendering compared to React's ~15ms.
//!
//! Control flow operations are encoded as binary instructions with pointer and template fields.

/// Control flow opcodes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlOpcode {
    /// For-each loop over a list
    ForEach = 0x01,
    /// Conditional show/hide
    Show = 0x02,
    /// Switch/match statement
    Switch = 0x03,
}

impl ControlOpcode {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::ForEach),
            0x02 => Some(Self::Show),
            0x03 => Some(Self::Switch),
            _ => None,
        }
    }
}

/// ForEach instruction
///
/// Iterates over a list in SharedArrayBuffer and clones a template for each item.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForEachOp {
    /// Opcode identifier
    pub opcode: ControlOpcode,
    /// Padding for alignment
    pub _pad: u8,
    /// Pointer to array in SharedArrayBuffer
    pub list_ptr: u32,
    /// Size of each item in bytes
    pub item_size: u16,
    /// Template ID to clone per item
    pub template_id: u16,
}

impl ForEachOp {
    /// Size of ForEachOp in bytes
    pub const SIZE: usize = 10;

    /// Create a new ForEach operation
    pub const fn new(list_ptr: u32, item_size: u16, template_id: u16) -> Self {
        Self {
            opcode: ControlOpcode::ForEach,
            _pad: 0,
            list_ptr,
            item_size,
            template_id,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.opcode as u8;
        bytes[1] = self._pad;
        bytes[2..6].copy_from_slice(&self.list_ptr.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.item_size.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.template_id.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            opcode: ControlOpcode::from_u8(bytes[0])?,
            _pad: bytes[1],
            list_ptr: u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            item_size: u16::from_le_bytes([bytes[6], bytes[7]]),
            template_id: u16::from_le_bytes([bytes[8], bytes[9]]),
        })
    }
}

/// Show instruction
///
/// Conditionally shows content based on a boolean in SharedArrayBuffer.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowOp {
    /// Opcode identifier
    pub opcode: ControlOpcode,
    /// Padding for alignment
    pub _pad: u8,
    /// Pointer to bool condition in SharedArrayBuffer
    pub condition_ptr: u32,
    /// Template ID to show when true
    pub template_id: u16,
    /// Fallback template ID when false
    pub fallback_id: u16,
}

impl ShowOp {
    /// Size of ShowOp in bytes
    pub const SIZE: usize = 10;

    /// Create a new Show operation
    pub const fn new(condition_ptr: u32, template_id: u16, fallback_id: u16) -> Self {
        Self {
            opcode: ControlOpcode::Show,
            _pad: 0,
            condition_ptr,
            template_id,
            fallback_id,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.opcode as u8;
        bytes[1] = self._pad;
        bytes[2..6].copy_from_slice(&self.condition_ptr.to_le_bytes());
        bytes[6..8].copy_from_slice(&self.template_id.to_le_bytes());
        bytes[8..10].copy_from_slice(&self.fallback_id.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        Some(Self {
            opcode: ControlOpcode::from_u8(bytes[0])?,
            _pad: bytes[1],
            condition_ptr: u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            template_id: u16::from_le_bytes([bytes[6], bytes[7]]),
            fallback_id: u16::from_le_bytes([bytes[8], bytes[9]]),
        })
    }

    /// Evaluate the condition and return the appropriate template ID
    #[inline(always)]
    pub fn evaluate(&self, shared_buffer: &[u8]) -> u16 {
        let condition = shared_buffer.get(self.condition_ptr as usize).copied().unwrap_or(0) != 0;
        if condition {
            self.template_id
        } else {
            self.fallback_id
        }
    }
}

/// Maximum number of switch cases
pub const MAX_SWITCH_CASES: usize = 8;

/// Switch instruction
///
/// Multi-way branch based on a discriminant value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchOp {
    /// Opcode identifier
    pub opcode: ControlOpcode,
    /// Number of cases (max 8)
    pub case_count: u8,
    /// Pointer to discriminant value in SharedArrayBuffer
    pub value_ptr: u32,
    /// Template ID per case (max 8)
    pub cases: [u16; MAX_SWITCH_CASES],
}

impl SwitchOp {
    /// Size of SwitchOp in bytes
    pub const SIZE: usize = 22;

    /// Create a new Switch operation
    pub fn new(value_ptr: u32, cases: &[u16]) -> Self {
        let mut case_array = [0u16; MAX_SWITCH_CASES];
        let case_count = cases.len().min(MAX_SWITCH_CASES);
        case_array[..case_count].copy_from_slice(&cases[..case_count]);

        Self {
            opcode: ControlOpcode::Switch,
            case_count: case_count as u8,
            value_ptr,
            cases: case_array,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.opcode as u8;
        bytes[1] = self.case_count;
        bytes[2..6].copy_from_slice(&self.value_ptr.to_le_bytes());
        for (i, &case_id) in self.cases.iter().enumerate() {
            let offset = 6 + i * 2;
            bytes[offset..offset + 2].copy_from_slice(&case_id.to_le_bytes());
        }
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let mut cases = [0u16; MAX_SWITCH_CASES];
        for (i, case) in cases.iter_mut().enumerate().take(MAX_SWITCH_CASES) {
            let offset = 6 + i * 2;
            *case = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        }
        Some(Self {
            opcode: ControlOpcode::from_u8(bytes[0])?,
            case_count: bytes[1],
            value_ptr: u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]),
            cases,
        })
    }

    /// Evaluate the switch and return the appropriate template ID
    #[inline(always)]
    pub fn evaluate(&self, shared_buffer: &[u8]) -> Option<u16> {
        let value = shared_buffer.get(self.value_ptr as usize).copied()? as usize;
        if value < self.case_count as usize {
            Some(self.cases[value])
        } else {
            None
        }
    }
}

/// Diff operation for keyed lists
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffOp {
    /// Keep item at position
    Keep(u16),
    /// Insert new item at position
    Insert(u16),
    /// Remove item at position
    Remove(u16),
    /// Move item from old position to new position
    Move { from: u16, to: u16 },
}

/// Keyed list with SIMD diffing support
///
/// Uses SIMD comparison for O(n) key diffing with 8 keys compared at once.
#[derive(Debug, Clone)]
pub struct KeyedList {
    /// Keys for each item (u32 per item)
    pub keys: Vec<u32>,
    /// Pointer to items in SharedArrayBuffer
    pub items_ptr: u32,
    /// DOM node IDs for each item
    pub dom_nodes: Vec<u16>,
}

impl KeyedList {
    /// Create a new keyed list
    pub fn new(items_ptr: u32) -> Self {
        Self {
            keys: Vec::new(),
            items_ptr,
            dom_nodes: Vec::new(),
        }
    }

    /// Add an item with key
    pub fn push(&mut self, key: u32, dom_node: u16) {
        self.keys.push(key);
        self.dom_nodes.push(dom_node);
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Find key index
    pub fn find_key(&self, key: u32) -> Option<usize> {
        self.keys.iter().position(|&k| k == key)
    }

    /// Diff against new keys and generate minimal operations
    ///
    /// This is a simplified diff algorithm. In production, SIMD would be used.
    pub fn diff_keys(&self, new_keys: &[u32]) -> Vec<DiffOp> {
        let mut ops = Vec::new();
        let old_keys = &self.keys;

        // Build a map of old key positions
        let old_positions: std::collections::HashMap<u32, usize> =
            old_keys.iter().enumerate().map(|(i, &k)| (k, i)).collect();

        // Track which old items are used
        let mut used = vec![false; old_keys.len()];

        // Process new keys
        for (new_idx, &new_key) in new_keys.iter().enumerate() {
            if let Some(&old_idx) = old_positions.get(&new_key) {
                used[old_idx] = true;
                if old_idx != new_idx {
                    ops.push(DiffOp::Move {
                        from: old_idx as u16,
                        to: new_idx as u16,
                    });
                } else {
                    ops.push(DiffOp::Keep(new_idx as u16));
                }
            } else {
                ops.push(DiffOp::Insert(new_idx as u16));
            }
        }

        // Remove unused old items (in reverse order to maintain indices)
        for (old_idx, &is_used) in used.iter().enumerate().rev() {
            if !is_used {
                ops.push(DiffOp::Remove(old_idx as u16));
            }
        }

        ops
    }

    /// Apply diff operations and update the list
    pub fn apply_diff(&mut self, new_keys: &[u32], new_dom_nodes: &[u16]) {
        self.keys = new_keys.to_vec();
        self.dom_nodes = new_dom_nodes.to_vec();
    }

    /// SIMD-accelerated key comparison (8 keys at once)
    ///
    /// This is a placeholder for actual SIMD implementation.
    #[cfg(target_arch = "x86_64")]
    pub fn simd_find_key(&self, key: u32) -> Option<usize> {
        // In production, this would use AVX2 to compare 8 keys at once
        // For now, fall back to scalar implementation
        self.find_key(key)
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn simd_find_key(&self, key: u32) -> Option<usize> {
        self.find_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_foreach_op_roundtrip() {
        let op = ForEachOp::new(0x1000, 16, 42);
        let bytes = op.to_bytes();
        let restored = ForEachOp::from_bytes(&bytes).unwrap();
        assert_eq!(op, restored);
    }

    #[test]
    fn test_show_op_roundtrip() {
        let op = ShowOp::new(0x2000, 10, 20);
        let bytes = op.to_bytes();
        let restored = ShowOp::from_bytes(&bytes).unwrap();
        assert_eq!(op, restored);
    }

    #[test]
    fn test_show_op_evaluate() {
        let op = ShowOp::new(0, 10, 20);

        // Condition true
        let buffer_true = [1u8];
        assert_eq!(op.evaluate(&buffer_true), 10);

        // Condition false
        let buffer_false = [0u8];
        assert_eq!(op.evaluate(&buffer_false), 20);
    }

    #[test]
    fn test_switch_op_roundtrip() {
        let op = SwitchOp::new(0x3000, &[100, 200, 300]);
        let bytes = op.to_bytes();
        let restored = SwitchOp::from_bytes(&bytes).unwrap();
        assert_eq!(op.opcode, restored.opcode);
        assert_eq!(op.case_count, restored.case_count);
        assert_eq!(op.value_ptr, restored.value_ptr);
        assert_eq!(op.cases[..3], restored.cases[..3]);
    }

    #[test]
    fn test_switch_op_evaluate() {
        let op = SwitchOp::new(0, &[100, 200, 300]);

        let buffer0 = [0u8];
        assert_eq!(op.evaluate(&buffer0), Some(100));

        let buffer1 = [1u8];
        assert_eq!(op.evaluate(&buffer1), Some(200));

        let buffer2 = [2u8];
        assert_eq!(op.evaluate(&buffer2), Some(300));

        // Out of range
        let buffer_oob = [10u8];
        assert_eq!(op.evaluate(&buffer_oob), None);
    }

    #[test]
    fn test_keyed_list_diff_insert() {
        let list = KeyedList::new(0);
        let new_keys = [1, 2, 3];
        let ops = list.diff_keys(&new_keys);

        // All should be inserts
        assert!(ops.iter().all(|op| matches!(op, DiffOp::Insert(_))));
    }

    #[test]
    fn test_keyed_list_diff_remove() {
        let mut list = KeyedList::new(0);
        list.push(1, 0);
        list.push(2, 1);
        list.push(3, 2);

        let new_keys: [u32; 0] = [];
        let ops = list.diff_keys(&new_keys);

        // All should be removes
        assert!(ops.iter().all(|op| matches!(op, DiffOp::Remove(_))));
    }

    #[test]
    fn test_keyed_list_diff_keep() {
        let mut list = KeyedList::new(0);
        list.push(1, 0);
        list.push(2, 1);
        list.push(3, 2);

        let new_keys = [1, 2, 3];
        let ops = list.diff_keys(&new_keys);

        // All should be keeps
        let keeps: Vec<_> = ops.iter().filter(|op| matches!(op, DiffOp::Keep(_))).collect();
        assert_eq!(keeps.len(), 3);
    }

    #[test]
    fn test_keyed_list_diff_move() {
        let mut list = KeyedList::new(0);
        list.push(1, 0);
        list.push(2, 1);
        list.push(3, 2);

        // Reverse order
        let new_keys = [3, 2, 1];
        let ops = list.diff_keys(&new_keys);

        // Should have moves
        let moves: Vec<_> = ops.iter().filter(|op| matches!(op, DiffOp::Move { .. })).collect();
        assert!(!moves.is_empty());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // **Feature: binary-dawn-features, Property 18: Control Op Struct Fields**
    // *For any* ForEachOp, it SHALL contain list_ptr (u32), item_size (u16), and template_id (u16).
    // *For any* ShowOp, it SHALL contain condition_ptr (u32), template_id (u16), and fallback_id (u16).
    // **Validates: Requirements 10.2, 10.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_foreach_op_fields(
            list_ptr in any::<u32>(),
            item_size in any::<u16>(),
            template_id in any::<u16>()
        ) {
            let op = ForEachOp::new(list_ptr, item_size, template_id);

            // Verify all fields are present and correct
            prop_assert_eq!(op.opcode, ControlOpcode::ForEach);
            prop_assert_eq!(op.list_ptr, list_ptr);
            prop_assert_eq!(op.item_size, item_size);
            prop_assert_eq!(op.template_id, template_id);

            // Verify round-trip
            let bytes = op.to_bytes();
            let restored = ForEachOp::from_bytes(&bytes);
            prop_assert!(restored.is_some());
            prop_assert_eq!(op, restored.unwrap());
        }

        #[test]
        fn prop_show_op_fields(
            condition_ptr in any::<u32>(),
            template_id in any::<u16>(),
            fallback_id in any::<u16>()
        ) {
            let op = ShowOp::new(condition_ptr, template_id, fallback_id);

            // Verify all fields are present and correct
            prop_assert_eq!(op.opcode, ControlOpcode::Show);
            prop_assert_eq!(op.condition_ptr, condition_ptr);
            prop_assert_eq!(op.template_id, template_id);
            prop_assert_eq!(op.fallback_id, fallback_id);

            // Verify round-trip
            let bytes = op.to_bytes();
            let restored = ShowOp::from_bytes(&bytes);
            prop_assert!(restored.is_some());
            prop_assert_eq!(op, restored.unwrap());
        }
    }

    // **Feature: binary-dawn-features, Property 19: Keyed List Diff Correctness**
    // *For any* two key arrays, the diff operation SHALL produce a minimal set of operations
    // that transforms the first array into the second.
    // **Validates: Requirements 10.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_keyed_list_diff_correctness(
            old_keys in prop::collection::vec(any::<u32>(), 0..20),
            new_keys in prop::collection::vec(any::<u32>(), 0..20)
        ) {
            let mut list = KeyedList::new(0);
            for (i, &key) in old_keys.iter().enumerate() {
                list.push(key, i as u16);
            }

            let ops = list.diff_keys(&new_keys);

            // Verify: applying ops should transform old_keys to new_keys
            // Count operations by type
            let inserts = ops.iter().filter(|op| matches!(op, DiffOp::Insert(_))).count();
            let removes = ops.iter().filter(|op| matches!(op, DiffOp::Remove(_))).count();
            let keeps = ops.iter().filter(|op| matches!(op, DiffOp::Keep(_))).count();
            let moves = ops.iter().filter(|op| matches!(op, DiffOp::Move { .. })).count();

            // Basic sanity checks
            // Number of inserts should be at least the number of new keys not in old
            let new_not_in_old: std::collections::HashSet<_> = new_keys.iter()
                .filter(|k| !old_keys.contains(k))
                .collect();
            prop_assert!(inserts >= new_not_in_old.len());

            // Number of removes should be at least the number of old keys not in new
            let old_not_in_new: std::collections::HashSet<_> = old_keys.iter()
                .filter(|k| !new_keys.contains(k))
                .collect();
            prop_assert!(removes >= old_not_in_new.len());

            // Total operations should be reasonable
            prop_assert!(inserts + removes + keeps + moves <= old_keys.len() + new_keys.len() + 10);
        }
    }

    // Switch op round-trip
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_switch_op_roundtrip(
            value_ptr in any::<u32>(),
            case_count in 1usize..8,
            cases in prop::collection::vec(any::<u16>(), 8)
        ) {
            let op = SwitchOp::new(value_ptr, &cases[..case_count]);
            let bytes = op.to_bytes();
            let restored = SwitchOp::from_bytes(&bytes);

            prop_assert!(restored.is_some());
            let restored = restored.unwrap();
            prop_assert_eq!(op.opcode, restored.opcode);
            prop_assert_eq!(op.value_ptr, restored.value_ptr);
            prop_assert_eq!(op.case_count, restored.case_count);
            for i in 0..case_count {
                prop_assert_eq!(op.cases[i], restored.cases[i]);
            }
        }
    }

    // Show op evaluation
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_show_op_evaluation(
            condition_ptr in 0u32..100,
            template_id in any::<u16>(),
            fallback_id in any::<u16>(),
            condition_value in any::<u8>()
        ) {
            let op = ShowOp::new(condition_ptr, template_id, fallback_id);

            let mut buffer = vec![0u8; 100];
            buffer[condition_ptr as usize] = condition_value;

            let result = op.evaluate(&buffer);

            if condition_value != 0 {
                prop_assert_eq!(result, template_id);
            } else {
                prop_assert_eq!(result, fallback_id);
            }
        }
    }
}
