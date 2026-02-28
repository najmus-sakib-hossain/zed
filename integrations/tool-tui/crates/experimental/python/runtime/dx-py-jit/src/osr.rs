//! On-Stack Replacement (OSR) for hot loop optimization

use crate::compiler::{FunctionId, ValueLocation};
use dashmap::DashMap;
use std::sync::Arc;

/// OSR entry point information
#[derive(Debug, Clone)]
pub struct OsrEntry {
    /// Pointer to compiled code entry point
    pub entry_point: *const u8,
    /// Frame layout for OSR entry
    pub frame_layout: Vec<ValueLocation>,
    /// Bytecode offset this entry corresponds to
    pub bytecode_offset: usize,
}

// Safety: OsrEntry is Send + Sync because entry_point points to
// immutable executable memory
unsafe impl Send for OsrEntry {}
unsafe impl Sync for OsrEntry {}

/// OSR Manager for hot loop optimization
pub struct OsrManager {
    /// OSR entries keyed by (function_id, loop_header_offset)
    osr_entries: DashMap<(FunctionId, usize), Arc<OsrEntry>>,
    /// Minimum loop iterations before OSR
    min_iterations: u64,
}

impl OsrManager {
    /// Create a new OSR manager
    pub fn new() -> Self {
        Self {
            osr_entries: DashMap::new(),
            min_iterations: 1000,
        }
    }

    /// Create with custom iteration threshold
    pub fn with_threshold(min_iterations: u64) -> Self {
        Self {
            osr_entries: DashMap::new(),
            min_iterations,
        }
    }

    /// Check if a loop is hot enough for OSR
    pub fn is_hot(&self, iteration_count: u64) -> bool {
        iteration_count >= self.min_iterations
    }

    /// Get an existing OSR entry
    pub fn get_entry(&self, func_id: FunctionId, loop_header: usize) -> Option<Arc<OsrEntry>> {
        self.osr_entries.get(&(func_id, loop_header)).map(|r| r.clone())
    }

    /// Register an OSR entry
    pub fn register_entry(&self, func_id: FunctionId, loop_header: usize, entry: OsrEntry) {
        self.osr_entries.insert((func_id, loop_header), Arc::new(entry));
    }

    /// Remove an OSR entry (e.g., when function is invalidated)
    pub fn remove_entry(&self, func_id: FunctionId, loop_header: usize) {
        self.osr_entries.remove(&(func_id, loop_header));
    }

    /// Remove all OSR entries for a function
    pub fn remove_function(&self, func_id: FunctionId) {
        self.osr_entries.retain(|(fid, _), _| *fid != func_id);
    }

    /// Get the number of registered OSR entries
    pub fn entry_count(&self) -> usize {
        self.osr_entries.len()
    }
}

impl Default for OsrManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Frame state snapshot for OSR entry
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    /// Local variable values
    pub locals: Vec<u64>,
    /// Stack values
    pub stack: Vec<u64>,
    /// Current instruction pointer (bytecode offset)
    pub ip: usize,
}

impl FrameSnapshot {
    /// Create a new frame snapshot
    pub fn new(locals: Vec<u64>, stack: Vec<u64>, ip: usize) -> Self {
        Self { locals, stack, ip }
    }

    /// Get a local variable value
    pub fn get_local(&self, index: usize) -> Option<u64> {
        self.locals.get(index).copied()
    }

    /// Get a stack value (0 = top of stack)
    pub fn get_stack(&self, depth: usize) -> Option<u64> {
        if depth < self.stack.len() {
            Some(self.stack[self.stack.len() - 1 - depth])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osr_manager() {
        let manager = OsrManager::new();
        let func_id = FunctionId(1);

        assert!(manager.get_entry(func_id, 10).is_none());

        let entry = OsrEntry {
            entry_point: std::ptr::null(),
            frame_layout: vec![ValueLocation::Register(0)],
            bytecode_offset: 10,
        };

        manager.register_entry(func_id, 10, entry);

        assert!(manager.get_entry(func_id, 10).is_some());
        assert_eq!(manager.entry_count(), 1);

        manager.remove_entry(func_id, 10);
        assert!(manager.get_entry(func_id, 10).is_none());
    }

    #[test]
    fn test_is_hot() {
        let manager = OsrManager::with_threshold(100);

        assert!(!manager.is_hot(50));
        assert!(!manager.is_hot(99));
        assert!(manager.is_hot(100));
        assert!(manager.is_hot(1000));
    }

    #[test]
    fn test_frame_snapshot() {
        let snapshot = FrameSnapshot::new(vec![1, 2, 3], vec![10, 20, 30], 42);

        assert_eq!(snapshot.get_local(0), Some(1));
        assert_eq!(snapshot.get_local(2), Some(3));
        assert_eq!(snapshot.get_local(5), None);

        assert_eq!(snapshot.get_stack(0), Some(30)); // Top
        assert_eq!(snapshot.get_stack(2), Some(10)); // Bottom
        assert_eq!(snapshot.get_stack(5), None);

        assert_eq!(snapshot.ip, 42);
    }

    #[test]
    fn test_remove_function() {
        let manager = OsrManager::new();
        let func1 = FunctionId(1);
        let func2 = FunctionId(2);

        let entry = OsrEntry {
            entry_point: std::ptr::null(),
            frame_layout: vec![],
            bytecode_offset: 0,
        };

        manager.register_entry(func1, 10, entry.clone());
        manager.register_entry(func1, 20, entry.clone());
        manager.register_entry(func2, 10, entry);

        assert_eq!(manager.entry_count(), 3);

        manager.remove_function(func1);

        assert_eq!(manager.entry_count(), 1);
        assert!(manager.get_entry(func1, 10).is_none());
        assert!(manager.get_entry(func2, 10).is_some());
    }
}
