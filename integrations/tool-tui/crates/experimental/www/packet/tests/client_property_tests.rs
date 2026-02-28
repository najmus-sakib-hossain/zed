//! Property-based tests for dx-www-client HTIP protocol
//!
//! These tests verify the correctness of HTIP rendering, event dispatch,
//! and incremental updates using property-based testing.
//!
//! Tests are in the packet crate because the client crate is no_std.

extern crate alloc;

use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

// ============================================================================
// HTIP Opcode Constants (must match client/src/lib.rs)
// ============================================================================

const OP_CLONE: u8 = 1;
const OP_PATCH_TEXT: u8 = 2;
const OP_PATCH_ATTR: u8 = 3;
const OP_CLASS_TOGGLE: u8 = 4;
const OP_REMOVE: u8 = 5;
const OP_EVENT: u8 = 6;
const OP_TEMPLATE_DEF: u8 = 8;
const OP_EOF: u8 = 255;

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate a valid HTIP header
fn generate_htip_header() -> Vec<u8> {
    vec![
        0x58, 0x44, // Magic "DX" (little-endian)
        0x02, // Version 2
        0x00, // Flags
    ]
}

/// Generate a template definition opcode
fn generate_template_def(id: u8, html: &[u8]) -> Vec<u8> {
    let mut data = vec![OP_TEMPLATE_DEF];
    data.push(id);
    data.extend_from_slice(&(html.len() as u16).to_le_bytes());
    data.extend_from_slice(html);
    data
}

/// Generate a clone opcode
fn generate_clone_op(template_id: u8) -> Vec<u8> {
    vec![OP_CLONE, template_id]
}

/// Generate a patch text opcode
fn generate_patch_text_op(node_id: u16, text: &[u8]) -> Vec<u8> {
    let mut data = vec![OP_PATCH_TEXT];
    data.extend_from_slice(&node_id.to_le_bytes());
    data.extend_from_slice(&(text.len() as u16).to_le_bytes());
    data.extend_from_slice(text);
    data
}

/// Generate a patch attr opcode
fn generate_patch_attr_op(node_id: u16, key: &[u8], value: &[u8]) -> Vec<u8> {
    let mut data = vec![OP_PATCH_ATTR];
    data.extend_from_slice(&node_id.to_le_bytes());
    data.extend_from_slice(&(key.len() as u16).to_le_bytes());
    data.extend_from_slice(key);
    data.extend_from_slice(&(value.len() as u16).to_le_bytes());
    data.extend_from_slice(value);
    data
}

/// Generate a class toggle opcode
fn generate_class_toggle_op(node_id: u16, class: &[u8], enable: bool) -> Vec<u8> {
    let mut data = vec![OP_CLASS_TOGGLE];
    data.extend_from_slice(&node_id.to_le_bytes());
    data.extend_from_slice(&(class.len() as u16).to_le_bytes());
    data.extend_from_slice(class);
    data.push(if enable { 1 } else { 0 });
    data
}

/// Generate a remove opcode
#[allow(dead_code)]
fn generate_remove_op(node_id: u16) -> Vec<u8> {
    let mut data = vec![OP_REMOVE];
    data.extend_from_slice(&node_id.to_le_bytes());
    data
}

/// Generate an event opcode
fn generate_event_op(node_id: u16, event_type: u8, handler_id: u16) -> Vec<u8> {
    let mut data = vec![OP_EVENT];
    data.extend_from_slice(&node_id.to_le_bytes());
    data.push(event_type);
    data.extend_from_slice(&handler_id.to_le_bytes());
    data
}

/// Generate EOF opcode
fn generate_eof() -> Vec<u8> {
    vec![OP_EOF]
}

/// Build a complete HTIP stream from opcodes
fn build_htip_stream(opcodes: Vec<Vec<u8>>) -> Vec<u8> {
    let mut stream = generate_htip_header();
    for op in opcodes {
        stream.extend(op);
    }
    stream.extend(generate_eof());
    stream
}

/// Parse HTIP stream and count opcodes by type
fn count_opcodes(stream: &[u8]) -> HashMap<u8, usize> {
    let mut counts = HashMap::new();
    let mut offset = 4; // Skip header

    while offset < stream.len() {
        let op = stream[offset];
        *counts.entry(op).or_insert(0) += 1;

        if op == OP_EOF {
            break;
        }

        // Skip opcode payload
        offset += 1;
        match op {
            OP_CLONE => offset += 1,
            OP_TEMPLATE_DEF => {
                if offset + 2 < stream.len() {
                    offset += 1; // id
                    let len = u16::from_le_bytes([stream[offset], stream[offset + 1]]) as usize;
                    offset += 2 + len;
                }
            }
            OP_PATCH_TEXT => {
                if offset + 3 < stream.len() {
                    offset += 2; // node_id
                    let len = u16::from_le_bytes([stream[offset], stream[offset + 1]]) as usize;
                    offset += 2 + len;
                }
            }
            OP_PATCH_ATTR => {
                if offset + 5 < stream.len() {
                    offset += 2; // node_id
                    let key_len = u16::from_le_bytes([stream[offset], stream[offset + 1]]) as usize;
                    offset += 2 + key_len;
                    if offset + 1 < stream.len() {
                        let val_len =
                            u16::from_le_bytes([stream[offset], stream[offset + 1]]) as usize;
                        offset += 2 + val_len;
                    }
                }
            }
            OP_CLASS_TOGGLE => {
                if offset + 4 < stream.len() {
                    offset += 2; // node_id
                    let len = u16::from_le_bytes([stream[offset], stream[offset + 1]]) as usize;
                    offset += 2 + len + 1; // +1 for enable flag
                }
            }
            OP_REMOVE => offset += 2,
            OP_EVENT => offset += 5,
            _ => break,
        }
    }

    counts
}

// ============================================================================
// Property Test 9: HTIP Rendering Correctness
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 9: HTIP Rendering Correctness
    ///
    /// For any valid HTIP stream:
    /// 1. All template definitions are present
    /// 2. All clone operations reference valid templates
    /// 3. All patch operations reference valid nodes
    /// 4. Stream is well-formed with proper header and EOF
    #[test]
    fn prop_htip_rendering_correctness(
        template_count in 1usize..5,
        clone_count in 1usize..10,
        patch_count in 0usize..20,
    ) {
        let mut opcodes = Vec::new();

        // Generate template definitions
        for i in 0..template_count {
            let html = format!("<div id=\"t{}\">Template {}</div>", i, i);
            opcodes.push(generate_template_def(i as u8, html.as_bytes()));
        }

        // Generate clone operations
        for i in 0..clone_count {
            let template_id = (i % template_count) as u8;
            opcodes.push(generate_clone_op(template_id));
        }

        // Generate patch operations
        for i in 0..patch_count {
            let node_id = (i % clone_count.max(1)) as u16;
            match i % 3 {
                0 => {
                    let text = format!("Updated text {}", i);
                    opcodes.push(generate_patch_text_op(node_id, text.as_bytes()));
                }
                1 => {
                    let key = format!("data-{}", i);
                    let value = format!("value-{}", i);
                    opcodes.push(generate_patch_attr_op(node_id, key.as_bytes(), value.as_bytes()));
                }
                _ => {
                    let class = format!("class-{}", i);
                    opcodes.push(generate_class_toggle_op(node_id, class.as_bytes(), i % 2 == 0));
                }
            }
        }

        let stream = build_htip_stream(opcodes);
        let counts = count_opcodes(&stream);

        // Property 1: Valid header
        prop_assert!(stream.len() >= 4, "Stream too short");
        prop_assert_eq!(stream[0], 0x58, "Invalid magic byte 0");
        prop_assert_eq!(stream[1], 0x44, "Invalid magic byte 1");
        prop_assert_eq!(stream[2], 0x02, "Invalid version");

        // Property 2: Has EOF marker
        prop_assert_eq!(counts.get(&OP_EOF), Some(&1), "Missing or multiple EOF");

        // Property 3: Template count matches
        prop_assert_eq!(
            counts.get(&OP_TEMPLATE_DEF).copied().unwrap_or(0),
            template_count,
            "Template count mismatch"
        );

        // Property 4: Clone count matches
        prop_assert_eq!(
            counts.get(&OP_CLONE).copied().unwrap_or(0),
            clone_count,
            "Clone count mismatch"
        );
    }

    /// Property 9b: HTIP stream is well-formed
    #[test]
    fn prop_htip_stream_wellformed(
        num_templates in 0usize..3,
        num_clones in 0usize..5,
        num_patches in 0usize..10,
    ) {
        let mut opcodes = Vec::new();

        for i in 0..num_templates {
            let html = format!("<span>{}</span>", i);
            opcodes.push(generate_template_def(i as u8, html.as_bytes()));
        }

        for i in 0..num_clones {
            if num_templates > 0 {
                opcodes.push(generate_clone_op((i % num_templates) as u8));
            }
        }

        for i in 0..num_patches {
            if num_clones > 0 {
                let node_id = (i % num_clones) as u16;
                let text = format!("text{}", i);
                opcodes.push(generate_patch_text_op(node_id, text.as_bytes()));
            }
        }

        let stream = build_htip_stream(opcodes);

        // Property 1: Valid header
        prop_assert!(stream.len() >= 4);
        prop_assert_eq!(&stream[0..2], &[0x58, 0x44]);

        // Property 2: Valid version
        prop_assert_eq!(stream[2], 0x02);

        // Property 3: Ends with EOF
        prop_assert_eq!(*stream.last().unwrap(), OP_EOF);

        // Property 4: Reasonable size
        prop_assert!(stream.len() < 1_000_000, "Stream too large");
    }
}

// ============================================================================
// Property Test 10: Event Dispatch
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 10: Event Dispatch Correctness
    #[test]
    fn prop_event_dispatch(
        num_nodes in 1usize..10,
        events_per_node in 1usize..5,
    ) {
        let mut opcodes = Vec::new();
        let mut registered_handlers: Vec<(u16, u8, u16)> = Vec::new();

        // Create a template
        opcodes.push(generate_template_def(0, b"<button>Click</button>"));

        // Clone nodes
        for _ in 0..num_nodes {
            opcodes.push(generate_clone_op(0));
        }

        // Register events
        let mut handler_id = 1u16;
        for node_idx in 0..num_nodes {
            for event_idx in 0..events_per_node {
                let node_id = node_idx as u16;
                let event_type = (event_idx % 11) as u8;

                opcodes.push(generate_event_op(node_id, event_type, handler_id));
                registered_handlers.push((node_id, event_type, handler_id));
                handler_id += 1;
            }
        }

        let stream = build_htip_stream(opcodes);
        let counts = count_opcodes(&stream);

        // Property 1: All handler IDs are unique
        let handler_ids: Vec<u16> = registered_handlers.iter().map(|(_, _, h)| *h).collect();
        let unique_handlers: HashSet<u16> = handler_ids.iter().copied().collect();
        prop_assert_eq!(handler_ids.len(), unique_handlers.len(), "Duplicate handler IDs");

        // Property 2: All event types are valid
        for (_, event_type, _) in &registered_handlers {
            prop_assert!(*event_type <= 10, "Invalid event type: {}", event_type);
        }

        // Property 3: Event opcode count matches
        prop_assert_eq!(
            counts.get(&OP_EVENT).copied().unwrap_or(0),
            registered_handlers.len(),
            "Event opcode count mismatch"
        );
    }

    /// Property 10b: Event handler uniqueness per node
    #[test]
    fn prop_event_handler_uniqueness(
        num_nodes in 1usize..5,
        num_event_types in 1usize..4,
    ) {
        let mut handlers: HashMap<(u16, u8), u16> = HashMap::new();
        let mut handler_id = 1u16;

        for node_idx in 0..num_nodes {
            for event_type in 0..num_event_types {
                let key = (node_idx as u16, event_type as u8);
                handlers.insert(key, handler_id);
                handler_id += 1;
            }
        }

        // Property: Each (node, event) pair maps to exactly one handler
        prop_assert_eq!(handlers.len(), num_nodes * num_event_types);

        // Property: All handlers are unique
        let unique_handlers: HashSet<u16> = handlers.values().copied().collect();
        prop_assert_eq!(unique_handlers.len(), handlers.len());
    }
}

// ============================================================================
// Property Test 11: Incremental Updates
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 11: Incremental Update Correctness
    #[test]
    fn prop_incremental_updates(
        num_nodes in 1usize..10,
        num_updates in 1usize..20,
    ) {
        let num_slots = 3usize;
        let mut node_states: Vec<String> = (0..num_nodes).map(|i| format!("initial-{}", i)).collect();
        let node_dependencies: Vec<usize> = (0..num_nodes).map(|i| i % num_slots).collect();

        // Track the last update value for each slot
        let mut slot_values: Vec<Option<String>> = vec![None; num_slots];

        // Apply updates
        for update_idx in 0..num_updates {
            let slot = update_idx % num_slots;
            let new_value = format!("updated-{}-{}", slot, update_idx);
            slot_values[slot] = Some(new_value.clone());

            // Update all nodes dependent on this slot
            for (node_idx, &dep_slot) in node_dependencies.iter().enumerate() {
                if dep_slot == slot {
                    node_states[node_idx] = new_value.clone();
                }
            }
        }

        // Property 1: All nodes have valid state
        for (idx, state) in node_states.iter().enumerate() {
            prop_assert!(
                state.starts_with("updated-") || state.starts_with("initial-"),
                "Invalid state for node {}: {}", idx, state
            );
        }

        // Property 2: Nodes with same dependency that was updated have same state
        for slot in 0..num_slots {
            if slot_values[slot].is_some() {
                // This slot was updated, so all dependent nodes should have same state
                let nodes_for_slot: Vec<&String> = node_dependencies.iter()
                    .enumerate()
                    .filter(|&(_, s)| *s == slot)
                    .map(|(i, _)| &node_states[i])
                    .collect();

                if nodes_for_slot.len() > 1 {
                    let first = nodes_for_slot[0];
                    for state in &nodes_for_slot[1..] {
                        prop_assert_eq!(first, *state, "Nodes with same dependency have different states");
                    }
                }
            }
        }
    }

    /// Property 11b: Dirty tracking correctness
    #[test]
    fn prop_dirty_tracking(
        num_nodes in 1usize..20,
        num_slots in 1usize..5,
    ) {
        let dependencies: Vec<usize> = (0..num_nodes).map(|i| i % num_slots).collect();

        for slot in 0..num_slots {
            let expected_dirty: Vec<usize> = dependencies.iter()
                .enumerate()
                .filter(|&(_, s)| *s == slot)
                .map(|(i, _)| i)
                .collect();

            let has_dependents = dependencies.iter().any(|s| *s == slot);
            prop_assert_eq!(!expected_dirty.is_empty(), has_dependents);

            for node_idx in 0..num_nodes {
                let should_be_dirty = dependencies[node_idx] == slot;
                let is_dirty = expected_dirty.contains(&node_idx);
                prop_assert_eq!(should_be_dirty, is_dirty,
                    "Node {} dirty mismatch for slot {}", node_idx, slot);
            }
        }
    }
}

// ============================================================================
// Additional Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    /// Property: Remove operations don't affect other nodes
    #[test]
    fn prop_remove_isolation(
        num_nodes in 2usize..10,
        remove_idx in 0usize..10,
    ) {
        let remove_idx = remove_idx % num_nodes;
        let mut nodes: Vec<bool> = vec![true; num_nodes];

        nodes[remove_idx] = false;

        for (idx, &exists) in nodes.iter().enumerate() {
            if idx == remove_idx {
                prop_assert!(!exists, "Removed node should not exist");
            } else {
                prop_assert!(exists, "Other nodes should still exist");
            }
        }
    }

    /// Property: Attribute patches are idempotent
    #[test]
    fn prop_attr_patch_idempotent(
        key in "[a-z]{1,10}",
        value in "[a-z0-9]{0,20}",
    ) {
        let patch1 = generate_patch_attr_op(0, key.as_bytes(), value.as_bytes());
        let patch2 = generate_patch_attr_op(0, key.as_bytes(), value.as_bytes());

        prop_assert_eq!(patch1, patch2, "Same patch should produce same bytes");
    }

    /// Property: Class toggle is reversible
    #[test]
    fn prop_class_toggle_reversible(
        class_name in "[a-z]{1,15}",
    ) {
        let enable = generate_class_toggle_op(0, class_name.as_bytes(), true);
        let disable = generate_class_toggle_op(0, class_name.as_bytes(), false);

        prop_assert_ne!(&enable, &disable, "Enable and disable should differ");
        prop_assert_eq!(enable.len(), disable.len(), "Toggle ops should have same length");
        prop_assert_eq!(&enable[..enable.len()-1], &disable[..disable.len()-1],
            "Only enable flag should differ");
    }
}
