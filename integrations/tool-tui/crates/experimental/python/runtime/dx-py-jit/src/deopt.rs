//! Deoptimization Infrastructure
//!
//! This module provides the infrastructure for deoptimizing from optimized
//! JIT code back to the interpreter when type assumptions are violated.
//!
//! ## Deoptimization Flow
//!
//! 1. Optimized code detects a type guard failure or other deopt condition
//! 2. The deopt handler is called with the current frame state
//! 3. Frame state is reconstructed from the deopt info
//! 4. Execution resumes in the interpreter at the correct bytecode offset
//!
//! ## Type Guards
//!
//! Type guards are runtime checks inserted into JIT-compiled code to verify
//! that values have the expected types. When a guard fails:
//!
//! 1. The guard calls the deoptimization handler
//! 2. The handler saves the current state (registers, stack)
//! 3. Control transfers back to the interpreter at the correct bytecode offset
//! 4. The interpreter continues execution with the saved state
//!
//! ## Requirements Validated
//!
//! - Requirement 7.3: WHEN a type guard fails in JIT code, THE Runtime SHALL
//!   deoptimize back to the interpreter
//! - Requirement 7.4: WHEN JIT compilation fails, THE Runtime SHALL fall back
//!   to interpretation without crashing

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::optimizing::DeoptReason;
use crate::FunctionId;

/// Value location in the deoptimization frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueLocation {
    /// Value is in a register
    Register(u8),
    /// Value is on the stack at an offset from the frame pointer
    Stack(i32),
    /// Value is a constant
    Constant(i64),
    /// Value is in a local variable slot
    Local(u16),
    /// Value is undefined/uninitialized
    Undefined,
}

/// Information about a single value in the deopt frame
#[derive(Debug, Clone)]
pub struct DeoptValue {
    /// Where the value is located
    pub location: ValueLocation,
    /// Python type tag (if known)
    pub type_tag: Option<u8>,
    /// Whether this value is boxed
    pub is_boxed: bool,
}

impl DeoptValue {
    /// Create a new deopt value from a register
    pub fn from_register(reg: u8) -> Self {
        Self {
            location: ValueLocation::Register(reg),
            type_tag: None,
            is_boxed: true,
        }
    }

    /// Create a new deopt value from a stack slot
    pub fn from_stack(offset: i32) -> Self {
        Self {
            location: ValueLocation::Stack(offset),
            type_tag: None,
            is_boxed: true,
        }
    }

    /// Create a new deopt value from a constant
    pub fn from_constant(value: i64) -> Self {
        Self {
            location: ValueLocation::Constant(value),
            type_tag: None,
            is_boxed: false,
        }
    }

    /// Create a new deopt value from a local variable
    pub fn from_local(slot: u16) -> Self {
        Self {
            location: ValueLocation::Local(slot),
            type_tag: None,
            is_boxed: true,
        }
    }

    /// Create an undefined deopt value
    pub fn undefined() -> Self {
        Self {
            location: ValueLocation::Undefined,
            type_tag: None,
            is_boxed: false,
        }
    }

    /// Set the type tag
    pub fn with_type_tag(mut self, tag: u8) -> Self {
        self.type_tag = Some(tag);
        self
    }

    /// Set whether the value is boxed
    pub fn with_boxed(mut self, boxed: bool) -> Self {
        self.is_boxed = boxed;
        self
    }
}

/// Frame state at a deoptimization point
#[derive(Debug, Clone)]
pub struct DeoptFrameState {
    /// Bytecode offset to resume at
    pub bytecode_offset: u32,
    /// Values on the operand stack (bottom to top)
    pub stack: Vec<DeoptValue>,
    /// Local variable values
    pub locals: Vec<DeoptValue>,
    /// Reason for deoptimization
    pub reason: DeoptReason,
    /// Native code offset where deopt occurred
    pub native_offset: u32,
}

impl DeoptFrameState {
    /// Create a new deopt frame state
    pub fn new(bytecode_offset: u32, reason: DeoptReason) -> Self {
        Self {
            bytecode_offset,
            stack: Vec::new(),
            locals: Vec::new(),
            reason,
            native_offset: 0,
        }
    }

    /// Set the native code offset
    pub fn with_native_offset(mut self, offset: u32) -> Self {
        self.native_offset = offset;
        self
    }

    /// Add a stack value
    pub fn push_stack(&mut self, value: DeoptValue) {
        self.stack.push(value);
    }

    /// Add a local variable value
    pub fn set_local(&mut self, index: usize, value: DeoptValue) {
        if index >= self.locals.len() {
            self.locals.resize(index + 1, DeoptValue::undefined());
        }
        self.locals[index] = value;
    }

    /// Get the stack depth
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }

    /// Get the number of locals
    pub fn num_locals(&self) -> usize {
        self.locals.len()
    }
}

/// Deoptimization metadata for a compiled function
#[derive(Debug, Clone)]
pub struct DeoptMetadata {
    /// Function ID
    pub func_id: FunctionId,
    /// Deopt points indexed by native code offset
    pub deopt_points: HashMap<u32, DeoptFrameState>,
    /// Total number of deoptimizations
    pub deopt_count: u32,
    /// Maximum allowed deoptimizations before giving up on optimization
    pub max_deopts: u32,
}

impl DeoptMetadata {
    /// Create new deopt metadata for a function
    pub fn new(func_id: FunctionId) -> Self {
        Self {
            func_id,
            deopt_points: HashMap::new(),
            deopt_count: 0,
            max_deopts: 10, // Default: give up after 10 deopts
        }
    }

    /// Set the maximum number of allowed deoptimizations
    pub fn with_max_deopts(mut self, max: u32) -> Self {
        self.max_deopts = max;
        self
    }

    /// Register a deopt point
    pub fn register_deopt_point(&mut self, native_offset: u32, frame_state: DeoptFrameState) {
        self.deopt_points.insert(native_offset, frame_state);
    }

    /// Get the frame state for a deopt point
    pub fn get_frame_state(&self, native_offset: u32) -> Option<&DeoptFrameState> {
        self.deopt_points.get(&native_offset)
    }

    /// Record a deoptimization
    pub fn record_deopt(&mut self) -> bool {
        self.deopt_count += 1;
        self.deopt_count <= self.max_deopts
    }

    /// Check if we should give up on optimization
    pub fn should_give_up(&self) -> bool {
        self.deopt_count > self.max_deopts
    }

    /// Get the deopt count
    pub fn get_deopt_count(&self) -> u32 {
        self.deopt_count
    }

    /// Reset the deopt count (e.g., after recompilation)
    pub fn reset_deopt_count(&mut self) {
        self.deopt_count = 0;
    }
}

/// Deoptimization manager for all compiled functions
pub struct DeoptManager {
    /// Deopt metadata for each function
    metadata: HashMap<FunctionId, DeoptMetadata>,
    /// Global deopt statistics
    total_deopts: u64,
    /// Deopt counts by reason
    deopts_by_reason: HashMap<DeoptReason, u64>,
}

impl DeoptManager {
    /// Create a new deopt manager
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            total_deopts: 0,
            deopts_by_reason: HashMap::new(),
        }
    }

    /// Register deopt metadata for a function
    pub fn register_function(&mut self, func_id: FunctionId, metadata: DeoptMetadata) {
        self.metadata.insert(func_id, metadata);
    }

    /// Get deopt metadata for a function
    pub fn get_metadata(&self, func_id: &FunctionId) -> Option<&DeoptMetadata> {
        self.metadata.get(func_id)
    }

    /// Get mutable deopt metadata for a function
    pub fn get_metadata_mut(&mut self, func_id: &FunctionId) -> Option<&mut DeoptMetadata> {
        self.metadata.get_mut(func_id)
    }

    /// Handle a deoptimization event
    pub fn handle_deopt(
        &mut self,
        func_id: &FunctionId,
        native_offset: u32,
    ) -> Option<DeoptResult> {
        self.total_deopts += 1;

        let metadata = self.metadata.get_mut(func_id)?;
        let frame_state = metadata.get_frame_state(native_offset)?.clone();

        // Update statistics
        *self.deopts_by_reason.entry(frame_state.reason).or_insert(0) += 1;

        // Record the deopt and check if we should give up
        let should_continue = metadata.record_deopt();

        Some(DeoptResult {
            frame_state,
            should_recompile: should_continue && metadata.deopt_count > 3,
            should_give_up: !should_continue,
        })
    }

    /// Remove deopt metadata for a function
    pub fn remove_function(&mut self, func_id: &FunctionId) {
        self.metadata.remove(func_id);
    }

    /// Get total deopt count
    pub fn get_total_deopts(&self) -> u64 {
        self.total_deopts
    }

    /// Get deopt count by reason
    pub fn get_deopts_by_reason(&self, reason: DeoptReason) -> u64 {
        self.deopts_by_reason.get(&reason).copied().unwrap_or(0)
    }

    /// Get all deopt statistics
    pub fn get_statistics(&self) -> DeoptStatistics {
        DeoptStatistics {
            total_deopts: self.total_deopts,
            functions_with_deopts: self.metadata.values().filter(|m| m.deopt_count > 0).count(),
            functions_given_up: self.metadata.values().filter(|m| m.should_give_up()).count(),
            deopts_by_reason: self.deopts_by_reason.clone(),
        }
    }
}

impl Default for DeoptManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of handling a deoptimization
#[derive(Debug, Clone)]
pub struct DeoptResult {
    /// Frame state to restore
    pub frame_state: DeoptFrameState,
    /// Whether we should recompile with updated type feedback
    pub should_recompile: bool,
    /// Whether we should give up on optimization entirely
    pub should_give_up: bool,
}

/// Deoptimization statistics
#[derive(Debug, Clone)]
pub struct DeoptStatistics {
    /// Total number of deoptimizations
    pub total_deopts: u64,
    /// Number of functions that have deoptimized at least once
    pub functions_with_deopts: usize,
    /// Number of functions that have given up on optimization
    pub functions_given_up: usize,
    /// Deopt counts by reason
    pub deopts_by_reason: HashMap<DeoptReason, u64>,
}

/// Builder for constructing deopt frame states during compilation
pub struct DeoptFrameBuilder {
    /// Current bytecode offset
    bytecode_offset: u32,
    /// Stack values
    stack: Vec<DeoptValue>,
    /// Local variable values
    locals: Vec<DeoptValue>,
    /// Number of locals
    num_locals: usize,
}

impl DeoptFrameBuilder {
    /// Create a new deopt frame builder
    pub fn new(bytecode_offset: u32, num_locals: usize) -> Self {
        Self {
            bytecode_offset,
            stack: Vec::new(),
            locals: vec![DeoptValue::undefined(); num_locals],
            num_locals,
        }
    }

    /// Push a value onto the stack
    pub fn push_stack(&mut self, value: DeoptValue) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    pub fn pop_stack(&mut self) -> Option<DeoptValue> {
        self.stack.pop()
    }

    /// Set a local variable
    pub fn set_local(&mut self, index: usize, value: DeoptValue) {
        if index < self.num_locals {
            self.locals[index] = value;
        }
    }

    /// Get a local variable
    pub fn get_local(&self, index: usize) -> Option<&DeoptValue> {
        self.locals.get(index)
    }

    /// Build the deopt frame state
    pub fn build(self, reason: DeoptReason) -> DeoptFrameState {
        DeoptFrameState {
            bytecode_offset: self.bytecode_offset,
            stack: self.stack,
            locals: self.locals,
            reason,
            native_offset: 0,
        }
    }

    /// Update the bytecode offset
    pub fn set_bytecode_offset(&mut self, offset: u32) {
        self.bytecode_offset = offset;
    }

    /// Get the current stack depth
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deopt_value_creation() {
        let reg_val = DeoptValue::from_register(5);
        assert!(matches!(reg_val.location, ValueLocation::Register(5)));
        assert!(reg_val.is_boxed);

        let stack_val = DeoptValue::from_stack(-16);
        assert!(matches!(stack_val.location, ValueLocation::Stack(-16)));

        let const_val = DeoptValue::from_constant(42);
        assert!(matches!(const_val.location, ValueLocation::Constant(42)));
        assert!(!const_val.is_boxed);

        let local_val = DeoptValue::from_local(3);
        assert!(matches!(local_val.location, ValueLocation::Local(3)));

        let undef_val = DeoptValue::undefined();
        assert!(matches!(undef_val.location, ValueLocation::Undefined));
    }

    #[test]
    fn test_deopt_frame_state() {
        let mut frame = DeoptFrameState::new(100, DeoptReason::TypeGuardFailed);

        frame.push_stack(DeoptValue::from_register(0));
        frame.push_stack(DeoptValue::from_register(1));
        frame.set_local(0, DeoptValue::from_local(0));
        frame.set_local(2, DeoptValue::from_local(2));

        assert_eq!(frame.bytecode_offset, 100);
        assert_eq!(frame.stack_depth(), 2);
        assert_eq!(frame.num_locals(), 3);
        assert_eq!(frame.reason, DeoptReason::TypeGuardFailed);
    }

    #[test]
    fn test_deopt_metadata() {
        let func_id = FunctionId(1);
        let mut metadata = DeoptMetadata::new(func_id);

        let frame_state = DeoptFrameState::new(50, DeoptReason::IntegerOverflow);
        metadata.register_deopt_point(1000, frame_state);

        assert!(metadata.get_frame_state(1000).is_some());
        assert!(metadata.get_frame_state(2000).is_none());

        // Test deopt counting
        for _ in 0..10 {
            assert!(metadata.record_deopt());
        }
        assert!(!metadata.record_deopt()); // 11th deopt should fail
        assert!(metadata.should_give_up());
    }

    #[test]
    fn test_deopt_manager() {
        let mut manager = DeoptManager::new();
        let func_id = FunctionId(1);

        let mut metadata = DeoptMetadata::new(func_id);
        let frame_state = DeoptFrameState::new(50, DeoptReason::TypeGuardFailed);
        metadata.register_deopt_point(1000, frame_state);
        manager.register_function(func_id, metadata);

        // Handle a deopt
        let result = manager.handle_deopt(&func_id, 1000);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.frame_state.bytecode_offset, 50);
        assert!(!result.should_give_up);

        // Check statistics
        assert_eq!(manager.get_total_deopts(), 1);
        assert_eq!(manager.get_deopts_by_reason(DeoptReason::TypeGuardFailed), 1);
    }

    #[test]
    fn test_deopt_frame_builder() {
        let mut builder = DeoptFrameBuilder::new(100, 5);

        builder.push_stack(DeoptValue::from_register(0));
        builder.push_stack(DeoptValue::from_register(1));
        builder.set_local(0, DeoptValue::from_local(0));
        builder.set_local(2, DeoptValue::from_constant(42));

        assert_eq!(builder.stack_depth(), 2);

        let frame = builder.build(DeoptReason::DivisionByZero);
        assert_eq!(frame.bytecode_offset, 100);
        assert_eq!(frame.stack.len(), 2);
        assert_eq!(frame.locals.len(), 5);
        assert_eq!(frame.reason, DeoptReason::DivisionByZero);
    }

    #[test]
    fn test_deopt_statistics() {
        let mut manager = DeoptManager::new();

        // Add two functions
        let func1 = FunctionId(1);
        let func2 = FunctionId(2);

        let mut meta1 = DeoptMetadata::new(func1);
        meta1.register_deopt_point(100, DeoptFrameState::new(10, DeoptReason::TypeGuardFailed));
        manager.register_function(func1, meta1);

        let mut meta2 = DeoptMetadata::new(func2).with_max_deopts(2);
        meta2.register_deopt_point(200, DeoptFrameState::new(20, DeoptReason::IntegerOverflow));
        manager.register_function(func2, meta2);

        // Trigger deopts
        manager.handle_deopt(&func1, 100);
        manager.handle_deopt(&func2, 200);
        manager.handle_deopt(&func2, 200);
        manager.handle_deopt(&func2, 200); // This should cause func2 to give up

        let stats = manager.get_statistics();
        assert_eq!(stats.total_deopts, 4);
        assert_eq!(stats.functions_with_deopts, 2);
        assert_eq!(stats.functions_given_up, 1);
    }

    #[test]
    fn test_type_guard() {
        let guard = TypeGuard::new(TypeGuardKind::IsInt, 100, 50);
        assert_eq!(guard.kind, TypeGuardKind::IsInt);
        assert_eq!(guard.native_offset, 100);
        assert_eq!(guard.bytecode_offset, 50);
        assert!(!guard.is_failed());

        guard.mark_failed();
        assert!(guard.is_failed());
    }

    #[test]
    fn test_deopt_handler() {
        let handler = DeoptHandler::new();
        let func_id = FunctionId(42);

        // Initially no deopt pending
        assert!(!handler.is_deopt_pending());

        // Trigger deopt
        handler.trigger_deopt(func_id, 100, DeoptReason::TypeGuardFailed);
        assert!(handler.is_deopt_pending());

        // Get pending deopt
        let pending = handler.get_pending_deopt();
        assert!(pending.is_some());
        let (fid, offset, reason) = pending.unwrap();
        assert_eq!(fid, func_id);
        assert_eq!(offset, 100);
        assert_eq!(reason, DeoptReason::TypeGuardFailed);

        // Clear deopt
        handler.clear_deopt();
        assert!(!handler.is_deopt_pending());
    }

    #[test]
    fn test_interpreter_fallback_state() {
        let state = InterpreterFallbackState::new(FunctionId(1), 50);
        assert_eq!(state.func_id, FunctionId(1));
        assert_eq!(state.bytecode_offset, 50);
        assert!(state.stack_values.is_empty());
        assert!(state.local_values.is_empty());

        let state_with_values = InterpreterFallbackState::with_state(
            FunctionId(2),
            100,
            vec![42, 43],
            vec![1, 2, 3],
        );
        assert_eq!(state_with_values.stack_values, vec![42, 43]);
        assert_eq!(state_with_values.local_values, vec![1, 2, 3]);
    }
}

// =============================================================================
// Type Guards for Speculative Optimizations
// =============================================================================

/// Type guard kinds for runtime type checking
///
/// Type guards are inserted into JIT-compiled code to verify that values
/// have the expected types. When a guard fails, the code deoptimizes back
/// to the interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeGuardKind {
    /// Guard that value is an integer
    IsInt,
    /// Guard that value is a float
    IsFloat,
    /// Guard that value is a string
    IsString,
    /// Guard that value is a list
    IsList,
    /// Guard that value is a dict
    IsDict,
    /// Guard that value is a tuple
    IsTuple,
    /// Guard that value is not None
    IsNotNone,
    /// Guard that value is a boolean
    IsBool,
    /// Guard that value is callable
    IsCallable,
    /// Guard that value is iterable
    IsIterable,
    /// Guard for specific class type
    IsInstance,
    /// Guard that integer is within safe range (no overflow)
    IntInRange,
    /// Guard that array index is in bounds
    IndexInBounds,
}

/// A type guard inserted into JIT-compiled code
///
/// Type guards verify runtime assumptions made during compilation.
/// When a guard fails, the JIT code triggers deoptimization.
#[derive(Debug)]
pub struct TypeGuard {
    /// The kind of type check
    pub kind: TypeGuardKind,
    /// Offset in native code where the guard is located
    pub native_offset: u32,
    /// Bytecode offset to resume at if guard fails
    pub bytecode_offset: u32,
    /// Whether this guard has failed (for profiling)
    failed: AtomicBool,
    /// Number of times this guard has been checked
    check_count: AtomicU64,
    /// Number of times this guard has failed
    fail_count: AtomicU64,
}

impl Clone for TypeGuard {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind.clone(),
            native_offset: self.native_offset,
            bytecode_offset: self.bytecode_offset,
            failed: AtomicBool::new(self.failed.load(Ordering::Relaxed)),
            check_count: AtomicU64::new(self.check_count.load(Ordering::Relaxed)),
            fail_count: AtomicU64::new(self.fail_count.load(Ordering::Relaxed)),
        }
    }
}

impl TypeGuard {
    /// Create a new type guard
    pub fn new(kind: TypeGuardKind, native_offset: u32, bytecode_offset: u32) -> Self {
        Self {
            kind,
            native_offset,
            bytecode_offset,
            failed: AtomicBool::new(false),
            check_count: AtomicU64::new(0),
            fail_count: AtomicU64::new(0),
        }
    }

    /// Record a guard check
    pub fn record_check(&self) {
        self.check_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a guard failure
    pub fn record_failure(&self) {
        self.fail_count.fetch_add(1, Ordering::Relaxed);
        self.failed.store(true, Ordering::Relaxed);
    }

    /// Mark this guard as failed
    pub fn mark_failed(&self) {
        self.failed.store(true, Ordering::Relaxed);
    }

    /// Check if this guard has ever failed
    pub fn is_failed(&self) -> bool {
        self.failed.load(Ordering::Relaxed)
    }

    /// Get the failure rate (0.0 to 1.0)
    pub fn failure_rate(&self) -> f64 {
        let checks = self.check_count.load(Ordering::Relaxed);
        let fails = self.fail_count.load(Ordering::Relaxed);
        if checks == 0 {
            0.0
        } else {
            fails as f64 / checks as f64
        }
    }

    /// Get the check count
    pub fn get_check_count(&self) -> u64 {
        self.check_count.load(Ordering::Relaxed)
    }

    /// Get the fail count
    pub fn get_fail_count(&self) -> u64 {
        self.fail_count.load(Ordering::Relaxed)
    }
}

// =============================================================================
// Deoptimization Handler
// =============================================================================

/// Handler for deoptimization events from JIT code
///
/// This handler is called when a type guard fails or other deoptimization
/// condition is detected. It coordinates the transition from JIT code back
/// to the interpreter.
///
/// ## Thread Safety
///
/// The handler uses atomic operations for thread-safe deoptimization tracking.
/// Each thread should have its own handler instance for per-thread deopt state.
pub struct DeoptHandler {
    /// Whether a deoptimization is pending
    deopt_pending: AtomicBool,
    /// Function ID that triggered deopt
    deopt_func_id: AtomicU64,
    /// Native code offset where deopt occurred
    deopt_offset: AtomicU64,
    /// Reason for deoptimization
    deopt_reason: std::sync::atomic::AtomicU8,
}

impl DeoptHandler {
    /// Create a new deoptimization handler
    pub fn new() -> Self {
        Self {
            deopt_pending: AtomicBool::new(false),
            deopt_func_id: AtomicU64::new(0),
            deopt_offset: AtomicU64::new(0),
            deopt_reason: std::sync::atomic::AtomicU8::new(0),
        }
    }

    /// Trigger a deoptimization
    ///
    /// This is called from JIT code when a type guard fails or other
    /// deoptimization condition is detected.
    pub fn trigger_deopt(&self, func_id: FunctionId, native_offset: u32, reason: DeoptReason) {
        self.deopt_func_id.store(func_id.0, Ordering::SeqCst);
        self.deopt_offset.store(native_offset as u64, Ordering::SeqCst);
        self.deopt_reason.store(reason as u8, Ordering::SeqCst);
        self.deopt_pending.store(true, Ordering::SeqCst);
    }

    /// Check if a deoptimization is pending
    pub fn is_deopt_pending(&self) -> bool {
        self.deopt_pending.load(Ordering::SeqCst)
    }

    /// Get the pending deoptimization info
    ///
    /// Returns (func_id, native_offset, reason) if a deopt is pending
    pub fn get_pending_deopt(&self) -> Option<(FunctionId, u32, DeoptReason)> {
        if self.is_deopt_pending() {
            let func_id = FunctionId(self.deopt_func_id.load(Ordering::SeqCst));
            let offset = self.deopt_offset.load(Ordering::SeqCst) as u32;
            let reason_byte = self.deopt_reason.load(Ordering::SeqCst);
            let reason = match reason_byte {
                0 => DeoptReason::TypeGuardFailed,
                1 => DeoptReason::IntegerOverflow,
                2 => DeoptReason::DivisionByZero,
                3 => DeoptReason::UnexpectedNone,
                4 => DeoptReason::BoundsCheckFailed,
                _ => DeoptReason::TypeGuardFailed,
            };
            Some((func_id, offset, reason))
        } else {
            None
        }
    }

    /// Clear the pending deoptimization
    pub fn clear_deopt(&self) {
        self.deopt_pending.store(false, Ordering::SeqCst);
    }
}

impl Default for DeoptHandler {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Interpreter Fallback State
// =============================================================================

/// State needed to resume execution in the interpreter after deoptimization
///
/// When JIT code deoptimizes, this structure captures all the state needed
/// to continue execution in the interpreter from the correct point.
#[derive(Debug, Clone)]
pub struct InterpreterFallbackState {
    /// Function that was being executed
    pub func_id: FunctionId,
    /// Bytecode offset to resume at
    pub bytecode_offset: u32,
    /// Values on the operand stack (as raw i64 values)
    pub stack_values: Vec<i64>,
    /// Local variable values (as raw i64 values)
    pub local_values: Vec<i64>,
    /// Reason for fallback
    pub reason: DeoptReason,
}

impl InterpreterFallbackState {
    /// Create a new fallback state
    pub fn new(func_id: FunctionId, bytecode_offset: u32) -> Self {
        Self {
            func_id,
            bytecode_offset,
            stack_values: Vec::new(),
            local_values: Vec::new(),
            reason: DeoptReason::TypeGuardFailed,
        }
    }

    /// Create a fallback state with stack and local values
    pub fn with_state(
        func_id: FunctionId,
        bytecode_offset: u32,
        stack_values: Vec<i64>,
        local_values: Vec<i64>,
    ) -> Self {
        Self {
            func_id,
            bytecode_offset,
            stack_values,
            local_values,
            reason: DeoptReason::TypeGuardFailed,
        }
    }

    /// Set the deoptimization reason
    pub fn with_reason(mut self, reason: DeoptReason) -> Self {
        self.reason = reason;
        self
    }
}

// =============================================================================
// Runtime Helper Functions for Type Guards
// =============================================================================

/// Type tag constants matching PyType enum values
pub mod type_tags {
    pub const UNKNOWN: u8 = 0;
    pub const NONE: u8 = 1;
    pub const BOOL: u8 = 2;
    pub const INT: u8 = 3;
    pub const FLOAT: u8 = 4;
    pub const STR: u8 = 5;
    pub const BYTES: u8 = 6;
    pub const LIST: u8 = 7;
    pub const TUPLE: u8 = 8;
    pub const DICT: u8 = 9;
    pub const SET: u8 = 10;
}

/// Runtime helper: Check if a value is an integer
///
/// This function is called from JIT code to verify type assumptions.
/// Returns 1 if the value is an integer, 0 otherwise.
///
/// # Safety
///
/// The `value` parameter is expected to be a pointer to a Python object
/// or a tagged value. The caller must ensure the value is valid.
#[no_mangle]
pub extern "C" fn rt_type_guard_is_int(value: i64) -> i64 {
    // In a real implementation, we'd extract the type tag from the object header
    // For now, we use a simple heuristic: check if it looks like a tagged integer
    // (In production, this would check the actual PyObject type field)
    
    // Placeholder implementation - assumes tagged pointer scheme
    // where low bits indicate type
    let type_tag = (value & 0xFF) as u8;
    if type_tag == type_tags::INT || type_tag == type_tags::BOOL {
        1
    } else {
        0
    }
}

/// Runtime helper: Check if a value is a float
#[no_mangle]
pub extern "C" fn rt_type_guard_is_float(value: i64) -> i64 {
    let type_tag = (value & 0xFF) as u8;
    if type_tag == type_tags::FLOAT {
        1
    } else {
        0
    }
}

/// Runtime helper: Check if a value is a string
#[no_mangle]
pub extern "C" fn rt_type_guard_is_string(value: i64) -> i64 {
    let type_tag = (value & 0xFF) as u8;
    if type_tag == type_tags::STR {
        1
    } else {
        0
    }
}

/// Runtime helper: Check if a value is not None
#[no_mangle]
pub extern "C" fn rt_type_guard_is_not_none(value: i64) -> i64 {
    let type_tag = (value & 0xFF) as u8;
    if type_tag != type_tags::NONE && value != 0 {
        1
    } else {
        0
    }
}

/// Runtime helper: Trigger deoptimization
///
/// This function is called from JIT code when a type guard fails.
/// It sets up the deoptimization state and returns a signal value
/// that tells the JIT code to exit and fall back to the interpreter.
///
/// # Parameters
///
/// - `func_id`: The function ID (as u64)
/// - `bytecode_offset`: The bytecode offset to resume at
/// - `reason`: The deoptimization reason (as u8)
///
/// # Returns
///
/// Returns a non-zero value to signal that deoptimization should occur.
/// The JIT code should check this return value and exit if non-zero.
#[no_mangle]
pub extern "C" fn rt_trigger_deopt(func_id: u64, bytecode_offset: u32, reason: u8) -> i64 {
    // In a real implementation, this would:
    // 1. Access thread-local deopt handler
    // 2. Set up the deopt state
    // 3. Return a signal to exit JIT code
    
    // For now, we just log and return the signal
    #[cfg(debug_assertions)]
    eprintln!(
        "[DEOPT] Function {} at bytecode offset {}, reason: {}",
        func_id, bytecode_offset, reason
    );
    
    // Return non-zero to signal deoptimization
    1
}

/// Runtime helper: Check and deoptimize if type guard fails
///
/// This is a combined check-and-deopt helper that can be called from JIT code.
/// It checks the type and triggers deoptimization if the check fails.
///
/// # Parameters
///
/// - `value`: The value to check
/// - `expected_type`: The expected type tag
/// - `func_id`: The function ID
/// - `bytecode_offset`: The bytecode offset to resume at
///
/// # Returns
///
/// Returns 0 if the type matches (continue execution), or non-zero if
/// deoptimization should occur.
#[no_mangle]
pub extern "C" fn rt_check_type_or_deopt(
    value: i64,
    expected_type: u8,
    func_id: u64,
    bytecode_offset: u32,
) -> i64 {
    let actual_type = (value & 0xFF) as u8;
    
    if actual_type == expected_type {
        0 // Type matches, continue
    } else {
        // Type mismatch, trigger deoptimization
        rt_trigger_deopt(func_id, bytecode_offset, DeoptReason::TypeGuardFailed as u8)
    }
}
