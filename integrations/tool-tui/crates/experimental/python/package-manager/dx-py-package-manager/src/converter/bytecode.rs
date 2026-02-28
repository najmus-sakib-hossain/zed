//! Bytecode compilation for DPP packages
//!
//! Compiles Python source files to DPB (DX-Py Bytecode) format during package build.
//! This enables faster startup by avoiding runtime compilation.

use std::collections::HashMap;

/// Python version target for bytecode compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PythonVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl PythonVersion {
    /// Create a new Python version
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Parse from string like "3.12.0" or ">=3.8"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim_start_matches(">=").trim_start_matches("==");
        let parts: Vec<&str> = s.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// Convert to u32 for header storage (e.g., 3.12 = 0x030C)
    pub fn to_u32(&self) -> u32 {
        ((self.major as u32) << 8) | (self.minor as u32)
    }

    /// Create from u32 header value
    pub fn from_u32(value: u32) -> Self {
        Self {
            major: ((value >> 8) & 0xFF) as u8,
            minor: (value & 0xFF) as u8,
            patch: 0,
        }
    }
}

impl Default for PythonVersion {
    fn default() -> Self {
        Self::new(3, 12, 0)
    }
}

impl std::fmt::Display for PythonVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Compiled bytecode entry for a single Python file
#[derive(Debug, Clone)]
pub struct CompiledBytecode {
    /// Original source file path (relative to package root)
    pub source_path: String,
    /// BLAKE3 hash of the source content
    pub source_hash: [u8; 32],
    /// Target Python version
    pub python_version: PythonVersion,
    /// Compiled DPB bytecode
    pub bytecode: Vec<u8>,
    /// Compilation timestamp (Unix epoch)
    pub compiled_at: u64,
}

impl CompiledBytecode {
    /// Create a new compiled bytecode entry
    pub fn new(
        source_path: String,
        source_hash: [u8; 32],
        python_version: PythonVersion,
        bytecode: Vec<u8>,
    ) -> Self {
        Self {
            source_path,
            source_hash,
            python_version,
            bytecode,
            compiled_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Validate bytecode against source hash
    pub fn validate(&self, source_content: &[u8]) -> bool {
        let hash = blake3::hash(source_content);
        hash.as_bytes() == &self.source_hash
    }
}

/// Bytecode compiler for Python source files
pub struct BytecodeCompiler {
    /// Target Python version
    python_version: PythonVersion,
    /// Compiled bytecode cache (path -> bytecode)
    cache: HashMap<String, CompiledBytecode>,
}

impl BytecodeCompiler {
    /// Create a new bytecode compiler for the specified Python version
    pub fn new(python_version: PythonVersion) -> Self {
        Self {
            python_version,
            cache: HashMap::new(),
        }
    }

    /// Get the target Python version
    pub fn python_version(&self) -> PythonVersion {
        self.python_version
    }

    /// Compile a Python source file to bytecode
    ///
    /// This creates a simplified bytecode representation that can be
    /// loaded quickly at runtime. The actual compilation uses a
    /// lightweight parser that extracts the essential structure.
    pub fn compile(&mut self, source_path: &str, source_content: &[u8]) -> CompiledBytecode {
        let source_hash = *blake3::hash(source_content).as_bytes();

        // Check cache first
        if let Some(cached) = self.cache.get(source_path) {
            if cached.source_hash == source_hash {
                return cached.clone();
            }
        }

        // Compile the source to bytecode
        let bytecode = self.compile_source(source_path, source_content);

        let compiled = CompiledBytecode::new(
            source_path.to_string(),
            source_hash,
            self.python_version,
            bytecode,
        );

        // Cache the result
        self.cache.insert(source_path.to_string(), compiled.clone());

        compiled
    }

    /// Internal compilation logic
    fn compile_source(&self, source_path: &str, source_content: &[u8]) -> Vec<u8> {
        // Build a DPB bytecode representation
        // This is a simplified compilation that stores the source structure
        // for fast loading at runtime

        let mut output = Vec::new();

        // DPB magic and version
        output.extend_from_slice(b"DPB\x01");
        output.extend_from_slice(&1u32.to_le_bytes()); // version
        output.extend_from_slice(&self.python_version.to_u32().to_le_bytes());
        output.extend_from_slice(&0u32.to_le_bytes()); // flags

        // Source path (for debugging)
        let path_bytes = source_path.as_bytes();
        output.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(path_bytes);

        // Source hash for validation
        let source_hash = blake3::hash(source_content);
        output.extend_from_slice(source_hash.as_bytes());

        // Source content length
        output.extend_from_slice(&(source_content.len() as u64).to_le_bytes());

        // For now, we store a pre-parsed representation
        // In a full implementation, this would be actual bytecode
        // Here we store markers for functions, classes, and imports

        let source_str = String::from_utf8_lossy(source_content);
        let markers = self.extract_markers(&source_str);

        // Write marker count
        output.extend_from_slice(&(markers.len() as u32).to_le_bytes());

        // Write each marker
        for marker in &markers {
            marker.serialize(&mut output);
        }

        output
    }

    /// Extract structural markers from Python source
    fn extract_markers(&self, source: &str) -> Vec<SourceMarker> {
        let mut markers = Vec::new();

        for (idx, line) in source.lines().enumerate() {
            let line_num = (idx + 1) as u32;
            let trimmed = line.trim();

            if trimmed.starts_with("def ") {
                if let Some(name) = Self::extract_name(trimmed, "def ") {
                    markers.push(SourceMarker::Function {
                        name,
                        line: line_num,
                    });
                }
            } else if trimmed.starts_with("class ") {
                if let Some(name) = Self::extract_name(trimmed, "class ") {
                    markers.push(SourceMarker::Class {
                        name,
                        line: line_num,
                    });
                }
            } else if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                markers.push(SourceMarker::Import {
                    statement: trimmed.to_string(),
                    line: line_num,
                });
            } else if trimmed.starts_with("async def ") {
                if let Some(name) = Self::extract_name(trimmed, "async def ") {
                    markers.push(SourceMarker::AsyncFunction {
                        name,
                        line: line_num,
                    });
                }
            }
        }

        markers
    }

    /// Extract name from a definition line
    fn extract_name(line: &str, prefix: &str) -> Option<String> {
        let rest = line.strip_prefix(prefix)?;
        let name_end = rest.find(|c: char| c == '(' || c == ':' || c.is_whitespace())?;
        Some(rest[..name_end].to_string())
    }

    /// Clear the compilation cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cached bytecode for a path
    pub fn get_cached(&self, source_path: &str) -> Option<&CompiledBytecode> {
        self.cache.get(source_path)
    }
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new(PythonVersion::default())
    }
}

/// Source code structural markers
#[derive(Debug, Clone)]
pub enum SourceMarker {
    Function { name: String, line: u32 },
    AsyncFunction { name: String, line: u32 },
    Class { name: String, line: u32 },
    Import { statement: String, line: u32 },
}

impl SourceMarker {
    /// Serialize marker to bytes
    pub fn serialize(&self, output: &mut Vec<u8>) {
        match self {
            SourceMarker::Function { name, line } => {
                output.push(0x01); // Function marker
                output.extend_from_slice(&line.to_le_bytes());
                output.extend_from_slice(&(name.len() as u16).to_le_bytes());
                output.extend_from_slice(name.as_bytes());
            }
            SourceMarker::AsyncFunction { name, line } => {
                output.push(0x02); // Async function marker
                output.extend_from_slice(&line.to_le_bytes());
                output.extend_from_slice(&(name.len() as u16).to_le_bytes());
                output.extend_from_slice(name.as_bytes());
            }
            SourceMarker::Class { name, line } => {
                output.push(0x03); // Class marker
                output.extend_from_slice(&line.to_le_bytes());
                output.extend_from_slice(&(name.len() as u16).to_le_bytes());
                output.extend_from_slice(name.as_bytes());
            }
            SourceMarker::Import { statement, line } => {
                output.push(0x04); // Import marker
                output.extend_from_slice(&line.to_le_bytes());
                output.extend_from_slice(&(statement.len() as u16).to_le_bytes());
                output.extend_from_slice(statement.as_bytes());
            }
        }
    }

    /// Deserialize marker from bytes
    pub fn deserialize(data: &[u8], offset: &mut usize) -> Option<Self> {
        if *offset >= data.len() {
            return None;
        }

        let marker_type = data[*offset];
        *offset += 1;

        if *offset + 4 > data.len() {
            return None;
        }
        let line = u32::from_le_bytes(data[*offset..*offset + 4].try_into().ok()?);
        *offset += 4;

        if *offset + 2 > data.len() {
            return None;
        }
        let name_len = u16::from_le_bytes(data[*offset..*offset + 2].try_into().ok()?) as usize;
        *offset += 2;

        if *offset + name_len > data.len() {
            return None;
        }
        let name = String::from_utf8_lossy(&data[*offset..*offset + name_len]).to_string();
        *offset += name_len;

        match marker_type {
            0x01 => Some(SourceMarker::Function { name, line }),
            0x02 => Some(SourceMarker::AsyncFunction { name, line }),
            0x03 => Some(SourceMarker::Class { name, line }),
            0x04 => Some(SourceMarker::Import {
                statement: name,
                line,
            }),
            _ => None,
        }
    }
}
