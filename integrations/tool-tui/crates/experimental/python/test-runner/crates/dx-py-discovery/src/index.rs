//! Binary test index (.dxti) format for fast test discovery caching

use dx_py_core::{DiscoveryError, FixtureId, Marker, TestCase};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Magic bytes for .dxti files
const MAGIC: &[u8; 4] = b"DXTI";
const VERSION: u16 = 1;

/// Entry for a file in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub path_hash: u64,
    pub mtime: u64,
    pub content_hash: [u8; 32],
    pub tests: Vec<TestEntry>,
}

/// Entry for a test in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEntry {
    pub name: String,
    pub line: u32,
    pub class_name: Option<String>,
    pub markers: Vec<Marker>,
    pub fixtures: Vec<FixtureId>,
}

impl TestEntry {
    pub fn to_test_case(&self, file_path: &Path) -> TestCase {
        let mut tc = TestCase::new(&self.name, file_path, self.line);
        if let Some(ref class) = self.class_name {
            tc = tc.with_class(class.clone());
        }
        for marker in &self.markers {
            tc = tc.with_marker(marker.clone());
        }
        for fixture in &self.fixtures {
            tc = tc.with_fixture(*fixture);
        }
        tc
    }
}

/// Binary test index for fast discovery caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestIndex {
    pub version: u16,
    pub files: HashMap<u64, FileEntry>,
}

impl TestIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            version: VERSION,
            files: HashMap::new(),
        }
    }

    /// Load index from file
    pub fn load(path: &Path) -> Result<Self, DiscoveryError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Read and verify magic
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(DiscoveryError::IndexCorrupted("Invalid magic bytes".into()));
        }

        // Read version
        let mut version_bytes = [0u8; 2];
        reader.read_exact(&mut version_bytes)?;
        let version = u16::from_le_bytes(version_bytes);
        if version != VERSION {
            return Err(DiscoveryError::IndexCorrupted(format!(
                "Unsupported version: {}",
                version
            )));
        }

        // Read the rest as bincode
        let index: TestIndex = bincode::deserialize_from(reader)
            .map_err(|e| DiscoveryError::IndexCorrupted(e.to_string()))?;

        Ok(index)
    }

    /// Save index to file
    pub fn save(&self, path: &Path) -> Result<(), DiscoveryError> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write magic
        writer.write_all(MAGIC)?;

        // Write version
        writer.write_all(&self.version.to_le_bytes())?;

        // Write the rest as bincode
        bincode::serialize_into(&mut writer, self)
            .map_err(|e| DiscoveryError::IoError(std::io::Error::other(e.to_string())))?;

        writer.flush()?;
        Ok(())
    }

    /// Get all test cases from the index
    pub fn all_tests(&self) -> Vec<TestCase> {
        self.files
            .values()
            .flat_map(|entry| entry.tests.iter().map(|t| t.to_test_case(&entry.path)))
            .collect()
    }

    /// Get tests for a specific file
    pub fn tests_for_file(&self, path: &Path) -> Vec<TestCase> {
        let hash = Self::hash_path(path);
        self.files
            .get(&hash)
            .map(|entry| entry.tests.iter().map(|t| t.to_test_case(&entry.path)).collect())
            .unwrap_or_default()
    }

    /// Check if a file needs re-scanning
    pub fn needs_rescan(&self, path: &Path) -> bool {
        let hash = Self::hash_path(path);
        match self.files.get(&hash) {
            None => true,
            Some(entry) => {
                // Check mtime
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(mtime) = metadata.modified() {
                        let mtime_secs = mtime
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        if mtime_secs != entry.mtime {
                            return true;
                        }
                    }
                }
                // Check content hash
                if let Ok(content) = std::fs::read(path) {
                    let hash = blake3::hash(&content);
                    if hash.as_bytes() != &entry.content_hash {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Hash a path for lookup
    pub fn hash_path(path: &Path) -> u64 {
        let hash = blake3::hash(path.to_string_lossy().as_bytes());
        let bytes = hash.as_bytes();
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }

    /// Get total test count
    pub fn test_count(&self) -> usize {
        self.files.values().map(|f| f.tests.len()).sum()
    }

    /// Get file count
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for TestIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing a TestIndex
pub struct TestIndexBuilder {
    index: TestIndex,
}

impl TestIndexBuilder {
    pub fn new() -> Self {
        Self {
            index: TestIndex::new(),
        }
    }

    /// Add a file with its tests to the index
    pub fn add_file(&mut self, path: &Path, tests: Vec<TestCase>) -> Result<(), DiscoveryError> {
        let content = std::fs::read(path)?;
        let content_hash = blake3::hash(&content);
        let mtime = std::fs::metadata(path)?
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let path_hash = TestIndex::hash_path(path);

        let test_entries: Vec<TestEntry> = tests
            .into_iter()
            .map(|tc| TestEntry {
                name: tc.name,
                line: tc.line_number,
                class_name: tc.class_name,
                markers: tc.markers,
                fixtures: tc.fixtures,
            })
            .collect();

        self.index.files.insert(
            path_hash,
            FileEntry {
                path: path.to_owned(),
                path_hash,
                mtime,
                content_hash: *content_hash.as_bytes(),
                tests: test_entries,
            },
        );

        Ok(())
    }

    /// Build the final index
    pub fn build(self) -> TestIndex {
        self.index
    }
}

impl Default for TestIndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}
