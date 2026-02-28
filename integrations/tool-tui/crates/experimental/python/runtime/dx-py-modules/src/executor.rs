//! Module Execution Engine
//!
//! This module provides the execution engine for Python modules, handling:
//! - Bytecode execution in module context
//! - Function object creation from code objects
//! - Class object creation from class definitions
//! - Proper namespace population during import

use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::importer::{ModuleValue, PyModule};

/// Errors that can occur during module execution
#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("Name error: name '{0}' is not defined")]
    NameError(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Attribute error: '{0}' object has no attribute '{1}'")]
    AttributeError(String, String),

    #[error("Import error: {0}")]
    ImportError(String),

    #[error("Syntax error: {0}")]
    SyntaxError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Value error: {0}")]
    ValueError(String),
}

/// Result type for execution operations
pub type ExecutionResult<T> = Result<T, ExecutionError>;

/// A Python function object
#[derive(Debug, Clone, PartialEq)]
pub struct PyFunction {
    /// Function name
    pub name: String,
    /// Qualified name (including module/class path)
    pub qualname: String,
    /// Module name where function is defined
    pub module: String,
    /// Function documentation
    pub doc: Option<String>,
    /// Default argument values
    pub defaults: Vec<PyValue>,
    /// Keyword-only argument defaults
    pub kwdefaults: HashMap<String, PyValue>,
    /// Annotations
    pub annotations: HashMap<String, String>,
    /// Code object (bytecode)
    pub code: Arc<CodeObject>,
    /// Global namespace reference
    pub globals: Arc<HashMap<String, PyValue>>,
    /// Closure variables
    pub closure: Vec<PyValue>,
    /// Whether this is a generator function
    pub is_generator: bool,
    /// Whether this is an async function
    pub is_async: bool,
}

impl PyFunction {
    /// Create a new function object
    pub fn new(
        name: impl Into<String>,
        qualname: impl Into<String>,
        module: impl Into<String>,
        code: Arc<CodeObject>,
        globals: Arc<HashMap<String, PyValue>>,
    ) -> Self {
        Self {
            name: name.into(),
            qualname: qualname.into(),
            module: module.into(),
            doc: None,
            defaults: Vec::new(),
            kwdefaults: HashMap::new(),
            annotations: HashMap::new(),
            code,
            globals,
            closure: Vec::new(),
            is_generator: false,
            is_async: false,
        }
    }

    /// Set function documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = Some(doc.into());
        self
    }

    /// Set default argument values
    pub fn with_defaults(mut self, defaults: Vec<PyValue>) -> Self {
        self.defaults = defaults;
        self
    }

    /// Set closure variables
    pub fn with_closure(mut self, closure: Vec<PyValue>) -> Self {
        self.closure = closure;
        self
    }

    /// Mark as generator function
    pub fn as_generator(mut self) -> Self {
        self.is_generator = true;
        self
    }

    /// Mark as async function
    pub fn as_async(mut self) -> Self {
        self.is_async = true;
        self
    }

    /// Get the number of positional arguments
    pub fn argcount(&self) -> usize {
        self.code.argcount
    }

    /// Get the number of keyword-only arguments
    pub fn kwonlyargcount(&self) -> usize {
        self.code.kwonlyargcount
    }

    /// Check if function accepts *args
    pub fn has_varargs(&self) -> bool {
        self.code.flags & CodeFlags::VARARGS != 0
    }

    /// Check if function accepts **kwargs
    pub fn has_varkeywords(&self) -> bool {
        self.code.flags & CodeFlags::VARKEYWORDS != 0
    }
}

/// A Python class object (type)
#[derive(Debug, Clone, PartialEq)]
pub struct PyClass {
    /// Class name
    pub name: String,
    /// Qualified name
    pub qualname: String,
    /// Module name where class is defined
    pub module: String,
    /// Class documentation
    pub doc: Option<String>,
    /// Base classes
    pub bases: Vec<Arc<PyClass>>,
    /// Method Resolution Order
    pub mro: Vec<Arc<PyClass>>,
    /// Class dictionary (methods, attributes)
    pub dict: HashMap<String, PyValue>,
    /// Metaclass
    pub metaclass: Option<Arc<PyClass>>,
}

impl PyClass {
    /// Create a new class object
    pub fn new(
        name: impl Into<String>,
        qualname: impl Into<String>,
        module: impl Into<String>,
    ) -> Self {
        let name = name.into();
        Self {
            name: name.clone(),
            qualname: qualname.into(),
            module: module.into(),
            doc: None,
            bases: Vec::new(),
            mro: Vec::new(),
            dict: HashMap::new(),
            metaclass: None,
        }
    }

    /// Set class documentation
    pub fn with_doc(mut self, doc: impl Into<String>) -> Self {
        self.doc = Some(doc.into());
        self
    }

    /// Set base classes
    pub fn with_bases(mut self, bases: Vec<Arc<PyClass>>) -> Self {
        self.bases = bases;
        self.compute_mro();
        self
    }

    /// Add a method to the class
    pub fn add_method(&mut self, name: impl Into<String>, method: PyFunction) {
        self.dict.insert(name.into(), PyValue::Function(Arc::new(method)));
    }

    /// Add an attribute to the class
    pub fn add_attribute(&mut self, name: impl Into<String>, value: PyValue) {
        self.dict.insert(name.into(), value);
    }

    /// Get an attribute from the class (including inherited)
    pub fn get_attribute(&self, name: &str) -> Option<&PyValue> {
        // Check own dict first
        if let Some(value) = self.dict.get(name) {
            return Some(value);
        }

        // Check MRO
        for base in &self.mro {
            if let Some(value) = base.dict.get(name) {
                return Some(value);
            }
        }

        None
    }

    /// Compute the Method Resolution Order using C3 linearization
    ///
    /// The C3 linearization algorithm ensures a consistent and predictable
    /// method resolution order for multiple inheritance. The algorithm:
    /// 1. MRO(C) = [C] + merge(MRO(B1), MRO(B2), ..., [B1, B2, ...])
    /// 2. merge takes the first head that doesn't appear in the tail of any list
    /// 3. Removes that head from all lists and adds it to the result
    /// 4. Repeats until all lists are empty
    fn compute_mro(&mut self) {
        if self.bases.is_empty() {
            // No bases, MRO is empty (self is implicit)
            self.mro.clear();
            return;
        }

        // Collect base MROs: for each base, prepend the base itself to its MRO
        let mut to_merge: Vec<Vec<Arc<PyClass>>> = Vec::new();
        for base in &self.bases {
            let mut base_mro = vec![Arc::clone(base)];
            base_mro.extend(base.mro.iter().cloned());
            to_merge.push(base_mro);
        }

        // Add the list of direct bases at the end
        to_merge.push(self.bases.clone());

        // Perform C3 merge
        self.mro = Self::c3_merge(to_merge);
    }

    /// C3 merge algorithm for computing MRO
    ///
    /// The merge operation takes a list of lists and produces a single list
    /// by repeatedly selecting a "good head" - a class that appears at the
    /// head of a list but not in the tail of any list.
    fn c3_merge(mut lists: Vec<Vec<Arc<PyClass>>>) -> Vec<Arc<PyClass>> {
        let mut result = Vec::new();

        loop {
            // Remove empty lists
            lists.retain(|l| !l.is_empty());

            if lists.is_empty() {
                break;
            }

            // Find a good head: one that doesn't appear in the tail of any list
            let mut found = None;
            for list in &lists {
                let head = &list[0];
                // Check if head appears in the tail (index 1..) of any list
                let in_tail = lists.iter().any(|l| {
                    l.len() > 1 && l[1..].iter().any(|t| Arc::ptr_eq(t, head))
                });

                if !in_tail {
                    found = Some(Arc::clone(head));
                    break;
                }
            }

            match found {
                Some(head) => {
                    result.push(Arc::clone(&head));
                    // Remove head from all lists where it appears at position 0
                    for list in &mut lists {
                        if !list.is_empty() && Arc::ptr_eq(&list[0], &head) {
                            list.remove(0);
                        }
                    }
                }
                None => {
                    // Inconsistent MRO - cannot create a consistent ordering
                    // This happens with invalid inheritance hierarchies
                    // In Python, this would raise TypeError
                    break;
                }
            }
        }

        result
    }

    /// Get a method from the class following the MRO
    ///
    /// This method looks up a method by name, first checking the class's own
    /// dictionary, then following the Method Resolution Order to check base classes.
    pub fn get_method(&self, name: &str) -> Option<&PyFunction> {
        // Check own dict first
        if let Some(PyValue::Function(f)) = self.dict.get(name) {
            return Some(f.as_ref());
        }

        // Follow MRO to find the method
        for cls in &self.mro {
            if let Some(PyValue::Function(f)) = cls.dict.get(name) {
                return Some(f.as_ref());
            }
        }

        None
    }

    /// Create an instance of this class
    pub fn instantiate(&self, args: Vec<PyValue>) -> ExecutionResult<PyInstance> {
        let instance = PyInstance::new(Arc::new(self.clone()));

        // Call __init__ if it exists
        if let Some(PyValue::Function(init)) = self.get_attribute("__init__") {
            // In a full implementation, we would call init with the instance and args
            let _ = (init, args); // Suppress unused warnings
        }

        Ok(instance)
    }
}

/// A Python instance object
#[derive(Debug, Clone, PartialEq)]
pub struct PyInstance {
    /// The class this is an instance of
    pub class: Arc<PyClass>,
    /// Instance dictionary
    pub dict: HashMap<String, PyValue>,
}

impl PyInstance {
    /// Create a new instance
    pub fn new(class: Arc<PyClass>) -> Self {
        Self {
            class,
            dict: HashMap::new(),
        }
    }

    /// Get an attribute from the instance
    pub fn get_attribute(&self, name: &str) -> Option<PyValue> {
        // Check instance dict first
        if let Some(value) = self.dict.get(name) {
            return Some(value.clone());
        }

        // Check class
        self.class.get_attribute(name).cloned()
    }

    /// Set an attribute on the instance
    pub fn set_attribute(&mut self, name: impl Into<String>, value: PyValue) {
        self.dict.insert(name.into(), value);
    }
}

/// Python values that can be stored in namespaces
#[derive(Debug, Clone)]
pub enum PyValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    List(Vec<PyValue>),
    Tuple(Vec<PyValue>),
    Dict(HashMap<String, PyValue>),
    Set(Vec<PyValue>),
    Function(Arc<PyFunction>),
    Class(Arc<PyClass>),
    Instance(Arc<PyInstance>),
    Module(Arc<PyModule>),
    Code(Arc<CodeObject>),
    /// Placeholder for values not yet fully implemented
    Placeholder(String),
}

impl PartialEq for PyValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PyValue::None, PyValue::None) => true,
            (PyValue::Bool(a), PyValue::Bool(b)) => a == b,
            (PyValue::Int(a), PyValue::Int(b)) => a == b,
            (PyValue::Float(a), PyValue::Float(b)) => a == b,
            (PyValue::Str(a), PyValue::Str(b)) => a == b,
            (PyValue::Bytes(a), PyValue::Bytes(b)) => a == b,
            (PyValue::List(a), PyValue::List(b)) => a == b,
            (PyValue::Tuple(a), PyValue::Tuple(b)) => a == b,
            (PyValue::Dict(a), PyValue::Dict(b)) => a == b,
            (PyValue::Set(a), PyValue::Set(b)) => a == b,
            (PyValue::Function(a), PyValue::Function(b)) => Arc::ptr_eq(a, b),
            (PyValue::Class(a), PyValue::Class(b)) => Arc::ptr_eq(a, b),
            (PyValue::Instance(a), PyValue::Instance(b)) => Arc::ptr_eq(a, b),
            (PyValue::Module(a), PyValue::Module(b)) => Arc::ptr_eq(a, b),
            (PyValue::Code(a), PyValue::Code(b)) => a == b,
            (PyValue::Placeholder(a), PyValue::Placeholder(b)) => a == b,
            _ => false,
        }
    }
}

impl PyValue {
    /// Get the type name of this value
    pub fn type_name(&self) -> &str {
        match self {
            PyValue::None => "NoneType",
            PyValue::Bool(_) => "bool",
            PyValue::Int(_) => "int",
            PyValue::Float(_) => "float",
            PyValue::Str(_) => "str",
            PyValue::Bytes(_) => "bytes",
            PyValue::List(_) => "list",
            PyValue::Tuple(_) => "tuple",
            PyValue::Dict(_) => "dict",
            PyValue::Set(_) => "set",
            PyValue::Function(_) => "function",
            PyValue::Class(_) => "type",
            PyValue::Instance(i) => &i.class.name,
            PyValue::Module(_) => "module",
            PyValue::Code(_) => "code",
            PyValue::Placeholder(_) => "placeholder",
        }
    }

    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            PyValue::None => false,
            PyValue::Bool(b) => *b,
            PyValue::Int(i) => *i != 0,
            PyValue::Float(f) => *f != 0.0,
            PyValue::Str(s) => !s.is_empty(),
            PyValue::Bytes(b) => !b.is_empty(),
            PyValue::List(l) => !l.is_empty(),
            PyValue::Tuple(t) => !t.is_empty(),
            PyValue::Dict(d) => !d.is_empty(),
            PyValue::Set(s) => !s.is_empty(),
            _ => true,
        }
    }
}

/// Code object flags
pub struct CodeFlags;

impl CodeFlags {
    pub const OPTIMIZED: u32 = 0x0001;
    pub const NEWLOCALS: u32 = 0x0002;
    pub const VARARGS: u32 = 0x0004;
    pub const VARKEYWORDS: u32 = 0x0008;
    pub const NESTED: u32 = 0x0010;
    pub const GENERATOR: u32 = 0x0020;
    pub const COROUTINE: u32 = 0x0080;
    pub const ASYNC_GENERATOR: u32 = 0x0200;
}

/// A code object (compiled bytecode)
#[derive(Debug, Clone, PartialEq)]
pub struct CodeObject {
    /// Function/code name
    pub name: String,
    /// Qualified name
    pub qualname: String,
    /// Source filename
    pub filename: String,
    /// First line number
    pub firstlineno: u32,
    /// Number of positional arguments
    pub argcount: usize,
    /// Number of positional-only arguments
    pub posonlyargcount: usize,
    /// Number of keyword-only arguments
    pub kwonlyargcount: usize,
    /// Number of local variables
    pub nlocals: usize,
    /// Stack size needed
    pub stacksize: usize,
    /// Code flags
    pub flags: u32,
    /// Bytecode
    pub code: Vec<u8>,
    /// Constants used in code
    pub consts: Vec<PyValue>,
    /// Names used in code
    pub names: Vec<String>,
    /// Local variable names
    pub varnames: Vec<String>,
    /// Free variable names (from enclosing scope)
    pub freevars: Vec<String>,
    /// Cell variable names (used by nested functions)
    pub cellvars: Vec<String>,
}

impl CodeObject {
    /// Create a new code object
    pub fn new(name: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            qualname: String::new(),
            filename: filename.into(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 0,
            flags: 0,
            code: Vec::new(),
            consts: Vec::new(),
            names: Vec::new(),
            varnames: Vec::new(),
            freevars: Vec::new(),
            cellvars: Vec::new(),
        }
    }

    /// Check if this is a generator function
    pub fn is_generator(&self) -> bool {
        self.flags & CodeFlags::GENERATOR != 0
    }

    /// Check if this is a coroutine
    pub fn is_coroutine(&self) -> bool {
        self.flags & CodeFlags::COROUTINE != 0
    }
}

/// Module executor - executes bytecode in module context
pub struct ModuleExecutor {
    /// Global namespace for the module being executed
    globals: HashMap<String, PyValue>,
    /// Local namespace (same as globals for module-level code)
    locals: HashMap<String, PyValue>,
    /// Built-in namespace
    builtins: HashMap<String, PyValue>,
}

impl ModuleExecutor {
    /// Create a new module executor
    pub fn new() -> Self {
        let mut builtins = HashMap::new();

        // Add built-in functions and types
        builtins.insert("None".to_string(), PyValue::None);
        builtins.insert("True".to_string(), PyValue::Bool(true));
        builtins.insert("False".to_string(), PyValue::Bool(false));

        // Add placeholder built-in functions
        builtins.insert(
            "print".to_string(),
            PyValue::Placeholder("<built-in function print>".to_string()),
        );
        builtins
            .insert("len".to_string(), PyValue::Placeholder("<built-in function len>".to_string()));
        builtins.insert(
            "range".to_string(),
            PyValue::Placeholder("<built-in function range>".to_string()),
        );
        builtins.insert(
            "type".to_string(),
            PyValue::Placeholder("<built-in function type>".to_string()),
        );
        builtins.insert(
            "isinstance".to_string(),
            PyValue::Placeholder("<built-in function isinstance>".to_string()),
        );
        builtins.insert(
            "getattr".to_string(),
            PyValue::Placeholder("<built-in function getattr>".to_string()),
        );
        builtins.insert(
            "setattr".to_string(),
            PyValue::Placeholder("<built-in function setattr>".to_string()),
        );
        builtins.insert(
            "hasattr".to_string(),
            PyValue::Placeholder("<built-in function hasattr>".to_string()),
        );

        // Add built-in types
        builtins.insert("int".to_string(), PyValue::Placeholder("<class 'int'>".to_string()));
        builtins.insert("float".to_string(), PyValue::Placeholder("<class 'float'>".to_string()));
        builtins.insert("str".to_string(), PyValue::Placeholder("<class 'str'>".to_string()));
        builtins.insert("list".to_string(), PyValue::Placeholder("<class 'list'>".to_string()));
        builtins.insert("dict".to_string(), PyValue::Placeholder("<class 'dict'>".to_string()));
        builtins.insert("tuple".to_string(), PyValue::Placeholder("<class 'tuple'>".to_string()));
        builtins.insert("set".to_string(), PyValue::Placeholder("<class 'set'>".to_string()));
        builtins.insert("object".to_string(), PyValue::Placeholder("<class 'object'>".to_string()));

        Self {
            globals: HashMap::new(),
            locals: HashMap::new(),
            builtins,
        }
    }

    /// Execute module code and populate namespace
    pub fn execute(&mut self, module: &PyModule, code: &CodeObject) -> ExecutionResult<()> {
        // Set up module globals
        self.globals
            .insert("__name__".to_string(), PyValue::Str(module.spec.name.clone()));
        self.globals.insert(
            "__doc__".to_string(),
            module.doc.as_ref().map(|d| PyValue::Str(d.clone())).unwrap_or(PyValue::None),
        );

        if let Some(ref origin) = module.spec.origin {
            self.globals
                .insert("__file__".to_string(), PyValue::Str(origin.to_string_lossy().to_string()));
        }

        // For module-level code, locals == globals
        self.locals = self.globals.clone();

        // Execute the bytecode
        // In a full implementation, this would interpret the bytecode
        // For now, we just process the code object's constants and names
        self.process_code_object(code, &module.spec.name)?;

        // Copy results back to module dict
        for (name, value) in &self.globals {
            let module_value = self.py_value_to_module_value(value);
            module.set_attr(name.clone(), module_value);
        }

        Ok(())
    }

    /// Process a code object to extract definitions
    fn process_code_object(&mut self, code: &CodeObject, module_name: &str) -> ExecutionResult<()> {
        // Process constants - look for nested code objects (functions, classes)
        for (i, const_val) in code.consts.iter().enumerate() {
            if let PyValue::Code(nested_code) = const_val {
                // This is a function or class definition
                let name = &nested_code.name;

                if nested_code.flags & CodeFlags::GENERATOR != 0 {
                    // Generator function
                    let func = self.create_function(nested_code, module_name);
                    self.globals
                        .insert(name.clone(), PyValue::Function(Arc::new(func.as_generator())));
                } else if nested_code.flags & CodeFlags::COROUTINE != 0 {
                    // Async function
                    let func = self.create_function(nested_code, module_name);
                    self.globals.insert(name.clone(), PyValue::Function(Arc::new(func.as_async())));
                } else {
                    // Regular function
                    let func = self.create_function(nested_code, module_name);
                    self.globals.insert(name.clone(), PyValue::Function(Arc::new(func)));
                }

                let _ = i; // Suppress unused warning
            }
        }

        Ok(())
    }

    /// Create a function object from a code object
    pub fn create_function(&self, code: &CodeObject, module_name: &str) -> PyFunction {
        let globals = Arc::new(self.globals.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        let mut func = PyFunction::new(
            &code.name,
            &code.qualname,
            module_name,
            Arc::new(code.clone()),
            globals,
        );

        // Extract docstring from first constant if it's a string
        if let Some(PyValue::Str(doc)) = code.consts.first() {
            func = func.with_doc(doc.clone());
        }

        func
    }

    /// Create a class object from a class definition
    pub fn create_class(
        &self,
        name: &str,
        qualname: &str,
        module_name: &str,
        bases: Vec<Arc<PyClass>>,
        body_code: &CodeObject,
    ) -> ExecutionResult<PyClass> {
        let mut class = PyClass::new(name, qualname, module_name).with_bases(bases);

        // Execute class body to populate class dict
        // In a full implementation, this would execute the body bytecode
        // For now, we extract methods from the code object
        for const_val in &body_code.consts {
            if let PyValue::Code(method_code) = const_val {
                let method = self.create_function(method_code, module_name);
                class.add_method(&method_code.name, method);
            }
        }

        // Extract docstring
        if let Some(PyValue::Str(doc)) = body_code.consts.first() {
            class = class.with_doc(doc.clone());
        }

        Ok(class)
    }

    /// Convert PyValue to ModuleValue for storage in module dict
    fn py_value_to_module_value(&self, value: &PyValue) -> ModuleValue {
        match value {
            PyValue::None => ModuleValue::None,
            PyValue::Bool(b) => ModuleValue::Bool(*b),
            PyValue::Int(i) => ModuleValue::Int(*i),
            PyValue::Float(f) => ModuleValue::Float(*f),
            PyValue::Str(s) => ModuleValue::Str(s.clone()),
            PyValue::Function(f) => ModuleValue::Str(format!("<function {}>", f.name)),
            PyValue::Class(c) => ModuleValue::Str(format!("<class '{}'>", c.name)),
            PyValue::Module(m) => ModuleValue::Module(Arc::clone(m)),
            PyValue::Placeholder(s) => ModuleValue::Str(s.clone()),
            _ => ModuleValue::Str(format!("<{}>", value.type_name())),
        }
    }

    /// Look up a name in the namespace hierarchy
    pub fn lookup_name(&self, name: &str) -> Option<&PyValue> {
        // LEGB rule: Local, Enclosing, Global, Built-in
        self.locals
            .get(name)
            .or_else(|| self.globals.get(name))
            .or_else(|| self.builtins.get(name))
    }

    /// Store a name in the local namespace
    pub fn store_name(&mut self, name: impl Into<String>, value: PyValue) {
        self.locals.insert(name.into(), value);
    }

    /// Store a name in the global namespace
    pub fn store_global(&mut self, name: impl Into<String>, value: PyValue) {
        self.globals.insert(name.into(), value);
    }

    /// Get the global namespace
    pub fn globals(&self) -> &HashMap<String, PyValue> {
        &self.globals
    }

    /// Get the local namespace
    pub fn locals(&self) -> &HashMap<String, PyValue> {
        &self.locals
    }
}

impl Default for ModuleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_function_creation() {
        let code = CodeObject::new("test_func", "<test>");
        let globals = Arc::new(HashMap::new());

        let func =
            PyFunction::new("test_func", "test_func", "test_module", Arc::new(code), globals);

        assert_eq!(func.name, "test_func");
        assert_eq!(func.module, "test_module");
        assert!(!func.is_generator);
        assert!(!func.is_async);
    }

    #[test]
    fn test_py_function_with_doc() {
        let code = CodeObject::new("documented", "<test>");
        let globals = Arc::new(HashMap::new());

        let func = PyFunction::new("documented", "documented", "test", Arc::new(code), globals)
            .with_doc("This is a docstring");

        assert_eq!(func.doc, Some("This is a docstring".to_string()));
    }

    #[test]
    fn test_py_class_creation() {
        let class = PyClass::new("TestClass", "TestClass", "test_module");

        assert_eq!(class.name, "TestClass");
        assert_eq!(class.module, "test_module");
        assert!(class.bases.is_empty());
    }

    #[test]
    fn test_py_class_with_method() {
        let mut class = PyClass::new("MyClass", "MyClass", "test");

        let code = CodeObject::new("my_method", "<test>");
        let globals = Arc::new(HashMap::new());
        let method =
            PyFunction::new("my_method", "MyClass.my_method", "test", Arc::new(code), globals);

        class.add_method("my_method", method);

        assert!(class.get_attribute("my_method").is_some());
    }

    #[test]
    fn test_py_instance_creation() {
        let class = Arc::new(PyClass::new("TestClass", "TestClass", "test"));
        let mut instance = PyInstance::new(class);

        instance.set_attribute("x", PyValue::Int(42));

        assert_eq!(instance.get_attribute("x"), Some(PyValue::Int(42)));
    }

    #[test]
    fn test_py_value_type_names() {
        assert_eq!(PyValue::None.type_name(), "NoneType");
        assert_eq!(PyValue::Bool(true).type_name(), "bool");
        assert_eq!(PyValue::Int(42).type_name(), "int");
        assert_eq!(PyValue::Float(3.125).type_name(), "float");
        assert_eq!(PyValue::Str("hello".to_string()).type_name(), "str");
    }

    #[test]
    fn test_py_value_truthiness() {
        assert!(!PyValue::None.is_truthy());
        assert!(!PyValue::Bool(false).is_truthy());
        assert!(PyValue::Bool(true).is_truthy());
        assert!(!PyValue::Int(0).is_truthy());
        assert!(PyValue::Int(1).is_truthy());
        assert!(!PyValue::Str(String::new()).is_truthy());
        assert!(PyValue::Str("hello".to_string()).is_truthy());
    }

    #[test]
    fn test_code_object_flags() {
        let mut code = CodeObject::new("gen", "<test>");
        code.flags = CodeFlags::GENERATOR;

        assert!(code.is_generator());
        assert!(!code.is_coroutine());

        code.flags = CodeFlags::COROUTINE;
        assert!(!code.is_generator());
        assert!(code.is_coroutine());
    }

    #[test]
    fn test_module_executor_creation() {
        let executor = ModuleExecutor::new();

        // Check built-ins are available
        assert!(executor.lookup_name("None").is_some());
        assert!(executor.lookup_name("True").is_some());
        assert!(executor.lookup_name("False").is_some());
        assert!(executor.lookup_name("print").is_some());
    }

    #[test]
    fn test_module_executor_store_lookup() {
        let mut executor = ModuleExecutor::new();

        executor.store_name("x", PyValue::Int(42));

        assert_eq!(executor.lookup_name("x"), Some(&PyValue::Int(42)));
    }

    #[test]
    fn test_module_executor_global_store() {
        let mut executor = ModuleExecutor::new();

        executor.store_global("CONSTANT", PyValue::Str("value".to_string()));

        assert!(executor.globals().contains_key("CONSTANT"));
    }

    #[test]
    fn test_create_function_from_code() {
        let executor = ModuleExecutor::new();

        let mut code = CodeObject::new("my_func", "<test>");
        code.consts.push(PyValue::Str("Function docstring".to_string()));

        let func = executor.create_function(&code, "test_module");

        assert_eq!(func.name, "my_func");
        assert_eq!(func.doc, Some("Function docstring".to_string()));
    }

    // ===== MRO Tests =====

    #[test]
    fn test_mro_no_bases() {
        let class = PyClass::new("NoBase", "NoBase", "test");
        assert!(class.mro.is_empty());
    }

    #[test]
    fn test_mro_single_inheritance() {
        // A -> B (B inherits from A)
        let a = Arc::new(PyClass::new("A", "A", "test"));
        let b = PyClass::new("B", "B", "test").with_bases(vec![Arc::clone(&a)]);

        // MRO of B should be [A]
        assert_eq!(b.mro.len(), 1);
        assert_eq!(b.mro[0].name, "A");
    }

    #[test]
    fn test_mro_chain_inheritance() {
        // A -> B -> C (C inherits from B, B inherits from A)
        let a = Arc::new(PyClass::new("A", "A", "test"));
        let b = Arc::new(PyClass::new("B", "B", "test").with_bases(vec![Arc::clone(&a)]));
        let c = PyClass::new("C", "C", "test").with_bases(vec![Arc::clone(&b)]);

        // MRO of C should be [B, A]
        assert_eq!(c.mro.len(), 2);
        assert_eq!(c.mro[0].name, "B");
        assert_eq!(c.mro[1].name, "A");
    }

    #[test]
    fn test_mro_diamond_inheritance() {
        // Diamond pattern:
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D
        let a = Arc::new(PyClass::new("A", "A", "test"));
        let b = Arc::new(PyClass::new("B", "B", "test").with_bases(vec![Arc::clone(&a)]));
        let c = Arc::new(PyClass::new("C", "C", "test").with_bases(vec![Arc::clone(&a)]));
        let d = PyClass::new("D", "D", "test").with_bases(vec![Arc::clone(&b), Arc::clone(&c)]);

        // MRO of D should be [B, C, A] (C3 linearization)
        assert_eq!(d.mro.len(), 3);
        assert_eq!(d.mro[0].name, "B");
        assert_eq!(d.mro[1].name, "C");
        assert_eq!(d.mro[2].name, "A");
    }

    #[test]
    fn test_mro_multiple_inheritance() {
        // Multiple inheritance without diamond:
        // A   B
        //  \ /
        //   C
        let a = Arc::new(PyClass::new("A", "A", "test"));
        let b = Arc::new(PyClass::new("B", "B", "test"));
        let c = PyClass::new("C", "C", "test").with_bases(vec![Arc::clone(&a), Arc::clone(&b)]);

        // MRO of C should be [A, B]
        assert_eq!(c.mro.len(), 2);
        assert_eq!(c.mro[0].name, "A");
        assert_eq!(c.mro[1].name, "B");
    }

    #[test]
    fn test_get_method_from_own_class() {
        let mut class = PyClass::new("MyClass", "MyClass", "test");

        let code = CodeObject::new("my_method", "<test>");
        let globals = Arc::new(HashMap::new());
        let method = PyFunction::new("my_method", "MyClass.my_method", "test", Arc::new(code), globals);

        class.add_method("my_method", method);

        let found = class.get_method("my_method");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my_method");
    }

    #[test]
    fn test_get_method_from_base_class() {
        // Base class with a method
        let mut base = PyClass::new("Base", "Base", "test");
        let code = CodeObject::new("base_method", "<test>");
        let globals = Arc::new(HashMap::new());
        let method = PyFunction::new("base_method", "Base.base_method", "test", Arc::new(code), globals);
        base.add_method("base_method", method);

        let base = Arc::new(base);

        // Derived class without the method
        let derived = PyClass::new("Derived", "Derived", "test").with_bases(vec![Arc::clone(&base)]);

        // Should find method in base class via MRO
        let found = derived.get_method("base_method");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "base_method");
    }

    #[test]
    fn test_get_method_override() {
        // Base class with a method
        let mut base = PyClass::new("Base", "Base", "test");
        let base_code = CodeObject::new("method", "<test>");
        let globals = Arc::new(HashMap::new());
        let base_method = PyFunction::new("method", "Base.method", "test", Arc::new(base_code), globals.clone());
        base.add_method("method", base_method);

        let base = Arc::new(base);

        // Derived class overriding the method
        let mut derived = PyClass::new("Derived", "Derived", "test").with_bases(vec![Arc::clone(&base)]);
        let derived_code = CodeObject::new("method", "<test>");
        let derived_method = PyFunction::new("method", "Derived.method", "test", Arc::new(derived_code), globals);
        derived.add_method("method", derived_method);

        // Should find the overridden method in derived class
        let found = derived.get_method("method");
        assert!(found.is_some());
        assert_eq!(found.unwrap().qualname, "Derived.method");
    }

    #[test]
    fn test_get_method_not_found() {
        let class = PyClass::new("MyClass", "MyClass", "test");
        let found = class.get_method("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_mro_complex_hierarchy() {
        // More complex hierarchy:
        //       O
        //      /|\
        //     A B C
        //      \|/
        //       D
        let o = Arc::new(PyClass::new("O", "O", "test"));
        let a = Arc::new(PyClass::new("A", "A", "test").with_bases(vec![Arc::clone(&o)]));
        let b = Arc::new(PyClass::new("B", "B", "test").with_bases(vec![Arc::clone(&o)]));
        let c = Arc::new(PyClass::new("C", "C", "test").with_bases(vec![Arc::clone(&o)]));
        let d = PyClass::new("D", "D", "test").with_bases(vec![
            Arc::clone(&a),
            Arc::clone(&b),
            Arc::clone(&c),
        ]);

        // MRO of D should be [A, B, C, O]
        assert_eq!(d.mro.len(), 4);
        assert_eq!(d.mro[0].name, "A");
        assert_eq!(d.mro[1].name, "B");
        assert_eq!(d.mro[2].name, "C");
        assert_eq!(d.mro[3].name, "O");
    }
}
