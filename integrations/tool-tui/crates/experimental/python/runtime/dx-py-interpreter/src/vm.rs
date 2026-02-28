//! Virtual Machine for DX-Py runtime

#![allow(clippy::result_large_err)]

use crate::Dispatcher;
use crate::{InterpreterError, InterpreterResult};
use dashmap::DashMap;
use dx_py_core::builtins::get_builtins;
use dx_py_core::pyframe::PyFrame;
use dx_py_core::pyfunction::{CodeRef, ParameterKind, PyBuiltinFunction, PyFunction};
use dx_py_core::pylist::{PyModule, PyValue};
use dx_py_core::PyDict;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Callable value - either a Python function or a builtin
#[derive(Clone)]
pub enum Callable {
    Function(Arc<PyFunction>),
    Builtin(Arc<PyBuiltinFunction>),
}

/// Virtual Machine state
pub struct VirtualMachine {
    /// Global namespace
    pub globals: Arc<PyDict>,
    /// Built-in functions
    pub builtins: DashMap<String, Arc<PyBuiltinFunction>>,
    /// User-defined functions
    pub functions: DashMap<String, Arc<PyFunction>>,
    /// Loaded modules (sys.modules equivalent)
    pub modules: Arc<DashMap<String, Arc<PyModule>>>,
    /// Module search paths (sys.path equivalent)
    pub sys_path: Arc<Vec<PathBuf>>,
    /// Current frame (if any)
    pub current_frame: Option<Arc<PyFrame>>,
    /// Bytecode storage (function name -> bytecode)
    pub bytecode_store: DashMap<String, Vec<u8>>,
    /// Constants storage (function name -> constants)
    pub constants_store: DashMap<String, Vec<PyValue>>,
    /// Names storage (function name -> names)
    pub names_store: DashMap<String, Vec<String>>,
}

impl VirtualMachine {
    /// Create a new VM
    pub fn new() -> Self {
        let vm = Self {
            globals: Arc::new(PyDict::new()),
            builtins: DashMap::new(),
            functions: DashMap::new(),
            modules: Arc::new(DashMap::new()),
            sys_path: Arc::new(Vec::new()),
            current_frame: None,
            bytecode_store: DashMap::new(),
            constants_store: DashMap::new(),
            names_store: DashMap::new(),
        };

        // Initialize builtins
        for builtin in get_builtins() {
            vm.builtins.insert(builtin.name.clone(), Arc::new(builtin));
        }

        vm
    }

    /// Create a new VM with custom sys.path
    pub fn with_sys_path(sys_path: Vec<PathBuf>) -> Self {
        let mut vm = Self::new();
        vm.sys_path = Arc::new(sys_path);
        vm
    }

    /// Add a path to sys.path
    pub fn add_path(&mut self, path: PathBuf) {
        let mut paths = (*self.sys_path).clone();
        paths.push(path);
        self.sys_path = Arc::new(paths);
    }

    /// Get sys.path
    pub fn get_sys_path(&self) -> &[PathBuf] {
        &self.sys_path
    }

    /// Get a global variable
    pub fn get_global(&self, name: &str) -> Option<PyValue> {
        use dx_py_core::pydict::PyKey;
        self.globals.getitem(&PyKey::Str(Arc::from(name))).ok()
    }

    /// Set a global variable
    pub fn set_global(&self, name: &str, value: PyValue) {
        use dx_py_core::pydict::PyKey;
        self.globals.setitem(PyKey::Str(Arc::from(name)), value);
    }

    /// Get a builtin function
    pub fn get_builtin(&self, name: &str) -> Option<Arc<PyBuiltinFunction>> {
        self.builtins.get(name).map(|r| r.clone())
    }

    /// Register a function with its bytecode
    pub fn register_function(
        &self,
        func: Arc<PyFunction>,
        bytecode: Vec<u8>,
        constants: Vec<PyValue>,
        names: Vec<String>,
    ) {
        let name = func.qualname.clone();
        self.functions.insert(name.clone(), func);
        self.bytecode_store.insert(name.clone(), bytecode);
        self.constants_store.insert(name.clone(), constants);
        self.names_store.insert(name, names);
    }

    /// Get a callable by name (checks functions first, then builtins)
    pub fn get_callable(&self, name: &str) -> Option<Callable> {
        if let Some(func) = self.functions.get(name) {
            return Some(Callable::Function(func.clone()));
        }
        if let Some(builtin) = self.builtins.get(name) {
            return Some(Callable::Builtin(builtin.clone()));
        }
        None
    }

    /// Call a builtin function
    pub fn call_builtin(&self, name: &str, args: &[PyValue]) -> InterpreterResult<PyValue> {
        let func = self.get_builtin(name).ok_or_else(|| {
            InterpreterError::NameError(format!("name '{}' is not defined", name))
        })?;

        func.call(args).map_err(InterpreterError::RuntimeError)
    }

    /// Bind arguments to function parameters
    fn bind_arguments(
        &self,
        func: &Arc<PyFunction>,
        args: &[PyValue],
        kwargs: Option<&HashMap<String, PyValue>>,
    ) -> InterpreterResult<Vec<PyValue>> {
        let mut locals = vec![PyValue::None; func.code.num_locals as usize];
        let mut args_iter = args.iter();
        let mut varargs: Vec<PyValue> = Vec::new();
        let mut kwdict: HashMap<String, PyValue> = HashMap::new();

        // Process parameters
        for (i, param) in func.params.iter().enumerate() {
            match param.kind {
                ParameterKind::Positional | ParameterKind::PositionalOrKeyword => {
                    // Try to get from positional args first
                    if let Some(value) = args_iter.next() {
                        locals[i] = value.clone();
                    } else if let Some(kw) = kwargs {
                        // Try keyword argument
                        if let Some(value) = kw.get(&param.name) {
                            locals[i] = value.clone();
                        } else if let Some(default) = func.get_default(i) {
                            locals[i] = default.clone();
                        } else {
                            return Err(InterpreterError::TypeError(format!(
                                "{}() missing required argument: '{}'",
                                func.name, param.name
                            )));
                        }
                    } else if let Some(default) = func.get_default(i) {
                        locals[i] = default.clone();
                    } else {
                        return Err(InterpreterError::TypeError(format!(
                            "{}() missing required argument: '{}'",
                            func.name, param.name
                        )));
                    }
                }
                ParameterKind::VarPositional => {
                    // Collect remaining positional args into *args
                    varargs.extend(args_iter.by_ref().cloned());
                    locals[i] =
                        PyValue::Tuple(Arc::new(dx_py_core::PyTuple::from_values(varargs.clone())));
                }
                ParameterKind::KeywordOnly => {
                    // Must come from kwargs
                    if let Some(kw) = kwargs {
                        if let Some(value) = kw.get(&param.name) {
                            locals[i] = value.clone();
                        } else if let Some(default) = &param.default {
                            locals[i] = default.clone();
                        } else {
                            return Err(InterpreterError::TypeError(format!(
                                "{}() missing required keyword argument: '{}'",
                                func.name, param.name
                            )));
                        }
                    } else if let Some(default) = &param.default {
                        locals[i] = default.clone();
                    } else {
                        return Err(InterpreterError::TypeError(format!(
                            "{}() missing required keyword argument: '{}'",
                            func.name, param.name
                        )));
                    }
                }
                ParameterKind::VarKeyword => {
                    // Collect remaining kwargs into **kwargs
                    if let Some(kw) = kwargs {
                        for (k, v) in kw {
                            // Only include kwargs not already bound
                            let already_bound = func.params.iter().take(i).any(|p| &p.name == k);
                            if !already_bound {
                                kwdict.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    let dict = Arc::new(PyDict::new());
                    for (k, v) in kwdict.iter() {
                        use dx_py_core::pydict::PyKey;
                        dict.setitem(PyKey::Str(Arc::from(k.as_str())), v.clone());
                    }
                    locals[i] = PyValue::Dict(dict);
                }
            }
        }

        // Check for extra positional arguments
        if !func.flags.has_varargs {
            let remaining: Vec<_> = args_iter.collect();
            if !remaining.is_empty() {
                return Err(InterpreterError::TypeError(format!(
                    "{}() takes {} positional arguments but {} were given",
                    func.name,
                    func.max_positional_args().unwrap_or(0),
                    args.len()
                )));
            }
        }

        Ok(locals)
    }

    /// Call a function with arguments
    pub fn call_function(
        &self,
        func: &Arc<PyFunction>,
        args: &[PyValue],
    ) -> InterpreterResult<PyValue> {
        self.call_function_with_kwargs(func, args, None)
    }

    /// Call a function with positional and keyword arguments
    pub fn call_function_with_kwargs(
        &self,
        func: &Arc<PyFunction>,
        args: &[PyValue],
        kwargs: Option<&HashMap<String, PyValue>>,
    ) -> InterpreterResult<PyValue> {
        // Bind arguments to locals
        let locals = self.bind_arguments(func, args, kwargs)?;

        // Get bytecode for this function
        let bytecode = self.bytecode_store.get(&func.qualname).ok_or_else(|| {
            InterpreterError::Runtime(format!("No bytecode found for function '{}'", func.qualname))
        })?;
        let constants =
            self.constants_store.get(&func.qualname).map(|c| c.clone()).unwrap_or_default();
        let names = self.names_store.get(&func.qualname).map(|n| n.clone()).unwrap_or_default();

        // Create a new frame
        let mut frame = PyFrame::new(Arc::clone(func), None);

        // Set locals from bound arguments
        for (i, value) in locals.into_iter().enumerate() {
            frame.set_local(i, value);
        }

        // Create dispatcher and execute with globals and builtins
        let builtins_map: std::collections::HashMap<String, Arc<PyBuiltinFunction>> =
            self.builtins.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let dispatcher = Dispatcher::with_globals(
            bytecode.clone(),
            constants,
            names,
            Arc::clone(&self.globals),
            builtins_map,
        );
        dispatcher.execute(&mut frame)
    }

    /// Execute a frame directly (for internal use)
    pub fn execute_frame(&self, frame: &mut PyFrame) -> InterpreterResult<PyValue> {
        let func_name = &frame.function.qualname;

        let bytecode = self.bytecode_store.get(func_name).ok_or_else(|| {
            InterpreterError::Runtime(format!("No bytecode found for function '{}'", func_name))
        })?;
        let constants = self.constants_store.get(func_name).map(|c| c.clone()).unwrap_or_default();
        let names = self.names_store.get(func_name).map(|n| n.clone()).unwrap_or_default();

        let builtins_map: std::collections::HashMap<String, Arc<PyBuiltinFunction>> =
            self.builtins.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let dispatcher = Dispatcher::with_globals(
            bytecode.clone(),
            constants,
            names,
            Arc::clone(&self.globals),
            builtins_map,
        );
        dispatcher.execute(frame)
    }

    /// Execute compiled bytecode directly
    pub fn execute_bytecode(
        &self,
        bytecode: Vec<u8>,
        constants: Vec<PyValue>,
        names: Vec<String>,
    ) -> InterpreterResult<PyValue> {
        self.execute_bytecode_with_locals(bytecode, constants, names, 256)
    }

    /// Execute compiled bytecode with specified number of locals
    pub fn execute_bytecode_with_locals(
        &self,
        bytecode: Vec<u8>,
        constants: Vec<PyValue>,
        names: Vec<String>,
        num_locals: usize,
    ) -> InterpreterResult<PyValue> {
        // Create a dummy function for the frame
        let func = Arc::new(PyFunction::new(
            "<module>",
            CodeRef {
                bytecode_offset: 0,
                num_locals: num_locals as u16,
                stack_size: 256,
                num_args: 0,
                num_kwonly_args: 0,
            },
            vec![],
        ));

        let mut frame = PyFrame::new(func, None);

        let builtins_map: std::collections::HashMap<String, Arc<PyBuiltinFunction>> =
            self.builtins.iter().map(|r| (r.key().clone(), r.value().clone())).collect();
        let dispatcher = Dispatcher::with_modules(
            bytecode,
            constants,
            names,
            Arc::clone(&self.globals),
            builtins_map,
            Arc::clone(&self.modules),
            Arc::clone(&self.sys_path),
        );
        dispatcher.execute(&mut frame)
    }

    /// Import a module by name
    ///
    /// This method provides module import functionality with caching.
    /// Modules are cached in self.modules (equivalent to sys.modules).
    pub fn import_module(&self, name: &str) -> InterpreterResult<Arc<PyModule>> {
        // Check if already loaded
        if let Some(module) = self.modules.get(name) {
            return Ok(Arc::clone(&module));
        }

        // Handle os.path as a special submodule
        if name == "os.path" {
            let module = self.create_builtin_module("os.path")?;
            let module_arc = Arc::new(module);
            self.modules.insert(name.to_string(), Arc::clone(&module_arc));
            return Ok(module_arc);
        }

        // List of built-in modules that we support
        let builtin_modules = [
            "sys",
            "builtins",
            "os",
            "io",
            "json",
            "re",
            "math",
            "collections",
            "itertools",
            "functools",
            "typing",
            "pathlib",
            "datetime",
            "time",
            "random",
            "string",
        ];

        // Check if it's a built-in module
        if builtin_modules.contains(&name) {
            let module = self.create_builtin_module(name)?;
            let module_arc = Arc::new(module);
            self.modules.insert(name.to_string(), Arc::clone(&module_arc));
            return Ok(module_arc);
        }

        // Try to find the module in sys.path
        for path in self.sys_path.iter() {
            // Check for package (directory with __init__.py)
            let package_dir = path.join(name);
            let init_py = package_dir.join("__init__.py");
            if init_py.exists() {
                let module = self.load_source_module(name, &init_py, true)?;
                let module_arc = Arc::new(module);
                self.modules.insert(name.to_string(), Arc::clone(&module_arc));
                return Ok(module_arc);
            }

            // Check for source file
            let py_file = path.join(format!("{}.py", name));
            if py_file.exists() {
                let module = self.load_source_module(name, &py_file, false)?;
                let module_arc = Arc::new(module);
                self.modules.insert(name.to_string(), Arc::clone(&module_arc));
                return Ok(module_arc);
            }
        }

        Err(InterpreterError::ImportError(format!("No module named '{}'", name)))
    }

    /// Get a cached module by name
    pub fn get_module(&self, name: &str) -> Option<Arc<PyModule>> {
        self.modules.get(name).map(|m| Arc::clone(&m))
    }

    /// Add a module to the cache
    pub fn add_module(&self, name: impl Into<String>, module: Arc<PyModule>) {
        self.modules.insert(name.into(), module);
    }

    /// Remove a module from the cache
    pub fn remove_module(&self, name: &str) -> Option<Arc<PyModule>> {
        self.modules.remove(name).map(|(_, m)| m)
    }

    /// Create a built-in module with standard attributes
    fn create_builtin_module(&self, name: &str) -> InterpreterResult<PyModule> {
        let mut module = PyModule::new(name);

        // Set standard module attributes
        module.dict.insert("__name__".to_string(), PyValue::Str(Arc::from(name)));
        module.dict.insert("__doc__".to_string(), PyValue::None);
        module.dict.insert("__package__".to_string(), PyValue::Str(Arc::from("")));
        module
            .dict
            .insert("__loader__".to_string(), PyValue::Str(Arc::from("<built-in>")));
        module.dict.insert("__spec__".to_string(), PyValue::None);

        // Add module-specific attributes based on the module name
        match name {
            "sys" => {
                module
                    .dict
                    .insert("version".to_string(), PyValue::Str(Arc::from("3.12.0 (dx-py)")));
                module
                    .dict
                    .insert("platform".to_string(), PyValue::Str(Arc::from(std::env::consts::OS)));
                module.dict.insert(
                    "executable".to_string(),
                    PyValue::Str(Arc::from(
                        std::env::current_exe()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default(),
                    )),
                );
                let path_list: Vec<PyValue> = self
                    .sys_path
                    .iter()
                    .map(|p| PyValue::Str(Arc::from(p.to_string_lossy().as_ref())))
                    .collect();
                module.dict.insert(
                    "path".to_string(),
                    PyValue::List(Arc::new(dx_py_core::PyList::from_values(path_list))),
                );
            }
            "os" => {
                module.dict.insert(
                    "name".to_string(),
                    PyValue::Str(Arc::from(if cfg!(windows) { "nt" } else { "posix" })),
                );
                module.dict.insert(
                    "sep".to_string(),
                    PyValue::Str(Arc::from(std::path::MAIN_SEPARATOR.to_string())),
                );
                module.dict.insert(
                    "linesep".to_string(),
                    PyValue::Str(Arc::from(if cfg!(windows) { "\r\n" } else { "\n" })),
                );
                
                // Add os.path as a submodule
                let path_module = self.create_builtin_module("os.path")?;
                module.dict.insert("path".to_string(), PyValue::Module(Arc::new(path_module)));
            }
            "os.path" => {
                // Add os.path module functions from stdlib
                for builtin in dx_py_core::stdlib::os_path_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "math" => {
                module.dict.insert("pi".to_string(), PyValue::Float(std::f64::consts::PI));
                module.dict.insert("e".to_string(), PyValue::Float(std::f64::consts::E));
                module.dict.insert("tau".to_string(), PyValue::Float(std::f64::consts::TAU));
                module.dict.insert("inf".to_string(), PyValue::Float(f64::INFINITY));
                module.dict.insert("nan".to_string(), PyValue::Float(f64::NAN));
            }
            "json" => {
                // Add json module functions from stdlib
                for builtin in dx_py_core::stdlib::json_builtins_expanded() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "collections" => {
                // Add collections module functions from stdlib
                for builtin in dx_py_core::stdlib::collections_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "itertools" => {
                // Add itertools module functions from stdlib
                for builtin in dx_py_core::stdlib::itertools_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "functools" => {
                // Add functools module functions from stdlib
                for builtin in dx_py_core::stdlib::functools_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "re" => {
                // Add re module functions from stdlib
                for builtin in dx_py_core::stdlib::re_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "datetime" => {
                // Add datetime module functions from stdlib
                for builtin in dx_py_core::stdlib::datetime_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            "pathlib" => {
                // Add pathlib module functions from stdlib
                for builtin in dx_py_core::stdlib::pathlib_builtins() {
                    module.dict.insert(builtin.name.clone(), PyValue::Builtin(Arc::new(builtin)));
                }
            }
            _ => {}
        }

        module.mark_initialized();
        Ok(module)
    }

    /// Load a Python source module from a file
    fn load_source_module(
        &self,
        name: &str,
        path: &std::path::Path,
        is_package: bool,
    ) -> InterpreterResult<PyModule> {
        let source = std::fs::read_to_string(path).map_err(|e| {
            InterpreterError::ImportError(format!("Cannot read '{}': {}", path.display(), e))
        })?;

        let mut module = PyModule::new(name).with_file(path.to_path_buf());

        if is_package {
            module = module.with_package(name);
            if let Some(parent) = path.parent() {
                module.dict.insert(
                    "__path__".to_string(),
                    PyValue::List(Arc::new(dx_py_core::PyList::from_values(vec![PyValue::Str(
                        Arc::from(parent.to_string_lossy().as_ref()),
                    )]))),
                );
            }
        }

        // Extract simple definitions from source
        Self::extract_definitions(&source, &module);

        module.mark_initialized();
        Ok(module)
    }

    /// Extract simple definitions from source
    fn extract_definitions(source: &str, module: &PyModule) {
        for line in source.lines() {
            let trimmed = line.trim();

            if line.starts_with("def ") {
                if let Some(name) = trimmed.strip_prefix("def ") {
                    let name = name.split('(').next().unwrap_or("").trim();
                    if !name.is_empty() {
                        module.set_attr(
                            name,
                            PyValue::Str(Arc::from(format!("<function {}>", name))),
                        );
                    }
                }
            } else if line.starts_with("class ") {
                if let Some(name) = trimmed.strip_prefix("class ") {
                    let name = name.split(['(', ':']).next().unwrap_or("").trim();
                    if !name.is_empty() {
                        module.set_attr(name, PyValue::Str(Arc::from(format!("<class {}>", name))));
                    }
                }
            } else if !line.starts_with(' ') && !line.starts_with('\t') && !line.starts_with('#') {
                if let Some(eq_pos) = line.find('=') {
                    let before_eq = &line[..eq_pos];
                    if !before_eq.ends_with(['!', '<', '>', '+', '-', '*', '/', '%', '&', '|', '^'])
                    {
                        let name = before_eq.trim();
                        if !name.is_empty()
                            && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                            && !name.starts_with(char::is_numeric)
                        {
                            let value = line[eq_pos + 1..].trim();
                            if let Some(v) = Self::parse_simple_value(value) {
                                module.set_attr(name, v);
                            } else {
                                module.set_attr(name, PyValue::Str(Arc::from(value)));
                            }
                        }
                    }
                }
            }
        }
    }

    /// Parse simple Python values
    fn parse_simple_value(value: &str) -> Option<PyValue> {
        let value = value.trim();

        if value == "None" {
            return Some(PyValue::None);
        }
        if value == "True" {
            return Some(PyValue::Bool(true));
        }
        if value == "False" {
            return Some(PyValue::Bool(false));
        }
        if let Ok(i) = value.parse::<i64>() {
            return Some(PyValue::Int(i));
        }
        if let Ok(f) = value.parse::<f64>() {
            return Some(PyValue::Float(f));
        }
        if (value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"'))
        {
            return Some(PyValue::Str(Arc::from(&value[1..value.len() - 1])));
        }
        None
    }

    /// Execute a simple expression (for REPL)
    pub fn eval_expr(&self, expr: &str) -> InterpreterResult<PyValue> {
        // Very simple expression evaluator for testing
        let expr = expr.trim();

        // Try to parse as integer
        if let Ok(i) = expr.parse::<i64>() {
            return Ok(PyValue::Int(i));
        }

        // Try to parse as float
        if let Ok(f) = expr.parse::<f64>() {
            return Ok(PyValue::Float(f));
        }

        // Check for string literal
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            let s = &expr[1..expr.len() - 1];
            return Ok(PyValue::Str(Arc::from(s)));
        }

        // Check for None, True, False
        match expr {
            "None" => return Ok(PyValue::None),
            "True" => return Ok(PyValue::Bool(true)),
            "False" => return Ok(PyValue::Bool(false)),
            _ => {}
        }

        // Check for binary arithmetic operations (simple cases without parentheses)
        // Handle operators in order of precedence (lowest first for left-to-right parsing)
        // IMPORTANT: Check multi-char operators before single-char ones (** before *, // before /)
        for (op_str, op_fn) in [
            ("+", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x + y)),
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x + y)),
                    (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(x as f64 + y)),
                    (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x + y as f64)),
                    (PyValue::Str(x), PyValue::Str(y)) => {
                        Ok(PyValue::Str(Arc::from(format!("{}{}", x, y))))
                    }
                    _ => {
                        Err(InterpreterError::TypeError("unsupported operand type(s) for +".into()))
                    }
                }
            } as fn(PyValue, PyValue) -> InterpreterResult<PyValue>),
            ("-", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x - y)),
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x - y)),
                    (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(x as f64 - y)),
                    (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x - y as f64)),
                    _ => {
                        Err(InterpreterError::TypeError("unsupported operand type(s) for -".into()))
                    }
                }
            }),
            ("**", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => {
                        if y >= 0 {
                            Ok(PyValue::Int(x.pow(y as u32)))
                        } else {
                            Ok(PyValue::Float((x as f64).powi(y as i32)))
                        }
                    }
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x.powf(y))),
                    (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float((x as f64).powf(y))),
                    (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x.powi(y as i32))),
                    _ => Err(InterpreterError::TypeError(
                        "unsupported operand type(s) for **".into(),
                    )),
                }
            }),
            ("*", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => Ok(PyValue::Int(x * y)),
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x * y)),
                    (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(x as f64 * y)),
                    (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x * y as f64)),
                    (PyValue::Str(s), PyValue::Int(n)) | (PyValue::Int(n), PyValue::Str(s)) => {
                        if n <= 0 {
                            Ok(PyValue::Str(Arc::from("")))
                        } else {
                            Ok(PyValue::Str(Arc::from(s.repeat(n as usize))))
                        }
                    }
                    _ => {
                        Err(InterpreterError::TypeError("unsupported operand type(s) for *".into()))
                    }
                }
            }),
            ("//", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => {
                        if y == 0 {
                            Err(InterpreterError::ValueError("integer division by zero".into()))
                        } else {
                            Ok(PyValue::Int(x.div_euclid(y)))
                        }
                    }
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float((x / y).floor())),
                    (PyValue::Int(x), PyValue::Float(y)) => {
                        Ok(PyValue::Float((x as f64 / y).floor()))
                    }
                    (PyValue::Float(x), PyValue::Int(y)) => {
                        Ok(PyValue::Float((x / y as f64).floor()))
                    }
                    _ => Err(InterpreterError::TypeError(
                        "unsupported operand type(s) for //".into(),
                    )),
                }
            }),
            ("/", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => {
                        if y == 0 {
                            Err(InterpreterError::ValueError("division by zero".into()))
                        } else {
                            Ok(PyValue::Float(x as f64 / y as f64))
                        }
                    }
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x / y)),
                    (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(x as f64 / y)),
                    (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x / y as f64)),
                    _ => {
                        Err(InterpreterError::TypeError("unsupported operand type(s) for /".into()))
                    }
                }
            }),
            ("%", |a: PyValue, b: PyValue| -> InterpreterResult<PyValue> {
                match (a, b) {
                    (PyValue::Int(x), PyValue::Int(y)) => {
                        if y == 0 {
                            Err(InterpreterError::ValueError("integer modulo by zero".into()))
                        } else {
                            Ok(PyValue::Int(x.rem_euclid(y)))
                        }
                    }
                    (PyValue::Float(x), PyValue::Float(y)) => Ok(PyValue::Float(x % y)),
                    (PyValue::Int(x), PyValue::Float(y)) => Ok(PyValue::Float(x as f64 % y)),
                    (PyValue::Float(x), PyValue::Int(y)) => Ok(PyValue::Float(x % y as f64)),
                    _ => {
                        Err(InterpreterError::TypeError("unsupported operand type(s) for %".into()))
                    }
                }
            }),
        ] {
            // Find the operator, but not inside parentheses or strings
            if let Some(pos) = self.find_operator(expr, op_str) {
                let left = expr[..pos].trim();
                let right = expr[pos + op_str.len()..].trim();
                if !left.is_empty() && !right.is_empty() {
                    let left_val = self.eval_expr(left)?;
                    let right_val = self.eval_expr(right)?;
                    return op_fn(left_val, right_val);
                }
            }
        }

        // Check for parenthesized expression
        if expr.starts_with('(') && expr.ends_with(')') {
            return self.eval_expr(&expr[1..expr.len() - 1]);
        }

        // Check for builtin function call
        if let Some(paren_pos) = expr.find('(') {
            if expr.ends_with(')') {
                let func_name = &expr[..paren_pos];
                let args_str = &expr[paren_pos + 1..expr.len() - 1];

                // Parse arguments (very simplified)
                let args: Vec<PyValue> = if args_str.is_empty() {
                    vec![]
                } else {
                    self.parse_args(args_str)?
                };

                // Check for user-defined function first
                if let Some(func) = self.functions.get(func_name) {
                    return self.call_function(&func, &args);
                }

                return self.call_builtin(func_name, &args);
            }
        }

        // Check for variable
        if let Some(value) = self.get_global(expr) {
            return Ok(value);
        }

        Err(InterpreterError::NameError(format!("name '{}' is not defined", expr)))
    }

    /// Find an operator in an expression, respecting parentheses and strings
    fn find_operator(&self, expr: &str, op: &str) -> Option<usize> {
        let mut paren_depth = 0;
        let mut in_string = false;
        let mut string_char = ' ';
        let chars: Vec<char> = expr.chars().collect();
        let op_chars: Vec<char> = op.chars().collect();

        // For ** we need to search from right to left to handle precedence correctly
        // For other operators, search from left to right
        if op == "**" {
            let mut i = chars.len();
            while i > 0 {
                i -= 1;
                let c = chars[i];

                if in_string {
                    if c == string_char && (i == 0 || chars[i - 1] != '\\') {
                        in_string = false;
                    }
                    continue;
                }

                if c == '"' || c == '\'' {
                    in_string = true;
                    string_char = c;
                    continue;
                }

                if c == ')' {
                    paren_depth += 1;
                    continue;
                }
                if c == '(' {
                    paren_depth -= 1;
                    continue;
                }

                if paren_depth == 0 && i + op_chars.len() <= chars.len() {
                    let matches = op_chars
                        .iter()
                        .enumerate()
                        .all(|(j, &oc)| i + j < chars.len() && chars[i + j] == oc);
                    if matches {
                        return Some(i);
                    }
                }
            }
        } else {
            // For // we need to check it before / to avoid false matches
            for i in 0..chars.len() {
                let c = chars[i];

                if in_string {
                    if c == string_char && (i == 0 || chars[i - 1] != '\\') {
                        in_string = false;
                    }
                    continue;
                }

                if c == '"' || c == '\'' {
                    in_string = true;
                    string_char = c;
                    continue;
                }

                if c == '(' {
                    paren_depth += 1;
                    continue;
                }
                if c == ')' {
                    paren_depth -= 1;
                    continue;
                }

                if paren_depth == 0 && i + op_chars.len() <= chars.len() {
                    let matches = op_chars
                        .iter()
                        .enumerate()
                        .all(|(j, &oc)| i + j < chars.len() && chars[i + j] == oc);
                    if matches {
                        // For / make sure it's not //
                        if op == "/" && i + 1 < chars.len() && chars[i + 1] == '/' {
                            continue;
                        }
                        return Some(i);
                    }
                }
            }
        }

        None
    }

    /// Parse function arguments, handling nested parentheses
    fn parse_args(&self, args_str: &str) -> InterpreterResult<Vec<PyValue>> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut paren_depth = 0;
        let mut in_string = false;
        let mut string_char = ' ';

        for c in args_str.chars() {
            if in_string {
                current.push(c);
                if c == string_char {
                    in_string = false;
                }
                continue;
            }

            if c == '"' || c == '\'' {
                in_string = true;
                string_char = c;
                current.push(c);
                continue;
            }

            if c == '(' {
                paren_depth += 1;
                current.push(c);
                continue;
            }

            if c == ')' {
                paren_depth -= 1;
                current.push(c);
                continue;
            }

            if c == ',' && paren_depth == 0 {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    args.push(self.eval_expr(trimmed)?);
                }
                current.clear();
                continue;
            }

            current.push(c);
        }

        let trimmed = current.trim();
        if !trimmed.is_empty() {
            args.push(self.eval_expr(trimmed)?);
        }

        Ok(args)
    }
}

impl Default for VirtualMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcodes::Opcode;
    use dx_py_core::pyfunction::{CodeRef, Parameter, ParameterKind};

    #[test]
    fn test_vm_creation() {
        let vm = VirtualMachine::new();
        assert!(vm.builtins.contains_key("print"));
        assert!(vm.builtins.contains_key("len"));
    }

    #[test]
    fn test_vm_globals() {
        let vm = VirtualMachine::new();
        vm.set_global("x", PyValue::Int(42));

        let value = vm.get_global("x").unwrap();
        if let PyValue::Int(v) = value {
            assert_eq!(v, 42);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_vm_builtin_call() {
        let vm = VirtualMachine::new();

        let result = vm.call_builtin("len", &[PyValue::Str(Arc::from("hello"))]).unwrap();
        if let PyValue::Int(len) = result {
            assert_eq!(len, 5);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_vm_eval_expr() {
        let vm = VirtualMachine::new();

        // Integer
        let result = vm.eval_expr("42").unwrap();
        assert!(matches!(result, PyValue::Int(42)));

        // Float
        let result = vm.eval_expr("3.125").unwrap();
        if let PyValue::Float(f) = result {
            assert!((f - 3.125).abs() < 0.001);
        }

        // String
        let result = vm.eval_expr("'hello'").unwrap();
        if let PyValue::Str(s) = result {
            assert_eq!(&*s, "hello");
        }

        // Builtin call
        let result = vm.eval_expr("len('test')").unwrap();
        assert!(matches!(result, PyValue::Int(4)));
    }

    #[test]
    fn test_vm_function_call() {
        let vm = VirtualMachine::new();

        // Create a simple function that returns its first argument + 1
        // Bytecode: LOAD_FAST 0, LOAD_CONST 0, BINARY_ADD, RETURN
        let func = Arc::new(PyFunction::new(
            "add_one",
            CodeRef {
                bytecode_offset: 0,
                num_locals: 1,
                stack_size: 4,
                num_args: 1,
                num_kwonly_args: 0,
            },
            vec![Parameter {
                name: "x".into(),
                kind: ParameterKind::PositionalOrKeyword,
                default: None,
                annotation: None,
            }],
        ));

        let bytecode = vec![
            Opcode::LoadFast as u8,
            0,
            0, // Load x (local 0)
            Opcode::LoadConst as u8,
            0,
            0,                       // Load 1
            Opcode::BinaryAdd as u8, // x + 1
            Opcode::Return as u8,    // Return result
        ];
        let constants = vec![PyValue::Int(1)];
        let names = vec![];

        vm.register_function(Arc::clone(&func), bytecode, constants, names);

        // Call the function
        let result = vm.call_function(&func, &[PyValue::Int(5)]).unwrap();
        if let PyValue::Int(v) = result {
            assert_eq!(v, 6);
        } else {
            panic!("Expected Int, got {:?}", result);
        }
    }

    #[test]
    fn test_vm_function_with_default() {
        let vm = VirtualMachine::new();

        // Create a function with a default argument: def add(x, y=10)
        let func = Arc::new(
            PyFunction::new(
                "add",
                CodeRef {
                    bytecode_offset: 0,
                    num_locals: 2,
                    stack_size: 4,
                    num_args: 2,
                    num_kwonly_args: 0,
                },
                vec![
                    Parameter {
                        name: "x".into(),
                        kind: ParameterKind::PositionalOrKeyword,
                        default: None,
                        annotation: None,
                    },
                    Parameter {
                        name: "y".into(),
                        kind: ParameterKind::PositionalOrKeyword,
                        default: Some(PyValue::Int(10)),
                        annotation: None,
                    },
                ],
            )
            .with_defaults(vec![PyValue::Int(10)]),
        );

        let bytecode = vec![
            Opcode::LoadFast as u8,
            0,
            0, // Load x
            Opcode::LoadFast as u8,
            1,
            0,                       // Load y
            Opcode::BinaryAdd as u8, // x + y
            Opcode::Return as u8,    // Return
        ];

        vm.register_function(Arc::clone(&func), bytecode, vec![], vec![]);

        // Call with both args
        let result = vm.call_function(&func, &[PyValue::Int(5), PyValue::Int(3)]).unwrap();
        if let PyValue::Int(v) = result {
            assert_eq!(v, 8);
        } else {
            panic!("Expected Int");
        }

        // Call with default
        let result = vm.call_function(&func, &[PyValue::Int(5)]).unwrap();
        if let PyValue::Int(v) = result {
            assert_eq!(v, 15); // 5 + 10
        } else {
            panic!("Expected Int");
        }
    }
}
