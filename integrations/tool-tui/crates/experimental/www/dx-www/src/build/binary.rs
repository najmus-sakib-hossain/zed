//! # Binary Object Builder
//!
//! Generates `.dxob` binary files from compiled components.

#![allow(dead_code)]

use crate::error::{DxError, DxResult};
use crate::parser::ComponentType;

/// DXOB binary format magic number
const DXOB_MAGIC: &[u8; 4] = b"DXOB";

/// DXOB binary version
const DXOB_VERSION: u8 = 1;

/// Section types in DXOB format
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DxobSection {
    /// Script section
    Script = 0x01,
    /// Template section
    Template = 0x02,
    /// Style section
    Style = 0x03,
    /// Metadata section
    Metadata = 0x04,
    /// Dependencies section
    Dependencies = 0x05,
    /// Exports section
    Exports = 0x06,
    /// Source map section
    SourceMap = 0x07,
}

/// DXOB file header (25 bytes)
#[derive(Debug, Clone)]
pub struct DxobHeader {
    /// Magic number (4 bytes)
    pub magic: [u8; 4],
    /// Version (1 byte)
    pub version: u8,
    /// Component type (1 byte)
    pub component_type: u8,
    /// Flags (2 bytes)
    pub flags: u16,
    /// Total file size (4 bytes)
    pub file_size: u32,
    /// Section count (1 byte)
    pub section_count: u8,
    /// Content hash (8 bytes)
    pub content_hash: [u8; 8],
    /// Reserved (4 bytes)
    pub reserved: [u8; 4],
}

impl DxobHeader {
    /// Header size in bytes
    pub const SIZE: usize = 25;

    /// Create a new header
    pub fn new(component_type: ComponentType) -> Self {
        Self {
            magic: *DXOB_MAGIC,
            version: DXOB_VERSION,
            component_type: match component_type {
                ComponentType::Page => 0x01,
                ComponentType::Component => 0x02,
                ComponentType::Layout => 0x03,
            },
            flags: 0,
            file_size: 0,
            section_count: 0,
            content_hash: [0u8; 8],
            reserved: [0u8; 4],
        }
    }

    /// Write header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.extend_from_slice(&self.magic);
        bytes.push(self.version);
        bytes.push(self.component_type);
        bytes.extend_from_slice(&self.flags.to_le_bytes());
        bytes.extend_from_slice(&self.file_size.to_le_bytes());
        bytes.push(self.section_count);
        bytes.extend_from_slice(&self.content_hash);
        bytes.extend_from_slice(&self.reserved);
        bytes
    }

    /// Read header from bytes
    pub fn from_bytes(bytes: &[u8]) -> DxResult<Self> {
        if bytes.len() < Self::SIZE {
            return Err(DxError::BinaryFormatError {
                message: format!("Header too small: {} bytes", bytes.len()),
            });
        }

        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if &magic != DXOB_MAGIC {
            return Err(DxError::BinaryFormatError {
                message: "Invalid magic number".to_string(),
            });
        }

        Ok(Self {
            magic,
            version: bytes[4],
            component_type: bytes[5],
            flags: u16::from_le_bytes([bytes[6], bytes[7]]),
            file_size: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
            section_count: bytes[12],
            content_hash: bytes[13..21].try_into().unwrap(),
            reserved: bytes[21..25].try_into().unwrap(),
        })
    }
}

/// Section header (9 bytes)
#[derive(Debug, Clone)]
pub struct SectionHeader {
    /// Section type
    pub section_type: DxobSection,
    /// Section offset from start of file
    pub offset: u32,
    /// Section size
    pub size: u32,
}

impl SectionHeader {
    /// Section header size
    pub const SIZE: usize = 9;

    /// Write section header to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);
        bytes.push(self.section_type as u8);
        bytes.extend_from_slice(&self.offset.to_le_bytes());
        bytes.extend_from_slice(&self.size.to_le_bytes());
        bytes
    }

    /// Read section header from bytes
    pub fn from_bytes(bytes: &[u8]) -> DxResult<Self> {
        if bytes.len() < Self::SIZE {
            return Err(DxError::BinaryFormatError {
                message: "Section header too small".to_string(),
            });
        }

        let section_type = match bytes[0] {
            0x01 => DxobSection::Script,
            0x02 => DxobSection::Template,
            0x03 => DxobSection::Style,
            0x04 => DxobSection::Metadata,
            0x05 => DxobSection::Dependencies,
            0x06 => DxobSection::Exports,
            0x07 => DxobSection::SourceMap,
            _ => {
                return Err(DxError::BinaryFormatError {
                    message: format!("Unknown section type: {}", bytes[0]),
                });
            }
        };

        Ok(Self {
            section_type,
            offset: u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
            size: u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]),
        })
    }
}

/// Builder for DXOB binary objects.
pub struct BinaryObjectBuilder {
    component_type: ComponentType,
    script: Option<Vec<u8>>,
    template: Option<Vec<u8>>,
    style: Option<Vec<u8>>,
    metadata: Option<Vec<u8>>,
    dependencies: Vec<String>,
    exports: Vec<(String, u8)>,
    source_map: Option<String>,
    flags: u16,
}

impl BinaryObjectBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            component_type: ComponentType::Component,
            script: None,
            template: None,
            style: None,
            metadata: None,
            dependencies: Vec::new(),
            exports: Vec::new(),
            source_map: None,
            flags: 0,
        }
    }

    /// Set the component type.
    pub fn component_type(mut self, ct: ComponentType) -> Self {
        self.component_type = ct;
        self
    }

    /// Set the compiled script bytes.
    pub fn script(mut self, bytes: Vec<u8>) -> Self {
        if !bytes.is_empty() {
            self.script = Some(bytes);
        }
        self
    }

    /// Set the compiled template bytes.
    pub fn template(mut self, bytes: Vec<u8>) -> Self {
        if !bytes.is_empty() {
            self.template = Some(bytes);
        }
        self
    }

    /// Set the compiled style bytes.
    pub fn style(mut self, bytes: Vec<u8>) -> Self {
        if !bytes.is_empty() {
            self.style = Some(bytes);
        }
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, bytes: Vec<u8>) -> Self {
        if !bytes.is_empty() {
            self.metadata = Some(bytes);
        }
        self
    }

    /// Add a dependency.
    pub fn dependency(mut self, dep: impl Into<String>) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    /// Add dependencies.
    pub fn dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies.extend(deps);
        self
    }

    /// Add an export.
    pub fn export(mut self, name: impl Into<String>, export_type: u8) -> Self {
        self.exports.push((name.into(), export_type));
        self
    }

    /// Set source map.
    pub fn source_map(mut self, sm: String) -> Self {
        if !sm.is_empty() {
            self.source_map = Some(sm);
        }
        self
    }

    /// Set flags.
    pub fn flags(mut self, flags: u16) -> Self {
        self.flags = flags;
        self
    }

    /// Build the binary object.
    pub fn build(self) -> DxResult<Vec<u8>> {
        let mut output = Vec::new();
        let mut sections: Vec<(DxobSection, Vec<u8>)> = Vec::new();

        // Collect sections
        if let Some(script) = self.script {
            sections.push((DxobSection::Script, script));
        }

        if let Some(template) = self.template {
            sections.push((DxobSection::Template, template));
        }

        if let Some(style) = self.style {
            sections.push((DxobSection::Style, style));
        }

        if let Some(metadata) = self.metadata {
            sections.push((DxobSection::Metadata, metadata));
        }

        // Dependencies section
        if !self.dependencies.is_empty() {
            let mut deps_bytes = Vec::new();
            deps_bytes.extend_from_slice(&(self.dependencies.len() as u16).to_le_bytes());
            for dep in &self.dependencies {
                let bytes = dep.as_bytes();
                deps_bytes.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
                deps_bytes.extend_from_slice(bytes);
            }
            sections.push((DxobSection::Dependencies, deps_bytes));
        }

        // Exports section
        if !self.exports.is_empty() {
            let mut exports_bytes = Vec::new();
            exports_bytes.extend_from_slice(&(self.exports.len() as u16).to_le_bytes());
            for (name, export_type) in &self.exports {
                exports_bytes.push(*export_type);
                let bytes = name.as_bytes();
                exports_bytes.push(bytes.len() as u8);
                exports_bytes.extend_from_slice(bytes);
            }
            sections.push((DxobSection::Exports, exports_bytes));
        }

        // Source map section
        if let Some(sm) = self.source_map {
            sections.push((DxobSection::SourceMap, sm.into_bytes()));
        }

        // Calculate offsets
        let header_size = DxobHeader::SIZE;
        let section_table_size = sections.len() * SectionHeader::SIZE;
        let mut current_offset = (header_size + section_table_size) as u32;

        let mut section_headers = Vec::new();
        for (section_type, data) in &sections {
            section_headers.push(SectionHeader {
                section_type: *section_type,
                offset: current_offset,
                size: data.len() as u32,
            });
            current_offset += data.len() as u32;
        }

        // Calculate content hash
        let mut hasher = blake3::Hasher::new();
        for (_, data) in &sections {
            hasher.update(data);
        }
        let hash = hasher.finalize();
        let content_hash: [u8; 8] = hash.as_bytes()[0..8].try_into().unwrap();

        // Create header
        let mut header = DxobHeader::new(self.component_type);
        header.flags = self.flags;
        header.file_size = current_offset;
        header.section_count = sections.len() as u8;
        header.content_hash = content_hash;

        // Write header
        output.extend_from_slice(&header.to_bytes());

        // Write section table
        for sh in &section_headers {
            output.extend_from_slice(&sh.to_bytes());
        }

        // Write section data
        for (_, data) in sections {
            output.extend_from_slice(&data);
        }

        Ok(output)
    }
}

impl Default for BinaryObjectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Reader for DXOB binary files.
pub struct DxobReader<'a> {
    data: &'a [u8],
    header: DxobHeader,
    section_headers: Vec<SectionHeader>,
}

impl<'a> DxobReader<'a> {
    /// Create a new reader.
    pub fn new(data: &'a [u8]) -> DxResult<Self> {
        let header = DxobHeader::from_bytes(data)?;

        let mut section_headers = Vec::new();
        let mut offset = DxobHeader::SIZE;

        for _ in 0..header.section_count {
            let sh = SectionHeader::from_bytes(&data[offset..])?;
            section_headers.push(sh);
            offset += SectionHeader::SIZE;
        }

        Ok(Self {
            data,
            header,
            section_headers,
        })
    }

    /// Get the header.
    pub fn header(&self) -> &DxobHeader {
        &self.header
    }

    /// Get a section by type.
    pub fn section(&self, section_type: DxobSection) -> Option<&[u8]> {
        for sh in &self.section_headers {
            if sh.section_type == section_type {
                let start = sh.offset as usize;
                let end = start + sh.size as usize;
                if end <= self.data.len() {
                    return Some(&self.data[start..end]);
                }
            }
        }
        None
    }

    /// Get the script section.
    pub fn script(&self) -> Option<&[u8]> {
        self.section(DxobSection::Script)
    }

    /// Get the template section.
    pub fn template(&self) -> Option<&[u8]> {
        self.section(DxobSection::Template)
    }

    /// Get the style section.
    pub fn style(&self) -> Option<&[u8]> {
        self.section(DxobSection::Style)
    }

    /// Get component type.
    pub fn component_type(&self) -> ComponentType {
        match self.header.component_type {
            0x01 => ComponentType::Page,
            0x02 => ComponentType::Component,
            0x03 => ComponentType::Layout,
            _ => ComponentType::Component,
        }
    }

    /// Get dependencies.
    pub fn dependencies(&self) -> Vec<String> {
        let mut deps = Vec::new();

        if let Some(data) = self.section(DxobSection::Dependencies) {
            if data.len() >= 2 {
                let count = u16::from_le_bytes([data[0], data[1]]) as usize;
                let mut offset = 2;

                for _ in 0..count {
                    if offset + 2 > data.len() {
                        break;
                    }
                    let len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;

                    if offset + len > data.len() {
                        break;
                    }
                    if let Ok(s) = std::str::from_utf8(&data[offset..offset + len]) {
                        deps.push(s.to_string());
                    }
                    offset += len;
                }
            }
        }

        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_empty_object() {
        let result = BinaryObjectBuilder::new().component_type(ComponentType::Page).build();

        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..4], DXOB_MAGIC);
    }

    #[test]
    fn test_build_with_script() {
        let script = b"test script".to_vec();

        let result = BinaryObjectBuilder::new()
            .component_type(ComponentType::Component)
            .script(script.clone())
            .build();

        assert!(result.is_ok());

        let bytes = result.unwrap();
        let reader = DxobReader::new(&bytes).unwrap();

        assert_eq!(reader.script(), Some(script.as_slice()));
    }

    #[test]
    fn test_build_with_all_sections() {
        let script = b"script".to_vec();
        let template = b"template".to_vec();
        let style = b"style".to_vec();

        let result = BinaryObjectBuilder::new()
            .component_type(ComponentType::Page)
            .script(script.clone())
            .template(template.clone())
            .style(style.clone())
            .dependency("dep1")
            .dependency("dep2")
            .build();

        assert!(result.is_ok());

        let bytes = result.unwrap();
        let reader = DxobReader::new(&bytes).unwrap();

        assert_eq!(reader.script(), Some(script.as_slice()));
        assert_eq!(reader.template(), Some(template.as_slice()));
        assert_eq!(reader.style(), Some(style.as_slice()));

        let deps = reader.dependencies();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0], "dep1");
        assert_eq!(deps[1], "dep2");
    }

    #[test]
    fn test_header_roundtrip() {
        let header = DxobHeader::new(ComponentType::Layout);
        let bytes = header.to_bytes();
        let parsed = DxobHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.magic, parsed.magic);
        assert_eq!(header.version, parsed.version);
        assert_eq!(header.component_type, parsed.component_type);
    }
}
