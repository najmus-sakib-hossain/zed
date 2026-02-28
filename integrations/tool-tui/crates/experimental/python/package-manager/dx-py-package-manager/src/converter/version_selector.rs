//! Multi-version bytecode selection for DPP packages
//!
//! Supports multiple Python version targets and selects appropriate bytecode at runtime.

use super::bytecode::PythonVersion;
use super::loader::LoadedBytecode;
use std::collections::HashMap;

/// Multi-version bytecode store
///
/// Stores bytecode compiled for multiple Python versions and provides
/// version-aware selection at runtime.
#[derive(Debug, Default)]
pub struct MultiVersionStore {
    /// Bytecode entries grouped by source path
    /// Each source file can have multiple bytecode versions
    entries: HashMap<String, Vec<LoadedBytecode>>,
}

impl MultiVersionStore {
    /// Create a new multi-version store
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Add a bytecode entry
    pub fn add(&mut self, entry: LoadedBytecode) {
        self.entries.entry(entry.source_path.clone()).or_default().push(entry);
    }

    /// Get all bytecode versions for a source path
    pub fn get_all(&self, source_path: &str) -> Option<&[LoadedBytecode]> {
        self.entries.get(source_path).map(|v| v.as_slice())
    }

    /// Get the best bytecode for a source path and target version
    pub fn get_best(&self, source_path: &str, target: &PythonVersion) -> Option<&LoadedBytecode> {
        let entries = self.entries.get(source_path)?;
        Self::select_best(entries, target)
    }

    /// Select the best bytecode from a list of entries
    fn select_best<'a>(
        entries: &'a [LoadedBytecode],
        target: &PythonVersion,
    ) -> Option<&'a LoadedBytecode> {
        entries
            .iter()
            .filter(|e| e.is_compatible(target))
            .max_by_key(|e| (e.python_version.major, e.python_version.minor))
    }

    /// Get all source paths
    pub fn source_paths(&self) -> impl Iterator<Item = &String> {
        self.entries.keys()
    }

    /// Get the number of source files
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all supported Python versions
    pub fn supported_versions(&self) -> Vec<PythonVersion> {
        let mut versions: Vec<PythonVersion> =
            self.entries.values().flatten().map(|e| e.python_version).collect();

        versions.sort_by_key(|v| (v.major, v.minor, v.patch));
        versions.dedup();
        versions
    }

    /// Check if a specific Python version is supported
    pub fn supports_version(&self, version: &PythonVersion) -> bool {
        self.entries.values().flatten().any(|e| e.is_compatible(version))
    }

    /// Get bytecode coverage statistics
    pub fn coverage_stats(&self) -> CoverageStats {
        let total_files = self.entries.len();
        let mut version_counts: HashMap<(u8, u8), usize> = HashMap::new();

        for entries in self.entries.values() {
            for entry in entries {
                *version_counts
                    .entry((entry.python_version.major, entry.python_version.minor))
                    .or_default() += 1;
            }
        }

        CoverageStats {
            total_files,
            version_counts,
        }
    }
}

/// Bytecode coverage statistics
#[derive(Debug)]
pub struct CoverageStats {
    /// Total number of source files
    pub total_files: usize,
    /// Number of bytecode entries per Python version
    pub version_counts: HashMap<(u8, u8), usize>,
}

impl CoverageStats {
    /// Get coverage percentage for a specific version
    pub fn coverage_for_version(&self, major: u8, minor: u8) -> f64 {
        if self.total_files == 0 {
            return 0.0;
        }
        let count = self.version_counts.get(&(major, minor)).copied().unwrap_or(0);
        (count as f64 / self.total_files as f64) * 100.0
    }

    /// Check if all files have bytecode for a version
    pub fn is_complete_for_version(&self, major: u8, minor: u8) -> bool {
        self.version_counts.get(&(major, minor)).copied().unwrap_or(0) == self.total_files
    }
}

/// Version selector for runtime bytecode selection
pub struct VersionSelector {
    /// Current runtime Python version
    runtime_version: PythonVersion,
    /// Fallback behavior when no compatible bytecode is found
    fallback: FallbackBehavior,
}

/// Fallback behavior when no compatible bytecode is found
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FallbackBehavior {
    /// Use source compilation (default)
    #[default]
    CompileFromSource,
    /// Return error
    Error,
    /// Use nearest version (may cause issues)
    UseNearest,
}

impl VersionSelector {
    /// Create a new version selector
    pub fn new(runtime_version: PythonVersion) -> Self {
        Self {
            runtime_version,
            fallback: FallbackBehavior::default(),
        }
    }

    /// Set fallback behavior
    pub fn with_fallback(mut self, fallback: FallbackBehavior) -> Self {
        self.fallback = fallback;
        self
    }

    /// Get the runtime version
    pub fn runtime_version(&self) -> &PythonVersion {
        &self.runtime_version
    }

    /// Select bytecode for a source file
    pub fn select<'a>(
        &self,
        store: &'a MultiVersionStore,
        source_path: &str,
    ) -> SelectionResult<'a> {
        match store.get_best(source_path, &self.runtime_version) {
            Some(bytecode) => SelectionResult::Found(bytecode),
            None => match self.fallback {
                FallbackBehavior::CompileFromSource => SelectionResult::NeedCompilation,
                FallbackBehavior::Error => SelectionResult::NotFound,
                FallbackBehavior::UseNearest => {
                    // Try to find the nearest version
                    if let Some(entries) = store.get_all(source_path) {
                        if let Some(nearest) = Self::find_nearest(entries, &self.runtime_version) {
                            return SelectionResult::NearestVersion(nearest);
                        }
                    }
                    SelectionResult::NotFound
                }
            },
        }
    }

    /// Find the nearest version (closest minor version)
    fn find_nearest<'a>(
        entries: &'a [LoadedBytecode],
        target: &PythonVersion,
    ) -> Option<&'a LoadedBytecode> {
        entries
            .iter()
            .filter(|e| e.python_version.major == target.major)
            .min_by_key(|e| (e.python_version.minor as i32 - target.minor as i32).abs())
    }
}

/// Result of bytecode selection
#[derive(Debug)]
pub enum SelectionResult<'a> {
    /// Compatible bytecode found
    Found(&'a LoadedBytecode),
    /// No compatible bytecode, need to compile from source
    NeedCompilation,
    /// Using nearest version (may not be fully compatible)
    NearestVersion(&'a LoadedBytecode),
    /// No bytecode found and fallback is Error
    NotFound,
}

impl<'a> SelectionResult<'a> {
    /// Check if bytecode was found
    pub fn is_found(&self) -> bool {
        matches!(self, SelectionResult::Found(_) | SelectionResult::NearestVersion(_))
    }

    /// Get the bytecode if found
    pub fn bytecode(&self) -> Option<&'a LoadedBytecode> {
        match self {
            SelectionResult::Found(b) | SelectionResult::NearestVersion(b) => Some(b),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(path: &str, major: u8, minor: u8) -> LoadedBytecode {
        LoadedBytecode::new(
            path.to_string(),
            [0u8; 32],
            PythonVersion::new(major, minor, 0),
            vec![],
        )
    }

    #[test]
    fn test_multi_version_store() {
        let mut store = MultiVersionStore::new();

        store.add(make_entry("test.py", 3, 10));
        store.add(make_entry("test.py", 3, 11));
        store.add(make_entry("test.py", 3, 12));

        assert_eq!(store.len(), 1);
        assert_eq!(store.get_all("test.py").unwrap().len(), 3);

        // Should select 3.12 for Python 3.12
        let best = store.get_best("test.py", &PythonVersion::new(3, 12, 0));
        assert!(best.is_some());
        assert_eq!(best.unwrap().python_version.minor, 12);

        // Should select 3.11 for Python 3.11
        let best = store.get_best("test.py", &PythonVersion::new(3, 11, 0));
        assert!(best.is_some());
        assert_eq!(best.unwrap().python_version.minor, 11);
    }

    #[test]
    fn test_supported_versions() {
        let mut store = MultiVersionStore::new();

        store.add(make_entry("a.py", 3, 10));
        store.add(make_entry("a.py", 3, 12));
        store.add(make_entry("b.py", 3, 11));

        let versions = store.supported_versions();
        assert_eq!(versions.len(), 3);
        assert!(store.supports_version(&PythonVersion::new(3, 12, 0)));
        assert!(store.supports_version(&PythonVersion::new(3, 11, 0)));
        assert!(store.supports_version(&PythonVersion::new(3, 10, 0)));
    }

    #[test]
    fn test_coverage_stats() {
        let mut store = MultiVersionStore::new();

        store.add(make_entry("a.py", 3, 12));
        store.add(make_entry("b.py", 3, 12));
        store.add(make_entry("a.py", 3, 11));

        let stats = store.coverage_stats();
        assert_eq!(stats.total_files, 2);
        assert!(stats.is_complete_for_version(3, 12));
        assert!(!stats.is_complete_for_version(3, 11));
    }

    #[test]
    fn test_version_selector() {
        let mut store = MultiVersionStore::new();
        store.add(make_entry("test.py", 3, 11));
        store.add(make_entry("test.py", 3, 12));

        let selector = VersionSelector::new(PythonVersion::new(3, 12, 0));

        match selector.select(&store, "test.py") {
            SelectionResult::Found(b) => assert_eq!(b.python_version.minor, 12),
            _ => panic!("Expected Found"),
        }

        // Non-existent file
        match selector.select(&store, "missing.py") {
            SelectionResult::NeedCompilation => {}
            _ => panic!("Expected NeedCompilation"),
        }
    }

    #[test]
    fn test_fallback_behavior() {
        let mut store = MultiVersionStore::new();
        store.add(make_entry("test.py", 3, 10));

        // With Error fallback
        let selector = VersionSelector::new(PythonVersion::new(3, 9, 0))
            .with_fallback(FallbackBehavior::Error);

        match selector.select(&store, "test.py") {
            SelectionResult::NotFound => {}
            _ => panic!("Expected NotFound"),
        }

        // With UseNearest fallback
        let selector = VersionSelector::new(PythonVersion::new(3, 9, 0))
            .with_fallback(FallbackBehavior::UseNearest);

        match selector.select(&store, "test.py") {
            SelectionResult::NearestVersion(b) => assert_eq!(b.python_version.minor, 10),
            _ => panic!("Expected NearestVersion"),
        }
    }
}
