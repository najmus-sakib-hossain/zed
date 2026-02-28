//! Bytecode Emitter
//!
//! This module provides the BytecodeEmitter for generating DPB bytecode
//! with support for jump patching and line number tracking.

use dx_py_bytecode::{CodeFlags, CodeObject, Constant, DpbOpcode};
use std::collections::HashMap;

/// Label for jump targets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Label(u32);

/// Bytecode emitter with jump patching support
#[derive(Debug)]
pub struct BytecodeEmitter {
    /// Bytecode buffer
    code: Vec<u8>,
    /// Line number table: (bytecode_offset, line_number)
    line_table: Vec<(u32, u32)>,
    /// Current line number
    current_line: u32,
    /// Label targets (label -> bytecode offset)
    label_targets: HashMap<Label, u32>,
    /// Pending jumps to patch (bytecode_offset, label)
    pending_jumps: Vec<(usize, Label)>,
    /// Next label ID
    next_label: u32,
    /// Current stack depth
    stack_depth: i32,
    /// Maximum stack depth
    max_stack: i32,
    /// Constants pool
    constants: Vec<Constant>,
    /// Constant deduplication map
    const_map: HashMap<ConstantKey, u32>,
    /// Names table
    names: Vec<String>,
    /// Name deduplication map
    name_map: HashMap<String, u32>,
    /// Local variable names
    varnames: Vec<String>,
    /// Free variable names
    freevars: Vec<String>,
    /// Cell variable names
    cellvars: Vec<String>,
}

/// Key for constant deduplication
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum ConstantKey {
    None,
    Bool(bool),
    Int(i64),
    Float(u64), // Use bits for exact comparison
    String(String),
    Ellipsis,
}

impl BytecodeEmitter {
    /// Create a new bytecode emitter
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            line_table: Vec::new(),
            current_line: 1,
            label_targets: HashMap::new(),
            pending_jumps: Vec::new(),
            next_label: 0,
            stack_depth: 0,
            max_stack: 0,
            constants: Vec::new(),
            const_map: HashMap::new(),
            names: Vec::new(),
            name_map: HashMap::new(),
            varnames: Vec::new(),
            freevars: Vec::new(),
            cellvars: Vec::new(),
        }
    }

    /// Set the current line number for subsequent instructions
    pub fn set_line(&mut self, line: usize) {
        let line = line as u32;
        if line != self.current_line {
            self.line_table.push((self.code.len() as u32, line));
            self.current_line = line;
        }
    }

    /// Emit an opcode with no argument
    pub fn emit(&mut self, opcode: DpbOpcode) {
        self.code.push(opcode as u8);
        self.update_stack(opcode, 0);
    }

    /// Emit an opcode with a 1-byte argument
    pub fn emit_arg(&mut self, opcode: DpbOpcode, arg: u16) {
        let arg_size = opcode.arg_size();
        self.code.push(opcode as u8);
        match arg_size {
            0 => {}
            1 => self.code.push(arg as u8),
            2 => self.code.extend_from_slice(&arg.to_le_bytes()),
            4 => self.code.extend_from_slice(&(arg as u32).to_le_bytes()),
            _ => unreachable!(),
        }
        self.update_stack(opcode, arg as u32);
    }

    /// Create a new label
    pub fn new_label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        label
    }

    /// Define a label at the current position
    pub fn define_label(&mut self, label: Label) {
        self.label_targets.insert(label, self.code.len() as u32);
    }

    /// Emit a jump instruction to a label
    pub fn emit_jump(&mut self, opcode: DpbOpcode, label: Label) {
        self.code.push(opcode as u8);
        let fixup_pos = self.code.len();
        // Placeholder for 2-byte offset
        self.code.extend_from_slice(&[0, 0]);
        self.pending_jumps.push((fixup_pos, label));
        self.update_stack(opcode, 0);
    }

    /// Patch all pending jumps
    pub fn patch_jumps(&mut self) -> Result<(), String> {
        for (pos, label) in &self.pending_jumps {
            let target = self
                .label_targets
                .get(label)
                .ok_or_else(|| format!("Undefined label: {:?}", label))?;
            // Calculate relative offset from after the jump instruction
            let offset = (*target as i32) - (*pos as i32) - 2;
            if offset < i16::MIN as i32 || offset > i16::MAX as i32 {
                return Err(format!("Jump offset too large: {}", offset));
            }
            self.code[*pos..*pos + 2].copy_from_slice(&(offset as i16).to_le_bytes());
        }
        self.pending_jumps.clear();
        Ok(())
    }

    /// Add a constant and return its index
    pub fn add_constant(&mut self, constant: Constant) -> u16 {
        // Try to deduplicate
        let key = match &constant {
            Constant::None => Some(ConstantKey::None),
            Constant::Bool(b) => Some(ConstantKey::Bool(*b)),
            Constant::Int(i) => Some(ConstantKey::Int(*i)),
            Constant::Float(f) => Some(ConstantKey::Float(f.to_bits())),
            Constant::String(s) => Some(ConstantKey::String(s.clone())),
            Constant::Ellipsis => Some(ConstantKey::Ellipsis),
            _ => None,
        };

        if let Some(key) = key {
            if let Some(&idx) = self.const_map.get(&key) {
                return idx as u16;
            }
            let idx = self.constants.len() as u32;
            self.const_map.insert(key, idx);
            self.constants.push(constant);
            idx as u16
        } else {
            let idx = self.constants.len();
            self.constants.push(constant);
            idx as u16
        }
    }

    /// Add a name and return its index
    pub fn add_name(&mut self, name: &str) -> u16 {
        if let Some(&idx) = self.name_map.get(name) {
            return idx as u16;
        }
        let idx = self.names.len();
        self.names.push(name.to_string());
        self.name_map.insert(name.to_string(), idx as u32);
        idx as u16
    }

    /// Add a local variable and return its index
    pub fn add_local(&mut self, name: &str) -> u16 {
        if let Some(idx) = self.varnames.iter().position(|n| n == name) {
            return idx as u16;
        }
        let idx = self.varnames.len();
        self.varnames.push(name.to_string());
        idx as u16
    }

    /// Get local variable index
    pub fn get_local(&self, name: &str) -> Option<u16> {
        self.varnames.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// Set local variables from symbol table
    pub fn set_locals(&mut self, locals: Vec<String>) {
        self.varnames = locals;
    }

    /// Set free variables from symbol table
    pub fn set_freevars(&mut self, freevars: Vec<String>) {
        self.freevars = freevars;
    }

    /// Set cell variables from symbol table
    pub fn set_cellvars(&mut self, cellvars: Vec<String>) {
        self.cellvars = cellvars;
    }

    /// Get free variable index
    pub fn get_freevar(&self, name: &str) -> Option<u16> {
        self.freevars.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// Get cell variable index
    pub fn get_cellvar(&self, name: &str) -> Option<u16> {
        self.cellvars.iter().position(|n| n == name).map(|i| i as u16)
    }

    /// Update stack depth tracking
    fn update_stack(&mut self, opcode: DpbOpcode, arg: u32) {
        self.stack_depth += opcode.stack_effect(arg);
        if self.stack_depth > self.max_stack {
            self.max_stack = self.stack_depth;
        }
    }

    /// Get the current bytecode offset
    pub fn current_offset(&self) -> usize {
        self.code.len()
    }

    /// Get the maximum stack depth
    pub fn max_stack_depth(&self) -> u32 {
        self.max_stack.max(0) as u32
    }

    /// Get the bytecode
    pub fn bytecode(&self) -> &[u8] {
        &self.code
    }

    /// Get the constants
    pub fn constants(&self) -> &[Constant] {
        &self.constants
    }

    /// Get the names
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Build a CodeObject from the emitted bytecode
    #[allow(clippy::too_many_arguments)]
    pub fn build_code_object(
        &self,
        name: String,
        filename: String,
        firstlineno: u32,
        argcount: u32,
        posonlyargcount: u32,
        kwonlyargcount: u32,
        flags: CodeFlags,
    ) -> CodeObject {
        CodeObject {
            name: name.clone(),
            qualname: name,
            filename,
            firstlineno,
            argcount,
            posonlyargcount,
            kwonlyargcount,
            nlocals: self.varnames.len() as u32,
            stacksize: self.max_stack_depth(),
            flags,
            code: self.code.clone(),
            constants: self.constants.clone(),
            names: self.names.clone(),
            varnames: self.varnames.clone(),
            freevars: self.freevars.clone(),
            cellvars: self.cellvars.clone(),
        }
    }

    /// Reset the emitter for reuse
    pub fn reset(&mut self) {
        self.code.clear();
        self.line_table.clear();
        self.current_line = 1;
        self.label_targets.clear();
        self.pending_jumps.clear();
        self.next_label = 0;
        self.stack_depth = 0;
        self.max_stack = 0;
        self.constants.clear();
        self.const_map.clear();
        self.names.clear();
        self.name_map.clear();
        self.varnames.clear();
        self.freevars.clear();
        self.cellvars.clear();
    }

    /// Get the line number table
    pub fn line_table(&self) -> &[(u32, u32)] {
        &self.line_table
    }

    /// Get the current line number
    pub fn current_line(&self) -> u32 {
        self.current_line
    }

    /// Get line number for a bytecode offset
    pub fn get_line_for_offset(&self, offset: u32) -> u32 {
        // Find the last line table entry before or at this offset
        let mut line = 1;
        for &(entry_offset, entry_line) in &self.line_table {
            if entry_offset <= offset {
                line = entry_line;
            } else {
                break;
            }
        }
        line
    }

    /// Get the local variable names
    pub fn varnames(&self) -> &[String] {
        &self.varnames
    }

    /// Get the free variable names
    pub fn freevars(&self) -> &[String] {
        &self.freevars
    }

    /// Get the cell variable names
    pub fn cellvars(&self) -> &[String] {
        &self.cellvars
    }

    /// Emit a forward jump and return the label
    pub fn emit_forward_jump(&mut self, opcode: DpbOpcode) -> Label {
        let label = self.new_label();
        self.emit_jump(opcode, label);
        label
    }

    /// Adjust stack depth manually (for complex control flow)
    pub fn adjust_stack(&mut self, delta: i32) {
        self.stack_depth += delta;
        if self.stack_depth > self.max_stack {
            self.max_stack = self.stack_depth;
        }
    }

    /// Get current stack depth
    pub fn stack_depth(&self) -> i32 {
        self.stack_depth
    }

    /// Set stack depth (for control flow merging)
    pub fn set_stack_depth(&mut self, depth: i32) {
        self.stack_depth = depth;
    }
}

impl Default for BytecodeEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_simple() {
        let mut emitter = BytecodeEmitter::new();
        emitter.emit(DpbOpcode::PopTop);
        assert_eq!(emitter.code.len(), 1);
        assert_eq!(emitter.code[0], DpbOpcode::PopTop as u8);
    }

    #[test]
    fn test_emit_with_arg() {
        let mut emitter = BytecodeEmitter::new();
        emitter.emit_arg(DpbOpcode::LoadFast, 5);
        assert_eq!(emitter.code.len(), 3);
        assert_eq!(emitter.code[0], DpbOpcode::LoadFast as u8);
        assert_eq!(u16::from_le_bytes([emitter.code[1], emitter.code[2]]), 5);
    }

    #[test]
    fn test_jump_patching() {
        let mut emitter = BytecodeEmitter::new();
        let label = emitter.new_label();
        emitter.emit_jump(DpbOpcode::Jump, label);
        emitter.emit(DpbOpcode::PopTop);
        emitter.define_label(label);
        emitter.emit(DpbOpcode::Return);

        emitter.patch_jumps().unwrap();

        // Jump should point to Return instruction
        let offset = i16::from_le_bytes([emitter.code[1], emitter.code[2]]);
        assert_eq!(offset, 1); // Skip PopTop
    }

    #[test]
    fn test_constant_deduplication() {
        let mut emitter = BytecodeEmitter::new();
        let idx1 = emitter.add_constant(Constant::Int(42));
        let idx2 = emitter.add_constant(Constant::Int(100));
        let idx3 = emitter.add_constant(Constant::Int(42));

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first
        assert_eq!(emitter.constants.len(), 2);
    }

    #[test]
    fn test_name_deduplication() {
        let mut emitter = BytecodeEmitter::new();
        let idx1 = emitter.add_name("foo");
        let idx2 = emitter.add_name("bar");
        let idx3 = emitter.add_name("foo");

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0);
        assert_eq!(emitter.names.len(), 2);
    }

    #[test]
    fn test_line_number_tracking() {
        let mut emitter = BytecodeEmitter::new();

        emitter.set_line(1);
        emitter.emit(DpbOpcode::PopTop);

        emitter.set_line(2);
        emitter.emit(DpbOpcode::PopTop);
        emitter.emit(DpbOpcode::PopTop);

        emitter.set_line(5);
        emitter.emit(DpbOpcode::Return);

        // Check line table
        assert_eq!(emitter.line_table().len(), 2); // Line 1 is default, so only 2 and 5 recorded

        // Check line lookup
        assert_eq!(emitter.get_line_for_offset(0), 1);
        assert_eq!(emitter.get_line_for_offset(1), 2);
        assert_eq!(emitter.get_line_for_offset(2), 2);
        assert_eq!(emitter.get_line_for_offset(3), 5);
    }

    #[test]
    fn test_forward_jump() {
        let mut emitter = BytecodeEmitter::new();

        let label = emitter.emit_forward_jump(DpbOpcode::JumpIfFalse);
        emitter.emit(DpbOpcode::PopTop);
        emitter.define_label(label);
        emitter.emit(DpbOpcode::Return);

        emitter.patch_jumps().unwrap();

        // Verify jump target
        let offset = i16::from_le_bytes([emitter.code[1], emitter.code[2]]);
        assert_eq!(offset, 1); // Skip PopTop
    }

    #[test]
    fn test_stack_tracking() {
        let mut emitter = BytecodeEmitter::new();

        // Load two constants
        emitter.emit_arg(DpbOpcode::LoadConst, 0);
        emitter.emit_arg(DpbOpcode::LoadConst, 1);
        assert_eq!(emitter.stack_depth(), 2);

        // Binary add pops 2, pushes 1
        emitter.emit(DpbOpcode::BinaryAdd);
        assert_eq!(emitter.stack_depth(), 1);

        // Return pops 1
        emitter.emit(DpbOpcode::Return);
        assert_eq!(emitter.stack_depth(), 0);

        // Max stack was 2
        assert_eq!(emitter.max_stack_depth(), 2);
    }

    #[test]
    fn test_build_code_object() {
        let mut emitter = BytecodeEmitter::new();

        emitter.add_constant(Constant::Int(42));
        emitter.add_name("x");
        emitter.set_locals(vec!["a".to_string(), "b".to_string()]);

        emitter.emit_arg(DpbOpcode::LoadConst, 0);
        emitter.emit(DpbOpcode::Return);

        let code = emitter.build_code_object(
            "test".to_string(),
            "<test>".to_string(),
            1,
            2,
            0,
            0,
            CodeFlags::OPTIMIZED,
        );

        assert_eq!(code.name, "test");
        assert_eq!(code.filename, "<test>");
        assert_eq!(code.argcount, 2);
        assert_eq!(code.nlocals, 2);
        assert_eq!(code.constants.len(), 1);
        assert_eq!(code.names.len(), 1);
    }
}
