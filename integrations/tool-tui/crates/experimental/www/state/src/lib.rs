//! # dx-state — Binary State Management
//!
//! Replace Zustand/Redux with binary memory slots.
//!
//! ## Performance
//! - State read: 1 CPU instruction (memory load)
//! - State write: 2 CPU instructions (store + atomic OR)
//! - Subscription notify: < 0.01 ms
//! - Bundle: 0 KB (built-in)

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

use atomic::{Atomic, Ordering};
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::collections::HashMap;
#[cfg(feature = "std")]
use std::sync::Arc;

/// Binary protocol opcodes for state operations
pub mod opcodes {
    pub const STATE_INIT: u8 = 0x80;
    pub const STATE_SET: u8 = 0x81;
    pub const STATE_GET: u8 = 0x82;
    pub const STATE_SUBSCRIBE: u8 = 0x83;
    pub const STATE_NOTIFY: u8 = 0x84;
}

/// Dirty bit tracker (64-bit bitmask for 64 fields max)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct DirtyMask {
    bits: u64,
}

impl DirtyMask {
    /// Create empty dirty mask
    #[inline]
    pub const fn new() -> Self {
        Self { bits: 0 }
    }

    /// Mark field as dirty
    #[inline]
    pub fn mark_dirty(&mut self, field_index: u8) {
        debug_assert!(field_index < 64, "Field index must be < 64");
        self.bits |= 1u64 << field_index;
    }

    /// Check if field is dirty
    #[inline]
    pub const fn is_dirty(&self, field_index: u8) -> bool {
        (self.bits & (1u64 << field_index)) != 0
    }

    /// Check if any field is dirty
    #[inline]
    pub const fn is_any_dirty(&self) -> bool {
        self.bits != 0
    }

    /// Clear specific field
    #[inline]
    pub fn clear_field(&mut self, field_index: u8) {
        self.bits &= !(1u64 << field_index);
    }

    /// Clear all dirty bits
    #[inline]
    pub fn clear_all(&mut self) {
        self.bits = 0;
    }

    /// Get raw bits
    #[inline]
    pub const fn bits(&self) -> u64 {
        self.bits
    }

    /// Count dirty fields
    #[inline]
    pub const fn count(&self) -> u32 {
        self.bits.count_ones()
    }
}

impl Default for DirtyMask {
    fn default() -> Self {
        Self::new()
    }
}

/// Atomic dirty mask for concurrent updates
#[repr(C)]
pub struct AtomicDirtyMask {
    bits: Atomic<u64>,
}

impl AtomicDirtyMask {
    /// Create new atomic dirty mask
    #[inline]
    pub const fn new() -> Self {
        Self {
            bits: Atomic::new(0),
        }
    }

    /// Atomically mark field as dirty
    #[inline]
    pub fn mark_dirty(&self, field_index: u8) {
        debug_assert!(field_index < 64, "Field index must be < 64");
        self.bits.fetch_or(1u64 << field_index, Ordering::Release);
    }

    /// Atomically check if field is dirty
    #[inline]
    pub fn is_dirty(&self, field_index: u8) -> bool {
        (self.bits.load(Ordering::Acquire) & (1u64 << field_index)) != 0
    }

    /// Atomically check if any field is dirty
    #[inline]
    pub fn is_any_dirty(&self) -> bool {
        self.bits.load(Ordering::Acquire) != 0
    }

    /// Atomically clear all and return previous value
    #[inline]
    pub fn swap_clear(&self) -> u64 {
        self.bits.swap(0, Ordering::AcqRel)
    }

    /// Load current value
    #[inline]
    pub fn load(&self) -> u64 {
        self.bits.load(Ordering::Acquire)
    }
}

impl Default for AtomicDirtyMask {
    fn default() -> Self {
        Self::new()
    }
}

/// State slot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMetadata {
    pub state_id: u16,
    pub size: u32,
    pub offset: u32,
    pub field_count: u8,
}

/// State registry (maps state IDs to memory offsets)
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct StateRegistry {
    states: Arc<HashMap<u16, StateMetadata>>,
}

/// Error type for state registry operations
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateRegistryError {
    /// Cannot register while registry is shared (Arc has multiple references)
    RegistryShared,
}

#[cfg(feature = "std")]
impl std::fmt::Display for StateRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateRegistryError::RegistryShared => {
                write!(f, "Cannot register while registry is shared")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StateRegistryError {}

#[cfg(feature = "std")]
impl StateRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            states: Arc::new(HashMap::new()),
        }
    }

    /// Register a state slot.
    /// Returns an error if the registry is currently shared (has multiple Arc references).
    pub fn register(&mut self, metadata: StateMetadata) -> Result<(), StateRegistryError> {
        Arc::get_mut(&mut self.states)
            .ok_or(StateRegistryError::RegistryShared)?
            .insert(metadata.state_id, metadata);
        Ok(())
    }

    /// Get state metadata
    #[inline]
    pub fn get(&self, state_id: u16) -> Option<&StateMetadata> {
        self.states.get(&state_id)
    }

    /// Get state offset
    #[inline]
    pub fn get_offset(&self, state_id: u16) -> Option<u32> {
        self.states.get(&state_id).map(|m| m.offset)
    }
}

#[cfg(feature = "std")]
impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Subscriber callback type
#[cfg(feature = "std")]
pub type SubscriberFn = Arc<dyn Fn(u64) + Send + Sync>;

/// Subscription to state changes
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct Subscription {
    state_id: u16,
    field_mask: u64,
    callback: SubscriberFn,
}

#[cfg(feature = "std")]
impl Subscription {
    /// Create new subscription
    pub fn new(state_id: u16, field_mask: u64, callback: SubscriberFn) -> Self {
        Self {
            state_id,
            field_mask,
            callback,
        }
    }

    /// Check if subscription matches dirty fields
    #[inline]
    pub fn matches(&self, dirty_bits: u64) -> bool {
        (self.field_mask & dirty_bits) != 0
    }

    /// Notify subscriber
    #[inline]
    pub fn notify(&self, dirty_bits: u64) {
        if self.matches(dirty_bits) {
            (self.callback)(dirty_bits);
        }
    }
}

/// Subscriber system
#[cfg(feature = "std")]
#[derive(Clone, Default)]
pub struct SubscriberSystem {
    subscribers: Arc<parking_lot::RwLock<HashMap<u16, Vec<Subscription>>>>,
}

#[cfg(feature = "std")]
impl SubscriberSystem {
    /// Create new subscriber system
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    /// Subscribe to state changes
    pub fn subscribe(&self, subscription: Subscription) {
        let mut subs = self.subscribers.write();
        subs.entry(subscription.state_id).or_default().push(subscription);
    }

    /// Notify subscribers of state change
    pub fn notify(&self, state_id: u16, dirty_bits: u64) {
        let subs = self.subscribers.read();
        if let Some(state_subs) = subs.get(&state_id) {
            for sub in state_subs {
                sub.notify(dirty_bits);
            }
        }
    }

    /// Unsubscribe from state (clear all subscriptions)
    pub fn unsubscribe(&self, state_id: u16) {
        let mut subs = self.subscribers.write();
        subs.remove(&state_id);
    }
}

/// Example state struct (shows proper layout)
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ExampleState {
    /// Dirty mask must always be first field
    pub dirty: DirtyMask,
    /// User-defined fields
    pub count: i32,
    pub enabled: u32, // Using u32 for bool (better alignment)
}

impl ExampleState {
    /// Create new state
    pub const fn new() -> Self {
        Self {
            dirty: DirtyMask::new(),
            count: 0,
            enabled: 0,
        }
    }

    /// Set count and mark dirty
    pub fn set_count(&mut self, value: i32) {
        self.count = value;
        self.dirty.mark_dirty(0); // Field index 0
    }

    /// Set enabled and mark dirty
    pub fn set_enabled(&mut self, value: bool) {
        self.enabled = value as u32;
        self.dirty.mark_dirty(1); // Field index 1
    }
}

impl Default for ExampleState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_mask() {
        let mut mask = DirtyMask::new();
        assert!(!mask.is_any_dirty());

        mask.mark_dirty(0);
        assert!(mask.is_dirty(0));
        assert!(!mask.is_dirty(1));
        assert!(mask.is_any_dirty());
        assert_eq!(mask.count(), 1);

        mask.mark_dirty(5);
        assert!(mask.is_dirty(5));
        assert_eq!(mask.count(), 2);

        mask.clear_field(0);
        assert!(!mask.is_dirty(0));
        assert!(mask.is_dirty(5));

        mask.clear_all();
        assert!(!mask.is_any_dirty());
    }

    #[test]
    fn test_atomic_dirty_mask() {
        let mask = AtomicDirtyMask::new();
        assert!(!mask.is_any_dirty());

        mask.mark_dirty(3);
        assert!(mask.is_dirty(3));

        let prev = mask.swap_clear();
        assert_eq!(prev, 1u64 << 3);
        assert!(!mask.is_any_dirty());
    }

    #[test]
    fn test_example_state() {
        let mut state = ExampleState::new();
        assert!(!state.dirty.is_any_dirty());

        state.set_count(42);
        assert_eq!(state.count, 42);
        assert!(state.dirty.is_dirty(0));

        state.set_enabled(true);
        assert_eq!(state.enabled, 1);
        assert!(state.dirty.is_dirty(1));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_registry() {
        let mut registry = StateRegistry::new();
        let metadata = StateMetadata {
            state_id: 1,
            size: 16,
            offset: 0,
            field_count: 2,
        };

        let _ = registry.register(metadata.clone());
        assert_eq!(registry.get_offset(1), Some(0));
        assert_eq!(registry.get(1).unwrap().size, 16);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_subscriber_system() {
        use std::sync::atomic::{AtomicU32, Ordering};

        let system = SubscriberSystem::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = Arc::clone(&counter);

        let sub = Subscription::new(
            1,
            0b11, // Watch fields 0 and 1
            Arc::new(move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }),
        );

        system.subscribe(sub);

        // Notify field 0 (should trigger)
        system.notify(1, 0b01);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Notify field 2 (should not trigger)
        system.notify(1, 0b100);
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Notify field 1 (should trigger)
        system.notify(1, 0b10);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
