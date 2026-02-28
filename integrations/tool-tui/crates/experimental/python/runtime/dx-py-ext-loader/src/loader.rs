//! Dynamic library loading for C extensions
//!
//! Handles loading shared libraries and calling PyInit_* functions.

use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use libloading::{Library, Symbol};
use parking_lot::RwLock;

use dx_py_ffi::PyObject;

use crate::abi::{AbiCompatibility, AbiVersion};
use crate::capi_table::{ApiUsageTracker, CApiTable};
use crate::discovery::ExtensionDiscovery;
use crate::error::{ExtensionError, ExtensionResult};

/// Type for PyInit_* module initialization functions
pub type PyModInitFunction = unsafe extern "C" fn() -> *mut PyObject;

/// Extension loader for C extension modules
pub struct ExtensionLoader {
    /// Extension discovery service
    discovery: ExtensionDiscovery,
    /// Loaded extensions cache
    loaded: DashMap<String, Arc<LoadedExtension>>,
    /// ABI version we support
    abi_version: AbiVersion,
    /// API usage tracking (module -> set of API functions called)
    api_usage: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Global API usage tracker
    api_tracker: Arc<ApiUsageTracker>,
    /// C API function table
    capi_table: Arc<CApiTable>,
}

impl ExtensionLoader {
    /// Create a new extension loader
    pub fn new(abi_version: AbiVersion) -> Self {
        Self {
            discovery: ExtensionDiscovery::new(abi_version),
            loaded: DashMap::new(),
            abi_version,
            api_usage: Arc::new(RwLock::new(HashMap::new())),
            api_tracker: Arc::new(ApiUsageTracker::new()),
            capi_table: Arc::new(CApiTable::new()),
        }
    }

    /// Create with default ABI version (DX-Py's supported version)
    pub fn with_default_abi() -> Self {
        Self::new(AbiVersion::dx_py_abi())
    }

    /// Add a search path for extensions
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        self.discovery.add_search_path(path);
    }

    /// Get the discovery service
    pub fn discovery(&self) -> &ExtensionDiscovery {
        &self.discovery
    }

    /// Get the ABI version
    pub fn abi_version(&self) -> AbiVersion {
        self.abi_version
    }

    /// Get the C API table
    pub fn capi_table(&self) -> &CApiTable {
        &self.capi_table
    }

    /// Get the API usage tracker
    pub fn api_tracker(&self) -> &ApiUsageTracker {
        &self.api_tracker
    }

    /// Check if an extension is already loaded
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.contains_key(name)
    }

    /// Get a loaded extension
    pub fn get(&self, name: &str) -> Option<Arc<LoadedExtension>> {
        self.loaded.get(name).map(|r| Arc::clone(&r))
    }

    /// Check compatibility of an extension before loading
    pub fn check_compatibility(&self, path: &Path) -> ExtensionResult<AbiCompatibility> {
        let filename = path.file_name().and_then(|s| s.to_str()).ok_or_else(|| {
            ExtensionError::InvalidExtension {
                path: path.to_path_buf(),
                reason: "Invalid filename".to_string(),
            }
        })?;

        let ext_abi = AbiVersion::from_filename(filename);

        match ext_abi {
            Some(abi) => Ok(self.abi_version.is_compatible_with(&abi)),
            None => {
                // Can't determine ABI from filename, assume compatible
                Ok(AbiCompatibility::Compatible {
                    warnings: vec!["Could not determine ABI version from filename".to_string()],
                })
            }
        }
    }

    /// Load a C extension module by name
    pub fn load(&self, name: &str) -> ExtensionResult<Arc<LoadedExtension>> {
        // Check if already loaded
        if let Some(ext) = self.loaded.get(name) {
            return Ok(Arc::clone(&ext));
        }

        // Find the extension file
        let path = self.discovery.find_extension(name)?;

        // Check ABI compatibility
        let compatibility = self.check_compatibility(&path)?;
        if !compatibility.can_load() {
            let ext_abi =
                AbiVersion::from_filename(path.file_name().and_then(|s| s.to_str()).unwrap_or(""))
                    .unwrap_or_default();

            return Err(ExtensionError::AbiMismatch {
                name: name.to_string(),
                expected: self.abi_version,
                found: ext_abi,
            });
        }

        // Load the extension
        let extension = self.load_from_path(name, &path)?;
        let extension = Arc::new(extension);

        self.loaded.insert(name.to_string(), Arc::clone(&extension));

        Ok(extension)
    }

    /// Load an extension from a specific path
    fn load_from_path(&self, name: &str, path: &Path) -> ExtensionResult<LoadedExtension> {
        // Load the shared library
        let library = unsafe {
            Library::new(path).map_err(|e| ExtensionError::LoadFailure {
                name: name.to_string(),
                path: path.to_path_buf(),
                reason: format!("OS error loading library: {}", e),
            })?
        };

        // Find the PyInit_* function
        // The function name is PyInit_<module_name> where module_name is the last
        // component of the dotted name (e.g., numpy.core._multiarray_umath -> _multiarray_umath)
        let module_base_name = name.split('.').next_back().unwrap_or(name);
        let init_func_name = format!("PyInit_{}", module_base_name);

        // Try to find the init function
        let init_func: Symbol<PyModInitFunction> = unsafe {
            library.get(init_func_name.as_bytes()).map_err(|e| {
                // Provide helpful error message with suggestions
                let mut reason = format!("Could not find symbol '{}': {}", init_func_name, e);

                // Check for common alternative names
                let alt_names = [
                    format!("PyInit_{}", name.replace('.', "_")),
                    format!("init{}", module_base_name), // Python 2 style
                ];

                for alt in &alt_names {
                    if library.get::<PyModInitFunction>(alt.as_bytes()).is_ok() {
                        reason.push_str(&format!(". Found alternative: '{}'", alt));
                        break;
                    }
                }

                ExtensionError::InitFailure {
                    name: name.to_string(),
                    reason,
                }
            })?
        };

        // Call the initialization function
        let module = unsafe {
            let ptr = init_func();
            if ptr.is_null() {
                return Err(ExtensionError::InitFailure {
                    name: name.to_string(),
                    reason: "PyInit function returned NULL (module initialization failed)"
                        .to_string(),
                });
            }
            ptr
        };

        // Initialize API usage tracking for this module
        self.api_usage.write().insert(name.to_string(), HashSet::new());

        Ok(LoadedExtension {
            name: name.to_string(),
            path: path.to_path_buf(),
            library,
            module,
            abi_version: AbiVersion::from_filename(
                path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            ),
        })
    }

    /// Load an extension directly from a path (bypassing discovery)
    pub fn load_from_file<P: AsRef<Path>>(
        &self,
        name: &str,
        path: P,
    ) -> ExtensionResult<Arc<LoadedExtension>> {
        let path = path.as_ref();

        // Check if already loaded
        if let Some(ext) = self.loaded.get(name) {
            return Ok(Arc::clone(&ext));
        }

        // Check ABI compatibility
        let compatibility = self.check_compatibility(path)?;
        if !compatibility.can_load() {
            let ext_abi =
                AbiVersion::from_filename(path.file_name().and_then(|s| s.to_str()).unwrap_or(""))
                    .unwrap_or_default();

            return Err(ExtensionError::AbiMismatch {
                name: name.to_string(),
                expected: self.abi_version,
                found: ext_abi,
            });
        }

        // Load the extension
        let extension = self.load_from_path(name, path)?;
        let extension = Arc::new(extension);

        self.loaded.insert(name.to_string(), Arc::clone(&extension));

        Ok(extension)
    }

    /// Get API usage report for a loaded extension
    pub fn api_usage(&self, name: &str) -> Option<HashSet<String>> {
        self.api_usage.read().get(name).cloned()
    }

    /// Record an API function call for a module
    pub fn record_api_call(&self, module: &str, api_function: &str) {
        if let Some(usage) = self.api_usage.write().get_mut(module) {
            usage.insert(api_function.to_string());
        }
    }

    /// Get all loaded extension names
    pub fn loaded_extensions(&self) -> Vec<String> {
        self.loaded.iter().map(|r| r.key().clone()).collect()
    }

    /// Unload an extension (if possible)
    pub fn unload(&self, name: &str) -> bool {
        self.loaded.remove(name).is_some()
    }

    /// Check if an extension uses any unsupported API functions
    ///
    /// Returns an error if unsupported functions were called
    pub fn check_unsupported_apis(&self, name: &str) -> ExtensionResult<()> {
        let unsupported = self.api_tracker.get_unsupported_for_extension(name);

        if unsupported.is_empty() {
            return Ok(());
        }

        // Build a descriptive error message
        let func_list = unsupported.join(", ");
        Err(ExtensionError::UnsupportedApi {
            name: name.to_string(),
            functions: unsupported,
            message: format!(
                "Extension '{}' uses unsupported CPython API functions: {}. \
                 These functions are not yet implemented in DX-Py.",
                name, func_list
            ),
        })
    }

    /// Get a report of all API usage across loaded extensions
    pub fn api_usage_report(&self) -> ApiUsageReport {
        let implemented = CApiTable::implemented_functions();
        let stubs = CApiTable::stub_functions();

        let mut used_implemented = Vec::new();
        let mut used_stubs = Vec::new();

        let usage = self.api_usage.read();
        for (module, funcs) in usage.iter() {
            for func in funcs {
                if implemented.contains(&func.as_str()) {
                    used_implemented.push((module.clone(), func.clone()));
                } else if stubs.contains(&func.as_str()) {
                    used_stubs.push((module.clone(), func.clone()));
                }
            }
        }

        ApiUsageReport {
            total_extensions: self.loaded.len(),
            implemented_functions_used: used_implemented,
            stub_functions_used: used_stubs,
            has_unsupported: self.api_tracker.has_unsupported_calls(),
        }
    }
}

impl Default for ExtensionLoader {
    fn default() -> Self {
        Self::with_default_abi()
    }
}

/// Report of API usage across loaded extensions
#[derive(Debug, Clone)]
pub struct ApiUsageReport {
    /// Total number of loaded extensions
    pub total_extensions: usize,
    /// Implemented functions that were used (module, function)
    pub implemented_functions_used: Vec<(String, String)>,
    /// Stub functions that were used (module, function)
    pub stub_functions_used: Vec<(String, String)>,
    /// Whether any unsupported functions were called
    pub has_unsupported: bool,
}

impl ApiUsageReport {
    /// Generate a markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# API Usage Report\n\n");
        md.push_str(&format!("**Total Extensions:** {}\n\n", self.total_extensions));

        if self.has_unsupported {
            md.push_str("⚠️ **Warning:** Some extensions use unsupported API functions.\n\n");
        }

        md.push_str("## Implemented Functions Used\n\n");
        if self.implemented_functions_used.is_empty() {
            md.push_str("_None_\n\n");
        } else {
            for (module, func) in &self.implemented_functions_used {
                md.push_str(&format!("- `{}` (by {})\n", func, module));
            }
            md.push('\n');
        }

        md.push_str("## Stub Functions Used\n\n");
        if self.stub_functions_used.is_empty() {
            md.push_str("_None_\n\n");
        } else {
            for (module, func) in &self.stub_functions_used {
                md.push_str(&format!("- `{}` (by {})\n", func, module));
            }
            md.push('\n');
        }

        md
    }
}

/// A loaded C extension module
pub struct LoadedExtension {
    /// Module name
    pub name: String,
    /// Path to the extension file
    pub path: PathBuf,
    /// The shared library handle
    library: Library,
    /// Module object after initialization
    module: *mut PyObject,
    /// Detected ABI version
    pub abi_version: Option<AbiVersion>,
}

impl LoadedExtension {
    /// Get the module object
    pub fn module(&self) -> *mut PyObject {
        self.module
    }

    /// Get a symbol from the library
    ///
    /// # Safety
    /// The caller must ensure the symbol type matches the actual symbol.
    pub unsafe fn get_symbol<T>(&self, name: &[u8]) -> Option<Symbol<'_, T>> {
        self.library.get(name).ok()
    }

    /// Get a symbol by string name
    ///
    /// # Safety
    /// The caller must ensure the symbol type matches the actual symbol.
    pub unsafe fn get_symbol_by_name<T>(&self, name: &str) -> Option<Symbol<'_, T>> {
        self.get_symbol(name.as_bytes())
    }

    /// Check if the extension has a specific symbol
    pub fn has_symbol(&self, name: &str) -> bool {
        unsafe { self.library.get::<*const c_void>(name.as_bytes()).is_ok() }
    }

    /// List all exported symbols (platform-dependent, may not work on all platforms)
    pub fn list_symbols(&self) -> Vec<String> {
        // This is a placeholder - actual implementation would need platform-specific code
        // to enumerate symbols from the loaded library
        Vec::new()
    }
}

// LoadedExtension contains raw pointers but they're managed by the library
// and module initialization. The library handle ensures the module stays valid.
unsafe impl Send for LoadedExtension {}
unsafe impl Sync for LoadedExtension {}

impl std::fmt::Debug for LoadedExtension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedExtension")
            .field("name", &self.name)
            .field("path", &self.path)
            .field("abi_version", &self.abi_version)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_loader_creation() {
        let loader = ExtensionLoader::with_default_abi();
        assert_eq!(loader.abi_version(), AbiVersion::dx_py_abi());
        assert!(loader.loaded_extensions().is_empty());
    }

    #[test]
    fn test_extension_loader_search_paths() {
        let mut loader = ExtensionLoader::default();
        loader.add_search_path("/usr/lib/python3.11");

        assert_eq!(loader.discovery().search_paths().len(), 1);
    }

    #[test]
    fn test_extension_not_found() {
        let loader = ExtensionLoader::default();
        let result = loader.load("nonexistent_module");

        assert!(matches!(result, Err(ExtensionError::NotFound { .. })));
    }

    #[test]
    fn test_api_usage_tracking() {
        let loader = ExtensionLoader::default();

        // Manually insert a module for testing
        loader.api_usage.write().insert("test_module".to_string(), HashSet::new());

        loader.record_api_call("test_module", "PyArg_ParseTuple");
        loader.record_api_call("test_module", "Py_BuildValue");

        let usage = loader.api_usage("test_module").unwrap();
        assert!(usage.contains("PyArg_ParseTuple"));
        assert!(usage.contains("Py_BuildValue"));
    }
}
