//! DPB Compiler - Compiles Python AST to DPB binary format

use crate::format::*;
use std::collections::HashMap;

/// DPB Compiler - transforms Python AST to DPB binary format
pub struct DpbCompiler {
    /// Bytecode buffer
    code: Vec<u8>,
    /// Constants pool
    constants: Vec<Constant>,
    /// Name table (interned strings)
    names: Vec<String>,
    /// Name to index mapping
    name_map: HashMap<String, u32>,
    /// Constant to index mapping (for deduplication)
    const_map: HashMap<ConstantKey, u32>,
    /// Local variable names
    varnames: Vec<String>,
    /// Free variable names
    freevars: Vec<String>,
    /// Cell variable names
    cellvars: Vec<String>,
    /// Current stack depth
    stack_depth: i32,
    /// Maximum stack depth
    max_stack: i32,
    /// Label targets for jumps
    labels: HashMap<u32, u32>,
    /// Pending jump fixups
    jump_fixups: Vec<(usize, u32)>,
    /// Next label ID
    next_label: u32,
}

/// Key for constant deduplication
#[derive(Hash, PartialEq, Eq)]
enum ConstantKey {
    None,
    Bool(bool),
    Int(i64),
    Float(u64), // Use bits for exact comparison
    String(String),
}

impl DpbCompiler {
    /// Create a new compiler
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
            name_map: HashMap::new(),
            const_map: HashMap::new(),
            varnames: Vec::new(),
            freevars: Vec::new(),
            cellvars: Vec::new(),
            stack_depth: 0,
            max_stack: 0,
            labels: HashMap::new(),
            jump_fixups: Vec::new(),
            next_label: 0,
        }
    }

    /// Compile a code object to DPB format
    pub fn compile(&mut self, code: &CodeObject) -> Result<Vec<u8>, DpbError> {
        // Reset state
        self.code.clear();
        self.constants.clear();
        self.names.clear();
        self.name_map.clear();
        self.const_map.clear();
        self.stack_depth = 0;
        self.max_stack = 0;
        self.labels.clear();
        self.jump_fixups.clear();
        self.next_label = 0;

        // Copy metadata
        self.varnames = code.varnames.clone();
        self.freevars = code.freevars.clone();
        self.cellvars = code.cellvars.clone();

        // Pre-populate names
        for name in &code.names {
            self.intern_name(name);
        }

        // Pre-populate constants
        for constant in &code.constants {
            self.add_constant(constant.clone());
        }

        // Copy bytecode (in real impl, would transform from Python bytecode)
        self.code = code.code.clone();

        // Build the DPB binary
        self.build_dpb(code)
    }

    /// Build the final DPB binary
    fn build_dpb(&self, code: &CodeObject) -> Result<Vec<u8>, DpbError> {
        let mut output = Vec::new();

        // Calculate section offsets
        let header_size = DpbHeader::size();
        let code_offset = header_size as u32;
        let code_size = self.code.len() as u32;
        let constants_offset = code_offset + code_size;
        let constants_data = self.serialize_constants();
        let names_offset = constants_offset + constants_data.len() as u32;
        let names_data = self.serialize_names();
        let symbols_offset = names_offset + names_data.len() as u32;
        let types_offset = symbols_offset; // No symbols for now
        let debug_offset = types_offset; // No types for now

        // Build header
        let mut header = DpbHeader::new();
        header.code_offset = code_offset;
        header.code_size = code_size;
        header.constants_offset = constants_offset;
        header.constants_count = self.constants.len() as u32;
        header.names_offset = names_offset;
        header.names_count = self.names.len() as u32;
        header.symbols_offset = symbols_offset;
        header.types_offset = types_offset;
        header.debug_offset = debug_offset;

        if code.flags.contains(CodeFlags::GENERATOR) {
            header.flags |= DpbFlags::HAS_GENERATORS;
        }
        if code.flags.contains(CodeFlags::COROUTINE)
            || code.flags.contains(CodeFlags::ASYNC_GENERATOR)
        {
            header.flags |= DpbFlags::HAS_ASYNC;
        }

        // Write header (with zero hash initially)
        output.extend_from_slice(unsafe {
            std::slice::from_raw_parts(&header as *const DpbHeader as *const u8, header_size)
        });

        // Write code section
        output.extend_from_slice(&self.code);

        // Write constants section
        output.extend_from_slice(&constants_data);

        // Write names section
        output.extend_from_slice(&names_data);

        // Calculate and update content hash
        // The hash is computed over everything after the header
        let hash = blake3::hash(&output[header_size..]);

        // The content_hash field is at the end of the header struct
        // We need to find its offset within the header
        // DpbHeader layout: magic(4) + version(4) + python_version(4) + flags(4) +
        //                   code_offset(4) + constants_offset(4) + names_offset(4) +
        //                   symbols_offset(4) + types_offset(4) + debug_offset(4) +
        //                   code_size(4) + constants_count(4) + names_count(4) +
        //                   content_hash(32) + padding to 128 bytes
        // Total before hash: 4+4+4+4+4+4+4+4+4+4+4+4+4 = 52 bytes
        // Hash is at offset 52, size 32, then padding to 128
        let hash_offset = 52; // Offset of content_hash in the header
        output[hash_offset..hash_offset + 32].copy_from_slice(hash.as_bytes());

        Ok(output)
    }

    /// Serialize constants to binary format
    fn serialize_constants(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Write count
        data.extend_from_slice(&(self.constants.len() as u32).to_le_bytes());

        // Write each constant
        for constant in &self.constants {
            Self::serialize_constant(&mut data, constant);
        }

        data
    }

    /// Serialize a single constant
    fn serialize_constant(data: &mut Vec<u8>, constant: &Constant) {
        match constant {
            Constant::None => {
                data.push(ConstantType::None as u8);
            }
            Constant::Bool(b) => {
                data.push(ConstantType::Bool as u8);
                data.push(if *b { 1 } else { 0 });
            }
            Constant::Int(i) => {
                data.push(ConstantType::Int as u8);
                data.extend_from_slice(&i.to_le_bytes());
            }
            Constant::Float(f) => {
                data.push(ConstantType::Float as u8);
                data.extend_from_slice(&f.to_le_bytes());
            }
            Constant::Complex(r, i) => {
                data.push(ConstantType::Complex as u8);
                data.extend_from_slice(&r.to_le_bytes());
                data.extend_from_slice(&i.to_le_bytes());
            }
            Constant::String(s) => {
                data.push(ConstantType::String as u8);
                data.extend_from_slice(&(s.len() as u32).to_le_bytes());
                data.extend_from_slice(s.as_bytes());
            }
            Constant::Bytes(b) => {
                data.push(ConstantType::Bytes as u8);
                data.extend_from_slice(&(b.len() as u32).to_le_bytes());
                data.extend_from_slice(b);
            }
            Constant::Tuple(items) => {
                data.push(ConstantType::Tuple as u8);
                data.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    Self::serialize_constant(data, item);
                }
            }
            Constant::FrozenSet(items) => {
                data.push(ConstantType::FrozenSet as u8);
                data.extend_from_slice(&(items.len() as u32).to_le_bytes());
                for item in items {
                    Self::serialize_constant(data, item);
                }
            }
            Constant::Code(_) => {
                // Nested code objects are handled separately
                data.push(ConstantType::Code as u8);
                // Nested code object serialization is not yet implemented
            }
            Constant::Ellipsis => {
                data.push(ConstantType::Ellipsis as u8);
            }
        }
    }

    /// Serialize names to binary format
    fn serialize_names(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Write count
        data.extend_from_slice(&(self.names.len() as u32).to_le_bytes());

        // Write each name
        for name in &self.names {
            data.extend_from_slice(&(name.len() as u32).to_le_bytes());
            data.extend_from_slice(name.as_bytes());
        }

        data
    }

    /// Intern a name and return its index
    pub fn intern_name(&mut self, name: &str) -> u32 {
        if let Some(&idx) = self.name_map.get(name) {
            return idx;
        }

        let idx = self.names.len() as u32;
        self.names.push(name.to_string());
        self.name_map.insert(name.to_string(), idx);
        idx
    }

    /// Add a constant and return its index
    pub fn add_constant(&mut self, constant: Constant) -> u32 {
        // Try to deduplicate
        let key = match &constant {
            Constant::None => Some(ConstantKey::None),
            Constant::Bool(b) => Some(ConstantKey::Bool(*b)),
            Constant::Int(i) => Some(ConstantKey::Int(*i)),
            Constant::Float(f) => Some(ConstantKey::Float(f.to_bits())),
            Constant::String(s) => Some(ConstantKey::String(s.clone())),
            _ => None,
        };

        if let Some(key) = key {
            if let Some(&idx) = self.const_map.get(&key) {
                return idx;
            }
            let idx = self.constants.len() as u32;
            self.const_map.insert(key, idx);
            self.constants.push(constant);
            idx
        } else {
            let idx = self.constants.len() as u32;
            self.constants.push(constant);
            idx
        }
    }

    /// Emit an opcode with no argument
    pub fn emit(&mut self, opcode: DpbOpcode) {
        self.code.push(opcode as u8);
        self.update_stack(opcode, 0);
    }

    /// Emit an opcode with a 1-byte argument
    pub fn emit_arg1(&mut self, opcode: DpbOpcode, arg: u8) {
        self.code.push(opcode as u8);
        self.code.push(arg);
        self.update_stack(opcode, arg as u32);
    }

    /// Emit an opcode with a 2-byte argument
    pub fn emit_arg2(&mut self, opcode: DpbOpcode, arg: u16) {
        self.code.push(opcode as u8);
        self.code.extend_from_slice(&arg.to_le_bytes());
        self.update_stack(opcode, arg as u32);
    }

    /// Emit an opcode with a 4-byte argument
    pub fn emit_arg4(&mut self, opcode: DpbOpcode, arg: u32) {
        self.code.push(opcode as u8);
        self.code.extend_from_slice(&arg.to_le_bytes());
        self.update_stack(opcode, arg);
    }

    /// Create a new label
    pub fn new_label(&mut self) -> u32 {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    /// Mark the current position as a label target
    pub fn mark_label(&mut self, label: u32) {
        self.labels.insert(label, self.code.len() as u32);
    }

    /// Emit a jump to a label (to be fixed up later)
    pub fn emit_jump(&mut self, opcode: DpbOpcode, label: u32) {
        self.code.push(opcode as u8);
        let fixup_pos = self.code.len();
        self.code.extend_from_slice(&[0, 0]); // Placeholder
        self.jump_fixups.push((fixup_pos, label));
        self.update_stack(opcode, 0);
    }

    /// Fix up all jump targets
    pub fn fixup_jumps(&mut self) -> Result<(), DpbError> {
        for (pos, label) in &self.jump_fixups {
            let target = self
                .labels
                .get(label)
                .ok_or_else(|| DpbError::CompileError(format!("Unknown label: {}", label)))?;
            let offset = (*target as i32 - *pos as i32 - 2) as i16;
            self.code[*pos..*pos + 2].copy_from_slice(&offset.to_le_bytes());
        }
        Ok(())
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
    pub fn max_stack_depth(&self) -> i32 {
        self.max_stack
    }
}

impl Default for DpbCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_new() {
        let compiler = DpbCompiler::new();
        assert!(compiler.code.is_empty());
        assert!(compiler.constants.is_empty());
        assert!(compiler.names.is_empty());
    }

    #[test]
    fn test_intern_name() {
        let mut compiler = DpbCompiler::new();
        let idx1 = compiler.intern_name("foo");
        let idx2 = compiler.intern_name("bar");
        let idx3 = compiler.intern_name("foo"); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first
        assert_eq!(compiler.names.len(), 2);
    }

    #[test]
    fn test_add_constant() {
        let mut compiler = DpbCompiler::new();
        let idx1 = compiler.add_constant(Constant::Int(42));
        let idx2 = compiler.add_constant(Constant::Int(100));
        let idx3 = compiler.add_constant(Constant::Int(42)); // Duplicate

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(idx3, 0); // Same as first
        assert_eq!(compiler.constants.len(), 2);
    }

    #[test]
    fn test_emit_opcodes() {
        let mut compiler = DpbCompiler::new();
        compiler.emit(DpbOpcode::PopTop);
        compiler.emit_arg2(DpbOpcode::LoadFast, 0);
        compiler.emit_arg2(DpbOpcode::LoadConst, 1);
        compiler.emit(DpbOpcode::BinaryAdd);
        compiler.emit(DpbOpcode::Return);

        assert_eq!(compiler.code.len(), 1 + 3 + 3 + 1 + 1);
    }

    #[test]
    fn test_compile_simple() {
        let mut compiler = DpbCompiler::new();

        let code = CodeObject {
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
            constants: vec![Constant::None],
            names: vec![],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        let result = compiler.compile(&code);
        assert!(result.is_ok());

        let dpb = result.unwrap();
        assert!(dpb.len() >= DpbHeader::size());
        assert_eq!(&dpb[0..4], b"DPB\x01");
    }
}
