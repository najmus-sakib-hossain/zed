//! CPython API Function Registry
//!
//! This module provides a comprehensive registry of CPython API functions,
//! categorized by priority (critical, important, optional) and category.
//!
//! ## Features
//!
//! - Complete list of CPython API functions
//! - Priority categorization for implementation planning
//! - Category grouping for organization
//! - Coverage tracking and reporting
//! - Historical progress tracking

use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cpython_compat::{ApiCategory, ApiPriority};

// =============================================================================
// API Function Definition
// =============================================================================

/// Definition of a CPython API function
#[derive(Debug, Clone)]
pub struct ApiFunctionDef {
    /// Function name (e.g., "Py_IncRef")
    pub name: String,
    /// Category (e.g., ObjectCore, TypeSystem)
    pub category: ApiCategory,
    /// Priority (Critical, Important, Optional)
    pub priority: ApiPriority,
    /// Brief description
    pub description: String,
    /// Whether this function is implemented
    pub implemented: bool,
    /// Extensions that use this function
    pub used_by: Vec<String>,
}

impl ApiFunctionDef {
    /// Create a new API function definition
    pub fn new(
        name: impl Into<String>,
        category: ApiCategory,
        priority: ApiPriority,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            category,
            priority,
            description: description.into(),
            implemented: false,
            used_by: Vec::new(),
        }
    }

    /// Mark as implemented
    pub fn mark_implemented(mut self) -> Self {
        self.implemented = true;
        self
    }

    /// Add an extension that uses this function
    pub fn add_user(&mut self, extension: impl Into<String>) {
        let ext = extension.into();
        if !self.used_by.contains(&ext) {
            self.used_by.push(ext);
        }
    }
}

// =============================================================================
// API Registry
// =============================================================================

/// Registry of all CPython API functions
pub struct ApiRegistry {
    /// All registered functions
    functions: HashMap<String, ApiFunctionDef>,
    /// Functions by category
    by_category: HashMap<ApiCategory, Vec<String>>,
    /// Functions by priority
    by_priority: HashMap<ApiPriority, Vec<String>>,
    /// Usage tracking (function -> extensions that called it)
    usage: RwLock<HashMap<String, HashSet<String>>>,
}

impl ApiRegistry {
    /// Create a new registry with all known CPython API functions
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
            by_category: HashMap::new(),
            by_priority: HashMap::new(),
            usage: RwLock::new(HashMap::new()),
        };

        registry.register_all_functions();
        registry
    }

    /// Register all known CPython API functions
    fn register_all_functions(&mut self) {
        // Critical Object Core functions
        self.register_critical_object_core();

        // Critical Type System functions
        self.register_critical_type_system();

        // Critical Arg Parsing functions
        self.register_critical_arg_parsing();

        // Critical Error Handling functions
        self.register_critical_error_handling();

        // Critical GIL functions
        self.register_critical_gil();

        // Important Protocol functions
        self.register_important_protocols();

        // Important Memory functions
        self.register_important_memory();

        // Important Import/Module functions
        self.register_important_import_module();

        // Optional functions
        self.register_optional_functions();
    }

    fn register_critical_object_core(&mut self) {
        let funcs = vec![
            ("Py_IncRef", "Increment reference count", true),
            ("Py_DecRef", "Decrement reference count", true),
            ("Py_REFCNT", "Get reference count", true),
            ("Py_TYPE", "Get object type", true),
            ("Py_None", "Get None singleton", true),
            ("Py_True", "Get True singleton", true),
            ("Py_False", "Get False singleton", true),
            ("PyObject_GetAttrString", "Get attribute by name", true),
            ("PyObject_SetAttrString", "Set attribute by name", true),
            ("PyObject_HasAttrString", "Check attribute exists", true),
            ("PyObject_GetAttr", "Get attribute", true),
            ("PyObject_SetAttr", "Set attribute", true),
            ("PyObject_Call", "Call object", true),
            ("PyObject_CallObject", "Call with args tuple", false),
            ("PyObject_CallFunction", "Call with format args", false),
            ("PyObject_CallMethod", "Call method", false),
            ("PyObject_Repr", "Get repr string", true),
            ("PyObject_Str", "Get str string", true),
            ("PyObject_Hash", "Get hash value", true),
            ("PyObject_IsTrue", "Check truthiness", true),
            ("PyObject_RichCompare", "Rich comparison", true),
            ("PyObject_RichCompareBool", "Rich comparison bool", false),
            ("PyObject_GetIter", "Get iterator", false),
            ("PyObject_Length", "Get length", false),
            ("PyObject_Size", "Get size", false),
        ];

        for (name, desc, implemented) in funcs {
            let mut def =
                ApiFunctionDef::new(name, ApiCategory::ObjectCore, ApiPriority::Critical, desc);
            if implemented {
                def = def.mark_implemented();
            }
            self.register(def);
        }
    }

    fn register_critical_type_system(&mut self) {
        let funcs = vec![
            ("PyType_Ready", "Initialize type object", false),
            ("PyType_GenericNew", "Generic type __new__", false),
            ("PyType_GenericAlloc", "Generic type allocation", false),
            ("PyType_IsSubtype", "Check subtype relationship", false),
            ("PyLong_FromLong", "Create int from long", false),
            ("PyLong_AsLong", "Get long from int", false),
            ("PyLong_FromLongLong", "Create int from long long", false),
            ("PyLong_AsLongLong", "Get long long from int", false),
            ("PyFloat_FromDouble", "Create float from double", false),
            ("PyFloat_AsDouble", "Get double from float", false),
            ("PyUnicode_FromString", "Create str from C string", false),
            ("PyUnicode_AsUTF8", "Get UTF-8 from str", false),
            ("PyUnicode_AsUTF8AndSize", "Get UTF-8 with size", false),
            ("PyBytes_FromString", "Create bytes from C string", false),
            ("PyBytes_AsString", "Get C string from bytes", false),
            ("PyBytes_Size", "Get bytes size", false),
            ("PyList_New", "Create new list", false),
            ("PyList_Size", "Get list size", false),
            ("PyList_GetItem", "Get list item", false),
            ("PyList_SetItem", "Set list item", false),
            ("PyList_Append", "Append to list", false),
            ("PyTuple_New", "Create new tuple", false),
            ("PyTuple_Size", "Get tuple size", false),
            ("PyTuple_GetItem", "Get tuple item", false),
            ("PyTuple_SetItem", "Set tuple item", false),
            ("PyDict_New", "Create new dict", false),
            ("PyDict_Size", "Get dict size", false),
            ("PyDict_GetItem", "Get dict item", false),
            ("PyDict_SetItem", "Set dict item", false),
            ("PyDict_GetItemString", "Get dict item by string key", false),
            ("PyDict_SetItemString", "Set dict item by string key", false),
        ];

        for (name, desc, implemented) in funcs {
            let mut def =
                ApiFunctionDef::new(name, ApiCategory::TypeSystem, ApiPriority::Critical, desc);
            if implemented {
                def = def.mark_implemented();
            }
            self.register(def);
        }
    }

    fn register_critical_arg_parsing(&mut self) {
        let funcs = vec![
            ("PyArg_ParseTuple", "Parse positional args", true),
            ("PyArg_ParseTupleAndKeywords", "Parse args with keywords", true),
            ("PyArg_UnpackTuple", "Unpack tuple args", false),
            ("Py_BuildValue", "Build return value", true),
            ("PyArg_VaParse", "Parse with va_list", false),
            ("PyArg_VaParseTupleAndKeywords", "Parse keywords with va_list", false),
        ];

        for (name, desc, implemented) in funcs {
            let mut def =
                ApiFunctionDef::new(name, ApiCategory::ArgParsing, ApiPriority::Critical, desc);
            if implemented {
                def = def.mark_implemented();
            }
            self.register(def);
        }
    }

    fn register_critical_error_handling(&mut self) {
        let funcs = vec![
            ("PyErr_SetString", "Set exception with message", false),
            ("PyErr_SetObject", "Set exception with value", false),
            ("PyErr_Occurred", "Check if exception set", false),
            ("PyErr_Clear", "Clear exception", false),
            ("PyErr_Fetch", "Fetch exception info", false),
            ("PyErr_Restore", "Restore exception info", false),
            ("PyErr_NormalizeException", "Normalize exception", false),
            ("PyErr_Format", "Set exception with format", false),
            ("PyErr_NoMemory", "Set MemoryError", false),
            ("PyErr_BadArgument", "Set TypeError for bad arg", false),
        ];

        for (name, desc, implemented) in funcs {
            let mut def =
                ApiFunctionDef::new(name, ApiCategory::ErrorHandling, ApiPriority::Critical, desc);
            if implemented {
                def = def.mark_implemented();
            }
            self.register(def);
        }
    }

    fn register_critical_gil(&mut self) {
        let funcs = vec![
            ("PyGILState_Ensure", "Acquire GIL", true),
            ("PyGILState_Release", "Release GIL", true),
            ("PyGILState_Check", "Check GIL held", true),
            ("Py_BEGIN_ALLOW_THREADS", "Release GIL macro", false),
            ("Py_END_ALLOW_THREADS", "Reacquire GIL macro", false),
        ];

        for (name, desc, implemented) in funcs {
            let mut def = ApiFunctionDef::new(name, ApiCategory::GIL, ApiPriority::Critical, desc);
            if implemented {
                def = def.mark_implemented();
            }
            self.register(def);
        }
    }

    fn register_important_protocols(&mut self) {
        // Number Protocol
        let number_funcs = vec![
            ("PyNumber_Add", "Add two numbers"),
            ("PyNumber_Subtract", "Subtract two numbers"),
            ("PyNumber_Multiply", "Multiply two numbers"),
            ("PyNumber_TrueDivide", "True divide"),
            ("PyNumber_FloorDivide", "Floor divide"),
            ("PyNumber_Remainder", "Modulo"),
            ("PyNumber_Power", "Power"),
            ("PyNumber_Negative", "Negate"),
            ("PyNumber_Positive", "Positive"),
            ("PyNumber_Absolute", "Absolute value"),
            ("PyNumber_Invert", "Bitwise invert"),
            ("PyNumber_Lshift", "Left shift"),
            ("PyNumber_Rshift", "Right shift"),
            ("PyNumber_And", "Bitwise and"),
            ("PyNumber_Or", "Bitwise or"),
            ("PyNumber_Xor", "Bitwise xor"),
        ];

        for (name, desc) in number_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::NumberProtocol,
                ApiPriority::Important,
                desc,
            ));
        }

        // Sequence Protocol
        let seq_funcs = vec![
            ("PySequence_Length", "Get sequence length"),
            ("PySequence_Concat", "Concatenate sequences"),
            ("PySequence_Repeat", "Repeat sequence"),
            ("PySequence_GetItem", "Get item by index"),
            ("PySequence_SetItem", "Set item by index"),
            ("PySequence_Contains", "Check containment"),
            ("PySequence_Index", "Find index of item"),
            ("PySequence_Count", "Count occurrences"),
            ("PySequence_List", "Convert to list"),
            ("PySequence_Tuple", "Convert to tuple"),
        ];

        for (name, desc) in seq_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::SequenceProtocol,
                ApiPriority::Important,
                desc,
            ));
        }

        // Mapping Protocol
        let map_funcs = vec![
            ("PyMapping_Length", "Get mapping length"),
            ("PyMapping_Keys", "Get keys"),
            ("PyMapping_Values", "Get values"),
            ("PyMapping_Items", "Get items"),
            ("PyMapping_GetItemString", "Get item by string key"),
            ("PyMapping_SetItemString", "Set item by string key"),
            ("PyMapping_HasKey", "Check key exists"),
            ("PyMapping_HasKeyString", "Check string key exists"),
        ];

        for (name, desc) in map_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::MappingProtocol,
                ApiPriority::Important,
                desc,
            ));
        }

        // Buffer Protocol
        let buf_funcs = vec![
            ("PyObject_GetBuffer", "Get buffer", true),
            ("PyBuffer_Release", "Release buffer", true),
            ("PyObject_CheckBuffer", "Check buffer support", true),
            ("PyBuffer_IsContiguous", "Check contiguous", true),
            ("PyBuffer_FillInfo", "Fill buffer info", false),
            ("PyBuffer_ToContiguous", "Copy to contiguous", false),
            ("PyBuffer_FromContiguous", "Copy from contiguous", false),
        ];

        for (name, desc, implemented) in buf_funcs {
            let mut def = ApiFunctionDef::new(
                name,
                ApiCategory::BufferProtocol,
                ApiPriority::Important,
                desc,
            );
            if implemented {
                def = def.mark_implemented();
            }
            self.register(def);
        }
    }

    fn register_important_memory(&mut self) {
        let funcs = vec![
            ("PyMem_Malloc", "Allocate memory"),
            ("PyMem_Realloc", "Reallocate memory"),
            ("PyMem_Free", "Free memory"),
            ("PyObject_Malloc", "Allocate object memory"),
            ("PyObject_Realloc", "Reallocate object memory"),
            ("PyObject_Free", "Free object memory"),
            ("PyMem_RawMalloc", "Raw malloc"),
            ("PyMem_RawRealloc", "Raw realloc"),
            ("PyMem_RawFree", "Raw free"),
        ];

        for (name, desc) in funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::MemoryAlloc,
                ApiPriority::Important,
                desc,
            ));
        }
    }

    fn register_important_import_module(&mut self) {
        let import_funcs = vec![
            ("PyImport_Import", "Import module"),
            ("PyImport_ImportModule", "Import module by name"),
            ("PyImport_ImportModuleLevel", "Import with level"),
            ("PyImport_GetModule", "Get loaded module"),
            ("PyImport_AddModule", "Add module to sys.modules"),
            ("PyImport_ExecCodeModule", "Execute code as module"),
        ];

        for (name, desc) in import_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::Import,
                ApiPriority::Important,
                desc,
            ));
        }

        let module_funcs = vec![
            ("PyModule_Create", "Create module"),
            ("PyModule_Create2", "Create module with API version"),
            ("PyModule_GetDict", "Get module __dict__"),
            ("PyModule_GetName", "Get module __name__"),
            ("PyModule_AddObject", "Add object to module"),
            ("PyModule_AddIntConstant", "Add int constant"),
            ("PyModule_AddStringConstant", "Add string constant"),
        ];

        for (name, desc) in module_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::Module,
                ApiPriority::Important,
                desc,
            ));
        }
    }

    fn register_optional_functions(&mut self) {
        // Iterator functions
        let iter_funcs = vec![
            ("PyIter_Next", "Get next item from iterator"),
            ("PyIter_Check", "Check if object is iterator"),
        ];

        for (name, desc) in iter_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::Other,
                ApiPriority::Optional,
                desc,
            ));
        }

        // Callable functions
        let call_funcs = vec![
            ("PyCallable_Check", "Check if callable"),
            ("PyObject_CallNoArgs", "Call with no args"),
            ("PyObject_CallOneArg", "Call with one arg"),
        ];

        for (name, desc) in call_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::ObjectCore,
                ApiPriority::Optional,
                desc,
            ));
        }

        // Weak reference functions
        let weakref_funcs = vec![
            ("PyWeakref_NewRef", "Create weak reference"),
            ("PyWeakref_NewProxy", "Create weak proxy"),
            ("PyWeakref_GetObject", "Get referenced object"),
        ];

        for (name, desc) in weakref_funcs {
            self.register(ApiFunctionDef::new(
                name,
                ApiCategory::Other,
                ApiPriority::Optional,
                desc,
            ));
        }
    }

    /// Register a function definition
    fn register(&mut self, def: ApiFunctionDef) {
        let name = def.name.clone();
        let category = def.category;
        let priority = def.priority;

        self.functions.insert(name.clone(), def);

        self.by_category.entry(category).or_default().push(name.clone());

        self.by_priority.entry(priority).or_default().push(name);
    }

    /// Get a function definition by name
    pub fn get(&self, name: &str) -> Option<&ApiFunctionDef> {
        self.functions.get(name)
    }

    /// Get all functions in a category
    pub fn get_by_category(&self, category: ApiCategory) -> Vec<&ApiFunctionDef> {
        self.by_category
            .get(&category)
            .map(|names| names.iter().filter_map(|n| self.functions.get(n)).collect())
            .unwrap_or_default()
    }

    /// Get all functions with a priority
    pub fn get_by_priority(&self, priority: ApiPriority) -> Vec<&ApiFunctionDef> {
        self.by_priority
            .get(&priority)
            .map(|names| names.iter().filter_map(|n| self.functions.get(n)).collect())
            .unwrap_or_default()
    }

    /// Get all implemented functions
    pub fn get_implemented(&self) -> Vec<&ApiFunctionDef> {
        self.functions.values().filter(|f| f.implemented).collect()
    }

    /// Get all unimplemented functions
    pub fn get_unimplemented(&self) -> Vec<&ApiFunctionDef> {
        self.functions.values().filter(|f| !f.implemented).collect()
    }

    /// Record that an extension uses a function
    pub fn record_usage(&self, function_name: &str, extension: &str) {
        if let Ok(mut usage) = self.usage.write() {
            usage
                .entry(function_name.to_string())
                .or_default()
                .insert(extension.to_string());
        }
    }

    /// Get extensions that use a function
    pub fn get_users(&self, function_name: &str) -> Vec<String> {
        self.usage
            .read()
            .ok()
            .and_then(|u| u.get(function_name).cloned())
            .map(|s| s.into_iter().collect())
            .unwrap_or_default()
    }

    /// Get total function count
    pub fn total_count(&self) -> usize {
        self.functions.len()
    }

    /// Get implemented count
    pub fn implemented_count(&self) -> usize {
        self.functions.values().filter(|f| f.implemented).count()
    }

    /// Get coverage statistics
    pub fn coverage_stats(&self) -> RegistryCoverageStats {
        let total = self.total_count();
        let implemented = self.implemented_count();

        let mut by_category = HashMap::new();
        for category in [
            ApiCategory::ObjectCore,
            ApiCategory::TypeSystem,
            ApiCategory::NumberProtocol,
            ApiCategory::SequenceProtocol,
            ApiCategory::MappingProtocol,
            ApiCategory::BufferProtocol,
            ApiCategory::ArgParsing,
            ApiCategory::ErrorHandling,
            ApiCategory::MemoryAlloc,
            ApiCategory::GIL,
            ApiCategory::Import,
            ApiCategory::Module,
            ApiCategory::Other,
        ] {
            let funcs = self.get_by_category(category);
            let cat_total = funcs.len();
            let cat_impl = funcs.iter().filter(|f| f.implemented).count();
            by_category.insert(
                category,
                CategoryStats {
                    total: cat_total,
                    implemented: cat_impl,
                },
            );
        }

        let mut by_priority = HashMap::new();
        for priority in [
            ApiPriority::Critical,
            ApiPriority::Important,
            ApiPriority::Optional,
        ] {
            let funcs = self.get_by_priority(priority);
            let pri_total = funcs.len();
            let pri_impl = funcs.iter().filter(|f| f.implemented).count();
            by_priority.insert(
                priority,
                CategoryStats {
                    total: pri_total,
                    implemented: pri_impl,
                },
            );
        }

        RegistryCoverageStats {
            total,
            implemented,
            by_category,
            by_priority,
        }
    }
}

impl Default for ApiRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Coverage Statistics
// =============================================================================

/// Statistics for a category or priority
#[derive(Debug, Clone)]
pub struct CategoryStats {
    /// Total functions in this category
    pub total: usize,
    /// Implemented functions
    pub implemented: usize,
}

impl CategoryStats {
    /// Get coverage percentage
    pub fn coverage_percentage(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.implemented as f64 / self.total as f64) * 100.0
        }
    }
}

/// Registry coverage statistics
#[derive(Debug, Clone)]
pub struct RegistryCoverageStats {
    /// Total functions registered
    pub total: usize,
    /// Total implemented
    pub implemented: usize,
    /// Stats by category
    pub by_category: HashMap<ApiCategory, CategoryStats>,
    /// Stats by priority
    pub by_priority: HashMap<ApiPriority, CategoryStats>,
}

impl RegistryCoverageStats {
    /// Get overall coverage percentage
    pub fn coverage_percentage(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            (self.implemented as f64 / self.total as f64) * 100.0
        }
    }

    /// Generate markdown report
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# CPython API Coverage Report\n\n");
        md.push_str(&format!(
            "**Overall Coverage:** {}/{} ({:.1}%)\n\n",
            self.implemented,
            self.total,
            self.coverage_percentage()
        ));

        md.push_str("## Coverage by Priority\n\n");
        md.push_str("| Priority | Implemented | Total | Coverage |\n");
        md.push_str("|----------|-------------|-------|----------|\n");

        for priority in [
            ApiPriority::Critical,
            ApiPriority::Important,
            ApiPriority::Optional,
        ] {
            if let Some(stats) = self.by_priority.get(&priority) {
                md.push_str(&format!(
                    "| {:?} | {} | {} | {:.1}% |\n",
                    priority,
                    stats.implemented,
                    stats.total,
                    stats.coverage_percentage()
                ));
            }
        }

        md.push_str("\n## Coverage by Category\n\n");
        md.push_str("| Category | Implemented | Total | Coverage |\n");
        md.push_str("|----------|-------------|-------|----------|\n");

        for category in [
            ApiCategory::ObjectCore,
            ApiCategory::TypeSystem,
            ApiCategory::ArgParsing,
            ApiCategory::ErrorHandling,
            ApiCategory::GIL,
            ApiCategory::BufferProtocol,
            ApiCategory::NumberProtocol,
            ApiCategory::SequenceProtocol,
            ApiCategory::MappingProtocol,
            ApiCategory::MemoryAlloc,
            ApiCategory::Import,
            ApiCategory::Module,
            ApiCategory::Other,
        ] {
            if let Some(stats) = self.by_category.get(&category) {
                md.push_str(&format!(
                    "| {:?} | {} | {} | {:.1}% |\n",
                    category,
                    stats.implemented,
                    stats.total,
                    stats.coverage_percentage()
                ));
            }
        }

        md
    }
}

// =============================================================================
// Historical Tracking
// =============================================================================

/// A snapshot of coverage at a point in time
#[derive(Debug, Clone)]
pub struct CoverageSnapshot {
    /// Timestamp (seconds since UNIX epoch)
    pub timestamp: u64,
    /// DX-Py version
    pub version: String,
    /// Coverage stats at this time
    pub stats: RegistryCoverageStats,
}

impl CoverageSnapshot {
    /// Create a new snapshot
    pub fn new(version: impl Into<String>, stats: RegistryCoverageStats) -> Self {
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Self {
            timestamp,
            version: version.into(),
            stats,
        }
    }
}

/// Historical coverage tracker
#[derive(Debug, Clone)]
pub struct CoverageHistory {
    /// All snapshots
    pub snapshots: Vec<CoverageSnapshot>,
}

impl CoverageHistory {
    /// Create a new history tracker
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    /// Add a snapshot
    pub fn add_snapshot(&mut self, snapshot: CoverageSnapshot) {
        self.snapshots.push(snapshot);
    }

    /// Get the latest snapshot
    pub fn latest(&self) -> Option<&CoverageSnapshot> {
        self.snapshots.last()
    }

    /// Get coverage trend (change from previous snapshot)
    pub fn trend(&self) -> Option<CoverageTrend> {
        if self.snapshots.len() < 2 {
            return None;
        }

        let current = &self.snapshots[self.snapshots.len() - 1];
        let previous = &self.snapshots[self.snapshots.len() - 2];

        Some(CoverageTrend {
            previous_coverage: previous.stats.coverage_percentage(),
            current_coverage: current.stats.coverage_percentage(),
            functions_added: current.stats.implemented.saturating_sub(previous.stats.implemented),
            time_elapsed_secs: current.timestamp.saturating_sub(previous.timestamp),
        })
    }

    /// Save history to a JSON file
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        use std::io::Write;

        let json = self.to_json();
        let mut file = std::fs::File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Load history from a JSON file
    pub fn load_from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_json(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        let mut json = String::from("{\n  \"snapshots\": [\n");

        for (i, snapshot) in self.snapshots.iter().enumerate() {
            if i > 0 {
                json.push_str(",\n");
            }
            json.push_str(&format!(
                "    {{\n      \"timestamp\": {},\n      \"version\": \"{}\",\n      \"total\": {},\n      \"implemented\": {},\n      \"coverage_percentage\": {:.2}\n    }}",
                snapshot.timestamp,
                snapshot.version,
                snapshot.stats.total,
                snapshot.stats.implemented,
                snapshot.stats.coverage_percentage()
            ));
        }

        json.push_str("\n  ]\n}");
        json
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, String> {
        let mut history = CoverageHistory::new();

        // Simple JSON parsing for our specific format
        // Find snapshots array
        let snapshots_start = json.find("\"snapshots\"").ok_or("Missing snapshots field")?;
        let array_start = json[snapshots_start..].find('[').ok_or("Missing snapshots array")?;
        let array_end = json[snapshots_start..].rfind(']').ok_or("Missing array end")?;

        let array_content = &json[snapshots_start + array_start + 1..snapshots_start + array_end];

        // Parse each snapshot object
        let mut depth = 0;
        let mut obj_start = None;

        for (i, c) in array_content.char_indices() {
            match c {
                '{' => {
                    if depth == 0 {
                        obj_start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = obj_start {
                            let obj_str = &array_content[start..i + c.len_utf8()];
                            if let Ok(snapshot) = parse_snapshot_json(obj_str) {
                                history.add_snapshot(snapshot);
                            }
                        }
                        obj_start = None;
                    }
                }
                _ => {}
            }
        }

        Ok(history)
    }

    /// Generate trend report
    pub fn trend_report(&self) -> String {
        let mut report = String::new();

        report.push_str("# Coverage Trend Report\n\n");

        if let Some(trend) = self.trend() {
            report.push_str(&format!(
                "**Coverage Change:** {:.1}% → {:.1}% ({:+.1}%)\n",
                trend.previous_coverage,
                trend.current_coverage,
                trend.current_coverage - trend.previous_coverage
            ));
            report.push_str(&format!("**Functions Added:** {}\n", trend.functions_added));
            report
                .push_str(&format!("**Time Elapsed:** {} days\n", trend.time_elapsed_secs / 86400));
        } else {
            report.push_str("*Not enough data for trend analysis*\n");
        }

        report.push_str("\n## Historical Snapshots\n\n");
        report.push_str("| Timestamp | Version | Coverage |\n");
        report.push_str("|-----------|---------|----------|\n");

        for snapshot in &self.snapshots {
            report.push_str(&format!(
                "| {} | {} | {:.1}% |\n",
                snapshot.timestamp,
                snapshot.version,
                snapshot.stats.coverage_percentage()
            ));
        }

        report
    }
}

impl Default for CoverageHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage trend information
#[derive(Debug, Clone)]
pub struct CoverageTrend {
    /// Previous coverage percentage
    pub previous_coverage: f64,
    /// Current coverage percentage
    pub current_coverage: f64,
    /// Number of functions added
    pub functions_added: usize,
    /// Time elapsed between snapshots (seconds)
    pub time_elapsed_secs: u64,
}

// =============================================================================
// JSON Parsing Helpers
// =============================================================================

/// Parse a snapshot from JSON object string
fn parse_snapshot_json(json: &str) -> Result<CoverageSnapshot, String> {
    let timestamp = extract_json_number(json, "timestamp").ok_or("Missing timestamp")? as u64;
    let version = extract_json_string(json, "version").ok_or("Missing version")?;
    let total = extract_json_number(json, "total").ok_or("Missing total")? as usize;
    let implemented =
        extract_json_number(json, "implemented").ok_or("Missing implemented")? as usize;

    // Create a minimal stats object for historical data
    let stats = RegistryCoverageStats {
        total,
        implemented,
        by_category: HashMap::new(),
        by_priority: HashMap::new(),
    };

    Ok(CoverageSnapshot {
        timestamp,
        version,
        stats,
    })
}

/// Extract a string value from JSON
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{}\"", key);
    let key_pos = json.find(&key_pattern)?;
    let after_key = &json[key_pos + key_pattern.len()..];

    // Find the colon and opening quote
    let colon_pos = after_key.find(':')?;
    let after_colon = &after_key[colon_pos + 1..];
    let quote_start = after_colon.find('"')?;
    let after_quote = &after_colon[quote_start + 1..];
    let quote_end = after_quote.find('"')?;

    Some(after_quote[..quote_end].to_string())
}

/// Extract a number value from JSON
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let key_pattern = format!("\"{}\"", key);
    let key_pos = json.find(&key_pattern)?;
    let after_key = &json[key_pos + key_pattern.len()..];

    // Find the colon
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    // Find the end of the number (comma, }, or whitespace)
    let end_pos = after_colon.find([',', '}', '\n']).unwrap_or(after_colon.len());

    after_colon[..end_pos].trim().parse().ok()
}

// =============================================================================
// Global Registry
// =============================================================================

/// Global API registry instance
static API_REGISTRY: std::sync::OnceLock<ApiRegistry> = std::sync::OnceLock::new();

/// Get the global API registry
pub fn get_api_registry() -> &'static ApiRegistry {
    API_REGISTRY.get_or_init(ApiRegistry::new)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ApiRegistry::new();
        assert!(registry.total_count() > 0);
    }

    #[test]
    fn test_registry_has_critical_functions() {
        let registry = ApiRegistry::new();

        // Check some critical functions exist
        assert!(registry.get("Py_IncRef").is_some());
        assert!(registry.get("Py_DecRef").is_some());
        assert!(registry.get("PyArg_ParseTuple").is_some());
        assert!(registry.get("PyGILState_Ensure").is_some());
    }

    #[test]
    fn test_registry_implemented_functions() {
        let registry = ApiRegistry::new();

        let py_incref = registry.get("Py_IncRef").unwrap();
        assert!(py_incref.implemented);

        let py_number_add = registry.get("PyNumber_Add").unwrap();
        assert!(!py_number_add.implemented);
    }

    #[test]
    fn test_registry_by_category() {
        let registry = ApiRegistry::new();

        let object_core = registry.get_by_category(ApiCategory::ObjectCore);
        assert!(!object_core.is_empty());

        let gil = registry.get_by_category(ApiCategory::GIL);
        assert!(!gil.is_empty());
    }

    #[test]
    fn test_registry_by_priority() {
        let registry = ApiRegistry::new();

        let critical = registry.get_by_priority(ApiPriority::Critical);
        assert!(!critical.is_empty());

        let important = registry.get_by_priority(ApiPriority::Important);
        assert!(!important.is_empty());
    }

    #[test]
    fn test_coverage_stats() {
        let registry = ApiRegistry::new();
        let stats = registry.coverage_stats();

        assert!(stats.total > 0);
        assert!(stats.implemented > 0);
        assert!(stats.coverage_percentage() > 0.0);
        assert!(stats.coverage_percentage() <= 100.0);
    }

    #[test]
    fn test_coverage_stats_by_category() {
        let registry = ApiRegistry::new();
        let stats = registry.coverage_stats();

        // GIL functions should have high coverage
        let gil_stats = stats.by_category.get(&ApiCategory::GIL).unwrap();
        assert!(gil_stats.coverage_percentage() > 50.0);
    }

    #[test]
    fn test_coverage_markdown() {
        let registry = ApiRegistry::new();
        let stats = registry.coverage_stats();
        let md = stats.to_markdown();

        assert!(md.contains("CPython API Coverage Report"));
        assert!(md.contains("Coverage by Priority"));
        assert!(md.contains("Coverage by Category"));
    }

    #[test]
    fn test_usage_tracking() {
        let registry = ApiRegistry::new();

        registry.record_usage("Py_IncRef", "numpy");
        registry.record_usage("Py_IncRef", "pandas");
        registry.record_usage("Py_DecRef", "numpy");

        let users = registry.get_users("Py_IncRef");
        assert_eq!(users.len(), 2);
        assert!(users.contains(&"numpy".to_string()));
        assert!(users.contains(&"pandas".to_string()));
    }

    #[test]
    fn test_coverage_history() {
        let registry = ApiRegistry::new();
        let mut history = CoverageHistory::new();

        // Add first snapshot
        let stats1 = registry.coverage_stats();
        history.add_snapshot(CoverageSnapshot::new("0.1.0", stats1));

        // Add second snapshot
        let stats2 = registry.coverage_stats();
        history.add_snapshot(CoverageSnapshot::new("0.2.0", stats2));

        assert_eq!(history.snapshots.len(), 2);
        assert!(history.latest().is_some());
        assert!(history.trend().is_some());
    }

    #[test]
    fn test_trend_report() {
        let registry = ApiRegistry::new();
        let mut history = CoverageHistory::new();

        let stats = registry.coverage_stats();
        history.add_snapshot(CoverageSnapshot::new("0.1.0", stats.clone()));
        history.add_snapshot(CoverageSnapshot::new("0.2.0", stats));

        let report = history.trend_report();
        assert!(report.contains("Coverage Trend Report"));
        assert!(report.contains("Historical Snapshots"));
    }

    #[test]
    fn test_category_stats() {
        let stats = CategoryStats {
            total: 10,
            implemented: 7,
        };

        assert_eq!(stats.coverage_percentage(), 70.0);
    }

    #[test]
    fn test_category_stats_empty() {
        let stats = CategoryStats {
            total: 0,
            implemented: 0,
        };

        assert_eq!(stats.coverage_percentage(), 100.0);
    }

    #[test]
    fn test_global_registry() {
        let registry = get_api_registry();
        assert!(registry.total_count() > 0);
    }

    #[test]
    fn test_history_json_serialization() {
        let registry = ApiRegistry::new();
        let mut history = CoverageHistory::new();

        let stats = registry.coverage_stats();
        history.add_snapshot(CoverageSnapshot::new("0.1.0", stats.clone()));
        history.add_snapshot(CoverageSnapshot::new("0.2.0", stats));

        let json = history.to_json();
        assert!(json.contains("\"snapshots\""));
        assert!(json.contains("\"version\": \"0.1.0\""));
        assert!(json.contains("\"version\": \"0.2.0\""));
    }

    #[test]
    fn test_history_json_deserialization() {
        let registry = ApiRegistry::new();
        let mut original = CoverageHistory::new();

        let stats = registry.coverage_stats();
        original.add_snapshot(CoverageSnapshot::new("0.1.0", stats.clone()));
        original.add_snapshot(CoverageSnapshot::new("0.2.0", stats));

        let json = original.to_json();
        let loaded = CoverageHistory::from_json(&json).unwrap();

        assert_eq!(loaded.snapshots.len(), 2);
        assert_eq!(loaded.snapshots[0].version, "0.1.0");
        assert_eq!(loaded.snapshots[1].version, "0.2.0");
    }

    #[test]
    fn test_history_file_roundtrip() {
        use std::path::PathBuf;

        let registry = ApiRegistry::new();
        let mut history = CoverageHistory::new();

        let stats = registry.coverage_stats();
        history.add_snapshot(CoverageSnapshot::new("0.1.0", stats.clone()));
        history.add_snapshot(CoverageSnapshot::new("0.2.0", stats));

        // Use temp file in current directory
        let temp_path = PathBuf::from("coverage_history_test.json");

        // Save
        history.save_to_file(&temp_path).unwrap();

        // Load
        let loaded = CoverageHistory::load_from_file(&temp_path).unwrap();

        assert_eq!(loaded.snapshots.len(), 2);
        assert_eq!(loaded.snapshots[0].version, "0.1.0");
        assert_eq!(loaded.snapshots[1].version, "0.2.0");

        // Cleanup
        let _ = std::fs::remove_file(&temp_path);
    }
}
