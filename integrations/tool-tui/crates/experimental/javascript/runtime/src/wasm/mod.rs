//! WebAssembly interop layer

pub mod loader;

pub use loader::WasmLoader;

use crate::error::{DxError, DxResult};
use std::collections::HashMap;

pub struct WasmModule {
    /// Module name - reserved for module identification
    #[allow(dead_code)]
    name: String,
    exports: HashMap<String, WasmExport>,
    memory: Option<WasmMemory>,
}

pub enum WasmExport {
    Function {
        params: Vec<WasmType>,
        results: Vec<WasmType>,
    },
    Memory {
        initial: u32,
        maximum: Option<u32>,
    },
    Global {
        value_type: WasmType,
        mutable: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum WasmType {
    I32,
    I64,
    F32,
    F64,
}

pub struct WasmMemory {
    data: Vec<u8>,
    pages: u32,
}

impl WasmMemory {
    pub fn new(initial_pages: u32) -> Self {
        Self {
            data: vec![0; (initial_pages as usize) * 65536],
            pages: initial_pages,
        }
    }

    pub fn read(&self, offset: usize, len: usize) -> DxResult<&[u8]> {
        self.data
            .get(offset..offset + len)
            .ok_or_else(|| DxError::RuntimeError("Memory access out of bounds".to_string()))
    }

    pub fn write(&mut self, offset: usize, data: &[u8]) -> DxResult<()> {
        let len = data.len();
        if offset + len > self.data.len() {
            return Err(DxError::RuntimeError("Memory write out of bounds".to_string()));
        }
        self.data[offset..offset + len].copy_from_slice(data);
        Ok(())
    }

    pub fn grow(&mut self, delta: u32) -> DxResult<u32> {
        let old_pages = self.pages;
        self.pages += delta;
        self.data.resize((self.pages as usize) * 65536, 0);
        Ok(old_pages)
    }
}

impl WasmModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            exports: HashMap::new(),
            memory: None,
        }
    }

    pub fn add_export(&mut self, name: String, export: WasmExport) {
        self.exports.insert(name, export);
    }

    pub fn set_memory(&mut self, memory: WasmMemory) {
        self.memory = Some(memory);
    }

    pub fn get_export(&self, name: &str) -> Option<&WasmExport> {
        self.exports.get(name)
    }
}
