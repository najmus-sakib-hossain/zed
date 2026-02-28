//! pyproject.toml parsing and binary conversion
//!
//! Provides parsing of pyproject.toml files and conversion to/from
//! the binary pyproject.dx format for faster loading.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Magic bytes for binary pyproject.dx format
pub const PYPROJECT_DX_MAGIC: &[u8; 4] = b"DXPY";

/// pyproject.toml structure
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct PyProjectToml {
    /// [project] section
    #[serde(default)]
    pub project: Option<ProjectSection>,
    /// [tool] section
    #[serde(default)]
    pub tool: Option<ToolSection>,
    /// [build-system] section
    #[serde(default, rename = "build-system")]
    pub build_system: Option<BuildSystem>,
}

impl PyProjectToml {
    /// Load from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse from TOML string
    pub fn parse(content: &str) -> Result<Self> {
        toml::from_str(content)
            .map_err(|e| Error::Cache(format!("Failed to parse pyproject.toml: {}", e)))
    }

    /// Save to a TOML file
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = self.to_toml()?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to TOML string
    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self)
            .map_err(|e| Error::Cache(format!("Failed to serialize pyproject.toml: {}", e)))
    }

    /// Get the project name
    pub fn name(&self) -> Option<&str> {
        self.project.as_ref().map(|p| p.name.as_str())
    }

    /// Get the project version
    pub fn version(&self) -> Option<&str> {
        self.project.as_ref().and_then(|p| p.version.as_deref())
    }

    /// Get dependencies
    pub fn dependencies(&self) -> &[String] {
        self.project.as_ref().and_then(|p| p.dependencies.as_deref()).unwrap_or(&[])
    }

    /// Get optional dependencies for a group
    pub fn optional_dependencies(&self, group: &str) -> &[String] {
        self.project
            .as_ref()
            .and_then(|p| p.optional_dependencies.as_ref())
            .and_then(|od| od.get(group))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// [project] section of pyproject.toml
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct ProjectSection {
    /// Package name
    pub name: String,
    /// Package version
    #[serde(default)]
    pub version: Option<String>,
    /// Package description
    #[serde(default)]
    pub description: Option<String>,
    /// Readme file path
    #[serde(default)]
    pub readme: Option<String>,
    /// Required Python version
    #[serde(default, rename = "requires-python")]
    pub requires_python: Option<String>,
    /// License
    #[serde(default)]
    pub license: Option<LicenseField>,
    /// Authors
    #[serde(default)]
    pub authors: Option<Vec<Author>>,
    /// Maintainers
    #[serde(default)]
    pub maintainers: Option<Vec<Author>>,
    /// Keywords
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    /// Classifiers
    #[serde(default)]
    pub classifiers: Option<Vec<String>>,
    /// Project URLs
    #[serde(default)]
    pub urls: Option<HashMap<String, String>>,
    /// Dependencies
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
    /// Optional dependencies
    #[serde(default, rename = "optional-dependencies")]
    pub optional_dependencies: Option<HashMap<String, Vec<String>>>,
    /// Entry points / scripts
    #[serde(default)]
    pub scripts: Option<HashMap<String, String>>,
    /// GUI scripts
    #[serde(default, rename = "gui-scripts")]
    pub gui_scripts: Option<HashMap<String, String>>,
    /// Entry points
    #[serde(default, rename = "entry-points")]
    pub entry_points: Option<HashMap<String, HashMap<String, String>>>,
}

/// License field (can be text or table)
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum LicenseField {
    /// Simple text license
    Text(String),
    /// Table with file or text
    Table {
        file: Option<String>,
        text: Option<String>,
    },
}

/// Author/maintainer information
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct Author {
    /// Name
    #[serde(default)]
    pub name: Option<String>,
    /// Email
    #[serde(default)]
    pub email: Option<String>,
}

/// [tool] section of pyproject.toml
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct ToolSection {
    /// [tool.dx-py] section
    #[serde(default, rename = "dx-py")]
    pub dx_py: Option<DxPyConfig>,
    /// Other tool configurations (preserved as-is)
    #[serde(flatten)]
    pub other: HashMap<String, toml::Value>,
}

/// [tool.dx-py] configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct DxPyConfig {
    /// Workspace configuration
    #[serde(default)]
    pub workspace: Option<WorkspaceConfig>,
    /// Sources configuration
    #[serde(default)]
    pub sources: Option<Vec<SourceConfig>>,
}

/// Workspace configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct WorkspaceConfig {
    /// Workspace members
    #[serde(default)]
    pub members: Vec<String>,
    /// Excluded paths
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Shared dependencies
    #[serde(default)]
    pub shared_dependencies: HashMap<String, String>,
}

/// Package source configuration
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SourceConfig {
    /// Source name
    pub name: String,
    /// Source URL
    pub url: String,
    /// Whether this is the default source
    #[serde(default)]
    pub default: bool,
}

/// [build-system] section
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
pub struct BuildSystem {
    /// Build requirements
    #[serde(default)]
    pub requires: Vec<String>,
    /// Build backend
    #[serde(default, rename = "build-backend")]
    pub build_backend: Option<String>,
    /// Backend path
    #[serde(default, rename = "backend-path")]
    pub backend_path: Option<Vec<String>>,
}

/// Binary header for pyproject.dx format
#[repr(C, packed)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PyProjectDxHeader {
    /// Magic bytes "DXPY"
    pub magic: [u8; 4],
    /// Format version
    pub version: u16,
    /// Flags (reserved)
    pub flags: u16,
    /// Total size of the binary data
    pub total_size: u32,
    /// Offset to project section
    pub project_offset: u32,
    /// Size of project section
    pub project_size: u32,
    /// Offset to tool section
    pub tool_offset: u32,
    /// Size of tool section
    pub tool_size: u32,
    /// Offset to build-system section
    pub build_system_offset: u32,
    /// Size of build-system section
    pub build_system_size: u32,
    /// Reserved for future use
    pub _reserved: [u8; 16],
}

const HEADER_SIZE: usize = std::mem::size_of::<PyProjectDxHeader>();

/// Convert pyproject.toml to binary pyproject.dx format
pub fn convert_to_binary(toml: &PyProjectToml) -> Result<Vec<u8>> {
    let mut output = Vec::new();

    // Serialize sections to JSON (compact binary-friendly format)
    let project_bytes = if let Some(ref project) = toml.project {
        serde_json::to_vec(project)
            .map_err(|e| Error::Cache(format!("Failed to serialize project: {}", e)))?
    } else {
        Vec::new()
    };

    let tool_bytes = if let Some(ref tool) = toml.tool {
        serde_json::to_vec(tool)
            .map_err(|e| Error::Cache(format!("Failed to serialize tool: {}", e)))?
    } else {
        Vec::new()
    };

    let build_system_bytes = if let Some(ref build_system) = toml.build_system {
        serde_json::to_vec(build_system)
            .map_err(|e| Error::Cache(format!("Failed to serialize build-system: {}", e)))?
    } else {
        Vec::new()
    };

    // Calculate offsets
    let project_offset = HEADER_SIZE as u32;
    let tool_offset = project_offset + project_bytes.len() as u32;
    let build_system_offset = tool_offset + tool_bytes.len() as u32;
    let total_size = build_system_offset + build_system_bytes.len() as u32;

    // Create header
    let header = PyProjectDxHeader {
        magic: *PYPROJECT_DX_MAGIC,
        version: 1,
        flags: 0,
        total_size,
        project_offset,
        project_size: project_bytes.len() as u32,
        tool_offset,
        tool_size: tool_bytes.len() as u32,
        build_system_offset,
        build_system_size: build_system_bytes.len() as u32,
        _reserved: [0u8; 16],
    };

    // Write header
    output.extend_from_slice(bytemuck::bytes_of(&header));

    // Write sections
    output.extend_from_slice(&project_bytes);
    output.extend_from_slice(&tool_bytes);
    output.extend_from_slice(&build_system_bytes);

    Ok(output)
}

/// Convert binary pyproject.dx back to PyProjectToml
pub fn convert_from_binary(binary: &[u8]) -> Result<PyProjectToml> {
    if binary.len() < HEADER_SIZE {
        return Err(Error::Cache("Binary data too small".to_string()));
    }

    // Read header
    let header: &PyProjectDxHeader = bytemuck::from_bytes(&binary[..HEADER_SIZE]);

    // Verify magic
    if &header.magic != PYPROJECT_DX_MAGIC {
        return Err(Error::InvalidMagic {
            expected: *PYPROJECT_DX_MAGIC,
            found: header.magic,
        });
    }

    // Copy header fields to avoid unaligned access
    let project_offset = header.project_offset as usize;
    let project_size = header.project_size as usize;
    let tool_offset = header.tool_offset as usize;
    let tool_size = header.tool_size as usize;
    let build_system_offset = header.build_system_offset as usize;
    let build_system_size = header.build_system_size as usize;

    // Parse sections
    let project = if project_size > 0 {
        let project_bytes = &binary[project_offset..project_offset + project_size];
        Some(
            serde_json::from_slice(project_bytes)
                .map_err(|e| Error::Cache(format!("Failed to parse project: {}", e)))?,
        )
    } else {
        None
    };

    let tool = if tool_size > 0 {
        let tool_bytes = &binary[tool_offset..tool_offset + tool_size];
        Some(
            serde_json::from_slice(tool_bytes)
                .map_err(|e| Error::Cache(format!("Failed to parse tool: {}", e)))?,
        )
    } else {
        None
    };

    let build_system = if build_system_size > 0 {
        let build_system_bytes =
            &binary[build_system_offset..build_system_offset + build_system_size];
        Some(
            serde_json::from_slice(build_system_bytes)
                .map_err(|e| Error::Cache(format!("Failed to parse build-system: {}", e)))?,
        )
    } else {
        None
    };

    Ok(PyProjectToml {
        project,
        tool,
        build_system,
    })
}

/// Load pyproject from either .toml or .dx format
pub fn load_pyproject(dir: &Path) -> Result<PyProjectToml> {
    let dx_path = dir.join("pyproject.dx");
    let toml_path = dir.join("pyproject.toml");

    if dx_path.exists() {
        let binary = std::fs::read(&dx_path)?;
        convert_from_binary(&binary)
    } else if toml_path.exists() {
        PyProjectToml::load(&toml_path)
    } else {
        Err(Error::Cache(format!(
            "No pyproject.toml or pyproject.dx found in {}",
            dir.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_pyproject() {
        let toml = r#"
[project]
name = "test-package"
version = "1.0.0"
"#;
        let parsed = PyProjectToml::parse(toml).unwrap();
        assert_eq!(parsed.name(), Some("test-package"));
        assert_eq!(parsed.version(), Some("1.0.0"));
    }

    #[test]
    fn test_parse_with_dependencies() {
        let toml = r#"
[project]
name = "test-package"
version = "1.0.0"
dependencies = ["requests>=2.0", "flask"]

[project.optional-dependencies]
dev = ["pytest", "black"]
"#;
        let parsed = PyProjectToml::parse(toml).unwrap();
        assert_eq!(parsed.dependencies().len(), 2);
        assert_eq!(parsed.optional_dependencies("dev").len(), 2);
    }

    #[test]
    fn test_parse_with_build_system() {
        let toml = r#"
[project]
name = "test-package"

[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"
"#;
        let parsed = PyProjectToml::parse(toml).unwrap();
        let build_system = parsed.build_system.as_ref().unwrap();
        assert_eq!(build_system.requires, vec!["setuptools>=61.0"]);
        assert_eq!(build_system.build_backend, Some("setuptools.build_meta".to_string()));
    }

    #[test]
    fn test_binary_roundtrip() {
        let toml = r#"
[project]
name = "test-package"
version = "1.0.0"
description = "A test package"
dependencies = ["requests>=2.0"]

[build-system]
requires = ["setuptools"]
build-backend = "setuptools.build_meta"
"#;
        let original = PyProjectToml::parse(toml).unwrap();

        // Convert to binary
        let binary = convert_to_binary(&original).unwrap();

        // Convert back
        let restored = convert_from_binary(&binary).unwrap();

        // Compare
        assert_eq!(original.name(), restored.name());
        assert_eq!(original.version(), restored.version());
        assert_eq!(original.dependencies(), restored.dependencies());
        assert_eq!(
            original.build_system.as_ref().map(|b| &b.requires),
            restored.build_system.as_ref().map(|b| &b.requires)
        );
    }

    #[test]
    fn test_binary_header_size() {
        // Ensure header is a reasonable size
        // Using const_assert pattern to verify at compile time
        const _: () = assert!(HEADER_SIZE <= 64);
    }

    #[test]
    fn test_to_toml() {
        let pyproject = PyProjectToml {
            project: Some(ProjectSection {
                name: "test".to_string(),
                version: Some("1.0.0".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let toml_str = pyproject.to_toml().unwrap();
        assert!(toml_str.contains("name = \"test\""));
        assert!(toml_str.contains("version = \"1.0.0\""));
    }
}
