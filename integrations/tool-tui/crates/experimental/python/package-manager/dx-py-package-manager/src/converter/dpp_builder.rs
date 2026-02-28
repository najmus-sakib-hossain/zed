//! DPP package builder
//!
//! Creates DPP binary packages from wheel files or raw data.

use dx_py_core::{
    headers::{dpp_flags, DppHeader, DppMetadata},
    DPP_MAGIC, PROTOCOL_VERSION,
};
use std::path::Path;

use super::bytecode::{BytecodeCompiler, PythonVersion};
use super::wheel::WheelFile;
use crate::Result;

/// File entry for DPP package
#[derive(Clone, Debug)]
pub struct DppFileEntry {
    /// Path within the package
    pub path: String,
    /// File content
    pub content: Vec<u8>,
    /// Is this a Python source file?
    pub is_python: bool,
}

/// Native extension entry for DPP package
#[derive(Clone, Debug)]
pub struct DppNativeEntry {
    /// Path within the package
    pub path: String,
    /// Extension content (compiled .so/.pyd)
    pub content: Vec<u8>,
    /// Platform tag (e.g., "manylinux_2_17_x86_64", "win_amd64")
    pub platform: String,
}

/// Bytecode entry for DPP package
#[derive(Clone, Debug)]
pub struct DppBytecodeEntry {
    /// Source file path
    pub source_path: String,
    /// Source hash for validation
    pub source_hash: [u8; 32],
    /// Target Python version
    pub python_version: PythonVersion,
    /// Compiled bytecode
    pub bytecode: Vec<u8>,
}

/// Builder for creating DPP packages
pub struct DppBuilder {
    /// Package name
    name: String,
    /// Package version
    version: String,
    /// Python version requirement
    python_requires: String,
    /// Files to include
    files: Vec<DppFileEntry>,
    /// Dependencies (as strings)
    dependencies: Vec<String>,
    /// Compression flags
    flags: u16,
    /// Compiled bytecode entries
    bytecode_entries: Vec<DppBytecodeEntry>,
    /// Native extension entries
    native_entries: Vec<DppNativeEntry>,
    /// Whether to compile bytecode during build
    compile_bytecode: bool,
    /// Target Python versions for bytecode
    target_versions: Vec<PythonVersion>,
}

impl DppBuilder {
    /// Create a new DPP builder
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            python_requires: String::new(),
            files: Vec::new(),
            dependencies: Vec::new(),
            flags: dpp_flags::NONE,
            bytecode_entries: Vec::new(),
            native_entries: Vec::new(),
            compile_bytecode: true,
            target_versions: vec![PythonVersion::default()],
        }
    }

    /// Create a DPP builder from a wheel file
    pub fn from_wheel(wheel: &WheelFile) -> Self {
        let mut builder = Self::new(&wheel.name, &wheel.version);

        if let Some(ref pr) = wheel.python_requires {
            builder.python_requires = pr.clone();
            // Parse target version from python_requires
            if let Some(version) = PythonVersion::parse(pr) {
                builder.target_versions = vec![version];
            }
        }

        builder.dependencies = wheel.dependencies.clone();

        for file in &wheel.files {
            builder.files.push(DppFileEntry {
                path: file.path.clone(),
                content: file.content.clone(),
                is_python: file.is_python,
            });
        }

        builder
    }

    /// Set Python version requirement
    pub fn python_requires(&mut self, requires: &str) -> &mut Self {
        self.python_requires = requires.to_string();
        if let Some(version) = PythonVersion::parse(requires) {
            self.target_versions = vec![version];
        }
        self
    }

    /// Add a file to the package
    pub fn add_file(&mut self, path: &str, content: Vec<u8>, is_python: bool) -> &mut Self {
        self.files.push(DppFileEntry {
            path: path.to_string(),
            content,
            is_python,
        });
        self
    }

    /// Add a native extension to the package
    pub fn add_native_extension(
        &mut self,
        path: &str,
        content: Vec<u8>,
        platform: &str,
    ) -> &mut Self {
        self.native_entries.push(DppNativeEntry {
            path: path.to_string(),
            content,
            platform: platform.to_string(),
        });
        self
    }

    /// Add pre-compiled bytecode
    pub fn add_bytecode(&mut self, entry: DppBytecodeEntry) -> &mut Self {
        self.bytecode_entries.push(entry);
        self
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep: &str) -> &mut Self {
        self.dependencies.push(dep.to_string());
        self
    }

    /// Set compression flags
    pub fn compression(&mut self, flags: u16) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Enable or disable bytecode compilation during build
    pub fn with_bytecode_compilation(&mut self, enabled: bool) -> &mut Self {
        self.compile_bytecode = enabled;
        self
    }

    /// Set target Python versions for bytecode compilation
    pub fn with_target_versions(&mut self, versions: Vec<PythonVersion>) -> &mut Self {
        self.target_versions = versions;
        self
    }

    /// Compile all Python files to bytecode
    fn compile_python_files(&self) -> Vec<DppBytecodeEntry> {
        let mut entries = Vec::new();

        for version in &self.target_versions {
            let mut compiler = BytecodeCompiler::new(*version);

            for file in &self.files {
                if file.is_python {
                    let compiled = compiler.compile(&file.path, &file.content);
                    entries.push(DppBytecodeEntry {
                        source_path: compiled.source_path,
                        source_hash: compiled.source_hash,
                        python_version: compiled.python_version,
                        bytecode: compiled.bytecode,
                    });
                }
            }
        }

        entries
    }

    /// Build the DPP package as bytes
    pub fn build(&self) -> Vec<u8> {
        let header_size = std::mem::size_of::<DppHeader>();

        // Calculate metadata section
        let name_bytes = self.name.as_bytes();
        let version_bytes = self.version.as_bytes();
        let python_requires_bytes = self.python_requires.as_bytes();

        let metadata = DppMetadata::new(
            name_bytes.len() as u16,
            version_bytes.len() as u16,
            python_requires_bytes.len() as u16,
        );

        let metadata_total = metadata.total_size();

        // Build file table (simple format: count + entries)
        let mut files_section = Vec::new();
        files_section.extend_from_slice(&(self.files.len() as u32).to_le_bytes());

        let mut file_contents = Vec::new();
        let mut file_offsets = Vec::new();
        let mut current_offset = 0u64;

        for file in &self.files {
            file_offsets.push(current_offset);
            file_contents.extend_from_slice(&file.content);
            current_offset += file.content.len() as u64;

            // Write file entry: path_len (u16) + path + offset (u64) + size (u64)
            let path_bytes = file.path.as_bytes();
            files_section.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            files_section.extend_from_slice(path_bytes);
            files_section.extend_from_slice(&file_offsets.last().unwrap().to_le_bytes());
            files_section.extend_from_slice(&(file.content.len() as u64).to_le_bytes());
        }

        // Build bytecode section with compiled bytecode
        let bytecode_entries = if self.compile_bytecode {
            self.compile_python_files()
        } else {
            self.bytecode_entries.clone()
        };

        let bytecode_section = self.serialize_bytecode_section(&bytecode_entries);

        // Build native section with platform-specific extensions
        let native_section = self.serialize_native_section();

        // Build deps section (simple format: count + strings)
        let mut deps_section = Vec::new();
        deps_section.extend_from_slice(&(self.dependencies.len() as u32).to_le_bytes());
        for dep in &self.dependencies {
            let dep_bytes = dep.as_bytes();
            deps_section.extend_from_slice(&(dep_bytes.len() as u16).to_le_bytes());
            deps_section.extend_from_slice(dep_bytes);
        }

        // Calculate offsets
        let metadata_offset = header_size as u32;
        let files_offset = metadata_offset + metadata_total as u32;
        let bytecode_offset =
            files_offset + files_section.len() as u32 + file_contents.len() as u32;
        let native_offset = bytecode_offset + bytecode_section.len() as u32;
        let deps_offset = native_offset + native_section.len() as u32;
        let total_size = deps_offset as u64 + deps_section.len() as u64;

        // Build content (everything after header)
        let mut content = Vec::new();

        // Metadata section
        content.extend_from_slice(bytemuck::bytes_of(&metadata));
        content.extend_from_slice(name_bytes);
        content.extend_from_slice(version_bytes);
        content.extend_from_slice(python_requires_bytes);

        // Files section
        content.extend_from_slice(&files_section);
        content.extend_from_slice(&file_contents);

        // Bytecode section
        content.extend_from_slice(&bytecode_section);

        // Native section
        content.extend_from_slice(&native_section);

        // Deps section
        content.extend_from_slice(&deps_section);

        // Compute BLAKE3 hash of content (truncated to 20 bytes)
        let hash = blake3::hash(&content);
        let mut blake3_hash = [0u8; 20];
        blake3_hash.copy_from_slice(&hash.as_bytes()[..20]);

        // Build header
        let header = DppHeader {
            magic: *DPP_MAGIC,
            version: PROTOCOL_VERSION,
            flags: self.flags,
            metadata_offset,
            files_offset,
            bytecode_offset,
            native_offset,
            deps_offset,
            total_size,
            uncompressed_size: total_size,
            blake3_hash,
        };

        // Combine header and content
        let mut output = Vec::with_capacity(total_size as usize);
        output.extend_from_slice(bytemuck::bytes_of(&header));
        output.extend_from_slice(&content);

        output
    }

    /// Serialize bytecode entries to binary format
    fn serialize_bytecode_section(&self, entries: &[DppBytecodeEntry]) -> Vec<u8> {
        let mut section = Vec::new();

        // Write entry count
        section.extend_from_slice(&(entries.len() as u32).to_le_bytes());

        for entry in entries {
            // Source path
            let path_bytes = entry.source_path.as_bytes();
            section.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            section.extend_from_slice(path_bytes);

            // Source hash (32 bytes)
            section.extend_from_slice(&entry.source_hash);

            // Python version (u32)
            section.extend_from_slice(&entry.python_version.to_u32().to_le_bytes());

            // Bytecode length and content
            section.extend_from_slice(&(entry.bytecode.len() as u64).to_le_bytes());
            section.extend_from_slice(&entry.bytecode);
        }

        section
    }

    /// Serialize native extension entries to binary format
    fn serialize_native_section(&self) -> Vec<u8> {
        let mut section = Vec::new();

        // Write entry count
        section.extend_from_slice(&(self.native_entries.len() as u32).to_le_bytes());

        for entry in &self.native_entries {
            // Path
            let path_bytes = entry.path.as_bytes();
            section.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            section.extend_from_slice(path_bytes);

            // Platform tag
            let platform_bytes = entry.platform.as_bytes();
            section.extend_from_slice(&(platform_bytes.len() as u16).to_le_bytes());
            section.extend_from_slice(platform_bytes);

            // Content length and data
            section.extend_from_slice(&(entry.content.len() as u64).to_le_bytes());
            section.extend_from_slice(&entry.content);
        }

        section
    }

    /// Build and write to a file
    pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let data = self.build();
        std::fs::write(path, data)?;
        Ok(())
    }
}

/// Pretty-print a DPP package for inspection
pub fn inspect_dpp(data: &[u8]) -> String {
    let mut output = String::new();

    if data.len() < std::mem::size_of::<DppHeader>() {
        return "Error: Data too small for header".to_string();
    }

    let header: &DppHeader = bytemuck::from_bytes(&data[..std::mem::size_of::<DppHeader>()]);

    // Copy packed fields to local variables to avoid unaligned reference errors
    let magic = header.magic;
    let version = header.version;
    let flags = header.flags;
    let total_size = header.total_size;
    let uncompressed_size = header.uncompressed_size;
    let metadata_offset = header.metadata_offset;
    let files_offset = header.files_offset;
    let bytecode_offset = header.bytecode_offset;
    let native_offset = header.native_offset;
    let deps_offset = header.deps_offset;

    output.push_str("=== DPP Package ===\n");
    output.push_str(&format!("Magic: {:?}\n", std::str::from_utf8(&magic[..3]).unwrap_or("???")));
    output.push_str(&format!("Version: {}\n", version));
    output.push_str(&format!("Flags: {}\n", flags));
    output.push_str(&format!("Total Size: {} bytes\n", total_size));
    output.push_str(&format!("Uncompressed Size: {} bytes\n", uncompressed_size));
    output.push_str("\n=== Section Offsets ===\n");
    output.push_str(&format!("Metadata: {}\n", metadata_offset));
    output.push_str(&format!("Files: {}\n", files_offset));
    output.push_str(&format!("Bytecode: {}\n", bytecode_offset));
    output.push_str(&format!("Native: {}\n", native_offset));
    output.push_str(&format!("Deps: {}\n", deps_offset));

    // Parse metadata
    let meta_offset = header.metadata_offset as usize;
    if meta_offset + std::mem::size_of::<DppMetadata>() <= data.len() {
        let metadata: &DppMetadata = bytemuck::from_bytes(
            &data[meta_offset..meta_offset + std::mem::size_of::<DppMetadata>()],
        );

        let name_start = meta_offset + metadata.name_offset();
        let name_end = name_start + metadata.name_len as usize;
        let version_start = meta_offset + metadata.version_offset();
        let version_end = version_start + metadata.version_len as usize;

        if name_end <= data.len() && version_end <= data.len() {
            let name = std::str::from_utf8(&data[name_start..name_end]).unwrap_or("???");
            let version = std::str::from_utf8(&data[version_start..version_end]).unwrap_or("???");

            output.push_str("\n=== Package Info ===\n");
            output.push_str(&format!("Name: {}\n", name));
            output.push_str(&format!("Version: {}\n", version));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpp_builder_basic() {
        let mut builder = DppBuilder::new("test-package", "1.0.0");
        builder.python_requires(">=3.8");
        builder.add_file("test.py", b"print('hello')".to_vec(), true);
        builder.add_dependency("requests>=2.0");

        let data = builder.build();

        // Verify header
        assert!(data.len() >= 64);
        assert_eq!(&data[0..4], DPP_MAGIC);

        // Verify we can inspect it
        let inspection = inspect_dpp(&data);
        assert!(inspection.contains("test-package"));
        assert!(inspection.contains("1.0.0"));
    }

    #[test]
    fn test_dpp_builder_roundtrip() {
        let mut builder = DppBuilder::new("mypackage", "2.1.0");
        builder.python_requires(">=3.9");
        builder.add_file("mypackage/__init__.py", b"# init".to_vec(), true);
        builder.add_file("mypackage/main.py", b"def main(): pass".to_vec(), true);
        builder.add_dependency("numpy>=1.20");
        builder.add_dependency("pandas>=1.3");

        let data = builder.build();

        // Open with DppPackage
        use crate::DppPackage;
        use std::io::Write;

        let mut temp = tempfile::NamedTempFile::new().unwrap();
        temp.write_all(&data).unwrap();
        temp.flush().unwrap();

        let package = DppPackage::open(temp.path()).unwrap();
        assert_eq!(package.name(), "mypackage");
        assert_eq!(package.version(), "2.1.0");
        assert_eq!(package.python_requires(), ">=3.9");
    }
}
