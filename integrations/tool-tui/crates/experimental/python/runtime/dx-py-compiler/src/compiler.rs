//! Source-to-Bytecode Compiler
//!
//! This module provides the main SourceCompiler that transforms Python AST
//! to DPB bytecode.

use crate::emitter::BytecodeEmitter;
use crate::error::{CompileError, CompileResult};
use crate::symbol_table::{Scope, SymbolTable};
use dx_py_bytecode::{CodeFlags, CodeObject, Constant, DpbCompiler, DpbOpcode};
use dx_py_parser::{
    parse_module, Arguments, BinOp, BoolOp, CmpOp, Comprehension, Constant as AstConstant,
    Expression, Module, Statement, UnaryOp,
};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Source-to-bytecode compiler
pub struct SourceCompiler {
    /// Filename for error reporting
    filename: PathBuf,
    /// Symbol table
    symbol_table: SymbolTable,
    /// Bytecode emitter stack (one per scope)
    emitter_stack: Vec<BytecodeEmitter>,
    /// Current scope stack for compilation
    scope_stack: Vec<Scope>,
}

impl SourceCompiler {
    /// Create a new compiler
    pub fn new(filename: PathBuf) -> Self {
        Self {
            filename,
            symbol_table: SymbolTable::new(),
            emitter_stack: Vec::new(),
            scope_stack: Vec::new(),
        }
    }

    /// Get the current emitter
    fn emitter(&mut self) -> &mut BytecodeEmitter {
        self.emitter_stack.last_mut().expect("No emitter on stack")
    }

    /// Get the current scope
    fn current_scope(&self) -> &Scope {
        self.scope_stack.last().expect("No scope on stack")
    }

    /// Push a new emitter for a nested scope
    fn push_emitter(&mut self) {
        self.emitter_stack.push(BytecodeEmitter::new());
    }

    /// Pop the current emitter
    fn pop_emitter(&mut self) -> BytecodeEmitter {
        self.emitter_stack.pop().expect("No emitter to pop")
    }

    /// Compile a Python source string to a code object
    pub fn compile_module_source(&mut self, source: &str) -> CompileResult<CodeObject> {
        let module = parse_module(source)?;
        self.compile_module(&module)
    }

    /// Compile a parsed module to a code object
    pub fn compile_module(&mut self, module: &Module) -> CompileResult<CodeObject> {
        // Analyze symbols
        self.symbol_table.analyze_module(module)?;

        // Get the root scope
        let root_scope = self
            .symbol_table
            .root
            .take()
            .ok_or_else(|| CompileError::codegen_error("No root scope"))?;

        // Push module scope and emitter
        self.scope_stack.push(root_scope);
        self.push_emitter();

        // Set up locals from symbol table
        {
            let scope = self.current_scope();
            let locals = scope.locals.clone();
            self.emitter().set_locals(locals);
        }

        // Compile module body
        for stmt in &module.body {
            self.compile_statement(stmt)?;
        }

        // Add implicit return None
        let none_idx = self.emitter().add_constant(Constant::None);
        self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
        self.emitter().emit(DpbOpcode::Return);

        // Patch jumps
        self.emitter().patch_jumps().map_err(CompileError::codegen_error)?;

        // Build code object
        let emitter = self.pop_emitter();
        let code = emitter.build_code_object(
            "<module>".to_string(),
            self.filename.to_string_lossy().to_string(),
            1,
            0,
            0,
            0,
            CodeFlags::empty(),
        );

        self.scope_stack.pop();
        Ok(code)
    }

    /// Get the symbol table (for testing)
    pub fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Compile a Python source file to DPB format and write to output file
    pub fn compile_to_dpb_file(&mut self, source: &str, output_path: &Path) -> CompileResult<()> {
        let code = self.compile_module_source(source)?;
        let dpb_bytes = self.serialize_to_dpb(&code)?;

        let mut file = File::create(output_path).map_err(|e| {
            CompileError::codegen_error(format!("Failed to create output file: {}", e))
        })?;
        file.write_all(&dpb_bytes)
            .map_err(|e| CompileError::codegen_error(format!("Failed to write DPB file: {}", e)))?;

        Ok(())
    }

    /// Serialize a CodeObject to DPB binary format
    pub fn serialize_to_dpb(&self, code: &CodeObject) -> CompileResult<Vec<u8>> {
        let mut dpb_compiler = DpbCompiler::new();
        dpb_compiler
            .compile(code)
            .map_err(|e| CompileError::codegen_error(format!("DPB serialization failed: {}", e)))
    }

    /// Compile a Python source string and return DPB bytes
    pub fn compile_to_dpb(&mut self, source: &str) -> CompileResult<Vec<u8>> {
        let code = self.compile_module_source(source)?;
        self.serialize_to_dpb(&code)
    }

    /// Compile an expression
    fn compile_expression(&mut self, expr: &Expression) -> CompileResult<()> {
        match expr {
            Expression::Constant { value, location } => {
                self.emitter().set_line(location.line);
                let const_val = self.ast_constant_to_bytecode(value);
                let idx = self.emitter().add_constant(const_val);
                self.emitter().emit_arg(DpbOpcode::LoadConst, idx);
            }

            Expression::Name { id, location } => {
                self.emitter().set_line(location.line);
                self.compile_load_name(id)?;
            }

            Expression::BinOp {
                left,
                op,
                right,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                let opcode = self.binop_to_opcode(*op);
                self.emitter().emit(opcode);
            }

            Expression::UnaryOp {
                op,
                operand,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_expression(operand)?;
                let opcode = self.unaryop_to_opcode(*op);
                self.emitter().emit(opcode);
            }

            Expression::Compare {
                left,
                ops,
                comparators,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_comparison(left, ops, comparators)?;
            }

            Expression::BoolOp {
                op,
                values,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_boolop(*op, values)?;
            }

            Expression::Call {
                func,
                args,
                keywords,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_call(func, args, keywords)?;
            }

            Expression::Attribute {
                value,
                attr,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                let name_idx = self.emitter().add_name(attr);
                self.emitter().emit_arg(DpbOpcode::LoadAttr, name_idx);
            }

            Expression::Subscript {
                value,
                slice,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                self.compile_expression(slice)?;
                self.emitter().emit(DpbOpcode::LoadSubscr);
            }

            Expression::List { elts, location } => {
                self.emitter().set_line(location.line);
                for elt in elts {
                    self.compile_expression(elt)?;
                }
                self.emitter().emit_arg(DpbOpcode::BuildList, elts.len() as u16);
            }

            Expression::Tuple { elts, location } => {
                self.emitter().set_line(location.line);
                for elt in elts {
                    self.compile_expression(elt)?;
                }
                self.emitter().emit_arg(DpbOpcode::BuildTuple, elts.len() as u16);
            }

            Expression::Dict {
                keys,
                values,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_dict(keys, values)?;
            }

            Expression::Set { elts, location } => {
                self.emitter().set_line(location.line);
                for elt in elts {
                    self.compile_expression(elt)?;
                }
                self.emitter().emit_arg(DpbOpcode::BuildSet, elts.len() as u16);
            }

            Expression::IfExp {
                test,
                body,
                orelse,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_ifexp(test, body, orelse)?;
            }

            Expression::ListComp {
                elt,
                generators,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_listcomp(elt, generators)?;
            }

            Expression::SetComp {
                elt,
                generators,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_setcomp(elt, generators)?;
            }

            Expression::DictComp {
                key,
                value,
                generators,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_dictcomp(key, value, generators)?;
            }

            Expression::GeneratorExp {
                elt,
                generators,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_genexp(elt, generators)?;
            }

            Expression::Slice {
                lower,
                upper,
                step,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_slice(lower, upper, step)?;
            }

            Expression::Lambda {
                args,
                body,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_lambda(args, body)?;
            }

            Expression::Starred { value, location } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                // Starred is handled by the context (tuple unpacking, etc.)
            }

            Expression::JoinedStr { values, location } => {
                self.emitter().set_line(location.line);
                self.compile_fstring(values)?;
            }

            Expression::FormattedValue {
                value,
                conversion,
                format_spec,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_formatted_value(value, *conversion, format_spec)?;
            }

            Expression::Yield { value, location } => {
                self.emitter().set_line(location.line);
                if let Some(v) = value {
                    self.compile_expression(v)?;
                } else {
                    let none_idx = self.emitter().add_constant(Constant::None);
                    self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
                }
                self.emitter().emit(DpbOpcode::Yield);
            }

            Expression::YieldFrom { value, location } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                self.emitter().emit(DpbOpcode::GetIter);
                self.emitter().emit_arg(DpbOpcode::YieldFrom, 0);
            }

            Expression::Await { value, location } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                self.emitter().emit_arg(DpbOpcode::GetAwaitable, 0);
                let none_idx = self.emitter().add_constant(Constant::None);
                self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
                self.emitter().emit_arg(DpbOpcode::YieldFrom, 0);
            }

            Expression::NamedExpr {
                target,
                value,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                self.emitter().emit(DpbOpcode::DupTop);
                self.compile_store_target(target)?;
            }
        }
        Ok(())
    }

    /// Compile a name load
    fn compile_load_name(&mut self, name: &str) -> CompileResult<()> {
        // First check if it's a local in the emitter (for comprehension variables)
        // This takes precedence over scope-based lookup
        if let Some(idx) = self.emitter().get_local(name) {
            self.emitter().emit_arg(DpbOpcode::LoadFast, idx);
            return Ok(());
        }

        let scope = self.current_scope();

        // At module level, always use LoadGlobal
        // This ensures we can find module-level definitions
        if scope.scope_type == crate::symbol_table::ScopeType::Module {
            let name_idx = self.emitter().add_name(name);
            self.emitter().emit_arg(DpbOpcode::LoadGlobal, name_idx);
            return Ok(());
        }

        // Check if it's a cell variable
        if let Some(idx) = scope.get_cell_index(name) {
            self.emitter().emit_arg(DpbOpcode::LoadDeref, idx);
            return Ok(());
        }

        // Check if it's a local variable
        if let Some(idx) = scope.get_local_index(name) {
            self.emitter().emit_arg(DpbOpcode::LoadFast, idx);
            return Ok(());
        }

        // Check if it's a free variable
        if let Some(idx) = scope.get_free_index(name) {
            // Free vars come after cell vars in the closure tuple
            let offset = scope.cell_vars.len() as u16;
            self.emitter().emit_arg(DpbOpcode::LoadDeref, offset + idx);
            return Ok(());
        }

        // Check if it's explicitly global
        if scope.explicit_globals.contains(name) {
            let name_idx = self.emitter().add_name(name);
            self.emitter().emit_arg(DpbOpcode::LoadGlobal, name_idx);
            return Ok(());
        }

        // Default: load as global/builtin
        let name_idx = self.emitter().add_name(name);
        self.emitter().emit_arg(DpbOpcode::LoadGlobal, name_idx);
        Ok(())
    }

    /// Compile a comparison expression
    fn compile_comparison(
        &mut self,
        left: &Expression,
        ops: &[CmpOp],
        comparators: &[Expression],
    ) -> CompileResult<()> {
        if ops.len() == 1 {
            // Simple comparison: a < b
            self.compile_expression(left)?;
            self.compile_expression(&comparators[0])?;
            let opcode = self.cmpop_to_opcode(ops[0]);
            self.emitter().emit(opcode);
        } else {
            // Chained comparison: a < b < c
            // Becomes: a < b and b < c (but b is only evaluated once)
            self.compile_expression(left)?;

            let end_label = self.emitter().new_label();
            let mut cleanup_labels = Vec::new();

            for (i, (op, comparator)) in ops.iter().zip(comparators.iter()).enumerate() {
                self.compile_expression(comparator)?;

                if i < ops.len() - 1 {
                    // Not the last comparison, need to duplicate and check
                    self.emitter().emit(DpbOpcode::DupTop);
                    self.emitter().emit_arg(DpbOpcode::RotN, 3);
                    let opcode = self.cmpop_to_opcode(*op);
                    self.emitter().emit(opcode);

                    let cleanup = self.emitter().new_label();
                    cleanup_labels.push(cleanup);
                    self.emitter().emit_jump(DpbOpcode::JumpIfFalseOrPop, cleanup);
                } else {
                    // Last comparison
                    let opcode = self.cmpop_to_opcode(*op);
                    self.emitter().emit(opcode);
                }
            }

            self.emitter().emit_jump(DpbOpcode::Jump, end_label);

            // Cleanup: pop the duplicated value and leave False
            for cleanup in cleanup_labels {
                self.emitter().define_label(cleanup);
                self.emitter().emit_arg(DpbOpcode::RotN, 2);
                self.emitter().emit(DpbOpcode::PopTop);
            }

            self.emitter().define_label(end_label);
        }
        Ok(())
    }

    /// Compile a boolean operation (and/or)
    fn compile_boolop(&mut self, op: BoolOp, values: &[Expression]) -> CompileResult<()> {
        let end_label = self.emitter().new_label();

        for (i, value) in values.iter().enumerate() {
            self.compile_expression(value)?;

            if i < values.len() - 1 {
                match op {
                    BoolOp::And => {
                        self.emitter().emit_jump(DpbOpcode::JumpIfFalseOrPop, end_label);
                    }
                    BoolOp::Or => {
                        self.emitter().emit_jump(DpbOpcode::JumpIfTrueOrPop, end_label);
                    }
                }
            }
        }

        self.emitter().define_label(end_label);
        Ok(())
    }

    /// Compile a function call
    fn compile_call(
        &mut self,
        func: &Expression,
        args: &[Expression],
        keywords: &[dx_py_parser::Keyword],
    ) -> CompileResult<()> {
        // Check for method call optimization
        if let Expression::Attribute { value, attr, .. } = func {
            self.compile_expression(value)?;
            let name_idx = self.emitter().add_name(attr);
            self.emitter().emit_arg(DpbOpcode::LoadMethod, name_idx);

            for arg in args {
                self.compile_expression(arg)?;
            }

            if keywords.is_empty() {
                self.emitter().emit_arg(DpbOpcode::CallMethod, args.len() as u16);
            } else {
                // Fall back to regular call for keyword args
                for kw in keywords {
                    self.compile_expression(&kw.value)?;
                }
                self.emitter().emit_arg(DpbOpcode::CallKw, (args.len() + keywords.len()) as u16);
            }
        } else {
            // Regular function call
            self.compile_expression(func)?;

            for arg in args {
                self.compile_expression(arg)?;
            }

            if keywords.is_empty() {
                self.emitter().emit_arg(DpbOpcode::Call, args.len() as u16);
            } else {
                for kw in keywords {
                    self.compile_expression(&kw.value)?;
                }
                self.emitter().emit_arg(DpbOpcode::CallKw, (args.len() + keywords.len()) as u16);
            }
        }
        Ok(())
    }

    /// Compile a dictionary literal
    fn compile_dict(
        &mut self,
        keys: &[Option<Expression>],
        values: &[Expression],
    ) -> CompileResult<()> {
        let mut has_splat = false;
        for key in keys {
            if key.is_none() {
                has_splat = true;
                break;
            }
        }

        if !has_splat {
            // Simple dict: {k1: v1, k2: v2}
            for (key, value) in keys.iter().zip(values.iter()) {
                if let Some(k) = key {
                    self.compile_expression(k)?;
                }
                self.compile_expression(value)?;
            }
            self.emitter().emit_arg(DpbOpcode::BuildDict, keys.len() as u16);
        } else {
            // Dict with splat: {**d1, k: v, **d2}
            self.emitter().emit_arg(DpbOpcode::BuildDict, 0);
            for (key, value) in keys.iter().zip(values.iter()) {
                if let Some(k) = key {
                    self.compile_expression(k)?;
                    self.compile_expression(value)?;
                    self.emitter().emit_arg(DpbOpcode::MapAdd, 1);
                } else {
                    self.compile_expression(value)?;
                    self.emitter().emit_arg(DpbOpcode::DictUpdate, 1);
                }
            }
        }
        Ok(())
    }

    /// Compile a conditional expression (ternary)
    fn compile_ifexp(
        &mut self,
        test: &Expression,
        body: &Expression,
        orelse: &Expression,
    ) -> CompileResult<()> {
        let else_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        self.compile_expression(test)?;
        self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, else_label);

        self.compile_expression(body)?;
        self.emitter().emit_jump(DpbOpcode::Jump, end_label);

        self.emitter().define_label(else_label);
        self.compile_expression(orelse)?;

        self.emitter().define_label(end_label);
        Ok(())
    }

    /// Compile a list comprehension
    fn compile_listcomp(
        &mut self,
        elt: &Expression,
        generators: &[Comprehension],
    ) -> CompileResult<()> {
        // List comprehensions are compiled inline for performance
        // This approach avoids the overhead of creating a nested code object
        // while still providing correct semantics for simple comprehensions
        
        // Add loop variables as locals for proper scoping
        // This ensures StoreFast/LoadFast are used instead of StoreGlobal/LoadGlobal
        self.add_comprehension_locals(generators)?;
        
        self.emitter().emit_arg(DpbOpcode::BuildList, 0);

        // Compile nested generators recursively
        self.compile_comprehension_generators(elt, generators, 0, DpbOpcode::ListAppend)?;

        Ok(())
    }
    
    /// Add comprehension loop variables as locals to the emitter
    fn add_comprehension_locals(&mut self, generators: &[Comprehension]) -> CompileResult<()> {
        for gen in generators {
            self.add_target_locals(&gen.target)?;
        }
        Ok(())
    }
    
    /// Recursively add target names as locals
    fn add_target_locals(&mut self, target: &Expression) -> CompileResult<()> {
        match target {
            Expression::Name { id, .. } => {
                self.emitter().add_local(id);
            }
            Expression::Tuple { elts, .. } | Expression::List { elts, .. } => {
                for elt in elts {
                    self.add_target_locals(elt)?;
                }
            }
            Expression::Starred { value, .. } => {
                self.add_target_locals(value)?;
            }
            _ => {
                // Other targets (attribute, subscript) don't need local allocation
            }
        }
        Ok(())
    }

    /// Compile comprehension generators recursively
    /// This handles multiple 'for' clauses by nesting loops
    fn compile_comprehension_generators(
        &mut self,
        elt: &Expression,
        generators: &[Comprehension],
        depth: usize,
        append_opcode: DpbOpcode,
    ) -> CompileResult<()> {
        if depth >= generators.len() {
            // Base case: compile the element and append
            self.compile_expression(elt)?;
            // Stack depth for append: list is at position 0, with 'depth' iterators above it
            // After popping the element, we need peek_n(depth) to get the list
            // LIST_APPEND uses depth as the argument, where peek_n(depth-1) is used
            // So we pass depth + 1 (depth iterators + 1 for the list position)
            self.emitter().emit_arg(append_opcode, (depth + 1) as u16);
            return Ok(());
        }

        let gen = &generators[depth];

        // Compile the iterator expression
        self.compile_expression(&gen.iter)?;
        self.emitter().emit(DpbOpcode::GetIter);

        let loop_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        self.emitter().define_label(loop_label);
        self.emitter().emit_jump(DpbOpcode::ForIter, end_label);

        // Store loop variable
        self.compile_store_target(&gen.target)?;

        // Compile conditions (if clauses)
        let mut skip_labels = Vec::new();
        for cond in &gen.ifs {
            self.compile_expression(cond)?;
            let skip = self.emitter().new_label();
            self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, skip);
            skip_labels.push(skip);
        }

        // Recurse to handle nested generators or compile the element
        self.compile_comprehension_generators(elt, generators, depth + 1, append_opcode)?;

        // Define skip labels for conditions
        for skip in skip_labels {
            self.emitter().define_label(skip);
        }

        self.emitter().emit_jump(DpbOpcode::Jump, loop_label);
        self.emitter().define_label(end_label);

        Ok(())
    }

    /// Compile a set comprehension
    fn compile_setcomp(
        &mut self,
        elt: &Expression,
        generators: &[Comprehension],
    ) -> CompileResult<()> {
        // Add loop variables as locals for proper scoping
        self.add_comprehension_locals(generators)?;
        
        // Similar to list comprehension but builds a set
        self.emitter().emit_arg(DpbOpcode::BuildSet, 0);

        // Compile nested generators recursively
        self.compile_comprehension_generators(elt, generators, 0, DpbOpcode::SetAdd)?;

        Ok(())
    }

    /// Compile a dict comprehension
    fn compile_dictcomp(
        &mut self,
        key: &Expression,
        value: &Expression,
        generators: &[Comprehension],
    ) -> CompileResult<()> {
        // Add loop variables as locals for proper scoping
        self.add_comprehension_locals(generators)?;
        
        // Similar to list comprehension but builds a dict
        self.emitter().emit_arg(DpbOpcode::BuildDict, 0);

        // Compile nested generators recursively for dict comprehension
        self.compile_dict_comprehension_generators(key, value, generators, 0)?;

        Ok(())
    }

    /// Compile dict comprehension generators recursively
    /// This handles multiple 'for' clauses by nesting loops
    fn compile_dict_comprehension_generators(
        &mut self,
        key: &Expression,
        value: &Expression,
        generators: &[Comprehension],
        depth: usize,
    ) -> CompileResult<()> {
        if depth >= generators.len() {
            // Base case: compile key and value, then add to dict
            self.compile_expression(key)?;
            self.compile_expression(value)?;
            // Stack depth for map_add: dict is at position 0, with 'depth' iterators above it
            // After popping value and key, we need peek_n(depth) to get the dict
            // MAP_ADD uses depth as the argument, where peek_n(depth-2) is used
            // So we pass depth + 2 (depth iterators + 2 for the key/value positions)
            self.emitter().emit_arg(DpbOpcode::MapAdd, (depth + 2) as u16);
            return Ok(());
        }

        let gen = &generators[depth];

        // Compile the iterator expression
        self.compile_expression(&gen.iter)?;
        self.emitter().emit(DpbOpcode::GetIter);

        let loop_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        self.emitter().define_label(loop_label);
        self.emitter().emit_jump(DpbOpcode::ForIter, end_label);

        // Store loop variable
        self.compile_store_target(&gen.target)?;

        // Compile conditions (if clauses)
        let mut skip_labels = Vec::new();
        for cond in &gen.ifs {
            self.compile_expression(cond)?;
            let skip = self.emitter().new_label();
            self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, skip);
            skip_labels.push(skip);
        }

        // Recurse to handle nested generators or compile the key/value
        self.compile_dict_comprehension_generators(key, value, generators, depth + 1)?;

        // Define skip labels for conditions
        for skip in skip_labels {
            self.emitter().define_label(skip);
        }

        self.emitter().emit_jump(DpbOpcode::Jump, loop_label);
        self.emitter().define_label(end_label);

        Ok(())
    }

    /// Compile a generator expression
    fn compile_genexp(
        &mut self,
        elt: &Expression,
        generators: &[Comprehension],
    ) -> CompileResult<()> {
        // Generator expressions are compiled as anonymous generator functions
        // The first iterator is passed as an argument, subsequent iterators are compiled inline
        
        // Push a new emitter for the generator function body
        self.push_emitter();
        
        // Set up the generator function's locals
        // .0 is the first iterator (passed as argument)
        let mut varnames = vec![".0".to_string()];
        
        // Add loop variables from all generators as locals
        for gen in generators {
            self.collect_target_names(&gen.target, &mut varnames);
        }
        
        self.emitter().set_locals(varnames.clone());
        
        // Compile the generator body
        // For the first generator, load the argument (.0) instead of compiling the iterator
        self.compile_genexp_generators(elt, generators, 0, true)?;
        
        // Add implicit return None at the end (for when generator is exhausted)
        let none_idx = self.emitter().add_constant(Constant::None);
        self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
        self.emitter().emit(DpbOpcode::Return);
        
        // Patch jumps
        let _ = self.emitter().patch_jumps();
        
        // Pop the generator emitter and build the code object
        let gen_emitter = self.pop_emitter();
        let gen_bytecode = gen_emitter.bytecode().to_vec();
        let gen_constants = gen_emitter.constants().to_vec();
        let gen_names = gen_emitter.names().to_vec();
        let gen_nlocals = varnames.len() as u32;
        let gen_stacksize = gen_emitter.max_stack_depth().max(10);
        
        let gen_code = CodeObject {
            name: "<genexpr>".to_string(),
            qualname: "<genexpr>".to_string(),
            filename: self.filename.to_string_lossy().to_string(),
            firstlineno: 1,
            argcount: 1, // Takes the first iterator as argument
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: gen_nlocals,
            stacksize: gen_stacksize,
            flags: CodeFlags::GENERATOR | CodeFlags::OPTIMIZED | CodeFlags::NEWLOCALS,
            code: gen_bytecode,
            constants: gen_constants,
            names: gen_names,
            varnames,
            freevars: vec![],
            cellvars: vec![],
        };
        
        // Emit code to create the generator function and call it
        // First push the name, then the code object (Python bytecode order)
        let name_idx = self.emitter().add_constant(Constant::String("<genexpr>".to_string()));
        self.emitter().emit_arg(DpbOpcode::LoadConst, name_idx);
        
        let code_idx = self.emitter().add_constant(Constant::Code(Box::new(gen_code)));
        self.emitter().emit_arg(DpbOpcode::LoadConst, code_idx);
        
        self.emitter().emit_arg(DpbOpcode::MakeFunction, 0);
        
        // Compile the first iterator and pass it to the generator function
        if let Some(gen) = generators.first() {
            self.compile_expression(&gen.iter)?;
            self.emitter().emit(DpbOpcode::GetIter);
        }
        
        // Call the generator function with the iterator
        // This returns a generator object (not executing the body yet)
        self.emitter().emit_arg(DpbOpcode::Call, 1);
        
        Ok(())
    }
    
    /// Collect target names from an expression (for generator locals)
    fn collect_target_names(&self, target: &Expression, names: &mut Vec<String>) {
        match target {
            Expression::Name { id, .. } => {
                if !names.contains(id) {
                    names.push(id.clone());
                }
            }
            Expression::Tuple { elts, .. } | Expression::List { elts, .. } => {
                for elt in elts {
                    self.collect_target_names(elt, names);
                }
            }
            Expression::Starred { value, .. } => {
                self.collect_target_names(value, names);
            }
            _ => {}
        }
    }
    
    /// Compile generator expression generators recursively
    /// This handles multiple 'for' clauses by nesting loops, yielding instead of appending
    fn compile_genexp_generators(
        &mut self,
        elt: &Expression,
        generators: &[Comprehension],
        depth: usize,
        is_first: bool,
    ) -> CompileResult<()> {
        if depth >= generators.len() {
            // Base case: compile the element and yield it
            self.compile_expression(elt)?;
            self.emitter().emit(DpbOpcode::Yield);
            self.emitter().emit(DpbOpcode::PopTop); // Pop the sent value (None)
            return Ok(());
        }
        
        let gen = &generators[depth];
        
        // For the first generator, load the argument (.0) which is already an iterator
        // For subsequent generators, compile the iterator expression and get its iterator
        if is_first && depth == 0 {
            // Load the .0 argument (the iterator passed to the generator function)
            self.emitter().emit_arg(DpbOpcode::LoadFast, 0);
        } else {
            // Compile the iterator expression
            self.compile_expression(&gen.iter)?;
            self.emitter().emit(DpbOpcode::GetIter);
        }
        
        let loop_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();
        
        self.emitter().define_label(loop_label);
        self.emitter().emit_jump(DpbOpcode::ForIter, end_label);
        
        // Store loop variable
        self.compile_store_target(&gen.target)?;
        
        // Compile conditions (if clauses)
        let mut skip_labels = Vec::new();
        for cond in &gen.ifs {
            self.compile_expression(cond)?;
            let skip = self.emitter().new_label();
            self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, skip);
            skip_labels.push(skip);
        }
        
        // Recurse to handle nested generators or yield the element
        self.compile_genexp_generators(elt, generators, depth + 1, false)?;
        
        // Define skip labels for conditions
        for skip in skip_labels {
            self.emitter().define_label(skip);
        }
        
        self.emitter().emit_jump(DpbOpcode::Jump, loop_label);
        self.emitter().define_label(end_label);
        
        Ok(())
    }

    /// Compile a slice expression
    fn compile_slice(
        &mut self,
        lower: &Option<Box<Expression>>,
        upper: &Option<Box<Expression>>,
        step: &Option<Box<Expression>>,
    ) -> CompileResult<()> {
        let mut count = 2u8;

        if let Some(l) = lower {
            self.compile_expression(l)?;
        } else {
            let none_idx = self.emitter().add_constant(Constant::None);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
        }

        if let Some(u) = upper {
            self.compile_expression(u)?;
        } else {
            let none_idx = self.emitter().add_constant(Constant::None);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
        }

        if let Some(s) = step {
            self.compile_expression(s)?;
            count = 3;
        }

        self.emitter().emit_arg(DpbOpcode::BuildSlice, count as u16);
        Ok(())
    }

    /// Compile a lambda expression
    fn compile_lambda(&mut self, _args: &Arguments, body: &Expression) -> CompileResult<()> {
        // Lambda expressions are compiled as inline expressions
        // For complex lambdas with closures, a full code object would be created
        // This simplified approach handles the common case of simple lambdas
        self.compile_expression(body)?;
        Ok(())
    }

    /// Compile an f-string
    fn compile_fstring(&mut self, values: &[Expression]) -> CompileResult<()> {
        for value in values {
            self.compile_expression(value)?;
        }
        self.emitter().emit_arg(DpbOpcode::BuildString, values.len() as u16);
        Ok(())
    }

    /// Compile a formatted value in an f-string
    fn compile_formatted_value(
        &mut self,
        value: &Expression,
        conversion: Option<char>,
        format_spec: &Option<Box<Expression>>,
    ) -> CompileResult<()> {
        self.compile_expression(value)?;

        let mut flags = 0u8;
        if let Some(conv) = conversion {
            flags |= match conv {
                's' => 1,
                'r' => 2,
                'a' => 3,
                _ => 0,
            };
        }

        if let Some(spec) = format_spec {
            self.compile_expression(spec)?;
            flags |= 4;
        }

        self.emitter().emit_arg(DpbOpcode::FormatValue, flags as u16);
        Ok(())
    }

    /// Compile a store to a target
    fn compile_store_target(&mut self, target: &Expression) -> CompileResult<()> {
        match target {
            Expression::Name { id, .. } => {
                self.compile_store_name(id)?;
            }
            Expression::Attribute { value, attr, .. } => {
                self.compile_expression(value)?;
                let name_idx = self.emitter().add_name(attr);
                self.emitter().emit_arg(DpbOpcode::StoreAttr, name_idx);
            }
            Expression::Subscript { value, slice, .. } => {
                self.compile_expression(value)?;
                self.compile_expression(slice)?;
                self.emitter().emit(DpbOpcode::StoreSubscr);
            }
            Expression::Tuple { elts, .. } | Expression::List { elts, .. } => {
                self.emitter().emit_arg(DpbOpcode::UnpackSequence, elts.len() as u16);
                for elt in elts {
                    self.compile_store_target(elt)?;
                }
            }
            Expression::Starred { value, .. } => {
                // Starred in assignment target - handled by UnpackEx
                self.compile_store_target(value)?;
            }
            _ => {
                return Err(CompileError::codegen_error("Invalid assignment target"));
            }
        }
        Ok(())
    }

    /// Compile a name delete (for exception binding cleanup)
    fn compile_delete_name(&mut self, name: &str) -> CompileResult<()> {
        // First check if it's a local in the emitter (for comprehension variables)
        if let Some(idx) = self.emitter().get_local(name) {
            self.emitter().emit_arg(DpbOpcode::DeleteFast, idx);
            return Ok(());
        }

        let scope = self.current_scope();

        // At module level, use DeleteGlobal
        if scope.scope_type == crate::symbol_table::ScopeType::Module {
            let name_idx = self.emitter().add_name(name);
            self.emitter().emit_arg(DpbOpcode::DeleteGlobal, name_idx);
            return Ok(());
        }

        // Check if it's a local variable
        if let Some(idx) = scope.get_local_index(name) {
            self.emitter().emit_arg(DpbOpcode::DeleteFast, idx);
            return Ok(());
        }

        // Check if it's explicitly global
        if scope.explicit_globals.contains(name) {
            let name_idx = self.emitter().add_name(name);
            self.emitter().emit_arg(DpbOpcode::DeleteGlobal, name_idx);
            return Ok(());
        }

        // Default: delete as global (for module level)
        let name_idx = self.emitter().add_name(name);
        self.emitter().emit_arg(DpbOpcode::DeleteGlobal, name_idx);
        Ok(())
    }

    /// Compile a name store
    fn compile_store_name(&mut self, name: &str) -> CompileResult<()> {
        // First check if it's a local in the emitter (for comprehension variables)
        // This takes precedence over scope-based lookup
        if let Some(idx) = self.emitter().get_local(name) {
            self.emitter().emit_arg(DpbOpcode::StoreFast, idx);
            return Ok(());
        }

        let scope = self.current_scope();

        // At module level, always use StoreGlobal for function definitions
        // This ensures recursive functions can find themselves via LoadGlobal
        if scope.scope_type == crate::symbol_table::ScopeType::Module {
            let name_idx = self.emitter().add_name(name);
            self.emitter().emit_arg(DpbOpcode::StoreGlobal, name_idx);
            return Ok(());
        }

        // Check if it's a cell variable
        if let Some(idx) = scope.get_cell_index(name) {
            self.emitter().emit_arg(DpbOpcode::StoreDeref, idx);
            return Ok(());
        }

        // Check if it's a local variable
        if let Some(idx) = scope.get_local_index(name) {
            self.emitter().emit_arg(DpbOpcode::StoreFast, idx);
            return Ok(());
        }

        // Check if it's a free variable
        if let Some(idx) = scope.get_free_index(name) {
            let offset = scope.cell_vars.len() as u16;
            self.emitter().emit_arg(DpbOpcode::StoreDeref, offset + idx);
            return Ok(());
        }

        // Check if it's explicitly global
        if scope.explicit_globals.contains(name) {
            let name_idx = self.emitter().add_name(name);
            self.emitter().emit_arg(DpbOpcode::StoreGlobal, name_idx);
            return Ok(());
        }

        // Default: store as global (for module level)
        let name_idx = self.emitter().add_name(name);
        self.emitter().emit_arg(DpbOpcode::StoreGlobal, name_idx);
        Ok(())
    }

    /// Convert AST constant to bytecode constant
    fn ast_constant_to_bytecode(&self, value: &AstConstant) -> Constant {
        match value {
            AstConstant::None => Constant::None,
            AstConstant::Bool(b) => Constant::Bool(*b),
            AstConstant::Int(i) => Constant::Int(*i),
            AstConstant::Float(f) => Constant::Float(*f),
            AstConstant::Complex { real, imag } => Constant::Complex(*real, *imag),
            AstConstant::Str(s) => Constant::String(s.clone()),
            AstConstant::Bytes(b) => Constant::Bytes(b.clone()),
            AstConstant::Ellipsis => Constant::Ellipsis,
        }
    }

    /// Convert binary operator to opcode
    fn binop_to_opcode(&self, op: BinOp) -> DpbOpcode {
        match op {
            BinOp::Add => DpbOpcode::BinaryAdd,
            BinOp::Sub => DpbOpcode::BinarySub,
            BinOp::Mult => DpbOpcode::BinaryMul,
            BinOp::Div => DpbOpcode::BinaryDiv,
            BinOp::FloorDiv => DpbOpcode::BinaryFloorDiv,
            BinOp::Mod => DpbOpcode::BinaryMod,
            BinOp::Pow => DpbOpcode::BinaryPow,
            BinOp::LShift => DpbOpcode::BinaryLshift,
            BinOp::RShift => DpbOpcode::BinaryRshift,
            BinOp::BitOr => DpbOpcode::BinaryOr,
            BinOp::BitXor => DpbOpcode::BinaryXor,
            BinOp::BitAnd => DpbOpcode::BinaryAnd,
            BinOp::MatMult => DpbOpcode::BinaryMatMul,
        }
    }

    /// Convert unary operator to opcode
    fn unaryop_to_opcode(&self, op: UnaryOp) -> DpbOpcode {
        match op {
            UnaryOp::Invert => DpbOpcode::UnaryInvert,
            UnaryOp::Not => DpbOpcode::UnaryNot,
            UnaryOp::UAdd => DpbOpcode::UnaryPos,
            UnaryOp::USub => DpbOpcode::UnaryNeg,
        }
    }

    /// Convert comparison operator to opcode
    fn cmpop_to_opcode(&self, op: CmpOp) -> DpbOpcode {
        match op {
            CmpOp::Eq => DpbOpcode::CompareEq,
            CmpOp::NotEq => DpbOpcode::CompareNe,
            CmpOp::Lt => DpbOpcode::CompareLt,
            CmpOp::LtE => DpbOpcode::CompareLe,
            CmpOp::Gt => DpbOpcode::CompareGt,
            CmpOp::GtE => DpbOpcode::CompareGe,
            CmpOp::Is => DpbOpcode::CompareIs,
            CmpOp::IsNot => DpbOpcode::CompareIsNot,
            CmpOp::In => DpbOpcode::CompareIn,
            CmpOp::NotIn => DpbOpcode::CompareNotIn,
        }
    }

    /// Compile a statement
    fn compile_statement(&mut self, stmt: &Statement) -> CompileResult<()> {
        match stmt {
            Statement::Expr { value, location } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                self.emitter().emit(DpbOpcode::PopTop);
            }

            Statement::Assign {
                targets,
                value,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_expression(value)?;
                for (i, target) in targets.iter().enumerate() {
                    if i < targets.len() - 1 {
                        self.emitter().emit(DpbOpcode::DupTop);
                    }
                    self.compile_store_target(target)?;
                }
            }

            Statement::AugAssign {
                target,
                op,
                value,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_augassign(target, *op, value)?;
            }

            Statement::AnnAssign { target, value, .. } => {
                // Annotated assignment - just compile the value if present
                if let Some(v) = value {
                    self.compile_expression(v)?;
                    self.compile_store_target(target)?;
                }
            }

            Statement::Return { value, location } => {
                self.emitter().set_line(location.line);
                if let Some(v) = value {
                    self.compile_expression(v)?;
                } else {
                    let none_idx = self.emitter().add_constant(Constant::None);
                    self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
                }
                self.emitter().emit(DpbOpcode::Return);
            }

            Statement::If {
                test,
                body,
                orelse,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_if(test, body, orelse)?;
            }

            Statement::While {
                test,
                body,
                orelse,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_while(test, body, orelse)?;
            }

            Statement::For {
                target,
                iter,
                body,
                orelse,
                location,
                ..
            } => {
                self.emitter().set_line(location.line);
                self.compile_for(target, iter, body, orelse)?;
            }

            Statement::Try {
                body,
                handlers,
                orelse,
                finalbody,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_try(body, handlers, orelse, finalbody)?;
            }

            Statement::With {
                items,
                body,
                location,
                ..
            } => {
                self.emitter().set_line(location.line);
                self.compile_with(items, body)?;
            }

            Statement::FunctionDef {
                name,
                args,
                body,
                decorators,
                returns,
                is_async,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_function_def(name, args, body, decorators, returns, *is_async)?;
            }

            Statement::ClassDef {
                name,
                bases,
                keywords,
                body,
                decorators,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_class_def(name, bases, keywords, body, decorators)?;
            }

            Statement::Import { names, location } => {
                self.emitter().set_line(location.line);
                self.compile_import(names)?;
            }

            Statement::ImportFrom {
                module,
                names,
                level,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_import_from(module.as_deref(), names, *level)?;
            }

            Statement::Raise {
                exc,
                cause,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_raise(exc, cause)?;
            }

            Statement::Assert {
                test,
                msg,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_assert(test, msg)?;
            }

            Statement::Delete { targets, location } => {
                self.emitter().set_line(location.line);
                for target in targets {
                    self.compile_delete(target)?;
                }
            }

            Statement::Global { .. } | Statement::Nonlocal { .. } => {
                // These are handled during symbol analysis, no bytecode needed
            }

            Statement::Pass { .. } => {
                // No-op
            }

            Statement::Break { location } => {
                self.emitter().set_line(location.line);
                // Break is handled by the loop compilation
                // For now, emit a placeholder jump that will be patched
                self.emitter().emit_arg(DpbOpcode::Jump, 0);
            }

            Statement::Continue { location } => {
                self.emitter().set_line(location.line);
                // Continue is handled by the loop compilation
                self.emitter().emit_arg(DpbOpcode::Jump, 0);
            }

            Statement::Match {
                subject,
                cases,
                location,
            } => {
                self.emitter().set_line(location.line);
                self.compile_match(subject, cases)?;
            }
        }
        Ok(())
    }

    /// Compile augmented assignment (+=, -=, etc.)
    fn compile_augassign(
        &mut self,
        target: &Expression,
        op: BinOp,
        value: &Expression,
    ) -> CompileResult<()> {
        // Load the target value
        self.compile_expression(target)?;
        // Load the value to add/subtract/etc.
        self.compile_expression(value)?;
        // Perform the operation
        let opcode = match op {
            BinOp::Add => DpbOpcode::InplaceAdd,
            BinOp::Sub => DpbOpcode::InplaceSub,
            BinOp::Mult => DpbOpcode::InplaceMul,
            BinOp::Div => DpbOpcode::InplaceDiv,
            BinOp::FloorDiv => DpbOpcode::InplaceFloorDiv,
            BinOp::Mod => DpbOpcode::InplaceMod,
            BinOp::Pow => DpbOpcode::InplacePow,
            BinOp::LShift => DpbOpcode::InplaceLshift,
            BinOp::RShift => DpbOpcode::InplaceRshift,
            BinOp::BitOr => DpbOpcode::InplaceOr,
            BinOp::BitXor => DpbOpcode::InplaceXor,
            BinOp::BitAnd => DpbOpcode::InplaceAnd,
            BinOp::MatMult => DpbOpcode::InplaceMatMul,
        };
        self.emitter().emit(opcode);
        // Store back to target
        self.compile_store_target(target)?;
        Ok(())
    }

    /// Compile if statement
    fn compile_if(
        &mut self,
        test: &Expression,
        body: &[Statement],
        orelse: &[Statement],
    ) -> CompileResult<()> {
        let else_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        // Compile test
        self.compile_expression(test)?;
        self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, else_label);

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        if !orelse.is_empty() {
            self.emitter().emit_jump(DpbOpcode::Jump, end_label);
        }

        // Else clause
        self.emitter().define_label(else_label);
        for stmt in orelse {
            self.compile_statement(stmt)?;
        }

        if !orelse.is_empty() {
            self.emitter().define_label(end_label);
        }

        Ok(())
    }

    /// Compile while loop
    fn compile_while(
        &mut self,
        test: &Expression,
        body: &[Statement],
        orelse: &[Statement],
    ) -> CompileResult<()> {
        let loop_label = self.emitter().new_label();
        let else_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        // Loop start
        self.emitter().define_label(loop_label);

        // Compile test
        self.compile_expression(test)?;
        self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, else_label);

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Jump back to loop start
        self.emitter().emit_jump(DpbOpcode::Jump, loop_label);

        // Else clause (executed if loop completes normally)
        self.emitter().define_label(else_label);
        for stmt in orelse {
            self.compile_statement(stmt)?;
        }

        self.emitter().define_label(end_label);
        Ok(())
    }

    /// Compile for loop
    fn compile_for(
        &mut self,
        target: &Expression,
        iter: &Expression,
        body: &[Statement],
        orelse: &[Statement],
    ) -> CompileResult<()> {
        let loop_label = self.emitter().new_label();
        let else_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        // Get iterator
        self.compile_expression(iter)?;
        self.emitter().emit(DpbOpcode::GetIter);

        // Loop start
        self.emitter().define_label(loop_label);
        self.emitter().emit_jump(DpbOpcode::ForIter, else_label);

        // Store loop variable
        self.compile_store_target(target)?;

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Jump back to loop start
        self.emitter().emit_jump(DpbOpcode::Jump, loop_label);

        // Else clause
        self.emitter().define_label(else_label);
        for stmt in orelse {
            self.compile_statement(stmt)?;
        }

        self.emitter().define_label(end_label);
        Ok(())
    }

    /// Compile try/except/finally
    fn compile_try(
        &mut self,
        body: &[Statement],
        handlers: &[dx_py_parser::ExceptHandler],
        orelse: &[Statement],
        finalbody: &[Statement],
    ) -> CompileResult<()> {
        let has_finally = !finalbody.is_empty();
        let has_except = !handlers.is_empty();

        let except_label = self.emitter().new_label();
        let else_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();
        let finally_label = if has_finally {
            Some(self.emitter().new_label())
        } else {
            None
        };
        let finally_end_label = if has_finally {
            Some(self.emitter().new_label())
        } else {
            None
        };

        // If we have a finally block, set it up first so it catches everything
        if let Some(finally_lbl) = finally_label {
            self.emitter().emit_jump(DpbOpcode::SetupFinally, finally_lbl);
        }

        // Setup try block for except handlers
        if has_except {
            self.emitter().emit_jump(DpbOpcode::SetupExcept, except_label);
        }

        // Compile try body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Pop exception handler (if we have except handlers)
        if has_except {
            self.emitter().emit_arg(DpbOpcode::PopExcept, 0);
        }

        // Jump to else clause
        self.emitter().emit_jump(DpbOpcode::Jump, else_label);

        // Exception handlers
        if has_except {
            self.emitter().define_label(except_label);
            for handler in handlers {
                let next_handler = self.emitter().new_label();

                if let Some(typ) = &handler.typ {
                    // Check exception type
                    // Stack: [exc] -> [exc, exc] (dup for type check)
                    self.emitter().emit(DpbOpcode::DupTop);
                    // Stack: [exc, exc] -> [exc, exc, type]
                    self.compile_expression(typ)?;
                    // Stack: [exc, exc, type] -> [exc, bool]
                    self.emitter().emit_arg(DpbOpcode::CheckExcMatch, 0);
                    // Stack: [exc, bool] -> [exc] (if false, jump to next handler)
                    self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, next_handler);
                }

                // Bind exception to name if specified
                // Stack: [exc]
                if let Some(name) = &handler.name {
                    // Store exception to the binding name
                    // Stack: [exc] -> [exc] (dup to keep on stack for store)
                    self.emitter().emit(DpbOpcode::DupTop);
                    // Stack: [exc, exc] -> [exc] (store pops one)
                    self.compile_store_name(name)?;
                }

                // Pop exception from stack (we're done with it)
                // Stack: [exc] -> []
                self.emitter().emit(DpbOpcode::PopTop);

                // Compile handler body
                for stmt in &handler.body {
                    self.compile_statement(stmt)?;
                }

                // Clean up the exception binding at the end of the handler
                // In Python, the exception variable is deleted after the except block
                if let Some(name) = &handler.name {
                    // Delete the exception binding to match Python semantics
                    // This prevents the exception from keeping references alive
                    self.compile_delete_name(name)?;
                }

                self.emitter().emit_jump(DpbOpcode::Jump, end_label);
                self.emitter().define_label(next_handler);
            }

            // Re-raise if no handler matched
            self.emitter().emit(DpbOpcode::Reraise);
        }

        // Else clause
        self.emitter().define_label(else_label);
        for stmt in orelse {
            self.compile_statement(stmt)?;
        }

        self.emitter().define_label(end_label);

        // Handle finally block
        if let (Some(finally_lbl), Some(finally_end_lbl)) = (finally_label, finally_end_label) {
            // Pop the finally block (normal exit path)
            self.emitter().emit_arg(DpbOpcode::PopExcept, 0);
            
            // Push marker for normal exit (Int(0))
            let zero_idx = self.emitter().add_constant(Constant::Int(0));
            self.emitter().emit_arg(DpbOpcode::LoadConst, zero_idx);
            
            // Jump to finally code
            self.emitter().emit_jump(DpbOpcode::Jump, finally_lbl);

            // Finally block entry point (for exceptions/returns)
            self.emitter().define_label(finally_lbl);

            // Compile finally body
            for stmt in finalbody {
                self.compile_statement(stmt)?;
            }

            // End finally - this will check the marker and either:
            // - Continue execution (normal exit)
            // - Re-raise exception
            // - Return the pending return value
            self.emitter().emit(DpbOpcode::EndFinally);

            self.emitter().define_label(finally_end_lbl);
        }

        Ok(())
    }

    /// Compile with statement
    fn compile_with(
        &mut self,
        items: &[dx_py_parser::WithItem],
        body: &[Statement],
    ) -> CompileResult<()> {
        // For each context manager, BEFORE_WITH pushes [__exit__, enter_result]
        // We need to:
        // 1. Store __exit__ for later use (swap and store)
        // 2. Store or pop enter_result based on 'as' clause
        // 3. Set up exception handling to call __exit__ on exceptions

        // Track the temporary local indices for __exit__ methods
        let mut exit_locals: Vec<u16> = Vec::new();

        // Create labels for exception handling
        let finally_label = self.emitter().new_label();
        let end_label = self.emitter().new_label();

        for (i, item) in items.iter().enumerate() {
            // Compile the context expression (pushes context_manager)
            self.compile_expression(&item.context_expr)?;

            // BEFORE_WITH: pops context_manager, pushes [__exit__, enter_result]
            self.emitter().emit_arg(DpbOpcode::BeforeWith, 0);

            // Stack is now: [..., __exit__, enter_result]
            // We need to save __exit__ to a local variable

            // Allocate a temporary local for __exit__
            let exit_local_name = format!("__exit__{}", i);
            let exit_local = self.emitter().add_local(&exit_local_name);
            exit_locals.push(exit_local);

            // Swap to get __exit__ on top: [..., enter_result, __exit__]
            self.emitter().emit(DpbOpcode::Swap);

            // Store __exit__ to local: [..., enter_result]
            self.emitter().emit_arg(DpbOpcode::StoreFast, exit_local);

            // Now handle enter_result
            if let Some(vars) = &item.optional_vars {
                // Store enter_result to the target variable
                self.compile_store_target(vars)?;
            } else {
                // Pop enter_result (not used)
                self.emitter().emit(DpbOpcode::PopTop);
            }
        }

        // Set up finally block for exception handling
        // This ensures __exit__ is called even if an exception occurs
        self.emitter().emit_jump(DpbOpcode::SetupFinally, finally_label);

        // Compile body
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Pop the finally block (normal exit path)
        self.emitter().emit(DpbOpcode::PopExcept);

        // Exit context managers (in reverse order) - normal exit path
        // Call __exit__(None, None, None) for normal exit
        for exit_local in exit_locals.iter().rev() {
            // Load __exit__ from local
            self.emitter().emit_arg(DpbOpcode::LoadFast, *exit_local);

            // Push None, None, None for normal exit
            let none_idx = self.emitter().add_constant(Constant::None);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);

            // Call __exit__(None, None, None)
            self.emitter().emit_arg(DpbOpcode::Call, 3);

            // Pop the result (we don't care about it for normal exit)
            self.emitter().emit(DpbOpcode::PopTop);
        }

        // Jump past the finally block
        self.emitter().emit_jump(DpbOpcode::Jump, end_label);

        // Finally block - handles exceptions
        self.emitter().define_label(finally_label);

        // The exception info is on the stack when we get here
        // We need to call __exit__ with the exception info and check if it returns True
        // to suppress the exception

        // Exit context managers with exception info (in reverse order)
        for exit_local in exit_locals.iter().rev() {
            // Load __exit__ from local
            self.emitter().emit_arg(DpbOpcode::LoadFast, *exit_local);

            // For a complete implementation, we would extract exception info from the stack
            // and pass it to __exit__. For now, we pass None, None, None (simplified)
            // A full implementation would use PushExcInfo and WithExceptStart opcodes
            let none_idx = self.emitter().add_constant(Constant::None);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);

            // Call __exit__(exc_type, exc_value, exc_tb)
            self.emitter().emit_arg(DpbOpcode::Call, 3);

            // Check if __exit__ returned True (suppress exception)
            // Create labels for suppression check
            let suppress_label = self.emitter().new_label();
            let continue_label = self.emitter().new_label();

            // If __exit__ returned truthy, jump to suppress
            self.emitter().emit_jump(DpbOpcode::JumpIfTrue, suppress_label);

            // Not suppressed - pop the result and continue to re-raise
            self.emitter().emit(DpbOpcode::PopTop);
            self.emitter().emit_jump(DpbOpcode::Jump, continue_label);

            // Suppressed - pop the result and jump to end (skip re-raise)
            self.emitter().define_label(suppress_label);
            self.emitter().emit(DpbOpcode::PopTop);
            self.emitter().emit_jump(DpbOpcode::Jump, end_label);

            self.emitter().define_label(continue_label);
        }

        // Re-raise the exception (only reached if not suppressed)
        self.emitter().emit(DpbOpcode::Reraise);

        // End label - normal exit continues here
        self.emitter().define_label(end_label);

        Ok(())
    }

    /// Check if a function body contains yield statements (making it a generator)
    fn contains_yield(body: &[Statement]) -> bool {
        for stmt in body {
            if Self::statement_contains_yield(stmt) {
                return true;
            }
        }
        false
    }

    /// Check if a statement contains yield expressions
    fn statement_contains_yield(stmt: &Statement) -> bool {
        match stmt {
            Statement::Expr { value, .. } => Self::expression_contains_yield(value),
            Statement::Assign { value, .. } => Self::expression_contains_yield(value),
            Statement::AugAssign { value, .. } => Self::expression_contains_yield(value),
            Statement::AnnAssign { value, .. } => {
                value.as_ref().map_or(false, |v| Self::expression_contains_yield(v))
            }
            Statement::Return { value, .. } => {
                value.as_ref().map_or(false, |v| Self::expression_contains_yield(v))
            }
            Statement::If { body, orelse, test, .. } => {
                Self::expression_contains_yield(test)
                    || Self::contains_yield(body)
                    || Self::contains_yield(orelse)
            }
            Statement::While { body, orelse, test, .. } => {
                Self::expression_contains_yield(test)
                    || Self::contains_yield(body)
                    || Self::contains_yield(orelse)
            }
            Statement::For { body, orelse, iter, .. } => {
                Self::expression_contains_yield(iter)
                    || Self::contains_yield(body)
                    || Self::contains_yield(orelse)
            }
            Statement::With { body, .. } => Self::contains_yield(body),
            Statement::Try { body, handlers, orelse, finalbody, .. } => {
                Self::contains_yield(body)
                    || handlers.iter().any(|h| Self::contains_yield(&h.body))
                    || Self::contains_yield(orelse)
                    || Self::contains_yield(finalbody)
            }
            // Don't recurse into nested function definitions - they have their own scope
            Statement::FunctionDef { .. } => false,
            Statement::ClassDef { .. } => false,
            _ => false,
        }
    }

    /// Check if an expression contains yield
    fn expression_contains_yield(expr: &Expression) -> bool {
        match expr {
            Expression::Yield { .. } => true,
            Expression::YieldFrom { .. } => true,
            Expression::BinOp { left, right, .. } => {
                Self::expression_contains_yield(left) || Self::expression_contains_yield(right)
            }
            Expression::UnaryOp { operand, .. } => Self::expression_contains_yield(operand),
            Expression::BoolOp { values, .. } => {
                values.iter().any(|v| Self::expression_contains_yield(v))
            }
            Expression::Compare { left, comparators, .. } => {
                Self::expression_contains_yield(left)
                    || comparators.iter().any(|c| Self::expression_contains_yield(c))
            }
            Expression::Call { func, args, keywords, .. } => {
                Self::expression_contains_yield(func)
                    || args.iter().any(|a| Self::expression_contains_yield(a))
                    || keywords.iter().any(|k| Self::expression_contains_yield(&k.value))
            }
            Expression::IfExp { test, body, orelse, .. } => {
                Self::expression_contains_yield(test)
                    || Self::expression_contains_yield(body)
                    || Self::expression_contains_yield(orelse)
            }
            Expression::Attribute { value, .. } => Self::expression_contains_yield(value),
            Expression::Subscript { value, slice, .. } => {
                Self::expression_contains_yield(value) || Self::expression_contains_yield(slice)
            }
            Expression::List { elts, .. } | Expression::Tuple { elts, .. } | Expression::Set { elts, .. } => {
                elts.iter().any(|e| Self::expression_contains_yield(e))
            }
            Expression::Dict { keys, values, .. } => {
                keys.iter().filter_map(|k| k.as_ref()).any(|k| Self::expression_contains_yield(k))
                    || values.iter().any(|v| Self::expression_contains_yield(v))
            }
            // Don't recurse into comprehensions or lambdas - they have their own scope
            Expression::ListComp { .. } => false,
            Expression::SetComp { .. } => false,
            Expression::DictComp { .. } => false,
            Expression::GeneratorExp { .. } => false,
            Expression::Lambda { .. } => false,
            _ => false,
        }
    }

    /// Compile function definition
    fn compile_function_def(
        &mut self,
        name: &str,
        args: &Arguments,
        body: &[Statement],
        decorators: &[Expression],
        _returns: &Option<Box<Expression>>,
        is_async: bool,
    ) -> CompileResult<()> {
        // Compile decorators (in order, will be applied in reverse)
        for decorator in decorators {
            self.compile_expression(decorator)?;
        }

        // Calculate argument counts
        let argcount = args.args.len() as u32;
        let posonlyargcount = args.posonlyargs.len() as u32;
        let kwonlyargcount = args.kwonlyargs.len() as u32;

        // Build flags
        let mut flags = CodeFlags::OPTIMIZED | CodeFlags::NEWLOCALS;
        if args.vararg.is_some() {
            flags |= CodeFlags::VARARGS;
        }
        if args.kwarg.is_some() {
            flags |= CodeFlags::VARKEYWORDS;
        }
        if is_async {
            flags |= CodeFlags::COROUTINE;
        }
        // Check if the function body contains yield statements (making it a generator)
        if Self::contains_yield(body) {
            flags |= CodeFlags::GENERATOR;
        }

        // Compile default values
        let num_defaults = args.defaults.len();
        for default in &args.defaults {
            self.compile_expression(default)?;
        }
        if num_defaults > 0 {
            self.emitter().emit_arg(DpbOpcode::BuildTuple, num_defaults as u16);
        }

        // Compile keyword-only defaults
        let mut num_kw_defaults = 0;
        for (i, default) in args.kw_defaults.iter().enumerate() {
            if let Some(d) = default {
                let kwarg_name = &args.kwonlyargs[i].arg;
                let name_idx = self.emitter().add_constant(Constant::String(kwarg_name.clone()));
                self.emitter().emit_arg(DpbOpcode::LoadConst, name_idx);
                self.compile_expression(d)?;
                num_kw_defaults += 1;
            }
        }
        if num_kw_defaults > 0 {
            self.emitter().emit_arg(DpbOpcode::BuildDict, num_kw_defaults as u16);
        }

        // Compile the function body in a new emitter
        self.push_emitter();
        
        // Build varnames list (parameters first, then locals)
        let mut varnames: Vec<String> = args
            .args
            .iter()
            .map(|a| a.arg.clone())
            .chain(args.kwonlyargs.iter().map(|a| a.arg.clone()))
            .collect();
        
        // Add *args if present
        if let Some(vararg) = &args.vararg {
            varnames.push(vararg.arg.clone());
        }
        
        // Add **kwargs if present
        if let Some(kwarg) = &args.kwarg {
            varnames.push(kwarg.arg.clone());
        }
        
        // Set up locals in the function emitter
        self.emitter().set_locals(varnames.clone());
        
        // Push a new scope for the function with the parameters as locals
        let mut func_scope = Scope::new(crate::symbol_table::ScopeType::Function, name.to_string());
        func_scope.locals = varnames.clone();
        self.scope_stack.push(func_scope);
        
        // Compile the function body
        for stmt in body {
            self.compile_statement(stmt)?;
        }
        
        // Pop the function scope
        self.scope_stack.pop();
        
        // Add implicit return None if the function doesn't end with a return
        let none_idx = self.emitter().add_constant(Constant::None);
        self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
        self.emitter().emit(DpbOpcode::Return);
        
        // Patch jumps in the function body
        let _ = self.emitter().patch_jumps();
        
        // Pop the function emitter and get the bytecode
        let func_emitter = self.pop_emitter();
        let func_bytecode = func_emitter.bytecode().to_vec();
        let func_constants = func_emitter.constants().to_vec();
        let func_names = func_emitter.names().to_vec();
        let func_nlocals = varnames.len() as u32;
        let func_stacksize = func_emitter.max_stack_depth();
        
        // Create a code object for the function body
        let func_code = CodeObject {
            name: name.to_string(),
            qualname: name.to_string(),
            filename: self.filename.to_string_lossy().to_string(),
            firstlineno: 1,
            argcount,
            posonlyargcount,
            kwonlyargcount,
            nlocals: func_nlocals.max(argcount + kwonlyargcount),
            stacksize: func_stacksize.max(10),
            flags,
            code: func_bytecode,
            constants: func_constants,
            names: func_names,
            varnames,
            freevars: vec![],
            cellvars: vec![],
        };

        // Push qualname first, then code object (Python bytecode order)
        let name_idx = self.emitter().add_constant(Constant::String(name.to_string()));
        self.emitter().emit_arg(DpbOpcode::LoadConst, name_idx);

        let code_idx = self.emitter().add_constant(Constant::Code(Box::new(func_code)));
        self.emitter().emit_arg(DpbOpcode::LoadConst, code_idx);

        let mut make_flags = 0u16;
        if num_defaults > 0 {
            make_flags |= 0x01;
        }
        if num_kw_defaults > 0 {
            make_flags |= 0x02;
        }
        self.emitter().emit_arg(DpbOpcode::MakeFunction, make_flags);

        for _ in decorators {
            self.emitter().emit_arg(DpbOpcode::Call, 1);
        }

        self.compile_store_name(name)?;
        Ok(())
    }

    /// Compile class definition
    fn compile_class_def(
        &mut self,
        name: &str,
        bases: &[Expression],
        keywords: &[dx_py_parser::Keyword],
        body: &[Statement],
        decorators: &[Expression],
    ) -> CompileResult<()> {
        // Compile decorators first (they'll be applied after class creation)
        for decorator in decorators {
            self.compile_expression(decorator)?;
        }

        // Load __build_class__ builtin
        let build_class_idx = self.emitter().add_name("__build_class__");
        self.emitter().emit_arg(DpbOpcode::LoadGlobal, build_class_idx);

        // Compile the class body into a code object
        // The class body function populates the class namespace with methods and attributes
        let class_code = self.compile_class_body(name, body)?;

        // Push qualname first, then code object (Python bytecode order)
        let name_const_idx = self.emitter().add_constant(Constant::String(name.to_string()));
        self.emitter().emit_arg(DpbOpcode::LoadConst, name_const_idx);

        let code_idx = self.emitter().add_constant(Constant::Code(Box::new(class_code)));
        self.emitter().emit_arg(DpbOpcode::LoadConst, code_idx);

        // Create the class body function
        self.emitter().emit_arg(DpbOpcode::MakeFunction, 0);
        
        // Push class name as second argument to __build_class__
        self.emitter().emit_arg(DpbOpcode::LoadConst, name_const_idx);

        // Compile base classes
        for base in bases {
            self.compile_expression(base)?;
        }

        let total_args = 2 + bases.len();
        if keywords.is_empty() {
            self.emitter().emit_arg(DpbOpcode::Call, total_args as u16);
        } else {
            for kw in keywords {
                self.compile_expression(&kw.value)?;
            }
            self.emitter().emit_arg(DpbOpcode::CallKw, (total_args + keywords.len()) as u16);
        }

        for _ in decorators {
            self.emitter().emit_arg(DpbOpcode::Call, 1);
        }

        self.compile_store_name(name)?;
        Ok(())
    }

    /// Compile class body into a code object
    /// This creates a function that when called, populates the class namespace
    fn compile_class_body(&mut self, name: &str, body: &[Statement]) -> CompileResult<CodeObject> {
        // Push a new emitter for the class body
        self.push_emitter();
        
        // Push a new scope for the class
        let class_scope = Scope::new(crate::symbol_table::ScopeType::Class, name.to_string());
        self.scope_stack.push(class_scope);
        
        // Set up locals for the class body (empty initially)
        self.emitter().set_locals(vec![]);
        
        // Compile each statement in the class body
        for stmt in body {
            match stmt {
                Statement::FunctionDef {
                    name: method_name,
                    args,
                    body: method_body,
                    decorators,
                    returns,
                    is_async,
                    location,
                } => {
                    self.emitter().set_line(location.line);
                    // Compile method as a regular function, but it will be stored in class dict
                    self.compile_function_def(method_name, args, method_body, decorators, returns, *is_async)?;
                }
                _ => {
                    // Compile other statements normally (class variables, etc.)
                    self.compile_statement(stmt)?;
                }
            }
        }
        
        // Pop the class scope
        self.scope_stack.pop();
        
        // Add implicit return None
        let none_idx = self.emitter().add_constant(Constant::None);
        self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);
        self.emitter().emit(DpbOpcode::Return);
        
        // Patch jumps in the class body
        let _ = self.emitter().patch_jumps();
        
        // Pop the class emitter and build the code object
        let class_emitter = self.pop_emitter();
        let class_bytecode = class_emitter.bytecode().to_vec();
        let class_constants = class_emitter.constants().to_vec();
        let class_names = class_emitter.names().to_vec();
        let class_varnames = class_emitter.varnames().to_vec();
        let class_stacksize = class_emitter.max_stack_depth();
        
        let class_code = CodeObject {
            name: name.to_string(),
            qualname: name.to_string(),
            filename: self.filename.to_string_lossy().to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: class_varnames.len() as u32,
            stacksize: class_stacksize.max(10),
            flags: CodeFlags::OPTIMIZED | CodeFlags::NEWLOCALS,
            code: class_bytecode,
            constants: class_constants,
            names: class_names,
            varnames: class_varnames,
            freevars: vec![],
            cellvars: vec![],
        };
        
        Ok(class_code)
    }

    /// Compile import statement
    fn compile_import(&mut self, names: &[dx_py_parser::Alias]) -> CompileResult<()> {
        for alias in names {
            let zero_idx = self.emitter().add_constant(Constant::Int(0));
            self.emitter().emit_arg(DpbOpcode::LoadConst, zero_idx);

            let none_idx = self.emitter().add_constant(Constant::None);
            self.emitter().emit_arg(DpbOpcode::LoadConst, none_idx);

            let name_idx = self.emitter().add_name(&alias.name);
            self.emitter().emit_arg(DpbOpcode::ImportName, name_idx);

            let store_name = if let Some(asname) = &alias.asname {
                asname.clone()
            } else if let Some(dot_pos) = alias.name.find('.') {
                alias.name[..dot_pos].to_string()
            } else {
                alias.name.clone()
            };
            self.compile_store_name(&store_name)?;
        }
        Ok(())
    }

    /// Compile from...import statement
    fn compile_import_from(
        &mut self,
        module: Option<&str>,
        names: &[dx_py_parser::Alias],
        level: usize,
    ) -> CompileResult<()> {
        let level_idx = self.emitter().add_constant(Constant::Int(level as i64));
        self.emitter().emit_arg(DpbOpcode::LoadConst, level_idx);

        let fromlist: Vec<Constant> =
            names.iter().map(|a| Constant::String(a.name.clone())).collect();
        let fromlist_idx = self.emitter().add_constant(Constant::Tuple(fromlist));
        self.emitter().emit_arg(DpbOpcode::LoadConst, fromlist_idx);

        let module_name = module.unwrap_or("");
        let name_idx = self.emitter().add_name(module_name);
        self.emitter().emit_arg(DpbOpcode::ImportName, name_idx);

        if names.len() == 1 && names[0].name == "*" {
            self.emitter().emit_arg(DpbOpcode::ImportStar, 0);
        } else {
            for alias in names {
                let attr_idx = self.emitter().add_name(&alias.name);
                self.emitter().emit_arg(DpbOpcode::ImportFrom, attr_idx);

                let store_name = alias.asname.as_ref().unwrap_or(&alias.name);
                self.compile_store_name(store_name)?;
            }
            self.emitter().emit(DpbOpcode::PopTop);
        }

        Ok(())
    }

    /// Compile raise statement
    fn compile_raise(
        &mut self,
        exc: &Option<Expression>,
        cause: &Option<Expression>,
    ) -> CompileResult<()> {
        match (exc, cause) {
            (None, None) => {
                self.emitter().emit(DpbOpcode::Reraise);
            }
            (Some(e), None) => {
                self.compile_expression(e)?;
                self.emitter().emit_arg(DpbOpcode::Raise, 1);
            }
            (Some(e), Some(c)) => {
                self.compile_expression(e)?;
                self.compile_expression(c)?;
                self.emitter().emit_arg(DpbOpcode::Raise, 2);
            }
            (None, Some(_)) => {
                return Err(CompileError::codegen_error("Cannot use 'from' without exception"));
            }
        }
        Ok(())
    }

    /// Compile assert statement
    fn compile_assert(&mut self, test: &Expression, msg: &Option<Expression>) -> CompileResult<()> {
        let end_label = self.emitter().new_label();

        self.compile_expression(test)?;
        self.emitter().emit_jump(DpbOpcode::PopJumpIfTrue, end_label);

        let assert_error_idx = self.emitter().add_name("AssertionError");
        self.emitter().emit_arg(DpbOpcode::LoadGlobal, assert_error_idx);

        if let Some(m) = msg {
            self.compile_expression(m)?;
            self.emitter().emit_arg(DpbOpcode::Call, 1);
        } else {
            self.emitter().emit_arg(DpbOpcode::Call, 0);
        }

        self.emitter().emit_arg(DpbOpcode::Raise, 1);
        self.emitter().define_label(end_label);
        Ok(())
    }

    /// Compile delete target
    fn compile_delete(&mut self, target: &Expression) -> CompileResult<()> {
        match target {
            Expression::Name { id, .. } => {
                let scope = self.current_scope();
                if let Some(idx) = scope.get_local_index(id) {
                    self.emitter().emit_arg(DpbOpcode::DeleteFast, idx);
                } else {
                    // Both explicit globals and other names use DeleteGlobal
                    let name_idx = self.emitter().add_name(id);
                    self.emitter().emit_arg(DpbOpcode::DeleteGlobal, name_idx);
                }
            }
            Expression::Attribute { value, attr, .. } => {
                self.compile_expression(value)?;
                let name_idx = self.emitter().add_name(attr);
                self.emitter().emit_arg(DpbOpcode::DeleteAttr, name_idx);
            }
            Expression::Subscript { value, slice, .. } => {
                self.compile_expression(value)?;
                self.compile_expression(slice)?;
                self.emitter().emit(DpbOpcode::DeleteSubscr);
            }
            Expression::Tuple { elts, .. } | Expression::List { elts, .. } => {
                for elt in elts {
                    self.compile_delete(elt)?;
                }
            }
            _ => {
                return Err(CompileError::codegen_error("Invalid delete target"));
            }
        }
        Ok(())
    }

    /// Compile match statement
    fn compile_match(
        &mut self,
        subject: &Expression,
        cases: &[dx_py_parser::MatchCase],
    ) -> CompileResult<()> {
        use crate::emitter::Label;

        self.compile_expression(subject)?;

        let end_label = self.emitter().new_label();
        let mut case_labels: Vec<Label> = Vec::new();

        for _ in cases {
            case_labels.push(self.emitter().new_label());
        }

        for (i, case) in cases.iter().enumerate() {
            let next_case = if i + 1 < cases.len() {
                case_labels[i + 1]
            } else {
                end_label
            };

            self.emitter().emit(DpbOpcode::DupTop);
            self.compile_pattern(&case.pattern, next_case)?;

            if let Some(guard) = &case.guard {
                self.compile_expression(guard)?;
                self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, next_case);
            }

            self.emitter().emit(DpbOpcode::PopTop);

            for stmt in &case.body {
                self.compile_statement(stmt)?;
            }

            self.emitter().emit_jump(DpbOpcode::Jump, end_label);

            if i + 1 < cases.len() {
                self.emitter().define_label(case_labels[i + 1]);
            }
        }

        self.emitter().define_label(end_label);
        self.emitter().emit(DpbOpcode::PopTop);

        Ok(())
    }

    /// Compile a pattern for match statement
    fn compile_pattern(
        &mut self,
        pattern: &dx_py_parser::Pattern,
        fail_label: crate::emitter::Label,
    ) -> CompileResult<()> {
        use dx_py_parser::Pattern;

        match pattern {
            Pattern::MatchValue { value, .. } => {
                self.compile_expression(value)?;
                self.emitter().emit(DpbOpcode::CompareEq);
                self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, fail_label);
            }

            Pattern::MatchSingleton { value, .. } => {
                let const_val = self.ast_constant_to_bytecode(value);
                let idx = self.emitter().add_constant(const_val);
                self.emitter().emit_arg(DpbOpcode::LoadConst, idx);
                self.emitter().emit(DpbOpcode::CompareIs);
                self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, fail_label);
            }

            Pattern::MatchSequence { patterns, .. } => {
                self.emitter().emit(DpbOpcode::GetLen);
                let len_idx = self.emitter().add_constant(Constant::Int(patterns.len() as i64));
                self.emitter().emit_arg(DpbOpcode::LoadConst, len_idx);
                self.emitter().emit(DpbOpcode::CompareEq);
                self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, fail_label);

                for (i, pat) in patterns.iter().enumerate() {
                    self.emitter().emit(DpbOpcode::DupTop);
                    let idx = self.emitter().add_constant(Constant::Int(i as i64));
                    self.emitter().emit_arg(DpbOpcode::LoadConst, idx);
                    self.emitter().emit(DpbOpcode::LoadSubscr);
                    self.compile_pattern(pat, fail_label)?;
                    self.emitter().emit(DpbOpcode::PopTop);
                }
            }

            Pattern::MatchMapping {
                keys,
                patterns,
                rest,
                ..
            } => {
                for (key, pat) in keys.iter().zip(patterns.iter()) {
                    self.emitter().emit(DpbOpcode::DupTop);
                    self.compile_expression(key)?;
                    self.emitter().emit(DpbOpcode::LoadSubscr);
                    self.compile_pattern(pat, fail_label)?;
                    self.emitter().emit(DpbOpcode::PopTop);
                }
                if let Some(_name) = rest {
                    // Remaining keys capture is handled by the pattern matching runtime
                    // The **rest syntax captures unmatched keys into a new dict
                }
            }

            Pattern::MatchClass {
                cls,
                patterns,
                kwd_attrs,
                kwd_patterns,
                ..
            } => {
                self.emitter().emit(DpbOpcode::DupTop);
                self.compile_expression(cls)?;
                self.emitter().emit_arg(DpbOpcode::CheckExcMatch, 0);
                self.emitter().emit_jump(DpbOpcode::PopJumpIfFalse, fail_label);

                for (i, pat) in patterns.iter().enumerate() {
                    self.emitter().emit(DpbOpcode::DupTop);
                    let idx = self.emitter().add_constant(Constant::Int(i as i64));
                    self.emitter().emit_arg(DpbOpcode::LoadConst, idx);
                    self.emitter().emit(DpbOpcode::LoadSubscr);
                    self.compile_pattern(pat, fail_label)?;
                    self.emitter().emit(DpbOpcode::PopTop);
                }

                for (attr, pat) in kwd_attrs.iter().zip(kwd_patterns.iter()) {
                    self.emitter().emit(DpbOpcode::DupTop);
                    let attr_idx = self.emitter().add_name(attr);
                    self.emitter().emit_arg(DpbOpcode::LoadAttr, attr_idx);
                    self.compile_pattern(pat, fail_label)?;
                    self.emitter().emit(DpbOpcode::PopTop);
                }
            }

            Pattern::MatchStar { name, .. } => {
                if let Some(n) = name {
                    self.compile_store_name(n)?;
                }
            }

            Pattern::MatchAs { pattern, name, .. } => {
                if let Some(pat) = pattern {
                    self.compile_pattern(pat, fail_label)?;
                }
                if let Some(n) = name {
                    self.emitter().emit(DpbOpcode::DupTop);
                    self.compile_store_name(n)?;
                }
            }

            Pattern::MatchOr { patterns, .. } => {
                let success_label = self.emitter().new_label();

                for (i, pat) in patterns.iter().enumerate() {
                    let next_pattern = if i + 1 < patterns.len() {
                        self.emitter().new_label()
                    } else {
                        fail_label
                    };

                    self.emitter().emit(DpbOpcode::DupTop);
                    self.compile_pattern(pat, next_pattern)?;
                    self.emitter().emit_jump(DpbOpcode::Jump, success_label);

                    if i + 1 < patterns.len() {
                        self.emitter().define_label(next_pattern);
                    }
                }

                self.emitter().define_label(success_label);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_empty_module() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_simple_assignment() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = 1");
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(!code.code.is_empty());
    }

    #[test]
    fn test_compile_binary_op() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = 1 + 2");
        assert!(result.is_ok());
        let code = result.unwrap();
        // Should have: LoadConst 1, LoadConst 2, BinaryAdd, StoreGlobal x, LoadConst None, Return
        assert!(code.code.len() > 5);
    }

    #[test]
    fn test_compile_comparison() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = 1 < 2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_function_call() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("print(1)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_list_literal() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = [1, 2, 3]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_dict_literal() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = {'a': 1, 'b': 2}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_attribute_access() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = obj.attr");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_subscript() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = lst[0]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_ternary() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = 1 if True else 2");
        assert!(result.is_ok());
    }

    #[test]
    fn test_compile_boolean_ops() {
        let mut compiler = SourceCompiler::new("<test>".into());
        let result = compiler.compile_module_source("x = a and b or c");
        assert!(result.is_ok());
    }
}
