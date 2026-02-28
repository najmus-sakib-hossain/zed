//! DPB Loader - Memory-mapped zero-copy bytecode loading

use crate::format::*;
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// A loaded DPB module with zero-copy access
pub struct DpbModule {
    /// Memory-mapped file data
    mmap: Mmap,
    /// Cached header pointer (points into mmap)
    header: *const DpbHeader,
}

// Safety: DpbModule is Send + Sync because:
// - Mmap is Send + Sync
// - header is a read-only pointer into mmap
unsafe impl Send for DpbModule {}
unsafe impl Sync for DpbModule {}

impl DpbModule {
    /// Get the header
    #[inline]
    pub fn header(&self) -> &DpbHeader {
        unsafe { &*self.header }
    }

    /// Get the bytecode slice (zero-copy)
    #[inline]
    pub fn code(&self) -> &[u8] {
        let header = self.header();
        let start = header.code_offset as usize;
        let end = start + header.code_size as usize;
        &self.mmap[start..end]
    }

    /// Get a constant by index
    pub fn get_constant(&self, index: u32) -> Option<Constant> {
        let header = self.header();
        if index >= header.constants_count {
            return None;
        }

        let constants_data = &self.mmap[header.constants_offset as usize..];
        self.parse_constant_at(constants_data, index)
    }

    /// Get a name by index
    pub fn get_name(&self, index: u32) -> Option<&str> {
        let header = self.header();
        if index >= header.names_count {
            return None;
        }

        let names_data = &self.mmap[header.names_offset as usize..];
        self.parse_name_at(names_data, index)
    }

    /// Get a pre-resolved symbol by index
    pub fn get_symbol(&self, index: u32) -> Option<u64> {
        let header = self.header();
        let symbols_offset = header.symbols_offset as usize;
        let types_offset = header.types_offset as usize;

        if symbols_offset >= types_offset {
            return None; // No symbols section
        }

        let symbol_size = 8; // u64
        let offset = symbols_offset + (index as usize * symbol_size);

        if offset + symbol_size > types_offset {
            return None;
        }

        let bytes: [u8; 8] = self.mmap[offset..offset + 8].try_into().ok()?;
        Some(u64::from_le_bytes(bytes))
    }

    /// Verify the content hash
    pub fn verify_hash(&self) -> bool {
        let header = self.header();
        let content_start = DpbHeader::size();
        let content = &self.mmap[content_start..];
        let computed_hash = blake3::hash(content);
        computed_hash.as_bytes() == &header.content_hash
    }

    /// Parse a constant at the given index
    fn parse_constant_at(&self, data: &[u8], index: u32) -> Option<Constant> {
        // Skip count (4 bytes)
        let mut offset = 4;

        // Skip to the requested index
        for _ in 0..index {
            offset = Self::skip_constant(data, offset)?;
        }

        Self::parse_constant(data, offset).map(|(c, _)| c)
    }

    /// Skip over a constant and return the new offset
    fn skip_constant(data: &[u8], offset: usize) -> Option<usize> {
        if offset >= data.len() {
            return None;
        }

        let type_byte = data[offset];
        let const_type = match type_byte {
            0 => ConstantType::None,
            1 => ConstantType::Bool,
            2 => ConstantType::Int,
            3 => ConstantType::Float,
            4 => ConstantType::Complex,
            5 => ConstantType::String,
            6 => ConstantType::Bytes,
            7 => ConstantType::Tuple,
            8 => ConstantType::FrozenSet,
            9 => ConstantType::Code,
            10 => ConstantType::Ellipsis,
            _ => return None,
        };

        match const_type {
            ConstantType::None | ConstantType::Ellipsis => Some(offset + 1),
            ConstantType::Bool => Some(offset + 2),
            ConstantType::Int | ConstantType::Float => Some(offset + 9),
            ConstantType::Complex => Some(offset + 17),
            ConstantType::String | ConstantType::Bytes => {
                let len =
                    u32::from_le_bytes(data[offset + 1..offset + 5].try_into().ok()?) as usize;
                Some(offset + 5 + len)
            }
            ConstantType::Tuple | ConstantType::FrozenSet => {
                let count =
                    u32::from_le_bytes(data[offset + 1..offset + 5].try_into().ok()?) as usize;
                let mut pos = offset + 5;
                for _ in 0..count {
                    pos = Self::skip_constant(data, pos)?;
                }
                Some(pos)
            }
            ConstantType::Code => {
                // Nested code objects are not yet fully supported
                Some(offset + 1)
            }
        }
    }

    /// Parse a constant at the given offset
    fn parse_constant(data: &[u8], offset: usize) -> Option<(Constant, usize)> {
        if offset >= data.len() {
            return None;
        }

        let type_byte = data[offset];

        match type_byte {
            0 => Some((Constant::None, offset + 1)),
            1 => {
                let value = data.get(offset + 1)? != &0;
                Some((Constant::Bool(value), offset + 2))
            }
            2 => {
                let bytes: [u8; 8] = data[offset + 1..offset + 9].try_into().ok()?;
                Some((Constant::Int(i64::from_le_bytes(bytes)), offset + 9))
            }
            3 => {
                let bytes: [u8; 8] = data[offset + 1..offset + 9].try_into().ok()?;
                Some((Constant::Float(f64::from_le_bytes(bytes)), offset + 9))
            }
            4 => {
                let real_bytes: [u8; 8] = data[offset + 1..offset + 9].try_into().ok()?;
                let imag_bytes: [u8; 8] = data[offset + 9..offset + 17].try_into().ok()?;
                Some((
                    Constant::Complex(
                        f64::from_le_bytes(real_bytes),
                        f64::from_le_bytes(imag_bytes),
                    ),
                    offset + 17,
                ))
            }
            5 => {
                let len =
                    u32::from_le_bytes(data[offset + 1..offset + 5].try_into().ok()?) as usize;
                let s = std::str::from_utf8(&data[offset + 5..offset + 5 + len]).ok()?;
                Some((Constant::String(s.to_string()), offset + 5 + len))
            }
            6 => {
                let len =
                    u32::from_le_bytes(data[offset + 1..offset + 5].try_into().ok()?) as usize;
                let bytes = data[offset + 5..offset + 5 + len].to_vec();
                Some((Constant::Bytes(bytes), offset + 5 + len))
            }
            7 => {
                let count =
                    u32::from_le_bytes(data[offset + 1..offset + 5].try_into().ok()?) as usize;
                let mut items = Vec::with_capacity(count);
                let mut pos = offset + 5;
                for _ in 0..count {
                    let (item, new_pos) = Self::parse_constant(data, pos)?;
                    items.push(item);
                    pos = new_pos;
                }
                Some((Constant::Tuple(items), pos))
            }
            8 => {
                let count =
                    u32::from_le_bytes(data[offset + 1..offset + 5].try_into().ok()?) as usize;
                let mut items = Vec::with_capacity(count);
                let mut pos = offset + 5;
                for _ in 0..count {
                    let (item, new_pos) = Self::parse_constant(data, pos)?;
                    items.push(item);
                    pos = new_pos;
                }
                Some((Constant::FrozenSet(items), pos))
            }
            9 => {
                // Nested code objects are not yet fully supported
                Some((Constant::None, offset + 1))
            }
            10 => Some((Constant::Ellipsis, offset + 1)),
            _ => None,
        }
    }

    /// Parse a name at the given index
    fn parse_name_at<'a>(&'a self, data: &'a [u8], index: u32) -> Option<&'a str> {
        // Skip count (4 bytes)
        let mut offset = 4;

        // Skip to the requested index
        for _ in 0..index {
            let len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
            offset += 4 + len;
        }

        let len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
        std::str::from_utf8(&data[offset + 4..offset + 4 + len]).ok()
    }
}

/// DPB Loader - loads DPB files with memory mapping
pub struct DpbLoader;

impl DpbLoader {
    /// Load a DPB file from disk with memory mapping (zero-copy)
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Arc<DpbModule>, DpbError> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Validate minimum size
        if mmap.len() < DpbHeader::size() {
            return Err(DpbError::InvalidOffset);
        }

        // Get header pointer
        let header = mmap.as_ptr() as *const DpbHeader;

        // Validate header
        let header_ref = unsafe { &*header };
        header_ref.validate()?;

        let module = DpbModule { mmap, header };

        // Optionally verify hash
        if !module.verify_hash() {
            return Err(DpbError::HashMismatch);
        }

        Ok(Arc::new(module))
    }

    /// Load a DPB module from bytes (for testing)
    pub fn load_from_bytes(data: Vec<u8>) -> Result<Arc<DpbModule>, DpbError> {
        // Validate minimum size
        if data.len() < DpbHeader::size() {
            return Err(DpbError::InvalidOffset);
        }

        // Create a memory map from the bytes
        // Note: In production, we'd use a proper memory-mapped approach
        let mmap = unsafe {
            // Create a temporary file and map it
            let mut temp = tempfile::tempfile()?;
            std::io::Write::write_all(&mut temp, &data)?;
            Mmap::map(&temp)?
        };

        let header = mmap.as_ptr() as *const DpbHeader;
        let header_ref = unsafe { &*header };
        header_ref.validate()?;

        Ok(Arc::new(DpbModule { mmap, header }))
    }
}

/// Trait for DPB loading operations
pub trait DpbLoaderTrait {
    /// Memory-map DPB file for zero-copy access
    fn load(path: &Path) -> Result<Arc<DpbModule>, DpbError>;

    /// Get bytecode slice without copying
    fn get_code(module: &DpbModule) -> &[u8];

    /// Get constant by index (O(1) for simple types)
    fn get_constant(module: &DpbModule, index: u32) -> Option<Constant>;

    /// Get pre-resolved symbol
    fn get_symbol(module: &DpbModule, index: u32) -> Option<u64>;
}

impl DpbLoaderTrait for DpbLoader {
    fn load(path: &Path) -> Result<Arc<DpbModule>, DpbError> {
        DpbLoader::load(path)
    }

    fn get_code(module: &DpbModule) -> &[u8] {
        module.code()
    }

    fn get_constant(module: &DpbModule, index: u32) -> Option<Constant> {
        module.get_constant(index)
    }

    fn get_symbol(module: &DpbModule, index: u32) -> Option<u64> {
        module.get_symbol(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::DpbCompiler;

    fn create_test_dpb() -> Vec<u8> {
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
            constants: vec![Constant::Int(42), Constant::String("hello".to_string())],
            names: vec!["foo".to_string(), "bar".to_string()],
            varnames: vec![],
            freevars: vec![],
            cellvars: vec![],
        };

        compiler.compile(&code).unwrap()
    }

    #[test]
    fn test_load_from_bytes() {
        let dpb = create_test_dpb();
        let module = DpbLoader::load_from_bytes(dpb).unwrap();

        assert!(module.header().validate_magic());
        assert!(!module.code().is_empty());
    }

    #[test]
    fn test_get_constant() {
        let dpb = create_test_dpb();
        let module = DpbLoader::load_from_bytes(dpb).unwrap();

        let const0 = module.get_constant(0);
        assert!(matches!(const0, Some(Constant::Int(42))));

        let const1 = module.get_constant(1);
        assert!(matches!(const1, Some(Constant::String(s)) if s == "hello"));
    }

    #[test]
    fn test_get_name() {
        let dpb = create_test_dpb();
        let module = DpbLoader::load_from_bytes(dpb).unwrap();

        assert_eq!(module.get_name(0), Some("foo"));
        assert_eq!(module.get_name(1), Some("bar"));
        assert_eq!(module.get_name(2), None);
    }
}
