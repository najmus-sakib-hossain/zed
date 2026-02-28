//! DX Test Cache - O(1) Layout Cache
//!
//! Memory-mapped test layout cache for instant test discovery.
//! Same breakthrough as package manager - cache entire test structure.

use blake3::Hasher;
use dx_test_core::*;
use memmap2::Mmap;
use parking_lot::RwLock;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

/// Test layout cache - pre-built test execution plan
pub struct TestLayoutCache {
    /// Cache directory
    #[allow(dead_code)]
    root: PathBuf,
    /// Layouts directory
    layouts_dir: PathBuf,
}

impl TestLayoutCache {
    pub fn new() -> io::Result<Self> {
        let root = std::env::temp_dir().join("dx-test-cache");
        let layouts_dir = root.join("layouts");

        fs::create_dir_all(&layouts_dir)?;

        Ok(Self { root, layouts_dir })
    }

    /// Compute layout hash from test sources
    pub fn compute_hash(project_root: &Path) -> u128 {
        let mut hasher = Hasher::new();

        // Walk test files
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| Self::is_test_file(e.path()))
        {
            // Hash file path
            hasher.update(entry.path().to_string_lossy().as_bytes());

            // Hash file content
            if let Ok(content) = fs::read(entry.path()) {
                hasher.update(&content);
            }
        }

        // Also hash config files
        for config in &["package.json", "tsconfig.json", "dx.toml"] {
            let config_path = project_root.join(config);
            if let Ok(content) = fs::read(config_path) {
                hasher.update(&content);
            }
        }

        let hash = hasher.finalize();
        u128::from_le_bytes(hash.as_bytes()[..16].try_into().unwrap())
    }

    /// Check if we have a valid cached layout
    pub fn get_cached_layout(&self, hash: u128) -> Option<CachedLayout> {
        let layout_path = self.layouts_dir.join(format!("{:032x}.dxtl", hash));

        if !layout_path.exists() {
            return None;
        }

        // Memory-map the layout
        let file = File::open(&layout_path).ok()?;
        let mmap = unsafe { Mmap::map(&file).ok()? };

        // Validate magic
        if mmap.len() < 4 || &mmap[0..4] != b"DXTL" {
            return None;
        }

        Some(CachedLayout {
            mmap: Arc::new(mmap),
            hash,
        })
    }

    /// Build and cache a new layout
    pub fn build_layout(&self, project_root: &Path) -> io::Result<CachedLayout> {
        let hash = Self::compute_hash(project_root);

        // Check if already exists
        if let Some(cached) = self.get_cached_layout(hash) {
            return Ok(cached);
        }

        let mut builder = LayoutBuilder::new();

        // Find all test files
        let mut test_files = Vec::new();
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| Self::is_test_file(e.path()))
        {
            test_files.push(entry.path().to_path_buf());
        }

        // Process each test file
        for (idx, path) in test_files.iter().enumerate() {
            let source = fs::read_to_string(path)?;
            builder.add_file(idx as u32, path, &source)?;
        }

        // Build the layout binary
        let layout_data = builder.build(hash)?;

        // Write to cache
        let layout_path = self.layouts_dir.join(format!("{:032x}.dxtl", hash));
        fs::write(&layout_path, &layout_data)?;

        // Memory-map and return
        let file = File::open(&layout_path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(CachedLayout {
            mmap: Arc::new(mmap),
            hash,
        })
    }

    fn is_test_file(path: &Path) -> bool {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        name.ends_with(".test.ts")
            || name.ends_with(".test.js")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.js")
            || name.ends_with("_test.ts")
            || name.ends_with("_test.js")
    }
}

/// Memory-mapped cached layout
pub struct CachedLayout {
    pub mmap: Arc<Mmap>,
    pub hash: u128,
}

impl CachedLayout {
    /// Get header
    #[inline(always)]
    pub fn header(&self) -> &TestLayoutHeader {
        unsafe { &*(self.mmap.as_ptr() as *const TestLayoutHeader) }
    }

    /// Get all tests - zero-copy slice
    #[inline(always)]
    pub fn tests(&self) -> &[FlatTestEntry] {
        let header = self.header();
        unsafe {
            std::slice::from_raw_parts(
                self.mmap.as_ptr().add(header.tests_offset as usize) as *const FlatTestEntry,
                header.test_count as usize,
            )
        }
    }

    /// Get test bytecode directly
    #[inline(always)]
    pub fn get_bytecode(&self, test: &FlatTestEntry) -> &[u8] {
        let start = test.bytecode_offset as usize;
        let end = start + test.bytecode_len as usize;
        &self.mmap[start..end]
    }

    /// Get test name
    pub fn get_test_name(&self, test: &FlatTestEntry) -> &str {
        let start = test.full_name_offset as usize;
        let end = start + test.full_name_len as usize;
        std::str::from_utf8(&self.mmap[start..end]).unwrap_or("")
    }
}

/// Layout builder
struct LayoutBuilder {
    files: Vec<TestFileEntry>,
    tests: Vec<FlatTestEntry>,
    bytecode_pool: Vec<u8>,
    names_pool: Vec<u8>,
}

impl LayoutBuilder {
    fn new() -> Self {
        Self {
            files: Vec::new(),
            tests: Vec::new(),
            bytecode_pool: Vec::new(),
            names_pool: Vec::new(),
        }
    }

    fn add_file(&mut self, file_idx: u32, path: &Path, source: &str) -> io::Result<()> {
        // Parse test file (simplified - just find describe/it blocks)
        let tests = self.parse_tests(source);

        let first_test = self.tests.len() as u32;

        for (name, bytecode) in tests {
            // Add bytecode to pool
            let bytecode_offset = self.bytecode_pool.len() as u64;
            self.bytecode_pool.extend_from_slice(&bytecode);

            // Add name to pool
            let name_offset = self.names_pool.len() as u32;
            self.names_pool.extend_from_slice(name.as_bytes());

            // Create test entry
            let entry = FlatTestEntry {
                name_hash: Self::hash_str(&name),
                full_name_offset: name_offset,
                full_name_len: name.len() as u16,
                file_idx,
                bytecode_offset,
                bytecode_len: bytecode.len() as u32,
                flags: 0,
                timeout_ms: 5000,
                assertion_count: 0,
                deps_bitmap: 0,
            };

            self.tests.push(entry);
        }

        // Add file entry
        let file_entry = TestFileEntry {
            path_hash: Self::hash_str(&path.to_string_lossy()),
            dxt_hash: 0,
            dxt_offset: 0,
            dxt_size: 0,
            test_count: (self.tests.len() as u32 - first_test),
            first_test,
        };

        self.files.push(file_entry);

        Ok(())
    }

    fn parse_tests(&self, source: &str) -> Vec<(String, Vec<u8>)> {
        let mut tests = Vec::new();

        // Simple regex-based parsing (for MVP)
        // In production, use swc_ecma_parser
        for line in source.lines() {
            let trimmed = line.trim();
            if let Some(test_name) = Self::extract_test_name(trimmed) {
                // Generate simple bytecode for this test
                let bytecode = self.generate_simple_bytecode(&test_name);
                tests.push((test_name, bytecode));
            }
        }

        tests
    }

    fn extract_test_name(line: &str) -> Option<String> {
        // Match: it('test name', ...) or test('test name', ...)
        if line.starts_with("it(") || line.starts_with("test(") {
            if let Some(start) = line.find('\'') {
                if let Some(end) = line[start + 1..].find('\'') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    return Some(line[start + 1..start + 1 + end].to_string());
                }
            }
        }
        None
    }

    fn generate_simple_bytecode(&self, _test_name: &str) -> Vec<u8> {
        // Generate simple bytecode: just mark test as passed
        vec![
            TestOpcode::PushTrue as u8,
            TestOpcode::TestPass as u8,
            TestOpcode::End as u8,
        ]
    }

    fn build(&mut self, source_hash: u128) -> io::Result<Vec<u8>> {
        let mut output = Vec::new();

        // Create header
        let mut header =
            TestLayoutHeader::new(source_hash, self.files.len() as u32, self.tests.len() as u32, 0);

        // Calculate offsets
        let header_size = std::mem::size_of::<TestLayoutHeader>();
        let files_size = self.files.len() * std::mem::size_of::<TestFileEntry>();
        let tests_size = self.tests.len() * std::mem::size_of::<FlatTestEntry>();

        header.files_offset = header_size as u64;
        header.tests_offset = (header_size + files_size) as u64;
        header.suites_offset = (header_size + files_size + tests_size) as u64;

        // Write header
        output.write_all(bytemuck::bytes_of(&header))?;

        // Write file entries
        for file in &self.files {
            output.write_all(bytemuck::bytes_of(file))?;
        }

        // Write test entries
        for test in &self.tests {
            output.write_all(bytemuck::bytes_of(test))?;
        }

        // Write names pool
        output.write_all(&self.names_pool)?;

        // Write bytecode pool
        output.write_all(&self.bytecode_pool)?;

        Ok(output)
    }

    fn hash_str(s: &str) -> u64 {
        let hash = blake3::hash(s.as_bytes());
        u64::from_le_bytes(hash.as_bytes()[..8].try_into().unwrap())
    }
}

/// Global warm state
pub struct WarmState {
    layout: RwLock<Option<CachedLayout>>,
}

impl WarmState {
    pub fn global() -> &'static Self {
        static INSTANCE: once_cell::sync::Lazy<WarmState> =
            once_cell::sync::Lazy::new(|| WarmState {
                layout: RwLock::new(None),
            });
        &INSTANCE
    }

    pub fn get_layout(&self, project_root: &Path) -> io::Result<CachedLayout> {
        let layout = self.layout.read();
        if let Some(cached) = layout.as_ref() {
            // Check if still valid
            let current_hash = TestLayoutCache::compute_hash(project_root);
            if cached.hash == current_hash {
                // Clone the Arc (cheap - just increments refcount)
                return Ok(CachedLayout {
                    mmap: Arc::clone(&cached.mmap),
                    hash: cached.hash,
                });
            }
        }
        drop(layout);

        // Build new layout
        let cache = TestLayoutCache::new()?;
        let new_layout = cache.build_layout(project_root)?;

        *self.layout.write() = Some(CachedLayout {
            mmap: Arc::clone(&new_layout.mmap),
            hash: new_layout.hash,
        });

        Ok(new_layout)
    }

    pub fn invalidate(&self) {
        *self.layout.write() = None;
    }
}
