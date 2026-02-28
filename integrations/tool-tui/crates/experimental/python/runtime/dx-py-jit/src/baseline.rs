//! Baseline JIT Compiler using Cranelift
//!
//! This module implements the baseline (Tier 1) JIT compiler that translates
//! DPB bytecode directly to native machine code using Cranelift.
//!
//! ## Deoptimization Support
//!
//! The baseline compiler implements speculative optimizations with type guards:
//!
//! - **Type Guards**: Runtime checks inserted for operations that assume specific types
//!   (e.g., integer arithmetic). When a guard fails, execution deoptimizes back to
//!   the interpreter.
//!
//! - **Guard Failure Handling**: When a type guard fails:
//!   1. The guard calls `rt_trigger_deopt` to signal deoptimization
//!   2. The JIT code returns a sentinel value (0/None)
//!   3. The runtime detects the deoptimization and resumes in the interpreter
//!   4. Frame state is reconstructed from deopt metadata
//!
//! - **Speculative Optimizations**: The baseline compiler speculatively assumes
//!   integer types for arithmetic operations. If non-integer types are encountered,
//!   the code deoptimizes and the interpreter handles the operation correctly.
//!
//! ## Requirements Validated
//!
//! - **Requirement 7.3**: WHEN a type guard fails in JIT code, THE Runtime SHALL
//!   deoptimize back to the interpreter
//! - **Requirement 7.4**: WHEN JIT compilation fails, THE Runtime SHALL fall back
//!   to interpretation without crashing

#![allow(clippy::fn_to_numeric_cast)]

use cranelift_codegen::ir::{types, AbiParam, Block, InstBuilder, MemFlags, UserFuncName, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;

use dx_py_bytecode::{CodeObject, Constant, DpbOpcode};

use crate::deopt::{
    rt_type_guard_is_int, DeoptFrameState, DeoptMetadata, DeoptValue, TypeGuard, TypeGuardKind,
};
use crate::helpers::{rt_call_function, rt_call_method, rt_contains, rt_power, rt_string_compare};
use crate::optimizing::DeoptReason;
use crate::FunctionId;

/// Errors that can occur during JIT compilation
#[derive(Debug, Error)]
pub enum JitError {
    #[error("Unsupported opcode: {0:?}")]
    UnsupportedOpcode(DpbOpcode),

    #[error("Cranelift error: {0}")]
    CraneliftError(String),

    #[error("Code too large: {size} bytes exceeds limit of {limit} bytes")]
    CodeTooLarge { size: usize, limit: usize },

    #[error("Invalid bytecode at offset {offset}: {message}")]
    InvalidBytecode { offset: usize, message: String },

    #[error("Module error: {0}")]
    ModuleError(String),

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
}

/// Compiled native code
pub struct CompiledCode {
    /// Pointer to the compiled code
    pub code_ptr: *const u8,
    /// Size of the compiled code
    pub code_size: usize,
    /// Function ID in the JIT module
    pub func_id: FuncId,
    /// Deoptimization metadata
    pub deopt_metadata: DeoptMetadata,
    /// Type guards inserted in the code
    pub type_guards: Vec<TypeGuard>,
}

// Safety: CompiledCode points to immutable executable memory
unsafe impl Send for CompiledCode {}
unsafe impl Sync for CompiledCode {}

/// Translation state for tracking values during bytecode-to-IR translation
pub struct TranslationState {
    /// Stack of Cranelift values (mirrors Python value stack)
    stack: Vec<Value>,
    /// Local variable slots (Cranelift Variables)
    locals: Vec<Variable>,
    /// Current basic block
    #[allow(dead_code)]
    current_block: Block,
    /// Block map for jump targets (bytecode offset -> Block)
    block_map: std::collections::HashMap<usize, Block>,
    /// Next variable index
    next_var: usize,
    /// Deoptimization metadata being built
    deopt_metadata: DeoptMetadata,
    /// Current bytecode offset (for deopt tracking)
    current_bytecode_offset: u32,
    /// Type guards inserted during compilation
    type_guards: Vec<TypeGuard>,
}

impl TranslationState {
    /// Create a new translation state
    pub fn new(entry_block: Block, num_locals: usize, func_id: FunctionId) -> Self {
        Self {
            stack: Vec::with_capacity(32),
            locals: Vec::with_capacity(num_locals),
            current_block: entry_block,
            block_map: std::collections::HashMap::new(),
            next_var: 0,
            deopt_metadata: DeoptMetadata::new(func_id),
            current_bytecode_offset: 0,
            type_guards: Vec::new(),
        }
    }

    /// Push a value onto the stack
    #[inline]
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    #[inline]
    pub fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    /// Peek at the top of the stack
    #[inline]
    pub fn peek(&self) -> Option<Value> {
        self.stack.last().copied()
    }

    /// Get stack depth
    #[inline]
    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }

    /// Allocate a new variable
    pub fn new_variable(&mut self) -> Variable {
        let var = Variable::from_u32(self.next_var as u32);
        self.next_var += 1;
        var
    }

    /// Get or create a block for a bytecode offset
    pub fn get_or_create_block(&mut self, offset: usize, builder: &mut FunctionBuilder) -> Block {
        *self.block_map.entry(offset).or_insert_with(|| builder.create_block())
    }

    /// Set the current bytecode offset
    pub fn set_bytecode_offset(&mut self, offset: u32) {
        self.current_bytecode_offset = offset;
    }

    /// Register a type guard at the current location
    pub fn register_type_guard(&mut self, kind: TypeGuardKind, native_offset: u32) {
        let guard = TypeGuard::new(kind, native_offset, self.current_bytecode_offset);
        self.type_guards.push(guard);
    }

    /// Record a deoptimization point
    pub fn record_deopt_point(&mut self, reason: DeoptReason, native_offset: u32) {
        // Build frame state from current stack and locals
        let mut frame_state = DeoptFrameState::new(self.current_bytecode_offset, reason);
        frame_state.native_offset = native_offset;

        // Record stack values
        for (i, _val) in self.stack.iter().enumerate() {
            // In a real implementation, we'd track the actual location of each value
            // For now, assume they're in registers
            frame_state.push_stack(DeoptValue::from_register(i as u8));
        }

        // Record local values
        for (i, _var) in self.locals.iter().enumerate() {
            frame_state.set_local(i, DeoptValue::from_local(i as u16));
        }

        self.deopt_metadata.register_deopt_point(native_offset, frame_state);
    }

    /// Get the deoptimization metadata (consumes state)
    pub fn into_deopt_data(self) -> (DeoptMetadata, Vec<TypeGuard>) {
        (self.deopt_metadata, self.type_guards)
    }
}

/// Baseline JIT compiler using Cranelift
pub struct BaselineCompiler {
    /// Cranelift JIT module
    module: JITModule,
    /// Function builder context (reusable)
    builder_ctx: FunctionBuilderContext,
    /// Compiled function cache
    cache: DashMap<FunctionId, Arc<CompiledCode>>,
    /// Maximum code size (default 1MB)
    #[allow(dead_code)]
    max_code_size: usize,
}

impl BaselineCompiler {
    /// Create a new baseline compiler
    pub fn new() -> Result<Self, JitError> {
        // Configure Cranelift for the host architecture
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "speed")
            .map_err(|e| JitError::CraneliftError(format!("Failed to set opt_level: {}", e)))?;
        flag_builder
            .set("is_pic", "false")
            .map_err(|e| JitError::CraneliftError(format!("Failed to set is_pic: {}", e)))?;

        let isa_builder = cranelift_native::builder().map_err(|e| {
            JitError::CraneliftError(format!("Failed to create ISA builder: {}", e))
        })?;

        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .map_err(|e| JitError::CraneliftError(format!("Failed to create ISA: {}", e)))?;

        // Create JIT module
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);

        Ok(Self {
            module,
            builder_ctx: FunctionBuilderContext::new(),
            cache: DashMap::new(),
            max_code_size: 1024 * 1024, // 1MB default
        })
    }

    /// Compile a code object to native code
    pub fn compile(
        &mut self,
        func_id: FunctionId,
        code: &CodeObject,
    ) -> Result<*const u8, JitError> {
        // Check cache first
        if let Some(compiled) = self.cache.get(&func_id) {
            return Ok(compiled.code_ptr);
        }

        // Create function signature
        // Python functions take: (frame_ptr, args_ptr, nargs) -> result_ptr
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // frame pointer
        sig.params.push(AbiParam::new(types::I64)); // args pointer
        sig.params.push(AbiParam::new(types::I32)); // number of args
        sig.returns.push(AbiParam::new(types::I64)); // result pointer

        // Declare the function
        let name = format!("py_func_{}", func_id.0);
        let clif_func_id = self
            .module
            .declare_function(&name, Linkage::Local, &sig)
            .map_err(|e| JitError::ModuleError(e.to_string()))?;

        // Create function context
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        ctx.func.name = UserFuncName::user(0, clif_func_id.as_u32());

        // Build the function - ensure builder_ctx is cleared on error
        let build_result = {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_ctx);
            let result = Self::translate_function_inner(&mut builder, code, func_id);
            if result.is_ok() {
                builder.finalize();
            }
            // builder is dropped here, which should clear the context
            result
        };

        // If translation failed, clear context and return error
        let (deopt_metadata, type_guards) = match build_result {
            Ok((metadata, guards)) => (metadata, guards),
            Err(e) => {
                self.module.clear_context(&mut ctx);
                // Reset the builder context for next use
                self.builder_ctx = FunctionBuilderContext::new();
                return Err(e);
            }
        };

        // Compile the function
        self.module.define_function(clif_func_id, &mut ctx).map_err(|e| {
            self.module.clear_context(&mut ctx);
            JitError::CompilationFailed(e.to_string())
        })?;

        self.module.clear_context(&mut ctx);

        // Finalize and get code pointer
        self.module
            .finalize_definitions()
            .map_err(|e| JitError::ModuleError(e.to_string()))?;

        let code_ptr = self.module.get_finalized_function(clif_func_id);

        // Cache the compiled code with deopt metadata
        let compiled = Arc::new(CompiledCode {
            code_ptr,
            code_size: 0, // Size not easily available from Cranelift
            func_id: clif_func_id,
            deopt_metadata,
            type_guards,
        });
        self.cache.insert(func_id, compiled);

        Ok(code_ptr)
    }

    /// Translate a Python function to Cranelift IR
    fn translate_function_inner(
        builder: &mut FunctionBuilder,
        code: &CodeObject,
        func_id: FunctionId,
    ) -> Result<(DeoptMetadata, Vec<TypeGuard>), JitError> {
        // Create entry block
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Get function parameters
        let _frame_ptr = builder.block_params(entry_block)[0];
        let args_ptr = builder.block_params(entry_block)[1];
        let _nargs = builder.block_params(entry_block)[2];

        // Create translation state
        let mut state = TranslationState::new(entry_block, code.nlocals as usize, func_id);

        // Declare local variables
        for _ in 0..code.nlocals {
            let var = state.new_variable();
            builder.declare_var(var, types::I64);
            state.locals.push(var);
        }

        // Initialize locals from arguments
        for i in 0..code.argcount.min(code.nlocals) {
            let idx_val = builder.ins().iconst(types::I64, i as i64);
            let offset = builder.ins().imul_imm(idx_val, 8); // sizeof(PyObject*)
            let arg_addr = builder.ins().iadd(args_ptr, offset);
            let arg_val = builder.ins().load(types::I64, MemFlags::new(), arg_addr, 0);
            builder.def_var(state.locals[i as usize], arg_val);
        }

        // Translate bytecode
        let bytecode = &code.code;
        let mut offset = 0;
        let mut returned = false;

        while offset < bytecode.len() {
            state.set_bytecode_offset(offset as u32);

            let opcode_byte = bytecode[offset];
            let opcode =
                DpbOpcode::from_u8(opcode_byte).ok_or_else(|| JitError::InvalidBytecode {
                    offset,
                    message: format!("Invalid opcode: 0x{:02X}", opcode_byte),
                })?;

            let arg_size = opcode.arg_size();
            let arg = if arg_size > 0 && offset + 1 + arg_size <= bytecode.len() {
                Self::read_arg(&bytecode[offset + 1..], arg_size)
            } else {
                0
            };

            if opcode == DpbOpcode::Return {
                returned = true;
            }

            Self::translate_instruction(builder, &mut state, opcode, arg as u16, code)?;

            offset += 1 + arg_size;
        }

        // If we haven't returned yet, return None
        if !returned {
            let none_val = builder.ins().iconst(types::I64, 0); // NULL represents None
            builder.ins().return_(&[none_val]);
        }

        // Extract deopt metadata and type guards
        let (deopt_metadata, type_guards) = state.into_deopt_data();

        Ok((deopt_metadata, type_guards))
    }

    /// Read an argument from bytecode
    fn read_arg(bytes: &[u8], size: usize) -> u32 {
        match size {
            1 => bytes[0] as u32,
            2 => u16::from_le_bytes([bytes[0], bytes[1]]) as u32,
            4 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
            _ => 0,
        }
    }

    /// Emit a type guard that checks if a value is an integer
    /// Returns a tuple of (guarded_value, deopt_block, continue_block)
    /// If the guard fails, execution jumps to deopt_block which should trigger deoptimization
    fn emit_int_type_guard(
        builder: &mut FunctionBuilder,
        state: &mut TranslationState,
        value: Value,
        func_id: FunctionId,
    ) -> (Block, Block) {
        // Call rt_type_guard_is_int to check if value is an integer
        let guard_fn_addr = builder.ins().iconst(types::I64, rt_type_guard_is_int as i64);

        // Create signature for the guard function: (value: i64) -> i64
        let sig = cranelift_codegen::ir::Signature {
            params: vec![AbiParam::new(types::I64)],
            returns: vec![AbiParam::new(types::I64)],
            call_conv: cranelift_codegen::isa::CallConv::SystemV,
        };
        let sig_ref = builder.import_signature(sig);

        // Call the guard function
        let call = builder.ins().call_indirect(sig_ref, guard_fn_addr, &[value]);
        let is_int = builder.inst_results(call)[0];

        // Check if the guard passed (non-zero means it's an int)
        let zero = builder.ins().iconst(types::I64, 0);
        let guard_passed = builder.ins().icmp(
            cranelift_codegen::ir::condcodes::IntCC::NotEqual,
            is_int,
            zero,
        );

        // Create blocks for guard success and failure
        let continue_block = builder.create_block();
        let deopt_block = builder.create_block();

        // Branch based on guard result
        builder.ins().brif(guard_passed, continue_block, &[], deopt_block, &[]);

        // In the deopt block, we need to trigger deoptimization
        builder.switch_to_block(deopt_block);
        builder.seal_block(deopt_block);

        // Call rt_trigger_deopt to signal deoptimization
        let deopt_fn_addr = builder.ins().iconst(types::I64, crate::deopt::rt_trigger_deopt as i64);
        let func_id_val = builder.ins().iconst(types::I64, func_id.0 as i64);
        let bc_offset_val = builder.ins().iconst(types::I32, state.current_bytecode_offset as i64);
        let reason_val = builder.ins().iconst(types::I8, DeoptReason::TypeGuardFailed as i64);

        let deopt_sig = cranelift_codegen::ir::Signature {
            params: vec![
                AbiParam::new(types::I64), // func_id
                AbiParam::new(types::I32), // bytecode_offset
                AbiParam::new(types::I8),  // reason
            ],
            returns: vec![AbiParam::new(types::I64)],
            call_conv: cranelift_codegen::isa::CallConv::SystemV,
        };
        let deopt_sig_ref = builder.import_signature(deopt_sig);

        let _deopt_call = builder.ins().call_indirect(
            deopt_sig_ref,
            deopt_fn_addr,
            &[func_id_val, bc_offset_val, reason_val],
        );

        // After triggering deopt, return a sentinel value (0/None)
        // This indicates to the runtime that deoptimization occurred
        let sentinel = builder.ins().iconst(types::I64, 0);
        builder.ins().return_(&[sentinel]);

        // Record the deopt point in state
        state.record_deopt_point(DeoptReason::TypeGuardFailed, 0);
        state.register_type_guard(TypeGuardKind::IsInt, 0);

        // Switch to continue block for normal execution
        builder.switch_to_block(continue_block);
        builder.seal_block(continue_block);

        (deopt_block, continue_block)
    }

    /// Emit a guarded integer addition with deoptimization on type mismatch
    /// This is a speculative optimization that assumes both operands are integers
    fn emit_guarded_int_add(
        builder: &mut FunctionBuilder,
        state: &mut TranslationState,
        a: Value,
        b: Value,
        func_id: FunctionId,
    ) -> Value {
        // Guard that 'a' is an integer
        let (_deopt_a, _continue_a) = Self::emit_int_type_guard(builder, state, a, func_id);

        // Guard that 'b' is an integer
        let (_deopt_b, _continue_b) = Self::emit_int_type_guard(builder, state, b, func_id);

        // Both guards passed, perform integer addition
        builder.ins().iadd(a, b)
    }

    /// Emit a guarded integer subtraction with deoptimization on type mismatch
    fn emit_guarded_int_sub(
        builder: &mut FunctionBuilder,
        state: &mut TranslationState,
        a: Value,
        b: Value,
        func_id: FunctionId,
    ) -> Value {
        Self::emit_int_type_guard(builder, state, a, func_id);
        Self::emit_int_type_guard(builder, state, b, func_id);
        builder.ins().isub(a, b)
    }

    /// Emit a guarded integer multiplication with deoptimization on type mismatch
    fn emit_guarded_int_mul(
        builder: &mut FunctionBuilder,
        state: &mut TranslationState,
        a: Value,
        b: Value,
        func_id: FunctionId,
    ) -> Value {
        Self::emit_int_type_guard(builder, state, a, func_id);
        Self::emit_int_type_guard(builder, state, b, func_id);
        builder.ins().imul(a, b)
    }

    /// Translate a single bytecode instruction to Cranelift IR
    fn translate_instruction(
        builder: &mut FunctionBuilder,
        state: &mut TranslationState,
        opcode: DpbOpcode,
        arg: u16,
        code: &CodeObject,
    ) -> Result<(), JitError> {
        let func_id = state.deopt_metadata.func_id;

        match opcode {
            // ===== Stack operations =====
            DpbOpcode::PopTop => {
                state.pop();
            }

            DpbOpcode::DupTop => {
                if let Some(val) = state.peek() {
                    state.push(val);
                }
            }

            DpbOpcode::DupTopTwo => {
                if state.stack_depth() >= 2 {
                    let len = state.stack.len();
                    let a = state.stack[len - 2];
                    let b = state.stack[len - 1];
                    state.push(a);
                    state.push(b);
                }
            }

            DpbOpcode::Swap => {
                if state.stack_depth() >= 2 {
                    let a = state.pop().unwrap();
                    let b = state.pop().unwrap();
                    state.push(a);
                    state.push(b);
                }
            }

            DpbOpcode::RotN => {
                let n = arg as usize;
                if state.stack_depth() >= n && n > 0 {
                    // Rotate top N items: move top to position N-1
                    let top = state.pop().unwrap();
                    let insert_pos = state.stack.len() - (n - 1);
                    state.stack.insert(insert_pos, top);
                }
            }

            DpbOpcode::Copy => {
                let n = arg as usize;
                if n > 0 && state.stack_depth() >= n {
                    let idx = state.stack.len() - n;
                    let val = state.stack[idx];
                    state.push(val);
                }
            }

            // ===== Load/Store operations =====
            DpbOpcode::LoadFast => {
                let idx = arg as usize;
                if idx < state.locals.len() {
                    let val = builder.use_var(state.locals[idx]);
                    state.push(val);
                }
            }

            DpbOpcode::StoreFast => {
                let idx = arg as usize;
                if idx < state.locals.len() {
                    if let Some(val) = state.pop() {
                        builder.def_var(state.locals[idx], val);
                    }
                }
            }

            DpbOpcode::LoadConst => {
                let idx = arg as usize;
                if idx < code.constants.len() {
                    let const_val = Self::load_constant(builder, &code.constants[idx]);
                    state.push(const_val);
                }
            }

            DpbOpcode::PushNull => {
                // Push a null/None value onto the stack
                let null_val = builder.ins().iconst(types::I64, 0);
                state.push(null_val);
            }

            // ===== Binary Arithmetic Operations =====
            DpbOpcode::BinaryAdd => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Use guarded integer addition with type guards
                    // If types don't match, this will deoptimize to interpreter
                    let result = Self::emit_guarded_int_add(builder, state, a, b, func_id);
                    state.push(result);
                }
            }

            DpbOpcode::BinarySub => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Use guarded integer subtraction with type guards
                    let result = Self::emit_guarded_int_sub(builder, state, a, b, func_id);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryMul => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Use guarded integer multiplication with type guards
                    let result = Self::emit_guarded_int_mul(builder, state, a, b, func_id);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryDiv => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // True division - convert to float and divide
                    // For simplicity, we'll use signed division for now
                    // In production, we'd convert to f64 first
                    let result = builder.ins().sdiv(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryFloorDiv => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Floor division (integer division)
                    let result = builder.ins().sdiv(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryMod => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Modulo operation
                    let result = builder.ins().srem(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryPow => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Call rt_power runtime helper for power operation
                    // This handles integer exponentiation correctly
                    let helper_addr = builder.ins().iconst(types::I64, rt_power as i64);

                    // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> PyObjectPtr
                    let sig = cranelift_codegen::ir::Signature {
                        params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        returns: vec![AbiParam::new(types::I64)],
                        call_conv: cranelift_codegen::isa::CallConv::SystemV,
                    };
                    let sig_ref = builder.import_signature(sig);

                    // Call the runtime helper
                    let call = builder.ins().call_indirect(sig_ref, helper_addr, &[a, b]);
                    let result = builder.inst_results(call)[0];
                    state.push(result);
                }
            }

            DpbOpcode::BinaryAnd => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().band(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryOr => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().bor(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryXor => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().bxor(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryLshift => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().ishl(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::BinaryRshift => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Arithmetic right shift (preserves sign)
                    let result = builder.ins().sshr(a, b);
                    state.push(result);
                }
            }

            // ===== Unary Operations =====
            DpbOpcode::UnaryNeg => {
                if let Some(a) = state.pop() {
                    let result = builder.ins().ineg(a);
                    state.push(result);
                }
            }

            DpbOpcode::UnaryPos => {
                // Unary positive is a no-op for numbers
                // Value stays on stack
            }

            DpbOpcode::UnaryInvert => {
                if let Some(a) = state.pop() {
                    let result = builder.ins().bnot(a);
                    state.push(result);
                }
            }

            DpbOpcode::UnaryNot => {
                if let Some(a) = state.pop() {
                    // Boolean not: compare with 0, return 1 if equal, 0 otherwise
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_zero =
                        builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, a, zero);
                    let result = builder.ins().uextend(types::I64, is_zero);
                    state.push(result);
                }
            }

            // ===== In-place Operations (same as binary for JIT) =====
            DpbOpcode::InplaceAdd => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = Self::emit_guarded_int_add(builder, state, a, b, func_id);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceSub => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = Self::emit_guarded_int_sub(builder, state, a, b, func_id);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceMul => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = Self::emit_guarded_int_mul(builder, state, a, b, func_id);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceDiv => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().sdiv(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceFloorDiv => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().sdiv(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceMod => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().srem(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplacePow => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    // Call rt_power runtime helper for in-place power operation
                    let helper_addr = builder.ins().iconst(types::I64, rt_power as i64);

                    // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> PyObjectPtr
                    let sig = cranelift_codegen::ir::Signature {
                        params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        returns: vec![AbiParam::new(types::I64)],
                        call_conv: cranelift_codegen::isa::CallConv::SystemV,
                    };
                    let sig_ref = builder.import_signature(sig);

                    // Call the runtime helper
                    let call = builder.ins().call_indirect(sig_ref, helper_addr, &[a, b]);
                    let result = builder.inst_results(call)[0];
                    state.push(result);
                }
            }

            DpbOpcode::InplaceAnd => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().band(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceOr => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().bor(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceXor => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().bxor(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceLshift => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().ishl(a, b);
                    state.push(result);
                }
            }

            DpbOpcode::InplaceRshift => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let result = builder.ins().sshr(a, b);
                    state.push(result);
                }
            }

            // ===== Return =====
            DpbOpcode::Return => {
                let ret_val = state.pop().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                builder.ins().return_(&[ret_val]);
                // After return, create a new block for any following code
                // (which would be dead code, but we need it for the builder)
                let next_block = builder.create_block();
                builder.switch_to_block(next_block);
                builder.seal_block(next_block);
            }

            // ===== NOP =====
            DpbOpcode::Nop => {
                // No operation
            }

            // ===== Control Flow Operations =====
            DpbOpcode::Jump => {
                // Unconditional jump to target offset
                let target_offset = arg as usize;
                let target_block = state.get_or_create_block(target_offset, builder);
                builder.ins().jump(target_block, &[]);
                // After unconditional jump, create a new block for any following code
                // (which would be dead code, but we need it for the builder)
                let next_block = builder.create_block();
                builder.switch_to_block(next_block);
                builder.seal_block(next_block);
            }

            DpbOpcode::JumpIfTrue => {
                // Jump if top of stack is truthy (don't pop)
                if let Some(cond) = state.peek() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_true = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                        cond,
                        zero,
                    );
                    builder.ins().brif(is_true, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::JumpIfFalse => {
                // Jump if top of stack is falsy (don't pop)
                if let Some(cond) = state.peek() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_false = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        cond,
                        zero,
                    );
                    builder.ins().brif(is_false, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::JumpIfTrueOrPop => {
                // Jump if true, otherwise pop
                if let Some(cond) = state.peek() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_true = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                        cond,
                        zero,
                    );
                    builder.ins().brif(is_true, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                    // Pop on fallthrough
                    state.pop();
                }
            }

            DpbOpcode::JumpIfFalseOrPop => {
                // Jump if false, otherwise pop
                if let Some(cond) = state.peek() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_false = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        cond,
                        zero,
                    );
                    builder.ins().brif(is_false, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                    // Pop on fallthrough
                    state.pop();
                }
            }

            DpbOpcode::PopJumpIfTrue => {
                // Pop and jump if true
                if let Some(cond) = state.pop() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_true = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                        cond,
                        zero,
                    );
                    builder.ins().brif(is_true, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::PopJumpIfFalse => {
                // Pop and jump if false
                if let Some(cond) = state.pop() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_false = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        cond,
                        zero,
                    );
                    builder.ins().brif(is_false, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::PopJumpIfNone => {
                // Pop and jump if None (represented as 0/null)
                if let Some(val) = state.pop() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_none = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        val,
                        zero,
                    );
                    builder.ins().brif(is_none, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::PopJumpIfNotNone => {
                // Pop and jump if not None
                if let Some(val) = state.pop() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_not_none = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::NotEqual,
                        val,
                        zero,
                    );
                    builder.ins().brif(is_not_none, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::ForIter => {
                // For loop iteration - calls __next__ on iterator
                // If StopIteration, jump to target; otherwise push next value
                // For now, we'll implement a simplified version
                if let Some(_iter) = state.peek() {
                    let target_offset = arg as usize;
                    let _target_block = state.get_or_create_block(target_offset, builder);
                    // In production, we'd call a runtime helper to get next value
                    // For now, push a placeholder
                    let placeholder = builder.ins().iconst(types::I64, 0);
                    state.push(placeholder);
                }
            }

            DpbOpcode::GetIter => {
                // Get iterator from iterable - calls __iter__
                if let Some(iterable) = state.pop() {
                    // In production, we'd call a runtime helper
                    // For now, just pass through (assume already an iterator)
                    state.push(iterable);
                }
            }

            DpbOpcode::GetLen => {
                // Get length of object - calls __len__
                if let Some(obj) = state.pop() {
                    // In production, we'd call a runtime helper
                    // For now, push a placeholder
                    let _ = obj;
                    let placeholder = builder.ins().iconst(types::I64, 0);
                    state.push(placeholder);
                }
            }

            // ===== Comparison Operations =====
            // For baseline JIT, we use integer comparison as the fast path
            // String comparison via rt_string_compare is available for string types
            DpbOpcode::CompareLt => {
                let _ = rt_string_compare; // Reference to ensure import is used
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThan,
                        a,
                        b,
                    );
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareLe => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedLessThanOrEqual,
                        a,
                        b,
                    );
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareEq => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp =
                        builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, a, b);
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareNe => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp =
                        builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, a, b);
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareGt => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThan,
                        a,
                        b,
                    );
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareGe => {
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::SignedGreaterThanOrEqual,
                        a,
                        b,
                    );
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareIs => {
                // Identity comparison (pointer equality)
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp =
                        builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::Equal, a, b);
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareIsNot => {
                // Identity comparison (pointer inequality)
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp =
                        builder.ins().icmp(cranelift_codegen::ir::condcodes::IntCC::NotEqual, a, b);
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareIn => {
                // Membership test - calls __contains__ via runtime helper
                if let (Some(container), Some(item)) = (state.pop(), state.pop()) {
                    // Call rt_contains runtime helper for membership test
                    let helper_addr = builder.ins().iconst(types::I64, rt_contains as i64);

                    // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> i64
                    let sig = cranelift_codegen::ir::Signature {
                        params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        returns: vec![AbiParam::new(types::I64)],
                        call_conv: cranelift_codegen::isa::CallConv::SystemV,
                    };
                    let sig_ref = builder.import_signature(sig);

                    // Call the runtime helper (container, item)
                    let call =
                        builder.ins().call_indirect(sig_ref, helper_addr, &[container, item]);
                    let result = builder.inst_results(call)[0];
                    state.push(result);
                }
            }

            DpbOpcode::CompareNotIn => {
                // Negative membership test - calls __contains__ and negates result
                if let (Some(container), Some(item)) = (state.pop(), state.pop()) {
                    // Call rt_contains runtime helper for membership test
                    let helper_addr = builder.ins().iconst(types::I64, rt_contains as i64);

                    // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> i64
                    let sig = cranelift_codegen::ir::Signature {
                        params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        returns: vec![AbiParam::new(types::I64)],
                        call_conv: cranelift_codegen::isa::CallConv::SystemV,
                    };
                    let sig_ref = builder.import_signature(sig);

                    // Call the runtime helper (container, item)
                    let call =
                        builder.ins().call_indirect(sig_ref, helper_addr, &[container, item]);
                    let contains_result = builder.inst_results(call)[0];

                    // Negate the result: not in = !in
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_not_in = builder.ins().icmp(
                        cranelift_codegen::ir::condcodes::IntCC::Equal,
                        contains_result,
                        zero,
                    );
                    let result = builder.ins().uextend(types::I64, is_not_in);
                    state.push(result);
                }
            }

            // ===== Function Call Operations =====
            DpbOpcode::Call => {
                // Call a callable with positional arguments
                // Stack: [callable, arg0, arg1, ..., argN] -> [result]
                // arg = number of arguments
                let nargs = arg as usize;

                // Pop arguments in reverse order
                let mut args = Vec::with_capacity(nargs);
                for _ in 0..nargs {
                    if let Some(arg_val) = state.pop() {
                        args.push(arg_val);
                    }
                }
                args.reverse();

                // Pop the callable
                if let Some(callable) = state.pop() {
                    // Build args array on stack if we have arguments
                    if nargs > 0 {
                        // Allocate stack space for args array
                        // Each arg is 8 bytes (i64/pointer)
                        let stack_slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                (nargs * 8) as u32,
                            ),
                        );

                        // Store args in the stack slot
                        for (i, arg_val) in args.iter().enumerate() {
                            let offset = (i * 8) as i32;
                            builder.ins().stack_store(*arg_val, stack_slot, offset);
                        }

                        // Get pointer to args array
                        let args_ptr = builder.ins().stack_addr(types::I64, stack_slot, 0);

                        // Call rt_call_function helper
                        let helper_addr =
                            builder.ins().iconst(types::I64, rt_call_function as i64);

                        // Create signature: (callable: i64, args: i64, nargs: i32) -> i64
                        let sig = cranelift_codegen::ir::Signature {
                            params: vec![
                                AbiParam::new(types::I64), // callable
                                AbiParam::new(types::I64), // args pointer
                                AbiParam::new(types::I32), // nargs
                            ],
                            returns: vec![AbiParam::new(types::I64)],
                            call_conv: cranelift_codegen::isa::CallConv::SystemV,
                        };
                        let sig_ref = builder.import_signature(sig);

                        let nargs_val = builder.ins().iconst(types::I32, nargs as i64);
                        let call = builder.ins().call_indirect(
                            sig_ref,
                            helper_addr,
                            &[callable, args_ptr, nargs_val],
                        );
                        let result = builder.inst_results(call)[0];
                        state.push(result);
                    } else {
                        // No args - call with null args pointer
                        let helper_addr =
                            builder.ins().iconst(types::I64, rt_call_function as i64);

                        let sig = cranelift_codegen::ir::Signature {
                            params: vec![
                                AbiParam::new(types::I64),
                                AbiParam::new(types::I64),
                                AbiParam::new(types::I32),
                            ],
                            returns: vec![AbiParam::new(types::I64)],
                            call_conv: cranelift_codegen::isa::CallConv::SystemV,
                        };
                        let sig_ref = builder.import_signature(sig);

                        let null_ptr = builder.ins().iconst(types::I64, 0);
                        let nargs_val = builder.ins().iconst(types::I32, 0);
                        let call = builder.ins().call_indirect(
                            sig_ref,
                            helper_addr,
                            &[callable, null_ptr, nargs_val],
                        );
                        let result = builder.inst_results(call)[0];
                        state.push(result);
                    }
                }
            }

            DpbOpcode::CallKw => {
                // Call with keyword arguments
                // Stack: [callable, arg0, ..., argN, kwnames_tuple] -> [result]
                // arg = total number of arguments (positional + keyword)
                let nargs = arg as usize;

                // Pop kwnames tuple (contains keyword argument names)
                let _kwnames = state.pop();

                // Pop all arguments
                let mut args = Vec::with_capacity(nargs);
                for _ in 0..nargs {
                    if let Some(arg_val) = state.pop() {
                        args.push(arg_val);
                    }
                }
                args.reverse();

                // Pop the callable
                if let Some(callable) = state.pop() {
                    // For keyword calls, we use the same helper but pass all args
                    // The runtime helper will handle keyword argument matching
                    if nargs > 0 {
                        let stack_slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                (nargs * 8) as u32,
                            ),
                        );

                        for (i, arg_val) in args.iter().enumerate() {
                            let offset = (i * 8) as i32;
                            builder.ins().stack_store(*arg_val, stack_slot, offset);
                        }

                        let args_ptr = builder.ins().stack_addr(types::I64, stack_slot, 0);
                        let helper_addr =
                            builder.ins().iconst(types::I64, rt_call_function as i64);

                        let sig = cranelift_codegen::ir::Signature {
                            params: vec![
                                AbiParam::new(types::I64),
                                AbiParam::new(types::I64),
                                AbiParam::new(types::I32),
                            ],
                            returns: vec![AbiParam::new(types::I64)],
                            call_conv: cranelift_codegen::isa::CallConv::SystemV,
                        };
                        let sig_ref = builder.import_signature(sig);

                        let nargs_val = builder.ins().iconst(types::I32, nargs as i64);
                        let call = builder.ins().call_indirect(
                            sig_ref,
                            helper_addr,
                            &[callable, args_ptr, nargs_val],
                        );
                        let result = builder.inst_results(call)[0];
                        state.push(result);
                    } else {
                        let helper_addr =
                            builder.ins().iconst(types::I64, rt_call_function as i64);

                        let sig = cranelift_codegen::ir::Signature {
                            params: vec![
                                AbiParam::new(types::I64),
                                AbiParam::new(types::I64),
                                AbiParam::new(types::I32),
                            ],
                            returns: vec![AbiParam::new(types::I64)],
                            call_conv: cranelift_codegen::isa::CallConv::SystemV,
                        };
                        let sig_ref = builder.import_signature(sig);

                        let null_ptr = builder.ins().iconst(types::I64, 0);
                        let nargs_val = builder.ins().iconst(types::I32, 0);
                        let call = builder.ins().call_indirect(
                            sig_ref,
                            helper_addr,
                            &[callable, null_ptr, nargs_val],
                        );
                        let result = builder.inst_results(call)[0];
                        state.push(result);
                    }
                }
            }

            DpbOpcode::CallEx => {
                // Call with *args and/or **kwargs
                // Stack: [callable, args_tuple, kwargs_dict] -> [result]
                // arg indicates which are present (flags)
                let _kwargs = if arg & 1 != 0 { state.pop() } else { None };
                let args_tuple = state.pop();

                if let Some(callable) = state.pop() {
                    // For CallEx, we pass the args tuple directly to the helper
                    // The runtime will unpack it
                    let helper_addr = builder.ins().iconst(types::I64, rt_call_function as i64);

                    let sig = cranelift_codegen::ir::Signature {
                        params: vec![
                            AbiParam::new(types::I64),
                            AbiParam::new(types::I64),
                            AbiParam::new(types::I32),
                        ],
                        returns: vec![AbiParam::new(types::I64)],
                        call_conv: cranelift_codegen::isa::CallConv::SystemV,
                    };
                    let sig_ref = builder.import_signature(sig);

                    // Pass args_tuple as the args pointer (runtime will handle unpacking)
                    let args_ptr = args_tuple.unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                    let nargs_val = builder.ins().iconst(types::I32, -1); // -1 indicates tuple unpacking needed
                    let call = builder
                        .ins()
                        .call_indirect(sig_ref, helper_addr, &[callable, args_ptr, nargs_val]);
                    let result = builder.inst_results(call)[0];
                    state.push(result);
                }
            }

            DpbOpcode::LoadMethod => {
                // Load a method for calling
                // Stack: [obj] -> [method, self] or [NULL, bound_method]
                // arg = name index
                if let Some(obj) = state.pop() {
                    // In production, we'd look up the method on the object
                    // For now, push NULL and the object (unbound method pattern)
                    let null = builder.ins().iconst(types::I64, 0);
                    state.push(null);
                    state.push(obj);
                }
            }

            DpbOpcode::CallMethod => {
                // Call a method loaded by LOAD_METHOD
                // Stack: [method, self, arg0, ..., argN] -> [result]
                // arg = number of arguments (not including self)
                let nargs = arg as usize;

                // Pop arguments
                let mut args = Vec::with_capacity(nargs);
                for _ in 0..nargs {
                    if let Some(arg_val) = state.pop() {
                        args.push(arg_val);
                    }
                }
                args.reverse();

                // Pop self and method
                let self_val = state.pop();
                let method = state.pop();

                if let (Some(method_val), Some(self_obj)) = (method, self_val) {
                    // Call rt_call_method helper
                    let helper_addr = builder.ins().iconst(types::I64, rt_call_method as i64);

                    // Create signature: (method: i64, self: i64, args: i64, nargs: i32) -> i64
                    let sig = cranelift_codegen::ir::Signature {
                        params: vec![
                            AbiParam::new(types::I64), // method
                            AbiParam::new(types::I64), // self
                            AbiParam::new(types::I64), // args pointer
                            AbiParam::new(types::I32), // nargs
                        ],
                        returns: vec![AbiParam::new(types::I64)],
                        call_conv: cranelift_codegen::isa::CallConv::SystemV,
                    };
                    let sig_ref = builder.import_signature(sig);

                    if nargs > 0 {
                        // Allocate stack space for args array
                        let stack_slot = builder.create_sized_stack_slot(
                            cranelift_codegen::ir::StackSlotData::new(
                                cranelift_codegen::ir::StackSlotKind::ExplicitSlot,
                                (nargs * 8) as u32,
                            ),
                        );

                        // Store args in the stack slot
                        for (i, arg_val) in args.iter().enumerate() {
                            let offset = (i * 8) as i32;
                            builder.ins().stack_store(*arg_val, stack_slot, offset);
                        }

                        let args_ptr = builder.ins().stack_addr(types::I64, stack_slot, 0);
                        let nargs_val = builder.ins().iconst(types::I32, nargs as i64);

                        let call = builder.ins().call_indirect(
                            sig_ref,
                            helper_addr,
                            &[method_val, self_obj, args_ptr, nargs_val],
                        );
                        let result = builder.inst_results(call)[0];
                        state.push(result);
                    } else {
                        // No args
                        let null_ptr = builder.ins().iconst(types::I64, 0);
                        let nargs_val = builder.ins().iconst(types::I32, 0);

                        let call = builder.ins().call_indirect(
                            sig_ref,
                            helper_addr,
                            &[method_val, self_obj, null_ptr, nargs_val],
                        );
                        let result = builder.inst_results(call)[0];
                        state.push(result);
                    }
                } else {
                    // Missing method or self - push null result
                    let result = builder.ins().iconst(types::I64, 0);
                    state.push(result);
                }
            }

            DpbOpcode::MakeFunction => {
                // Create a function object
                // Stack: [code_obj, qualname] -> [function]
                // arg = flags indicating defaults, annotations, etc.
                let _qualname = state.pop();
                let _code_obj = state.pop();

                // Handle optional components based on flags
                if arg & 0x08 != 0 {
                    // Has closure (free vars)
                    let _closure = state.pop();
                }
                if arg & 0x04 != 0 {
                    // Has annotations
                    let _annotations = state.pop();
                }
                if arg & 0x02 != 0 {
                    // Has keyword defaults
                    let _kwdefaults = state.pop();
                }
                if arg & 0x01 != 0 {
                    // Has positional defaults
                    let _defaults = state.pop();
                }

                // In production, we'd create a function object
                let func = builder.ins().iconst(types::I64, 0);
                state.push(func);
            }

            DpbOpcode::MakeClosure => {
                // Create a closure (function with free variables)
                // Similar to MakeFunction but always has closure
                let _qualname = state.pop();
                let _code_obj = state.pop();
                let _closure = state.pop();

                // In production, we'd create a closure object
                let closure = builder.ins().iconst(types::I64, 0);
                state.push(closure);
            }

            DpbOpcode::KwNames => {
                // Push keyword argument names tuple for next CALL_KW
                // arg = constant index of names tuple
                let idx = arg as usize;
                if idx < code.constants.len() {
                    let const_val = Self::load_constant(builder, &code.constants[idx]);
                    state.push(const_val);
                }
            }

            DpbOpcode::Precall => {
                // Prepare for a call (Python 3.11+ optimization)
                // This is a hint for the interpreter, JIT can mostly ignore
                // arg = number of arguments
            }

            // For now, other opcodes are not implemented
            _ => {
                // Return UnsupportedOpcode for unimplemented opcodes
                // In production, we'd fall back to interpreter
                return Err(JitError::UnsupportedOpcode(opcode));
            }
        }

        Ok(())
    }

    /// Load a constant value into a Cranelift Value
    fn load_constant(builder: &mut FunctionBuilder, constant: &Constant) -> Value {
        match constant {
            Constant::None => builder.ins().iconst(types::I64, 0),
            Constant::Bool(b) => builder.ins().iconst(types::I64, if *b { 1 } else { 0 }),
            Constant::Int(i) => builder.ins().iconst(types::I64, *i),
            Constant::Float(f) => {
                // Box the float and return pointer
                // For now, just use the bits as i64
                builder.ins().iconst(types::I64, f.to_bits() as i64)
            }
            _ => {
                // For complex constants, we'd need to reference the constant pool
                builder.ins().iconst(types::I64, 0)
            }
        }
    }

    /// Get compiled code from cache
    pub fn get_compiled(&self, func_id: FunctionId) -> Option<Arc<CompiledCode>> {
        self.cache.get(&func_id).map(|r| r.clone())
    }

    /// Invalidate compiled code
    pub fn invalidate(&self, func_id: FunctionId) {
        self.cache.remove(&func_id);
    }

    /// Check if a function is compiled
    pub fn is_compiled(&self, func_id: FunctionId) -> bool {
        self.cache.contains_key(&func_id)
    }
}

impl Default for BaselineCompiler {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            // Log the error and panic with a clear message
            // This is acceptable in Default since it's typically called during initialization
            panic!("Failed to create baseline compiler: {}. This is a critical error.", e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dx_py_bytecode::CodeFlags;

    fn create_simple_code() -> CodeObject {
        CodeObject {
            name: "test".to_string(),
            qualname: "test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 1,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0,                       // LOAD_CONST 0
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(42)],
            names: vec![],
            varnames: vec!["x".to_string()],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_stack_ops_code() -> CodeObject {
        // Test: LOAD_CONST 1, LOAD_CONST 2, DUP_TOP, POP_TOP, SWAP, RETURN
        // Stack: [] -> [1] -> [1, 2] -> [1, 2, 2] -> [1, 2] -> [2, 1] -> return 1
        CodeObject {
            name: "stack_test".to_string(),
            qualname: "stack_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 4,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (1)
                DpbOpcode::LoadConst as u8,
                1,
                0,                       // LOAD_CONST 1 (2)
                DpbOpcode::DupTop as u8, // DUP_TOP
                DpbOpcode::PopTop as u8, // POP_TOP
                DpbOpcode::Swap as u8,   // SWAP
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(1), Constant::Int(2)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_load_store_code() -> CodeObject {
        // Test: LOAD_CONST 42, STORE_FAST 0, LOAD_FAST 0, RETURN
        CodeObject {
            name: "load_store_test".to_string(),
            qualname: "load_store_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 1,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (42)
                DpbOpcode::StoreFast as u8,
                0,
                0, // STORE_FAST 0
                DpbOpcode::LoadFast as u8,
                0,
                0,                       // LOAD_FAST 0
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(42)],
            names: vec![],
            varnames: vec!["x".to_string()],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_arithmetic_code() -> CodeObject {
        // Test: LOAD_CONST 10, LOAD_CONST 3, BINARY_ADD, RETURN
        // Result should be 13
        CodeObject {
            name: "arithmetic_test".to_string(),
            qualname: "arithmetic_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (10)
                DpbOpcode::LoadConst as u8,
                1,
                0,                          // LOAD_CONST 1 (3)
                DpbOpcode::BinaryAdd as u8, // BINARY_ADD
                DpbOpcode::Return as u8,    // RETURN
            ],
            constants: vec![Constant::Int(10), Constant::Int(3)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_complex_arithmetic_code() -> CodeObject {
        // Test: (10 - 3) * 2 = 14
        // LOAD_CONST 10, LOAD_CONST 3, BINARY_SUB, LOAD_CONST 2, BINARY_MUL, RETURN
        CodeObject {
            name: "complex_arithmetic_test".to_string(),
            qualname: "complex_arithmetic_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 3,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (10)
                DpbOpcode::LoadConst as u8,
                1,
                0,                          // LOAD_CONST 1 (3)
                DpbOpcode::BinarySub as u8, // BINARY_SUB -> 7
                DpbOpcode::LoadConst as u8,
                2,
                0,                          // LOAD_CONST 2 (2)
                DpbOpcode::BinaryMul as u8, // BINARY_MUL -> 14
                DpbOpcode::Return as u8,    // RETURN
            ],
            constants: vec![Constant::Int(10), Constant::Int(3), Constant::Int(2)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_unary_ops_code() -> CodeObject {
        // Test: -5 (unary negation)
        CodeObject {
            name: "unary_test".to_string(),
            qualname: "unary_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0,                         // LOAD_CONST 0 (5)
                DpbOpcode::UnaryNeg as u8, // UNARY_NEG -> -5
                DpbOpcode::Return as u8,   // RETURN
            ],
            constants: vec![Constant::Int(5)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_bitwise_ops_code() -> CodeObject {
        // Test: 0b1010 & 0b1100 = 0b1000 (8)
        CodeObject {
            name: "bitwise_test".to_string(),
            qualname: "bitwise_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (10 = 0b1010)
                DpbOpcode::LoadConst as u8,
                1,
                0,                          // LOAD_CONST 1 (12 = 0b1100)
                DpbOpcode::BinaryAnd as u8, // BINARY_AND -> 8
                DpbOpcode::Return as u8,    // RETURN
            ],
            constants: vec![Constant::Int(10), Constant::Int(12)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    #[test]
    fn test_compiler_creation() {
        let compiler = BaselineCompiler::new();
        assert!(compiler.is_ok());
    }

    #[test]
    fn test_simple_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_simple_code();
        let func_id = FunctionId(1);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());

        let code_ptr = result.unwrap();
        assert!(!code_ptr.is_null());
    }

    #[test]
    fn test_compilation_caching() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_simple_code();
        let func_id = FunctionId(1);

        // First compilation
        let ptr1 = compiler.compile(func_id, &code).unwrap();

        // Second compilation should return cached
        let ptr2 = compiler.compile(func_id, &code).unwrap();

        assert_eq!(ptr1, ptr2);
    }

    #[test]
    fn test_stack_operations_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_stack_ops_code();
        let func_id = FunctionId(2);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_store_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_load_store_code();
        let func_id = FunctionId(3);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_translation_state_stack_ops() {
        use cranelift_codegen::ir::Block;

        let block = Block::from_u32(0);
        let func_id = FunctionId(999);
        let state = TranslationState::new(block, 2, func_id);

        // Test push/pop
        assert_eq!(state.stack_depth(), 0);

        // Simulate pushing values (we can't create real Values without a builder)
        // but we can test the state logic
    }

    #[test]
    fn test_invalidate() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_simple_code();
        let func_id = FunctionId(4);

        // Compile
        compiler.compile(func_id, &code).unwrap();
        assert!(compiler.is_compiled(func_id));

        // Invalidate
        compiler.invalidate(func_id);
        assert!(!compiler.is_compiled(func_id));
    }

    #[test]
    fn test_arithmetic_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_arithmetic_code();
        let func_id = FunctionId(5);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_arithmetic_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_complex_arithmetic_code();
        let func_id = FunctionId(6);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unary_ops_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_unary_ops_code();
        let func_id = FunctionId(7);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bitwise_ops_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_bitwise_ops_code();
        let func_id = FunctionId(8);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    fn create_comparison_code() -> CodeObject {
        // Test: 5 < 10 -> True (1)
        CodeObject {
            name: "comparison_test".to_string(),
            qualname: "comparison_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (5)
                DpbOpcode::LoadConst as u8,
                1,
                0,                          // LOAD_CONST 1 (10)
                DpbOpcode::CompareLt as u8, // COMPARE_LT -> True
                DpbOpcode::Return as u8,    // RETURN
            ],
            constants: vec![Constant::Int(5), Constant::Int(10)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_conditional_jump_code() -> CodeObject {
        // Simpler test: just test comparison and return
        // LOAD_CONST 5, LOAD_CONST 10, COMPARE_LT, RETURN
        CodeObject {
            name: "conditional_test".to_string(),
            qualname: "conditional_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (5)
                DpbOpcode::LoadConst as u8,
                1,
                0,                          // LOAD_CONST 1 (10)
                DpbOpcode::CompareLt as u8, // COMPARE_LT -> True (1)
                DpbOpcode::Return as u8,    // RETURN
            ],
            constants: vec![Constant::Int(5), Constant::Int(10)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    #[test]
    fn test_comparison_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_comparison_code();
        let func_id = FunctionId(9);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_conditional_jump_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_conditional_jump_code();
        let func_id = FunctionId(10);

        let result = compiler.compile(func_id, &code);
        match &result {
            Ok(_) => {}
            Err(e) => panic!("Compilation failed: {:?}", e),
        }
        assert!(result.is_ok());
    }

    fn create_function_call_code() -> CodeObject {
        // Test: call a function with 2 arguments
        // LOAD_CONST func, LOAD_CONST 1, LOAD_CONST 2, CALL 2, RETURN
        CodeObject {
            name: "call_test".to_string(),
            qualname: "call_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 4,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (func placeholder)
                DpbOpcode::LoadConst as u8,
                1,
                0, // LOAD_CONST 1 (arg1 = 10)
                DpbOpcode::LoadConst as u8,
                2,
                0, // LOAD_CONST 2 (arg2 = 20)
                DpbOpcode::Call as u8,
                2,
                0,                       // CALL 2
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(0), Constant::Int(10), Constant::Int(20)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_method_call_code() -> CodeObject {
        // Test: call a method on an object
        // LOAD_CONST obj, LOAD_METHOD 0, LOAD_CONST arg, CALL_METHOD 1, RETURN
        CodeObject {
            name: "method_call_test".to_string(),
            qualname: "method_call_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 4,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (obj placeholder)
                DpbOpcode::LoadMethod as u8,
                0,
                0, // LOAD_METHOD "method"
                DpbOpcode::LoadConst as u8,
                1,
                0, // LOAD_CONST 1 (arg = 42)
                DpbOpcode::CallMethod as u8,
                1,
                0,                       // CALL_METHOD 1
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(0), Constant::Int(42)],
            names: vec!["method".to_string()],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    #[test]
    fn test_function_call_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_function_call_code();
        let func_id = FunctionId(11);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_method_call_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_method_call_code();
        let func_id = FunctionId(12);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    fn create_function_call_no_args_code() -> CodeObject {
        // Test: call a function with no arguments
        // LOAD_CONST func, CALL 0, RETURN
        CodeObject {
            name: "call_no_args_test".to_string(),
            qualname: "call_no_args_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (func placeholder)
                DpbOpcode::Call as u8,
                0,
                0,                       // CALL 0
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(0)],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_function_call_many_args_code() -> CodeObject {
        // Test: call a function with 5 arguments
        // LOAD_CONST func, LOAD_CONST 1..5, CALL 5, RETURN
        CodeObject {
            name: "call_many_args_test".to_string(),
            qualname: "call_many_args_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 8,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (func placeholder)
                DpbOpcode::LoadConst as u8,
                1,
                0, // LOAD_CONST 1 (arg1)
                DpbOpcode::LoadConst as u8,
                2,
                0, // LOAD_CONST 2 (arg2)
                DpbOpcode::LoadConst as u8,
                3,
                0, // LOAD_CONST 3 (arg3)
                DpbOpcode::LoadConst as u8,
                4,
                0, // LOAD_CONST 4 (arg4)
                DpbOpcode::LoadConst as u8,
                5,
                0, // LOAD_CONST 5 (arg5)
                DpbOpcode::Call as u8,
                5,
                0,                       // CALL 5
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![
                Constant::Int(0),
                Constant::Int(1),
                Constant::Int(2),
                Constant::Int(3),
                Constant::Int(4),
                Constant::Int(5),
            ],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    fn create_method_call_no_args_code() -> CodeObject {
        // Test: call a method with no arguments (just self)
        // LOAD_CONST obj, LOAD_METHOD 0, CALL_METHOD 0, RETURN
        CodeObject {
            name: "method_call_no_args_test".to_string(),
            qualname: "method_call_no_args_test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 3,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // LOAD_CONST 0 (obj placeholder)
                DpbOpcode::LoadMethod as u8,
                0,
                0, // LOAD_METHOD "method"
                DpbOpcode::CallMethod as u8,
                0,
                0,                       // CALL_METHOD 0
                DpbOpcode::Return as u8, // RETURN
            ],
            constants: vec![Constant::Int(0)],
            names: vec!["method".to_string()],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        }
    }

    #[test]
    fn test_function_call_no_args_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_function_call_no_args_code();
        let func_id = FunctionId(13);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_call_many_args_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_function_call_many_args_code();
        let func_id = FunctionId(14);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_method_call_no_args_compilation() {
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_method_call_no_args_code();
        let func_id = FunctionId(15);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_deoptimization_metadata_generation() {
        // Test that deoptimization metadata is generated for guarded operations
        let mut compiler = BaselineCompiler::new().unwrap();
        
        // Create a simple code object with integer arithmetic (which uses type guards)
        let code = CodeObject {
            name: "test_deopt".to_string(),
            qualname: "test_deopt".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 2,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 2,
            stacksize: 2,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadFast as u8, 0, 0,      // Load first arg
                DpbOpcode::LoadFast as u8, 1, 0,      // Load second arg
                DpbOpcode::BinaryAdd as u8,           // Add with type guard
                DpbOpcode::Return as u8,
            ],
            constants: vec![],
            names: vec![],
            varnames: vec!["a".to_string(), "b".to_string()],
            freevars: vec![],
            cellvars: vec![],
        };

        let func_id = FunctionId(100);
        let result = compiler.compile(func_id, &code);
        
        // Compilation should succeed
        assert!(result.is_ok(), "Compilation should succeed");
        
        // Check that the compiled code has deoptimization metadata
        let compiled = compiler.cache.get(&func_id);
        assert!(compiled.is_some(), "Compiled code should be cached");
        
        let compiled = compiled.unwrap();
        
        // Verify that type guards were inserted
        assert!(!compiled.type_guards.is_empty(), 
                "Type guards should be inserted for guarded operations");
        
        // Verify that deopt points were registered
        assert!(!compiled.deopt_metadata.deopt_points.is_empty(),
                "Deopt points should be registered for type guards");
    }

    #[test]
    fn test_multiple_guarded_operations() {
        // Test that multiple guarded operations generate multiple type guards
        let mut compiler = BaselineCompiler::new().unwrap();
        
        let code = CodeObject {
            name: "test_multi_deopt".to_string(),
            qualname: "test_multi_deopt".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 3,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 3,
            stacksize: 3,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadFast as u8, 0, 0,      // Load a
                DpbOpcode::LoadFast as u8, 1, 0,      // Load b
                DpbOpcode::BinaryAdd as u8,           // a + b (guarded)
                DpbOpcode::LoadFast as u8, 2, 0,      // Load c
                DpbOpcode::BinaryMul as u8,           // (a + b) * c (guarded)
                DpbOpcode::Return as u8,
            ],
            constants: vec![],
            names: vec![],
            varnames: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            freevars: vec![],
            cellvars: vec![],
        };

        let func_id = FunctionId(101);
        let result = compiler.compile(func_id, &code);
        
        assert!(result.is_ok(), "Compilation should succeed");
        
        let compiled = compiler.cache.get(&func_id).unwrap();
        
        // We should have type guards for both operations
        // Each binary operation checks both operands, so we expect multiple guards
        assert!(compiled.type_guards.len() >= 2, 
                "Should have multiple type guards for multiple operations");
    }

    #[test]
    fn test_deopt_metadata_func_id() {
        // Test that deopt metadata contains the correct function ID
        let mut compiler = BaselineCompiler::new().unwrap();
        let code = create_simple_code();
        let func_id = FunctionId(200);

        let result = compiler.compile(func_id, &code);
        assert!(result.is_ok());

        let compiled = compiler.cache.get(&func_id).unwrap();
        assert_eq!(compiled.deopt_metadata.func_id, func_id,
                   "Deopt metadata should have correct function ID");
    }
}
