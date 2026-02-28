//! Wheel file parser
//!
//! Parses Python wheel files (ZIP format) and extracts metadata.

use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

use crate::{Error, Result};

/// Parsed wheel file
pub struct WheelFile {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Python version requirement
    pub python_requires: Option<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Platform tags from WHEEL file
    pub platform_tags: Vec<String>,
    /// Files in the wheel
    pub files: Vec<WheelFileEntry>,
    /// Raw METADATA content
    pub metadata_raw: String,
}

/// Entry in a wheel file
#[derive(Clone, Debug)]
pub struct WheelFileEntry {
    /// Path within the wheel
    pub path: String,
    /// File size
    pub size: u64,
    /// File content
    pub content: Vec<u8>,
    /// Is this a Python source file?
    pub is_python: bool,
}

impl WheelFile {
    /// Open and parse a wheel file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path.as_ref())?;
        let mut archive = ZipArchive::new(file).map_err(|e| Error::Cache(e.to_string()))?;

        let mut metadata_raw = String::new();
        let mut wheel_content = String::new();
        let mut record_content = String::new();
        let mut files = Vec::new();

        // First pass: find and read metadata files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| Error::Cache(e.to_string()))?;
            let name = file.name().to_string();

            if name.ends_with("/METADATA") {
                file.read_to_string(&mut metadata_raw)
                    .map_err(|e| Error::Cache(e.to_string()))?;
            } else if name.ends_with("/WHEEL") {
                file.read_to_string(&mut wheel_content)
                    .map_err(|e| Error::Cache(e.to_string()))?;
            } else if name.ends_with("/RECORD") {
                file.read_to_string(&mut record_content)
                    .map_err(|e| Error::Cache(e.to_string()))?;
            }
        }

        // Second pass: read all files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| Error::Cache(e.to_string()))?;
            let name = file.name().to_string();

            // Skip directories
            if name.ends_with('/') {
                continue;
            }

            let mut content = Vec::new();
            file.read_to_end(&mut content).map_err(|e| Error::Cache(e.to_string()))?;

            let is_python = name.ends_with(".py");

            files.push(WheelFileEntry {
                path: name,
                size: content.len() as u64,
                content,
                is_python,
            });
        }

        // Parse METADATA
        let (name, version, python_requires, dependencies) = Self::parse_metadata(&metadata_raw)?;

        // Parse WHEEL for platform tags
        let platform_tags = Self::parse_wheel(&wheel_content);

        Ok(Self {
            name,
            version,
            python_requires,
            dependencies,
            platform_tags,
            files,
            metadata_raw,
        })
    }

    /// Parse METADATA file (RFC 822 format)
    fn parse_metadata(content: &str) -> Result<(String, String, Option<String>, Vec<String>)> {
        let mut name = String::new();
        let mut version = String::new();
        let mut python_requires = None;
        let mut dependencies = Vec::new();

        for line in content.lines() {
            if let Some(value) = line.strip_prefix("Name: ") {
                name = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Version: ") {
                version = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("Requires-Python: ") {
                python_requires = Some(value.trim().to_string());
            } else if let Some(value) = line.strip_prefix("Requires-Dist: ") {
                dependencies.push(value.trim().to_string());
            }
        }

        if name.is_empty() {
            return Err(Error::InvalidPackageName("Missing Name in METADATA".to_string()));
        }
        if version.is_empty() {
            return Err(Error::InvalidPackageName("Missing Version in METADATA".to_string()));
        }

        Ok((name, version, python_requires, dependencies))
    }

    /// Parse WHEEL file for platform tags
    fn parse_wheel(content: &str) -> Vec<String> {
        let mut tags = Vec::new();

        for line in content.lines() {
            if let Some(value) = line.strip_prefix("Tag: ") {
                tags.push(value.trim().to_string());
            }
        }

        tags
    }

    /// Get Python source files
    pub fn python_files(&self) -> impl Iterator<Item = &WheelFileEntry> {
        self.files.iter().filter(|f| f.is_python)
    }

    /// Get all files
    pub fn all_files(&self) -> &[WheelFileEntry] {
        &self.files
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_metadata() {
        let metadata = r#"Metadata-Version: 2.1
Name: requests
Version: 2.31.0
Requires-Python: >=3.7
Requires-Dist: charset-normalizer<4,>=2
Requires-Dist: idna<4,>=2.5
Requires-Dist: urllib3<3,>=1.21.1
"#;

        let (name, version, python_requires, deps) = WheelFile::parse_metadata(metadata).unwrap();

        assert_eq!(name, "requests");
        assert_eq!(version, "2.31.0");
        assert_eq!(python_requires, Some(">=3.7".to_string()));
        assert_eq!(deps.len(), 3);
    }

    #[test]
    fn test_parse_wheel() {
        let wheel = r#"Wheel-Version: 1.0
Generator: bdist_wheel
Root-Is-Purelib: true
Tag: py3-none-any
"#;

        let tags = WheelFile::parse_wheel(wheel);
        assert_eq!(tags, vec!["py3-none-any"]);
    }
}
