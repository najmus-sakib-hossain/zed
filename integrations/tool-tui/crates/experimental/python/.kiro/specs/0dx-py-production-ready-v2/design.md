
# Design Document: DX-Py Production Ready

## Overview

This design document outlines the technical approach for making DX-Py a production-ready Python toolchain. The implementation focuses on completing the core runtime features (string/list/dict methods, comprehensions, exceptions, classes), fixing the test runner and package manager, and ensuring honest benchmark reporting.

## Architecture

The DX-Py runtime follows a tiered architecture: @tree[]

### Key Design Decisions

- Method Dispatch via PyValue: Type methods (str.upper, list.append) will be implemented as method lookup on PyValue variants, not as separate builtin functions.
- Attribute Resolution Chain: `instance.attr` → instance dict → class dict → parent class dict → AttributeError
- Exception Propagation: Exceptions will use Rust's Result type internally, with InterpreterError variants for each Python exception type.
- Worker Process Communication: Test runner will use JSON-over-stdio for reliable cross-process communication.

## Components and Interfaces

### Component 1: String Methods Module

Location: `runtime/dx-py-core/src/pystr_methods.rs`
```rust
/// String method implementations pub struct StrMethods;
impl StrMethods { /// str.upper() -> str pub fn upper(s: &str) -> String;
/// str.lower() -> str pub fn lower(s: &str) -> String;
/// str.split(sep: Option<&str>) -> Vec<String> pub fn split(s: &str, sep: Option<&str>) -> Vec<String>;
/// str.replace(old: &str, new: &str) -> String pub fn replace(s: &str, old: &str, new: &str) -> String;
/// str.join(iterable: &[PyValue]) -> Result<String> pub fn join(sep: &str, items: &[PyValue]) -> RuntimeResult<String>;
/// str.strip() -> str pub fn strip(s: &str) -> String;
/// str.startswith(prefix: &str) -> bool pub fn startswith(s: &str, prefix: &str) -> bool;
/// str.endswith(suffix: &str) -> bool pub fn endswith(s: &str, suffix: &str) -> bool;
/// str.find(sub: &str) -> i64 pub fn find(s: &str, sub: &str) -> i64;
}
```

### Component 2: List Methods Module

Location: `runtime/dx-py-core/src/pylist_methods.rs`
```rust
/// List method implementations impl PyList { /// list.append(item) - mutates list, returns None pub fn append(&self, item: PyValue);
/// list.extend(iterable) - mutates list pub fn extend(&self, items: &[PyValue]);
/// list.insert(index, item) - mutates list pub fn insert(&self, index: i64, item: PyValue);
/// list.remove(item) - mutates list, raises ValueError if not found pub fn remove(&self, item: &PyValue) -> RuntimeResult<()>;
/// list.pop(index?) - removes and returns item pub fn pop(&self, index: Option<i64>) -> RuntimeResult<PyValue>;
/// list.sort() - sorts in place pub fn sort(&self) -> RuntimeResult<()>;
/// list.reverse() - reverses in place pub fn reverse(&self);
/// list.index(item) -> int pub fn index_of(&self, item: &PyValue) -> RuntimeResult<i64>;
/// list.count(item) -> int pub fn count(&self, item: &PyValue) -> i64;
}
```

### Component 3: Dict Methods Module

Location: `runtime/dx-py-core/src/pydict_methods.rs`
```rust
/// Dict method implementations impl PyDict { /// dict.keys() -> list of keys pub fn keys_list(&self) -> PyValue;
/// dict.values() -> list of values pub fn values_list(&self) -> PyValue;
/// dict.items() -> list of (key, value) tuples pub fn items_list(&self) -> PyValue;
/// dict.get(key, default=None) -> value pub fn get_with_default(&self, key: &PyKey, default: PyValue) -> PyValue;
/// dict.pop(key, default?) -> value pub fn pop(&self, key: &PyKey, default: Option<PyValue>) -> RuntimeResult<PyValue>;
/// dict.update(other) - merges other into self pub fn update(&self, other: &PyDict);
/// dict.clear() - removes all items pub fn clear(&self);
}
```

### Component 4: Method Dispatch in Interpreter

Location: `runtime/dx-py-interpreter/src/dispatcher.rs` The dispatcher needs to handle `LOAD_ATTR` and `CALL_METHOD` opcodes for type methods:
```rust
/// Handle attribute access on PyValue fn load_attr(&self, obj: &PyValue, attr: &str) -> InterpreterResult<PyValue> { match obj { PyValue::Str(s) => self.get_str_method(s, attr), PyValue::List(l) => self.get_list_method(l, attr), PyValue::Dict(d) => self.get_dict_method(d, attr), PyValue::Instance(inst) => self.get_instance_attr(inst, attr), _ => Err(InterpreterError::AttributeError(...))
}
}
/// Get a bound method for string operations fn get_str_method(&self, s: &Arc<str>, attr: &str) -> InterpreterResult<PyValue> { match attr { "upper" | "lower" | "split" | "replace" | "join" |
"strip" | "startswith" | "endswith" | "find" => { Ok(PyValue::BoundMethod(BoundMethod::String { value: Arc::clone(s), method: attr.to_string()
}))
}
_ => Err(InterpreterError::AttributeError(...))
}
}
```

### Component 5: List Comprehension Compilation

Location: `runtime/dx-py-compiler/src/compiler.rs` List comprehensions need proper bytecode generation:
```rust
/// Compile [expr for x in iterable if condition]
fn compile_list_comp(&mut self, elt: &Expr, generators: &[Comprehension]) -> CompileResult<()> { // 1. Create empty list: BUILD_LIST 0 self.emit(Opcode::BuildList, 0);
// 2. For each generator:
for gen in generators { // Evaluate iterable self.compile_expr(&gen.iter)?;
// Get iterator self.emit(Opcode::GetIter, 0);
// Loop start let loop_start = self.current_offset();
self.emit(Opcode::ForIter, 0); // placeholder let loop_end_patch = self.current_offset() - 2;
// Store loop variable self.compile_store(&gen.target)?;
// Compile conditions for cond in &gen.ifs { self.compile_expr(cond)?;
self.emit(Opcode::PopJumpIfFalse, loop_start as u16);
}
// Compile element expression self.compile_expr(elt)?;
// Append to list self.emit(Opcode::ListAppend, 1);
// Jump back to loop start self.emit(Opcode::JumpAbsolute, loop_start as u16);
// Patch loop end self.patch_jump(loop_end_patch);
}
Ok(())
}
```

### Component 6: Exception Handling

Location: `runtime/dx-py-interpreter/src/exception_handler.rs`
```rust
/// Exception handling state pub struct ExceptionHandler { /// Stack of active try blocks try_blocks: Vec<TryBlock>, /// Current exception being handled current_exception: Option<PyException>, }
pub struct TryBlock { /// Bytecode offset of except handler except_offset: usize, /// Bytecode offset of finally handler (if any)
finally_offset: Option<usize>, /// Stack depth when entering try block stack_depth: usize, }
impl ExceptionHandler { /// Enter a try block pub fn push_try(&mut self, except_offset: usize, finally_offset: Option<usize>, stack_depth: usize);
/// Handle an exception - returns offset to jump to pub fn handle_exception(&mut self, exc: PyException) -> Option<usize>;
/// Execute finally block pub fn run_finally(&mut self) -> Option<usize>;
}
```

### Component 7: Class System

Location: `runtime/dx-py-core/src/types.rs`
```rust
/// Enhanced PyType for class definitions pub struct PyType { pub name: String, pub bases: Vec<Arc<PyType>>, pub mro: Vec<Arc<PyType>>, // Method Resolution Order pub dict: HashMap<String, PyValue>, pub slots: TypeSlots, }
/// Type slots for special methods pub struct TypeSlots { pub tp_init: Option<Arc<PyFunction>>, pub tp_new: Option<Arc<PyFunction>>, pub tp_call: Option<Arc<PyFunction>>, pub tp_getattr: Option<Arc<PyFunction>>, pub tp_setattr: Option<Arc<PyFunction>>, }
/// Instance of a class pub struct PyInstance { pub class: Arc<PyType>, pub dict: HashMap<String, PyValue>, }
impl PyInstance { /// Get attribute with proper MRO lookup pub fn getattr(&self, name: &str) -> RuntimeResult<PyValue> { // 1. Check instance dict if let Some(value) = self.dict.get(name) { return Ok(value.clone());
}
// 2. Check class and MRO for cls in &self.class.mro { if let Some(value) = cls.dict.get(name) { // If it's a function, return bound method if let PyValue::Function(f) = value { return Ok(PyValue::BoundMethod(BoundMethod::Instance { instance: Arc::new(self.clone()), method: Arc::clone(f), }));
}
return Ok(value.clone());
}
}
Err(RuntimeError::attribute_error(format!( "'{}' object has no attribute '{}'", self.class.name, name )))
}
}
```

### Component 8: JSON Module Implementation

Location: `runtime/dx-py-core/src/stdlib/json.rs`
```rust
/// JSON module implementation pub struct JsonModule;
impl JsonModule { /// json.dumps(obj) -> str pub fn dumps(obj: &PyValue) -> RuntimeResult<String> { match obj { PyValue::None => Ok("null".to_string()), PyValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()), PyValue::Int(i) => Ok(i.to_string()), PyValue::Float(f) => Ok(f.to_string()), PyValue::Str(s) => Ok(format!("\"{}\"", escape_json_string(s))), PyValue::List(l) => { let items: RuntimeResult<Vec<String>> = l.to_vec().iter().map(Self::dumps).collect();
Ok(format!("[{}]", items?.join(", ")))
}
PyValue::Dict(d) => { let items: RuntimeResult<Vec<String>> = d.items().iter()
.map(|(k, v)| { let key_str = match k { PyKey::Str(s) => format!("\"{}\"", escape_json_string(s)), PyKey::Int(i) => format!("\"{}\"", i), _ => return Err(RuntimeError::type_error(...))
};
Ok(format!("{}: {}", key_str, Self::dumps(v)?))
})
.collect();
Ok(format!("{{{}}}", items?.join(", ")))
}
_ => Err(RuntimeError::type_error("JSON serializable", obj.type_name()))
}
}
/// json.loads(s) -> obj pub fn loads(s: &str) -> RuntimeResult<PyValue> { // Use a simple recursive descent parser let mut parser = JsonParser::new(s);
parser.parse_value()
}
}
```

### Component 9: Test Runner Worker Fix

Location: `test-runner/dx-py-executor/src/worker.rs`
```rust
/// Fixed worker process communication pub struct TestWorker { child: Child, stdin: ChildStdin, stdout: BufReader<ChildStdout>, }
impl TestWorker { /// Spawn a new worker process pub fn spawn(runtime_path: &Path) -> Result<Self> { let mut child = Command::new(runtime_path)
.arg("--worker-mode")
.stdin(Stdio::piped())
.stdout(Stdio::piped())
.stderr(Stdio::piped())
.spawn()?;
let stdin = child.stdin.take().expect("stdin");
let stdout = BufReader::new(child.stdout.take().expect("stdout"));
Ok(Self { child, stdin, stdout })
}
/// Run a test and get result pub fn run_test(&mut self, test: &TestCase) -> TestResult { // Send test request as JSON let request = serde_json::to_string(&TestRequest { file: test.file.clone(), function: test.name.clone(), }).unwrap();
writeln!(self.stdin, "{}", request).ok();
self.stdin.flush().ok();
// Read response with timeout let mut response = String::new();
match self.stdout.read_line(&mut response) { Ok(0) => TestResult::Error("Worker closed stdout".into()), Ok(_) => serde_json::from_str(&response)
.unwrap_or(TestResult::Error("Invalid response".into())), Err(e) => TestResult::Error(format!("Read error: {}", e)), }
}
}
```

### Component 10: Package Manager Add Fix

Location: `package-manager/dx-py-cli/src/commands/add.rs`
```rust
/// Fixed add command implementation pub fn execute_add(package: &str, dev: bool) -> Result<()> { // 1. Read existing pyproject.toml let content = fs::read_to_string("pyproject.toml")?;
let mut doc = content.parse::<toml_edit::Document>()?;
// 2. Determine target section let section = if dev { &mut doc["project"]["optional-dependencies"]["dev"]
} else { &mut doc["project"]["dependencies"]
};
// 3. Ensure section is an array if section.is_none() { *section = toml_edit::value(toml_edit::Array::new());
}
// 4. Add package if not already present let array = section.as_array_mut().expect("dependencies is array");
let package_base = package.split(['=', '<', '>', '!']).next().unwrap();
// Check if already present let already_present = array.iter()
.any(|item| item.as_str()
.map(|s| s.starts_with(package_base))
.unwrap_or(false));
if !already_present { array.push(package);
}
// 5. Write back fs::write("pyproject.toml", doc.to_string())?;
println!("Added {} to dependencies", package);
Ok(())
}
```

## Data Models

### PyValue Enum Extension

```rust
pub enum PyValue { // Existing variants...
None, Bool(bool), Int(i64), Float(f64), Str(Arc<str>), List(Arc<PyList>), Dict(Arc<PyDict>), Tuple(Arc<PyTuple>), // Enhanced variants for method binding BoundMethod(BoundMethod), // Class system Type(Arc<PyType>), Instance(Arc<PyInstance>), }
pub enum BoundMethod { /// Method bound to a string value String { value: Arc<str>, method: String }, /// Method bound to a list List { value: Arc<PyList>, method: String }, /// Method bound to a dict Dict { value: Arc<PyDict>, method: String }, /// Method bound to an instance Instance { instance: Arc<PyInstance>, method: Arc<PyFunction> }, }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: String Method Round-Trip Consistency

For any string `s` and separator `sep`, calling `sep.join(s.split(sep))` should produce a string equivalent to `s` (when `s` does not start or end with `sep` and has no consecutive `sep` occurrences). Validates: Requirements 1.3, 1.4, 1.6

### Property 2: String Case Conversion Idempotence

For any string `s`, calling `s.upper().upper()` should equal `s.upper()`, and `s.lower().lower()` should equal `s.lower()`. Validates: Requirements 1.1, 1.2

### Property 3: String Search Consistency

For any string `s` and substring `sub`, if `s.find(sub)` returns index `i >= 0`, then `s[i:i+len(sub)]` should equal `sub`. If `s.startswith(sub)` is True, then `s.find(sub)` should return 0. Validates: Requirements 1.8, 1.9, 1.10

### Property 4: List Append Invariant

For any list `lst` and item `x`, after calling `lst.append(x)`, the length of `lst` should increase by 1 and `lst[-1]` should equal `x`. Validates: Requirements 2.1

### Property 5: List Sort Ordering

For any list `lst` of comparable elements, after calling `lst.sort()`, for all valid indices `i`, `lst[i] <= lst[i+1]` should hold. Validates: Requirements 2.7

### Property 6: List Reverse Round-Trip

For any list `lst`, calling `lst.reverse()` twice should restore the original order. Validates: Requirements 2.8

### Property 7: List Pop Consistency

For any non-empty list `lst`, if `last = lst[-1]` before calling `pop()`, then `lst.pop()` should return `last` and the length should decrease by 1. Validates: Requirements 2.5, 2.6

### Property 8: Dict Keys-Values-Items Consistency

For any dict `d`, `len(d.keys())` should equal `len(d.values())` should equal `len(d.items())` should equal `len(d)`. Validates: Requirements 3.1, 3.2, 3.3

### Property 9: Dict Get Consistency

For any dict `d` and key `k`, if `k in d` then `d.get(k)` should equal `d[k]`, otherwise `d.get(k)` should be None and `d.get(k, default)` should equal `default`. Validates: Requirements 3.4, 3.5

### Property 10: Dict Update Merge

For any dicts `d1` and `d2`, after `d1.update(d2)`, for all keys `k` in `d2`, `d1[k]` should equal `d2[k]`. Validates: Requirements 3.7

### Property 11: List Comprehension Length

For any iterable of length `n`, the comprehension `[x for x in iterable]` should produce a list of length `n`. Validates: Requirements 4.1

### Property 12: List Comprehension Filter

For any list comprehension `[x for x in iterable if cond(x)]`, all elements in the result should satisfy `cond(x) == True`. Validates: Requirements 4.2

### Property 13: Exception Finally Guarantee

For any code with a try/finally block, the finally block should execute regardless of whether an exception was raised, caught, or propagated. Validates: Requirements 5.4

### Property 14: Exception Type Matching

For any exception of type `E` raised in a try block, an `except E` handler should catch it, and an `except OtherType` handler (where OtherType is not a parent of E) should not catch it. Validates: Requirements 5.1, 5.2, 5.5

### Property 15: Class Instance Attribute Access

For any class `C` with method `m` and instance `obj = C()`, calling `obj.m()` should invoke `m` with `obj` as the first argument (self). Validates: Requirements 6.3, 6.4, 6.5

### Property 16: Class Inheritance

For any class `Child(Parent)` where `Parent` has method `m`, instances of `Child` should be able to call `m` unless `Child` overrides it. Validates: Requirements 6.6

### Property 17: JSON Round-Trip

For any Python object `obj` that is JSON-serializable (None, bool, int, float, str, list, dict), `json.loads(json.dumps(obj))` should produce an equivalent object. Validates: Requirements 7.5, 7.6

### Property 18: Module Import Caching

For any module `m`, importing it multiple times should return the same module object (identity check). Validates: Requirements 7.7

## Error Handling

### Exception Types

+-------------+-------+--------+
| Exception   | When  | Raised |
+=============+=======+========+
| `TypeError` | Wrong | type   |
+-------------+-------+--------+



### Error Propagation

- Errors in Rust code are represented as `InterpreterError` variants
- When an error occurs, the interpreter unwinds the stack looking for exception handlers
- If no handler is found, the error propagates to the top level and is printed with a traceback
- The traceback includes file name, line number, and function name for each frame

### Graceful Degradation

For features not yet implemented: -Return a clear error message indicating the feature is not supported -Do not crash or hang -Log the unsupported operation for debugging

## Testing Strategy

### Unit Tests

Unit tests verify individual components in isolation: -String Methods: Test each method with various inputs including empty strings, unicode, edge cases -List Methods: Test mutations, return values, error conditions -Dict Methods: Test CRUD operations, iteration, error conditions -Comprehensions: Test simple, filtered, nested comprehensions -Exceptions: Test try/except/finally combinations -Classes: Test instantiation, method calls, inheritance

### Property-Based Tests

Property-based tests use the `proptest` crate to generate random inputs: -Minimum 100 iterations per property test -Each test references its design document property -Tag format: Feature: dx-py-production-ready-v2, Property N: description

### Integration Tests

Integration tests verify end-to-end behavior: -Runtime: Execute Python scripts and verify output matches CPython -Test Runner: Run actual test files and verify results -Package Manager: Execute add/remove commands and verify pyproject.toml changes -Benchmarks: Run benchmarks and verify output validation

### Test Organization

@tree:runtime/dx-py-core/tests[]
