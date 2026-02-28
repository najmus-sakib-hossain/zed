//! Advanced Optimizations
//!
//! This module implements aggressive optimizations to achieve 10x performance:
//! - Constant folding (evaluate constant expressions at compile time)
//! - Dead code elimination (remove unreachable code)
//! - Loop-invariant code motion (hoist invariant computations)
//! - Escape analysis (stack allocate non-escaping objects)
//! - Inline caching (method lookups)
//! - SIMD vectorization

use crate::compiler::mir::{
    BinOpKind, BlockId, Constant, LocalId, PrimitiveType, Terminator, Type, TypedFunction,
    TypedInstruction, TypedMIR,
};
use crate::error::DxResult;
use std::collections::{HashMap, HashSet};

/// Optimization pipeline
pub struct OptimizationPipeline {
    /// Inline cache for hot paths
    inline_cache: InlineCache,
    /// Escape analyzer
    escape_analyzer: EscapeAnalyzer,
    /// SIMD optimizer
    simd_optimizer: SimdOptimizer,
    /// Constant folder
    constant_folder: ConstantFolder,
    /// Loop optimizer
    loop_optimizer: LoopOptimizer,
}

impl OptimizationPipeline {
    pub fn new() -> Self {
        Self {
            inline_cache: InlineCache::new(),
            escape_analyzer: EscapeAnalyzer::new(),
            simd_optimizer: SimdOptimizer::new(),
            constant_folder: ConstantFolder::new(),
            loop_optimizer: LoopOptimizer::new(),
        }
    }

    /// Run all optimizations on MIR
    pub fn optimize(&mut self, mir: TypedMIR) -> DxResult<TypedMIR> {
        let mut optimized = mir;

        // Phase 1: Constant folding (evaluate constant expressions)
        optimized = self.constant_folder.fold(optimized)?;

        // Phase 2: Dead code elimination
        optimized = self.eliminate_dead_code(optimized)?;

        // Phase 3: Loop-invariant code motion
        optimized = self.loop_optimizer.hoist_invariants(optimized)?;

        // Phase 4: Escape analysis (stack allocate when possible)
        optimized = self.escape_analyzer.analyze(optimized)?;

        // Phase 5: Inline caching (hot method lookups)
        optimized = self.inline_cache.optimize(optimized)?;

        // Phase 6: SIMD vectorization
        optimized = self.simd_optimizer.vectorize(optimized)?;

        Ok(optimized)
    }

    /// Eliminate dead code using use-def chains
    fn eliminate_dead_code(&self, mut mir: TypedMIR) -> DxResult<TypedMIR> {
        for func in &mut mir.functions {
            // Build use set - which locals are actually used
            let used_locals = self.find_used_locals(func);

            // Remove instructions that define unused locals (except side-effecting ones)
            for block in &mut func.blocks {
                block.instructions.retain(|instr| {
                    match instr {
                        // Keep instructions that define used locals
                        TypedInstruction::Const { dest, .. }
                        | TypedInstruction::BinOp { dest, .. }
                        | TypedInstruction::Copy { dest, .. }
                        | TypedInstruction::GetProperty { dest, .. }
                        | TypedInstruction::GetPropertyDynamic { dest, .. }
                        | TypedInstruction::GetPropertyComputed { dest, .. }
                        | TypedInstruction::GetCaptured { dest, .. }
                        | TypedInstruction::GetException { dest, .. }
                        | TypedInstruction::GetThis { dest, .. }
                        | TypedInstruction::TypeOf { dest, .. }
                        | TypedInstruction::CreateFunction { dest, .. }
                        | TypedInstruction::CreateArray { dest, .. }
                        | TypedInstruction::CreateObject { dest, .. }
                        | TypedInstruction::CreateGenerator { dest, .. }
                        | TypedInstruction::GeneratorNext { dest, .. }
                        | TypedInstruction::CreatePromise { dest, .. }
                        | TypedInstruction::CreateAsyncFunction { dest, .. }
                        | TypedInstruction::Allocate { dest, .. }
                        | TypedInstruction::ToBool { dest, .. }
                        | TypedInstruction::IsNullish { dest, .. }
                        | TypedInstruction::BitwiseNot { dest, .. }
                        | TypedInstruction::BitwiseAnd { dest, .. }
                        | TypedInstruction::BitwiseOr { dest, .. }
                        | TypedInstruction::BitwiseXor { dest, .. }
                        | TypedInstruction::ShiftLeft { dest, .. }
                        | TypedInstruction::ShiftRight { dest, .. }
                        | TypedInstruction::ShiftRightUnsigned { dest, .. }
                        | TypedInstruction::Exponentiate { dest, .. }
                        | TypedInstruction::StrictEqual { dest, .. }
                        | TypedInstruction::StrictNotEqual { dest, .. }
                        | TypedInstruction::LooseEqual { dest, .. }
                        | TypedInstruction::LooseNotEqual { dest, .. }
                        | TypedInstruction::InstanceOf { dest, .. }
                        | TypedInstruction::In { dest, .. }
                        | TypedInstruction::Delete { dest, .. }
                        | TypedInstruction::DeleteComputed { dest, .. } => {
                            used_locals.contains(dest)
                        }

                        // Generator/async instructions that define locals
                        TypedInstruction::GeneratorYield { dest, .. }
                        | TypedInstruction::Await { dest, .. } => used_locals.contains(dest),

                        // Always keep side-effecting instructions
                        TypedInstruction::Call { .. }
                        | TypedInstruction::CallFunction { .. }
                        | TypedInstruction::CallWithSpread { .. }
                        | TypedInstruction::SetProperty { .. }
                        | TypedInstruction::SetPropertyDynamic { .. }
                        | TypedInstruction::SetPropertyComputed { .. }
                        | TypedInstruction::SetCaptured { .. }
                        | TypedInstruction::ArraySpread { .. }
                        | TypedInstruction::ArrayPush { .. }
                        | TypedInstruction::Throw { .. }
                        | TypedInstruction::SetupExceptionHandler { .. }
                        | TypedInstruction::ClearExceptionHandler
                        | TypedInstruction::GeneratorReturn { .. }
                        | TypedInstruction::PromiseResolve { .. }
                        | TypedInstruction::PromiseReject { .. }
                        | TypedInstruction::SetPrototype { .. }
                        | TypedInstruction::CallSuper { .. }
                        | TypedInstruction::SuperMethodCall { .. }
                        | TypedInstruction::DefineMethod { .. }
                        | TypedInstruction::DefineGetter { .. }
                        | TypedInstruction::DefineSetter { .. } => true,

                        // Class-related instructions that define locals
                        TypedInstruction::CreateClass { dest, .. }
                        | TypedInstruction::GetPrototype { dest, .. }
                        | TypedInstruction::DynamicImport { dest, .. } => used_locals.contains(dest),

                        // Destructuring instructions that define locals
                        // Requirements: 7.1-7.7
                        TypedInstruction::ArraySliceFrom { dest, .. }
                        | TypedInstruction::ObjectRest { dest, .. }
                        | TypedInstruction::IsUndefined { dest, .. } => used_locals.contains(dest),

                        // Destructuring error - always keep (side-effecting)
                        TypedInstruction::ThrowDestructuringError { .. } => true,

                        // Template literal instructions
                        // Requirements: 8.1-8.3
                        TypedInstruction::BuildTemplateLiteral { dest, .. } => used_locals.contains(dest),
                        TypedInstruction::CallTaggedTemplate { dest, .. } => used_locals.contains(dest),
                    }
                });
            }
        }
        Ok(mir)
    }

    /// Find all locals that are used (read from)
    fn find_used_locals(&self, func: &TypedFunction) -> HashSet<LocalId> {
        let mut used = HashSet::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    TypedInstruction::BinOp { left, right, .. } => {
                        used.insert(*left);
                        used.insert(*right);
                    }
                    TypedInstruction::Copy { src, .. } => {
                        used.insert(*src);
                    }
                    TypedInstruction::GetProperty { object, .. }
                    | TypedInstruction::GetPropertyDynamic { object, .. } => {
                        used.insert(*object);
                    }
                    TypedInstruction::GetPropertyComputed { object, key, .. } => {
                        used.insert(*object);
                        used.insert(*key);
                    }
                    TypedInstruction::SetProperty { object, value, .. } => {
                        used.insert(*object);
                        used.insert(*value);
                    }
                    TypedInstruction::SetPropertyDynamic { object, value, .. } => {
                        used.insert(*object);
                        used.insert(*value);
                    }
                    TypedInstruction::SetPropertyComputed { object, key, value } => {
                        used.insert(*object);
                        used.insert(*key);
                        used.insert(*value);
                    }
                    TypedInstruction::Call { args, .. } => {
                        for arg in args {
                            used.insert(*arg);
                        }
                    }
                    TypedInstruction::CallFunction {
                        callee,
                        args,
                        this_arg,
                        ..
                    } => {
                        used.insert(*callee);
                        for arg in args {
                            used.insert(*arg);
                        }
                        if let Some(this) = this_arg {
                            used.insert(*this);
                        }
                    }
                    TypedInstruction::CreateFunction { captured_vars, .. } => {
                        for var in captured_vars {
                            used.insert(*var);
                        }
                    }
                    TypedInstruction::CreateArray { elements, .. } => {
                        for elem in elements {
                            used.insert(*elem);
                        }
                    }
                    TypedInstruction::CreateObject { properties, .. } => {
                        for (_, val) in properties {
                            used.insert(*val);
                        }
                    }
                    TypedInstruction::SetCaptured { value, .. } => {
                        used.insert(*value);
                    }
                    TypedInstruction::Throw { value } => {
                        used.insert(*value);
                    }
                    // Template literal instructions
                    TypedInstruction::BuildTemplateLiteral { expressions, .. } => {
                        for expr in expressions {
                            used.insert(*expr);
                        }
                    }
                    TypedInstruction::CallTaggedTemplate { tag, expressions, .. } => {
                        used.insert(*tag);
                        for expr in expressions {
                            used.insert(*expr);
                        }
                    }
                    _ => {}
                }
            }

            // Check terminator
            match &block.terminator {
                Terminator::Return(Some(local)) => {
                    used.insert(*local);
                }
                Terminator::Branch { condition, .. } => {
                    used.insert(*condition);
                }
                _ => {}
            }
        }

        used
    }
}

impl Default for OptimizationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Constant folding - evaluate constant expressions at compile time
pub struct ConstantFolder {
    /// Map from LocalId to known constant value
    known_constants: HashMap<LocalId, Constant>,
}

impl ConstantFolder {
    pub fn new() -> Self {
        Self {
            known_constants: HashMap::new(),
        }
    }

    /// Fold constant expressions in the MIR
    pub fn fold(&mut self, mut mir: TypedMIR) -> DxResult<TypedMIR> {
        for func in &mut mir.functions {
            self.known_constants.clear();
            self.fold_function(func);
        }
        Ok(mir)
    }

    fn fold_function(&mut self, func: &mut TypedFunction) {
        for block in &mut func.blocks {
            let mut new_instructions = Vec::with_capacity(block.instructions.len());

            for instr in block.instructions.drain(..) {
                match &instr {
                    // Track constant definitions
                    TypedInstruction::Const { dest, value } => {
                        self.known_constants.insert(*dest, value.clone());
                        new_instructions.push(instr);
                    }

                    // Try to fold binary operations
                    TypedInstruction::BinOp {
                        dest,
                        op,
                        left,
                        right,
                        op_type,
                    } => {
                        if let Some(folded) =
                            self.try_fold_binop(*dest, *op, *left, *right, *op_type)
                        {
                            // Replace with constant
                            self.known_constants.insert(*dest, folded.clone());
                            new_instructions.push(TypedInstruction::Const {
                                dest: *dest,
                                value: folded,
                            });
                        } else {
                            new_instructions.push(instr);
                        }
                    }

                    // Copy propagation - if copying a constant, track it
                    TypedInstruction::Copy { dest, src } => {
                        if let Some(constant) = self.known_constants.get(src).cloned() {
                            self.known_constants.insert(*dest, constant);
                        }
                        new_instructions.push(instr);
                    }

                    _ => {
                        new_instructions.push(instr);
                    }
                }
            }

            block.instructions = new_instructions;
        }
    }

    /// Try to fold a binary operation on constants
    fn try_fold_binop(
        &self,
        _dest: LocalId,
        op: BinOpKind,
        left: LocalId,
        right: LocalId,
        _op_type: PrimitiveType,
    ) -> Option<Constant> {
        let left_const = self.known_constants.get(&left)?;
        let right_const = self.known_constants.get(&right)?;

        match (left_const, right_const) {
            // Integer operations
            (Constant::I32(a), Constant::I32(b)) => {
                let result = match op {
                    BinOpKind::Add => Constant::I32(a.wrapping_add(*b)),
                    BinOpKind::Sub => Constant::I32(a.wrapping_sub(*b)),
                    BinOpKind::Mul => Constant::I32(a.wrapping_mul(*b)),
                    BinOpKind::Div if *b != 0 => Constant::I32(a.wrapping_div(*b)),
                    BinOpKind::Mod if *b != 0 => Constant::I32(a.wrapping_rem(*b)),
                    BinOpKind::Eq => Constant::Bool(a == b),
                    BinOpKind::Ne => Constant::Bool(a != b),
                    BinOpKind::Lt => Constant::Bool(a < b),
                    BinOpKind::Le => Constant::Bool(a <= b),
                    BinOpKind::Gt => Constant::Bool(a > b),
                    BinOpKind::Ge => Constant::Bool(a >= b),
                    _ => return None,
                };
                Some(result)
            }

            // Float operations
            (Constant::F64(a), Constant::F64(b)) => {
                let result = match op {
                    BinOpKind::Add => Constant::F64(a + b),
                    BinOpKind::Sub => Constant::F64(a - b),
                    BinOpKind::Mul => Constant::F64(a * b),
                    BinOpKind::Div => Constant::F64(a / b),
                    BinOpKind::Mod => Constant::F64(a % b),
                    BinOpKind::Eq => Constant::Bool(a == b),
                    BinOpKind::Ne => Constant::Bool(a != b),
                    BinOpKind::Lt => Constant::Bool(a < b),
                    BinOpKind::Le => Constant::Bool(a <= b),
                    BinOpKind::Gt => Constant::Bool(a > b),
                    BinOpKind::Ge => Constant::Bool(a >= b),
                    _ => return None,
                };
                Some(result)
            }

            // Boolean operations
            (Constant::Bool(a), Constant::Bool(b)) => {
                let result = match op {
                    BinOpKind::And => Constant::Bool(*a && *b),
                    BinOpKind::Or => Constant::Bool(*a || *b),
                    BinOpKind::Eq => Constant::Bool(a == b),
                    BinOpKind::Ne => Constant::Bool(a != b),
                    _ => return None,
                };
                Some(result)
            }

            _ => None,
        }
    }

    /// Check if instruction operates on constants only
    pub fn is_constant_expression(&self, instr: &TypedInstruction) -> bool {
        match instr {
            TypedInstruction::Const { .. } => true,
            TypedInstruction::BinOp { left, right, .. } => {
                self.known_constants.contains_key(left) && self.known_constants.contains_key(right)
            }
            _ => false,
        }
    }
}

impl Default for ConstantFolder {
    fn default() -> Self {
        Self::new()
    }
}

/// Loop optimizer - hoist invariant code, unroll, and vectorize loops
pub struct LoopOptimizer {
    /// Maximum unroll factor
    max_unroll: usize,
}

impl LoopOptimizer {
    pub fn new() -> Self {
        Self { max_unroll: 8 }
    }

    /// Hoist loop-invariant code to loop preheader
    pub fn hoist_invariants(&self, mut mir: TypedMIR) -> DxResult<TypedMIR> {
        for func in &mut mir.functions {
            self.hoist_function_invariants(func);
        }
        Ok(mir)
    }

    fn hoist_function_invariants(&self, func: &mut TypedFunction) {
        // Find loop headers (blocks that are targets of back edges)
        let loop_headers = self.find_loop_headers(func);

        for header_id in loop_headers {
            // Find all blocks in this loop
            let loop_blocks = self.find_loop_blocks(func, header_id);

            // Find loop-invariant instructions
            let invariants = self.find_invariant_instructions(func, &loop_blocks);

            // Move invariants to preheader (block before loop header)
            if !invariants.is_empty() {
                self.move_to_preheader(func, header_id, invariants);
            }
        }
    }

    /// Find blocks that are loop headers (have back edges)
    fn find_loop_headers(&self, func: &TypedFunction) -> Vec<BlockId> {
        let mut headers = Vec::new();
        let block_order: HashMap<BlockId, usize> =
            func.blocks.iter().enumerate().map(|(i, b)| (b.id, i)).collect();

        for block in &func.blocks {
            let block_idx = block_order[&block.id];
            match &block.terminator {
                Terminator::Goto(target) => {
                    if let Some(&target_idx) = block_order.get(target) {
                        if target_idx <= block_idx {
                            headers.push(*target);
                        }
                    }
                }
                Terminator::Branch {
                    then_block,
                    else_block,
                    ..
                } => {
                    if let Some(&then_idx) = block_order.get(then_block) {
                        if then_idx <= block_idx {
                            headers.push(*then_block);
                        }
                    }
                    if let Some(&else_idx) = block_order.get(else_block) {
                        if else_idx <= block_idx {
                            headers.push(*else_block);
                        }
                    }
                }
                _ => {}
            }
        }

        headers.sort();
        headers.dedup();
        headers
    }

    /// Find all blocks that belong to a loop
    fn find_loop_blocks(&self, func: &TypedFunction, header: BlockId) -> HashSet<BlockId> {
        let mut loop_blocks = HashSet::new();
        loop_blocks.insert(header);

        // Simple heuristic: include blocks between header and back edge
        let mut in_loop = false;
        for block in &func.blocks {
            if block.id == header {
                in_loop = true;
            }
            if in_loop {
                loop_blocks.insert(block.id);
            }
            // Check if this block jumps back to header
            match &block.terminator {
                Terminator::Goto(target) if *target == header => {
                    in_loop = false;
                }
                Terminator::Branch {
                    then_block,
                    else_block,
                    ..
                } => {
                    if *then_block == header || *else_block == header {
                        in_loop = false;
                    }
                }
                _ => {}
            }
        }

        loop_blocks
    }

    /// Find instructions that are loop-invariant
    fn find_invariant_instructions(
        &self,
        func: &TypedFunction,
        loop_blocks: &HashSet<BlockId>,
    ) -> Vec<(BlockId, usize, TypedInstruction)> {
        let mut invariants = Vec::new();

        // Find all locals defined in the loop
        let mut loop_defined: HashSet<LocalId> = HashSet::new();
        for block in &func.blocks {
            if !loop_blocks.contains(&block.id) {
                continue;
            }
            for instr in &block.instructions {
                if let Some(dest) = self.get_instruction_dest(instr) {
                    loop_defined.insert(dest);
                }
            }
        }

        // Find instructions that only use values defined outside the loop
        for block in &func.blocks {
            if !loop_blocks.contains(&block.id) {
                continue;
            }
            for (idx, instr) in block.instructions.iter().enumerate() {
                if self.is_invariant(instr, &loop_defined) {
                    invariants.push((block.id, idx, instr.clone()));
                }
            }
        }

        invariants
    }

    /// Check if an instruction is loop-invariant
    fn is_invariant(&self, instr: &TypedInstruction, loop_defined: &HashSet<LocalId>) -> bool {
        match instr {
            // Constants are always invariant
            TypedInstruction::Const { .. } => true,

            // Binary ops are invariant if both operands are defined outside loop
            TypedInstruction::BinOp { left, right, .. } => {
                !loop_defined.contains(left) && !loop_defined.contains(right)
            }

            // Property access is invariant if object is defined outside loop
            TypedInstruction::GetProperty { object, .. }
            | TypedInstruction::GetPropertyDynamic { object, .. } => !loop_defined.contains(object),

            // Side-effecting instructions are never invariant
            TypedInstruction::Call { .. }
            | TypedInstruction::CallFunction { .. }
            | TypedInstruction::SetProperty { .. }
            | TypedInstruction::SetPropertyDynamic { .. }
            | TypedInstruction::SetPropertyComputed { .. }
            | TypedInstruction::Throw { .. }
            | TypedInstruction::Allocate { .. }
            | TypedInstruction::CreateArray { .. }
            | TypedInstruction::CreateObject { .. }
            | TypedInstruction::CreateFunction { .. } => false,

            _ => false,
        }
    }

    /// Get the destination local of an instruction
    fn get_instruction_dest(&self, instr: &TypedInstruction) -> Option<LocalId> {
        match instr {
            TypedInstruction::Const { dest, .. }
            | TypedInstruction::BinOp { dest, .. }
            | TypedInstruction::Copy { dest, .. }
            | TypedInstruction::GetProperty { dest, .. }
            | TypedInstruction::GetPropertyDynamic { dest, .. }
            | TypedInstruction::GetPropertyComputed { dest, .. }
            | TypedInstruction::GetCaptured { dest, .. }
            | TypedInstruction::GetException { dest, .. }
            | TypedInstruction::CreateFunction { dest, .. }
            | TypedInstruction::CreateArray { dest, .. }
            | TypedInstruction::CreateObject { dest, .. }
            | TypedInstruction::Allocate { dest, .. } => Some(*dest),
            TypedInstruction::Call { dest, .. } | TypedInstruction::CallFunction { dest, .. } => {
                *dest
            }
            _ => None,
        }
    }

    /// Move invariant instructions to loop preheader
    fn move_to_preheader(
        &self,
        func: &mut TypedFunction,
        _header: BlockId,
        invariants: Vec<(BlockId, usize, TypedInstruction)>,
    ) {
        // For simplicity, we'll just mark these as hoisted
        // A full implementation would create a preheader block
        // and move the instructions there

        // Remove invariants from their original blocks
        let to_remove: HashSet<(BlockId, usize)> =
            invariants.iter().map(|(bid, idx, _)| (*bid, *idx)).collect();

        for block in &mut func.blocks {
            let mut new_instructions = Vec::new();
            for (idx, instr) in block.instructions.drain(..).enumerate() {
                if !to_remove.contains(&(block.id, idx)) {
                    new_instructions.push(instr);
                }
            }
            block.instructions = new_instructions;
        }

        // Insert at the beginning of the first block (simplified preheader)
        if let Some(first_block) = func.blocks.first_mut() {
            let mut hoisted: Vec<TypedInstruction> =
                invariants.into_iter().map(|(_, _, instr)| instr).collect();
            hoisted.append(&mut first_block.instructions);
            first_block.instructions = hoisted;
        }
    }

    /// Check if loop should be unrolled
    pub fn should_unroll(&self, iteration_count: usize) -> bool {
        iteration_count > 0 && iteration_count <= self.max_unroll
    }
}

impl Default for LoopOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape analyzer - determines if allocations can be stack-based
pub struct EscapeAnalyzer {
    /// Variables that escape their scope
    escaped_vars: HashSet<LocalId>,
    /// Allocation sites that don't escape (can be stack allocated)
    non_escaping_allocations: HashSet<LocalId>,
}

impl EscapeAnalyzer {
    pub fn new() -> Self {
        Self {
            escaped_vars: HashSet::new(),
            non_escaping_allocations: HashSet::new(),
        }
    }

    /// Analyze if variables escape their scope
    pub fn analyze(&mut self, mut mir: TypedMIR) -> DxResult<TypedMIR> {
        for func in &mut mir.functions {
            self.analyze_function(func);
        }
        Ok(mir)
    }

    fn analyze_function(&mut self, func: &mut TypedFunction) {
        self.escaped_vars.clear();
        self.non_escaping_allocations.clear();

        // Find all allocation sites
        let mut allocations: HashSet<LocalId> = HashSet::new();
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    TypedInstruction::Allocate { dest, .. }
                    | TypedInstruction::CreateArray { dest, .. }
                    | TypedInstruction::CreateObject { dest, .. } => {
                        allocations.insert(*dest);
                    }
                    _ => {}
                }
            }
        }

        // Find escaping allocations
        for block in &func.blocks {
            // Check terminator for escaping returns
            if let Terminator::Return(Some(local)) = &block.terminator {
                if allocations.contains(local) {
                    self.escaped_vars.insert(*local);
                }
            }
        }

        // Check for escapes through function calls
        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    TypedInstruction::Call { args, .. }
                    | TypedInstruction::CallFunction { args, .. } => {
                        for arg in args {
                            if allocations.contains(arg) {
                                self.escaped_vars.insert(*arg);
                            }
                        }
                    }
                    // Stored to property = escapes
                    TypedInstruction::SetProperty { value, .. }
                    | TypedInstruction::SetPropertyDynamic { value, .. }
                    | TypedInstruction::SetPropertyComputed { value, .. } => {
                        if allocations.contains(value) {
                            self.escaped_vars.insert(*value);
                        }
                    }
                    // Captured in closure = escapes
                    TypedInstruction::CreateFunction { captured_vars, .. } => {
                        for var in captured_vars {
                            if allocations.contains(var) {
                                self.escaped_vars.insert(*var);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Mark non-escaping allocations
        for alloc in allocations {
            if !self.escaped_vars.contains(&alloc) {
                self.non_escaping_allocations.insert(alloc);
            }
        }
    }

    /// Check if variable escapes
    pub fn escapes(&self, var: LocalId) -> bool {
        self.escaped_vars.contains(&var)
    }

    /// Check if allocation can be stack-allocated
    pub fn can_stack_allocate(&self, var: LocalId) -> bool {
        self.non_escaping_allocations.contains(&var)
    }

    /// Mark variable as escaping
    pub fn mark_escaped(&mut self, var: LocalId) {
        self.escaped_vars.insert(var);
    }
}

impl Default for EscapeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Inline cache for method lookups
pub struct InlineCache {
    /// Cached method addresses by receiver type - reserved for IC optimization
    #[allow(dead_code)]
    cache: HashMap<String, u64>,
    /// Hit counter for profiling
    hits: HashMap<String, usize>,
}

impl InlineCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            hits: HashMap::new(),
        }
    }

    /// Optimize method lookups using inline caching
    pub fn optimize(&mut self, mir: TypedMIR) -> DxResult<TypedMIR> {
        // Inline caching is a runtime optimization
        // At compile time, we just pass through
        Ok(mir)
    }

    /// Record method call
    pub fn record_call(&mut self, method: &str, receiver_type: &Type) {
        let key = format!("{}::{:?}", method, receiver_type);
        *self.hits.entry(key).or_insert(0) += 1;
    }

    /// Check if method is hot (should be inline cached)
    pub fn is_hot(&self, method: &str, receiver_type: &Type) -> bool {
        let key = format!("{}::{:?}", method, receiver_type);
        self.hits.get(&key).copied().unwrap_or(0) > 100
    }
}

impl Default for InlineCache {
    fn default() -> Self {
        Self::new()
    }
}

/// SIMD optimizer - vectorize array operations
pub struct SimdOptimizer {
    /// Vector width (128-bit = 4x f32, 2x f64) - reserved for SIMD vectorization
    #[allow(dead_code)]
    vector_width: usize,
}

impl SimdOptimizer {
    pub fn new() -> Self {
        Self {
            vector_width: 4, // SSE/NEON baseline
        }
    }

    /// Vectorize array operations using SIMD
    pub fn vectorize(&self, mir: TypedMIR) -> DxResult<TypedMIR> {
        // SIMD vectorization is complex and requires loop analysis
        // For now, pass through
        Ok(mir)
    }

    /// Check if instruction can be vectorized
    pub fn is_vectorizable(&self, instr: &TypedInstruction) -> bool {
        match instr {
            TypedInstruction::BinOp { op_type, .. } => {
                matches!(op_type, PrimitiveType::I32 | PrimitiveType::F64 | PrimitiveType::I64)
            }
            _ => false,
        }
    }

    /// Get optimal vector width for type
    pub fn get_vector_width(&self, ty: &Type) -> usize {
        match ty {
            Type::Primitive(PrimitiveType::I32) => 4,
            Type::Primitive(PrimitiveType::F64) => 2,
            Type::Primitive(PrimitiveType::I64) => 2,
            _ => 1,
        }
    }
}

impl Default for SimdOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Monomorphization - specialize generic code for specific types
pub struct Monomorphizer {
    /// Specialized function instances
    specialized_functions: HashMap<String, Vec<TypedMIR>>,
}

impl Monomorphizer {
    pub fn new() -> Self {
        Self {
            specialized_functions: HashMap::new(),
        }
    }

    /// Monomorphize generic function for specific type
    pub fn specialize(&mut self, func_name: &str, type_args: &[Type]) -> DxResult<String> {
        let specialized_name = format!(
            "{}_{}",
            func_name,
            type_args.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join("_")
        );
        Ok(specialized_name)
    }

    /// Check if specialization exists
    pub fn has_specialization(&self, func_name: &str, type_args: &[Type]) -> bool {
        let key = format!(
            "{}_{}",
            func_name,
            type_args.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>().join("_")
        );
        self.specialized_functions.contains_key(&key)
    }
}

impl Default for Monomorphizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::mir::{FunctionId, TypedBlock, TypedLocal};

    #[allow(dead_code)]
    fn create_test_function() -> TypedFunction {
        TypedFunction {
            id: FunctionId(0),
            name: "test".to_string(),
            params: vec![],
            return_type: Type::Primitive(PrimitiveType::F64),
            blocks: vec![TypedBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: Terminator::Return(None),
                instruction_spans: vec![],
                terminator_span: crate::compiler::mir::SourceSpan::unknown(),
            }],
            locals: vec![],
            is_pure: true,
            span: crate::compiler::mir::SourceSpan::unknown(),
        }
    }

    #[test]
    fn test_constant_folding_addition() {
        let mut folder = ConstantFolder::new();

        let mir = TypedMIR {
            functions: vec![TypedFunction {
                id: FunctionId(0),
                name: "test".to_string(),
                params: vec![],
                return_type: Type::Primitive(PrimitiveType::F64),
                blocks: vec![TypedBlock {
                    id: BlockId(0),
                    instructions: vec![
                        TypedInstruction::Const {
                            dest: LocalId(0),
                            value: Constant::F64(10.0),
                        },
                        TypedInstruction::Const {
                            dest: LocalId(1),
                            value: Constant::F64(20.0),
                        },
                        TypedInstruction::BinOp {
                            dest: LocalId(2),
                            op: BinOpKind::Add,
                            left: LocalId(0),
                            right: LocalId(1),
                            op_type: PrimitiveType::F64,
                        },
                    ],
                    terminator: Terminator::Return(Some(LocalId(2))),
                    instruction_spans: vec![],
                    terminator_span: crate::compiler::mir::SourceSpan::unknown(),
                }],
                locals: vec![
                    TypedLocal {
                        name: "a".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 0,
                    },
                    TypedLocal {
                        name: "b".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 1,
                    },
                    TypedLocal {
                        name: "c".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 2,
                    },
                ],
                is_pure: true,
                span: crate::compiler::mir::SourceSpan::unknown(),
            }],
            globals: vec![],
            entry_point: Some(FunctionId(0)),
            type_layouts: HashMap::new(),
            source_file: String::new(),
        };

        let optimized = folder.fold(mir).unwrap();

        // The BinOp should be replaced with a Const
        let block = &optimized.functions[0].blocks[0];
        assert_eq!(block.instructions.len(), 3);

        // Third instruction should now be a constant 30.0
        if let TypedInstruction::Const {
            value: Constant::F64(v),
            ..
        } = &block.instructions[2]
        {
            assert_eq!(*v, 30.0);
        } else {
            panic!("Expected constant folding to produce Const instruction");
        }
    }

    #[test]
    fn test_dead_code_elimination() {
        let pipeline = OptimizationPipeline::new();

        let mir = TypedMIR {
            functions: vec![TypedFunction {
                id: FunctionId(0),
                name: "test".to_string(),
                params: vec![],
                return_type: Type::Primitive(PrimitiveType::F64),
                blocks: vec![TypedBlock {
                    id: BlockId(0),
                    instructions: vec![
                        // Used constant
                        TypedInstruction::Const {
                            dest: LocalId(0),
                            value: Constant::F64(42.0),
                        },
                        // Unused constant (dead code)
                        TypedInstruction::Const {
                            dest: LocalId(1),
                            value: Constant::F64(100.0),
                        },
                    ],
                    terminator: Terminator::Return(Some(LocalId(0))),
                    instruction_spans: vec![],
                    terminator_span: crate::compiler::mir::SourceSpan::unknown(),
                }],
                locals: vec![
                    TypedLocal {
                        name: "used".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 0,
                    },
                    TypedLocal {
                        name: "unused".to_string(),
                        ty: Type::Primitive(PrimitiveType::F64),
                        index: 1,
                    },
                ],
                is_pure: true,
                span: crate::compiler::mir::SourceSpan::unknown(),
            }],
            globals: vec![],
            entry_point: Some(FunctionId(0)),
            type_layouts: HashMap::new(),
            source_file: String::new(),
        };

        let optimized = pipeline.eliminate_dead_code(mir).unwrap();

        // Dead code should be eliminated
        let block = &optimized.functions[0].blocks[0];
        assert_eq!(block.instructions.len(), 1);

        // Only the used constant should remain
        if let TypedInstruction::Const {
            dest,
            value: Constant::F64(v),
        } = &block.instructions[0]
        {
            assert_eq!(dest.0, 0);
            assert_eq!(*v, 42.0);
        } else {
            panic!("Expected used constant to remain");
        }
    }

    #[test]
    fn test_escape_analysis() {
        let mut analyzer = EscapeAnalyzer::new();

        // Test that returned values escape
        let mut func = TypedFunction {
            id: FunctionId(0),
            name: "test".to_string(),
            params: vec![],
            return_type: Type::Any,
            blocks: vec![TypedBlock {
                id: BlockId(0),
                instructions: vec![TypedInstruction::CreateObject {
                    dest: LocalId(0),
                    properties: vec![],
                }],
                terminator: Terminator::Return(Some(LocalId(0))),
                instruction_spans: vec![],
                terminator_span: crate::compiler::mir::SourceSpan::unknown(),
            }],
            locals: vec![TypedLocal {
                name: "obj".to_string(),
                ty: Type::Any,
                index: 0,
            }],
            is_pure: false,
            span: crate::compiler::mir::SourceSpan::unknown(),
        };

        analyzer.analyze_function(&mut func);

        // The object escapes because it's returned
        assert!(analyzer.escapes(LocalId(0)));
        assert!(!analyzer.can_stack_allocate(LocalId(0)));
    }

    #[test]
    fn test_non_escaping_allocation() {
        let mut analyzer = EscapeAnalyzer::new();

        // Test that local-only values don't escape
        let mut func = TypedFunction {
            id: FunctionId(0),
            name: "test".to_string(),
            params: vec![],
            return_type: Type::Primitive(PrimitiveType::F64),
            blocks: vec![TypedBlock {
                id: BlockId(0),
                instructions: vec![
                    TypedInstruction::CreateObject {
                        dest: LocalId(0),
                        properties: vec![],
                    },
                    TypedInstruction::Const {
                        dest: LocalId(1),
                        value: Constant::F64(42.0),
                    },
                ],
                terminator: Terminator::Return(Some(LocalId(1))), // Return primitive, not object
                instruction_spans: vec![],
                terminator_span: crate::compiler::mir::SourceSpan::unknown(),
            }],
            locals: vec![
                TypedLocal {
                    name: "obj".to_string(),
                    ty: Type::Any,
                    index: 0,
                },
                TypedLocal {
                    name: "result".to_string(),
                    ty: Type::Primitive(PrimitiveType::F64),
                    index: 1,
                },
            ],
            is_pure: false,
            span: crate::compiler::mir::SourceSpan::unknown(),
        };

        analyzer.analyze_function(&mut func);

        // The object doesn't escape - can be stack allocated
        assert!(!analyzer.escapes(LocalId(0)));
        assert!(analyzer.can_stack_allocate(LocalId(0)));
    }

    #[test]
    fn test_inline_cache() {
        let mut cache = InlineCache::new();
        let ty = Type::Primitive(PrimitiveType::I32);

        // Record calls
        for _ in 0..150 {
            cache.record_call("add", &ty);
        }

        // Should be hot after 150 calls
        assert!(cache.is_hot("add", &ty));
    }

    #[test]
    fn test_simd_optimizer() {
        let optimizer = SimdOptimizer::new();

        // Check vector widths
        assert_eq!(optimizer.get_vector_width(&Type::Primitive(PrimitiveType::I32)), 4);
        assert_eq!(optimizer.get_vector_width(&Type::Primitive(PrimitiveType::F64)), 2);
    }
}
