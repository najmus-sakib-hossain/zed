//! Content-addressed package store with memory-mapped access

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use memmap2::Mmap;

use crate::index::{PackageIndex, PackageIndexHeader};
use crate::mapped::MappedPackage;
use crate::{StoreError, StoreResult, DXPK_MAGIC, MAX_PACKAGE_SIZE, STORE_VERSION};

/// Installation result statistics
#[derive(Debug, Default, Clone)]
pub struct InstallResult {
    /// Number of files installed
    pub files_installed: u64,
    /// Number of symlinks created
    pub symlinks: u64,
    /// Number of files copied (fallback)
    pub copies: u64,
    /// Total bytes installed
    pub bytes_installed: u64,
}

/// Content-addressed package store with memory-mapped access
pub struct PackageStore {
    /// Root directory (~/.dx-py/store/)
    root: PathBuf,
    /// Memory-mapped packages (lazy loaded)
    packages: DashMap<[u8; 32], Arc<MappedPackage>>,
    /// File index cache
    indices: DashMap<[u8; 32], PackageIndex>,
}

impl PackageStore {
    /// Create or open package store at the given root directory
    pub fn open<P: AsRef<Path>>(root: P) -> StoreResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        Ok(Self {
            root,
            packages: DashMap::new(),
            indices: DashMap::new(),
        })
    }

    /// Get the store root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Compute the cache path for a given content hash
    ///
    /// Uses a two-level directory structure:
    /// store_root/{hash[0:2]}/{hash[2:4]}/{hash}.dxpkg
    pub fn get_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex = hex::encode(hash);
        self.root.join(&hex[0..2]).join(&hex[2..4]).join(format!("{}.dxpkg", hex))
    }

    /// Check if a package with the given hash exists in the store
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.get_path(hash).exists()
    }

    /// Store package data with the given hash
    ///
    /// Returns the path where the data was stored.
    /// If data with this hash already exists, returns the existing path.
    pub fn store(&self, hash: &[u8; 32], data: &[u8]) -> StoreResult<PathBuf> {
        let path = self.get_path(hash);

        // If already exists, just return the path (deduplication)
        if path.exists() {
            return Ok(path);
        }

        // Check size limit
        if data.len() as u64 > MAX_PACKAGE_SIZE {
            return Err(StoreError::PackageTooLarge {
                size: data.len() as u64,
                limit: MAX_PACKAGE_SIZE,
            });
        }

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically using a temp file
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, data)?;
        fs::rename(&temp_path, &path)?;

        Ok(path)
    }

    /// Store package data and verify the hash matches
    pub fn store_verified(&self, expected_hash: &[u8; 32], data: &[u8]) -> StoreResult<PathBuf> {
        // Compute hash of data
        let computed = blake3::hash(data);
        let computed_bytes: [u8; 32] = *computed.as_bytes();

        if &computed_bytes != expected_hash {
            return Err(StoreError::IntegrityError {
                expected: hex::encode(expected_hash),
                actual: hex::encode(computed_bytes),
            });
        }

        self.store(expected_hash, data)
    }

    /// Get raw data from the store by hash
    pub fn get_raw(&self, hash: &[u8; 32]) -> StoreResult<Vec<u8>> {
        let path = self.get_path(hash);
        if !path.exists() {
            return Err(StoreError::PackageNotFound(hex::encode(hash)));
        }
        Ok(fs::read(&path)?)
    }

    /// Get memory-mapped package (lazy load)
    pub fn get(&self, hash: &[u8; 32]) -> StoreResult<Arc<MappedPackage>> {
        // Check cache first
        if let Some(pkg) = self.packages.get(hash) {
            return Ok(Arc::clone(&pkg));
        }

        // Load from disk
        let path = self.get_path(hash);
        if !path.exists() {
            return Err(StoreError::PackageNotFound(hex::encode(hash)));
        }

        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file).map_err(|e| StoreError::MmapFailed(e.to_string()))? };

        // Verify magic and version
        if mmap.len() < std::mem::size_of::<PackageIndexHeader>() {
            return Err(StoreError::IndexCorrupted("file too small".to_string()));
        }

        let header: PackageIndexHeader =
            *bytemuck::from_bytes(&mmap[0..std::mem::size_of::<PackageIndexHeader>()]);

        if &header.magic != DXPK_MAGIC {
            return Err(StoreError::InvalidMagic {
                expected: *DXPK_MAGIC,
                found: header.magic,
            });
        }

        if header.version != STORE_VERSION {
            return Err(StoreError::UnsupportedVersion(header.version));
        }

        let mapped = Arc::new(MappedPackage::new(mmap, *hash, header.data_offset));

        // Cache it
        self.packages.insert(*hash, Arc::clone(&mapped));

        Ok(mapped)
    }

    /// Get the package index for a hash
    pub fn get_index(&self, hash: &[u8; 32]) -> StoreResult<PackageIndex> {
        // Check cache first
        if let Some(index) = self.indices.get(hash) {
            return Ok(index.clone());
        }

        // Load from disk
        let path = self.get_path(hash);
        if !path.exists() {
            return Err(StoreError::PackageNotFound(hex::encode(hash)));
        }

        let data = fs::read(&path)?;

        // Parse header
        if data.len() < std::mem::size_of::<PackageIndexHeader>() {
            return Err(StoreError::IndexCorrupted("file too small".to_string()));
        }

        let header: PackageIndexHeader =
            *bytemuck::from_bytes(&data[0..std::mem::size_of::<PackageIndexHeader>()]);

        // Parse index
        let index_start = header.index_offset as usize;
        let index_bytes = &data[index_start..];
        let index = PackageIndex::from_bytes(index_bytes, header.file_count)
            .ok_or_else(|| StoreError::IndexCorrupted("failed to parse index".to_string()))?;

        // Cache it
        self.indices.insert(*hash, index.clone());

        Ok(index)
    }

    /// Get file data from a package (zero-copy when possible)
    pub fn get_file(&self, hash: &[u8; 32], file_path: &str) -> StoreResult<Vec<u8>> {
        let mapped = self.get(hash)?;
        let index = self.get_index(hash)?;

        mapped
            .get_file(&index, file_path)
            .map(|data| data.to_vec())
            .ok_or_else(|| StoreError::FileNotFound(file_path.to_string()))
    }

    /// Store a package with files
    ///
    /// Creates a .dxpkg file with header, file data, and index.
    pub fn store_package(&self, hash: &[u8; 32], files: &[(&str, &[u8])]) -> StoreResult<PathBuf> {
        let path = self.get_path(hash);

        // If already exists, return existing path
        if path.exists() {
            return Ok(path);
        }

        // Build the package
        let header_size = std::mem::size_of::<PackageIndexHeader>();
        let mut data_section = Vec::new();
        let mut index = PackageIndex::new();

        // Write file data and build index
        for (file_path, content) in files {
            let offset = data_section.len() as u64;
            data_section.extend_from_slice(content);
            index.add_file(file_path, offset, content.len() as u64);
        }

        let index_bytes = index.to_bytes();
        let data_offset = header_size as u64;
        let index_offset = data_offset + data_section.len() as u64;
        let total_size = index_offset + index_bytes.len() as u64;

        // Build header
        let header =
            PackageIndexHeader::new(index.file_count(), index_offset, data_offset, total_size);

        // Assemble final package
        let mut package = Vec::with_capacity(total_size as usize);
        package.extend_from_slice(bytemuck::bytes_of(&header));
        package.extend_from_slice(&data_section);
        package.extend_from_slice(&index_bytes);

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write atomically
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &package)?;
        fs::rename(&temp_path, &path)?;

        Ok(path)
    }

    /// Install package to virtual environment using symlinks
    pub fn install_to_venv(
        &self,
        hash: &[u8; 32],
        site_packages: &Path,
    ) -> StoreResult<InstallResult> {
        let mapped = self.get(hash)?;
        let index = self.get_index(hash)?;
        let mut result = InstallResult::default();

        // Get the extracted directory path
        let store_path = self.get_path(hash);
        let extracted_dir = store_path.with_extension("extracted");

        // Extract if not already extracted
        if !extracted_dir.exists() {
            self.extract_package(hash, &extracted_dir)?;
        }

        // Create symlinks for each file
        for entry in index.iter() {
            let file_path = entry.path_str();
            let src = extracted_dir.join(file_path);
            let dst = site_packages.join(file_path);

            // Create parent directories
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }

            // Try symlink first, fall back to copy
            if src.exists() {
                match create_symlink(&src, &dst) {
                    Ok(()) => {
                        result.symlinks += 1;
                    }
                    Err(_) => {
                        // Fall back to copy
                        if let Some(data) = mapped.get_file(&index, file_path) {
                            fs::write(&dst, data)?;
                            result.copies += 1;
                        }
                    }
                }
            } else if let Some(data) = mapped.get_file(&index, file_path) {
                fs::write(&dst, data)?;
                result.copies += 1;
            }

            result.files_installed += 1;
            result.bytes_installed += entry.size;
        }

        Ok(result)
    }

    /// Extract package to a directory
    fn extract_package(&self, hash: &[u8; 32], target: &Path) -> StoreResult<()> {
        let mapped = self.get(hash)?;
        let index = self.get_index(hash)?;

        fs::create_dir_all(target)?;

        for entry in index.iter() {
            let file_path = entry.path_str();
            let dst = target.join(file_path);

            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }

            if let Some(data) = mapped.get_file(&index, file_path) {
                fs::write(&dst, data)?;
            }
        }

        Ok(())
    }

    /// Remove a package from the store
    pub fn remove(&self, hash: &[u8; 32]) -> StoreResult<bool> {
        let path = self.get_path(hash);

        // Remove from caches
        self.packages.remove(hash);
        self.indices.remove(hash);

        if path.exists() {
            fs::remove_file(&path)?;

            // Also remove extracted directory if exists
            let extracted = path.with_extension("extracted");
            if extracted.exists() {
                fs::remove_dir_all(&extracted)?;
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all package hashes in the store
    pub fn list(&self) -> StoreResult<Vec<[u8; 32]>> {
        let mut hashes = Vec::new();
        Self::list_recursive(&self.root, &mut hashes)?;
        Ok(hashes)
    }

    fn list_recursive(dir: &Path, hashes: &mut Vec<[u8; 32]>) -> StoreResult<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::list_recursive(&path, hashes)?;
            } else if path.is_file() {
                if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                    if name.len() == 64 {
                        if let Ok(bytes) = hex::decode(name) {
                            if bytes.len() == 32 {
                                let mut hash = [0u8; 32];
                                hash.copy_from_slice(&bytes);
                                hashes.push(hash);
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Create a symlink (platform-specific)
#[cfg(unix)]
fn create_symlink(src: &Path, dst: &Path) -> StoreResult<()> {
    std::os::unix::fs::symlink(src, dst).map_err(|e| {
        StoreError::SymlinkFailed(format!("{} -> {}: {}", src.display(), dst.display(), e))
    })
}

#[cfg(windows)]
fn create_symlink(src: &Path, dst: &Path) -> StoreResult<()> {
    // On Windows, use junction for directories, symlink for files
    if src.is_dir() {
        junction::create(src, dst).map_err(|e| {
            StoreError::SymlinkFailed(format!("{} -> {}: {}", src.display(), dst.display(), e))
        })
    } else {
        std::os::windows::fs::symlink_file(src, dst).map_err(|e| {
            StoreError::SymlinkFailed(format!("{} -> {}: {}", src.display(), dst.display(), e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_path_format() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let hash = [
            0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
            0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01,
            0x23, 0x45, 0x67, 0x89,
        ];

        let path = store.get_path(&hash);
        let path_str = path.to_string_lossy();

        // Should have two-level directory structure
        assert!(path_str.contains("ab"));
        assert!(path_str.contains("cd"));
        assert!(path_str.ends_with(".dxpkg"));
    }

    #[test]
    fn test_store_and_retrieve() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let files = vec![
            ("package/__init__.py", b"# init" as &[u8]),
            ("package/module.py", b"def hello(): pass"),
        ];

        // Compute hash
        let mut hasher = blake3::Hasher::new();
        for (path, content) in &files {
            hasher.update(path.as_bytes());
            hasher.update(content);
        }
        let hash = *hasher.finalize().as_bytes();

        // Store
        store.store_package(&hash, &files).unwrap();
        assert!(store.contains(&hash));

        // Retrieve
        let content = store.get_file(&hash, "package/__init__.py").unwrap();
        assert_eq!(content, b"# init");

        let content = store.get_file(&hash, "package/module.py").unwrap();
        assert_eq!(content, b"def hello(): pass");
    }

    #[test]
    fn test_store_verified() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let data = b"test data";
        let correct_hash = *blake3::hash(data).as_bytes();
        let wrong_hash = [0u8; 32];

        // Correct hash should work
        store.store_verified(&correct_hash, data).unwrap();

        // Wrong hash should fail
        let result = store.store_verified(&wrong_hash, data);
        assert!(matches!(result, Err(StoreError::IntegrityError { .. })));
    }

    #[test]
    fn test_store_deduplication() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let files = vec![("file.py", b"content" as &[u8])];
        let hash = *blake3::hash(b"content").as_bytes();

        // Store twice
        let path1 = store.store_package(&hash, &files).unwrap();
        let path2 = store.store_package(&hash, &files).unwrap();

        // Should return same path
        assert_eq!(path1, path2);

        // Should only have one file
        let hashes = store.list().unwrap();
        assert_eq!(hashes.len(), 1);
    }

    #[test]
    fn test_package_not_found() {
        let temp = TempDir::new().unwrap();
        let store = PackageStore::open(temp.path()).unwrap();

        let hash = [0u8; 32];
        let result = store.get(&hash);

        assert!(matches!(result, Err(StoreError::PackageNotFound(_))));
    }
}
