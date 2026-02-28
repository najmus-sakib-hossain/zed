
# Design Document: DX-Py Production Ready

## Overview

This design document outlines the technical approach to make DX-Py a production-ready Python toolchain. The current codebase has solid Rust architecture but critical functionality gaps that prevent real-world usage. This design addresses 15 requirements across three components: Runtime, Package Manager, and Test Runner. The design prioritizes: -Correctness first - Fix broken core functionality before adding features -Incremental progress - Each fix should be independently testable -Minimal changes - Leverage existing architecture where possible

## Architecture

### Current Architecture Overview

@tree[]

### Key Components Requiring Changes

- dx-py-compiler
- Fix class/exception/comprehension bytecode generation
- dx-py-interpreter/dispatch.rs
- Fix bytecode execution for classes, exceptions, iterators
- dx-py-core/stdlib.rs
- Fix JSON module attribute access
- dx-py-jit/baseline.rs
- Implement actual code generation
- dx-py-package-manager/download
- Implement PyPI HTTP client
- dx-py-package-manager/installer
- Implement wheel extraction
- test-runner/crates/dx-py-fixture
- Implement fixture injection

## Components and Interfaces

### Component 1: Class System Fix

Problem: `__init__` methods fail with "expected code object, got str" Root Cause Analysis: The compiler stores method bodies as string references instead of compiling them to bytecode. When the dispatcher tries to call `__init__`, it expects a `CodeRef` but receives a string. Solution Design:
```rust
// In dx-py-compiler/src/compiler.rs impl Compiler { fn compile_class(&mut self, class_def: &ClassDef) -> CompileResult<()> { // 1. Create class namespace let class_name = &class_def.name;
// 2. Compile each method as a separate function for stmt in &class_def.body { if let Stmt::FunctionDef(method) = stmt { // Compile method body to bytecode let method_code = self.compile_function(method)?;
// Store method with proper CodeRef self.emit_make_function(method_code);
}
}
// 3. Build class object with method dict self.emit_build_class(class_name, base_classes);
Ok(())
}
}
```
Dispatcher Changes:
```rust
// In dx-py-interpreter/src/dispatch.rs fn handle_call_method(&mut self, frame: &mut PyFrame) -> InterpreterResult<PyValue> { let instance = frame.pop()?;
let method_name = self.get_name(name_idx);
// Look up method in class __dict__ let class = instance.get_class()?;
let method = class.get_method(method_name)?;
// Bind self and call let bound_method = BoundMethod::new(instance, method);
self.call_function(bound_method, args)
}
```

### Component 2: Exception Handling Fix

Problem: Exceptions raised in try blocks are not caught by except handlers. Root Cause Analysis: The dispatcher doesn't properly set up exception handlers or unwind the stack when exceptions occur. Solution Design:
```rust
// Exception handler block structure struct ExceptionHandler { try_start: usize, // Bytecode offset where try begins try_end: usize, // Bytecode offset where try ends handler_start: usize, // Bytecode offset of except handler exception_type: Option<PyValue>, // Type to catch (None = catch all)
finally_start: Option<usize>, // Optional finally block }
// In dispatch.rs - main execution loop fn execute(&self, frame: &mut PyFrame) -> InterpreterResult<PyValue> { loop { match self.dispatch_opcode(frame) { Ok(result) => { /* continue */ }
Err(exception) => { // Find matching handler if let Some(handler) = self.find_handler(frame.ip, &exception) { // Jump to handler, push exception on stack frame.ip = handler.handler_start;
frame.push(exception.to_pyvalue());
} else { // No handler, propagate return Err(exception);
}
}
}
}
}
```
New Opcodes Required: -`SETUP_EXCEPT(handler_offset)` - Push exception handler -`POP_EXCEPT` - Pop exception handler after successful try -`RERAISE` - Re-raise current exception -`SETUP_FINALLY(finally_offset)` - Push finally handler

### Component 3: List Comprehension Fix

Problem: List comprehensions fail with "iterator object is not callable" Root Cause Analysis: The compiler generates `CALL_FUNCTION` on the iterator instead of `GET_ITER` + `FOR_ITER`. Solution Design:
```rust
// Correct bytecode for [x*2 for x in range(5)]
// // BUILD_LIST 0 ; Create empty result list // LOAD_GLOBAL 'range' // LOAD_CONST 5 // CALL_FUNCTION 1 ; Call range(5)
// GET_ITER ; Get iterator from range object // FOR_ITER end ; Start iteration loop // STORE_FAST 'x' ; Store current item // LOAD_FAST 'x' // LOAD_CONST 2 // BINARY_MULTIPLY // LIST_APPEND 1 ; Append to result list // JUMP_ABSOLUTE loop // end:
// ; Result list is on stack fn compile_list_comp(&mut self, comp: &ListComp) -> CompileResult<()> { // Create result list self.emit(Opcode::BuildList, 0);
// Compile generators (for clauses)
for generator in &comp.generators { // Compile iterable self.compile_expr(&generator.iter)?;
self.emit(Opcode::GetIter, 0);
let loop_start = self.current_offset();
let loop_end_placeholder = self.emit_placeholder(Opcode::ForIter);
// Store loop variable self.compile_store(&generator.target)?;
// Compile conditions (if clauses)
for condition in &generator.ifs { self.compile_expr(condition)?;
let skip_placeholder = self.emit_placeholder(Opcode::PopJumpIfFalse);
// Will patch to continue to next iteration }
// Compile element expression and append self.compile_expr(&comp.elt)?;
self.emit(Opcode::ListAppend, 1);
// Jump back to loop start self.emit(Opcode::JumpAbsolute, loop_start);
// Patch loop end self.patch_jump(loop_end_placeholder);
}
Ok(())
}
```

### Component 4: Dict/Set Comprehensions

Design: Similar to list comprehensions but with different result container opcodes.
```rust
// {k: v for k, v in items}
fn compile_dict_comp(&mut self, comp: &DictComp) -> CompileResult<()> { self.emit(Opcode::BuildMap, 0);
// Same iteration structure as list comp for generator in &comp.generators { self.compile_expr(&generator.iter)?;
self.emit(Opcode::GetIter, 0);
let loop_start = self.current_offset();
let loop_end = self.emit_placeholder(Opcode::ForIter);
self.compile_store(&generator.target)?;
// Compile key and value self.compile_expr(&comp.key)?;
self.compile_expr(&comp.value)?;
self.emit(Opcode::MapAdd, 1); // Different from ListAppend self.emit(Opcode::JumpAbsolute, loop_start);
self.patch_jump(loop_end);
}
Ok(())
}
```

### Component 5: JSON Module Fix

Problem: `json.dumps` is not accessible despite json module loading. Root Cause Analysis: The `json_builtins_expanded()` function exists but the module dict isn't populated correctly during import. Solution Design:
```rust
// In dx-py-core/src/stdlib.rs pub fn json_builtins_expanded() -> Vec<PyBuiltinFunction> { vec![ PyBuiltinFunction::new("dumps", json_dumps), PyBuiltinFunction::new("loads", json_loads), PyBuiltinFunction::new("dump", json_dump), PyBuiltinFunction::new("load", json_load), ]
}
fn json_dumps(args: &[PyValue]) -> Result<PyValue, String> { let obj = args.get(0).ok_or("dumps requires 1 argument")?;
let indent = args.get(1); // Optional indent parameter let json_str = pyvalue_to_json(obj, indent)?;
Ok(PyValue::Str(Arc::from(json_str)))
}
fn pyvalue_to_json(value: &PyValue, indent: Option<&PyValue>) -> Result<String, String> { match value { PyValue::None => Ok("null".to_string()), PyValue::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()), PyValue::Int(i) => Ok(i.to_string()), PyValue::Float(f) => Ok(f.to_string()), PyValue::Str(s) => Ok(format!("\"{}\"", escape_json_string(s))), PyValue::List(list) => { let items: Result<Vec<_>, _> = list.iter()
.map(|v| pyvalue_to_json(&v, None))
.collect();
Ok(format!("[{}]", items?.join(",")))
}
PyValue::Dict(dict) => { let pairs: Result<Vec<_>, _> = dict.iter()
.map(|(k, v)| { let key_str = match k { PyKey::Str(s) => format!("\"{}\"", s), _ => return Err("JSON keys must be strings"), };
let val_str = pyvalue_to_json(&v, None)?;
Ok(format!("{}:{}", key_str, val_str))
})
.collect();
Ok(format!("{{{}}}", pairs?.join(",")))
}
_ => Err(format!("Object of type {} is not JSON serializable", value.type_name())), }
}
```

### Component 6: Generator Implementation

Design: Generators require coroutine-style execution with suspend/resume.
```rust
// Generator state machine enum GeneratorState { Created, // Not yet started Running, // Currently executing Suspended(usize), // Suspended at bytecode offset Completed, // Finished (raised StopIteration)
}
struct PyGenerator { frame: PyFrame, // Suspended frame state state: GeneratorState, yielded_value: Option<PyValue>, }
impl PyGenerator { fn send(&mut self, value: PyValue) -> Result<PyValue, StopIteration> { match self.state { GeneratorState::Created => { self.state = GeneratorState::Running;
self.resume()
}
GeneratorState::Suspended(offset) => { self.frame.ip = offset;
self.frame.push(value);
self.state = GeneratorState::Running;
self.resume()
}
GeneratorState::Completed => Err(StopIteration), GeneratorState::Running => Err(ValueError("generator already running")), }
}
fn resume(&mut self) -> Result<PyValue, StopIteration> { // Execute until YIELD_VALUE or RETURN_VALUE loop { match self.execute_one_opcode() { OpcodeResult::Continue => continue, OpcodeResult::Yield(value) => { self.state = GeneratorState::Suspended(self.frame.ip);
return Ok(value);
}
OpcodeResult::Return(_) => { self.state = GeneratorState::Completed;
return Err(StopIteration);
}
OpcodeResult::Error(e) => { self.state = GeneratorState::Completed;
return Err(e);
}
}
}
}
}
```

### Component 7: JIT Implementation

Problem: `compile_baseline()` and `compile_optimized()` return `None`. Solution Design: Implement actual Cranelift code generation.
```rust
// In dx-py-jit/src/baseline.rs impl BaselineCompiler { pub fn compile(&mut self, func_id: FunctionId, code: &CodeObject) -> Result<*const u8, JitError> { let mut ctx = self.module.make_context();
let mut builder_ctx = FunctionBuilderContext::new();
// Create function signature let sig = self.create_signature(code);
ctx.func.signature = sig;
let mut builder = FunctionBuilder::new(&mut ctx.func, &mut builder_ctx);
// Create entry block let entry_block = builder.create_block();
builder.switch_to_block(entry_block);
builder.seal_block(entry_block);
// Translate bytecode to Cranelift IR let mut ip = 0;
while ip < code.code.len() { let opcode = DpbOpcode::from_u8(code.code[ip]);
match opcode { DpbOpcode::LoadConst => { let idx = read_u16(&code.code[ip+1..]);
let const_val = &code.constants[idx as usize];
let ir_val = self.const_to_ir(&mut builder, const_val);
self.push_value(ir_val);
ip += 3;
}
DpbOpcode::BinaryAdd => { let b = self.pop_value();
let a = self.pop_value();
let result = builder.ins().iadd(a, b);
self.push_value(result);
ip += 1;
}
DpbOpcode::Return => { let ret_val = self.pop_value();
builder.ins().return_(&[ret_val]);
ip += 1;
}
// ... other opcodes _ => return Err(JitError::UnsupportedOpcode(opcode)), }
}
builder.finalize();
// Compile to machine code let code_id = self.module.declare_function( &format!("func_{}", func_id.0), Linkage::Local, &ctx.func.signature, )?;
self.module.define_function(code_id, &mut ctx)?;
self.module.finalize_definitions()?;
let code_ptr = self.module.get_finalized_function(code_id);
Ok(code_ptr)
}
}
```

### Component 8: PyPI Download Implementation

Design: HTTP client for PyPI JSON API and package downloads.
```rust
// In dx-py-package-manager/src/download/pypi.rs pub struct PyPiDownloader { client: reqwest::Client, cache_dir: PathBuf, }
impl PyPiDownloader { pub async fn download_package(&self, name: &str, version: &str) -> Result<PathBuf> { // 1. Query PyPI JSON API let url = format!("https://pypi.org/pypi/{}/{}/json", name, version);
let metadata: PyPiPackageMetadata = self.client.get(&url).send().await?.json().await?;
// 2. Find best wheel for platform let wheel = self.select_wheel(&metadata.urls)?;
// 3. Download wheel let wheel_path = self.cache_dir.join(&wheel.filename);
if !wheel_path.exists() { let bytes = self.client.get(&wheel.url).send().await?.bytes().await?;
// 4. Verify SHA256 let hash = compute_sha256(&bytes);
if hash != wheel.digests.sha256 { return Err(Error::HashMismatch);
}
std::fs::write(&wheel_path, bytes)?;
}
Ok(wheel_path)
}
fn select_wheel(&self, urls: &[ReleaseFile]) -> Result<&ReleaseFile> { // Prefer wheels over sdist // Match platform tags (cp312-cp312-win_amd64, etc.)
let platform = Platform::current();
urls.iter()
.filter(|f| f.packagetype == "bdist_wheel")
.filter(|f| platform.matches_wheel_tag(&f.filename))
.max_by_key(|f| f.upload_time)
.ok_or(Error::NoCompatibleWheel)
}
}
```

### Component 9: Wheel Installation

Design: Extract wheel contents to site-packages.
```rust
// In dx-py-package-manager/src/installer/wheel.rs pub struct WheelInstaller { site_packages: PathBuf, }
impl WheelInstaller { pub fn install(&self, wheel_path: &Path) -> Result<InstalledPackage> { let wheel = WheelFile::open(wheel_path)?;
let metadata = wheel.parse_metadata()?;
// Create package directory let pkg_dir = self.site_packages.join(&metadata.name);
std::fs::create_dir_all(&pkg_dir)?;
// Extract files let mut record = Vec::new();
for entry in wheel.entries()? { let dest = self.compute_destination(&entry)?;
std::fs::create_dir_all(dest.parent().unwrap())?;
let mut file = std::fs::File::create(&dest)?;
std::io::copy(&mut entry.reader()?, &mut file)?;
record.push(RecordEntry { path: dest.clone(), hash: compute_sha256_file(&dest)?, size: std::fs::metadata(&dest)?.len(), });
}
// Create .dist-info directory let dist_info = self.site_packages.join(format!( "{}-{}.dist-info", metadata.name, metadata.version ));
std::fs::create_dir_all(&dist_info)?;
// Write RECORD file self.write_record(&dist_info.join("RECORD"), &record)?;
// Write METADATA std::fs::write(dist_info.join("METADATA"), &metadata.raw)?;
// Create entry point scripts if let Some(scripts) = &metadata.entry_points { self.create_scripts(scripts)?;
}
Ok(InstalledPackage { name: metadata.name, version: metadata.version, location: pkg_dir, })
}
}
```

### Component 10: Virtual Environment Support

```rust
// In project-manager/src/venv.rs pub struct VenvManager { base_path: PathBuf, }
impl VenvManager { pub fn create(&self, python_version: &str) -> Result<VirtualEnv> { let venv_path = self.base_path.join(".venv");
std::fs::create_dir_all(&venv_path)?;
// Create directory structure let bin_dir = if cfg!(windows) { "Scripts" } else { "bin" };
std::fs::create_dir_all(venv_path.join(bin_dir))?;
std::fs::create_dir_all(venv_path.join("lib").join(format!("python{}", python_version)).join("site-packages"))?;
// Write pyvenv.cfg let cfg = format!( "home = {}\ninclude-system-site-packages = false\nversion = {}\n", std::env::current_exe()?.parent().unwrap().display(), python_version );
std::fs::write(venv_path.join("pyvenv.cfg"), cfg)?;
// Create activation scripts self.create_activation_scripts(&venv_path)?;
Ok(VirtualEnv { path: venv_path })
}
}
```

### Component 11: Fixture Support

```rust
// In test-runner/crates/dx-py-fixture/src/lib.rs pub struct FixtureManager { fixtures: HashMap<String, FixtureDefinition>, scopes: HashMap<FixtureScope, HashMap<String, PyValue>>, }
impl FixtureManager { pub fn resolve_fixtures(&mut self, test_fn: &TestFunction) -> Result<Vec<PyValue>> { let mut args = Vec::new();
for param in &test_fn.parameters { if let Some(fixture) = self.fixtures.get(&param.name) { let value = self.get_or_create_fixture(fixture)?;
args.push(value);
}
}
Ok(args)
}
fn get_or_create_fixture(&mut self, fixture: &FixtureDefinition) -> Result<PyValue> { // Check if already created for this scope if let Some(cached) = self.scopes.get(&fixture.scope).and_then(|s| s.get(&fixture.name)) {
return Ok(cached.clone());
}
// Resolve fixture dependencies let deps = self.resolve_fixture_deps(fixture)?;
// Call fixture function let value = self.call_fixture_fn(fixture, &deps)?;
// Cache by scope self.scopes .entry(fixture.scope)
.or_default()
.insert(fixture.name.clone(), value.clone());
Ok(value)
}
}
```

### Component 12: Parametrized Tests

```rust
// In test-runner/crates/dx-py-discovery/src/parametrize.rs pub fn expand_parametrized_test(test: &TestFunction) -> Vec<TestCase> { let mut cases = vec![TestCase::new(test, vec![], "")];
for decorator in &test.decorators { if decorator.name == "pytest.mark.parametrize" { let param_name = &decorator.args[0];
let param_values: Vec<PyValue> = parse_param_values(&decorator.args[1]);
// Expand: each existing case × each param value cases = cases.into_iter()
.flat_map(|case| { param_values.iter().enumerate().map(|(i, val)| { let mut new_case = case.clone();
new_case.params.push((param_name.clone(), val.clone()));
new_case.id = format!("{}[{}]", case.id, i);
new_case }).collect::<Vec<_>>()
})
.collect();
}
}
cases }
```

### Component 13: Async/Await Support

```rust
// Coroutine implementation (similar to generators)
struct PyCoroutine { frame: PyFrame, state: CoroutineState, awaiting: Option<Box<dyn Future<Output = PyValue>>>, }
// Event loop integration pub struct AsyncioRuntime { tasks: VecDeque<PyCoroutine>, ready: VecDeque<PyCoroutine>, }
impl AsyncioRuntime { pub fn run_until_complete(&mut self, coro: PyCoroutine) -> Result<PyValue> { self.tasks.push_back(coro);
while !self.tasks.is_empty() || !self.ready.is_empty() { // Process ready tasks while let Some(mut task) = self.ready.pop_front() { match task.resume() { CoroutineResult::Yield(awaitable) => { task.awaiting = Some(awaitable);
self.tasks.push_back(task);
}
CoroutineResult::Return(value) => return Ok(value), CoroutineResult::Error(e) => return Err(e), }
}
// Poll waiting tasks for task in self.tasks.drain(..).collect::<Vec<_>>() { if task.awaiting.as_ref().map(|a| a.is_ready()).unwrap_or(true) { self.ready.push_back(task);
} else { self.tasks.push_back(task);
}
}
}
Err(Error::NoTasksToRun)
}
}
```

### Component 14: Standard Library Modules

Design approach: Implement as Rust builtins with Python-compatible interfaces.
```rust
// os.path module pub fn os_path_builtins() -> Vec<PyBuiltinFunction> { vec![ PyBuiltinFunction::new("join", |args| {
let parts: Vec<&str> = args.iter()
.map(|a| a.as_str())
.collect::<Result<_, _>>()?;
Ok(PyValue::Str(Arc::from(PathBuf::from_iter(parts).to_string_lossy().as_ref())))
}), PyBuiltinFunction::new("exists", |args| { let path = args[0].as_str()?;
Ok(PyValue::Bool(Path::new(path).exists()))
}), // ... dirname, basename, etc.
]
}
```

### Component 15: CLI Expression Improvements

```rust
// In dx-py-cli/src/main.rs fn execute_command(code: &str) -> Result<()> { // Support semicolon-separated statements let statements: Vec<&str> = code.split(';')
.map(|s| s.trim())
.filter(|s| !s.is_empty())
.collect();
let vm = VirtualMachine::new();
for stmt in statements { // Parse and compile each statement let ast = parse_statement(stmt)?;
let bytecode = compile_statement(&ast)?;
vm.execute_bytecode(bytecode)?;
}
Ok(())
}
```

## Data Models

### PyValue Enum (existing, needs extension)

```rust
pub enum PyValue { None, Bool(bool), Int(i64), Float(f64), Str(Arc<str>), Bytes(Arc<[u8]>), List(Arc<PyList>), Tuple(Arc<PyTuple>), Dict(Arc<PyDict>), Set(Arc<PySet>), // NEW Function(Arc<PyFunction>), Builtin(Arc<PyBuiltinFunction>), Class(Arc<PyClass>), // NEW - proper class object Instance(Arc<PyInstance>), // NEW - class instance Generator(Arc<PyGenerator>), // NEW Coroutine(Arc<PyCoroutine>), // NEW Module(Arc<PyModule>), Iterator(Arc<dyn PyIterator>), // NEW - iterator protocol }
```

### PyClass Structure (new)

```rust
pub struct PyClass { pub name: String, pub bases: Vec<Arc<PyClass>>, pub mro: Vec<Arc<PyClass>>, // Method Resolution Order pub dict: PyDict, // Class attributes and methods pub metaclass: Option<Arc<PyClass>>, }
impl PyClass { pub fn get_method(&self, name: &str) -> Option<&PyFunction> { // Follow MRO for method lookup for cls in &self.mro { if let Some(method) = cls.dict.get(name) { if let PyValue::Function(f) = method { return Some(f);
}
}
}
None }
pub fn compute_mro(bases: &[Arc<PyClass>]) -> Vec<Arc<PyClass>> { // C3 linearization algorithm c3_merge(bases)
}
}
```

### PyInstance Structure (new)

```rust
pub struct PyInstance { pub class: Arc<PyClass>, pub dict: PyDict, // Instance attributes }
impl PyInstance { pub fn get_attr(&self, name: &str) -> Option<PyValue> { // 1. Check instance dict if let Some(val) = self.dict.get(name) { return Some(val);
}
// 2. Check class (follows MRO)
self.class.get_method(name).map(|m| PyValue::Function(m.clone()))
}
pub fn set_attr(&mut self, name: &str, value: PyValue) { self.dict.set(name, value);
}
}
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Class Method Compilation Produces Valid Bytecode

For any class definition with methods, the compiler SHALL produce bytecode where each method body is a valid `CodeRef` (not a string reference). Validates: Requirements 1.1

### Property 2: Class Instantiation Correctly Binds Arguments

For any class with an `__init__` method that stores its arguments as instance attributes, instantiating the class with arguments SHALL result in instance attributes matching the passed arguments. Validates: Requirements 1.2, 1.3

### Property 3: Method Resolution Order Follows C3 Linearization

For any class hierarchy, the computed MRO SHALL match Python's C3 linearization algorithm output. Validates: Requirements 1.4, 1.5

### Property 4: Exception Handler Selection Is Correct

For any try/except block with multiple handlers, when an exception is raised, the Runtime SHALL select the first handler whose exception type matches (via isinstance check). Validates: Requirements 2.1, 2.2

### Property 5: Finally Blocks Always Execute

For any try/finally block, the finally block SHALL execute regardless of whether an exception was raised, caught, or propagated. Validates: Requirements 2.3, 2.7

### Property 6: Exception Propagation Preserves Stack Semantics

For any nested function call where an exception is raised and not caught, the exception SHALL propagate to the caller's exception handler or terminate the program. Validates: Requirements 2.4, 2.5, 2.6

### Property 7: List Comprehension Equivalence

For any list comprehension `[expr for x in iterable if cond]`, the result SHALL be equivalent to:
```python
result = []
for x in iterable:
if cond:
result.append(expr)
```
Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5

### Property 8: Dict Comprehension Equivalence

For any dict comprehension `{k: v for x in iterable if cond}`, the result SHALL be equivalent to building a dict with a for loop, where duplicate keys keep the last value. Validates: Requirements 4.1, 4.3, 4.5

### Property 9: Set Comprehension Equivalence

For any set comprehension `{expr for x in iterable if cond}`, the result SHALL be equivalent to building a set with a for loop (with automatic deduplication). Validates: Requirements 4.2, 4.4

### Property 10: JSON Round-Trip Consistency

For any JSON-serializable Python object (dict, list, str, int, float, bool, None), `json.loads(json.dumps(obj))` SHALL produce a value equal to the original object. Validates: Requirements 5.2, 5.3, 5.4

### Property 11: Generator Iteration Equivalence

For any generator expression or generator function, iterating with `next()` or a for loop SHALL yield values in the same order as the equivalent list comprehension or function with return statements. Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.6

### Property 12: Yield From Delegation

For any generator using `yield from sub_iter`, the yielded values SHALL be exactly the values from `sub_iter` in order. Validates: Requirements 6.5

### Property 13: JIT Compilation Threshold

For any function called at least 100 times, the JIT compiler SHALL attempt compilation (success or recorded failure). Validates: Requirements 7.1

### Property 14: JIT Semantic Equivalence

For any function that compiles successfully, executing the JIT-compiled code SHALL produce the same result as interpreting the bytecode for all valid inputs. Validates: Requirements 7.2, 7.3, 7.4, 7.5, 7.6, 7.7

### Property 15: Package Download Hash Verification

For any downloaded package, the computed SHA256 hash SHALL match the hash provided by PyPI metadata. Validates: Requirements 8.2

### Property 16: Dependency Resolution Completeness

For any package with dependencies, the resolver SHALL include all transitive dependencies in the resolution, selecting the highest compatible version for each. Validates: Requirements 8.3, 8.4

### Property 17: Wheel Installation Completeness

For any installed wheel, all files listed in the wheel's RECORD SHALL exist in the installation directory with matching hashes. Validates: Requirements 9.1, 9.2, 9.4, 9.6

### Property 18: Uninstall Completeness

For any uninstalled package, all files that were installed (per RECORD) SHALL be removed from the filesystem. Validates: Requirements 9.5

### Property 19: Virtual Environment Isolation

For any package installed in a virtual environment, importing that package SHALL only succeed when the virtual environment is active. Validates: Requirements 10.2, 10.3

### Property 20: Fixture Injection Correctness

For any test function with parameters matching fixture names, the test SHALL receive the fixture values as arguments. Validates: Requirements 11.1, 11.5

### Property 21: Fixture Scope Semantics

For any fixture with a specified scope, the fixture function SHALL be called at most once per scope instance (once per module for module scope, once per session for session scope). Validates: Requirements 11.2, 11.3

### Property 22: Fixture Teardown Execution

For any fixture using `yield`, the code after yield SHALL execute after the test completes (regardless of test outcome). Validates: Requirements 11.4

### Property 23: Parametrize Expansion

For any test with `@pytest.mark.parametrize(name, values)`, the test SHALL run exactly `len(values)` times, once per parameter value. Validates: Requirements 12.1, 12.4, 12.5

### Property 24: Parametrize Cartesian Product

For any test with multiple `@pytest.mark.parametrize` decorators, the test SHALL run `product(len(v) for v in all_values)` times. Validates: Requirements 12.2

### Property 25: Async Function Returns Coroutine

For any `async def` function, calling it SHALL return a coroutine object (not execute the body). Validates: Requirements 13.1

### Property 26: Asyncio.run Executes to Completion

For any coroutine passed to `asyncio.run()`, the coroutine SHALL execute to completion and return its result. Validates: Requirements 13.2, 13.3

### Property 27: Asyncio.gather Concurrent Execution

For any set of coroutines passed to `asyncio.gather()`, all coroutines SHALL complete and their results SHALL be returned in the same order as the input. Validates: Requirements 13.4

### Property 28: Standard Library Equivalence

For any implemented stdlib function, calling it with valid arguments SHALL produce the same result as CPython's implementation. Validates: Requirements 14.1, 14.2, 14.3, 14.4, 14.5, 14.6, 14.7

### Property 29: CLI Multi-Statement Execution

For any `-c` argument containing semicolon-separated statements, all statements SHALL execute in order with shared namespace. Validates: Requirements 15.1, 15.2, 15.4

## Error Handling

### Compilation Errors

+-------------+---------+-----------+----------+
| Error       | Type    | Condition | Response |
+=============+=========+===========+==========+
| SyntaxError | Invalid | Python    | syntax   |
+-------------+---------+-----------+----------+



### Runtime Errors

+-----------+------+-----------+----------+
| Error     | Type | Condition | Response |
+===========+======+===========+==========+
| TypeError | Type | mismatch  | in       |
+-----------+------+-----------+----------+



### JIT Errors

+-------------------+--------+-----------+-------------+
| Error             | Type   | Condition | Response    |
+===================+========+===========+=============+
| UnsupportedOpcode | Opcode | not       | implemented |
+-------------------+--------+-----------+-------------+



### Package Manager Errors

+-----------------+---------+-----------+----------+
| Error           | Type    | Condition | Response |
+=================+=========+===========+==========+
| PackageNotFound | Package | doesn't   | exist    |
+-----------------+---------+-----------+----------+



## Testing Strategy

### Dual Testing Approach

This project uses both unit tests and property-based tests: -Unit tests: Verify specific examples, edge cases, and error conditions -Property tests: Verify universal properties across randomly generated inputs Both are complementary and necessary for comprehensive coverage.

### Property-Based Testing Configuration

- Library: `proptest` for Rust components
- Minimum iterations: 100 per property test
- Shrinking: Enabled for minimal counterexamples
- Seed: Configurable for reproducibility

### Test Tagging Format

Each property test must be tagged with:
```rust
// Feature: dx-py-production-ready, Property N: [Property Title]
// Validates: Requirements X.Y, X.Z ```


### Test Organization


```
runtime/ dx-py-compiler/tests/ class_properties.rs # Properties 1-3 exception_properties.rs # Properties 4-6 comprehension_properties.rs # Properties 7-9 dx-py-core/tests/ json_properties.rs # Property 10 generator_properties.rs # Properties 11-12 dx-py-jit/tests/ jit_properties.rs # Properties 13-14 package-manager/ dx-py-package-manager/tests/ download_properties.rs # Properties 15-16 install_properties.rs # Properties 17-18 venv_properties.rs # Property 19 test-runner/ crates/dx-py-fixture/tests/ fixture_properties.rs # Properties 20-22 crates/dx-py-discovery/tests/ parametrize_properties.rs # Properties 23-24 ```

### Unit Test Focus Areas

Unit tests should focus on: -Specific examples that demonstrate correct behavior -Edge cases: empty inputs, boundary values, special characters -Error conditions: invalid inputs, resource exhaustion -Integration points: component boundaries

### Property Test Focus Areas

Property tests should focus on: -Round-trip properties: serialize/deserialize, compile/execute -Equivalence properties: JIT vs interpreter, comprehension vs loop -Invariant properties: MRO consistency, hash verification -Metamorphic properties: order independence, idempotence

### Test Execution

```bash


# Run all tests


cargo test --workspace


# Run property tests with more iterations


cargo test --workspace -- --test-threads=1 PROPTEST_CASES=1000


# Run specific property test


cargo test -p dx-py-compiler class_properties ```
