//! PyList - Python list type

use crate::error::{RuntimeError, RuntimeResult};
use crate::header::{ObjectFlags, PyObjectHeader, TypeTag};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

/// A Python cell for closure variables
///
/// Cells are used to implement closures - they hold a reference to a value
/// that can be shared between a function and its enclosing scope.
#[derive(Debug, Clone)]
pub struct PyCell {
    /// The value stored in the cell
    value: Arc<RwLock<PyValue>>,
}

impl PyCell {
    /// Create a new cell with the given value
    pub fn new(value: PyValue) -> Self {
        Self {
            value: Arc::new(RwLock::new(value)),
        }
    }

    /// Create an empty cell (unbound)
    pub fn empty() -> Self {
        Self::new(PyValue::None)
    }

    /// Get the value from the cell
    pub fn get(&self) -> PyValue {
        self.value.read().clone()
    }

    /// Set the value in the cell
    pub fn set(&self, value: PyValue) {
        *self.value.write() = value;
    }

    /// Check if the cell is empty (contains None)
    pub fn is_empty(&self) -> bool {
        matches!(*self.value.read(), PyValue::None)
    }
}

/// A Python module object
#[derive(Debug, Clone)]
pub struct PyModule {
    /// Object header
    pub header: PyObjectHeader,
    /// Module name
    pub name: Arc<str>,
    /// Module file path (if loaded from file)
    pub file: Option<PathBuf>,
    /// Module dictionary (__dict__)
    pub dict: Arc<DashMap<String, PyValue>>,
    /// Module documentation
    pub doc: Option<Arc<str>>,
    /// Package name (for submodules)
    pub package: Option<Arc<str>>,
    /// Whether the module has been fully initialized
    pub initialized: bool,
}

impl PyModule {
    /// Create a new module with the given name
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        let name = name.into();
        let dict = Arc::new(DashMap::new());

        // Set __name__ attribute
        dict.insert("__name__".to_string(), PyValue::Str(Arc::clone(&name)));

        Self {
            header: PyObjectHeader::new(TypeTag::Module, ObjectFlags::NONE),
            name,
            file: None,
            dict,
            doc: None,
            package: None,
            initialized: false,
        }
    }

    /// Set the file path
    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.dict.insert(
            "__file__".to_string(),
            PyValue::Str(Arc::from(file.to_string_lossy().as_ref())),
        );
        self.file = Some(file);
        self
    }

    /// Set the documentation
    pub fn with_doc(mut self, doc: impl Into<Arc<str>>) -> Self {
        let doc = doc.into();
        self.dict.insert("__doc__".to_string(), PyValue::Str(Arc::clone(&doc)));
        self.doc = Some(doc);
        self
    }

    /// Set the package name
    pub fn with_package(mut self, package: impl Into<Arc<str>>) -> Self {
        let package = package.into();
        self.dict.insert("__package__".to_string(), PyValue::Str(Arc::clone(&package)));
        self.package = Some(package);
        self
    }

    /// Mark the module as initialized
    pub fn mark_initialized(&mut self) {
        self.initialized = true;
    }

    /// Get an attribute from the module
    pub fn get_attr(&self, name: &str) -> Option<PyValue> {
        self.dict.get(name).map(|v| v.clone())
    }

    /// Set an attribute on the module
    pub fn set_attr(&self, name: impl Into<String>, value: PyValue) {
        self.dict.insert(name.into(), value);
    }

    /// Check if module has an attribute
    pub fn has_attr(&self, name: &str) -> bool {
        self.dict.contains_key(name)
    }

    /// Delete an attribute from the module
    pub fn del_attr(&self, name: &str) -> bool {
        self.dict.remove(name).is_some()
    }

    /// Get all attribute names
    pub fn dir(&self) -> Vec<String> {
        self.dict.iter().map(|r| r.key().clone()).collect()
    }
}

/// A Python code object (compiled bytecode)
///
/// This is a simplified representation for use in PyValue.
/// The full CodeObject is in dx-py-bytecode.
#[derive(Debug, Clone)]
pub struct PyCode {
    /// Object header
    pub header: PyObjectHeader,
    /// Function/code name
    pub name: Arc<str>,
    /// Qualified name
    pub qualname: Arc<str>,
    /// Source filename
    pub filename: Arc<str>,
    /// First line number
    pub firstlineno: u32,
    /// Number of positional arguments
    pub argcount: u32,
    /// Number of positional-only arguments
    pub posonlyargcount: u32,
    /// Number of keyword-only arguments
    pub kwonlyargcount: u32,
    /// Number of local variables
    pub nlocals: u32,
    /// Stack size needed
    pub stacksize: u32,
    /// Code flags
    pub flags: u32,
    /// Bytecode
    pub code: Arc<[u8]>,
    /// Constants
    pub constants: Arc<[PyValue]>,
    /// Names used
    pub names: Arc<[Arc<str>]>,
    /// Local variable names
    pub varnames: Arc<[Arc<str>]>,
    /// Free variable names (from enclosing scope)
    pub freevars: Arc<[Arc<str>]>,
    /// Cell variable names (used by nested functions)
    pub cellvars: Arc<[Arc<str>]>,
}

impl PyCode {
    /// Create a new code object
    pub fn new(name: impl Into<Arc<str>>, filename: impl Into<Arc<str>>) -> Self {
        let name = name.into();
        Self {
            header: PyObjectHeader::new(TypeTag::Code, ObjectFlags::NONE),
            qualname: Arc::clone(&name),
            name,
            filename: filename.into(),
            firstlineno: 1,
            argcount: 0,
            posonlyargcount: 0,
            kwonlyargcount: 0,
            nlocals: 0,
            stacksize: 0,
            flags: 0,
            code: Arc::from([]),
            constants: Arc::from([]),
            names: Arc::from([]),
            varnames: Arc::from([]),
            freevars: Arc::from([]),
            cellvars: Arc::from([]),
        }
    }

    /// Check if this is a generator function
    pub fn is_generator(&self) -> bool {
        self.flags & 0x0020 != 0 // GENERATOR flag
    }

    /// Check if this is a coroutine
    pub fn is_coroutine(&self) -> bool {
        self.flags & 0x0080 != 0 // COROUTINE flag
    }

    /// Check if this function has *args
    pub fn has_varargs(&self) -> bool {
        self.flags & 0x0004 != 0 // VARARGS flag
    }

    /// Check if this function has **kwargs
    pub fn has_varkeywords(&self) -> bool {
        self.flags & 0x0008 != 0 // VARKEYWORDS flag
    }
}

/// A Python value (simplified for core types)
#[derive(Clone)]
pub enum PyValue {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Arc<str>),
    List(Arc<PyList>),
    Set(Arc<PySet>),
    Tuple(Arc<crate::PyTuple>),
    Dict(Arc<crate::PyDict>),
    Exception(Arc<crate::pyexception::PyException>),
    Type(Arc<crate::types::PyType>),
    Instance(Arc<crate::types::PyInstance>),
    BoundMethod(crate::types::BoundMethod),
    Generator(Arc<crate::pygenerator::PyGenerator>),
    Coroutine(Arc<crate::pygenerator::PyCoroutine>),
    Builtin(Arc<crate::pyfunction::PyBuiltinFunction>),
    Function(Arc<crate::pyfunction::PyFunction>),
    Iterator(Arc<PyIterator>),
    /// A Python module
    Module(Arc<PyModule>),
    /// A Python code object (compiled bytecode)
    Code(Arc<PyCode>),
    /// A cell for closure variables
    Cell(Arc<PyCell>),
    /// A super object for super() calls
    Super(Arc<crate::types::PySuper>),
    /// A property descriptor
    Property(Arc<crate::types::PropertyDescriptor>),
    /// A static method descriptor
    StaticMethod(Box<PyValue>),
    /// A class method descriptor
    ClassMethod(Box<PyValue>),
}

/// Python iterator wrapper
pub struct PyIterator {
    /// Current index
    index: std::sync::atomic::AtomicUsize,
    /// Items to iterate over
    items: Vec<PyValue>,
}

impl PyIterator {
    /// Create a new iterator from a list of items
    pub fn new(items: Vec<PyValue>) -> Self {
        Self {
            index: std::sync::atomic::AtomicUsize::new(0),
            items,
        }
    }

    /// Get the next item
    pub fn next(&self) -> Option<PyValue> {
        let idx = self.index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.items.get(idx).cloned()
    }
}

impl PyValue {
    pub fn type_name(&self) -> &str {
        match self {
            PyValue::None => "NoneType",
            PyValue::Bool(_) => "bool",
            PyValue::Int(_) => "int",
            PyValue::Float(_) => "float",
            PyValue::Str(_) => "str",
            PyValue::List(_) => "list",
            PyValue::Set(_) => "set",
            PyValue::Tuple(_) => "tuple",
            PyValue::Dict(_) => "dict",
            PyValue::Exception(_) => "exception",
            PyValue::Type(_) => "type",
            PyValue::Instance(inst) => &inst.class.name,
            PyValue::BoundMethod(_) => "method",
            PyValue::Generator(_) => "generator",
            PyValue::Coroutine(_) => "coroutine",
            PyValue::Builtin(_) => "builtin_function_or_method",
            PyValue::Function(_) => "function",
            PyValue::Iterator(_) => "iterator",
            PyValue::Module(_) => "module",
            PyValue::Code(_) => "code",
            PyValue::Cell(_) => "cell",
            PyValue::Super(_) => "super",
            PyValue::Property(_) => "property",
            PyValue::StaticMethod(_) => "staticmethod",
            PyValue::ClassMethod(_) => "classmethod",
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            PyValue::None => false,
            PyValue::Bool(b) => *b,
            PyValue::Int(i) => *i != 0,
            PyValue::Float(f) => *f != 0.0,
            PyValue::Str(s) => !s.is_empty(),
            PyValue::List(l) => !l.is_empty(),
            PyValue::Set(s) => !s.is_empty(),
            PyValue::Tuple(t) => !t.is_empty(),
            PyValue::Dict(d) => !d.is_empty(),
            PyValue::Exception(_) => true,
            PyValue::Type(_) => true,
            PyValue::Instance(_) => true,
            PyValue::BoundMethod(_) => true,
            PyValue::Generator(_) => true,
            PyValue::Coroutine(_) => true,
            PyValue::Builtin(_) => true,
            PyValue::Function(_) => true,
            PyValue::Iterator(_) => true,
            PyValue::Module(_) => true,
            PyValue::Code(_) => true,
            PyValue::Cell(cell) => !cell.is_empty(),
            PyValue::Super(_) => true,
            PyValue::Property(_) => true,
            PyValue::StaticMethod(_) => true,
            PyValue::ClassMethod(_) => true,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, PyValue::None)
    }

    pub fn as_instance(&self) -> Option<&Arc<crate::types::PyInstance>> {
        match self {
            PyValue::Instance(inst) => Some(inst),
            _ => None,
        }
    }

    pub fn as_type(&self) -> Option<&Arc<crate::types::PyType>> {
        match self {
            PyValue::Type(ty) => Some(ty),
            _ => None,
        }
    }

    pub fn as_generator(&self) -> Option<&Arc<crate::pygenerator::PyGenerator>> {
        match self {
            PyValue::Generator(gen) => Some(gen),
            _ => None,
        }
    }

    pub fn as_coroutine(&self) -> Option<&Arc<crate::pygenerator::PyCoroutine>> {
        match self {
            PyValue::Coroutine(coro) => Some(coro),
            _ => None,
        }
    }

    pub fn as_bound_method(&self) -> Option<&crate::types::BoundMethod> {
        match self {
            PyValue::BoundMethod(method) => Some(method),
            _ => None,
        }
    }

    /// Handle reference counting cleanup for this value
    /// This should be called when a reference to this value is being dropped
    pub fn handle_cleanup(&self) {
        // Only objects with headers need cleanup
        match self {
            PyValue::List(list) => {
                // Check if this might be part of a cycle before decrementing
                if list.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if list.header.decref_with_cleanup(|| {
                    // Clear weak references first
                    crate::weakref::clear_weak_references(self);
                    // Call finalizer if needed
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated - this happens automatically when Arc is dropped
                }
            }
            PyValue::Tuple(tuple) => {
                if tuple.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if tuple.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            PyValue::Dict(dict) => {
                if dict.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if dict.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            PyValue::Instance(instance) => {
                if instance.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if instance.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            PyValue::Type(type_obj) => {
                if type_obj.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if type_obj.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            PyValue::Generator(gen) => {
                if gen.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if gen.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            PyValue::Coroutine(coro) => {
                if coro.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if coro.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            // Primitive types don't need cleanup
            PyValue::None
            | PyValue::Bool(_)
            | PyValue::Int(_)
            | PyValue::Float(_)
            | PyValue::Str(_) => {}
            // Set has a header
            PyValue::Set(set) => {
                if set.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if set.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            // Exception, BoundMethod, Builtin, Function, Iterator don't have headers in current implementation
            PyValue::Exception(_)
            | PyValue::BoundMethod(_)
            | PyValue::Builtin(_)
            | PyValue::Function(_)
            | PyValue::Iterator(_) => {}
            // Module has a header but uses Arc for reference counting
            PyValue::Module(module) => {
                if module.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if module.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            // Code objects have headers
            PyValue::Code(code) => {
                if code.header.refcount() > 1 {
                    crate::gc::add_potential_cycle(self);
                }

                if code.header.decref_with_cleanup(|| {
                    crate::weakref::clear_weak_references(self);
                    if let Err(e) = crate::cleanup::CleanupManager::finalize_object(self) {
                        eprintln!("Warning: Error during finalization: {}", e);
                    }
                }) {
                    // Object should be deallocated
                }
            }
            // Cells don't have headers, they use Arc internally
            PyValue::Cell(_) => {}
            // Super objects don't have headers, they use Arc internally
            PyValue::Super(_) => {}
            // Property, StaticMethod, ClassMethod don't have headers
            PyValue::Property(_) | PyValue::StaticMethod(_) | PyValue::ClassMethod(_) => {}
        }
    }

    /// Get the value as a module reference
    pub fn as_module(&self) -> Option<&Arc<PyModule>> {
        match self {
            PyValue::Module(m) => Some(m),
            _ => None,
        }
    }

    /// Get the value as a code object reference
    pub fn as_code(&self) -> Option<&Arc<PyCode>> {
        match self {
            PyValue::Code(c) => Some(c),
            _ => None,
        }
    }

    /// Get the value as a cell reference
    pub fn as_cell(&self) -> Option<&Arc<PyCell>> {
        match self {
            PyValue::Cell(c) => Some(c),
            _ => None,
        }
    }

    /// Get the value as a function reference
    pub fn as_function(&self) -> Option<&Arc<crate::pyfunction::PyFunction>> {
        match self {
            PyValue::Function(f) => Some(f),
            _ => None,
        }
    }

    /// Get the value as a builtin function reference
    pub fn as_builtin(&self) -> Option<&Arc<crate::pyfunction::PyBuiltinFunction>> {
        match self {
            PyValue::Builtin(b) => Some(b),
            _ => None,
        }
    }
}

impl std::fmt::Debug for PyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PyValue::None => write!(f, "None"),
            PyValue::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            PyValue::Int(i) => write!(f, "{}", i),
            PyValue::Float(fl) => write!(f, "{}", fl),
            PyValue::Str(s) => write!(f, "'{}'", s),
            PyValue::List(_) => write!(f, "[...]"),
            PyValue::Set(_) => write!(f, "{{...}}"),
            PyValue::Tuple(_) => write!(f, "(...)"),
            PyValue::Dict(_) => write!(f, "{{...}}"),
            PyValue::Exception(e) => write!(f, "<{}: {}>", e.exc_type, e.message),
            PyValue::Type(t) => write!(f, "<class '{}'>", t.name),
            PyValue::Instance(inst) => write!(f, "<{} object>", inst.class.name),
            PyValue::BoundMethod(_) => write!(f, "<bound method>"),
            PyValue::Generator(gen) => write!(f, "<generator '{}'>", gen.name),
            PyValue::Coroutine(coro) => write!(f, "<coroutine '{}'>", coro.name),
            PyValue::Builtin(b) => write!(f, "<built-in function {}>", b.name),
            PyValue::Function(func) => write!(f, "<function {}>", func.name),
            PyValue::Iterator(_) => write!(f, "<iterator>"),
            PyValue::Module(m) => write!(f, "<module '{}'>", m.name),
            PyValue::Code(c) => write!(f, "<code object {} at {:p}>", c.name, Arc::as_ptr(c)),
            PyValue::Cell(cell) => write!(f, "<cell: {:?}>", cell.get()),
            PyValue::Super(s) => write!(f, "<super: <class '{}'>>", s.type_.name),
            PyValue::Property(p) => write!(f, "<property: {}>", p.get_doc().unwrap_or("no doc")),
            PyValue::StaticMethod(inner) => write!(f, "<staticmethod({:?})>", inner),
            PyValue::ClassMethod(inner) => write!(f, "<classmethod({:?})>", inner),
        }
    }
}

/// Python list object
pub struct PyList {
    /// Object header
    pub header: PyObjectHeader,
    /// List elements (thread-safe)
    elements: RwLock<Vec<PyValue>>,
}

impl PyList {
    /// Create a new empty list
    pub fn new() -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::List, ObjectFlags::NONE),
            elements: RwLock::new(Vec::new()),
        }
    }

    /// Create a list with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::List, ObjectFlags::NONE),
            elements: RwLock::new(Vec::with_capacity(capacity)),
        }
    }

    /// Create from values
    pub fn from_values(values: Vec<PyValue>) -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::List, ObjectFlags::NONE),
            elements: RwLock::new(values),
        }
    }

    /// Get length
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.read().len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.read().is_empty()
    }

    /// Get item at index
    pub fn getitem(&self, index: i64) -> RuntimeResult<PyValue> {
        let elements = self.elements.read();
        let len = elements.len() as i64;
        let idx = if index < 0 { len + index } else { index };

        if idx < 0 || idx >= len {
            return Err(RuntimeError::index_error(index, elements.len()));
        }

        Ok(elements[idx as usize].clone())
    }

    /// Set item at index
    pub fn setitem(&self, index: i64, value: PyValue) -> RuntimeResult<()> {
        let mut elements = self.elements.write();
        let len = elements.len() as i64;
        let idx = if index < 0 { len + index } else { index };

        if idx < 0 || idx >= len {
            return Err(RuntimeError::index_error(index, elements.len()));
        }

        elements[idx as usize] = value;
        Ok(())
    }

    /// Append item
    pub fn append(&self, value: PyValue) {
        self.elements.write().push(value);
    }

    /// Insert item at index
    pub fn insert(&self, index: i64, value: PyValue) {
        let mut elements = self.elements.write();
        let len = elements.len() as i64;
        let idx = if index < 0 {
            (len + index + 1).max(0) as usize
        } else {
            (index as usize).min(elements.len())
        };
        elements.insert(idx, value);
    }

    /// Remove and return item at index
    pub fn pop(&self, index: Option<i64>) -> RuntimeResult<PyValue> {
        let mut elements = self.elements.write();
        if elements.is_empty() {
            return Err(RuntimeError::index_error(-1, 0));
        }

        let len = elements.len() as i64;
        let idx = match index {
            Some(i) if i < 0 => (len + i) as usize,
            Some(i) => i as usize,
            None => elements.len() - 1,
        };

        if idx >= elements.len() {
            return Err(RuntimeError::index_error(idx as i64, elements.len()));
        }

        Ok(elements.remove(idx))
    }

    /// Remove first occurrence of value
    pub fn remove(&self, value: &PyValue) -> RuntimeResult<()> {
        let mut elements = self.elements.write();
        for (i, elem) in elements.iter().enumerate() {
            if Self::values_equal(elem, value) {
                elements.remove(i);
                return Ok(());
            }
        }
        Err(RuntimeError::value_error("list.remove(x): x not in list"))
    }

    /// Clear the list
    pub fn clear(&self) {
        self.elements.write().clear();
    }

    /// Extend with items from iterator
    pub fn extend(&self, items: impl IntoIterator<Item = PyValue>) {
        self.elements.write().extend(items);
    }

    /// Get slice
    pub fn slice(&self, start: Option<i64>, end: Option<i64>) -> PyList {
        let elements = self.elements.read();
        let len = elements.len() as i64;

        let start = match start {
            Some(s) if s < 0 => (len + s).max(0) as usize,
            Some(s) => (s as usize).min(elements.len()),
            None => 0,
        };

        let end = match end {
            Some(e) if e < 0 => (len + e).max(0) as usize,
            Some(e) => (e as usize).min(elements.len()),
            None => elements.len(),
        };

        if start >= end {
            return PyList::new();
        }

        PyList::from_values(elements[start..end].to_vec())
    }

    /// Reverse in place
    pub fn reverse(&self) {
        self.elements.write().reverse();
    }

    /// Sort in place (simplified - only works for homogeneous numeric lists)
    pub fn sort(&self) -> RuntimeResult<()> {
        let mut elements = self.elements.write();

        // Check if all elements are comparable (simplified: only ints)
        let all_ints = elements.iter().all(|v| matches!(v, PyValue::Int(_)));

        if all_ints {
            elements.sort_by(|a, b| {
                if let (PyValue::Int(x), PyValue::Int(y)) = (a, b) {
                    x.cmp(y)
                } else {
                    std::cmp::Ordering::Equal
                }
            });
            Ok(())
        } else {
            Err(RuntimeError::type_error("comparable types", "mixed types"))
        }
    }

    /// Count occurrences
    pub fn count(&self, value: &PyValue) -> usize {
        self.elements.read().iter().filter(|v| Self::values_equal(v, value)).count()
    }

    /// Find index of value
    pub fn index(&self, value: &PyValue) -> RuntimeResult<usize> {
        self.elements
            .read()
            .iter()
            .position(|v| Self::values_equal(v, value))
            .ok_or_else(|| RuntimeError::value_error("value not in list"))
    }

    /// Check if contains value
    pub fn contains(&self, value: &PyValue) -> bool {
        self.elements.read().iter().any(|v| Self::values_equal(v, value))
    }

    /// Concatenate two lists
    pub fn concat(&self, other: &PyList) -> PyList {
        let mut elements = self.elements.read().clone();
        elements.extend(other.elements.read().iter().cloned());
        PyList::from_values(elements)
    }

    /// Repeat list n times
    pub fn repeat(&self, n: usize) -> PyList {
        let elements = self.elements.read();
        let mut result = Vec::with_capacity(elements.len() * n);
        for _ in 0..n {
            result.extend(elements.iter().cloned());
        }
        PyList::from_values(result)
    }

    /// Get all elements as a vector
    pub fn to_vec(&self) -> Vec<PyValue> {
        self.elements.read().clone()
    }

    /// Simple value equality check
    fn values_equal(a: &PyValue, b: &PyValue) -> bool {
        match (a, b) {
            (PyValue::None, PyValue::None) => true,
            (PyValue::Bool(x), PyValue::Bool(y)) => x == y,
            (PyValue::Int(x), PyValue::Int(y)) => x == y,
            (PyValue::Float(x), PyValue::Float(y)) => x == y,
            (PyValue::Str(x), PyValue::Str(y)) => x == y,
            _ => false,
        }
    }
}

impl Default for PyList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PyList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PyList({:?})", self.elements.read())
    }
}

/// Python set object
///
/// A set is an unordered collection of unique elements.
/// For simplicity, this implementation uses a Vec and checks for duplicates on add.
pub struct PySet {
    /// Object header
    pub header: PyObjectHeader,
    /// Set elements (thread-safe)
    elements: RwLock<Vec<PyValue>>,
}

impl PySet {
    /// Create a new empty set
    pub fn new() -> Self {
        Self {
            header: PyObjectHeader::new(TypeTag::Set, ObjectFlags::NONE),
            elements: RwLock::new(Vec::new()),
        }
    }

    /// Create from values (removes duplicates)
    pub fn from_values(values: Vec<PyValue>) -> Self {
        let mut unique = Vec::new();
        for value in values {
            if !unique.iter().any(|v| Self::values_equal(v, &value)) {
                unique.push(value);
            }
        }
        Self {
            header: PyObjectHeader::new(TypeTag::Set, ObjectFlags::NONE),
            elements: RwLock::new(unique),
        }
    }

    /// Get length
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.read().len()
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.read().is_empty()
    }

    /// Add an element to the set
    pub fn add(&self, value: PyValue) {
        let mut elements = self.elements.write();
        if !elements.iter().any(|v| Self::values_equal(v, &value)) {
            elements.push(value);
        }
    }

    /// Remove an element from the set
    pub fn remove(&self, value: &PyValue) -> RuntimeResult<()> {
        let mut elements = self.elements.write();
        for (i, elem) in elements.iter().enumerate() {
            if Self::values_equal(elem, value) {
                elements.remove(i);
                return Ok(());
            }
        }
        Err(RuntimeError::key_error("element not in set"))
    }

    /// Discard an element (no error if not present)
    pub fn discard(&self, value: &PyValue) {
        let mut elements = self.elements.write();
        if let Some(i) = elements.iter().position(|v| Self::values_equal(v, value)) {
            elements.remove(i);
        }
    }

    /// Check if contains value
    pub fn contains(&self, value: &PyValue) -> bool {
        self.elements.read().iter().any(|v| Self::values_equal(v, value))
    }

    /// Clear the set
    pub fn clear(&self) {
        self.elements.write().clear();
    }

    /// Get all elements as a vector
    pub fn to_vec(&self) -> Vec<PyValue> {
        self.elements.read().clone()
    }

    /// Union with another set
    pub fn union(&self, other: &PySet) -> PySet {
        let mut result = self.elements.read().clone();
        for elem in other.elements.read().iter() {
            if !result.iter().any(|v| Self::values_equal(v, elem)) {
                result.push(elem.clone());
            }
        }
        PySet::from_values(result)
    }

    /// Intersection with another set
    pub fn intersection(&self, other: &PySet) -> PySet {
        let elements = self.elements.read();
        let other_elements = other.elements.read();
        let result: Vec<PyValue> = elements
            .iter()
            .filter(|v| other_elements.iter().any(|o| Self::values_equal(v, o)))
            .cloned()
            .collect();
        PySet::from_values(result)
    }

    /// Difference with another set
    pub fn difference(&self, other: &PySet) -> PySet {
        let elements = self.elements.read();
        let other_elements = other.elements.read();
        let result: Vec<PyValue> = elements
            .iter()
            .filter(|v| !other_elements.iter().any(|o| Self::values_equal(v, o)))
            .cloned()
            .collect();
        PySet::from_values(result)
    }

    /// Simple value equality check
    fn values_equal(a: &PyValue, b: &PyValue) -> bool {
        match (a, b) {
            (PyValue::None, PyValue::None) => true,
            (PyValue::Bool(x), PyValue::Bool(y)) => x == y,
            (PyValue::Int(x), PyValue::Int(y)) => x == y,
            (PyValue::Float(x), PyValue::Float(y)) => x == y,
            (PyValue::Str(x), PyValue::Str(y)) => x == y,
            _ => false,
        }
    }
}

impl Default for PySet {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PySet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PySet({:?})", self.elements.read())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_creation() {
        let list = PyList::new();
        assert!(list.is_empty());
        assert_eq!(list.header.type_tag(), TypeTag::List);
    }

    #[test]
    fn test_list_append_get() {
        let list = PyList::new();
        list.append(PyValue::Int(1));
        list.append(PyValue::Int(2));
        list.append(PyValue::Int(3));

        assert_eq!(list.len(), 3);

        if let PyValue::Int(v) = list.getitem(0).unwrap() {
            assert_eq!(v, 1);
        } else {
            panic!("Expected Int");
        }

        if let PyValue::Int(v) = list.getitem(-1).unwrap() {
            assert_eq!(v, 3);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_list_slice() {
        let list = PyList::from_values(vec![
            PyValue::Int(1),
            PyValue::Int(2),
            PyValue::Int(3),
            PyValue::Int(4),
            PyValue::Int(5),
        ]);

        let slice = list.slice(Some(1), Some(4));
        assert_eq!(slice.len(), 3);
    }

    #[test]
    fn test_list_pop() {
        let list = PyList::from_values(vec![PyValue::Int(1), PyValue::Int(2), PyValue::Int(3)]);

        let popped = list.pop(None).unwrap();
        if let PyValue::Int(v) = popped {
            assert_eq!(v, 3);
        }
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_sort() {
        let list = PyList::from_values(vec![PyValue::Int(3), PyValue::Int(1), PyValue::Int(2)]);

        list.sort().unwrap();

        if let PyValue::Int(v) = list.getitem(0).unwrap() {
            assert_eq!(v, 1);
        }
    }

    #[test]
    fn test_pycell_creation() {
        let cell = PyCell::new(PyValue::Int(42));
        if let PyValue::Int(v) = cell.get() {
            assert_eq!(v, 42);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_pycell_set() {
        let cell = PyCell::new(PyValue::Int(1));
        cell.set(PyValue::Int(100));
        if let PyValue::Int(v) = cell.get() {
            assert_eq!(v, 100);
        } else {
            panic!("Expected Int");
        }
    }

    #[test]
    fn test_pycell_empty() {
        let cell = PyCell::empty();
        assert!(cell.is_empty());

        cell.set(PyValue::Int(1));
        assert!(!cell.is_empty());
    }

    #[test]
    fn test_pymodule_creation() {
        let module = PyModule::new("test_module");
        assert_eq!(&*module.name, "test_module");
        assert!(!module.initialized);

        // Check __name__ is set
        if let Some(PyValue::Str(name)) = module.get_attr("__name__") {
            assert_eq!(&*name, "test_module");
        } else {
            panic!("Expected __name__ attribute");
        }
    }

    #[test]
    fn test_pymodule_attributes() {
        let module = PyModule::new("mymod");
        module.set_attr("x", PyValue::Int(42));

        assert!(module.has_attr("x"));
        if let Some(PyValue::Int(v)) = module.get_attr("x") {
            assert_eq!(v, 42);
        } else {
            panic!("Expected Int");
        }

        assert!(module.del_attr("x"));
        assert!(!module.has_attr("x"));
    }

    #[test]
    fn test_pymodule_with_doc() {
        let module = PyModule::new("documented").with_doc("This is a test module");

        assert_eq!(module.doc.as_deref(), Some("This is a test module"));
    }

    #[test]
    fn test_pycode_creation() {
        let code = PyCode::new("test_func", "test.py");
        assert_eq!(&*code.name, "test_func");
        assert_eq!(&*code.filename, "test.py");
        assert_eq!(code.firstlineno, 1);
    }

    #[test]
    fn test_pycode_flags() {
        let mut code = PyCode::new("gen", "test.py");
        code.flags = 0x0020; // GENERATOR
        assert!(code.is_generator());
        assert!(!code.is_coroutine());

        code.flags = 0x0080; // COROUTINE
        assert!(!code.is_generator());
        assert!(code.is_coroutine());

        code.flags = 0x0004; // VARARGS
        assert!(code.has_varargs());

        code.flags = 0x0008; // VARKEYWORDS
        assert!(code.has_varkeywords());
    }

    #[test]
    fn test_pyvalue_new_variants_type_name() {
        let module = Arc::new(PyModule::new("test"));
        let code = Arc::new(PyCode::new("func", "test.py"));
        let cell = Arc::new(PyCell::new(PyValue::Int(1)));

        assert_eq!(PyValue::Module(module).type_name(), "module");
        assert_eq!(PyValue::Code(code).type_name(), "code");
        assert_eq!(PyValue::Cell(cell).type_name(), "cell");
    }

    #[test]
    fn test_pyvalue_new_variants_to_bool() {
        let module = Arc::new(PyModule::new("test"));
        let code = Arc::new(PyCode::new("func", "test.py"));
        let cell_with_value = Arc::new(PyCell::new(PyValue::Int(1)));
        let cell_empty = Arc::new(PyCell::empty());

        assert!(PyValue::Module(module).to_bool());
        assert!(PyValue::Code(code).to_bool());
        assert!(PyValue::Cell(cell_with_value).to_bool());
        assert!(!PyValue::Cell(cell_empty).to_bool());
    }

    #[test]
    fn test_pyvalue_new_variants_debug() {
        let module = Arc::new(PyModule::new("mymodule"));
        let code = Arc::new(PyCode::new("myfunc", "test.py"));
        let cell = Arc::new(PyCell::new(PyValue::Int(42)));

        let module_debug = format!("{:?}", PyValue::Module(module));
        assert!(module_debug.contains("mymodule"));

        let code_debug = format!("{:?}", PyValue::Code(code));
        assert!(code_debug.contains("myfunc"));

        let cell_debug = format!("{:?}", PyValue::Cell(cell));
        assert!(cell_debug.contains("42"));
    }
}
