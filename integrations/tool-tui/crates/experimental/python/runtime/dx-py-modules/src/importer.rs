//! Python Module Import System
//!
//! Implements sys.path search, package imports, relative imports,
//! and importlib compatibility.

use dashmap::DashMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

/// Import system errors
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("No module named '{0}'")]
    ModuleNotFound(String),

    #[error("Cannot import name '{name}' from '{module}'")]
    ImportFromError { name: String, module: String },

    #[error("Attempted relative import with no known parent package")]
    NoParentPackage,

    #[error("Attempted relative import beyond top-level package")]
    BeyondTopLevel,

    #[error("Circular import detected: {0}")]
    CircularImport(String),

    #[error("Module '{0}' has no attribute '{1}'")]
    AttributeError(String, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid module spec: {0}")]
    InvalidSpec(String),
}

/// Result type for import operations
pub type ImportResult<T> = Result<T, ImportError>;

/// Module specification (similar to importlib.machinery.ModuleSpec)
#[derive(Debug, Clone)]
pub struct ModuleSpec {
    /// Fully qualified module name
    pub name: String,
    /// Loader to use for this module
    pub loader: LoaderType,
    /// Origin (file path or built-in)
    pub origin: Option<PathBuf>,
    /// Whether this is a package
    pub is_package: bool,
    /// Submodule search locations (for packages)
    pub submodule_search_locations: Option<Vec<PathBuf>>,
    /// Parent package name
    pub parent: Option<String>,
}

impl ModuleSpec {
    /// Create a new module spec
    pub fn new(name: impl Into<String>, loader: LoaderType) -> Self {
        let name = name.into();
        let parent = if name.contains('.') {
            Some(name.rsplit_once('.').unwrap().0.to_string())
        } else {
            None
        };

        Self {
            name,
            loader,
            origin: None,
            is_package: false,
            submodule_search_locations: None,
            parent,
        }
    }

    /// Set the origin path
    pub fn with_origin(mut self, origin: PathBuf) -> Self {
        self.origin = Some(origin);
        self
    }

    /// Mark as a package with search locations
    pub fn as_package(mut self, locations: Vec<PathBuf>) -> Self {
        self.is_package = true;
        self.submodule_search_locations = Some(locations);
        self
    }
}

/// Types of module loaders
#[derive(Debug, Clone, PartialEq)]
pub enum LoaderType {
    /// Source file loader (.py)
    SourceFile,
    /// Compiled bytecode loader (.pyc)
    BytecodeFile,
    /// DPM binary module loader (.dpm)
    DpmModule,
    /// Built-in module
    BuiltIn,
    /// Frozen module
    Frozen,
    /// C extension module (.pyd/.so)
    Extension,
    /// Namespace package (no __init__.py)
    NamespacePackage,
}

/// A loaded Python module
#[derive(Debug, Clone)]
pub struct PyModule {
    /// Module specification
    pub spec: ModuleSpec,
    /// Module dictionary (__dict__)
    pub dict: Arc<DashMap<String, ModuleValue>>,
    /// Module documentation
    pub doc: Option<String>,
    /// Whether the module has been fully initialized
    pub initialized: bool,
}

impl PyModule {
    /// Create a new module from a spec
    pub fn new(spec: ModuleSpec) -> Self {
        let dict = Arc::new(DashMap::new());

        // Set standard module attributes
        dict.insert("__name__".to_string(), ModuleValue::Str(spec.name.clone()));
        dict.insert("__loader__".to_string(), ModuleValue::Loader(spec.loader.clone()));

        if let Some(ref origin) = spec.origin {
            dict.insert(
                "__file__".to_string(),
                ModuleValue::Str(origin.to_string_lossy().to_string()),
            );
        }

        if spec.is_package {
            if let Some(ref locations) = spec.submodule_search_locations {
                let paths: Vec<String> =
                    locations.iter().map(|p| p.to_string_lossy().to_string()).collect();
                dict.insert("__path__".to_string(), ModuleValue::List(paths));
            }
            dict.insert("__package__".to_string(), ModuleValue::Str(spec.name.clone()));
        } else if let Some(ref parent) = spec.parent {
            dict.insert("__package__".to_string(), ModuleValue::Str(parent.clone()));
        }

        Self {
            spec,
            dict,
            doc: None,
            initialized: false,
        }
    }

    /// Get an attribute from the module
    pub fn get_attr(&self, name: &str) -> Option<ModuleValue> {
        self.dict.get(name).map(|v| v.clone())
    }

    /// Set an attribute on the module
    pub fn set_attr(&self, name: impl Into<String>, value: ModuleValue) {
        self.dict.insert(name.into(), value);
    }

    /// Check if module has an attribute
    pub fn has_attr(&self, name: &str) -> bool {
        self.dict.contains_key(name)
    }

    /// Get all exported names (__all__ or all public names)
    pub fn get_exports(&self) -> Vec<String> {
        if let Some(all) = self.dict.get("__all__") {
            if let ModuleValue::List(names) = &*all {
                return names.clone();
            }
        }

        // Return all public names (not starting with _)
        self.dict
            .iter()
            .map(|r| r.key().clone())
            .filter(|name| !name.starts_with('_'))
            .collect()
    }
}

/// Values that can be stored in a module
#[derive(Debug, Clone)]
pub enum ModuleValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<String>),
    Module(Arc<PyModule>),
    Loader(LoaderType),
    // In a full implementation, this would include functions, classes, etc.
}

/// The import system
pub struct ImportSystem {
    /// sys.path - list of directories to search
    sys_path: Vec<PathBuf>,
    /// sys.modules - cache of loaded modules
    sys_modules: DashMap<String, Arc<PyModule>>,
    /// Modules currently being imported (for circular import detection)
    importing: DashMap<String, ()>,
    /// Built-in module names
    builtin_modules: Vec<String>,
    /// Meta path finders
    meta_path: Vec<Box<dyn MetaPathFinder>>,
}

impl ImportSystem {
    /// Create a new import system
    pub fn new() -> Self {
        let mut sys = Self {
            sys_path: Vec::new(),
            sys_modules: DashMap::new(),
            importing: DashMap::new(),
            builtin_modules: vec![
                "sys".to_string(),
                "builtins".to_string(),
                "os".to_string(),
                "io".to_string(),
                "json".to_string(),
                "re".to_string(),
                "math".to_string(),
                "collections".to_string(),
                "itertools".to_string(),
                "functools".to_string(),
                "typing".to_string(),
            ],
            meta_path: Vec::new(),
        };

        // Add default finders
        sys.meta_path.push(Box::new(BuiltinFinder::new(&sys.builtin_modules)));
        sys.meta_path.push(Box::new(PathFinder::new()));

        sys
    }

    /// Add a path to sys.path
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.sys_path.push(path.into());
    }

    /// Set sys.path
    pub fn set_path(&mut self, paths: Vec<PathBuf>) {
        self.sys_path = paths;
    }

    /// Get sys.path
    pub fn get_path(&self) -> &[PathBuf] {
        &self.sys_path
    }

    /// Import a module by name
    pub fn import_module(&self, name: &str) -> ImportResult<Arc<PyModule>> {
        self.import_module_with_package(name, None)
    }

    /// Import a module with package context (for relative imports)
    pub fn import_module_with_package(
        &self,
        name: &str,
        package: Option<&str>,
    ) -> ImportResult<Arc<PyModule>> {
        // Handle relative imports
        let absolute_name = if name.starts_with('.') {
            self.resolve_relative_import(name, package)?
        } else {
            name.to_string()
        };

        // Check sys.modules cache
        if let Some(module) = self.sys_modules.get(&absolute_name) {
            return Ok(Arc::clone(&module));
        }

        // Check for circular import
        if self.importing.contains_key(&absolute_name) {
            // Return partially initialized module (Python allows this)
            if let Some(module) = self.sys_modules.get(&absolute_name) {
                return Ok(Arc::clone(&module));
            }
            return Err(ImportError::CircularImport(absolute_name));
        }

        // Mark as importing
        self.importing.insert(absolute_name.clone(), ());

        // Import parent packages first
        if let Some(parent_name) = absolute_name.rsplit_once('.').map(|(p, _)| p.to_string()) {
            self.import_module(&parent_name)?;
        }

        // Find and load the module
        let result = self.find_and_load(&absolute_name);

        // Remove from importing set
        self.importing.remove(&absolute_name);

        result
    }

    /// Resolve a relative import to an absolute name
    fn resolve_relative_import(&self, name: &str, package: Option<&str>) -> ImportResult<String> {
        let package = package.ok_or(ImportError::NoParentPackage)?;

        // Count leading dots
        let dots = name.chars().take_while(|&c| c == '.').count();
        let relative_name = &name[dots..];

        // Split package into parts
        let mut parts: Vec<&str> = package.split('.').collect();

        // For relative imports:
        // - 1 dot (.foo) means import from current package
        // - 2 dots (..foo) means go up 1 level from current package
        // - 3 dots (...foo) means go up 2 levels, etc.
        //
        // So we need to go up (dots - 1) levels from the package.
        // But first, we need to check if we have enough levels to go up.
        let levels_to_go_up = dots - 1;

        // Check if we're trying to go beyond the top level
        // We need at least `levels_to_go_up` parts to go up that many levels
        // AND if we're importing something (relative_name is not empty),
        // we need to end up with at least one part remaining OR be at the top level
        if levels_to_go_up > parts.len() {
            return Err(ImportError::BeyondTopLevel);
        }

        // Go up the required levels
        for _ in 0..levels_to_go_up {
            parts.pop();
        }

        // Build absolute name
        if relative_name.is_empty() {
            // Just importing the parent package itself (e.g., `from .. import`)
            if parts.is_empty() {
                return Err(ImportError::BeyondTopLevel);
            }
            Ok(parts.join("."))
        } else {
            // Importing something from the parent package
            // If parts is empty after going up, we're trying to import from "nowhere"
            // which is beyond the top level
            if parts.is_empty() && levels_to_go_up > 0 {
                return Err(ImportError::BeyondTopLevel);
            }

            if parts.is_empty() {
                // Single dot import from top-level module (shouldn't happen normally)
                Ok(relative_name.to_string())
            } else {
                parts.push(relative_name);
                Ok(parts.join("."))
            }
        }
    }

    /// Find and load a module
    fn find_and_load(&self, name: &str) -> ImportResult<Arc<PyModule>> {
        // Try each meta path finder
        for finder in &self.meta_path {
            if let Some(spec) = finder.find_spec(name, &self.sys_path, None)? {
                return self.load_module(spec);
            }
        }

        Err(ImportError::ModuleNotFound(name.to_string()))
    }

    /// Load a module from a spec
    fn load_module(&self, spec: ModuleSpec) -> ImportResult<Arc<PyModule>> {
        let name = spec.name.clone();

        // Create the module
        let module = Arc::new(PyModule::new(spec.clone()));

        // Add to sys.modules before executing (for circular imports)
        self.sys_modules.insert(name.clone(), Arc::clone(&module));

        // Execute the module based on loader type
        match spec.loader {
            LoaderType::BuiltIn => {
                // Built-in modules are pre-initialized
                // In a full implementation, we'd call the module's init function
            }
            LoaderType::SourceFile => {
                if let Some(ref origin) = spec.origin {
                    self.exec_source_module(&module, origin)?;
                }
            }
            LoaderType::DpmModule => {
                if let Some(ref origin) = spec.origin {
                    self.exec_dpm_module(&module, origin)?;
                }
            }
            LoaderType::NamespacePackage => {
                // Namespace packages have no code to execute
            }
            _ => {
                // Other loaders not yet implemented
            }
        }

        Ok(module)
    }

    /// Execute a source module
    ///
    /// This reads the source file and sets up module attributes.
    /// In a full implementation, this would also compile and execute the bytecode.
    fn exec_source_module(&self, module: &PyModule, path: &Path) -> ImportResult<()> {
        // Read the source file
        let source = std::fs::read_to_string(path)?;

        // Set __file__ attribute (may already be set, but ensure it's correct)
        module.set_attr("__file__", ModuleValue::Str(path.to_string_lossy().to_string()));

        // Set __cached__ attribute (path to .pyc file)
        let cached_path = self.get_cached_path(path);
        if let Some(cached) = cached_path {
            module.set_attr("__cached__", ModuleValue::Str(cached.to_string_lossy().to_string()));
        }

        // Set __doc__ if there's a module docstring
        if let Some(doc) = Self::extract_docstring(&source) {
            module.set_attr("__doc__", ModuleValue::Str(doc));
        } else {
            module.set_attr("__doc__", ModuleValue::None);
        }

        // Extract and set __all__ if defined in source
        if let Some(all_list) = Self::extract_all(&source) {
            module.set_attr("__all__", ModuleValue::List(all_list));
        }

        // In a full implementation, we would:
        // 1. Parse the source to AST
        // 2. Compile AST to bytecode
        // 3. Execute bytecode in module's namespace
        // 4. Populate module dict with defined names
        //
        // For now, we extract simple definitions from source
        Self::extract_definitions(&source, module);

        Ok(())
    }

    /// Get the cached bytecode path for a source file
    fn get_cached_path(&self, source_path: &Path) -> Option<PathBuf> {
        let parent = source_path.parent()?;
        let stem = source_path.file_stem()?.to_string_lossy();

        // Python 3 style: __pycache__/module.cpython-312.pyc
        let pycache = parent.join("__pycache__");
        Some(pycache.join(format!("{}.cpython-312.pyc", stem)))
    }

    /// Extract module docstring from source
    fn extract_docstring(source: &str) -> Option<String> {
        let trimmed = source.trim_start();

        // Check for triple-quoted string at start
        if let Some(rest) = trimmed.strip_prefix("\"\"\"") {
            if let Some(end) = rest.find("\"\"\"") {
                return Some(rest[..end].trim().to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("'''") {
            if let Some(end) = rest.find("'''") {
                return Some(rest[..end].trim().to_string());
            }
        }

        None
    }

    /// Extract __all__ list from source
    fn extract_all(source: &str) -> Option<Vec<String>> {
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("__all__") {
                // Simple parsing: __all__ = ['a', 'b', 'c'] or __all__ = ["a", "b", "c"]
                if let Some(eq_pos) = trimmed.find('=') {
                    let value = trimmed[eq_pos + 1..].trim();
                    if value.starts_with('[') && value.ends_with(']') {
                        let inner = &value[1..value.len() - 1];
                        let names: Vec<String> = inner
                            .split(',')
                            .filter_map(|s| {
                                let s = s.trim();
                                if (s.starts_with('\'') && s.ends_with('\''))
                                    || (s.starts_with('"') && s.ends_with('"'))
                                {
                                    Some(s[1..s.len() - 1].to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        if !names.is_empty() {
                            return Some(names);
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract simple definitions from source (functions, classes, constants)
    fn extract_definitions(source: &str, module: &PyModule) {
        for line in source.lines() {
            let trimmed = line.trim();

            // Function definition at module level (no leading whitespace)
            if line.starts_with("def ") {
                if let Some(name) = trimmed.strip_prefix("def ") {
                    let name = name.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() {
                        // Mark as a function placeholder
                        module.set_attr(name, ModuleValue::Str(format!("<function {}>", name)));
                    }
                }
            }
            // Class definition at module level
            else if line.starts_with("class ") {
                if let Some(name) = trimmed.strip_prefix("class ") {
                    let name = name.split(['(', ':']).next().unwrap_or("").trim();
                    if !name.is_empty() {
                        // Mark as a class placeholder
                        module.set_attr(name, ModuleValue::Str(format!("<class {}>", name)));
                    }
                }
            }
            // Module-level assignment (simple cases)
            else if !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('#') {
                if let Some(eq_pos) = line.find('=') {
                    // Skip augmented assignments and comparisons
                    let before_eq = &line[..eq_pos];
                    if !before_eq.ends_with(['!', '<', '>', '+', '-', '*', '/', '%', '&', '|', '^'])
                    {
                        let name = before_eq.trim();
                        // Only simple identifiers
                        if !name.is_empty()
                            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                            && !name.starts_with(char::is_numeric)
                        {
                            let value = line[eq_pos + 1..].trim();
                            // Try to parse simple values
                            if let Some(v) = Self::parse_simple_value(value) {
                                module.set_attr(name, v);
                            } else {
                                // Store as string representation
                                module.set_attr(name, ModuleValue::Str(value.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Parse simple Python values
    fn parse_simple_value(value: &str) -> Option<ModuleValue> {
        let value = value.trim();

        // None
        if value == "None" {
            return Some(ModuleValue::None);
        }

        // Boolean
        if value == "True" {
            return Some(ModuleValue::Bool(true));
        }
        if value == "False" {
            return Some(ModuleValue::Bool(false));
        }

        // Integer
        if let Ok(i) = value.parse::<i64>() {
            return Some(ModuleValue::Int(i));
        }

        // Float
        if let Ok(f) = value.parse::<f64>() {
            return Some(ModuleValue::Float(f));
        }

        // String (single or double quoted)
        if (value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"'))
        {
            return Some(ModuleValue::Str(value[1..value.len() - 1].to_string()));
        }

        // Triple-quoted string
        if (value.starts_with("'''") && value.ends_with("'''"))
            || (value.starts_with("\"\"\"") && value.ends_with("\"\"\""))
        {
            return Some(ModuleValue::Str(value[3..value.len() - 3].to_string()));
        }

        None
    }

    /// Execute a DPM binary module
    fn exec_dpm_module(&self, _module: &PyModule, _path: &Path) -> ImportResult<()> {
        // In a full implementation:
        // 1. Load the DPM file
        // 2. Execute the init bytecode
        // 3. Populate the module namespace
        Ok(())
    }

    /// Import specific names from a module
    pub fn import_from(
        &self,
        module_name: &str,
        names: &[&str],
        package: Option<&str>,
    ) -> ImportResult<HashMap<String, ModuleValue>> {
        let module = self.import_module_with_package(module_name, package)?;

        let mut result = HashMap::new();
        for name in names {
            if *name == "*" {
                // Import all exported names
                for export_name in module.get_exports() {
                    if let Some(value) = module.get_attr(&export_name) {
                        result.insert(export_name, value);
                    }
                }
            } else if let Some(value) = module.get_attr(name) {
                result.insert(name.to_string(), value);
            } else {
                return Err(ImportError::ImportFromError {
                    name: name.to_string(),
                    module: module_name.to_string(),
                });
            }
        }

        Ok(result)
    }

    /// Get a module from sys.modules
    pub fn get_module(&self, name: &str) -> Option<Arc<PyModule>> {
        self.sys_modules.get(name).map(|m| Arc::clone(&m))
    }

    /// Add a module to sys.modules
    pub fn add_module(&self, name: impl Into<String>, module: Arc<PyModule>) {
        self.sys_modules.insert(name.into(), module);
    }

    /// Remove a module from sys.modules
    pub fn remove_module(&self, name: &str) -> Option<Arc<PyModule>> {
        self.sys_modules.remove(name).map(|(_, m)| m)
    }

    /// Reload a module
    pub fn reload(&self, name: &str) -> ImportResult<Arc<PyModule>> {
        // Remove from cache
        self.sys_modules.remove(name);

        // Re-import
        self.import_module(name)
    }
}

impl Default for ImportSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Meta path finder trait (similar to importlib.abc.MetaPathFinder)
pub trait MetaPathFinder: Send + Sync {
    /// Find a module spec
    fn find_spec(
        &self,
        name: &str,
        path: &[PathBuf],
        target: Option<&PyModule>,
    ) -> ImportResult<Option<ModuleSpec>>;
}

/// Finder for built-in modules
pub struct BuiltinFinder {
    modules: Vec<String>,
}

impl BuiltinFinder {
    pub fn new(modules: &[String]) -> Self {
        Self {
            modules: modules.to_vec(),
        }
    }
}

impl MetaPathFinder for BuiltinFinder {
    fn find_spec(
        &self,
        name: &str,
        _path: &[PathBuf],
        _target: Option<&PyModule>,
    ) -> ImportResult<Option<ModuleSpec>> {
        if self.modules.contains(&name.to_string()) {
            Ok(Some(ModuleSpec::new(name, LoaderType::BuiltIn)))
        } else {
            Ok(None)
        }
    }
}

/// Finder for modules on sys.path
pub struct PathFinder;

impl PathFinder {
    pub fn new() -> Self {
        Self
    }

    /// Find a module in a directory
    fn find_in_directory(&self, name: &str, dir: &Path) -> Option<ModuleSpec> {
        let parts: Vec<&str> = name.split('.').collect();
        let module_name = parts.last().unwrap();

        // Build the path to search
        let mut search_path = dir.to_path_buf();
        for part in &parts[..parts.len() - 1] {
            search_path = search_path.join(part);
        }

        // Check for package (directory with __init__.py)
        let package_dir = search_path.join(module_name);
        let init_py = package_dir.join("__init__.py");
        if init_py.exists() {
            return Some(
                ModuleSpec::new(name, LoaderType::SourceFile)
                    .with_origin(init_py)
                    .as_package(vec![package_dir]),
            );
        }

        // Check for namespace package (directory without __init__.py)
        if package_dir.is_dir() {
            return Some(
                ModuleSpec::new(name, LoaderType::NamespacePackage).as_package(vec![package_dir]),
            );
        }

        // Check for source file
        let py_file = search_path.join(format!("{}.py", module_name));
        if py_file.exists() {
            return Some(ModuleSpec::new(name, LoaderType::SourceFile).with_origin(py_file));
        }

        // Check for DPM binary module
        let dpm_file = search_path.join(format!("{}.dpm", module_name));
        if dpm_file.exists() {
            return Some(ModuleSpec::new(name, LoaderType::DpmModule).with_origin(dpm_file));
        }

        // Check for compiled bytecode
        let pyc_file =
            search_path.join("__pycache__").join(format!("{}.cpython-312.pyc", module_name));
        if pyc_file.exists() {
            return Some(ModuleSpec::new(name, LoaderType::BytecodeFile).with_origin(pyc_file));
        }

        // Check for C extension
        #[cfg(windows)]
        let ext_file = search_path.join(format!("{}.pyd", module_name));
        #[cfg(not(windows))]
        let ext_file = search_path.join(format!("{}.so", module_name));

        if ext_file.exists() {
            return Some(ModuleSpec::new(name, LoaderType::Extension).with_origin(ext_file));
        }

        None
    }
}

impl Default for PathFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaPathFinder for PathFinder {
    fn find_spec(
        &self,
        name: &str,
        path: &[PathBuf],
        _target: Option<&PyModule>,
    ) -> ImportResult<Option<ModuleSpec>> {
        for dir in path {
            if let Some(spec) = self.find_in_directory(name, dir) {
                return Ok(Some(spec));
            }
        }
        Ok(None)
    }
}
