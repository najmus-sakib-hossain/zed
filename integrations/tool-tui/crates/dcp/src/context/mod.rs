//! Memory-mapped context for zero-copy sharing.
//!
//! Provides shared memory context for DCP protocol with atomic operations
//! and memory fencing for thread safety.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::DCPError;

/// Magic number for context validation
pub const CONTEXT_MAGIC: u64 = 0x4443505F_43545831; // "DCP_CTX1"

/// Maximum number of tool states in context
pub const MAX_TOOL_STATES: usize = 25;

/// Context memory layout header offset
pub const HEADER_OFFSET: usize = 0;
/// Conversation ID offset
pub const CONVERSATION_ID_OFFSET: usize = 8;
/// Message count offset
pub const MESSAGE_COUNT_OFFSET: usize = 16;
/// Tool states offset
pub const TOOL_STATES_OFFSET: usize = 24;
/// Dynamic content offset (after tool states)
pub const DYNAMIC_CONTENT_OFFSET: usize = TOOL_STATES_OFFSET + (MAX_TOOL_STATES * ToolState::SIZE);

/// Tool state structure
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolState {
    /// Tool ID
    pub tool_id: u32,
    /// State flags
    pub flags: u32,
    /// Last invocation timestamp
    pub last_invoked: u64,
    /// State data (24 bytes)
    pub data: [u8; 24],
}

impl ToolState {
    /// Size of ToolState in bytes
    pub const SIZE: usize = 40; // 4 + 4 + 8 + 24

    /// Create a new empty tool state
    pub fn new(tool_id: u32) -> Self {
        Self {
            tool_id,
            flags: 0,
            last_invoked: 0,
            data: [0u8; 24],
        }
    }

    /// Parse from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }
}

/// Context memory layout
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ContextLayout {
    /// Magic + version (8 bytes)
    pub header: u64,
    /// Conversation ID (8 bytes)
    pub conversation_id: u64,
    /// Message count (8 bytes)
    pub message_count: u64,
    /// Tool states: 25 tools × 40 bytes = 1000 bytes
    pub tool_states: [ToolState; MAX_TOOL_STATES],
}

impl ContextLayout {
    /// Size of the layout in bytes
    pub const SIZE: usize = 8 + 8 + 8 + (MAX_TOOL_STATES * ToolState::SIZE); // 1024 bytes

    /// Create a new context layout
    pub fn new(conversation_id: u64) -> Self {
        Self {
            header: CONTEXT_MAGIC,
            conversation_id,
            message_count: 0,
            tool_states: [ToolState::new(0); MAX_TOOL_STATES],
        }
    }

    /// Parse from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        let layout = unsafe { &*(bytes.as_ptr() as *const Self) };
        if layout.header != CONTEXT_MAGIC {
            return Err(DCPError::InvalidMagic);
        }
        Ok(layout)
    }

    /// Serialize to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }
}

/// Memory-mapped context for zero-copy sharing
pub struct DcpContext {
    /// Shared memory buffer
    buffer: Box<[u8]>,
    /// Memory layout version
    version: u32,
}

impl DcpContext {
    /// Minimum buffer size
    pub const MIN_SIZE: usize = ContextLayout::SIZE;

    /// Create a new context with the given conversation ID
    pub fn new(conversation_id: u64) -> Self {
        let mut buffer = vec![0u8; Self::MIN_SIZE].into_boxed_slice();

        // Initialize the layout
        let layout = ContextLayout::new(conversation_id);
        buffer[..ContextLayout::SIZE].copy_from_slice(layout.as_bytes());

        Self { buffer, version: 1 }
    }

    /// Create a context from existing shared memory
    pub fn from_shared(buffer: Box<[u8]>) -> Result<Self, DCPError> {
        if buffer.len() < Self::MIN_SIZE {
            return Err(DCPError::InsufficientData);
        }

        // Validate magic
        let header = u64::from_le_bytes(buffer[0..8].try_into().unwrap());
        if header != CONTEXT_MAGIC {
            return Err(DCPError::InvalidMagic);
        }

        Ok(Self { buffer, version: 1 })
    }

    /// Get the raw buffer for sharing
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Get the buffer size
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    /// Get the context layout
    #[inline(always)]
    fn as_layout(&self) -> &ContextLayout {
        unsafe { &*(self.buffer.as_ptr() as *const ContextLayout) }
    }

    /// Get the conversation ID
    pub fn conversation_id(&self) -> u64 {
        self.as_layout().conversation_id
    }

    /// Get the message count (atomic read)
    pub fn message_count(&self) -> u64 {
        let ptr = unsafe { self.buffer.as_ptr().add(MESSAGE_COUNT_OFFSET) as *const AtomicU64 };
        unsafe { (*ptr).load(Ordering::Acquire) }
    }

    /// Increment the message count atomically
    pub fn increment_message_count(&self) -> u64 {
        let ptr = unsafe { self.buffer.as_ptr().add(MESSAGE_COUNT_OFFSET) as *const AtomicU64 };
        unsafe { (*ptr).fetch_add(1, Ordering::AcqRel) + 1 }
    }

    /// Get the offset for a tool state by index
    #[inline(always)]
    fn tool_state_offset(&self, index: usize) -> usize {
        TOOL_STATES_OFFSET + (index * ToolState::SIZE)
    }

    /// Zero-copy access to tool state by index
    #[inline(always)]
    pub fn get_tool_state(&self, index: usize) -> Option<&ToolState> {
        if index >= MAX_TOOL_STATES {
            return None;
        }
        let offset = self.tool_state_offset(index);
        Some(unsafe { &*(self.buffer.as_ptr().add(offset) as *const ToolState) })
    }

    /// Find tool state by tool_id
    pub fn find_tool_state(&self, tool_id: u32) -> Option<&ToolState> {
        for i in 0..MAX_TOOL_STATES {
            if let Some(state) = self.get_tool_state(i) {
                if state.tool_id == tool_id {
                    return Some(state);
                }
            }
        }
        None
    }

    /// Atomic update with memory fence
    ///
    /// # Safety
    /// This function uses unsafe pointer operations but ensures memory safety
    /// through proper bounds checking and memory fencing.
    pub fn update_tool_state(&mut self, index: usize, state: &ToolState) -> Result<(), DCPError> {
        if index >= MAX_TOOL_STATES {
            return Err(DCPError::OutOfBounds);
        }

        let offset = self.tool_state_offset(index);

        // Copy the state data
        unsafe {
            std::ptr::copy_nonoverlapping(
                state as *const ToolState as *const u8,
                self.buffer.as_mut_ptr().add(offset),
                ToolState::SIZE,
            );
        }

        // Memory fence for cross-thread visibility
        std::sync::atomic::fence(Ordering::Release);

        Ok(())
    }

    /// Update tool state by tool_id, finding or allocating a slot
    pub fn set_tool_state(&mut self, state: &ToolState) -> Result<usize, DCPError> {
        // First, try to find existing slot
        for i in 0..MAX_TOOL_STATES {
            if let Some(existing) = self.get_tool_state(i) {
                if existing.tool_id == state.tool_id {
                    self.update_tool_state(i, state)?;
                    return Ok(i);
                }
            }
        }

        // Find empty slot (tool_id == 0)
        for i in 0..MAX_TOOL_STATES {
            if let Some(existing) = self.get_tool_state(i) {
                if existing.tool_id == 0 {
                    self.update_tool_state(i, state)?;
                    return Ok(i);
                }
            }
        }

        Err(DCPError::OutOfBounds)
    }

    /// Clear a tool state slot
    pub fn clear_tool_state(&mut self, index: usize) -> Result<(), DCPError> {
        let empty = ToolState::new(0);
        self.update_tool_state(index, &empty)
    }

    /// Get the version
    pub fn version(&self) -> u32 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_state_size() {
        assert_eq!(std::mem::size_of::<ToolState>(), ToolState::SIZE);
    }

    #[test]
    fn test_context_layout_size() {
        assert_eq!(std::mem::size_of::<ContextLayout>(), ContextLayout::SIZE);
    }

    #[test]
    fn test_context_creation() {
        let ctx = DcpContext::new(12345);
        assert_eq!(ctx.conversation_id(), 12345);
        assert_eq!(ctx.message_count(), 0);
    }

    #[test]
    fn test_message_count_increment() {
        let ctx = DcpContext::new(1);
        assert_eq!(ctx.message_count(), 0);
        assert_eq!(ctx.increment_message_count(), 1);
        assert_eq!(ctx.message_count(), 1);
        assert_eq!(ctx.increment_message_count(), 2);
        assert_eq!(ctx.message_count(), 2);
    }

    #[test]
    fn test_tool_state_operations() {
        let mut ctx = DcpContext::new(1);

        let state = ToolState {
            tool_id: 42,
            flags: 0x1234,
            last_invoked: 1234567890,
            data: [0xAB; 24],
        };

        // Update tool state
        ctx.update_tool_state(0, &state).unwrap();

        // Read it back
        let read_state = ctx.get_tool_state(0).unwrap();
        assert_eq!(read_state.tool_id, 42);
        assert_eq!(read_state.flags, 0x1234);
        assert_eq!(read_state.last_invoked, 1234567890);
        assert_eq!(read_state.data, [0xAB; 24]);
    }

    #[test]
    fn test_find_tool_state() {
        let mut ctx = DcpContext::new(1);

        let state1 = ToolState {
            tool_id: 100,
            flags: 1,
            last_invoked: 0,
            data: [0; 24],
        };
        let state2 = ToolState {
            tool_id: 200,
            flags: 2,
            last_invoked: 0,
            data: [0; 24],
        };

        ctx.update_tool_state(0, &state1).unwrap();
        ctx.update_tool_state(1, &state2).unwrap();

        let found = ctx.find_tool_state(200).unwrap();
        assert_eq!(found.tool_id, 200);
        assert_eq!(found.flags, 2);

        assert!(ctx.find_tool_state(999).is_none());
    }

    #[test]
    fn test_set_tool_state() {
        let mut ctx = DcpContext::new(1);

        let state = ToolState {
            tool_id: 42,
            flags: 1,
            last_invoked: 100,
            data: [0; 24],
        };

        // First set should allocate slot 0
        let idx = ctx.set_tool_state(&state).unwrap();
        assert_eq!(idx, 0);

        // Update same tool_id should use same slot
        let updated = ToolState {
            tool_id: 42,
            flags: 2,
            last_invoked: 200,
            data: [0; 24],
        };
        let idx2 = ctx.set_tool_state(&updated).unwrap();
        assert_eq!(idx2, 0);

        // Verify update
        let read = ctx.get_tool_state(0).unwrap();
        assert_eq!(read.flags, 2);
        assert_eq!(read.last_invoked, 200);
    }

    #[test]
    fn test_out_of_bounds() {
        let mut ctx = DcpContext::new(1);
        let state = ToolState::new(1);

        assert!(ctx.get_tool_state(MAX_TOOL_STATES).is_none());
        assert_eq!(ctx.update_tool_state(MAX_TOOL_STATES, &state), Err(DCPError::OutOfBounds));
    }

    #[test]
    fn test_from_shared() {
        let ctx1 = DcpContext::new(99999);
        let bytes = ctx1.as_bytes().to_vec().into_boxed_slice();

        let ctx2 = DcpContext::from_shared(bytes).unwrap();
        assert_eq!(ctx2.conversation_id(), 99999);
    }

    #[test]
    fn test_invalid_magic() {
        let mut buffer = vec![0u8; DcpContext::MIN_SIZE].into_boxed_slice();
        buffer[0..8].copy_from_slice(&[0xFF; 8]);

        let result = DcpContext::from_shared(buffer);
        assert!(matches!(result, Err(DCPError::InvalidMagic)));
    }

    #[test]
    fn test_tool_state_round_trip() {
        let state = ToolState {
            tool_id: 123,
            flags: 0xDEAD,
            last_invoked: 0xCAFEBABE,
            data: [0x42; 24],
        };

        let bytes = state.as_bytes();
        let parsed = ToolState::from_bytes(bytes).unwrap();

        assert_eq!(parsed.tool_id, 123);
        assert_eq!(parsed.flags, 0xDEAD);
        assert_eq!(parsed.last_invoked, 0xCAFEBABE);
        assert_eq!(parsed.data, [0x42; 24]);
    }
}
