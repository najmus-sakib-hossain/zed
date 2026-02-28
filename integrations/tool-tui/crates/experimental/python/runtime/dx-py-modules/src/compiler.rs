//! DPM Compiler - Compiles Python modules to DPM format

use std::collections::HashMap;

use crate::export_table::ExportTable;
use crate::format::{
    DpmError, DpmFlags, DpmHeader, ExportEntry, ExportKind, ImportEntry, ImportFlags,
};

/// Represents a Python module to be compiled
#[derive(Debug, Default)]
pub struct ModuleDefinition {
    /// Module name
    pub name: String,
    /// Is this a package?
    pub is_package: bool,
    /// Imports
    pub imports: Vec<ImportDef>,
    /// Exports (functions, classes, constants, variables)
    pub exports: Vec<ExportDef>,
    /// Module initialization bytecode
    pub init_bytecode: Vec<u8>,
    /// Type annotations
    pub type_annotations: Vec<u8>,
}

/// Import definition
#[derive(Debug, Clone)]
pub struct ImportDef {
    pub module_name: String,
    pub symbol_name: Option<String>,
    pub alias: Option<String>,
    pub is_star: bool,
    pub level: u8,
}

/// Export definition
#[derive(Debug, Clone)]
pub struct ExportDef {
    pub name: String,
    pub kind: ExportKind,
    pub data: Vec<u8>,
}

/// DPM Compiler
pub struct DpmCompiler {
    /// String table for deduplication
    strings: HashMap<String, u32>,
    /// Current string table offset
    string_offset: u32,
    /// String data buffer
    string_data: Vec<u8>,
}

impl DpmCompiler {
    pub fn new() -> Self {
        Self {
            strings: HashMap::new(),
            string_offset: 0,
            string_data: Vec::new(),
        }
    }

    /// Compile a module definition to DPM binary format
    pub fn compile(&mut self, module: &ModuleDefinition) -> Result<Vec<u8>, DpmError> {
        self.strings.clear();
        self.string_offset = 0;
        self.string_data.clear();

        // Build export table first to get the seed
        let export_defs: Vec<_> = module
            .exports
            .iter()
            .enumerate()
            .map(|(i, e)| (e.name.clone(), e.kind, i as u32))
            .collect();
        let export_table = ExportTable::build(&export_defs)?;

        // Calculate section sizes and offsets
        let header_size = DpmHeader::size();

        // Imports section
        let imports_offset = header_size;
        let imports_size = module.imports.len() * std::mem::size_of::<ImportEntry>();

        // Exports section
        let exports_offset = imports_offset + imports_size;
        let exports_size = module.exports.len() * std::mem::size_of::<ExportEntry>();

        // Functions/data section (export data)
        let functions_offset = exports_offset + exports_size;
        let mut functions_data = Vec::new();
        let mut export_offsets = Vec::new();
        for export in &module.exports {
            export_offsets.push(functions_offset + functions_data.len());
            // Write size + data
            functions_data.extend_from_slice(&(export.data.len() as u32).to_le_bytes());
            functions_data.extend_from_slice(&export.data);
        }
        let functions_size = functions_data.len();

        // Init bytecode section
        let init_offset = functions_offset + functions_size;
        let init_size = if module.init_bytecode.is_empty() {
            0
        } else {
            4 + module.init_bytecode.len()
        };

        // Type annotations section
        let types_offset = init_offset + init_size;
        let types_size = module.type_annotations.len();

        // String table section (at the end)
        let strings_offset = types_offset + types_size;

        // Intern all strings
        for import in &module.imports {
            self.intern_string(&import.module_name);
            if let Some(ref sym) = import.symbol_name {
                self.intern_string(sym);
            }
        }
        for export in &module.exports {
            self.intern_string(&export.name);
        }

        // Build the binary
        let total_size = strings_offset + self.string_data.len();
        let mut output = Vec::with_capacity(total_size);

        // Write header placeholder (will update hash later)
        let mut header = DpmHeader::new();
        header.flags = if module.is_package {
            DpmFlags::IS_PACKAGE
        } else {
            DpmFlags::empty()
        };
        if !module.type_annotations.is_empty() {
            header.flags |= DpmFlags::HAS_TYPES;
        }
        header.imports_offset = imports_offset as u32;
        header.exports_offset = exports_offset as u32;
        header.functions_offset = functions_offset as u32;
        header.classes_offset = 0; // Not used separately
        header.constants_offset = 0; // Not used separately
        header.type_annotations_offset = if types_size > 0 {
            types_offset as u32
        } else {
            0
        };
        header.init_bytecode_offset = if init_size > 0 { init_offset as u32 } else { 0 };
        header.imports_count = module.imports.len() as u32;
        header.exports_count = module.exports.len() as u32;
        header.export_hash_seed = export_table.seed();

        // Write header bytes manually to ensure correct layout
        // Fields before content_hash: 56 bytes total
        output.extend_from_slice(&header.magic); // 4 bytes
        output.extend_from_slice(&header.version.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.flags.bits().to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header._reserved.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.imports_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.exports_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.functions_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.classes_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.constants_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.type_annotations_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.init_bytecode_offset.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.imports_count.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.exports_count.to_le_bytes()); // 4 bytes
        output.extend_from_slice(&header.export_hash_seed.to_le_bytes()); // 4 bytes
                                                                          // Total so far: 56 bytes
                                                                          // Content hash placeholder (32 bytes)
        let hash_offset = output.len();
        output.extend_from_slice(&[0u8; 32]);
        // Total so far: 88 bytes
        // Pad to header size (struct is 64-byte aligned, so size is 128 bytes)
        while output.len() < header_size {
            output.push(0);
        }

        // Write imports
        for import in &module.imports {
            let module_offset = self.get_string_offset(&import.module_name, strings_offset);
            let symbol_offset = import
                .symbol_name
                .as_ref()
                .map(|s| self.get_string_offset(s, strings_offset))
                .unwrap_or(0);

            let mut flags = ImportFlags::empty();
            if import.symbol_name.is_some() {
                flags |= ImportFlags::FROM_IMPORT;
            }
            if import.is_star {
                flags |= ImportFlags::STAR_IMPORT;
            }
            if import.level > 0 {
                flags |= ImportFlags::RELATIVE;
            }
            if import.alias.is_some() {
                flags |= ImportFlags::ALIASED;
            }

            output.extend_from_slice(&module_offset.to_le_bytes());
            output.extend_from_slice(&symbol_offset.to_le_bytes());
            output.push(flags.bits());
            output.push(import.level);
            output.extend_from_slice(&[0u8; 2]); // reserved
        }

        // Write exports
        for (i, export) in module.exports.iter().enumerate() {
            let name_offset = self.get_string_offset(&export.name, strings_offset);
            let name_hash = self.compute_name_hash(&export.name);

            output.extend_from_slice(&name_hash.to_le_bytes()); // 8 bytes
            output.extend_from_slice(&name_offset.to_le_bytes()); // 4 bytes
            output.push(export.kind as u8); // 1 byte
            output.extend_from_slice(&[0u8; 3]); // 3 bytes (reserved)
            output.extend_from_slice(&(export_offsets[i] as u32).to_le_bytes()); // 4 bytes
            output.extend_from_slice(&[0u8; 4]); // 4 bytes (alignment padding)
                                                 // Total: 24 bytes per ExportEntry
        }

        // Write functions/data
        output.extend_from_slice(&functions_data);

        // Write init bytecode
        if !module.init_bytecode.is_empty() {
            output.extend_from_slice(&(module.init_bytecode.len() as u32).to_le_bytes());
            output.extend_from_slice(&module.init_bytecode);
        }

        // Write type annotations
        output.extend_from_slice(&module.type_annotations);

        // Write string table
        output.extend_from_slice(&self.string_data);

        // Compute and update content hash (hash is at offset 56, covering content after header)
        let content_hash = blake3::hash(&output[header_size..]);
        output[hash_offset..hash_offset + 32].copy_from_slice(content_hash.as_bytes());

        Ok(output)
    }

    /// Intern a string and return its offset
    fn intern_string(&mut self, s: &str) -> u32 {
        if let Some(&offset) = self.strings.get(s) {
            return offset;
        }

        let offset = self.string_offset;
        self.strings.insert(s.to_string(), offset);
        self.string_data.extend_from_slice(s.as_bytes());
        self.string_data.push(0); // null terminator
        self.string_offset += s.len() as u32 + 1;
        offset
    }

    /// Get the absolute offset of a string
    fn get_string_offset(&self, s: &str, base: usize) -> u32 {
        base as u32 + self.strings.get(s).copied().unwrap_or(0)
    }

    /// Compute FNV-1a hash for a name
    fn compute_name_hash(&self, name: &str) -> u64 {
        let mut hash: u64 = 14695981039346656037;
        for byte in name.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(1099511628211);
        }
        hash
    }

    /// Extract imports from Python source (simplified)
    pub fn extract_imports(source: &str) -> Vec<ImportDef> {
        let mut imports = Vec::new();

        for line in source.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("import ") {
                let module = rest.split_whitespace().next().unwrap_or("");
                if !module.is_empty() {
                    imports.push(ImportDef {
                        module_name: module.to_string(),
                        symbol_name: None,
                        alias: None,
                        is_star: false,
                        level: 0,
                    });
                }
            } else if line.starts_with("from ") {
                // Simple parsing: from X import Y
                if let Some(rest) = line.strip_prefix("from ") {
                    let parts: Vec<&str> = rest.split(" import ").collect();
                    if parts.len() == 2 {
                        let module = parts[0].trim();
                        let symbols = parts[1].trim();

                        let level = module.chars().take_while(|&c| c == '.').count() as u8;
                        let module_name = module.trim_start_matches('.');

                        if symbols == "*" {
                            imports.push(ImportDef {
                                module_name: module_name.to_string(),
                                symbol_name: None,
                                alias: None,
                                is_star: true,
                                level,
                            });
                        } else {
                            for sym in symbols.split(',') {
                                let sym = sym.trim();
                                imports.push(ImportDef {
                                    module_name: module_name.to_string(),
                                    symbol_name: Some(sym.to_string()),
                                    alias: None,
                                    is_star: false,
                                    level,
                                });
                            }
                        }
                    }
                }
            }
        }

        imports
    }

    /// Extract exports from Python source (simplified - looks for def/class at module level)
    pub fn extract_exports(source: &str) -> Vec<ExportDef> {
        let mut exports = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            // Function definition at module level (no leading whitespace)
            if line.starts_with("def ") {
                if let Some(name) = trimmed.strip_prefix("def ") {
                    let name = name.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() && !name.starts_with('_') {
                        exports.push(ExportDef {
                            name: name.to_string(),
                            kind: ExportKind::Function,
                            data: Vec::new(),
                        });
                    }
                }
            }
            // Class definition at module level
            else if line.starts_with("class ") {
                if let Some(name) = trimmed.strip_prefix("class ") {
                    let name = name.split(['(', ':']).next().unwrap_or("").trim();
                    if !name.is_empty() && !name.starts_with('_') {
                        exports.push(ExportDef {
                            name: name.to_string(),
                            kind: ExportKind::Class,
                            data: Vec::new(),
                        });
                    }
                }
            }
            // Module-level constant (UPPER_CASE = ...)
            else if !line.starts_with(' ') && !line.starts_with('\t') {
                if let Some(eq_pos) = line.find('=') {
                    let name = line[..eq_pos].trim();
                    if !name.is_empty()
                        && name.chars().all(|c| c.is_uppercase() || c == '_' || c.is_numeric())
                        && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                    {
                        exports.push(ExportDef {
                            name: name.to_string(),
                            kind: ExportKind::Constant,
                            data: Vec::new(),
                        });
                    }
                }
            }
        }

        exports
    }
}

impl Default for DpmCompiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::DPM_MAGIC;

    #[test]
    fn test_extract_imports() {
        let source = r#"
import os
import sys
from collections import defaultdict
from . import utils
from ..base import Base
from typing import *
"#;

        let imports = DpmCompiler::extract_imports(source);
        assert_eq!(imports.len(), 6);
        assert_eq!(imports[0].module_name, "os");
        assert_eq!(imports[2].module_name, "collections");
        assert_eq!(imports[2].symbol_name, Some("defaultdict".to_string()));
        assert_eq!(imports[3].level, 1);
        assert_eq!(imports[4].level, 2);
        assert!(imports[5].is_star);
    }

    #[test]
    fn test_extract_exports() {
        let source = r#"
MAX_SIZE = 100
VERSION = "1.0"

def public_function():
    pass

def _private_function():
    pass

class MyClass:
    pass

class _PrivateClass:
    pass
"#;

        let exports = DpmCompiler::extract_exports(source);
        assert_eq!(exports.len(), 4); // MAX_SIZE, VERSION, public_function, MyClass
    }

    #[test]
    fn test_compile_module() {
        let module = ModuleDefinition {
            name: "test_module".to_string(),
            is_package: false,
            imports: vec![ImportDef {
                module_name: "os".to_string(),
                symbol_name: None,
                alias: None,
                is_star: false,
                level: 0,
            }],
            exports: vec![ExportDef {
                name: "foo".to_string(),
                kind: ExportKind::Function,
                data: vec![1, 2, 3],
            }],
            init_bytecode: vec![0xF0], // NOP
            type_annotations: Vec::new(),
        };

        let mut compiler = DpmCompiler::new();
        let binary = compiler.compile(&module).unwrap();

        // Verify magic bytes
        assert_eq!(&binary[0..4], DPM_MAGIC);
    }
}
