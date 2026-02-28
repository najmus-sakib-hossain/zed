//! Property-based tests for context memory.
//!
//! Feature: dcp-protocol, Property 13: Context Memory Thread Safety

use dcp::context::{CONTEXT_MAGIC, MAX_TOOL_STATES};
use dcp::{ContextLayout, DCPError, DcpContext, ToolState};
use proptest::prelude::*;
use std::sync::Arc;
use std::thread;

/// Strategy to generate a random ToolState
fn arb_tool_state() -> impl Strategy<Value = ToolState> {
    (
        1u32..=65535,      // tool_id (non-zero)
        any::<u32>(),      // flags
        any::<u64>(),      // last_invoked
        any::<[u8; 24]>(), // data
    )
        .prop_map(|(tool_id, flags, last_invoked, data)| ToolState {
            tool_id,
            flags,
            last_invoked,
            data,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, updates SHALL be visible after memory fencing.
    /// **Validates: Requirements 10.1, 10.3, 10.4, 10.5**
    #[test]
    fn prop_tool_state_update_visible(
        conversation_id in any::<u64>(),
        state in arb_tool_state(),
        index in 0usize..MAX_TOOL_STATES,
    ) {
        let mut ctx = DcpContext::new(conversation_id);

        // Update tool state
        ctx.update_tool_state(index, &state).unwrap();

        // Read it back - should be visible
        let read = ctx.get_tool_state(index).unwrap();
        prop_assert_eq!(read.tool_id, state.tool_id);
        prop_assert_eq!(read.flags, state.flags);
        prop_assert_eq!(read.last_invoked, state.last_invoked);
        prop_assert_eq!(read.data, state.data);
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, conversation_id SHALL be preserved.
    /// **Validates: Requirements 10.1, 10.2**
    #[test]
    fn prop_conversation_id_preserved(
        conversation_id in any::<u64>(),
    ) {
        let ctx = DcpContext::new(conversation_id);
        prop_assert_eq!(ctx.conversation_id(), conversation_id);
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, message count increments SHALL be atomic.
    /// **Validates: Requirements 10.4, 10.5**
    #[test]
    fn prop_message_count_increments(
        conversation_id in any::<u64>(),
        increments in 1usize..100,
    ) {
        let ctx = DcpContext::new(conversation_id);
        prop_assert_eq!(ctx.message_count(), 0);

        for i in 1..=increments {
            let new_count = ctx.increment_message_count();
            prop_assert_eq!(new_count, i as u64);
        }

        prop_assert_eq!(ctx.message_count(), increments as u64);
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any out-of-bounds index, operations SHALL return error without panicking.
    /// **Validates: Requirements 10.3**
    #[test]
    fn prop_out_of_bounds_returns_error(
        conversation_id in any::<u64>(),
        index in MAX_TOOL_STATES..1000usize,
        state in arb_tool_state(),
    ) {
        let mut ctx = DcpContext::new(conversation_id);

        // Get should return None
        prop_assert!(ctx.get_tool_state(index).is_none());

        // Update should return error
        let result = ctx.update_tool_state(index, &state);
        prop_assert_eq!(result, Err(DCPError::OutOfBounds));
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, find_tool_state SHALL find the correct state.
    /// **Validates: Requirements 10.3**
    #[test]
    fn prop_find_tool_state_correct(
        conversation_id in any::<u64>(),
        states in prop::collection::vec(arb_tool_state(), 1..MAX_TOOL_STATES),
    ) {
        let mut ctx = DcpContext::new(conversation_id);

        // Deduplicate by tool_id
        let mut unique_states: Vec<ToolState> = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();
        for state in states {
            if seen_ids.insert(state.tool_id) {
                unique_states.push(state);
            }
        }

        // Set all states
        for (i, state) in unique_states.iter().enumerate() {
            ctx.update_tool_state(i, state).unwrap();
        }

        // Find each state
        for state in &unique_states {
            let found = ctx.find_tool_state(state.tool_id);
            prop_assert!(found.is_some(), "Should find tool_id {}", state.tool_id);
            let found = found.unwrap();
            prop_assert_eq!(found.tool_id, state.tool_id);
            prop_assert_eq!(found.flags, state.flags);
            prop_assert_eq!(found.last_invoked, state.last_invoked);
        }

        // Non-existent tool_id (that we didn't set) should return None
        // Note: tool_id 0 is used for empty slots, so we use a high value
        prop_assert!(ctx.find_tool_state(0xFFFFFFFF).is_none());
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, set_tool_state SHALL update existing or allocate new slot.
    /// **Validates: Requirements 10.3, 10.4**
    #[test]
    fn prop_set_tool_state_allocates_correctly(
        conversation_id in any::<u64>(),
        state1 in arb_tool_state(),
        state2_flags in any::<u32>(),
    ) {
        let mut ctx = DcpContext::new(conversation_id);

        // First set should allocate
        let idx1 = ctx.set_tool_state(&state1).unwrap();

        // Update same tool_id should use same slot
        let updated = ToolState {
            tool_id: state1.tool_id,
            flags: state2_flags,
            last_invoked: state1.last_invoked + 1,
            data: state1.data,
        };
        let idx2 = ctx.set_tool_state(&updated).unwrap();
        prop_assert_eq!(idx1, idx2, "Same tool_id should use same slot");

        // Verify update
        let read = ctx.get_tool_state(idx1).unwrap();
        prop_assert_eq!(read.flags, state2_flags);
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, clear_tool_state SHALL reset the slot.
    /// **Validates: Requirements 10.3, 10.4**
    #[test]
    fn prop_clear_tool_state_resets(
        conversation_id in any::<u64>(),
        state in arb_tool_state(),
        index in 0usize..MAX_TOOL_STATES,
    ) {
        let mut ctx = DcpContext::new(conversation_id);

        // Set a state
        ctx.update_tool_state(index, &state).unwrap();

        // Clear it
        ctx.clear_tool_state(index).unwrap();

        // Should be zeroed
        let read = ctx.get_tool_state(index).unwrap();
        prop_assert_eq!(read.tool_id, 0);
        prop_assert_eq!(read.flags, 0);
        prop_assert_eq!(read.last_invoked, 0);
        prop_assert_eq!(read.data, [0u8; 24]);
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (partial)
    /// For any ToolState, serializing to bytes and deserializing back
    /// SHALL produce an equivalent struct.
    /// **Validates: Requirements 10.2**
    #[test]
    fn prop_tool_state_round_trip(
        state in arb_tool_state(),
    ) {
        let bytes = state.as_bytes();
        let parsed = ToolState::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.tool_id, state.tool_id);
        prop_assert_eq!(parsed.flags, state.flags);
        prop_assert_eq!(parsed.last_invoked, state.last_invoked);
        prop_assert_eq!(parsed.data, state.data);
    }

    /// Feature: dcp-protocol, Property 1: Binary Struct Round-Trip (partial)
    /// For any ContextLayout, serializing to bytes and deserializing back
    /// SHALL produce an equivalent struct.
    /// **Validates: Requirements 10.2**
    #[test]
    fn prop_context_layout_round_trip(
        conversation_id in any::<u64>(),
    ) {
        let layout = ContextLayout::new(conversation_id);
        let bytes = layout.as_bytes();
        let parsed = ContextLayout::from_bytes(bytes).unwrap();

        prop_assert_eq!(parsed.header, CONTEXT_MAGIC);
        prop_assert_eq!(parsed.conversation_id, conversation_id);
        prop_assert_eq!(parsed.message_count, 0);
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// For any DcpContext, from_shared SHALL reconstruct the context correctly.
    /// **Validates: Requirements 10.1, 10.2**
    #[test]
    fn prop_from_shared_reconstructs(
        conversation_id in any::<u64>(),
        states in prop::collection::vec(arb_tool_state(), 0..5),
    ) {
        let mut ctx = DcpContext::new(conversation_id);

        // Set some states
        for (i, state) in states.iter().enumerate() {
            ctx.update_tool_state(i, state).unwrap();
        }

        // Increment message count
        ctx.increment_message_count();
        ctx.increment_message_count();

        // Reconstruct from shared bytes
        let bytes = ctx.as_bytes().to_vec().into_boxed_slice();
        let ctx2 = DcpContext::from_shared(bytes).unwrap();

        // Verify reconstruction
        prop_assert_eq!(ctx2.conversation_id(), conversation_id);
        prop_assert_eq!(ctx2.message_count(), 2);

        for (i, state) in states.iter().enumerate() {
            let read = ctx2.get_tool_state(i).unwrap();
            prop_assert_eq!(read.tool_id, state.tool_id);
            prop_assert_eq!(read.flags, state.flags);
        }
    }

    /// Feature: dcp-protocol, Property 13: Context Memory Thread Safety
    /// Invalid magic SHALL be rejected.
    /// **Validates: Requirements 10.1**
    #[test]
    fn prop_invalid_magic_rejected(
        bad_magic in any::<u64>().prop_filter("not context magic", |m| *m != CONTEXT_MAGIC),
    ) {
        let mut buffer = vec![0u8; DcpContext::MIN_SIZE].into_boxed_slice();
        buffer[0..8].copy_from_slice(&bad_magic.to_le_bytes());

        let result = DcpContext::from_shared(buffer);
        prop_assert_eq!(result.err(), Some(DCPError::InvalidMagic));
    }
}

/// Test concurrent message count increments
#[test]
fn test_concurrent_message_count() {
    use std::sync::atomic::AtomicU64;

    let ctx = Arc::new(DcpContext::new(1));
    let num_threads = 4;
    let increments_per_thread = 100;

    let handles: Vec<_> = (0..num_threads)
        .map(|_| {
            let ctx = Arc::clone(&ctx);
            thread::spawn(move || {
                for _ in 0..increments_per_thread {
                    ctx.increment_message_count();
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(ctx.message_count(), (num_threads * increments_per_thread) as u64);
}
