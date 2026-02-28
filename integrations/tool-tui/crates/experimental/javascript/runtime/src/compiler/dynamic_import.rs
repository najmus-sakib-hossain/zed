//! Dynamic Import Support
//!
//! Implements the `import()` expression for runtime module loading.
//! Supports:
//! - Relative path resolution
//! - Bare specifier resolution (Node.js module resolution)
//! - Module caching
//! - Promise-based async loading
//! - ESM/CJS interop
//!
//! # Requirements
//! - 2.1: WHEN `import(specifier)` is called with a valid module path, THE Runtime SHALL return a Promise that resolves to the module namespace
//! - 2.2: WHEN `import(specifier)` is called with a relative path, THE Runtime SHALL resolve it relative to the importing module
//! - 2.3: WHEN `import(specifier)` is called with a bare specifier, THE Runtime SHALL resolve it using Node.js module resolution

use crate::compiler::modules::{ModuleResolver, ModuleType, PackageJson};
use crate::error::DxError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Represents a loaded ES module's namespace object
#[derive(Debug, Clone)]
pub struct ModuleNamespace {
    /// Module URL/path (resolved absolute path)
    pub url: String,
    /// Exported bindings (export name -> value as f64)
    pub exports: HashMap<String, f64>,
    /// Default export (if any)
    pub default: Option<f64>,
    /// Module type (ESM or CJS)
    pub module_type: ModuleType,
    /// Whether the module has been fully evaluated
    pub evaluated: bool,
}

impl ModuleNamespace {
    /// Create a new empty module namespace
    pub fn new(url: String, module_type: ModuleType) -> Self {
        Self {
            url,
            exports: HashMap::new(),
            default: None,
            module_type,
            evaluated: false,
        }
    }

    /// Add an export to the namespace
    pub fn add_export(&mut self, name: String, value: f64) {
        if name == "default" {
            self.default = Some(value);
        }
        self.exports.insert(name, value);
    }

    /// Get an export by name
    pub fn get_export(&self, name: &str) -> Option<f64> {
        self.exports.get(name).copied()
    }

    /// Check if the module has a default export
    pub fn has_default(&self) -> bool {
        self.default.is_some()
    }

    /// Mark the module as evaluated
    pub fn mark_evaluated(&mut self) {
        self.evaluated = true;
    }
}

/// Error types for dynamic import operations
#[derive(Debug, Clone)]
pub enum ImportError {
    /// Module not found
    ModuleNotFound(String),
    /// Syntax error in module
    SyntaxError(String),
    /// Module resolution failed
    ResolutionError(String),
    /// Circular dependency detected
    CircularDependency(String),
    /// Module evaluation failed
    EvaluationError(String),
    /// Invalid specifier
    InvalidSpecifier(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::ModuleNotFound(path) => write!(f, "Cannot find module '{}'", path),
            ImportError::SyntaxError(msg) => write!(f, "SyntaxError: {}", msg),
            ImportError::ResolutionError(msg) => write!(f, "Module resolution failed: {}", msg),
            ImportError::CircularDependency(path) => {
                write!(f, "Circular dependency detected: {}", path)
            }
            ImportError::EvaluationError(msg) => write!(f, "Module evaluation failed: {}", msg),
            ImportError::InvalidSpecifier(spec) => write!(f, "Invalid module specifier: {}", spec),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<ImportError> for DxError {
    fn from(err: ImportError) -> Self {
        match err {
            ImportError::ModuleNotFound(path) => DxError::ModuleNotFound(path),
            ImportError::SyntaxError(msg) => DxError::ParseError(msg),
            ImportError::ResolutionError(msg) => DxError::ModuleNotFound(msg),
            ImportError::CircularDependency(path) => {
                DxError::RuntimeError(format!("Circular dependency: {}", path))
            }
            ImportError::EvaluationError(msg) => DxError::RuntimeError(msg),
            ImportError::InvalidSpecifier(spec) => {
                DxError::ParseError(format!("Invalid specifier: {}", spec))
            }
        }
    }
}

/// Promise state for dynamic import
#[derive(Debug, Clone, PartialEq)]
pub enum PromiseState {
    /// Promise is pending
    Pending,
    /// Promise is fulfilled with module namespace
    Fulfilled,
    /// Promise is rejected with error
    Rejected,
}

/// Represents a Promise for dynamic import
#[derive(Debug, Clone)]
pub struct ImportPromise {
    /// Promise state
    pub state: PromiseState,
    /// Resolved module namespace (if fulfilled)
    pub value: Option<ModuleNamespace>,
    /// Error message (if rejected)
    pub error: Option<ImportError>,
    /// Promise ID for tracking
    pub id: u64,
}

impl ImportPromise {
    /// Create a new pending promise
    pub fn pending(id: u64) -> Self {
        Self {
            state: PromiseState::Pending,
            value: None,
            error: None,
            id,
        }
    }

    /// Resolve the promise with a module namespace
    pub fn resolve(mut self, namespace: ModuleNamespace) -> Self {
        self.state = PromiseState::Fulfilled;
        self.value = Some(namespace);
        self
    }

    /// Reject the promise with an error
    pub fn reject(mut self, error: ImportError) -> Self {
        self.state = PromiseState::Rejected;
        self.error = Some(error);
        self
    }

    /// Check if the promise is pending
    pub fn is_pending(&self) -> bool {
        self.state == PromiseState::Pending
    }

    /// Check if the promise is fulfilled
    pub fn is_fulfilled(&self) -> bool {
        self.state == PromiseState::Fulfilled
    }

    /// Check if the promise is rejected
    pub fn is_rejected(&self) -> bool {
        self.state == PromiseState::Rejected
    }
}

/// Dynamic import loader for runtime module loading
///
/// Handles the `import()` expression by:
/// 1. Resolving the module specifier
/// 2. Loading and compiling the module
/// 3. Caching the result
/// 4. Returning a Promise that resolves to the module namespace
pub struct DynamicImportLoader {
    /// Cache of loaded modules (resolved path -> namespace)
    module_cache: HashMap<String, ModuleNamespace>,
    /// Modules currently being loaded (for circular dependency detection)
    loading: HashMap<String, bool>,
    /// Module resolver for path resolution
    resolver: ModuleResolver,
    /// Next promise ID
    next_promise_id: u64,
    /// Pending promises
    pending_promises: HashMap<u64, ImportPromise>,
}

impl Default for DynamicImportLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicImportLoader {
    /// Create a new dynamic import loader
    pub fn new() -> Self {
        Self {
            module_cache: HashMap::new(),
            loading: HashMap::new(),
            resolver: ModuleResolver::new(),
            next_promise_id: 1,
            pending_promises: HashMap::new(),
        }
    }

    /// Create a new dynamic import loader with custom search paths
    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        let mut loader = Self::new();
        // The resolver will use these paths for bare specifier resolution
        for path in search_paths {
            loader.add_search_path(path);
        }
        loader
    }

    /// Add a search path for module resolution
    pub fn add_search_path(&mut self, path: PathBuf) {
        // ModuleResolver has internal search_paths, we need to recreate it
        // For now, we'll just use the default resolver
        // TODO: Add method to ModuleResolver to add search paths
        let _ = path;
    }

    /// Resolve a module specifier to an absolute path
    ///
    /// # Arguments
    /// * `specifier` - The module specifier (relative path, bare specifier, or absolute path)
    /// * `referrer` - The path of the importing module
    ///
    /// # Returns
    /// The resolved absolute path to the module
    pub fn resolve(&mut self, specifier: &str, referrer: &Path) -> Result<PathBuf, ImportError> {
        // Validate specifier
        if specifier.is_empty() {
            return Err(ImportError::InvalidSpecifier(
                "Module specifier cannot be empty".to_string(),
            ));
        }

        // Use the module resolver
        self.resolver
            .resolve(specifier, referrer)
            .map_err(|e| ImportError::ResolutionError(e.to_string()))
    }

    /// Load a module dynamically, returning a Promise
    ///
    /// This is the main entry point for `import()` expressions.
    ///
    /// # Arguments
    /// * `specifier` - The module specifier
    /// * `referrer` - The path of the importing module
    ///
    /// # Returns
    /// A Promise that resolves to the module namespace or rejects with an error
    pub fn import(&mut self, specifier: &str, referrer: &str) -> ImportPromise {
        let promise_id = self.next_promise_id;
        self.next_promise_id += 1;

        let promise = ImportPromise::pending(promise_id);

        // Resolve the specifier
        let referrer_path = Path::new(referrer);
        let resolved = match self.resolve(specifier, referrer_path) {
            Ok(path) => path,
            Err(e) => {
                return promise.reject(e);
            }
        };

        let resolved_str = resolved.to_string_lossy().to_string();

        // Check cache first
        if let Some(namespace) = self.module_cache.get(&resolved_str) {
            return promise.resolve(namespace.clone());
        }

        // Check for circular dependency
        if self.loading.contains_key(&resolved_str) {
            return promise.reject(ImportError::CircularDependency(resolved_str));
        }

        // Mark as loading
        self.loading.insert(resolved_str.clone(), true);

        // Load and compile the module
        let result = self.load_module(&resolved);

        // Remove from loading set
        self.loading.remove(&resolved_str);

        match result {
            Ok(namespace) => {
                // Cache the result
                self.module_cache.insert(resolved_str, namespace.clone());
                promise.resolve(namespace)
            }
            Err(e) => promise.reject(e),
        }
    }

    /// Load and compile a module from disk
    /// 
    /// Handles ESM/CJS interop:
    /// - ESM modules: exports are directly available
    /// - CJS modules: module.exports becomes the default export
    /// - JSON modules: parsed JSON becomes the default export
    /// 
    /// # Error Handling
    /// - Returns ImportError::ModuleNotFound if the file doesn't exist
    /// - Returns ImportError::SyntaxError if the module has syntax errors
    /// - Returns ImportError::EvaluationError for other evaluation failures
    fn load_module(&mut self, path: &Path) -> Result<ModuleNamespace, ImportError> {
        // Check if file exists first (Requirement 2.4)
        if !path.exists() {
            return Err(ImportError::ModuleNotFound(format!(
                "Cannot find module '{}'",
                path.display()
            )));
        }

        // Read the source file
        let source = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ImportError::ModuleNotFound(format!("Cannot find module '{}'", path.display()))
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                ImportError::EvaluationError(format!(
                    "Permission denied reading module '{}'",
                    path.display()
                ))
            } else {
                ImportError::EvaluationError(format!(
                    "Failed to read module '{}': {}",
                    path.display(),
                    e
                ))
            }
        })?;

        // Determine module type from extension and content
        let module_type = self.detect_module_type(path, &source);

        // Create the module namespace
        let url = path.to_string_lossy().to_string();
        let mut namespace = ModuleNamespace::new(url, module_type.clone());

        // Handle different module types
        match module_type {
            ModuleType::ESModule => {
                // ESM: Parse and validate syntax, then extract exports
                self.load_esm_module(&mut namespace, &source, path)?;
            }
            ModuleType::CommonJS => {
                // CJS: Validate syntax and set up module.exports as default
                self.load_cjs_module(&mut namespace, &source, path)?;
            }
            ModuleType::JSON => {
                // JSON: Parse and validate JSON syntax
                self.load_json_module(&mut namespace, &source)?;
            }
            ModuleType::WASM => {
                // WASM: not yet supported
                return Err(ImportError::EvaluationError(
                    "WASM modules are not yet supported".to_string(),
                ));
            }
        }

        // Mark as evaluated
        namespace.mark_evaluated();

        Ok(namespace)
    }

    /// Load an ESM module
    /// 
    /// ESM modules have named exports and optionally a default export.
    /// When importing ESM from CJS, all exports are available.
    /// 
    /// # Error Handling (Requirement 2.5)
    /// Returns SyntaxError if the module has syntax errors.
    fn load_esm_module(&self, namespace: &mut ModuleNamespace, source: &str, path: &Path) -> Result<(), ImportError> {
        // Validate syntax using OXC parser (Requirement 2.5)
        self.validate_javascript_syntax(source, path, true)?;
        
        // Parse the source to extract exports
        // For now, we do a simple regex-based extraction
        // Full parsing will be done by the JIT compiler
        // Look for export statements
        let export_regex = regex_lite::Regex::new(r"export\s+(?:const|let|var|function|class)\s+(\w+)")
            .map_err(|e| ImportError::EvaluationError(e.to_string()))?;
        
        for cap in export_regex.captures_iter(source) {
            if let Some(name) = cap.get(1) {
                // Add a placeholder export (actual value will be set during evaluation)
                namespace.add_export(name.as_str().to_string(), f64::NAN);
            }
        }
        
        // Look for default export
        if source.contains("export default") {
            namespace.add_export("default".to_string(), f64::NAN);
        }
        
        // Look for re-exports: export { x } from 'y'
        // These will be resolved during full compilation
        
        Ok(())
    }

    /// Load a CommonJS module
    /// 
    /// CJS modules use module.exports for their exports.
    /// When importing CJS from ESM:
    /// - module.exports becomes the default export
    /// - Named properties of module.exports become named exports
    /// 
    /// # Error Handling (Requirement 2.5)
    /// Returns SyntaxError if the module has syntax errors.
    fn load_cjs_module(&self, namespace: &mut ModuleNamespace, source: &str, path: &Path) -> Result<(), ImportError> {
        // Validate syntax using OXC parser (Requirement 2.5)
        self.validate_javascript_syntax(source, path, false)?;
        
        // For CJS modules, we create a default export that will hold module.exports
        // The actual value will be set during evaluation
        namespace.add_export("default".to_string(), f64::NAN);
        
        // CJS modules can also have named exports if module.exports is an object
        // These will be extracted during evaluation
        
        Ok(())
    }

    /// Load a JSON module
    /// 
    /// JSON modules have a single default export containing the parsed JSON.
    fn load_json_module(&self, namespace: &mut ModuleNamespace, source: &str) -> Result<(), ImportError> {
        // Parse the JSON to validate it
        let _: serde_json::Value = serde_json::from_str(source)
            .map_err(|e| ImportError::SyntaxError(format!("Invalid JSON: {}", e)))?;
        
        // The parsed JSON becomes the default export
        // The actual value will be stored during evaluation
        namespace.add_export("default".to_string(), f64::NAN);
        
        Ok(())
    }

    /// Validate JavaScript/TypeScript syntax using OXC parser
    /// 
    /// # Arguments
    /// * `source` - The source code to validate
    /// * `path` - The file path (used for error messages)
    /// * `is_module` - Whether to parse as ESM (true) or script/CJS (false)
    /// 
    /// # Returns
    /// Ok(()) if syntax is valid, Err(ImportError::SyntaxError) otherwise
    /// 
    /// # Requirements
    /// - 2.5: IF the imported module has syntax errors, THEN THE Runtime SHALL reject the Promise with a SyntaxError
    fn validate_javascript_syntax(&self, source: &str, path: &Path, is_module: bool) -> Result<(), ImportError> {
        use oxc_allocator::Allocator;
        use oxc_parser::Parser;
        use oxc_span::SourceType;
        
        let allocator = Allocator::default();
        
        // Determine source type from file extension
        let source_type = if is_module {
            SourceType::mjs()
        } else {
            // For CJS, parse as script
            SourceType::cjs()
        };
        
        // Adjust source type based on file extension
        let source_type = if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext {
                "ts" | "mts" | "cts" => source_type.with_typescript(true),
                "tsx" => source_type.with_typescript(true).with_jsx(true),
                "jsx" => source_type.with_jsx(true),
                _ => source_type,
            }
        } else {
            source_type
        };
        
        // Parse the source
        let parser = Parser::new(&allocator, source, source_type);
        let result = parser.parse();
        
        // Check for syntax errors
        if !result.errors.is_empty() {
            // Format the first error with location information
            let first_error = &result.errors[0];
            let error_msg = format!(
                "SyntaxError in '{}': {}",
                path.display(),
                first_error
            );
            return Err(ImportError::SyntaxError(error_msg));
        }
        
        Ok(())
    }

    /// Detect the module type from file extension and content
    fn detect_module_type(&self, path: &Path, source: &str) -> ModuleType {
        // Check extension first
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext {
                "mjs" | "mts" => return ModuleType::ESModule,
                "cjs" | "cts" => return ModuleType::CommonJS,
                "json" => return ModuleType::JSON,
                "wasm" => return ModuleType::WASM,
                _ => {}
            }
        }

        // Check for ESM syntax
        if source.contains("import ") || source.contains("export ") {
            return ModuleType::ESModule;
        }

        // Check for CommonJS syntax
        if source.contains("require(") || source.contains("module.exports") {
            return ModuleType::CommonJS;
        }

        // Check package.json for type field
        if let Some(pkg_dir) = self.find_package_dir(path) {
            let pkg_json_path = pkg_dir.join("package.json");
            if let Ok(content) = std::fs::read_to_string(&pkg_json_path) {
                if let Ok(pkg) = PackageJson::parse(&content) {
                    if pkg.is_esm() {
                        return ModuleType::ESModule;
                    }
                }
            }
        }

        // Default to CommonJS for .js files without ESM syntax
        ModuleType::CommonJS
    }

    /// Find the nearest directory containing package.json
    fn find_package_dir(&self, from: &Path) -> Option<PathBuf> {
        let mut current = from.parent()?;
        loop {
            if current.join("package.json").exists() {
                return Some(current.to_path_buf());
            }
            current = current.parent()?;
        }
    }

    /// Check if a module is cached
    pub fn is_cached(&self, resolved_path: &str) -> bool {
        self.module_cache.contains_key(resolved_path)
    }

    /// Get a cached module namespace
    pub fn get_cached(&self, resolved_path: &str) -> Option<&ModuleNamespace> {
        self.module_cache.get(resolved_path)
    }

    /// Clear the module cache
    pub fn clear_cache(&mut self) {
        self.module_cache.clear();
    }

    /// Get the number of cached modules
    pub fn cache_size(&self) -> usize {
        self.module_cache.len()
    }

    /// Get a promise by ID
    pub fn get_promise(&self, id: u64) -> Option<&ImportPromise> {
        self.pending_promises.get(&id)
    }

    /// Remove a promise by ID
    pub fn remove_promise(&mut self, id: u64) -> Option<ImportPromise> {
        self.pending_promises.remove(&id)
    }
}

// ============================================================================
// Thread-safe global loader for JIT integration
// ============================================================================

use std::sync::OnceLock;

/// Global dynamic import loader instance
static DYNAMIC_IMPORT_LOADER: OnceLock<Mutex<DynamicImportLoader>> = OnceLock::new();

/// Get the global dynamic import loader
pub fn get_dynamic_import_loader() -> &'static Mutex<DynamicImportLoader> {
    DYNAMIC_IMPORT_LOADER.get_or_init(|| Mutex::new(DynamicImportLoader::new()))
}

/// Initialize the global dynamic import loader with custom configuration
pub fn init_dynamic_import_loader(loader: DynamicImportLoader) {
    let _ = DYNAMIC_IMPORT_LOADER.set(Mutex::new(loader));
}

// ============================================================================
// Built-in functions for JIT integration
// ============================================================================

/// Built-in function to perform dynamic import (called from JIT-compiled code)
///
/// # Arguments
/// * `specifier_id` - String ID of the module specifier (encoded as f64)
/// * `referrer_id` - String ID of the referrer path (encoded as f64)
///
/// # Returns
/// Promise ID (as f64) that can be used to track the import
///
/// # Requirements
/// - 2.1: WHEN `import(specifier)` is called with a valid module path, THE Runtime SHALL return a Promise
/// - 2.2: WHEN `import(specifier)` is called with a relative path, THE Runtime SHALL resolve it relative to the importing module
/// - 2.3: WHEN `import(specifier)` is called with a bare specifier, THE Runtime SHALL resolve it using Node.js module resolution
#[no_mangle]
pub extern "C" fn builtin_dynamic_import(specifier_id: f64, referrer_id: f64) -> f64 {
    // Constants for value tagging (must match codegen.rs)
    const STRING_TAG_OFFSET: f64 = 1_000_000.0;
    const OBJECT_TAG_OFFSET: f64 = 3_000_000.0;
    const PROMISE_TAG_OFFSET: f64 = 5_000_000.0;
    
    fn is_string_id(value: f64) -> bool {
        value < -STRING_TAG_OFFSET + 1.0 
            && value >= -STRING_TAG_OFFSET - 1_000_000.0 
            && value.fract() == 0.0
    }
    
    fn decode_string_id(value: f64) -> u64 {
        (-(value + STRING_TAG_OFFSET)) as u64
    }
    
    fn encode_promise_id(id: u64) -> f64 {
        -(id as f64 + PROMISE_TAG_OFFSET)
    }
    
    fn encode_object_id(id: u64) -> f64 {
        -(id as f64 + OBJECT_TAG_OFFSET)
    }
    
    fn encode_string_id(id: u64) -> f64 {
        -(id as f64 + STRING_TAG_OFFSET)
    }
    
    // Use the thread-local heap access pattern from codegen.rs
    // We need to access the heap through the builtin functions
    
    // Get the specifier string
    let specifier = if is_string_id(specifier_id) {
        let id = decode_string_id(specifier_id);
        // Call the builtin to get the string
        let ptr = unsafe { builtin_get_string_ptr(id) };
        if ptr.is_null() {
            return f64::NAN; // Invalid string ID
        }
        unsafe {
            let len = builtin_get_string_len(id);
            let slice = std::slice::from_raw_parts(ptr, len);
            String::from_utf8_lossy(slice).to_string()
        }
    } else {
        return f64::NAN; // Invalid specifier type
    };
    
    // Get the referrer path
    let referrer = if is_string_id(referrer_id) {
        let id = decode_string_id(referrer_id);
        let ptr = unsafe { builtin_get_string_ptr(id) };
        if ptr.is_null() {
            get_default_referrer()
        } else {
            unsafe {
                let len = builtin_get_string_len(id);
                let slice = std::slice::from_raw_parts(ptr, len);
                String::from_utf8_lossy(slice).to_string()
            }
        }
    } else if referrer_id.is_nan() {
        get_default_referrer()
    } else {
        return f64::NAN; // Invalid referrer type
    };
    
    // Create a Promise
    let promise_id = unsafe { builtin_create_promise_raw() };
    
    // Get the dynamic import loader and perform the import
    let loader = get_dynamic_import_loader();
    let mut loader = loader.lock().unwrap();
    
    let import_promise = loader.import(&specifier, &referrer);
    
    // Update the promise based on the import result
    match import_promise.state {
        PromiseState::Fulfilled => {
            if let Some(namespace) = import_promise.value {
                // Create a module namespace object
                let namespace_id = unsafe { builtin_create_object_raw() };
                
                // Add exports to the namespace object
                for (name, value) in namespace.exports {
                    let name_id = unsafe { builtin_allocate_string_raw(name.as_ptr(), name.len()) };
                    unsafe { builtin_set_object_property(namespace_id, name_id, value) };
                }
                
                // Resolve the promise with the namespace object
                let namespace_value = encode_object_id(namespace_id);
                unsafe { builtin_resolve_promise(promise_id, namespace_value) };
            }
        }
        PromiseState::Rejected => {
            if let Some(error) = import_promise.error {
                let error_msg = error.to_string();
                let error_id = unsafe { builtin_allocate_string_raw(error_msg.as_ptr(), error_msg.len()) };
                let error_value = encode_string_id(error_id);
                unsafe { builtin_reject_promise(promise_id, error_value) };
            }
        }
        PromiseState::Pending => {
            // Promise is still pending - shouldn't happen with sync loading
            // Leave the promise in pending state
        }
    }
    
    // Return the promise ID encoded as f64
    encode_promise_id(promise_id)
}

/// Get the default referrer path (current working directory)
fn get_default_referrer() -> String {
    std::env::current_dir()
        .map(|p| p.join("main.js").to_string_lossy().to_string())
        .unwrap_or_else(|_| "./main.js".to_string())
}

// External builtin functions from codegen.rs
// These are defined with #[no_mangle] in codegen.rs
extern "C" {
    fn builtin_get_string_ptr(id: u64) -> *const u8;
    fn builtin_get_string_len(id: u64) -> usize;
    fn builtin_allocate_string_raw(ptr: *const u8, len: usize) -> u64;
    fn builtin_create_object_raw() -> u64;
    fn builtin_set_object_property(obj_id: u64, name_id: u64, value: f64);
    fn builtin_create_promise_raw() -> u64;
    fn builtin_resolve_promise(promise_id: u64, value: f64);
    fn builtin_reject_promise(promise_id: u64, error: f64);
}

/// Built-in function to check if a dynamic import promise is resolved
///
/// # Arguments
/// * `promise_id` - The promise ID returned by builtin_dynamic_import
///
/// # Returns
/// 1.0 if resolved, 0.0 if pending, -1.0 if rejected
#[no_mangle]
pub extern "C" fn builtin_dynamic_import_status(promise_id: f64) -> f64 {
    let loader = get_dynamic_import_loader();
    let loader = loader.lock().unwrap();
    
    let id = promise_id as u64;
    match loader.get_promise(id) {
        Some(promise) => {
            if promise.is_fulfilled() {
                1.0
            } else if promise.is_rejected() {
                -1.0
            } else {
                0.0
            }
        }
        None => f64::NAN, // Promise not found
    }
}

/// Built-in function to get the module namespace from a resolved import promise
///
/// # Arguments
/// * `promise_id` - The promise ID
///
/// # Returns
/// Object ID of the module namespace, or NaN if not resolved
#[no_mangle]
pub extern "C" fn builtin_dynamic_import_result(_promise_id: f64) -> f64 {
    // This will be integrated with the runtime heap in task 3.2
    // For now, return NaN
    f64::NAN
}

/// Built-in function to get the error from a rejected import promise
///
/// # Arguments
/// * `promise_id` - The promise ID
///
/// # Returns
/// String ID of the error message, or NaN if not rejected
#[no_mangle]
pub extern "C" fn builtin_dynamic_import_error(_promise_id: f64) -> f64 {
    // This will be integrated with the runtime heap in task 3.2
    // For now, return NaN
    f64::NAN
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_module(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_module_namespace_creation() {
        let namespace = ModuleNamespace::new("test.js".to_string(), ModuleType::ESModule);
        assert_eq!(namespace.url, "test.js");
        assert!(namespace.exports.is_empty());
        assert!(namespace.default.is_none());
        assert!(!namespace.evaluated);
    }

    #[test]
    fn test_module_namespace_exports() {
        let mut namespace = ModuleNamespace::new("test.js".to_string(), ModuleType::ESModule);
        namespace.add_export("foo".to_string(), 42.0);
        namespace.add_export("default".to_string(), 100.0);

        assert_eq!(namespace.get_export("foo"), Some(42.0));
        assert_eq!(namespace.get_export("default"), Some(100.0));
        assert!(namespace.has_default());
        assert_eq!(namespace.default, Some(100.0));
    }

    #[test]
    fn test_import_promise_states() {
        let promise = ImportPromise::pending(1);
        assert!(promise.is_pending());
        assert!(!promise.is_fulfilled());
        assert!(!promise.is_rejected());

        let namespace = ModuleNamespace::new("test.js".to_string(), ModuleType::ESModule);
        let resolved = promise.resolve(namespace);
        assert!(resolved.is_fulfilled());
        assert!(!resolved.is_pending());

        let promise2 = ImportPromise::pending(2);
        let rejected = promise2.reject(ImportError::ModuleNotFound("test".to_string()));
        assert!(rejected.is_rejected());
        assert!(!rejected.is_pending());
    }

    #[test]
    fn test_dynamic_import_loader_creation() {
        let loader = DynamicImportLoader::new();
        assert_eq!(loader.cache_size(), 0);
    }

    #[test]
    fn test_resolve_relative_path() {
        let temp_dir = TempDir::new().unwrap();
        let module_path = create_test_module(
            temp_dir.path(),
            "module.js",
            "export const foo = 42;",
        );
        let referrer_path = create_test_module(
            temp_dir.path(),
            "main.js",
            "import { foo } from './module.js';",
        );

        let mut loader = DynamicImportLoader::new();
        let resolved = loader.resolve("./module.js", &referrer_path);
        
        assert!(resolved.is_ok());
        let resolved_path = resolved.unwrap();
        assert!(resolved_path.exists());
    }

    #[test]
    fn test_resolve_invalid_specifier() {
        let mut loader = DynamicImportLoader::new();
        let result = loader.resolve("", Path::new("/test/main.js"));
        
        assert!(result.is_err());
        match result {
            Err(ImportError::InvalidSpecifier(_)) => {}
            _ => panic!("Expected InvalidSpecifier error"),
        }
    }

    #[test]
    fn test_import_caching() {
        let temp_dir = TempDir::new().unwrap();
        let module_path = create_test_module(
            temp_dir.path(),
            "cached.js",
            "export const value = 123;",
        );
        let referrer_path = create_test_module(
            temp_dir.path(),
            "main.js",
            "",
        );

        let mut loader = DynamicImportLoader::new();
        
        // First import
        let promise1 = loader.import("./cached.js", referrer_path.to_str().unwrap());
        assert!(promise1.is_fulfilled());
        
        // Second import should use cache
        let promise2 = loader.import("./cached.js", referrer_path.to_str().unwrap());
        assert!(promise2.is_fulfilled());
        
        // Cache should have one entry
        assert_eq!(loader.cache_size(), 1);
    }

    #[test]
    fn test_import_module_not_found() {
        let mut loader = DynamicImportLoader::new();
        let promise = loader.import("./nonexistent.js", "/test/main.js");
        
        assert!(promise.is_rejected());
        match promise.error {
            Some(ImportError::ResolutionError(_)) | Some(ImportError::ModuleNotFound(_)) => {}
            _ => panic!("Expected ModuleNotFound or ResolutionError"),
        }
    }

    #[test]
    fn test_detect_module_type_by_extension() {
        let loader = DynamicImportLoader::new();
        
        assert_eq!(
            loader.detect_module_type(Path::new("test.mjs"), ""),
            ModuleType::ESModule
        );
        assert_eq!(
            loader.detect_module_type(Path::new("test.cjs"), ""),
            ModuleType::CommonJS
        );
        assert_eq!(
            loader.detect_module_type(Path::new("test.json"), ""),
            ModuleType::JSON
        );
    }

    #[test]
    fn test_detect_module_type_by_content() {
        let loader = DynamicImportLoader::new();
        
        assert_eq!(
            loader.detect_module_type(Path::new("test.js"), "import { foo } from 'bar';"),
            ModuleType::ESModule
        );
        assert_eq!(
            loader.detect_module_type(Path::new("test.js"), "export const x = 1;"),
            ModuleType::ESModule
        );
        assert_eq!(
            loader.detect_module_type(Path::new("test.js"), "const x = require('foo');"),
            ModuleType::CommonJS
        );
        assert_eq!(
            loader.detect_module_type(Path::new("test.js"), "module.exports = {};"),
            ModuleType::CommonJS
        );
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let _module_path = create_test_module(
            temp_dir.path(),
            "test.js",
            "export const x = 1;",
        );
        let referrer_path = create_test_module(
            temp_dir.path(),
            "main.js",
            "",
        );

        let mut loader = DynamicImportLoader::new();
        loader.import("./test.js", referrer_path.to_str().unwrap());
        
        assert_eq!(loader.cache_size(), 1);
        
        loader.clear_cache();
        assert_eq!(loader.cache_size(), 0);
    }
}
