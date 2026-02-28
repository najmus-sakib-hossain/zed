//! Layout cache for O(1) virtual environment installation

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dx_py_store::{InstallResult, PackageStore};

use crate::headers::LayoutEntry;
use crate::index::LayoutIndex;
use crate::{LayoutError, LayoutResult};

/// Resolved package information for layout building
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Content hash (Blake3)
    pub hash: [u8; 32],
}

/// Layout cache for O(1) virtual environment installation
pub struct LayoutCache {
    /// Root directory for layouts (~/.dx-py/layouts/)
    root: PathBuf,
    /// Layout index
    index: LayoutIndex,
    /// Package store
    store: Arc<PackageStore>,
}

impl LayoutCache {
    /// Open or create a layout cache
    pub fn open<P: AsRef<Path>>(root: P, store: Arc<PackageStore>) -> LayoutResult<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        let index_path = root.join("layouts.dxc");
        let index = LayoutIndex::open(&index_path)?;

        Ok(Self { root, index, store })
    }

    /// Get the cache root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Compute project hash from resolved dependencies
    ///
    /// The hash is computed from sorted package names, versions, and hashes
    /// to ensure deterministic results.
    pub fn compute_project_hash(packages: &[ResolvedPackage]) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();

        // Sort packages by name for deterministic hashing
        let mut sorted: Vec<_> = packages.iter().collect();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));

        for pkg in sorted {
            hasher.update(pkg.name.as_bytes());
            hasher.update(b":");
            hasher.update(pkg.version.as_bytes());
            hasher.update(b":");
            hasher.update(&pkg.hash);
            hasher.update(b"\n");
        }

        *hasher.finalize().as_bytes()
    }

    /// Check if a layout exists for the given project hash
    pub fn contains(&self, project_hash: &[u8; 32]) -> bool {
        self.index.contains(project_hash)
    }

    /// Get layout entry for project hash
    pub fn get(&self, project_hash: &[u8; 32]) -> Option<LayoutEntry> {
        self.index.get(project_hash)
    }

    /// Get the layout directory path
    pub fn get_layout_path(&self, project_hash: &[u8; 32]) -> PathBuf {
        // Use first 16 bytes of hash for layout name (32 hex chars)
        let hex = hex::encode(&project_hash[..16]);
        self.root.join(&hex)
    }

    /// Install from cached layout (single symlink/junction)
    pub fn install_cached(
        &self,
        project_hash: &[u8; 32],
        target: &Path,
    ) -> LayoutResult<InstallResult> {
        let entry = self
            .index
            .get(project_hash)
            .ok_or_else(|| LayoutError::LayoutNotFound(hex::encode(project_hash)))?;

        let layout_path = self.root.join(entry.layout_name_str());
        let site_packages = layout_path.join("site-packages");

        if !site_packages.exists() {
            return Err(LayoutError::LayoutCorrupted(
                "site-packages directory missing".to_string(),
            ));
        }

        // Create target directory
        fs::create_dir_all(target)?;

        let target_site_packages = target.join("site-packages");

        // Create single symlink/junction to the cached layout
        create_layout_link(&site_packages, &target_site_packages)?;

        Ok(InstallResult {
            files_installed: entry.package_count as u64,
            symlinks: 1,
            copies: 0,
            bytes_installed: entry.total_size,
        })
    }

    /// Build and cache a new layout
    pub fn build_layout(
        &mut self,
        project_hash: &[u8; 32],
        packages: &[ResolvedPackage],
    ) -> LayoutResult<PathBuf> {
        // Use first 16 bytes of hash for layout name (32 hex chars)
        // This fits in the 63-char limit while being unique enough
        let layout_name = hex::encode(&project_hash[..16]);
        let layout_path = self.root.join(&layout_name);
        let site_packages = layout_path.join("site-packages");

        // Create layout directory
        fs::create_dir_all(&site_packages)?;

        let mut total_size = 0u64;

        // Install each package to the layout
        for pkg in packages {
            let result = self.store.install_to_venv(&pkg.hash, &site_packages)?;
            total_size += result.bytes_installed;
        }

        // Add to index
        let entry =
            LayoutEntry::new(*project_hash, &layout_name, packages.len() as u32, total_size);
        self.index.add(entry)?;

        Ok(layout_path)
    }

    /// Verify layout integrity
    pub fn verify_layout(&self, project_hash: &[u8; 32]) -> LayoutResult<bool> {
        let entry = match self.index.get(project_hash) {
            Some(e) => e,
            None => return Ok(false),
        };

        let layout_path = self.root.join(entry.layout_name_str());
        let site_packages = layout_path.join("site-packages");

        // Check if directory exists
        // Note: An empty site-packages is valid for projects with no packages
        Ok(site_packages.exists())
    }

    /// Rebuild a corrupted layout
    pub fn rebuild_layout(
        &mut self,
        project_hash: &[u8; 32],
        packages: &[ResolvedPackage],
    ) -> LayoutResult<PathBuf> {
        // Remove existing layout
        if let Some(entry) = self.index.get(project_hash) {
            let layout_path = self.root.join(entry.layout_name_str());
            if layout_path.exists() {
                fs::remove_dir_all(&layout_path)?;
            }
            self.index.remove(project_hash)?;
        }

        // Build fresh
        self.build_layout(project_hash, packages)
    }

    /// Remove a layout from the cache
    pub fn remove(&mut self, project_hash: &[u8; 32]) -> LayoutResult<bool> {
        if let Some(entry) = self.index.get(project_hash) {
            let layout_path = self.root.join(entry.layout_name_str());
            if layout_path.exists() {
                fs::remove_dir_all(&layout_path)?;
            }
            self.index.remove(project_hash)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get layout count
    pub fn layout_count(&self) -> u32 {
        self.index.layout_count()
    }

    /// Iterate over all layouts
    pub fn iter(&self) -> impl Iterator<Item = LayoutEntry> + '_ {
        self.index.iter()
    }
}

/// Create filesystem link appropriate for platform
#[cfg(unix)]
fn create_layout_link(source: &Path, target: &Path) -> LayoutResult<()> {
    std::os::unix::fs::symlink(source, target).map_err(|e| {
        LayoutError::LinkFailed(format!("{} -> {}: {}", source.display(), target.display(), e))
    })
}

#[cfg(windows)]
fn create_layout_link(source: &Path, target: &Path) -> LayoutResult<()> {
    // Use junction for directories (no admin required)
    junction::create(source, target).map_err(|e| {
        LayoutError::LinkFailed(format!("{} -> {}: {}", source.display(), target.display(), e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store(temp: &TempDir) -> Arc<PackageStore> {
        Arc::new(PackageStore::open(temp.path().join("store")).unwrap())
    }

    #[test]
    fn test_compute_project_hash_deterministic() {
        let packages = vec![
            ResolvedPackage {
                name: "requests".to_string(),
                version: "2.31.0".to_string(),
                hash: [1u8; 32],
            },
            ResolvedPackage {
                name: "numpy".to_string(),
                version: "1.26.0".to_string(),
                hash: [2u8; 32],
            },
        ];

        let hash1 = LayoutCache::compute_project_hash(&packages);
        let hash2 = LayoutCache::compute_project_hash(&packages);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_project_hash_order_independent() {
        let packages1 = vec![
            ResolvedPackage {
                name: "requests".to_string(),
                version: "2.31.0".to_string(),
                hash: [1u8; 32],
            },
            ResolvedPackage {
                name: "numpy".to_string(),
                version: "1.26.0".to_string(),
                hash: [2u8; 32],
            },
        ];

        let packages2 = vec![
            ResolvedPackage {
                name: "numpy".to_string(),
                version: "1.26.0".to_string(),
                hash: [2u8; 32],
            },
            ResolvedPackage {
                name: "requests".to_string(),
                version: "2.31.0".to_string(),
                hash: [1u8; 32],
            },
        ];

        let hash1 = LayoutCache::compute_project_hash(&packages1);
        let hash2 = LayoutCache::compute_project_hash(&packages2);

        // Order should not matter due to sorting
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_project_hash_different_packages() {
        let packages1 = vec![ResolvedPackage {
            name: "requests".to_string(),
            version: "2.31.0".to_string(),
            hash: [1u8; 32],
        }];

        let packages2 = vec![ResolvedPackage {
            name: "numpy".to_string(),
            version: "1.26.0".to_string(),
            hash: [2u8; 32],
        }];

        let hash1 = LayoutCache::compute_project_hash(&packages1);
        let hash2 = LayoutCache::compute_project_hash(&packages2);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_layout_cache_open() {
        let temp = TempDir::new().unwrap();
        let store = create_test_store(&temp);

        let cache = LayoutCache::open(temp.path().join("layouts"), store).unwrap();
        assert_eq!(cache.layout_count(), 0);
    }
}
