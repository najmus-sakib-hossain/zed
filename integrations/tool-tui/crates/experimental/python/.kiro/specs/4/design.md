
# Design Document: dx-py Performance Phase 1

## Overview

This design document specifies the implementation of Phase 1 performance optimizations for dx-py-package-manager, targeting 5x+ faster performance than uv. The three core features are: -O(1) Virtual Environment Layout Cache - Enables 100x faster warm installs through pre-built venv layouts -Binary Lock File (DPL Format) - Provides 5000x faster lock file operations through binary format with hash table lookup -Memory-Mapped Package Store - Delivers zero-copy package access with shared storage These features build upon the existing dx-py architecture, extending the current `GlobalCache`, `DplLockFile`, and `Installer` implementations.

## Architecture

@flow:TD[]

## Components and Interfaces

### Component 1: Layout Cache (`dx-py-layout`)

The Layout Cache provides O(1) virtual environment setup by caching pre-built venv layouts indexed by project hash.
```rust
/// Layout cache for O(1) virtual environment installation pub struct LayoutCache { /// Root directory for layouts (~/.dx-py/layouts/)
root: PathBuf, /// Memory-mapped index file (layouts.dxc)
index: LayoutIndex, /// Package store for shared packages store: Arc<PackageStore>, }
/// Binary index file for O(1) layout lookup pub struct LayoutIndex { /// Memory-mapped index data mmap: Option<Mmap>, /// Header with metadata header: LayoutIndexHeader, }
/// Layout index header (64 bytes)


#[repr(C, packed)]


pub struct LayoutIndexHeader { /// Magic: "DXLC"
magic: [u8; 4], /// Version version: u16, /// Number of layouts layout_count: u32, /// Hash table offset hash_table_offset: u32, /// Hash table size (slots)
hash_table_size: u32, /// Entries offset entries_offset: u32, /// Reserved _reserved: [u8; 42], }
/// Layout entry (128 bytes)


#[repr(C, packed)]


pub struct LayoutEntry { /// Project hash (Blake3)
project_hash: [u8; 32], /// Layout directory name (relative path)
layout_name: [u8; 64], /// Creation timestamp created_at: u64, /// Last accessed timestamp last_accessed: u64, /// Package count in layout package_count: u32, /// Total size in bytes total_size: u64, /// Reserved _reserved: [u8; 8], }
impl LayoutCache { /// Create or open layout cache pub fn open(root: PathBuf, store: Arc<PackageStore>) -> Result<Self>;
/// O(1) lookup for existing layout pub fn get(&self, project_hash: &[u8; 32]) -> Option<&LayoutEntry>;
/// Install from cached layout (single symlink/junction)
pub fn install_cached(&self, project_hash: &[u8; 32], target: &Path) -> Result<InstallResult>;
/// Build and cache a new layout pub fn build_layout(&mut self, project_hash: &[u8; 32], packages: &[ResolvedPackage]) -> Result<PathBuf>;
/// Compute project hash from resolved dependencies pub fn compute_project_hash(packages: &[ResolvedPackage]) -> [u8; 32];
/// Verify layout integrity pub fn verify_layout(&self, project_hash: &[u8; 32]) -> Result<bool>;
/// Rebuild corrupted layout pub fn rebuild_layout(&mut self, project_hash: &[u8; 32], packages: &[ResolvedPackage]) -> Result<PathBuf>;
}
```
Platform-Specific Linking:
```rust
/// Create filesystem link appropriate for platform pub fn create_layout_link(source: &Path, target: &Path) -> Result<()> {


#[cfg(unix)]


{ std::os::unix::fs::symlink(source, target)?;
}


#[cfg(windows)]


{ // Use junction for directories (no admin required)
junction::create(source, target)?;
}
Ok(())
}
```

### Component 2: Binary Lock File (DPL Format)

Extends the existing `DplLockFile` and `DplBuilder` with enhanced O(1) lookup capabilities.
```rust
/// Enhanced DPL entry with extras bitmap (128 bytes, same as existing)


#[repr(C, packed)]


pub struct DplEntryV2 { /// FNV-1a hash of package name pub name_hash: u64, /// Package name (null-terminated)
pub name: [u8; 48], /// Version (packed integers for fast comparison)
pub version_major: u16, pub version_minor: u16, pub version_patch: u16, /// Extras bitmap (up to 64 extras)
pub extras_bitmap: u64, /// Dependencies offset in deps section pub deps_offset: u32, /// Dependencies count pub deps_count: u16, /// Source type pub source_type: u8, /// Padding pub _padding: [u8; 5], /// Blake3 wheel hash pub wheel_hash: [u8; 32], }
/// DPL file reader with O(1) lookup impl DplLockFile { /// O(1) package lookup by name pub fn lookup(&self, package_name: &str) -> Option<DplEntry>;
/// Get all dependencies for a package pub fn get_dependencies(&self, entry: &DplEntry) -> Vec<&str>;
/// Verify integrity pub fn verify(&self) -> bool;
/// Check if package has specific extra pub fn has_extra(&self, entry: &DplEntry, extra_index: u8) -> bool;
}
/// DPL file builder impl DplBuilder { /// Create new builder pub fn new(python_version: &str, platform: &str) -> Self;
/// Add package with extras pub fn add_package_with_extras( &mut self, name: &str, version: &str, extras: &[&str], wheel_hash: [u8; 32], ) -> &mut Self;
/// Build to bytes pub fn build(&self) -> Vec<u8>;
/// Write to file pub fn write_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()>;
}
```

### Component 3: Memory-Mapped Package Store

```rust
/// Content-addressed package store with memory-mapped access pub struct PackageStore { /// Root directory (~/.dx-py/store/)
root: PathBuf, /// Memory-mapped packages (lazy loaded)
packages: DashMap<[u8; 32], Arc<MappedPackage>>, /// File index cache indices: DashMap<[u8; 32], PackageIndex>, }
/// Memory-mapped package data pub struct MappedPackage { /// Memory-mapped file mmap: Mmap, /// Package hash hash: [u8; 32], }
/// Package file index for O(1) file lookup


#[repr(C)]


pub struct PackageIndex { /// Number of files file_count: u32, /// File entries entries: Vec<PackageFileEntry>, }
/// File entry in package index


#[repr(C, packed)]


pub struct PackageFileEntry { /// Path hash for O(1) lookup path_hash: u64, /// Offset in package data offset: u64, /// File size size: u64, /// Path length path_len: u16, /// Path bytes (variable, up to 256)
path: [u8; 256], }
impl PackageStore { /// Create or open package store pub fn open(root: PathBuf) -> Result<Self>;
/// Get package path in store pub fn get_path(&self, hash: &[u8; 32]) -> PathBuf;
/// Check if package exists pub fn contains(&self, hash: &[u8; 32]) -> bool;
/// Get memory-mapped package (lazy load)
pub fn get(&self, hash: &[u8; 32]) -> Result<Arc<MappedPackage>>;
/// Get file slice from package (zero-copy)
pub fn get_file(&self, hash: &[u8; 32], file_path: &str) -> Result<&[u8]>;
/// Store package data pub fn store(&self, hash: &[u8; 32], data: &[u8]) -> Result<PathBuf>;
/// Store and verify hash pub fn store_verified(&self, expected_hash: &[u8; 32], data: &[u8]) -> Result<PathBuf>;
/// Install package to venv using symlinks pub fn install_to_venv(&self, hash: &[u8; 32], site_packages: &Path) -> Result<InstallResult>;
}
impl MappedPackage { /// Get file data by path (zero-copy slice)
pub fn get_file(&self, index: &PackageIndex, path: &str) -> Option<&[u8]>;
/// Iterate over all files pub fn files(&self, index: &PackageIndex) -> impl Iterator<Item = (&str, &[u8])>;
}
```
Directory Structure: @tree:~/.dx-py[]

## Data Models

### Layout Index Format (layouts.dxc)

@tree[]

### Package Store Format (.dxpkg)

@tree[]

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Layout Cache Hash Determinism

For any set of resolved packages, computing the project hash multiple times SHALL always produce the same hash value, and different package sets SHALL produce different hashes. Validates: Requirements 1.5

### Property 2: Layout Cache Cold-to-Warm Transition

For any valid package set, after building and caching a layout, subsequent lookups with the same project hash SHALL find the cached layout. Validates: Requirements 1.2

### Property 3: Layout Cache Corruption Recovery

For any cached layout, if files are corrupted or removed, the Layout_Cache SHALL detect the corruption during verification and successfully rebuild the layout. Validates: Requirements 1.7

### Property 4: Layout Cache Concurrent Access

For any sequence of concurrent read and write operations on the Layout_Cache from multiple threads, the cache SHALL remain consistent and no data corruption SHALL occur. Validates: Requirements 1.8

### Property 5: DPL Round-Trip Consistency

For any valid DPL structure containing packages with names, versions, extras, and hashes, serializing to binary format and deserializing back SHALL produce an equivalent structure. Validates: Requirements 2.8, 2.9, 2.10

### Property 6: DPL Entry Fields Correctness

For any package entry stored in a DPL file, the pre-computed name hash SHALL match the FNV-1a hash of the package name, version fields SHALL correctly represent the semantic version, extras bitmap SHALL correctly encode all extras, and wheel hash SHALL match the stored value. Validates: Requirements 2.4, 2.5, 2.6, 2.7

### Property 7: DPL Magic Bytes

For any DPL file generated by DplBuilder, the first 4 bytes SHALL be the magic bytes "DPL\x01". Validates: Requirements 2.1

### Property 8: DPL Corruption Handling

For any byte sequence that is not a valid DPL file (corrupted magic bytes, truncated data, invalid structure), the DPL_File_Reader SHALL return an appropriate error rather than panicking or returning invalid data. Validates: Requirements 2.11

### Property 9: Package Store Path Format

For any package stored in the Package_Store with hash H, the storage path SHALL be `{root}/{H[0:2]}/{H[2:4]}/{H}.dxpkg` where H is the hex-encoded Blake3 hash. Validates: Requirements 3.6

### Property 10: Package Store File Lookup

For any package in the store and any file path within that package, the Package_Store SHALL return the correct file contents via the file index lookup. Validates: Requirements 3.4

### Property 11: Package Store Symlink Installation

For any package installed from the Package_Store to a virtual environment, the installed files SHALL be symlinks (or junctions on Windows) pointing to the store, not copies. Validates: Requirements 3.5

### Property 12: Package Store Integrity Verification

For any package data and expected hash, storing with `store_verified` SHALL succeed only if the Blake3 hash of the data matches the expected hash, and SHALL fail with an error otherwise. Validates: Requirements 3.9

### Property 13: Package Store Concurrent Access

For any sequence of concurrent read operations on the Package_Store from multiple threads, all reads SHALL return correct data without corruption. Validates: Requirements 3.8

### Property 14: Package Store Deduplication

For any two projects that depend on the same package (same hash), the Package_Store SHALL store only one copy of the package data, and both projects SHALL reference the same store file. Validates: Requirements 3.10

### Property 15: Package Store Error Handling

For any hash that does not correspond to a package in the store, requesting that package SHALL return a "not found" error rather than panicking or returning invalid data. Validates: Requirements 3.7

## Error Handling

### Layout Cache Errors

+------------------+---------+----------+
| Error            | Cause   | Recovery |
+==================+=========+==========+
| `LayoutNotFound` | Project | hash     |
+------------------+---------+----------+



### DPL Errors

+----------------+-------+----------+
| Error          | Cause | Recovery |
+================+=======+==========+
| `InvalidMagic` | Wrong | magic    |
+----------------+-------+----------+



### Package Store Errors

+-------------------+-------+----------+
| Error             | Cause | Recovery |
+===================+=======+==========+
| `PackageNotFound` | Hash  | not      |
+-------------------+-------+----------+



## Testing Strategy

### Property-Based Testing Configuration

- Framework: `proptest` crate for Rust
- Minimum iterations: 100 per property test
- Shrinking: Enabled for finding minimal failing cases

### Unit Tests

Unit tests will cover: -Specific examples demonstrating correct behavior -Edge cases (empty package sets, maximum name lengths, special characters) -Error conditions (corrupted files, missing data) -Platform-specific behavior (symlinks vs junctions)

### Property Tests

Each correctness property will be implemented as a property-based test:
```rust
// Example: Property 5 - DPL Round-Trip


#[test]


fn prop_dpl_roundtrip() { // Feature: dx-py-performance-phase1, Property 5: DPL Round-Trip Consistency proptest!(|(packages in arb_package_list(1..100))| {
let mut builder = DplBuilder::new("3.12.0", "linux_x86_64");
for pkg in &packages { builder.add_package(&pkg.name, &pkg.version, pkg.hash);
}
let bytes = builder.build();
let lock_file = DplLockFile::from_bytes(bytes)?;
for pkg in &packages { let entry = lock_file.lookup(&pkg.name).unwrap();
prop_assert_eq!(entry.name_str(), pkg.name);
prop_assert_eq!(entry.version_str(), pkg.version);
}
});
}
```

### Test Generators

```rust
// Generator for valid package names (PEP 503 normalized)
fn arb_package_name() -> impl Strategy<Value = String> { "[a-z][a-z0-9_-]{0,46}".prop_map(|s| s.to_lowercase())
}
// Generator for valid versions (PEP 440)
fn arb_version() -> impl Strategy<Value = String> { (0u16..100, 0u16..100, 0u16..1000)
.prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
}
// Generator for package entries fn arb_package_entry() -> impl Strategy<Value = TestPackage> { (arb_package_name(), arb_version(), any::<[u8; 32]>())
.prop_map(|(name, version, hash)| TestPackage { name, version, hash })
}
// Generator for package lists fn arb_package_list(size: impl Into<SizeRange>) -> impl Strategy<Value = Vec<TestPackage>> { prop::collection::vec(arb_package_entry(), size)
}
```

### Integration Tests

Integration tests will verify: -CLI commands work correctly with the new performance features -Warm install path is used when cache is available -Fallback to standard installation works when cache is unavailable -Cache statistics are correctly reported

### Performance Benchmarks

Using `criterion` crate: -Warm install time (target: <10ms) -Cold install time (target: <1s for typical project) -DPL lookup time (target: <0.01ms) -Package store access time (target: <1ms)
