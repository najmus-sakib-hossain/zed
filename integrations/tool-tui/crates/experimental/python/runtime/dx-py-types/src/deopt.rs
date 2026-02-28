//! Deoptimization handler for JIT bailout

use dashmap::DashMap;
use dx_py_jit::compiler::{FunctionId, ValueLocation};
use std::sync::atomic::{AtomicU64, Ordering};

/// Deoptimization information for a code address
#[derive(Debug, Clone)]
pub struct DeoptInfo {
    /// Function this deopt point belongs to
    pub func_id: FunctionId,
    /// Bytecode offset to resume at
    pub bytecode_offset: usize,
    /// Locations of live values
    pub value_locations: Vec<ValueLocation>,
    /// Reason for deoptimization
    pub reason: DeoptReason,
}

/// Reason for deoptimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeoptReason {
    /// Type guard failed
    TypeGuardFailed,
    /// Overflow in arithmetic
    Overflow,
    /// Division by zero
    DivisionByZero,
    /// Null pointer dereference
    NullPointer,
    /// Array bounds check failed
    BoundsCheck,
    /// Unknown reason
    Unknown,
}

/// Deoptimization handler
pub struct DeoptHandler {
    /// Map from deopt point (code address) to deopt info
    deopt_info: DashMap<usize, DeoptInfo>,
    /// Total deoptimization count
    deopt_count: AtomicU64,
    /// Deopt counts by reason
    reason_counts: [AtomicU64; 6],
}

impl DeoptHandler {
    /// Create a new deopt handler
    pub fn new() -> Self {
        Self {
            deopt_info: DashMap::new(),
            deopt_count: AtomicU64::new(0),
            reason_counts: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    /// Register a deoptimization point
    pub fn register(&self, code_addr: usize, info: DeoptInfo) {
        self.deopt_info.insert(code_addr, info);
    }

    /// Handle a deoptimization
    ///
    /// Returns the deopt info if found, None otherwise.
    pub fn deoptimize(&self, deopt_point: usize) -> Option<DeoptInfo> {
        let info = self.deopt_info.get(&deopt_point)?.clone();

        // Update statistics
        self.deopt_count.fetch_add(1, Ordering::Relaxed);
        let reason_idx = info.reason as usize;
        if reason_idx < self.reason_counts.len() {
            self.reason_counts[reason_idx].fetch_add(1, Ordering::Relaxed);
        }

        Some(info)
    }

    /// Get deopt info without triggering deoptimization
    pub fn get_info(&self, deopt_point: usize) -> Option<DeoptInfo> {
        self.deopt_info.get(&deopt_point).map(|r| r.clone())
    }

    /// Remove deopt points for a function
    pub fn remove_function(&self, func_id: FunctionId) {
        self.deopt_info.retain(|_, info| info.func_id != func_id);
    }

    /// Get total deoptimization count
    pub fn total_deopts(&self) -> u64 {
        self.deopt_count.load(Ordering::Relaxed)
    }

    /// Get deoptimization count by reason
    pub fn deopts_by_reason(&self, reason: DeoptReason) -> u64 {
        let idx = reason as usize;
        if idx < self.reason_counts.len() {
            self.reason_counts[idx].load(Ordering::Relaxed)
        } else {
            0
        }
    }

    /// Get all deopt statistics
    pub fn get_stats(&self) -> DeoptStats {
        DeoptStats {
            total: self.deopt_count.load(Ordering::Relaxed),
            type_guard_failed: self.reason_counts[0].load(Ordering::Relaxed),
            overflow: self.reason_counts[1].load(Ordering::Relaxed),
            division_by_zero: self.reason_counts[2].load(Ordering::Relaxed),
            null_pointer: self.reason_counts[3].load(Ordering::Relaxed),
            bounds_check: self.reason_counts[4].load(Ordering::Relaxed),
            unknown: self.reason_counts[5].load(Ordering::Relaxed),
        }
    }

    /// Clear all deopt points
    pub fn clear(&self) {
        self.deopt_info.clear();
    }
}

impl Default for DeoptHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Deoptimization statistics
#[derive(Debug, Clone)]
pub struct DeoptStats {
    pub total: u64,
    pub type_guard_failed: u64,
    pub overflow: u64,
    pub division_by_zero: u64,
    pub null_pointer: u64,
    pub bounds_check: u64,
    pub unknown: u64,
}

/// Frame state for deoptimization
#[derive(Debug, Clone)]
pub struct DeoptFrame {
    /// Function ID
    pub func_id: FunctionId,
    /// Bytecode offset to resume at
    pub ip: usize,
    /// Local variable values
    pub locals: Vec<u64>,
    /// Stack values
    pub stack: Vec<u64>,
}

impl DeoptFrame {
    /// Create a new deopt frame
    pub fn new(func_id: FunctionId, ip: usize) -> Self {
        Self {
            func_id,
            ip,
            locals: Vec::new(),
            stack: Vec::new(),
        }
    }

    /// Restore a value from a location
    pub fn restore_value(
        &mut self,
        location: &ValueLocation,
        registers: &[u64],
        stack: &[u64],
        constants: &[u64],
    ) {
        let value = match location {
            ValueLocation::Register(reg) => registers.get(*reg as usize).copied().unwrap_or(0),
            ValueLocation::Stack(offset) => {
                if *offset >= 0 {
                    stack.get(*offset as usize).copied().unwrap_or(0)
                } else {
                    0
                }
            }
            ValueLocation::Constant(idx) => constants.get(*idx as usize).copied().unwrap_or(0),
        };

        self.locals.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deopt_handler() {
        let handler = DeoptHandler::new();

        let info = DeoptInfo {
            func_id: FunctionId(1),
            bytecode_offset: 42,
            value_locations: vec![ValueLocation::Register(0)],
            reason: DeoptReason::TypeGuardFailed,
        };

        handler.register(0x1000, info.clone());

        let result = handler.deoptimize(0x1000);
        assert!(result.is_some());

        let deopt_info = result.unwrap();
        assert_eq!(deopt_info.func_id, FunctionId(1));
        assert_eq!(deopt_info.bytecode_offset, 42);
        assert_eq!(deopt_info.reason, DeoptReason::TypeGuardFailed);

        assert_eq!(handler.total_deopts(), 1);
        assert_eq!(handler.deopts_by_reason(DeoptReason::TypeGuardFailed), 1);
    }

    #[test]
    fn test_deopt_stats() {
        let handler = DeoptHandler::new();

        // Register and trigger various deopts
        for i in 0..5 {
            handler.register(
                i,
                DeoptInfo {
                    func_id: FunctionId(1),
                    bytecode_offset: 0,
                    value_locations: vec![],
                    reason: DeoptReason::TypeGuardFailed,
                },
            );
            handler.deoptimize(i);
        }

        for i in 5..8 {
            handler.register(
                i,
                DeoptInfo {
                    func_id: FunctionId(1),
                    bytecode_offset: 0,
                    value_locations: vec![],
                    reason: DeoptReason::Overflow,
                },
            );
            handler.deoptimize(i);
        }

        let stats = handler.get_stats();
        assert_eq!(stats.total, 8);
        assert_eq!(stats.type_guard_failed, 5);
        assert_eq!(stats.overflow, 3);
    }

    #[test]
    fn test_remove_function() {
        let handler = DeoptHandler::new();

        handler.register(
            0x1000,
            DeoptInfo {
                func_id: FunctionId(1),
                bytecode_offset: 0,
                value_locations: vec![],
                reason: DeoptReason::Unknown,
            },
        );

        handler.register(
            0x2000,
            DeoptInfo {
                func_id: FunctionId(2),
                bytecode_offset: 0,
                value_locations: vec![],
                reason: DeoptReason::Unknown,
            },
        );

        handler.remove_function(FunctionId(1));

        assert!(handler.get_info(0x1000).is_none());
        assert!(handler.get_info(0x2000).is_some());
    }
}
