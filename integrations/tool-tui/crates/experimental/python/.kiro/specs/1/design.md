
# Design Document: DX-Py Package Manager

## Overview

DX-Py is a high-performance Python package manager designed to be 5-50x faster than uv. The system leverages binary-first architecture, SIMD acceleration, and platform-native async I/O through dx-reactor integration. Following the proven patterns from dx-js-package-manager, DX-Py uses zero-copy memory-mapped binary formats for all core operations. The architecture consists of four main crates: -dx-py-core: Core types, binary format definitions, and shared utilities -dx-py-package-manager: Binary package operations (DPP format, resolution, installation) -dx-py-project-manager: Project lifecycle (Python management, venv, workspaces) -dx-py-cli: Unified command-line interface

## Architecture

@tree[]

## Components and Interfaces

### 1. dx-py-core

Core types and binary format definitions shared across all crates.
```rust
// dx-py-core/src/lib.rs pub mod error;
pub mod hash;
pub mod headers;
pub mod version;
/// Magic numbers for binary format identification pub const DPP_MAGIC: &[u8; 4] = b"DPP\x01"; // Dx Python Package pub const DPL_MAGIC: &[u8; 4] = b"DPL\x01"; // Dx Python Lock pub const DPI_MAGIC: &[u8; 4] = b"DPI\x01"; // Dx Python Index /// Protocol version pub const PROTOCOL_VERSION: u16 = 1;
/// Security limits pub const MAX_PACKAGE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB (PyTorch is large)
pub const MAX_FILE_COUNT: u32 = 500_000;
pub const MAX_LOCK_SIZE: u64 = 512 * 1024 * 1024; // 512MB ```


### 2. DPP Header (Binary Package Format)


```rust
// dx-py-core/src/headers.rs /// DPP Header - 64 bytes, fixed layout for O(1) access

#[repr(C, packed)]

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]

pub struct DppHeader { pub magic: [u8; 4], // "DPP\x01"
pub version: u16, // Format version pub flags: u16, // Compression, platform flags // Offsets for O(1) section access pub metadata_offset: u32, // Package metadata section pub files_offset: u32, // File table section pub bytecode_offset: u32, // Pre-compiled bytecode (.pyc)
pub native_offset: u32, // Native extensions (.so/.pyd)
pub deps_offset: u32, // Dependency graph (pre-resolved)
// Sizes pub total_size: u64, // Total package size pub uncompressed_size: u64, // Uncompressed size // Integrity pub blake3_hash: [u8; 32], // BLAKE3 hash of content }
/// Package metadata - variable length, but fixed structure

#[repr(C)]

pub struct DppMetadata { pub name_len: u16, pub version_len: u16, pub python_requires_len: u16, pub _padding: u16, // Followed by: name bytes, version bytes, python_requires bytes }
```


### 3. DPL Header (Binary Lock Format)


```rust
// dx-py-core/src/headers.rs /// DPL Header - instant access to lock state

#[repr(C, packed)]

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]

pub struct DplHeader { pub magic: [u8; 4], // "DPL\x01"
pub version: u16, pub package_count: u32, pub _padding: u16, // Hash table for O(1) lookup pub hash_table_offset: u32, pub hash_table_size: u32, // Package entries pub entries_offset: u32, // Resolution metadata pub python_version: [u8; 16], // e.g., "3.12.0"
pub platform: [u8; 32], // e.g., "manylinux_2_17_x86_64"
pub resolved_at: u64, // Unix timestamp // Integrity pub content_hash: [u8; 32], // BLAKE3 of all entries }
/// Package entry - fixed 128 bytes for predictable layout

#[repr(C, packed)]

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]

pub struct DplEntry { pub name_hash: u64, // FNV-1a hash of package name pub name: [u8; 64], // Package name (null-terminated)
pub version: [u8; 24], // Version string pub source_type: u8, // PyPI, Git, URL, Path pub _padding: [u8; 7], pub source_hash: [u8; 32], // Source integrity hash }
```


### 4. Version Comparison (SIMD)


```rust
// dx-py-core/src/version.rs use std::arch::x86_64::*;
/// Packed version for SIMD operations

#[repr(C)]

#[derive(Clone, Copy)]

pub struct PackedVersion { pub major: u32, pub minor: u32, pub patch: u32, pub _padding: u32, }
/// SIMD version comparison - process 8 versions in parallel

#[cfg(target_arch = "x86_64")]

#[target_feature(enable = "avx2")]

pub unsafe fn compare_versions_simd( constraint_min: &PackedVersion, candidates: &[PackedVersion; 8], ) -> u8 { // Load constraint minimum let min_major = _mm256_set1_epi32(constraint_min.major as i32);
let min_minor = _mm256_set1_epi32(constraint_min.minor as i32);
let min_patch = _mm256_set1_epi32(constraint_min.patch as i32);
// Load 8 candidate majors let majors = _mm256_loadu_si256( candidates.as_ptr() as *const __m256i );
// Compare major >= min_major let major_ge = _mm256_cmpgt_epi32( majors, _mm256_sub_epi32(min_major, _mm256_set1_epi32(1))
);
// Extract match mask _mm256_movemask_ps(_mm256_castsi256_ps(major_ge)) as u8 }
/// Scalar fallback for non-AVX2 systems pub fn compare_versions_scalar( constraint_min: &PackedVersion, candidates: &[PackedVersion], ) -> Vec<bool> { candidates.iter().map(|c| {
c.major > constraint_min.major || (c.major == constraint_min.major && c.minor > constraint_min.minor) || (c.major == constraint_min.major && c.minor == constraint_min.minor && c.patch >= constraint_min.patch)
}).collect()
}
```


### 5. Package Manager Components


```rust
// dx-py-package-manager/src/lib.rs pub mod formats; // DPP package format pub mod resolver; // PubGrub + SIMD pub mod installer; // Zero-copy installation pub mod cache; // Content-addressable storage pub mod registry; // PyPI + DPRP protocol pub mod converter; // Wheel -> DPP conversion // dx-py-package-manager/src/formats/dpp.rs use dx_py_core::{DppHeader, DPP_MAGIC};
use memmap2::Mmap;
/// Zero-copy DPP package access pub struct DppPackage { mmap: Mmap, header: DppHeader, }
impl DppPackage { /// Open package with memory mapping (zero-copy)
pub fn open(path: &Path) -> Result<Self> { let file = File::open(path)?;
let mmap = unsafe { Mmap::map(&file)? };
// Verify magic if &mmap[0..4] != DPP_MAGIC { return Err(Error::InvalidMagic);
}
// Zero-copy header access let header = *bytemuck::from_bytes::<DppHeader>(&mmap[0..64]);
// Verify integrity let content = &mmap[64..header.total_size as usize];
let computed = blake3::hash(content);
if computed.as_bytes() != &header.blake3_hash { return Err(Error::IntegrityError);
}
Ok(Self { mmap, header })
}
/// O(1) metadata access - no parsing!

#[inline]

pub fn metadata(&self) -> &DppMetadata { let offset = self.header.metadata_offset as usize;
bytemuck::from_bytes(&self.mmap[offset..offset + size_of::<DppMetadata>()])
}
/// Get pre-compiled bytecode section pub fn bytecode(&self) -> &[u8] { let start = self.header.bytecode_offset as usize;
let end = self.header.native_offset as usize;
&self.mmap[start..end]
}
}
```


### 6. Lock File Operations


```rust
// dx-py-package-manager/src/formats/dpl.rs use dx_py_core::{DplHeader, DplEntry, DPL_MAGIC};
/// Zero-copy lock file access pub struct DplLockFile { mmap: Mmap, header: DplHeader, }
impl DplLockFile { /// O(1) package lookup using hash table pub fn lookup(&self, package_name: &str) -> Option<&DplEntry> { let hash = fnv1a_hash(package_name.as_bytes());
let slot = (hash % self.header.hash_table_size as u64) as usize;
// Read hash table entry let table_offset = self.header.hash_table_offset as usize;
let entry_idx: u32 = bytemuck::from_bytes( &self.mmap[table_offset + slot * 4..table_offset + slot * 4 + 4]
);
if entry_idx != u32::MAX { let entries_offset = self.header.entries_offset as usize;
let entry_offset = entries_offset + (entry_idx as usize * 128);
Some(bytemuck::from_bytes(&self.mmap[entry_offset..entry_offset + 128]))
} else { None }
}
/// Verify integrity with SIMD BLAKE3 pub fn verify(&self) -> bool { let content_start = self.header.entries_offset as usize;
let content_end = content_start + (self.header.package_count as usize * 128);
let computed = blake3::hash(&self.mmap[content_start..content_end]);
computed.as_bytes() == &self.header.content_hash }
}
/// Lock file builder pub struct DplBuilder { entries: Vec<DplEntry>, python_version: String, platform: String, }
impl DplBuilder { pub fn new(python_version: &str, platform: &str) -> Self { Self { entries: Vec::new(), python_version: python_version.to_string(), platform: platform.to_string(), }
}
pub fn add_package(&mut self, name: &str, version: &str, hash: [u8; 32]) { let mut entry = DplEntry::zeroed();
entry.name_hash = fnv1a_hash(name.as_bytes());
entry.name[..name.len().min(63)].copy_from_slice(name.as_bytes());
entry.version[..version.len().min(23)].copy_from_slice(version.as_bytes());
entry.source_hash = hash;
self.entries.push(entry);
}
pub fn build(&self) -> Vec<u8> { // Build hash table and serialize to binary let mut output = Vec::new();
// ... serialization logic output }
}
```


### 7. Resolver with Resolution Hints


```rust
// dx-py-package-manager/src/resolver/mod.rs pub mod pubgrub;
pub mod simd;
pub mod hints;
use dx_py_core::version::PackedVersion;
/// Resolution result pub struct Resolution { pub packages: Vec<ResolvedPackage>, pub resolution_time_ms: u64, }
/// Resolver with hint cache pub struct Resolver { hints: HintCache, pypi: PyPiClient, }
impl Resolver { /// Resolve dependencies with hint cache pub async fn resolve(&mut self, deps: &[Dependency]) -> Result<Resolution> { // Check hint cache first let input_hash = self.hash_dependencies(deps);
if let Some(cached) = self.hints.lookup(input_hash) { if cached.is_valid() { return Ok(cached.into_resolution());
}
}
// Try delta resolution if similar resolution exists if let Some(delta) = self.hints.find_similar_and_patch(deps) { return Ok(delta);
}
// Full resolution using PubGrub let resolution = self.pubgrub_resolve(deps).await?;
// Cache the result self.hints.store(input_hash, &resolution);
Ok(resolution)
}
}
// dx-py-package-manager/src/resolver/hints.rs /// Resolution hint cache pub struct HintCache { mmap: MmapMut, hints: HashMap<u64, ResolutionSnapshot>, }
/// Snapshot of a successful resolution

#[repr(C)]

pub struct ResolutionSnapshot { pub input_hash: [u8; 32], pub resolved_count: u32, pub resolved_offset: u32, pub created_at: u64, pub valid_until: u64, pub parent_hash: [u8; 32], // For delta chain pub delta_offset: u32, }
impl HintCache { /// Try to reuse previous resolution pub fn lookup(&self, input_hash: u64) -> Option<&ResolutionSnapshot> { self.hints.get(&input_hash).filter(|s| s.is_valid())
}
/// Find similar resolution and compute delta pub fn find_similar_and_patch(&self, deps: &[Dependency]) -> Option<Resolution> { let closest = self.find_closest(deps)?;
let diff = self.compute_diff(deps, &closest);
// Only use delta if less than 10% changed if diff.changes < deps.len() / 10 { Some(self.apply_delta_resolution(closest, diff))
} else { None }
}
}
```


### 8. Download Manager with dx-reactor


```rust
// dx-py-package-manager/src/registry/download.rs use dx_reactor::{DxReactor, IoBackend};
/// High-performance download manager pub struct DownloadManager { reactor: DxReactor, connection_pool: ConnectionPool, }
impl DownloadManager { pub fn new() -> Self { let reactor = DxReactor::build()
.io_backend(IoBackend::Auto) // io_uring/kqueue/IOCP .buffer_size(64 * 1024) // 64KB buffers .buffer_count(256) // 256 pre-allocated buffers .build();
Self { reactor, connection_pool: ConnectionPool::new(), }
}
/// Download multiple packages in parallel pub async fn download_batch(&self, packages: &[PackageSpec]) -> Result<Vec<Vec<u8>>> { use futures::stream::{self, StreamExt};
// Process in batches of 32 concurrent downloads let results: Vec<_> = stream::iter(packages)
.map(|spec| self.download_single(spec))
.buffer_unordered(32)
.collect()
.await;
results.into_iter().collect()
}
async fn download_single(&self, spec: &PackageSpec) -> Result<Vec<u8>> { let url = spec.download_url();
let response = self.connection_pool .get(&url)
.await?;
// Verify integrity let hash = blake3::hash(&response);
if hash.as_bytes() != &spec.expected_hash { return Err(Error::IntegrityError);
}
Ok(response)
}
}
```


### 9. Zero-Copy Installer


```rust
// dx-py-package-manager/src/installer/mod.rs pub mod zerocopy;
pub mod link;

#[cfg(target_os = "linux")]

pub mod fuse;
/// Installation strategy pub enum InstallStrategy { /// FUSE mount (Linux only) - instant, zero disk usage Fuse, /// Hard links from cache - fast, deduplication HardLink, /// Copy files - fallback, always works Copy, }
/// Zero-copy installer pub struct Installer { cache: GlobalCache, strategy: InstallStrategy, }
impl Installer { /// Install packages to site-packages pub async fn install( &self, packages: &[DppPackage], site_packages: &Path, ) -> Result<()> { match self.strategy { InstallStrategy::HardLink => { for package in packages { self.install_hardlink(package, site_packages)?;
}
}
InstallStrategy::Copy => { for package in packages { self.install_copy(package, site_packages)?;
}
}

#[cfg(target_os = "linux")]

InstallStrategy::Fuse => { self.install_fuse(packages, site_packages)?;
}
}
Ok(())
}
fn install_hardlink(&self, package: &DppPackage, dest: &Path) -> Result<()> { // Get cached package path let cache_path = self.cache.get_path(package.hash());
// Create hard links for all files for file in package.files() { let src = cache_path.join(&file.path);
let dst = dest.join(&file.path);
if let Some(parent) = dst.parent() { std::fs::create_dir_all(parent)?;
}
std::fs::hard_link(&src, &dst)?;
}
Ok(())
}
}
// dx-py-package-manager/src/cache/mod.rs /// Content-addressable global cache pub struct GlobalCache { root: PathBuf, index: CacheIndex, }
impl GlobalCache { /// Get or download package to cache pub async fn ensure(&self, spec: &PackageSpec) -> Result<PathBuf> { let hash = spec.content_hash();
let cache_path = self.root.join(format!("{:x}", hash));
if cache_path.exists() { return Ok(cache_path);
}
// Download and extract to cache let data = self.download(spec).await?;
self.extract_to_cache(&data, &cache_path)?;
Ok(cache_path)
}
/// Garbage collect unused packages pub fn gc(&self, keep_days: u32) -> Result<GcStats> { let cutoff = SystemTime::now() - Duration::from_secs(keep_days as u64 * 86400);
let mut stats = GcStats::default();
for entry in std::fs::read_dir(&self.root)? { let entry = entry?;
let metadata = entry.metadata()?;
if metadata.accessed()? < cutoff { let size = metadata.len();
std::fs::remove_dir_all(entry.path())?;
stats.removed_count += 1;
stats.freed_bytes += size;
}
}
Ok(stats)
}
}
```


### 10. Project Manager Components


```rust
// dx-py-project-manager/src/lib.rs pub mod python; // Python version management pub mod venv; // Virtual environment pub mod workspace; // Monorepo support pub mod scripts; // Script execution pub mod tools; // Global tools (pipx replacement)
pub mod config; // pyproject.dx format // dx-py-project-manager/src/python/mod.rs /// Python version manager pub struct PythonManager { install_dir: PathBuf, builds_url: String, }
impl PythonManager { /// Install Python version from pre-built binaries pub async fn install(&self, version: &str) -> Result<PathBuf> { let url = format!("{}/python-{}-{}.tar.zst", self.builds_url, version, current_platform());
let data = download(&url).await?;
let dest = self.install_dir.join(version);
// Extract with zstd (fast decompression)
zstd::decompress_to_path(&data, &dest)?;
Ok(dest.join("bin/python"))
}
/// Discover system Python installations pub fn discover(&self) -> Vec<PythonInstall> { let mut found = Vec::new();
// Check common locations for path in &["/usr/bin/python3", "/usr/local/bin/python3"] { if let Ok(version) = self.get_version(path) { found.push(PythonInstall { path: path.into(), version });
}
}
// Check pyenv if let Ok(pyenv_root) = std::env::var("PYENV_ROOT") { // ... scan pyenv versions }
found }
}
```


### 11. Virtual Environment Manager


```rust
// dx-py-project-manager/src/venv/mod.rs /// Ultra-fast venv creation pub struct VenvManager { cache: VenvCache, }
impl VenvManager { /// Create virtual environment in under 10ms pub fn create(&self, path: &Path, python: &Path) -> Result<Venv> { // Check if we can reuse a cached venv skeleton let python_version = self.get_python_version(python)?;
if let Some(skeleton) = self.cache.get_skeleton(&python_version) { // Copy skeleton (mostly symlinks) - ~5ms self.copy_skeleton(&skeleton, path)?;
} else { // Create minimal venv structure std::fs::create_dir_all(path.join("bin"))?;
std::fs::create_dir_all(path.join("lib/python/site-packages"))?;
// Symlink Python executable std::os::unix::fs::symlink(python, path.join("bin/python"))?;
// Create activation scripts self.write_activate_scripts(path, python)?;
// Cache skeleton for future use self.cache.store_skeleton(&python_version, path)?;
}
Ok(Venv { path: path.to_path_buf() })
}
fn write_activate_scripts(&self, venv: &Path, python: &Path) -> Result<()> { // bash/zsh activation let activate_sh = format!(r#"
export VIRTUAL_ENV="{}"
export PATH="BenchmarksIRTUAL_ENV/bin:10.59xATH"
unset PYTHONHOME "#, venv.display());
std::fs::write(venv.join("bin/activate"), activate_sh)?;
// fish activation let activate_fish = format!(r#"
set -gx VIRTUAL_ENV "{}"
set -gx PATH "BenchmarksIRTUAL_ENV/bin" 10.59xATH "#, venv.display());
std::fs::write(venv.join("bin/activate.fish"), activate_fish)?;
// PowerShell activation let activate_ps1 = format!(r#"
$env:VIRTUAL_ENV = "{}"
$env:PATH = "$env:VIRTUAL_ENV\Scripts;$env:PATH"
"#, venv.display());
std::fs::write(venv.join("Scripts/Activate.ps1"), activate_ps1)?;
Ok(())
}
}
```


### 12. Workspace Support


```rust
// dx-py-project-manager/src/workspace/mod.rs /// Cargo-style workspace manager pub struct WorkspaceManager { root: PathBuf, config: WorkspaceConfig, }

#[derive(Deserialize)]

pub struct WorkspaceConfig { pub members: Vec<String>, pub exclude: Vec<String>, pub shared_dependencies: HashMap<String, String>, }
impl WorkspaceManager { /// Load workspace from pyproject.dx or pyproject.toml pub fn load(root: &Path) -> Result<Self> { let config_path = root.join("pyproject.dx");
let config = if config_path.exists() { // Binary format - instant load Self::load_binary(&config_path)?
} else { // TOML fallback Self::load_toml(&root.join("pyproject.toml"))?
};
Ok(Self { root: root.to_path_buf(), config })
}
/// Get all workspace members pub fn members(&self) -> Result<Vec<WorkspaceMember>> { let mut members = Vec::new();
for pattern in &self.config.members { for entry in glob::glob(&self.root.join(pattern).to_string_lossy())? { let path = entry?;
if path.join("pyproject.toml").exists() || path.join("pyproject.dx").exists() { members.push(WorkspaceMember::load(&path)?);
}
}
}
Ok(members)
}
/// Resolve dependencies across all workspace members pub async fn resolve_all(&self) -> Result<Resolution> { let members = self.members()?;
let mut all_deps = HashMap::new();
// Collect all dependencies for member in &members { for (name, version) in &member.dependencies { // Use shared version if defined let version = self.config.shared_dependencies .get(name)
.unwrap_or(version);
all_deps.insert(name.clone(), version.clone());
}
}
// Single resolution for entire workspace let resolver = Resolver::new();
resolver.resolve(&all_deps.into_iter().collect::<Vec<_>>()).await }
}
```


### 13. CLI Interface


```rust
// dx-py-cli/src/main.rs use clap::{Parser, Subcommand};

#[derive(Parser)]

#[command(name = "dx-py")]

#[command(about = "Ultra-fast Python package manager")]

struct Cli {

#[command(subcommand)]

command: Commands, }

#[derive(Subcommand)]

enum Commands { /// Initialize a new Python project Init {

#[arg(long)]

python: Option<String>,

#[arg(long)]

name: Option<String>, }, /// Add dependencies Add { packages: Vec<String>,

#[arg(short = 'D', long)]

dev: bool, }, /// Remove dependencies Remove { packages: Vec<String>, }, /// Generate lock file Lock, /// Install from lock file Sync, /// Lock + sync (convenience)
Install, /// Run a command in the virtual environment Run { command: Vec<String>, }, /// Python version management Python {

#[command(subcommand)]

command: PythonCommands, }, /// Global tool management (pipx replacement)
Tool {

#[command(subcommand)]

command: ToolCommands, }, /// Build package for distribution Build, /// Publish to PyPI Publish, }

#[derive(Subcommand)]

enum PythonCommands { /// Install a Python version Install { version: String }, /// List installed versions List, /// Pin Python version for project Pin { version: String }, }

#[derive(Subcommand)]

enum ToolCommands { /// Install a tool globally Install { name: String }, /// Run a tool ephemerally Run { name: String, args: Vec<String> }, /// List installed tools List, }
```


### 14. Compatibility Layer


```rust
// dx-py-compat/src/lib.rs pub mod pip;
pub mod uv;
pub mod pyproject;
// dx-py-compat/src/pyproject.rs use serde::{Deserialize, Serialize};
/// pyproject.toml structure

#[derive(Deserialize, Serialize)]

pub struct PyProjectToml { pub project: Option<ProjectSection>, pub tool: Option<ToolSection>, pub build_system: Option<BuildSystem>, }

#[derive(Deserialize, Serialize)]

pub struct ProjectSection { pub name: String, pub version: String, pub description: Option<String>, pub dependencies: Option<Vec<String>>, pub optional_dependencies: Option<HashMap<String, Vec<String>>>, pub requires_python: Option<String>, }
/// Convert pyproject.toml to binary pyproject.dx pub fn convert_to_binary(toml: &PyProjectToml) -> Result<Vec<u8>> { let mut output = Vec::new();
// Write header output.extend_from_slice(b"DXPY");
output.extend_from_slice(&1u16.to_le_bytes()); // version // Serialize project metadata if let Some(project) = &toml.project { // ... binary serialization }
Ok(output)
}
/// Convert binary pyproject.dx back to TOML (for compatibility)
pub fn convert_to_toml(binary: &[u8]) -> Result<PyProjectToml> { // Verify magic if &binary[0..4] != b"DXPY" { return Err(Error::InvalidFormat);
}
// Parse binary format // ... deserialization Ok(PyProjectToml::default())
}
```


## Data Models



### Package Specification


```rust
pub struct PackageSpec { pub name: String, pub version: VersionConstraint, pub extras: Vec<String>, pub markers: Option<String>, // PEP 508 markers }
pub enum VersionConstraint { Exact(String), // ==1.2.3 Compatible(String), // ~=1.2.3 GreaterEqual(String), // >=1.2.3 Less(String), // <2.0.0 Range(String, String), // >=1.2.3,<2.0.0 Any, // * }
```


### Resolved Package


```rust
pub struct ResolvedPackage { pub name: String, pub version: String, pub source: PackageSource, pub dependencies: Vec<String>, pub content_hash: [u8; 32], pub download_url: String, }
pub enum PackageSource { PyPi, Git { url: String, rev: String }, Url { url: String }, Path { path: PathBuf }, }
```


## Correctness Properties


A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees. Based on the acceptance criteria analysis, the following correctness properties must be validated through property-based testing:


### Property 1: DPP Format Structure Validity


For any valid DPP package, the header SHALL be exactly 64 bytes, all section offsets SHALL point to valid locations within the file, and the BLAKE3 hash SHALL match the content. Validates: Requirements 1.1, 1.3, 1.4, 1.5


### Property 2: DPP Wheel Conversion Round-Trip


For any valid Python wheel file, converting to DPP format and then extracting back to wheel format SHALL produce a functionally equivalent package (same files, same metadata, same dependencies). Validates: Requirements 1.7, 1.8


### Property 3: DPL Format Structure Validity


For any valid DPL lock file, the hash table SHALL have correct size, all entries SHALL be exactly 128 bytes, and platform/Python version metadata SHALL be present and valid. Validates: Requirements 2.1, 2.3, 2.4


### Property 4: DPL Round-Trip Consistency


For any valid DPL lock file, parsing the binary format and then serializing back to binary SHALL produce an identical byte sequence. Validates: Requirements 2.8


### Property 5: SIMD/Scalar Version Resolution Equivalence


For any version constraint and set of candidate versions, the SIMD-accelerated comparison SHALL produce the exact same matching results as the scalar fallback implementation. Validates: Requirements 4.5, 4.6


### Property 6: Hash Table O(1) Lookup Correctness


For any DPL lock file with N packages, looking up any package by name SHALL return the correct entry, and looking up a non-existent package SHALL return None. The lookup SHALL not depend on the number of packages (O(1) complexity). Validates: Requirements 2.1


### Property 7: Content-Addressable Storage Deduplication


For any two packages with identical file content, storing both in the cache SHALL result in only one copy of the shared content on disk, and both packages SHALL correctly reference the shared content. Validates: Requirements 6.2, 6.3


### Property 8: pyproject.toml Round-Trip Conversion


For any valid pyproject.toml file, converting to binary pyproject.dx format and back to TOML SHALL preserve all project metadata, dependencies, and configuration. Validates: Requirements 12.4, 12.5


### Property 9: Resolution Hint Cache Correctness


For any set of dependencies, if a cached resolution exists and is valid, using the cached resolution SHALL produce the same installed package set as performing a fresh resolution. Validates: Requirements 7.1, 7.3


## Error Handling



### Error Types


```rust
// dx-py-core/src/error.rs use thiserror::Error;

#[derive(Error, Debug)]

pub enum Error { // Format errors

#[error("Invalid magic number: expected {expected:?}, found {found:?}")]

InvalidMagic { expected: [u8; 4], found: [u8; 4] },

#[error("Corrupted data: integrity check failed")]

IntegrityError,

#[error("Unsupported format version: {0}")]

UnsupportedVersion(u16), // Resolution errors

#[error("No matching version found for {package} with constraint {constraint}")]

NoMatchingVersion { package: String, constraint: String },

#[error("Dependency conflict: {0}")]

DependencyConflict(String),

#[error("Circular dependency detected: {0}")]

CircularDependency(String), // Network errors

#[error("Network error: {0}")]

Network(#[from] reqwest::Error),

#[error("Package not found: {0}")]

PackageNotFound(String), // I/O errors

#[error("I/O error: {0}")]

Io(#[from] std::io::Error),

#[error("Cache error: {0}")]

Cache(String), // Python errors

#[error("Python version not found: {0}")]

PythonNotFound(String),

#[error("Virtual environment error: {0}")]

VenvError(String), }
pub type Result<T> = std::result::Result<T, Error>;
```


### Error Recovery Strategies


- Network failures: Retry with exponential backoff, fall back to cached data
- Integrity failures: Re-download package, report corruption
- Resolution conflicts: Provide detailed conflict explanation with suggestions
- Cache corruption: Clear affected entries, rebuild from source


## Testing Strategy



### Property-Based Testing


The project uses `proptest` for property-based testing with minimum 100 iterations per property.
```rust
// Example property test structure use proptest::prelude::*;
proptest! {

#![proptest_config(ProptestConfig::with_cases(100))]

/// Feature: dx-py-package-manager, Property 4: DPL Round-Trip Consistency /// Validates: Requirements 2.8

#[test]

fn prop_dpl_roundtrip(entries in prop::collection::vec(arb_dpl_entry(), 1..100)) { let builder = DplBuilder::new("3.12.0", "manylinux_2_17_x86_64");
for entry in &entries { builder.add_package(&entry.name, &entry.version, entry.hash);
}
let binary = builder.build();
let parsed = DplLockFile::from_bytes(&binary).unwrap();
let reserialized = parsed.to_bytes();
prop_assert_eq!(binary, reserialized);
}
/// Feature: dx-py-package-manager, Property 5: SIMD/Scalar Equivalence /// Validates: Requirements 4.5, 4.6

#[test]

fn prop_simd_scalar_equivalence( constraint in arb_version_constraint(), candidates in prop::collection::vec(arb_version(), 8..64)
) { let simd_result = unsafe { compare_versions_simd(&constraint, &candidates) };
let scalar_result = compare_versions_scalar(&constraint, &candidates);
prop_assert_eq!(simd_result, scalar_result);
}
}
```


### Unit Tests


Unit tests focus on specific examples and edge cases: -Empty package lists -Single package resolution -Maximum size packages (2GB limit) -Invalid format detection -Platform-specific behavior


### Integration Tests


- End-to-end installation workflow
- CLI command execution
- Compatibility with real PyPI packages
- Workspace operations


### Benchmark Tests


Performance validation against targets:
```rust

#[bench]

fn bench_lock_file_read(b: &mut Bencher) { let lock_file = create_test_lock_file(1000); // 1000 packages b.iter(|| { DplLockFile::open(&lock_file).unwrap()
});
// Target: < 0.1ms }

#[bench]

fn bench_version_resolution_simd(b: &mut Bencher) { let candidates: Vec<_> = (0..1000).map(|i| PackedVersion::new(1, i, 0)).collect();
let constraint = VersionConstraint::gte(PackedVersion::new(1, 500, 0));
b.iter(|| { find_best_version_simd(&constraint, &candidates)
});
// Target: < 1ms for 1000 versions }
```
