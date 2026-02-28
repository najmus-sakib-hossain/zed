
# Design Document: DX-Py VM Integration

## Overview

This design document describes how to wire together the existing DX-Py components into a fully functional Python runtime. The architecture is already excellent - the issue is that the interpreter's dispatcher doesn't properly handle function creation, class definitions, and module imports. The key insight is that most of the infrastructure exists: -`PyFunction` struct in `dx-py-core/src/pyfunction.rs` ✓ -`VirtualMachine` with `call_function()` in `dx-py-interpreter/src/vm.rs` ✓ -`Dispatcher` with opcode handlers in `dx-py-interpreter/src/dispatch.rs` ✓ -`CodeObject` in `dx-py-bytecode/src/format.rs` ✓ What's missing is the glue - the dispatcher needs to: -Handle `MAKE_FUNCTION` to create `PyFunction` objects -Handle `CALL` to invoke user-defined functions (not just builtins) -Handle `BUILD_CLASS` to create class objects -Handle `IMPORT_NAME` to load modules

## Architecture

@tree[]

## Components and Interfaces

### 1. Enhanced Dispatcher

The `Dispatcher` in `dx-py-interpreter/src/dispatch.rs` needs to handle function-related opcodes.
```rust
// In dispatch.rs - add to the dispatch() match statement Opcode::MakeFunction => { let flags = arg.ok_or_else(|| {
InterpreterError::Runtime("MAKE_FUNCTION requires argument".into())
})?;
// Pop code object from stack (as PyValue::Code)
let code_obj = frame.pop();
// Pop qualified name let qualname = frame.pop();
// Extract CodeObject from PyValue let code = match code_obj { PyValue::Code(c) => c, _ => return Err(InterpreterError::TypeError("expected code object".into())), };
// Create PyFunction let mut func = PyFunction::new( qualname.to_string(), CodeRef { bytecode_offset: 0, num_locals: code.nlocals as u16, stack_size: code.stacksize as u16, num_args: code.argcount as u8, num_kwonly_args: code.kwonlyargcount as u8, }, code.varnames.iter().take(code.argcount as usize)
.map(|name| Parameter { name: name.clone(), kind: ParameterKind::PositionalOrKeyword, default: None, annotation: None, })
.collect(), );
// Handle flags for defaults, closure, annotations if flags & 0x01 != 0 { // Has positional defaults let defaults = frame.pop();
if let PyValue::Tuple(t) = defaults { func = func.with_defaults(t.to_vec());
}
}
if flags & 0x08 != 0 { // Has closure let closure = frame.pop();
if let PyValue::Tuple(t) = closure { func = func.with_closure(t.to_vec());
}
}
frame.push(PyValue::Function(Arc::new(func)));
}
```

### 2. Function Call Enhancement

The `CALL` opcode handler needs to dispatch to user-defined functions:
```rust
Opcode::Call => { let argc = arg.ok_or_else(|| { InterpreterError::Runtime("CALL requires argument".into())
})?;
// Pop arguments let mut args = Vec::with_capacity(argc);
for _ in 0..argc { args.push(frame.pop());
}
args.reverse();
// Pop callable let callable = frame.pop();
let result = match callable { PyValue::Builtin(builtin) => { builtin.call(&args).map_err(InterpreterError::RuntimeError)?
}
PyValue::Function(func) => { // Create new frame for function execution self.call_user_function(&func, &args)?
}
PyValue::BoundMethod(method) => { // Prepend self to args let mut full_args = vec![method.instance.clone()];
full_args.extend(args);
self.call_user_function(&method.func, &full_args)?
}
PyValue::Class(class) => { // Instantiate class self.instantiate_class(&class, &args)?
}
_ => { return Err(InterpreterError::TypeError(format!( "'{}' object is not callable", callable.type_name()
)));
}
};
frame.push(result);
}
```

### 3. Class System

Add `PyClass` and `PyInstance` to `dx-py-core`:
```rust
// In dx-py-core/src/types.rs (already exists, needs enhancement)
pub struct PyClass { pub header: PyObjectHeader, pub name: String, pub bases: Vec<Arc<PyClass>>, pub mro: Vec<Arc<PyClass>>, // Method Resolution Order pub namespace: Arc<PyDict>, pub metaclass: Option<Arc<PyClass>>, }
impl PyClass { pub fn new(name: String, bases: Vec<Arc<PyClass>>, namespace: Arc<PyDict>) -> Self { let mro = Self::compute_mro(&bases);
Self { header: PyObjectHeader::new(TypeTag::Type, ObjectFlags::CALLABLE), name, bases, mro, namespace, metaclass: None, }
}
/// C3 linearization for MRO fn compute_mro(bases: &[Arc<PyClass>]) -> Vec<Arc<PyClass>> { // Simplified MRO - just flatten bases // Full C3 linearization for complex hierarchies let mut mro = Vec::new();
for base in bases { mro.push(Arc::clone(base));
mro.extend(base.mro.iter().cloned());
}
mro }
pub fn getattr(&self, name: &str) -> Option<PyValue> { // Check own namespace first if let Ok(value) = self.namespace.getitem(&PyKey::Str(Arc::from(name))) { return Some(value);
}
// Then check MRO for base in &self.mro { if let Ok(value) = base.namespace.getitem(&PyKey::Str(Arc::from(name))) { return Some(value);
}
}
None }
}
pub struct PyInstance { pub header: PyObjectHeader, pub class: Arc<PyClass>, pub dict: Arc<PyDict>, }
impl PyInstance { pub fn new(class: Arc<PyClass>) -> Self { Self { header: PyObjectHeader::new(TypeTag::Instance, ObjectFlags::empty()), class, dict: Arc::new(PyDict::new()), }
}
pub fn getattr(&self, name: &str) -> Option<PyValue> { // Check instance dict first if let Ok(value) = self.dict.getitem(&PyKey::Str(Arc::from(name))) { return Some(value);
}
// Then check class if let Some(value) = self.class.getattr(name) { // If it's a function, create bound method if let PyValue::Function(func) = &value { return Some(PyValue::BoundMethod(Arc::new(PyBoundMethod { func: Arc::clone(func), instance: PyValue::Instance(Arc::new(self.clone())), })));
}
return Some(value);
}
None }
}
```

### 4. BUILD_CLASS Handler

```rust
Opcode::BuildClass => { // Stack: func, name, *bases, **kwargs -> class // For simplicity, assume no kwargs initially let func = frame.pop(); // Class body function let name = frame.pop(); // Class name // Get bases (simplified - assume BUILD_TUPLE was called before)
let bases_tuple = frame.pop();
let bases: Vec<Arc<PyClass>> = match bases_tuple { PyValue::Tuple(t) => { t.to_vec().into_iter()
.filter_map(|v| match v { PyValue::Class(c) => Some(c), _ => None, })
.collect()
}
_ => vec![], };
// Create namespace dict let namespace = Arc::new(PyDict::new());
// Execute class body function to populate namespace if let PyValue::Function(body_func) = func { // The class body function takes __locals__ dict as argument // and populates it with class attributes/methods let mut class_frame = PyFrame::new(Arc::clone(&body_func), None);
// ... execute body_func with namespace as locals }
// Create class object let class_name = match name { PyValue::Str(s) => s.to_string(), _ => "<unknown>".to_string(), };
let class = PyClass::new(class_name, bases, namespace);
frame.push(PyValue::Class(Arc::new(class)));
}
```

### 5. Module Import System Integration

Connect the dispatcher to `dx-py-modules`:
```rust
Opcode::ImportName => { let name_idx = arg.ok_or_else(|| { InterpreterError::Runtime("IMPORT_NAME requires argument".into())
})?;
let module_name = self.names.get(name_idx).ok_or_else(|| { InterpreterError::NameError(format!("name index {} out of range", name_idx))
})?;
// Pop fromlist and level let _fromlist = frame.pop();
let _level = frame.pop();
// Use the module system to import let module = self.import_module(module_name)?;
frame.push(PyValue::Module(module));
}
Opcode::ImportFrom => { let name_idx = arg.ok_or_else(|| { InterpreterError::Runtime("IMPORT_FROM requires argument".into())
})?;
let attr_name = self.names.get(name_idx).ok_or_else(|| { InterpreterError::NameError(format!("name index {} out of range", name_idx))
})?;
// Module is on top of stack (don't pop it)
let module = frame.peek().clone();
if let PyValue::Module(m) = module { let value = m.dict.getitem(&PyKey::Str(Arc::from(attr_name.as_str())))
.map_err(|_| InterpreterError::ImportError( format!("cannot import name '{}' from '{}'", attr_name, m.name)
))?;
frame.push(value);
} else { return Err(InterpreterError::TypeError("expected module".into()));
}
}
```

### 6. PyValue Enum Extension

Add new variants to `PyValue` in `dx-py-core/src/pylist.rs`:
```rust
pub enum PyValue { // Existing variants None, Bool(bool), Int(i64), Float(f64), Str(Arc<str>), List(Arc<PyList>), Tuple(Arc<PyTuple>), Dict(Arc<PyDict>), Iterator(Arc<PyIterator>), Builtin(Arc<PyBuiltinFunction>), Exception(Arc<PyException>), // New variants needed for VM integration Function(Arc<PyFunction>), BoundMethod(Arc<PyBoundMethod>), Class(Arc<PyClass>), Instance(Arc<PyInstance>), Module(Arc<PyModule>), Code(Arc<CodeObject>), // For MAKE_FUNCTION Cell(Arc<PyCell>), // For closures }
```

## Data Models

### CodeObject (already exists in dx-py-bytecode)

```rust
pub struct CodeObject { pub name: String, pub qualname: String, pub filename: String, pub firstlineno: u32, pub argcount: u32, pub posonlyargcount: u32, pub kwonlyargcount: u32, pub nlocals: u32, pub stacksize: u32, pub flags: CodeFlags, pub code: Vec<u8>, pub constants: Vec<Constant>, pub names: Vec<String>, pub varnames: Vec<String>, pub freevars: Vec<String>, pub cellvars: Vec<String>, }
```

### PyCell (for closures)

```rust
pub struct PyCell { value: std::cell::RefCell<PyValue>, }
impl PyCell { pub fn new(value: PyValue) -> Self { Self { value: std::cell::RefCell::new(value) }
}
pub fn get(&self) -> PyValue { self.value.borrow().clone()
}
pub fn set(&self, value: PyValue) { *self.value.borrow_mut() = value;
}
}
```

### PyModule

```rust
pub struct PyModule { pub header: PyObjectHeader, pub name: Arc<str>, pub file: Option<PathBuf>, pub dict: Arc<PyDict>, pub package: Option<Arc<str>>, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system -, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Function Definition Round-Trip

For any valid function definition (with any combination of positional args, defaults, *args, **kwargs), defining the function via MAKE_FUNCTION and then calling it via CALL with appropriate arguments SHALL return the expected computed value. Validates: Requirements 1.1, 1.2, 1.3, 1.4, 1.5, 1.6

### Property 2: Closure Cell Preservation

For any closure that captures variables from an enclosing scope, the closure SHALL access the current value of the captured variable even after the enclosing function has returned. Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5

### Property 3: Class Instantiation and Method Resolution

For any class definition with methods and optional base classes, instantiating the class SHALL create an instance where attribute lookup follows the correct order: instance dict → class dict → base classes (MRO). Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6

### Property 4: Module Import Caching

For any module that is imported multiple times, the second and subsequent imports SHALL return the exact same module object (identity, not just equality). Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6

### Property 5: List Comprehension Equivalence

For any list comprehension expression, the result SHALL be equal to the result of the equivalent explicit for-loop with append operations. Validates: Requirements 5.1, 5.2, 5.3, 5.4, 5.5

### Property 6: Exception Handler Unwinding

For any try/except/finally block, when an exception is raised, the interpreter SHALL execute the matching except handler (if any) and always execute the finally block regardless of whether an exception occurred. Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5

### Property 7: Context Manager Protocol

For any context manager used in a with statement, enter SHALL be called on entry, and exit SHALL be called on exit with appropriate arguments (None for normal exit, exception info for exceptional exit). Validates: Requirements 7.1, 7.2, 7.3, 7.4, 7.5

### Property 8: Decorator Application Order

For any function or class with stacked decorators, the decorators SHALL be applied bottom-to-top, and the final result SHALL be the return value of the outermost (topmost) decorator. Validates: Requirements 8.1, 8.2, 8.3, 8.4

### Property 9: Builtin Function Correctness

For any call to a builtin function (isinstance, getattr, super, enumerate, sorted, etc.) with valid arguments, the result SHALL match CPython's behavior for the same inputs. Validates: Requirements 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8

## Error Handling

### Exception Types

+-----------+-----------+-------------+
| Condition | Exception | Type        |
+===========+===========+=============+
| Unknown   | name      | `NameError` |
+-----------+-----------+-------------+



### Traceback Generation

When an exception propagates, the interpreter should generate a traceback containing: -File name and line number for each frame -Function name for each frame -The exception type and message

## Testing Strategy

### Unit Tests

Unit tests should verify specific opcode behaviors: -MAKE_FUNCTION creates correct PyFunction -CALL invokes functions with correct argument binding -BUILD_CLASS creates correct PyClass -IMPORT_NAME loads modules correctly -Exception handling opcodes manage block stack correctly

### Property-Based Tests

Property-based tests using `proptest` should verify: -Function round-trip (define → call → result) -Closure cell preservation -Class MRO correctness -Module caching identity -Comprehension equivalence -Exception handler selection -Context manager protocol -Decorator ordering Configuration: -Minimum 100 iterations per property test -Use `proptest` crate for Rust property testing -Tag format: `Feature: dx-py-vm-integration, Property N: description`

### Integration Tests

Integration tests should verify end-to-end behavior: -Run simple Python programs (fibonacci, factorial) -Import and use standard library modules -Run pytest test files -Execute real-world package code (json, pathlib)
