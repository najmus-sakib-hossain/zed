//! DPB Pretty Printer - Human-readable bytecode disassembly

use crate::format::*;
use crate::loader::DpbModule;
use std::fmt::Write;

/// DPB Pretty Printer for human-readable output
pub struct DpbPrettyPrinter;

impl DpbPrettyPrinter {
    /// Disassemble DPB to human-readable format
    pub fn disassemble(module: &DpbModule) -> String {
        let mut output = String::new();

        // Print header info
        let header = module.header();
        writeln!(output, "=== DPB Module ===").unwrap();
        writeln!(output, "Version: {}", header.version).unwrap();
        writeln!(
            output,
            "Python Version: {}.{}",
            header.python_version >> 8,
            header.python_version & 0xFF
        )
        .unwrap();
        writeln!(output, "Flags: {:?}", header.flags).unwrap();
        writeln!(output, "Code Size: {} bytes", header.code_size).unwrap();
        writeln!(output, "Constants: {}", header.constants_count).unwrap();
        writeln!(output, "Names: {}", header.names_count).unwrap();
        writeln!(output).unwrap();

        // Print constants
        writeln!(output, "=== Constants ===").unwrap();
        for i in 0..header.constants_count {
            if let Some(constant) = module.get_constant(i) {
                writeln!(output, "  {}: {}", i, Self::format_constant(&constant)).unwrap();
            }
        }
        writeln!(output).unwrap();

        // Print names
        writeln!(output, "=== Names ===").unwrap();
        for i in 0..header.names_count {
            if let Some(name) = module.get_name(i) {
                writeln!(output, "  {}: {}", i, name).unwrap();
            }
        }
        writeln!(output).unwrap();

        // Print bytecode
        writeln!(output, "=== Bytecode ===").unwrap();
        Self::disassemble_code(&mut output, module.code(), module);

        output
    }

    /// Disassemble bytecode with type annotations
    pub fn disassemble_annotated(module: &DpbModule, type_info: &TypeAnnotations) -> String {
        let mut output = String::new();

        writeln!(output, "=== Annotated Bytecode ===").unwrap();

        let code = module.code();
        let mut offset = 0;

        while offset < code.len() {
            let opcode_byte = code[offset];

            if let Some(opcode) = DpbOpcode::from_u8(opcode_byte) {
                let arg_size = opcode.arg_size();
                let arg = Self::read_arg(code, offset + 1, arg_size);

                // Print offset
                write!(output, "{:04X}  ", offset).unwrap();

                // Print opcode name
                write!(output, "{:<20}", format!("{:?}", opcode)).unwrap();

                // Print argument if present
                if arg_size > 0 {
                    write!(output, " {:>5}", arg).unwrap();

                    // Print argument interpretation
                    let interp = Self::interpret_arg(opcode, arg, module);
                    if !interp.is_empty() {
                        write!(output, "  ({})", interp).unwrap();
                    }
                }

                // Print type annotation if available
                if let Some(types) = type_info.get(offset) {
                    write!(output, "  ; types: {:?}", types).unwrap();
                }

                writeln!(output).unwrap();
                offset += 1 + arg_size;
            } else {
                writeln!(output, "{:04X}  <invalid: 0x{:02X}>", offset, opcode_byte).unwrap();
                offset += 1;
            }
        }

        output
    }

    /// Disassemble bytecode to output string
    fn disassemble_code(output: &mut String, code: &[u8], module: &DpbModule) {
        let mut offset = 0;

        while offset < code.len() {
            let opcode_byte = code[offset];

            if let Some(opcode) = DpbOpcode::from_u8(opcode_byte) {
                let arg_size = opcode.arg_size();
                let arg = Self::read_arg(code, offset + 1, arg_size);

                // Print offset
                write!(output, "{:04X}  ", offset).unwrap();

                // Print opcode name
                write!(output, "{:<20}", format!("{:?}", opcode)).unwrap();

                // Print argument if present
                if arg_size > 0 {
                    write!(output, " {:>5}", arg).unwrap();

                    // Print argument interpretation
                    let interp = Self::interpret_arg(opcode, arg, module);
                    if !interp.is_empty() {
                        write!(output, "  ({})", interp).unwrap();
                    }
                }

                writeln!(output).unwrap();
                offset += 1 + arg_size;
            } else {
                writeln!(output, "{:04X}  <invalid: 0x{:02X}>", offset, opcode_byte).unwrap();
                offset += 1;
            }
        }
    }

    /// Read an argument of the given size
    fn read_arg(code: &[u8], offset: usize, size: usize) -> u32 {
        match size {
            0 => 0,
            1 => code.get(offset).copied().unwrap_or(0) as u32,
            2 => {
                let bytes: [u8; 2] = code[offset..offset + 2].try_into().unwrap_or([0, 0]);
                u16::from_le_bytes(bytes) as u32
            }
            4 => {
                let bytes: [u8; 4] = code[offset..offset + 4].try_into().unwrap_or([0, 0, 0, 0]);
                u32::from_le_bytes(bytes)
            }
            _ => 0,
        }
    }

    /// Interpret an argument based on the opcode
    fn interpret_arg(opcode: DpbOpcode, arg: u32, module: &DpbModule) -> String {
        match opcode {
            DpbOpcode::LoadConst => {
                if let Some(constant) = module.get_constant(arg) {
                    Self::format_constant(&constant)
                } else {
                    String::new()
                }
            }
            DpbOpcode::LoadFast | DpbOpcode::StoreFast | DpbOpcode::DeleteFast => {
                format!("local[{}]", arg)
            }
            DpbOpcode::LoadGlobal
            | DpbOpcode::StoreGlobal
            | DpbOpcode::DeleteGlobal
            | DpbOpcode::LoadAttr
            | DpbOpcode::StoreAttr
            | DpbOpcode::DeleteAttr
            | DpbOpcode::LoadName
            | DpbOpcode::StoreName
            | DpbOpcode::LoadMethod
            | DpbOpcode::ImportName
            | DpbOpcode::ImportFrom => module.get_name(arg).unwrap_or("?").to_string(),
            DpbOpcode::Jump
            | DpbOpcode::JumpIfTrue
            | DpbOpcode::JumpIfFalse
            | DpbOpcode::JumpIfTrueOrPop
            | DpbOpcode::JumpIfFalseOrPop
            | DpbOpcode::PopJumpIfTrue
            | DpbOpcode::PopJumpIfFalse
            | DpbOpcode::PopJumpIfNone
            | DpbOpcode::PopJumpIfNotNone
            | DpbOpcode::ForIter => {
                format!("to {:04X}", arg)
            }
            DpbOpcode::BuildTuple | DpbOpcode::BuildList | DpbOpcode::BuildSet => {
                format!("{} items", arg)
            }
            DpbOpcode::BuildDict => {
                format!("{} pairs", arg)
            }
            DpbOpcode::Call | DpbOpcode::CallKw | DpbOpcode::CallMethod => {
                format!("{} args", arg)
            }
            _ => String::new(),
        }
    }

    /// Format a constant for display
    fn format_constant(constant: &Constant) -> String {
        match constant {
            Constant::None => "None".to_string(),
            Constant::Bool(b) => if *b { "True" } else { "False" }.to_string(),
            Constant::Int(i) => i.to_string(),
            Constant::Float(f) => format!("{:?}", f),
            Constant::Complex(r, i) => format!("({:?}+{:?}j)", r, i),
            Constant::String(s) => format!("{:?}", s),
            Constant::Bytes(b) => format!("b{:?}", String::from_utf8_lossy(b)),
            Constant::Tuple(items) => {
                let items_str: Vec<_> = items.iter().map(Self::format_constant).collect();
                format!("({})", items_str.join(", "))
            }
            Constant::FrozenSet(items) => {
                let items_str: Vec<_> = items.iter().map(Self::format_constant).collect();
                format!("frozenset({{{}}})", items_str.join(", "))
            }
            Constant::Code(_) => "<code>".to_string(),
            Constant::Ellipsis => "...".to_string(),
        }
    }
}

/// Type annotations for bytecode locations
pub struct TypeAnnotations {
    /// Map from bytecode offset to observed types
    annotations: std::collections::HashMap<usize, Vec<String>>,
}

impl TypeAnnotations {
    /// Create empty annotations
    pub fn new() -> Self {
        Self {
            annotations: std::collections::HashMap::new(),
        }
    }

    /// Add a type annotation at an offset
    pub fn add(&mut self, offset: usize, type_name: String) {
        self.annotations.entry(offset).or_default().push(type_name);
    }

    /// Get type annotations at an offset
    pub fn get(&self, offset: usize) -> Option<&Vec<String>> {
        self.annotations.get(&offset)
    }
}

impl Default for TypeAnnotations {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for DPB pretty printing
pub trait DpbPrettyPrinterTrait {
    /// Decompile DPB to human-readable format
    fn disassemble(module: &DpbModule) -> String;

    /// Print bytecode with annotations
    fn print_annotated(module: &DpbModule, annotations: &TypeAnnotations) -> String;
}

impl DpbPrettyPrinterTrait for DpbPrettyPrinter {
    fn disassemble(module: &DpbModule) -> String {
        DpbPrettyPrinter::disassemble(module)
    }

    fn print_annotated(module: &DpbModule, annotations: &TypeAnnotations) -> String {
        DpbPrettyPrinter::disassemble_annotated(module, annotations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::DpbCompiler;
    use crate::loader::DpbLoader;

    fn create_test_module() -> std::sync::Arc<DpbModule> {
        let mut compiler = DpbCompiler::new();

        let code = CodeObject {
            name: "test".to_string(),
            qualname: "test".to_string(),
            filename: "<test>".to_string(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 2,
            stacksize: 4,
            flags: CodeFlags::OPTIMIZED,
            code: vec![
                DpbOpcode::LoadConst as u8,
                0,
                0, // Load 42
                DpbOpcode::StoreFast as u8,
                0,
                0, // Store to local 0
                DpbOpcode::LoadFast as u8,
                0,
                0, // Load local 0
                DpbOpcode::LoadConst as u8,
                1,
                0,                          // Load 10
                DpbOpcode::BinaryAdd as u8, // Add
                DpbOpcode::Return as u8,    // Return
            ],
            constants: vec![Constant::Int(42), Constant::Int(10)],
            names: vec!["x".to_string()],
            varnames: vec!["a".to_string(), "b".to_string()],
            freevars: vec![],
            cellvars: vec![],
        };

        let dpb = compiler.compile(&code).unwrap();
        DpbLoader::load_from_bytes(dpb).unwrap()
    }

    #[test]
    fn test_disassemble() {
        let module = create_test_module();
        let output = DpbPrettyPrinter::disassemble(&module);

        assert!(output.contains("DPB Module"));
        assert!(output.contains("LoadConst"));
        assert!(output.contains("StoreFast"));
        assert!(output.contains("BinaryAdd"));
        assert!(output.contains("Return"));
    }

    #[test]
    fn test_format_constant() {
        assert_eq!(DpbPrettyPrinter::format_constant(&Constant::None), "None");
        assert_eq!(DpbPrettyPrinter::format_constant(&Constant::Bool(true)), "True");
        assert_eq!(DpbPrettyPrinter::format_constant(&Constant::Int(42)), "42");
        assert_eq!(
            DpbPrettyPrinter::format_constant(&Constant::String("hello".to_string())),
            "\"hello\""
        );
    }

    #[test]
    fn test_annotated_disassembly() {
        let module = create_test_module();
        let mut annotations = TypeAnnotations::new();
        annotations.add(0, "int".to_string());
        annotations.add(6, "int".to_string());

        let output = DpbPrettyPrinter::disassemble_annotated(&module, &annotations);

        assert!(output.contains("types:"));
        assert!(output.contains("int"));
    }
}
