//! Memory Mapper
//!
//! Zero-copy file mapping for efficient scanning.

use crate::error::{Result, SecurityError};
use memmap2::Mmap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Memory-mapped file
pub struct MappedFile {
    /// Path to the mapped file
    pub path: PathBuf,
    /// Memory-mapped data
    pub data: Mmap,
    /// File size in bytes
    pub size: usize,
}

/// Memory-mapped directory
pub struct MappedDirectory {
    /// Files in the directory
    pub files: Vec<MappedFile>,
}

/// Mapping statistics
#[derive(Debug, Clone, Default)]
pub struct MapStats {
    pub files_mapped: usize,
    pub total_bytes: usize,
    pub failed_maps: usize,
}

/// Thread-safe mapping statistics for parallel operations
struct AtomicMapStats {
    files_mapped: AtomicUsize,
    total_bytes: AtomicUsize,
    failed_maps: AtomicUsize,
}

impl AtomicMapStats {
    fn new() -> Self {
        Self {
            files_mapped: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
            failed_maps: AtomicUsize::new(0),
        }
    }

    fn to_map_stats(&self) -> MapStats {
        MapStats {
            files_mapped: self.files_mapped.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            failed_maps: self.failed_maps.load(Ordering::Relaxed),
        }
    }
}

/// Memory mapper for zero-copy file access
pub struct MemoryMapper {
    stats: MapStats,
}

impl MemoryMapper {
    /// Create a new memory mapper
    pub fn new() -> Self {
        Self {
            stats: MapStats::default(),
        }
    }

    /// Map a file into memory
    pub fn map_file(&mut self, path: &Path) -> Result<MappedFile> {
        let file = std::fs::File::open(path).map_err(|e| SecurityError::MapError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| SecurityError::MapError {
                path: path.to_path_buf(),
                source: e,
            })?
        };

        let size = mmap.len();
        self.stats.files_mapped += 1;
        self.stats.total_bytes += size;

        Ok(MappedFile {
            path: path.to_path_buf(),
            data: mmap,
            size,
        })
    }

    /// Map a single file (static version for parallel use)
    fn map_file_static(path: &Path, stats: &AtomicMapStats) -> Option<MappedFile> {
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => {
                stats.failed_maps.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        let mmap = match unsafe { Mmap::map(&file) } {
            Ok(m) => m,
            Err(_) => {
                stats.failed_maps.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };

        let size = mmap.len();
        stats.files_mapped.fetch_add(1, Ordering::Relaxed);
        stats.total_bytes.fetch_add(size, Ordering::Relaxed);

        Some(MappedFile {
            path: path.to_path_buf(),
            data: mmap,
            size,
        })
    }

    /// Collect all file paths in a directory recursively
    fn collect_file_paths(dir: &Path) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        fn walk(dir: &Path, paths: &mut Vec<PathBuf>) {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        walk(&path, paths);
                    } else if path.is_file() {
                        paths.push(path);
                    }
                }
            }
        }

        walk(dir, &mut paths);
        paths
    }

    /// Map entire directory recursively (sequential version)
    pub fn map_directory(&mut self, path: &Path) -> Result<MappedDirectory> {
        let mut files = Vec::new();

        fn walk_dir(
            mapper: &mut MemoryMapper,
            dir: &Path,
            files: &mut Vec<MappedFile>,
        ) -> Result<()> {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    walk_dir(mapper, &path, files)?;
                } else if path.is_file() {
                    match mapper.map_file(&path) {
                        Ok(mapped) => files.push(mapped),
                        Err(_) => mapper.stats.failed_maps += 1,
                    }
                }
            }
            Ok(())
        }

        walk_dir(self, path, &mut files)?;
        Ok(MappedDirectory { files })
    }

    /// Map entire directory recursively using parallel processing
    #[cfg(feature = "parallel")]
    pub fn map_directory_parallel(&mut self, path: &Path) -> Result<MappedDirectory> {
        use rayon::prelude::*;

        let file_paths = Self::collect_file_paths(path);
        let stats = Arc::new(AtomicMapStats::new());

        let files: Vec<MappedFile> =
            file_paths.par_iter().filter_map(|p| Self::map_file_static(p, &stats)).collect();

        self.stats = stats.to_map_stats();
        Ok(MappedDirectory { files })
    }

    /// Map entire directory recursively using parallel processing (fallback when rayon not available)
    #[cfg(not(feature = "parallel"))]
    pub fn map_directory_parallel(&mut self, path: &Path) -> Result<MappedDirectory> {
        // Fall back to sequential when rayon is not available
        self.map_directory(path)
    }

    /// Get mapping statistics
    pub fn stats(&self) -> &MapStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = MapStats::default();
    }
}

impl Default for MemoryMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_map_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, b"Hello, World!").unwrap();

        let mut mapper = MemoryMapper::new();
        let mapped = mapper.map_file(&file_path).unwrap();

        assert_eq!(mapped.size, 13);
        assert_eq!(&mapped.data[..], b"Hello, World!");
    }

    #[test]
    fn test_map_file_not_found() {
        let mut mapper = MemoryMapper::new();
        let result = mapper.map_file(Path::new("/nonexistent/file.txt"));

        assert!(result.is_err());
    }

    #[test]
    fn test_map_directory() {
        let dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::write(dir.path().join("file1.txt"), b"Content 1").unwrap();
        std::fs::write(dir.path().join("file2.txt"), b"Content 2").unwrap();

        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("file3.txt"), b"Content 3").unwrap();

        let mut mapper = MemoryMapper::new();
        let mapped_dir = mapper.map_directory(dir.path()).unwrap();

        assert_eq!(mapped_dir.files.len(), 3);
        assert_eq!(mapper.stats().files_mapped, 3);
    }

    #[test]
    fn test_stats_tracking() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file1.txt"), b"12345").unwrap();
        std::fs::write(dir.path().join("file2.txt"), b"1234567890").unwrap();

        let mut mapper = MemoryMapper::new();
        mapper.map_directory(dir.path()).unwrap();

        let stats = mapper.stats();
        assert_eq!(stats.files_mapped, 2);
        assert_eq!(stats.total_bytes, 15);
        assert_eq!(stats.failed_maps, 0);
    }
}
