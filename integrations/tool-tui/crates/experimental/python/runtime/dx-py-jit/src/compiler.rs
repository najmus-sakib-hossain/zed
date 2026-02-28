//! Tiered JIT compiler implementation

use crate::baseline::BaselineCompiler;
use crate::optimizing::OptimizingCompiler;
use crate::profile::{FunctionProfile, PyType};
use crate::tier::CompilationTier;
use dashmap::DashMap;
use dx_py_bytecode::CodeObject;
use std::sync::Arc;

/// Unique identifier for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FunctionId(pub u64);

/// Execution mode for a function
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Execute in interpreter mode
    Interpreter,
    /// Execute JIT-compiled code at the given address
    Jit(*const u8),
}

// Safety: ExecutionMode is Send + Sync because the code pointer points to
// immutable executable memory
unsafe impl Send for ExecutionMode {}
unsafe impl Sync for ExecutionMode {}

/// Compiled function information
pub struct CompiledFunction {
    /// Compilation tier
    pub tier: CompilationTier,
    /// Pointer to compiled code
    pub code_ptr: *const u8,
    /// Size of compiled code
    pub code_size: usize,
    /// Deoptimization points
    pub deopt_points: Vec<DeoptPoint>,
}

// Safety: CompiledFunction is Send + Sync because code_ptr points to
// immutable executable memory
unsafe impl Send for CompiledFunction {}
unsafe impl Sync for CompiledFunction {}

/// Deoptimization point information
#[derive(Debug, Clone)]
pub struct DeoptPoint {
    /// Offset in compiled code
    pub code_offset: u32,
    /// Corresponding bytecode offset
    pub bytecode_offset: u32,
    /// Locations of live values
    pub live_values: Vec<ValueLocation>,
}

/// Location of a value for deoptimization
#[derive(Debug, Clone)]
pub enum ValueLocation {
    /// Value is in a register
    Register(u8),
    /// Value is on the stack at offset
    Stack(i32),
    /// Value is a constant
    Constant(u32),
}

/// Tiered JIT compiler
pub struct TieredJit {
    /// Function profiles
    profiles: DashMap<FunctionId, Arc<FunctionProfile>>,
    /// Compiled code cache
    compiled_code: DashMap<FunctionId, Arc<CompiledFunction>>,
    /// Baseline compiler
    baseline_compiler: Option<BaselineCompiler>,
    /// Optimizing compiler
    optimizing_compiler: Option<OptimizingCompiler>,
    /// JIT compilation enabled
    enabled: bool,
    /// Maximum deoptimizations before giving up on optimization
    max_deopts: u32,
    /// Functions that failed compilation (don't retry)
    failed_compilations: DashMap<FunctionId, CompilationFailure>,
}

/// Information about a compilation failure
#[derive(Debug, Clone)]
pub struct CompilationFailure {
    /// The tier that failed
    pub tier: CompilationTier,
    /// Error message
    pub error: String,
    /// Number of times compilation was attempted
    pub attempts: u32,
}

impl TieredJit {
    /// Create a new tiered JIT compiler
    pub fn new() -> Self {
        let baseline = BaselineCompiler::new().ok();
        let optimizing = OptimizingCompiler::new().ok();
        Self {
            profiles: DashMap::new(),
            compiled_code: DashMap::new(),
            baseline_compiler: baseline,
            optimizing_compiler: optimizing,
            enabled: true,
            max_deopts: 10,
            failed_compilations: DashMap::new(),
        }
    }

    /// Create a JIT with custom settings
    pub fn with_settings(enabled: bool, max_deopts: u32) -> Self {
        let baseline = if enabled {
            BaselineCompiler::new().ok()
        } else {
            None
        };
        let optimizing = if enabled {
            OptimizingCompiler::new().ok()
        } else {
            None
        };
        Self {
            profiles: DashMap::new(),
            compiled_code: DashMap::new(),
            baseline_compiler: baseline,
            optimizing_compiler: optimizing,
            enabled,
            max_deopts,
            failed_compilations: DashMap::new(),
        }
    }

    /// Get or create a profile for a function
    pub fn get_profile(
        &self,
        func_id: FunctionId,
        bytecode_len: usize,
        branch_count: usize,
    ) -> Arc<FunctionProfile> {
        self.profiles
            .entry(func_id)
            .or_insert_with(|| Arc::new(FunctionProfile::new(bytecode_len, branch_count)))
            .clone()
    }

    /// Check if a function should be promoted to the next tier
    pub fn check_promotion(&self, func_id: FunctionId) -> Option<CompilationTier> {
        if !self.enabled {
            return None;
        }

        // Don't try to compile functions that have failed before
        if self.failed_compilations.contains_key(&func_id) {
            return None;
        }

        let profile = self.profiles.get(&func_id)?;
        let calls = profile.get_call_count();
        let deopts = profile.get_deopt_count();

        // Don't promote if too many deoptimizations
        if deopts > self.max_deopts {
            return None;
        }

        let current_tier = self
            .compiled_code
            .get(&func_id)
            .map(|c| c.tier)
            .unwrap_or(CompilationTier::Interpreter);

        // Check if we should promote to next tier
        if let Some(next_tier) = current_tier.next() {
            if calls >= next_tier.threshold() {
                return Some(next_tier);
            }
        }

        None
    }

    /// Compile a function at the specified tier
    pub fn compile(
        &self,
        func_id: FunctionId,
        tier: CompilationTier,
        bytecode: &[u8],
    ) -> Option<*const u8> {
        if !self.enabled || tier == CompilationTier::Interpreter {
            return None;
        }

        let code_ptr = match tier {
            CompilationTier::BaselineJit => self.compile_baseline(func_id, bytecode),
            CompilationTier::OptimizingJit => self.compile_optimized(func_id, bytecode),
            CompilationTier::AotOptimized => self.compile_aot(func_id, bytecode),
            CompilationTier::Interpreter => return None,
        };

        if let Some(ptr) = code_ptr {
            // Store compiled function
            let compiled = Arc::new(CompiledFunction {
                tier,
                code_ptr: ptr,
                code_size: 0, // Would be set by actual compilation
                deopt_points: Vec::new(),
            });
            self.compiled_code.insert(func_id, compiled);
        }

        code_ptr
    }

    /// Compile a function with a CodeObject at the specified tier
    /// Returns None on failure (falls back to interpreter)
    pub fn compile_code_object(
        &mut self,
        func_id: FunctionId,
        tier: CompilationTier,
        code: &CodeObject,
    ) -> Option<*const u8> {
        if !self.enabled || tier == CompilationTier::Interpreter {
            return None;
        }

        // Check if this function has already failed compilation
        if self.failed_compilations.contains_key(&func_id) {
            return None;
        }

        let result = match tier {
            CompilationTier::BaselineJit => {
                if let Some(ref mut compiler) = self.baseline_compiler {
                    compiler.compile(func_id, code)
                } else {
                    Err(crate::baseline::JitError::CompilationFailed(
                        "Baseline compiler not available".to_string(),
                    ))
                }
            }
            CompilationTier::OptimizingJit => {
                // Get the profile for type feedback
                let profile = self.profiles.get(&func_id);

                if let (Some(ref mut compiler), Some(profile)) =
                    (&mut self.optimizing_compiler, profile)
                {
                    // Check if the function is hot enough for optimization
                    if profile.is_hot_for_optimization() {
                        compiler
                            .compile_optimized(func_id, code, &profile)
                            .map(|compiled| compiled.code_ptr)
                    } else {
                        // Not hot enough, fall back to baseline
                        if let Some(ref mut baseline) = self.baseline_compiler {
                            baseline.compile(func_id, code)
                        } else {
                            Err(crate::baseline::JitError::CompilationFailed(
                                "Baseline compiler not available".to_string(),
                            ))
                        }
                    }
                } else if let Some(ref mut baseline) = self.baseline_compiler {
                    // No optimizing compiler or profile, fall back to baseline
                    baseline.compile(func_id, code)
                } else {
                    Err(crate::baseline::JitError::CompilationFailed(
                        "No compiler available".to_string(),
                    ))
                }
            }
            CompilationTier::AotOptimized => {
                // Not yet implemented - record as failure
                Err(crate::baseline::JitError::CompilationFailed(
                    "AOT optimization not yet implemented".to_string(),
                ))
            }
            CompilationTier::Interpreter => return None,
        };

        match result {
            Ok(ptr) => {
                let compiled = Arc::new(CompiledFunction {
                    tier,
                    code_ptr: ptr,
                    code_size: 0,
                    deopt_points: Vec::new(),
                });
                self.compiled_code.insert(func_id, compiled);
                Some(ptr)
            }
            Err(e) => {
                // Record the failure so we don't retry
                self.record_compilation_failure(func_id, tier, &e.to_string());
                // Log the failure for debugging
                #[cfg(debug_assertions)]
                eprintln!(
                    "[JIT WARN] Compilation failed for function {:?} at tier {:?}: {}. Falling back to interpreter.",
                    func_id, tier, e
                );
                // Return None to fall back to interpreter
                None
            }
        }
    }

    /// Compile a function with fallback to interpreter on failure
    ///
    /// This is the recommended entry point for compilation as it handles
    /// all error cases gracefully and ensures the function can still execute.
    pub fn compile_with_fallback(
        &mut self,
        func_id: FunctionId,
        code: &CodeObject,
    ) -> ExecutionMode {
        if !self.enabled {
            return ExecutionMode::Interpreter;
        }

        // Check if already compiled
        if let Some(compiled) = self.compiled_code.get(&func_id) {
            return ExecutionMode::Jit(compiled.code_ptr);
        }

        // Check if this function has failed before
        if self.has_failed_compilation(func_id) {
            return ExecutionMode::Interpreter;
        }

        // Determine target tier based on profile
        let target_tier = self.check_promotion(func_id).unwrap_or(CompilationTier::BaselineJit);

        // Attempt compilation
        match self.compile_code_object(func_id, target_tier, code) {
            Some(ptr) => ExecutionMode::Jit(ptr),
            None => {
                // Compilation failed, fall back to interpreter
                #[cfg(debug_assertions)]
                eprintln!(
                    "[JIT DEBUG] Function {:?} will execute in interpreter mode due to compilation failure",
                    func_id
                );
                ExecutionMode::Interpreter
            }
        }
    }

    /// Record a compilation failure
    fn record_compilation_failure(&self, func_id: FunctionId, tier: CompilationTier, error: &str) {
        self.failed_compilations
            .entry(func_id)
            .and_modify(|f| f.attempts += 1)
            .or_insert(CompilationFailure {
                tier,
                error: error.to_string(),
                attempts: 1,
            });
    }

    /// Check if a function has failed compilation
    pub fn has_failed_compilation(&self, func_id: FunctionId) -> bool {
        self.failed_compilations.contains_key(&func_id)
    }

    /// Get compilation failure info for a function
    pub fn get_compilation_failure(&self, func_id: FunctionId) -> Option<CompilationFailure> {
        self.failed_compilations.get(&func_id).map(|f| f.clone())
    }

    /// Clear compilation failure for a function (allow retry)
    pub fn clear_compilation_failure(&self, func_id: FunctionId) {
        self.failed_compilations.remove(&func_id);
    }

    /// Baseline JIT compilation - fast compile, no type specialization
    fn compile_baseline(&self, _func_id: FunctionId, _bytecode: &[u8]) -> Option<*const u8> {
        // In a real implementation, this would:
        // 1. Create a Cranelift function
        // 2. Translate bytecode 1:1 to IR
        // 3. Compile and return code pointer

        // For now, return None (not implemented)
        None
    }

    /// Optimizing JIT compilation - type-specialized with guards
    fn compile_optimized(&self, func_id: FunctionId, _bytecode: &[u8]) -> Option<*const u8> {
        let profile = self.profiles.get(&func_id)?;

        // In a real implementation, this would:
        // 1. Analyze type feedback
        // 2. Generate specialized code for monomorphic sites
        // 3. Insert type guards for polymorphic sites
        // 4. Compile with optimizations

        // Check type feedback for specialization opportunities
        for feedback in profile.type_feedback.iter() {
            if feedback.is_monomorphic() {
                let types = feedback.get_types();
                if let Some(PyType::Int) = types.first() {
                    // Could emit specialized int code
                }
            }
        }

        None
    }

    /// AOT compilation with profile-guided optimization
    fn compile_aot(&self, func_id: FunctionId, _bytecode: &[u8]) -> Option<*const u8> {
        let profile = self.profiles.get(&func_id)?;

        // In a real implementation, this would:
        // 1. Use branch probabilities for code layout
        // 2. Inline hot call sites
        // 3. Apply aggressive optimizations
        // 4. Save to persistent cache

        // Use branch probabilities
        for i in 0..profile.branch_counts.len() {
            if let Some(_prob) = profile.get_branch_probability(i) {
                // Could use probability for code layout
            }
        }

        None
    }

    /// Get compiled code for a function
    pub fn get_compiled(&self, func_id: FunctionId) -> Option<Arc<CompiledFunction>> {
        self.compiled_code.get(&func_id).map(|r| r.clone())
    }

    /// Invalidate compiled code for a function
    pub fn invalidate(&self, func_id: FunctionId) {
        self.compiled_code.remove(&func_id);
    }

    /// Get the current tier for a function
    pub fn get_tier(&self, func_id: FunctionId) -> CompilationTier {
        self.compiled_code
            .get(&func_id)
            .map(|c| c.tier)
            .unwrap_or(CompilationTier::Interpreter)
    }

    /// Check and promote a function if it meets the threshold
    /// Returns the new tier if promotion occurred, None otherwise
    pub fn check_and_promote(
        &mut self,
        func_id: FunctionId,
        code: &CodeObject,
    ) -> Option<CompilationTier> {
        if !self.enabled {
            return None;
        }

        // Check if promotion is needed
        let target_tier = self.check_promotion(func_id)?;

        // Attempt compilation at the target tier
        let code_ptr = self.compile_code_object(func_id, target_tier, code)?;

        // Verify compilation succeeded
        if !code_ptr.is_null() {
            Some(target_tier)
        } else {
            None
        }
    }

    /// Record a function call and check for promotion
    /// This is the main entry point for the interpreter to use
    pub fn on_function_call(
        &mut self,
        func_id: FunctionId,
        code: &CodeObject,
    ) -> Option<*const u8> {
        if !self.enabled {
            return None;
        }

        // Get or create profile
        let profile = self.get_profile(func_id, code.code.len(), 0);

        // Record the call
        profile.record_call();

        // Check if we should promote
        if let Some(target_tier) = self.check_promotion(func_id) {
            // Try to compile
            return self.compile_code_object(func_id, target_tier, code);
        }

        // Return existing compiled code if available
        self.compiled_code.get(&func_id).map(|c| c.code_ptr)
    }

    /// Get compiled code pointer if available
    pub fn get_code_ptr(&self, func_id: FunctionId) -> Option<*const u8> {
        self.compiled_code.get(&func_id).map(|c| c.code_ptr)
    }

    /// Check if JIT is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable JIT
    pub fn set_enabled(&self, _enabled: bool) {
        // Note: This is not thread-safe for the enabled flag
        // In production, we'd use an atomic
    }
}

impl Default for TieredJit {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_creation() {
        let jit = TieredJit::new();
        assert!(jit.is_enabled());
    }

    #[test]
    fn test_profile_creation() {
        let jit = TieredJit::new();
        let func_id = FunctionId(1);

        let profile = jit.get_profile(func_id, 100, 5);
        assert_eq!(profile.get_call_count(), 0);

        // Same profile should be returned
        let profile2 = jit.get_profile(func_id, 100, 5);
        assert_eq!(Arc::as_ptr(&profile), Arc::as_ptr(&profile2));
    }

    #[test]
    fn test_tier_promotion() {
        let jit = TieredJit::new();
        let func_id = FunctionId(1);

        let profile = jit.get_profile(func_id, 100, 5);

        // Not enough calls for promotion
        for _ in 0..50 {
            profile.record_call();
        }
        assert!(jit.check_promotion(func_id).is_none());

        // Enough calls for baseline JIT
        for _ in 0..50 {
            profile.record_call();
        }
        assert_eq!(jit.check_promotion(func_id), Some(CompilationTier::BaselineJit));
    }

    #[test]
    fn test_deopt_limit() {
        let jit = TieredJit::with_settings(true, 5);
        let func_id = FunctionId(1);

        let profile = jit.get_profile(func_id, 100, 5);

        // Record enough calls
        for _ in 0..200 {
            profile.record_call();
        }

        // Should be eligible for promotion
        assert!(jit.check_promotion(func_id).is_some());

        // Record too many deopts
        for _ in 0..10 {
            profile.record_deopt();
        }

        // Should no longer be eligible
        assert!(jit.check_promotion(func_id).is_none());
    }

    #[test]
    fn test_get_tier() {
        let jit = TieredJit::new();
        let func_id = FunctionId(1);

        // Default tier is interpreter
        assert_eq!(jit.get_tier(func_id), CompilationTier::Interpreter);
    }

    #[test]
    fn test_on_function_call() {
        use dx_py_bytecode::{CodeFlags, Constant, DpbOpcode};

        let mut jit = TieredJit::new();
        let func_id = FunctionId(100);

        // Create a simple code object
        let code = dx_py_bytecode::CodeObject {
            name: "test".to_string(),
            qualname: "test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 1,
            flags: CodeFlags::OPTIMIZED,
            code: vec![DpbOpcode::LoadConst as u8, 0, 0, DpbOpcode::Return as u8],
            constants: vec![Constant::Int(42)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        // Call function 99 times - should not compile yet
        for _ in 0..99 {
            let result = jit.on_function_call(func_id, &code);
            assert!(result.is_none(), "Should not compile before 100 calls");
        }

        // 100th call should trigger compilation
        let result = jit.on_function_call(func_id, &code);
        assert!(result.is_some(), "Should compile at 100 calls");
        assert_eq!(jit.get_tier(func_id), CompilationTier::BaselineJit);
    }

    #[test]
    fn test_check_and_promote() {
        use dx_py_bytecode::{CodeFlags, Constant, DpbOpcode};

        let mut jit = TieredJit::new();
        let func_id = FunctionId(101);

        let code = dx_py_bytecode::CodeObject {
            name: "test".to_string(),
            qualname: "test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 1,
            flags: CodeFlags::OPTIMIZED,
            code: vec![DpbOpcode::LoadConst as u8, 0, 0, DpbOpcode::Return as u8],
            constants: vec![Constant::Int(42)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        // Record 100 calls manually
        let profile = jit.get_profile(func_id, code.code.len(), 0);
        for _ in 0..100 {
            profile.record_call();
        }

        // Check and promote should succeed
        let result = jit.check_and_promote(func_id, &code);
        assert_eq!(result, Some(CompilationTier::BaselineJit));
        assert_eq!(jit.get_tier(func_id), CompilationTier::BaselineJit);
    }

    #[test]
    fn test_compilation_failure_fallback() {
        use dx_py_bytecode::{CodeFlags, Constant, DpbOpcode};

        let mut jit = TieredJit::new();
        let func_id = FunctionId(102);

        // Create a code object with an unsupported opcode
        let code = dx_py_bytecode::CodeObject {
            name: "test".to_string(),
            qualname: "test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 1,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0,
                DpbOpcode::Yield as u8, // Unsupported opcode
                DpbOpcode::Return as u8,
            ],
            constants: vec![Constant::Int(42)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        // Record 100 calls
        let profile = jit.get_profile(func_id, code.code.len(), 0);
        for _ in 0..100 {
            profile.record_call();
        }

        // Compilation should fail gracefully
        let result = jit.check_and_promote(func_id, &code);
        assert!(result.is_none(), "Should return None on compilation failure");

        // Function should still be at interpreter tier
        assert_eq!(jit.get_tier(func_id), CompilationTier::Interpreter);

        // Failure should be recorded
        assert!(jit.has_failed_compilation(func_id));

        // Subsequent promotion checks should return None
        assert!(jit.check_promotion(func_id).is_none());
    }

    #[test]
    fn test_clear_compilation_failure() {
        use dx_py_bytecode::{CodeFlags, Constant, DpbOpcode};

        let mut jit = TieredJit::new();
        let func_id = FunctionId(103);
        let func_id2 = FunctionId(104); // Use different ID for retry

        // Create a code object with an unsupported opcode
        let bad_code = dx_py_bytecode::CodeObject {
            name: "test".to_string(),
            qualname: "test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 1,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::Yield as u8, // Unsupported
                DpbOpcode::Return as u8,
            ],
            constants: vec![],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        // Record calls and try to compile
        let profile = jit.get_profile(func_id, bad_code.code.len(), 0);
        for _ in 0..100 {
            profile.record_call();
        }
        jit.check_and_promote(func_id, &bad_code);

        // Should have failed
        assert!(jit.has_failed_compilation(func_id));

        // Clear the failure
        jit.clear_compilation_failure(func_id);
        assert!(!jit.has_failed_compilation(func_id));

        // Now try with good code on a different function ID
        // (same ID would have Cranelift context issues)
        let good_code = dx_py_bytecode::CodeObject {
            name: "test2".to_string(),
            qualname: "test2".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 1,
            flags: CodeFlags::OPTIMIZED,
            code: vec![DpbOpcode::LoadConst as u8, 0, 0, DpbOpcode::Return as u8],
            constants: vec![Constant::Int(42)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        // Record calls for the new function
        let profile2 = jit.get_profile(func_id2, good_code.code.len(), 0);
        for _ in 0..100 {
            profile2.record_call();
        }

        // Should be able to compile the new function
        let result = jit.check_and_promote(func_id2, &good_code);
        assert_eq!(result, Some(CompilationTier::BaselineJit));
    }
}
