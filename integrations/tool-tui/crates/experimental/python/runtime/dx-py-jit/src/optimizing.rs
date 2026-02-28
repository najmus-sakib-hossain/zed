//! Optimizing JIT Compiler with Type Specialization
//!
//! This module implements the optimizing (Tier 2) JIT compiler that uses
//! type feedback to generate specialized code without type checks for
//! monomorphic sites.

#![allow(clippy::fn_to_numeric_cast)]

use cranelift_codegen::ir::condcodes::{FloatCC, IntCC};
use cranelift_codegen::ir::{types, AbiParam, Block, InstBuilder, MemFlags, UserFuncName, Value};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};
use dashmap::DashMap;
use std::sync::Arc;

use dx_py_bytecode::{CodeObject, Constant, DpbOpcode};

use crate::baseline::JitError;
use crate::helpers::{rt_power, rt_string_compare, rt_string_concat, rt_string_repeat};
use crate::profile::{FunctionProfile, PyType, TypeFeedback, TypeState};
use crate::FunctionId;

/// Compiled optimized code with deoptimization info
pub struct OptimizedCode {
    /// Pointer to the compiled code
    pub code_ptr: *const u8,
    /// Size of the compiled code
    pub code_size: usize,
    /// Function ID in the JIT module
    pub func_id: FuncId,
    /// Deoptimization points
    pub deopt_points: Vec<DeoptPoint>,
}

// Safety: OptimizedCode points to immutable executable memory
unsafe impl Send for OptimizedCode {}
unsafe impl Sync for OptimizedCode {}

/// A deoptimization point in compiled code
#[derive(Debug, Clone)]
pub struct DeoptPoint {
    /// Offset in native code where deopt can occur
    pub native_offset: u32,
    /// Bytecode offset to resume at
    pub bytecode_offset: u32,
    /// Reason for potential deoptimization
    pub reason: DeoptReason,
}

/// Reasons for deoptimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeoptReason {
    /// Type guard failed
    TypeGuardFailed,
    /// Overflow in integer arithmetic
    IntegerOverflow,
    /// Division by zero
    DivisionByZero,
    /// Unexpected None value
    UnexpectedNone,
    /// Array bounds check failed
    BoundsCheckFailed,
}

/// Specialization decision for a bytecode location
#[derive(Debug, Clone)]
pub enum Specialization {
    /// No specialization - use generic code
    Generic,
    /// Specialized for integer operations
    IntSpecialized,
    /// Specialized for float operations
    FloatSpecialized,
    /// Specialized for string operations
    StringSpecialized,
    /// Inline cache with type guards
    InlineCache { types: Vec<PyType> },
}

impl Specialization {
    /// Determine specialization from type feedback
    pub fn from_feedback(feedback: &TypeFeedback) -> Self {
        match feedback.get_state() {
            TypeState::Uninitialized => Self::Generic,
            TypeState::Monomorphic => {
                if let Some(ty) = feedback.get_primary_type() {
                    match ty {
                        PyType::Int | PyType::Bool => Self::IntSpecialized,
                        PyType::Float => Self::FloatSpecialized,
                        PyType::Str => Self::StringSpecialized,
                        _ => Self::Generic,
                    }
                } else {
                    Self::Generic
                }
            }
            TypeState::Polymorphic => {
                let types = feedback.get_types();
                Self::InlineCache { types }
            }
            TypeState::Megamorphic => Self::Generic,
        }
    }

    /// Check if this specialization can use fast integer path
    pub fn is_int_specialized(&self) -> bool {
        matches!(self, Self::IntSpecialized)
    }

    /// Check if this specialization can use fast float path
    pub fn is_float_specialized(&self) -> bool {
        matches!(self, Self::FloatSpecialized)
    }

    /// Check if this specialization can use fast string path
    pub fn is_string_specialized(&self) -> bool {
        matches!(self, Self::StringSpecialized)
    }
}

/// Translation state for optimizing compiler
pub struct OptimizingTranslationState {
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
    /// Deoptimization points collected during compilation
    deopt_points: Vec<DeoptPoint>,
    /// Current bytecode offset (for deopt info)
    current_bc_offset: u32,
}

impl OptimizingTranslationState {
    /// Create a new translation state
    pub fn new(entry_block: Block, num_locals: usize) -> Self {
        Self {
            stack: Vec::with_capacity(32),
            locals: Vec::with_capacity(num_locals),
            current_block: entry_block,
            block_map: std::collections::HashMap::new(),
            next_var: 0,
            deopt_points: Vec::new(),
            current_bc_offset: 0,
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

    /// Record a deoptimization point
    pub fn record_deopt_point(&mut self, reason: DeoptReason) {
        self.deopt_points.push(DeoptPoint {
            native_offset: 0, // Will be filled in after compilation
            bytecode_offset: self.current_bc_offset,
            reason,
        });
    }

    /// Set current bytecode offset
    pub fn set_bc_offset(&mut self, offset: u32) {
        self.current_bc_offset = offset;
    }
}

/// Optimizing JIT compiler with type specialization
pub struct OptimizingCompiler {
    /// Cranelift JIT module
    module: JITModule,
    /// Function builder context (reusable)
    builder_ctx: FunctionBuilderContext,
    /// Compiled function cache
    cache: DashMap<FunctionId, Arc<OptimizedCode>>,
    /// Maximum code size (default 2MB for optimized code)
    #[allow(dead_code)]
    max_code_size: usize,
}

impl OptimizingCompiler {
    /// Create a new optimizing compiler
    pub fn new() -> Result<Self, JitError> {
        // Configure Cranelift for the host architecture with aggressive optimizations
        let mut flag_builder = settings::builder();
        flag_builder
            .set("opt_level", "speed_and_size")
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
            max_code_size: 2 * 1024 * 1024, // 2MB for optimized code
        })
    }

    /// Compile a code object with type specialization
    pub fn compile_optimized(
        &mut self,
        func_id: FunctionId,
        code: &CodeObject,
        profile: &FunctionProfile,
    ) -> Result<Arc<OptimizedCode>, JitError> {
        // Check cache first
        if let Some(compiled) = self.cache.get(&func_id) {
            return Ok(compiled.clone());
        }

        // Create function signature
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64)); // frame pointer
        sig.params.push(AbiParam::new(types::I64)); // args pointer
        sig.params.push(AbiParam::new(types::I32)); // number of args
        sig.returns.push(AbiParam::new(types::I64)); // result pointer

        // Declare the function
        let name = format!("py_opt_func_{}", func_id.0);
        let clif_func_id = self
            .module
            .declare_function(&name, Linkage::Local, &sig)
            .map_err(|e| JitError::ModuleError(e.to_string()))?;

        // Create function context
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        ctx.func.name = UserFuncName::user(0, clif_func_id.as_u32());

        // Build the function with type specialization
        let deopt_points = {
            let mut builder = FunctionBuilder::new(&mut ctx.func, &mut self.builder_ctx);
            let result = Self::translate_optimized_impl(&mut builder, code, profile);
            if result.is_ok() {
                builder.finalize();
            }
            result
        };

        let deopt_points = match deopt_points {
            Ok(points) => points,
            Err(e) => {
                self.module.clear_context(&mut ctx);
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

        // Create optimized code with deopt info
        let compiled = Arc::new(OptimizedCode {
            code_ptr,
            code_size: 0,
            func_id: clif_func_id,
            deopt_points,
        });
        self.cache.insert(func_id, compiled.clone());

        Ok(compiled)
    }

    /// Translate a function with type specialization (static method to avoid borrow issues)
    fn translate_optimized_impl(
        builder: &mut FunctionBuilder,
        code: &CodeObject,
        profile: &FunctionProfile,
    ) -> Result<Vec<DeoptPoint>, JitError> {
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
        let mut state = OptimizingTranslationState::new(entry_block, code.nlocals as usize);

        // Declare local variables
        for _ in 0..code.nlocals {
            let var = state.new_variable();
            builder.declare_var(var, types::I64);
            state.locals.push(var);
        }

        // Initialize locals from arguments
        for i in 0..code.argcount.min(code.nlocals) {
            let idx_val = builder.ins().iconst(types::I64, i as i64);
            let offset = builder.ins().imul_imm(idx_val, 8);
            let arg_addr = builder.ins().iadd(args_ptr, offset);
            let arg_val = builder.ins().load(types::I64, MemFlags::new(), arg_addr, 0);
            builder.def_var(state.locals[i as usize], arg_val);
        }

        // Translate bytecode with type specialization
        let bytecode = &code.code;
        let mut offset = 0;
        let mut returned = false;

        while offset < bytecode.len() {
            state.set_bc_offset(offset as u32);

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

            // Get type feedback for this location
            let specialization = profile
                .type_feedback
                .get(offset)
                .map(Specialization::from_feedback)
                .unwrap_or(Specialization::Generic);

            Self::translate_instruction_optimized_impl(
                builder,
                &mut state,
                opcode,
                arg as u16,
                code,
                &specialization,
            )?;

            offset += 1 + arg_size;
        }

        // If we haven't returned yet, return None
        if !returned {
            let none_val = builder.ins().iconst(types::I64, 0);
            builder.ins().return_(&[none_val]);
        }

        Ok(state.deopt_points)
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

    /// Translate a single instruction with type specialization (static method)
    fn translate_instruction_optimized_impl(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        opcode: DpbOpcode,
        arg: u16,
        code: &CodeObject,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        match opcode {
            // ===== Stack operations (same as baseline) =====
            DpbOpcode::PopTop => {
                state.pop();
            }

            DpbOpcode::DupTop => {
                if let Some(val) = state.peek() {
                    state.push(val);
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
                    let const_val = Self::load_constant_optimized(builder, &code.constants[idx]);
                    state.push(const_val);
                }
            }

            DpbOpcode::PushNull => {
                let null_val = builder.ins().iconst(types::I64, 0);
                state.push(null_val);
            }

            // ===== Type-Specialized Binary Arithmetic =====
            DpbOpcode::BinaryAdd => {
                Self::emit_specialized_binary_add(builder, state, specialization)?;
            }

            DpbOpcode::BinarySub => {
                Self::emit_specialized_binary_sub(builder, state, specialization)?;
            }

            DpbOpcode::BinaryMul => {
                Self::emit_specialized_binary_mul(builder, state, specialization)?;
            }

            DpbOpcode::BinaryDiv => {
                Self::emit_specialized_binary_div(builder, state, specialization)?;
            }

            DpbOpcode::BinaryFloorDiv => {
                Self::emit_specialized_binary_floordiv(builder, state, specialization)?;
            }

            DpbOpcode::BinaryMod => {
                Self::emit_specialized_binary_mod(builder, state, specialization)?;
            }

            DpbOpcode::BinaryPow => {
                // Power operation with specialization for small constant exponents
                Self::emit_specialized_binary_pow(builder, state, specialization, code, arg)?;
            }

            // ===== Bitwise operations (always integer) =====
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
                    let result = builder.ins().sshr(a, b);
                    state.push(result);
                }
            }

            // ===== Unary Operations =====
            DpbOpcode::UnaryNeg => {
                Self::emit_specialized_unary_neg(builder, state, specialization)?;
            }

            DpbOpcode::UnaryPos => {
                // Unary positive is a no-op for numbers
            }

            DpbOpcode::UnaryInvert => {
                if let Some(a) = state.pop() {
                    let result = builder.ins().bnot(a);
                    state.push(result);
                }
            }

            DpbOpcode::UnaryNot => {
                if let Some(a) = state.pop() {
                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_zero = builder.ins().icmp(IntCC::Equal, a, zero);
                    let result = builder.ins().uextend(types::I64, is_zero);
                    state.push(result);
                }
            }

            // ===== In-place Operations (use specialized paths) =====
            DpbOpcode::InplaceAdd => {
                Self::emit_specialized_binary_add(builder, state, specialization)?;
            }

            DpbOpcode::InplaceSub => {
                Self::emit_specialized_binary_sub(builder, state, specialization)?;
            }

            DpbOpcode::InplaceMul => {
                Self::emit_specialized_binary_mul(builder, state, specialization)?;
            }

            DpbOpcode::InplaceDiv => {
                Self::emit_specialized_binary_div(builder, state, specialization)?;
            }

            DpbOpcode::InplaceFloorDiv => {
                Self::emit_specialized_binary_floordiv(builder, state, specialization)?;
            }

            DpbOpcode::InplaceMod => {
                Self::emit_specialized_binary_mod(builder, state, specialization)?;
            }

            DpbOpcode::InplacePow => {
                // In-place power operation with specialization for small constant exponents
                Self::emit_specialized_binary_pow(builder, state, specialization, code, arg)?;
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
                let next_block = builder.create_block();
                builder.switch_to_block(next_block);
                builder.seal_block(next_block);
            }

            // ===== NOP =====
            DpbOpcode::Nop => {}

            // ===== Control Flow =====
            DpbOpcode::Jump => {
                let target_offset = arg as usize;
                let target_block = state.get_or_create_block(target_offset, builder);
                builder.ins().jump(target_block, &[]);
                let next_block = builder.create_block();
                builder.switch_to_block(next_block);
                builder.seal_block(next_block);
            }

            DpbOpcode::PopJumpIfTrue => {
                if let Some(cond) = state.pop() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_true = builder.ins().icmp(IntCC::NotEqual, cond, zero);
                    builder.ins().brif(is_true, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            DpbOpcode::PopJumpIfFalse => {
                if let Some(cond) = state.pop() {
                    let target_offset = arg as usize;
                    let target_block = state.get_or_create_block(target_offset, builder);
                    let fallthrough_block = builder.create_block();

                    let zero = builder.ins().iconst(types::I64, 0);
                    let is_false = builder.ins().icmp(IntCC::Equal, cond, zero);
                    builder.ins().brif(is_false, target_block, &[], fallthrough_block, &[]);
                    builder.switch_to_block(fallthrough_block);
                    builder.seal_block(fallthrough_block);
                }
            }

            // ===== Type-Specialized Comparison Operations =====
            DpbOpcode::CompareLt => {
                Self::emit_specialized_compare(
                    builder,
                    state,
                    specialization,
                    IntCC::SignedLessThan,
                    FloatCC::LessThan,
                )?;
            }

            DpbOpcode::CompareLe => {
                Self::emit_specialized_compare(
                    builder,
                    state,
                    specialization,
                    IntCC::SignedLessThanOrEqual,
                    FloatCC::LessThanOrEqual,
                )?;
            }

            DpbOpcode::CompareEq => {
                Self::emit_specialized_compare(
                    builder,
                    state,
                    specialization,
                    IntCC::Equal,
                    FloatCC::Equal,
                )?;
            }

            DpbOpcode::CompareNe => {
                Self::emit_specialized_compare(
                    builder,
                    state,
                    specialization,
                    IntCC::NotEqual,
                    FloatCC::NotEqual,
                )?;
            }

            DpbOpcode::CompareGt => {
                Self::emit_specialized_compare(
                    builder,
                    state,
                    specialization,
                    IntCC::SignedGreaterThan,
                    FloatCC::GreaterThan,
                )?;
            }

            DpbOpcode::CompareGe => {
                Self::emit_specialized_compare(
                    builder,
                    state,
                    specialization,
                    IntCC::SignedGreaterThanOrEqual,
                    FloatCC::GreaterThanOrEqual,
                )?;
            }

            DpbOpcode::CompareIs => {
                // Identity comparison - always use pointer equality
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp = builder.ins().icmp(IntCC::Equal, a, b);
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareIsNot => {
                // Identity comparison - always use pointer inequality
                if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
                    let cmp = builder.ins().icmp(IntCC::NotEqual, a, b);
                    let result = builder.ins().uextend(types::I64, cmp);
                    state.push(result);
                }
            }

            DpbOpcode::CompareIn | DpbOpcode::CompareNotIn => {
                // Membership tests require runtime support
                // Pop both operands and push placeholder result
                if let (Some(_container), Some(_item)) = (state.pop(), state.pop()) {
                    let result = if opcode == DpbOpcode::CompareIn {
                        builder.ins().iconst(types::I64, 0) // Placeholder: false
                    } else {
                        builder.ins().iconst(types::I64, 1) // Placeholder: true
                    };
                    state.push(result);
                }
            }

            // Default: fall back to generic handling
            _ => {
                // For unsupported opcodes, we could either:
                // 1. Return an error and fall back to interpreter
                // 2. Generate a call to a runtime helper
                // For now, we'll just skip them (no-op)
            }
        }

        Ok(())
    }

    // ===== Type-Specialized Comparison Operations =====

    /// Emit type-specialized comparison operation
    fn emit_specialized_compare(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
        int_cc: IntCC,
        float_cc: FloatCC,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => {
                    // Fast path: direct integer comparison
                    Self::emit_int_compare(builder, a, b, int_cc)
                }
                Specialization::FloatSpecialized => {
                    // Fast path: direct float comparison
                    Self::emit_float_compare(builder, a, b, float_cc)
                }
                Specialization::StringSpecialized => {
                    // String comparison via runtime helper
                    Self::emit_string_compare(builder, a, b, int_cc)
                }
                Specialization::InlineCache { types } => {
                    // Polymorphic: check for int/float and use appropriate comparison
                    Self::emit_inline_cache_compare(builder, state, a, b, types, int_cc, float_cc)
                }
                Specialization::Generic => {
                    // Generic path: assume integer comparison
                    Self::emit_int_compare(builder, a, b, int_cc)
                }
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit integer comparison
    fn emit_int_compare(builder: &mut FunctionBuilder, a: Value, b: Value, cc: IntCC) -> Value {
        let cmp = builder.ins().icmp(cc, a, b);
        builder.ins().uextend(types::I64, cmp)
    }

    /// Emit float comparison
    fn emit_float_compare(builder: &mut FunctionBuilder, a: Value, b: Value, cc: FloatCC) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        let cmp = builder.ins().fcmp(cc, a_f64, b_f64);
        builder.ins().uextend(types::I64, cmp)
    }

    /// Emit string comparison via runtime helper
    /// Calls rt_string_compare and converts result to boolean based on comparison type
    fn emit_string_compare(
        builder: &mut FunctionBuilder,
        a: Value,
        b: Value,
        int_cc: IntCC,
    ) -> Value {
        // Call rt_string_compare runtime helper
        // Returns -1 if a < b, 0 if a == b, 1 if a > b
        let helper_addr = builder.ins().iconst(types::I64, rt_string_compare as i64);

        // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> i64
        let sig = cranelift_codegen::ir::Signature {
            params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
            returns: vec![AbiParam::new(types::I64)],
            call_conv: cranelift_codegen::isa::CallConv::SystemV,
        };
        let sig_ref = builder.import_signature(sig);

        // Call the runtime helper
        let call = builder.ins().call_indirect(sig_ref, helper_addr, &[a, b]);
        let cmp_result = builder.inst_results(call)[0];

        // Convert comparison result to boolean based on the comparison type
        let zero = builder.ins().iconst(types::I64, 0);
        let cmp = builder.ins().icmp(int_cc, cmp_result, zero);
        builder.ins().uextend(types::I64, cmp)
    }

    /// Emit inline cache for polymorphic comparison
    fn emit_inline_cache_compare(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        a: Value,
        b: Value,
        types: &[PyType],
        int_cc: IntCC,
        float_cc: FloatCC,
    ) -> Value {
        let has_int = types.iter().any(|t| matches!(t, PyType::Int | PyType::Bool));
        let has_float = types.iter().any(|t| matches!(t, PyType::Float));

        if has_int && has_float {
            // Mixed int/float - record deopt point and use int comparison
            state.record_deopt_point(DeoptReason::TypeGuardFailed);
            Self::emit_int_compare(builder, a, b, int_cc)
        } else if has_float {
            Self::emit_float_compare(builder, a, b, float_cc)
        } else {
            Self::emit_int_compare(builder, a, b, int_cc)
        }
    }

    // ===== Type-Specialized Code Generation =====

    /// Emit specialized integer addition (no type checks)
    fn emit_int_add(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().iadd(a, b)
    }

    /// Emit specialized float addition (no type checks)
    fn emit_float_add(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        // Convert i64 bit patterns to f64 and add
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        let result = builder.ins().fadd(a_f64, b_f64);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized string concatenation
    /// Calls rt_string_concat runtime helper for actual string concatenation
    fn emit_string_concat(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        // Call rt_string_concat runtime helper for string concatenation
        let helper_addr = builder.ins().iconst(types::I64, rt_string_concat as i64);

        // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> PyObjectPtr
        let sig = cranelift_codegen::ir::Signature {
            params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
            returns: vec![AbiParam::new(types::I64)],
            call_conv: cranelift_codegen::isa::CallConv::SystemV,
        };
        let sig_ref = builder.import_signature(sig);

        // Call the runtime helper
        let call = builder.ins().call_indirect(sig_ref, helper_addr, &[a, b]);
        builder.inst_results(call)[0]
    }

    /// Emit specialized binary add based on type feedback
    fn emit_specialized_binary_add(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => {
                    // Fast path: direct integer addition without type checks
                    Self::emit_int_add(builder, a, b)
                }
                Specialization::FloatSpecialized => {
                    // Fast path: direct float addition without type checks
                    Self::emit_float_add(builder, a, b)
                }
                Specialization::StringSpecialized => {
                    // String concatenation
                    Self::emit_string_concat(builder, a, b)
                }
                Specialization::InlineCache { types } => {
                    // Polymorphic: generate inline cache with type guards
                    Self::emit_inline_cache_add(builder, state, a, b, types)
                }
                Specialization::Generic => {
                    // Generic path: assume integer for now
                    builder.ins().iadd(a, b)
                }
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit specialized integer subtraction
    fn emit_int_sub(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().isub(a, b)
    }

    /// Emit specialized float subtraction
    fn emit_float_sub(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        let result = builder.ins().fsub(a_f64, b_f64);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized binary sub based on type feedback
    fn emit_specialized_binary_sub(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => Self::emit_int_sub(builder, a, b),
                Specialization::FloatSpecialized => Self::emit_float_sub(builder, a, b),
                Specialization::InlineCache { types } => {
                    Self::emit_inline_cache_sub(builder, state, a, b, types)
                }
                _ => builder.ins().isub(a, b),
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit specialized integer multiplication
    fn emit_int_mul(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().imul(a, b)
    }

    /// Emit specialized float multiplication
    fn emit_float_mul(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        let result = builder.ins().fmul(a_f64, b_f64);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized string repetition (str * int)
    /// Calls rt_string_repeat runtime helper for actual string repetition
    fn emit_string_repeat(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        // Call rt_string_repeat runtime helper for string repetition
        let helper_addr = builder.ins().iconst(types::I64, rt_string_repeat as i64);

        // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> PyObjectPtr
        let sig = cranelift_codegen::ir::Signature {
            params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
            returns: vec![AbiParam::new(types::I64)],
            call_conv: cranelift_codegen::isa::CallConv::SystemV,
        };
        let sig_ref = builder.import_signature(sig);

        // Call the runtime helper (string, count)
        let call = builder.ins().call_indirect(sig_ref, helper_addr, &[a, b]);
        builder.inst_results(call)[0]
    }

    /// Emit specialized binary mul based on type feedback
    fn emit_specialized_binary_mul(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => Self::emit_int_mul(builder, a, b),
                Specialization::FloatSpecialized => Self::emit_float_mul(builder, a, b),
                Specialization::StringSpecialized => Self::emit_string_repeat(builder, a, b),
                Specialization::InlineCache { types } => {
                    Self::emit_inline_cache_mul(builder, state, a, b, types)
                }
                _ => builder.ins().imul(a, b),
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit specialized integer division
    fn emit_int_div(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        // True division for integers - convert to float first
        let a_f64 = builder.ins().fcvt_from_sint(types::F64, a);
        let b_f64 = builder.ins().fcvt_from_sint(types::F64, b);
        let result = builder.ins().fdiv(a_f64, b_f64);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized float division
    fn emit_float_div(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        let result = builder.ins().fdiv(a_f64, b_f64);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized binary div based on type feedback
    fn emit_specialized_binary_div(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => Self::emit_int_div(builder, a, b),
                Specialization::FloatSpecialized => Self::emit_float_div(builder, a, b),
                Specialization::InlineCache { types } => {
                    Self::emit_inline_cache_div(builder, state, a, b, types)
                }
                _ => builder.ins().sdiv(a, b),
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit specialized integer floor division
    fn emit_int_floordiv(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().sdiv(a, b)
    }

    /// Emit specialized float floor division
    fn emit_float_floordiv(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        let result = builder.ins().fdiv(a_f64, b_f64);
        let floored = builder.ins().floor(result);
        builder.ins().bitcast(types::I64, MemFlags::new(), floored)
    }

    /// Emit specialized binary floordiv based on type feedback
    fn emit_specialized_binary_floordiv(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => Self::emit_int_floordiv(builder, a, b),
                Specialization::FloatSpecialized => Self::emit_float_floordiv(builder, a, b),
                _ => builder.ins().sdiv(a, b),
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit specialized integer modulo
    fn emit_int_mod(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        builder.ins().srem(a, b)
    }

    /// Emit specialized float modulo
    fn emit_float_mod(builder: &mut FunctionBuilder, a: Value, b: Value) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let b_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), b);
        // fmod: a - floor(a/b) * b
        let div = builder.ins().fdiv(a_f64, b_f64);
        let floored = builder.ins().floor(div);
        let mul = builder.ins().fmul(floored, b_f64);
        let result = builder.ins().fsub(a_f64, mul);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized binary mod based on type feedback
    fn emit_specialized_binary_mod(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let (Some(b), Some(a)) = (state.pop(), state.pop()) {
            let result = match specialization {
                Specialization::IntSpecialized => Self::emit_int_mod(builder, a, b),
                Specialization::FloatSpecialized => Self::emit_float_mod(builder, a, b),
                _ => builder.ins().srem(a, b),
            };
            state.push(result);
        }
        Ok(())
    }

    /// Emit specialized binary power operation
    /// Optimizes for small constant exponents (0, 1, 2) with inline code
    fn emit_specialized_binary_pow(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        _specialization: &Specialization,
        code: &CodeObject,
        _arg: u16,
    ) -> Result<(), JitError> {
        if let (Some(exp), Some(base)) = (state.pop(), state.pop()) {
            // Try to detect constant exponents for optimization
            // Check if the exponent might be a small constant
            // For now, we'll use the runtime helper for all cases
            // but the structure is here for future constant folding

            // Call rt_power runtime helper
            let helper_addr = builder.ins().iconst(types::I64, rt_power as i64);

            // Create signature for the helper: (PyObjectPtr, PyObjectPtr) -> PyObjectPtr
            let sig = cranelift_codegen::ir::Signature {
                params: vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                returns: vec![AbiParam::new(types::I64)],
                call_conv: cranelift_codegen::isa::CallConv::SystemV,
            };
            let sig_ref = builder.import_signature(sig);

            // Call the runtime helper
            let call = builder.ins().call_indirect(sig_ref, helper_addr, &[base, exp]);
            let result = builder.inst_results(call)[0];
            state.push(result);

            // Reference code to avoid unused warning
            let _ = code;
        }
        Ok(())
    }

    /// Emit inline power for x**2 (square)
    /// This is a common case that can be optimized to a single multiplication
    #[allow(dead_code)]
    fn emit_int_square(builder: &mut FunctionBuilder, base: Value) -> Value {
        builder.ins().imul(base, base)
    }

    /// Emit inline power for x**0 (always 1)
    #[allow(dead_code)]
    fn emit_power_zero(builder: &mut FunctionBuilder) -> Value {
        builder.ins().iconst(types::I64, 1)
    }

    /// Emit inline power for x**1 (identity)
    #[allow(dead_code)]
    fn emit_power_one(base: Value) -> Value {
        base
    }

    /// Emit specialized integer negation
    fn emit_int_neg(builder: &mut FunctionBuilder, a: Value) -> Value {
        builder.ins().ineg(a)
    }

    /// Emit specialized float negation
    fn emit_float_neg(builder: &mut FunctionBuilder, a: Value) -> Value {
        let a_f64 = builder.ins().bitcast(types::F64, MemFlags::new(), a);
        let result = builder.ins().fneg(a_f64);
        builder.ins().bitcast(types::I64, MemFlags::new(), result)
    }

    /// Emit specialized unary neg based on type feedback
    fn emit_specialized_unary_neg(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        specialization: &Specialization,
    ) -> Result<(), JitError> {
        if let Some(a) = state.pop() {
            let result = match specialization {
                Specialization::IntSpecialized => Self::emit_int_neg(builder, a),
                Specialization::FloatSpecialized => Self::emit_float_neg(builder, a),
                _ => builder.ins().ineg(a),
            };
            state.push(result);
        }
        Ok(())
    }

    // ===== Inline Cache Generation for Polymorphic Sites =====

    /// Type tag constants for runtime type checking
    /// These match the PyType enum values
    const TYPE_TAG_INT: i64 = 3;
    #[allow(dead_code)]
    const TYPE_TAG_FLOAT: i64 = 4;
    #[allow(dead_code)]
    const TYPE_TAG_STR: i64 = 5;
    #[allow(dead_code)]
    const TYPE_TAG_LIST: i64 = 7;

    /// Emit a type guard that checks if a value has the expected type tag
    /// Returns the comparison result (1 if match, 0 if not)
    fn emit_type_guard(
        builder: &mut FunctionBuilder,
        value: Value,
        expected_type_tag: i64,
    ) -> Value {
        // In a real implementation, we'd extract the type tag from the object header
        // For now, we assume the type tag is stored in the low bits of the value
        // This is a placeholder - real implementation would load from object header
        let type_mask = builder.ins().iconst(types::I64, 0xFF);
        let type_tag = builder.ins().band(value, type_mask);
        let expected = builder.ins().iconst(types::I64, expected_type_tag);
        let cmp = builder.ins().icmp(IntCC::Equal, type_tag, expected);
        builder.ins().uextend(types::I64, cmp)
    }

    /// Emit inline cache with type guards for polymorphic addition
    /// Generates a chain of type checks with specialized code for each type
    fn emit_inline_cache_add(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        a: Value,
        b: Value,
        types: &[PyType],
    ) -> Value {
        // For polymorphic sites, we generate a chain of type checks
        // with specialized code for each observed type

        // Analyze observed types
        let has_int = types.iter().any(|t| matches!(t, PyType::Int | PyType::Bool));
        let has_float = types.iter().any(|t| matches!(t, PyType::Float));
        let has_str = types.iter().any(|t| matches!(t, PyType::Str));

        // If we have multiple numeric types, generate type guards
        if has_int && has_float {
            // Generate: if is_int { int_add } else if is_float { float_add } else { deopt }
            // For now, use a simplified version that checks int first
            let int_check = Self::emit_type_guard(builder, a, Self::TYPE_TAG_INT);
            let zero = builder.ins().iconst(types::I64, 0);
            let is_int = builder.ins().icmp(IntCC::NotEqual, int_check, zero);

            // Create blocks for the inline cache
            let int_block = builder.create_block();
            let float_block = builder.create_block();
            let merge_block = builder.create_block();

            // Add block parameter for the result
            builder.append_block_param(merge_block, types::I64);

            // Branch based on type check
            builder.ins().brif(is_int, int_block, &[], float_block, &[]);

            // Int path
            builder.switch_to_block(int_block);
            builder.seal_block(int_block);
            let int_result = Self::emit_int_add(builder, a, b);
            builder.ins().jump(merge_block, &[int_result]);

            // Float path
            builder.switch_to_block(float_block);
            builder.seal_block(float_block);
            let float_result = Self::emit_float_add(builder, a, b);
            builder.ins().jump(merge_block, &[float_result]);

            // Merge block
            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            // Record deopt point for type guard failure
            state.record_deopt_point(DeoptReason::TypeGuardFailed);

            builder.block_params(merge_block)[0]
        } else if has_int && has_str {
            // Int + Str is an error in Python, but we might see it in polymorphic code
            // Use int path and record deopt point
            state.record_deopt_point(DeoptReason::TypeGuardFailed);
            Self::emit_int_add(builder, a, b)
        } else if has_int {
            Self::emit_int_add(builder, a, b)
        } else if has_float {
            Self::emit_float_add(builder, a, b)
        } else if has_str {
            Self::emit_string_concat(builder, a, b)
        } else {
            // Generic fallback
            builder.ins().iadd(a, b)
        }
    }

    /// Emit inline cache with type guards for polymorphic subtraction
    fn emit_inline_cache_sub(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        a: Value,
        b: Value,
        types: &[PyType],
    ) -> Value {
        let has_int = types.iter().any(|t| matches!(t, PyType::Int | PyType::Bool));
        let has_float = types.iter().any(|t| matches!(t, PyType::Float));

        if has_int && has_float {
            // Generate type-guarded inline cache
            let int_check = Self::emit_type_guard(builder, a, Self::TYPE_TAG_INT);
            let zero = builder.ins().iconst(types::I64, 0);
            let is_int = builder.ins().icmp(IntCC::NotEqual, int_check, zero);

            let int_block = builder.create_block();
            let float_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            builder.ins().brif(is_int, int_block, &[], float_block, &[]);

            builder.switch_to_block(int_block);
            builder.seal_block(int_block);
            let int_result = Self::emit_int_sub(builder, a, b);
            builder.ins().jump(merge_block, &[int_result]);

            builder.switch_to_block(float_block);
            builder.seal_block(float_block);
            let float_result = Self::emit_float_sub(builder, a, b);
            builder.ins().jump(merge_block, &[float_result]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            state.record_deopt_point(DeoptReason::TypeGuardFailed);
            builder.block_params(merge_block)[0]
        } else if has_int {
            Self::emit_int_sub(builder, a, b)
        } else if has_float {
            Self::emit_float_sub(builder, a, b)
        } else {
            builder.ins().isub(a, b)
        }
    }

    /// Emit inline cache with type guards for polymorphic multiplication
    fn emit_inline_cache_mul(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        a: Value,
        b: Value,
        types: &[PyType],
    ) -> Value {
        let has_int = types.iter().any(|t| matches!(t, PyType::Int | PyType::Bool));
        let has_float = types.iter().any(|t| matches!(t, PyType::Float));
        let has_str = types.iter().any(|t| matches!(t, PyType::Str));

        if has_int && has_float {
            // Generate type-guarded inline cache
            let int_check = Self::emit_type_guard(builder, a, Self::TYPE_TAG_INT);
            let zero = builder.ins().iconst(types::I64, 0);
            let is_int = builder.ins().icmp(IntCC::NotEqual, int_check, zero);

            let int_block = builder.create_block();
            let float_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            builder.ins().brif(is_int, int_block, &[], float_block, &[]);

            builder.switch_to_block(int_block);
            builder.seal_block(int_block);
            let int_result = Self::emit_int_mul(builder, a, b);
            builder.ins().jump(merge_block, &[int_result]);

            builder.switch_to_block(float_block);
            builder.seal_block(float_block);
            let float_result = Self::emit_float_mul(builder, a, b);
            builder.ins().jump(merge_block, &[float_result]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            state.record_deopt_point(DeoptReason::TypeGuardFailed);
            builder.block_params(merge_block)[0]
        } else if has_str && has_int {
            // String repetition: str * int
            state.record_deopt_point(DeoptReason::TypeGuardFailed);
            Self::emit_string_repeat(builder, a, b)
        } else if has_int {
            Self::emit_int_mul(builder, a, b)
        } else if has_float {
            Self::emit_float_mul(builder, a, b)
        } else {
            builder.ins().imul(a, b)
        }
    }

    /// Emit inline cache with type guards for polymorphic division
    fn emit_inline_cache_div(
        builder: &mut FunctionBuilder,
        state: &mut OptimizingTranslationState,
        a: Value,
        b: Value,
        types: &[PyType],
    ) -> Value {
        let has_int = types.iter().any(|t| matches!(t, PyType::Int | PyType::Bool));
        let has_float = types.iter().any(|t| matches!(t, PyType::Float));

        if has_int && has_float {
            // Generate type-guarded inline cache
            let int_check = Self::emit_type_guard(builder, a, Self::TYPE_TAG_INT);
            let zero = builder.ins().iconst(types::I64, 0);
            let is_int = builder.ins().icmp(IntCC::NotEqual, int_check, zero);

            let int_block = builder.create_block();
            let float_block = builder.create_block();
            let merge_block = builder.create_block();
            builder.append_block_param(merge_block, types::I64);

            builder.ins().brif(is_int, int_block, &[], float_block, &[]);

            builder.switch_to_block(int_block);
            builder.seal_block(int_block);
            let int_result = Self::emit_int_div(builder, a, b);
            builder.ins().jump(merge_block, &[int_result]);

            builder.switch_to_block(float_block);
            builder.seal_block(float_block);
            let float_result = Self::emit_float_div(builder, a, b);
            builder.ins().jump(merge_block, &[float_result]);

            builder.switch_to_block(merge_block);
            builder.seal_block(merge_block);

            state.record_deopt_point(DeoptReason::TypeGuardFailed);
            builder.block_params(merge_block)[0]
        } else if has_int {
            Self::emit_int_div(builder, a, b)
        } else if has_float {
            Self::emit_float_div(builder, a, b)
        } else {
            builder.ins().sdiv(a, b)
        }
    }

    /// Load a constant value
    fn load_constant_optimized(builder: &mut FunctionBuilder, constant: &Constant) -> Value {
        match constant {
            Constant::None => builder.ins().iconst(types::I64, 0),
            Constant::Bool(b) => builder.ins().iconst(types::I64, if *b { 1 } else { 0 }),
            Constant::Int(i) => builder.ins().iconst(types::I64, *i),
            Constant::Float(f) => {
                let bits = f.to_bits() as i64;
                builder.ins().iconst(types::I64, bits)
            }
            Constant::String(_) => {
                // String constants need runtime support
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
            Constant::Bytes(_) => {
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
            Constant::Tuple(_) => {
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
            Constant::Code(_) => {
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
            Constant::FrozenSet(_) => {
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
            Constant::Complex(_, _) => {
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
            Constant::Ellipsis => {
                builder.ins().iconst(types::I64, 0) // Placeholder
            }
        }
    }

    /// Check if a function is cached
    pub fn is_cached(&self, func_id: &FunctionId) -> bool {
        self.cache.contains_key(func_id)
    }

    /// Get cached optimized code
    pub fn get_cached(&self, func_id: &FunctionId) -> Option<Arc<OptimizedCode>> {
        self.cache.get(func_id).map(|r| r.clone())
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for OptimizingCompiler {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            // Log the error and panic with a clear message
            // This is acceptable in Default since it's typically called during initialization
            panic!("Failed to create optimizing compiler: {}. This is a critical error.", e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_specialization_from_feedback() {
        let feedback = TypeFeedback::new();

        // Uninitialized -> Generic
        assert!(matches!(Specialization::from_feedback(&feedback), Specialization::Generic));

        // Single int -> IntSpecialized
        feedback.record(PyType::Int);
        assert!(matches!(
            Specialization::from_feedback(&feedback),
            Specialization::IntSpecialized
        ));

        // Add float -> InlineCache
        feedback.record(PyType::Float);
        assert!(matches!(
            Specialization::from_feedback(&feedback),
            Specialization::InlineCache { .. }
        ));
    }

    #[test]
    fn test_specialization_float() {
        let feedback = TypeFeedback::new();
        feedback.record(PyType::Float);

        assert!(matches!(
            Specialization::from_feedback(&feedback),
            Specialization::FloatSpecialized
        ));
    }

    #[test]
    fn test_specialization_string() {
        let feedback = TypeFeedback::new();
        feedback.record(PyType::Str);

        assert!(matches!(
            Specialization::from_feedback(&feedback),
            Specialization::StringSpecialized
        ));
    }

    #[test]
    fn test_specialization_methods() {
        let int_spec = Specialization::IntSpecialized;
        assert!(int_spec.is_int_specialized());
        assert!(!int_spec.is_float_specialized());
        assert!(!int_spec.is_string_specialized());

        let float_spec = Specialization::FloatSpecialized;
        assert!(!float_spec.is_int_specialized());
        assert!(float_spec.is_float_specialized());
        assert!(!float_spec.is_string_specialized());

        let str_spec = Specialization::StringSpecialized;
        assert!(!str_spec.is_int_specialized());
        assert!(!str_spec.is_float_specialized());
        assert!(str_spec.is_string_specialized());
    }

    #[test]
    fn test_deopt_point() {
        let deopt = DeoptPoint {
            native_offset: 100,
            bytecode_offset: 50,
            reason: DeoptReason::TypeGuardFailed,
        };

        assert_eq!(deopt.native_offset, 100);
        assert_eq!(deopt.bytecode_offset, 50);
        assert_eq!(deopt.reason, DeoptReason::TypeGuardFailed);
    }

    #[test]
    fn test_translation_state() {
        use cranelift_codegen::ir::Function;
        use cranelift_frontend::FunctionBuilderContext;

        // Create a minimal function builder context for testing
        let mut ctx = FunctionBuilderContext::new();
        let mut func = Function::new();
        let mut builder = FunctionBuilder::new(&mut func, &mut ctx);

        let entry = builder.create_block();
        builder.switch_to_block(entry);

        let mut state = OptimizingTranslationState::new(entry, 5);

        // Test variable allocation
        let var1 = state.new_variable();
        let var2 = state.new_variable();
        assert_ne!(var1, var2);

        // Test bytecode offset tracking
        state.set_bc_offset(42);
        assert_eq!(state.current_bc_offset, 42);

        // Test deopt point recording
        state.record_deopt_point(DeoptReason::IntegerOverflow);
        assert_eq!(state.deopt_points.len(), 1);
        assert_eq!(state.deopt_points[0].bytecode_offset, 42);
        assert_eq!(state.deopt_points[0].reason, DeoptReason::IntegerOverflow);
    }

    #[test]
    fn test_optimizing_compiler_creation() {
        let compiler = OptimizingCompiler::new();
        assert!(compiler.is_ok());

        let compiler = compiler.unwrap();
        assert_eq!(compiler.cache_size(), 0);
        assert_eq!(compiler.max_code_size, 2 * 1024 * 1024);
    }

    #[test]
    fn test_inline_cache_types() {
        let feedback = TypeFeedback::new();
        feedback.record(PyType::Int);
        feedback.record(PyType::Float);

        let spec = Specialization::from_feedback(&feedback);
        if let Specialization::InlineCache { types } = spec {
            assert!(types.contains(&PyType::Int));
            assert!(types.contains(&PyType::Float));
        } else {
            panic!("Expected InlineCache specialization");
        }
    }

    #[test]
    fn test_megamorphic_fallback() {
        let feedback = TypeFeedback::new();
        // Record 5 different types to trigger megamorphic
        feedback.record(PyType::Int);
        feedback.record(PyType::Float);
        feedback.record(PyType::Str);
        feedback.record(PyType::List);
        feedback.record(PyType::Dict); // This won't be stored (max 4)

        // With 4 types stored, it's still polymorphic
        // The megamorphic state is tracked by type_count > 4
        // which happens when we try to add a 5th unique type
        let spec = Specialization::from_feedback(&feedback);
        // Should be InlineCache since we have 4 types
        assert!(matches!(spec, Specialization::InlineCache { .. }));
    }
}
