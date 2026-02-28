
# Design Document: DX-JS Production Complete

## Overview

This design document outlines the architecture and implementation approach to fix all 42 identified weaknesses in dx-js and achieve production readiness. The design prioritizes: -Correctness - Match ECMAScript specification behavior -Performance - Maintain competitive speed with Bun -Maintainability - Single, well-tested code paths -Safety - No memory leaks, crashes, or undefined behavior

## Architecture

### High-Level Architecture

@tree[]

### Runtime Architecture (Complete Rewrite)

The current runtime has two incomplete implementations. We will consolidate into a single, complete architecture: @tree[]

## Components and Interfaces

### 1. Value Representation (Fix #4, #22)

Replace the current f64-only representation with a proper tagged value system:
```rust
/// Tagged value representation using NaN-boxing /// /// Layout (64 bits):
/// - Float: Standard IEEE 754 double /// - Pointer: 0x7FF8_xxxx_xxxx_xxxx (quiet NaN + 48-bit pointer)
/// - Integer: 0x7FF9_xxxx_xxxx_xxxx (32-bit signed integer)
/// - Boolean: 0x7FFA_0000_0000_000x (x = 0 or 1)
/// - Null: 0x7FFB_0000_0000_0000 /// - Undefined: 0x7FFC_0000_0000_0000 /// - Symbol: 0x7FFD_xxxx_xxxx_xxxx (symbol ID)


#[repr(transparent)]


pub struct Value(u64);
impl Value { pub fn from_f64(n: f64) -> Self;
pub fn from_i32(n: i32) -> Self;
pub fn from_bool(b: bool) -> Self;
pub fn from_string(s: GcRef<String>) -> Self;
pub fn from_object(o: GcRef<Object>) -> Self;
pub fn from_array(a: GcRef<Array>) -> Self;
pub fn is_number(&self) -> bool;
pub fn is_string(&self) -> bool;
pub fn is_object(&self) -> bool;
pub fn is_truthy(&self) -> bool;
pub fn to_string(&self, gc: &GcHeap) -> String;
pub fn to_number(&self) -> f64;
pub fn to_boolean(&self) -> bool;
}
```

### 2. Garbage Collector (Fix #2, #40)

Implement a generational mark-and-sweep garbage collector:
```rust
/// Generational garbage collector pub struct GcHeap { /// Young generation (nursery)
young: Arena, /// Old generation old: Arena, /// Remembered set (old young pointers)
remembered: HashSet<GcRef<()>>, /// Allocation threshold for minor GC young_threshold: usize, /// Allocation threshold for major GC old_threshold: usize, /// Total allocated bytes allocated: usize, }
impl GcHeap { pub fn new(config: GcConfig) -> Self;
/// Allocate a new object pub fn alloc<T: GcObject>(&mut self, value: T) -> GcRef<T>;
/// Trigger minor GC (young generation only)
pub fn minor_gc(&mut self, roots: &[Value]);
/// Trigger major GC (full collection)
pub fn major_gc(&mut self, roots: &[Value]);
/// Write barrier for old young pointers pub fn write_barrier(&mut self, old: GcRef<()>, young: GcRef<()>);
}
/// Reference to a GC-managed object


#[repr(transparent)]


pub struct GcRef<T> { ptr: NonNull<GcHeader>, _marker: PhantomData<T>, }
```

### 3. Expression Evaluator (Fix #3, #20, #21)

Implement proper operator precedence using a Pratt parser:
```rust
/// Expression evaluator with correct precedence pub struct ExprEvaluator<'a> { vm: &'a mut VM, }
impl<'a> ExprEvaluator<'a> { /// Evaluate expression with precedence climbing pub fn eval(&mut self, expr: &Expression) -> Result<Value, RuntimeError> { match expr { Expression::Binary(op, left, right) => { self.eval_binary(*op, left, right)
}
Expression::Unary(op, operand) => { self.eval_unary(*op, operand)
}
Expression::Call(callee, args) => { self.eval_call(callee, args)
}
// ... other expression types }
}
fn eval_binary(&mut self, op: BinaryOp, left: &Expression, right: &Expression) -> Result<Value, RuntimeError> { // Evaluate operands let left_val = self.eval(left)?;
// Short-circuit for && and ||
if op == BinaryOp::And && !left_val.is_truthy() { return Ok(left_val);
}
if op == BinaryOp::Or && left_val.is_truthy() { return Ok(left_val);
}
let right_val = self.eval(right)?;
// Apply operator with correct semantics match op { BinaryOp::Add => self.add(left_val, right_val), BinaryOp::Sub => self.sub(left_val, right_val), BinaryOp::Mul => self.mul(left_val, right_val), BinaryOp::Div => self.div(left_val, right_val), BinaryOp::Mod => self.modulo(left_val, right_val), BinaryOp::Eq => self.loose_eq(left_val, right_val), BinaryOp::StrictEq => self.strict_eq(left_val, right_val), // ... other operators }
}
}
```

### 4. Event Loop (Fix #5)

Implement a proper event loop with microtask and macrotask queues:
```rust
/// Event loop implementation pub struct EventLoop { /// Microtask queue (promises, queueMicrotask)
microtasks: VecDeque<Task>, /// Macrotask queue (setTimeout, setInterval, I/O)
macrotasks: VecDeque<Task>, /// Timer heap (sorted by fire time)
timers: BinaryHeap<Timer>, /// Pending I/O operations io_pending: Vec<IoOperation>, /// Async runtime (tokio)
runtime: tokio::runtime::Runtime, }
impl EventLoop { pub fn new() -> Self;
/// Run the event loop until all tasks complete pub fn run(&mut self, vm: &mut VM) -> Result<(), RuntimeError> { loop { // 1. Run all microtasks while let Some(task) = self.microtasks.pop_front() { vm.execute_task(task)?;
// Microtasks can queue more microtasks self.drain_microtasks(vm)?;
}
// 2. Run one macrotask if let Some(task) = self.macrotasks.pop_front() { vm.execute_task(task)?;
continue;
}
// 3. Check timers if let Some(timer) = self.check_timers() { self.macrotasks.push_back(timer.callback);
continue;
}
// 4. Poll I/O if self.poll_io()? { continue;
}
// 5. No more work break;
}
Ok(())
}
/// Queue a microtask (Promise.then, queueMicrotask)
pub fn queue_microtask(&mut self, task: Task);
/// Queue a macrotask (setTimeout callback)
pub fn queue_macrotask(&mut self, task: Task);
/// Schedule a timer pub fn set_timeout(&mut self, callback: Task, delay: Duration) -> TimerId;
pub fn set_interval(&mut self, callback: Task, interval: Duration) -> TimerId;
pub fn clear_timer(&mut self, id: TimerId);
}
```

### 5. Module System (Fix #1.7)

Implement ES modules with proper resolution:
```rust
/// Module loader and linker pub struct ModuleLoader { /// Loaded modules by specifier modules: HashMap<String, Module>, /// Module resolution cache resolve_cache: HashMap<(String, String), String>, }
impl ModuleLoader { /// Load a module from a specifier pub async fn load(&mut self, specifier: &str, referrer: &str) -> Result<&Module, ModuleError> { // 1. Resolve specifier to URL/path let resolved = self.resolve(specifier, referrer)?;
// 2. Check cache if let Some(module) = self.modules.get(&resolved) { return Ok(module);
}
// 3. Fetch source let source = self.fetch(&resolved).await?;
// 4. Parse module let ast = parse_module(&source)?;
// 5. Analyze imports/exports let module_record = analyze_module(&ast)?;
// 6. Load dependencies recursively for import in &module_record.imports { self.load(&import.specifier, &resolved).await?;
}
// 7. Link module let module = self.link(module_record)?;
// 8. Cache and return self.modules.insert(resolved.clone(), module);
Ok(self.modules.get(&resolved).unwrap())
}
/// Resolve module specifier fn resolve(&self, specifier: &str, referrer: &str) -> Result<String, ModuleError> { // Handle different specifier types:
// - Relative: ./foo, ../bar // - Absolute: /foo, file:///foo // - Bare: lodash, @scope/pkg // - URL: https://example.com/foo.js }
}
```

### 6. Bundler Tree Shaking (Fix #6, #10)

Implement proper dead code elimination:
```rust
/// Tree shaking implementation pub struct TreeShaker { /// Module graph graph: ModuleGraph, /// Used exports per module used_exports: HashMap<ModuleId, HashSet<String>>, /// Side effect modules (must include)
side_effect_modules: HashSet<ModuleId>, }
impl TreeShaker { /// Analyze and mark used exports pub fn analyze(&mut self, entry: ModuleId) { // Start from entry point exports let entry_module = self.graph.get(entry);
for export in &entry_module.exports { self.mark_used(entry, &export.name);
}
}
/// Mark an export as used and trace dependencies fn mark_used(&mut self, module_id: ModuleId, export_name: &str) { let used = self.used_exports.entry(module_id).or_default();
if used.contains(export_name) { return; // Already processed }
used.insert(export_name.to_string());
let module = self.graph.get(module_id);
// Find the export binding if let Some(binding) = module.find_export(export_name) { // Trace all references in the binding for reference in &binding.references { match reference { Reference::Import { module, name } => { self.mark_used(*module, name);
}
Reference::Local { name } => { // Mark local binding as used }
}
}
}
}
/// Generate tree-shaken output pub fn generate(&self) -> Vec<TransformedModule> { self.graph.modules()
.filter(|m| self.is_included(m.id))
.map(|m| self.transform_module(m))
.collect()
}
}
```

### 7. Source Map Generator (Fix #7, #9)

Implement proper source map generation:
```rust
/// Source map generator following v3 spec pub struct SourceMapGenerator { /// Output file name file: String, /// Source files sources: Vec<String>, /// Source contents (optional)
sources_content: Vec<Option<String>>, /// Symbol names names: Vec<String>, /// Mappings (VLQ encoded)
mappings: Vec<Mapping>, }


#[derive(Clone)]


pub struct Mapping { /// Generated line (0-indexed)
pub gen_line: u32, /// Generated column (0-indexed)
pub gen_column: u32, /// Source file index pub source: Option<u32>, /// Original line (0-indexed)
pub orig_line: Option<u32>, /// Original column (0-indexed)
pub orig_column: Option<u32>, /// Name index pub name: Option<u32>, }
impl SourceMapGenerator { pub fn new(file: &str) -> Self;
/// Add a source file pub fn add_source(&mut self, source: &str, content: Option<&str>) -> u32;
/// Add a name pub fn add_name(&mut self, name: &str) -> u32;
/// Add a mapping pub fn add_mapping(&mut self, mapping: Mapping);
/// Generate the source map JSON pub fn generate(&self) -> String { let mappings = self.encode_mappings();
serde_json::json!({ "version": 3, "file": self.file, "sources": self.sources, "sourcesContent": self.sources_content, "names": self.names, "mappings": mappings }).to_string()
}
/// Encode mappings as VLQ fn encode_mappings(&self) -> String { // VLQ encoding implementation }
}
```

### 8. Package Manager Lifecycle Scripts (Fix #8)

Implement lifecycle script execution:
```rust
/// Lifecycle script executor pub struct ScriptExecutor { /// Shell to use shell: Shell, /// Environment variables env: HashMap<String, String>, }
impl ScriptExecutor { /// Execute a lifecycle script pub async fn execute_lifecycle( &self, package: &Package, script: LifecycleScript, cwd: &Path, ) -> Result<(), ScriptError> { let script_content = match script { LifecycleScript::PreInstall => package.scripts.get("preinstall"), LifecycleScript::Install => package.scripts.get("install"), LifecycleScript::PostInstall => package.scripts.get("postinstall"), LifecycleScript::Prepare => package.scripts.get("prepare"), };
let Some(script_content) = script_content else { return Ok(()); // No script defined };
// Set up environment let mut env = self.env.clone();
env.insert("npm_package_name".to_string(), package.name.clone());
env.insert("npm_package_version".to_string(), package.version.clone());
env.insert("PATH".to_string(), self.get_path_with_node_modules(cwd));
// Execute script let output = Command::new(&self.shell.program)
.args(&self.shell.args)
.arg(script_content)
.current_dir(cwd)
.envs(&env)
.output()
.await?;
if !output.status.success() { return Err(ScriptError::Failed { script: script.name(), package: package.name.clone(), stderr: String::from_utf8_lossy(&output.stderr).to_string(), });
}
Ok(())
}
}
```

## Data Models

### Runtime Value Types

```rust
/// JavaScript object representation pub struct Object { /// Property storage (hidden class + values)
properties: PropertyStorage, /// Prototype chain prototype: Option<GcRef<Object>>, /// Internal slots internal: InternalSlots, }
/// JavaScript array representation pub struct Array { /// Dense elements (for integer indices)
elements: Vec<Value>, /// Sparse elements (for large indices)
sparse: Option<HashMap<u32, Value>>, /// Length property length: u32, }
/// JavaScript function representation pub struct Function { /// Function kind kind: FunctionKind, /// Compiled bytecode or native code code: FunctionCode, /// Captured environment (for closures)
environment: Option<GcRef<Environment>>, /// Bound this value bound_this: Option<Value>, }
pub enum FunctionKind { Normal, Arrow, Generator, Async, AsyncGenerator, }
```

### Package Manager Data Models

```rust
/// Package manifest (package.json)
pub struct PackageManifest { pub name: String, pub version: String, pub dependencies: HashMap<String, String>, pub dev_dependencies: HashMap<String, String>, pub peer_dependencies: HashMap<String, String>, pub optional_dependencies: HashMap<String, String>, pub scripts: HashMap<String, String>, pub bin: HashMap<String, String>, pub workspaces: Option<Vec<String>>, pub private: bool, }
/// Lockfile entry pub struct LockfileEntry { pub name: String, pub version: String, pub resolved: String, pub integrity: String, // SHA-512 hash pub dependencies: HashMap<String, String>, }
/// Registry configuration pub struct RegistryConfig { pub default_registry: String, pub scoped_registries: HashMap<String, String>, pub auth_tokens: HashMap<String, String>, }
```

### Test Runner Data Models

```rust
/// Test suite pub struct TestSuite { pub name: String, pub file: PathBuf, pub tests: Vec<Test>, pub before_all: Option<TestHook>, pub after_all: Option<TestHook>, pub before_each: Option<TestHook>, pub after_each: Option<TestHook>, }
/// Individual test pub struct Test { pub name: String, pub function: TestFunction, pub timeout: Duration, pub skip: bool, pub only: bool, }
/// Mock function pub struct MockFunction { pub calls: Vec<MockCall>, pub return_values: VecDeque<Value>, pub implementation: Option<Box<dyn Fn(Vec<Value>) -> Value>>, }
/// Coverage data pub struct CoverageData { pub file: PathBuf, pub lines: HashMap<u32, u32>, // line -> hit count pub branches: HashMap<u32, (u32, u32)>, // branch -> (true hits, false hits)
pub functions: HashMap<String, u32>, // function -> hit count }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system—, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.

### Property 1: Expression Evaluation Correctness

For any valid JavaScript expression, evaluating it in dx-js SHALL produce the same result as evaluating it in V8/Node.js. Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6

### Property 2: Garbage Collection Safety

For any program execution, the garbage collector SHALL NOT collect objects that are still reachable from roots, AND SHALL eventually collect all unreachable objects. Validates: Requirements 2.1, 2.2, 2.3, 2.6

### Property 3: Event Loop Ordering

For any sequence of async operations, microtasks SHALL always execute before macrotasks, and tasks within each queue SHALL execute in FIFO order. Validates: Requirements 5.7

### Property 4: Module Resolution Determinism

For any module specifier and referrer, the resolver SHALL always return the same resolved path. Validates: Requirements 1.7

### Property 5: Tree Shaking Correctness

For any bundle, removing unused exports SHALL NOT change the observable behavior of the program. Validates: Requirements 6.3

### Property 6: Source Map Round-Trip

For any source position in the original file, mapping through the source map and back SHALL return the same position (within one line/column). Validates: Requirements 7.1, 7.2

### Property 7: Lockfile Reproducibility

For any package.json, running `dx install` twice with the same lockfile SHALL produce identical node_modules contents. Validates: Requirements 23.1, 23.3

### Property 8: Type Coercion Consistency

For any value and target type, type coercion in dx-js SHALL match ECMAScript specification behavior. Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6

### Property 9: Test Isolation

For any test suite, each test SHALL run in isolation such that the order of test execution does not affect results. Validates: Requirements 11.1, 11.2, 11.3

### Property 10: Coverage Accuracy

For any test run with coverage enabled, the reported coverage percentages SHALL accurately reflect the executed code paths. Validates: Requirements 12.2, 12.3, 12.4

### Property 11: Snapshot Determinism

For any value, serializing to snapshot format and comparing SHALL be deterministic (same value always produces same snapshot). Validates: Requirements 13.1, 13.2

### Property 12: Cross-Platform Path Handling

For any file path, the tools SHALL handle it correctly regardless of the operating system's path separator. Validates: Requirements 38.2, 38.3

## Error Handling

### Runtime Errors

```rust
/// Runtime error types


#[derive(Debug, thiserror::Error)]


pub enum RuntimeError {


#[error("SyntaxError: {message} at {file}:{line}:{column}")]


SyntaxError { message: String, file: String, line: u32, column: u32, },


#[error("TypeError: {message}")]


TypeError { message: String },


#[error("ReferenceError: {name} is not defined")]


ReferenceError { name: String },


#[error("RangeError: {message}")]


RangeError { message: String },


#[error("Error: {message}")]


Generic { message: String }, }
impl RuntimeError { /// Create error with stack trace pub fn with_stack(self, stack: Vec<StackFrame>) -> Self;
/// Format error for display pub fn format(&self) -> String;
}
```

### Package Manager Errors

```rust
/// Package manager error types


#[derive(Debug, thiserror::Error)]


pub enum PackageError {


#[error("Package not found: {name}")]


NotFound { name: String },


#[error("Version not found: {name}@{version}")]


VersionNotFound { name: String, version: String },


#[error("Integrity check failed for {name}@{version}: expected {expected}, got {actual}")]


IntegrityMismatch { name: String, version: String, expected: String, actual: String, },


#[error("Lifecycle script failed: {script} in {package}\n{stderr}")]


ScriptFailed { script: String, package: String, stderr: String, },


#[error("Registry error: {message}")]


RegistryError { message: String },


#[error("Network error: {message}")]


NetworkError { message: String }, }
```

### Bundler Errors

```rust
/// Bundler error types


#[derive(Debug, thiserror::Error)]


pub enum BundleError {


#[error("Module not found: {specifier} from {referrer}")]


ModuleNotFound { specifier: String, referrer: String },


#[error("Parse error in {file}:{line}:{column}: {message}")]


ParseError { file: String, line: u32, column: u32, message: String, },


#[error("Circular dependency detected: {cycle:?}")]


CircularDependency { cycle: Vec<String> },


#[error("Transform error: {message}")]


TransformError { message: String }, }
```

## Testing Strategy

### Unit Tests

Unit tests will cover individual components in isolation: -Value representation - Test all value types and conversions -Expression evaluation - Test all operators with various operand types -Garbage collector - Test allocation, collection, and memory limits -Event loop - Test task ordering and timer behavior -Module resolver - Test all specifier types and edge cases -Tree shaker - Test dead code detection -Source map generator - Test VLQ encoding and mapping accuracy

### Property-Based Tests

Property-based tests will verify invariants across random inputs: -Expression evaluation - Generate random expressions, compare with V8 -GC safety - Generate random object graphs, verify no premature collection -Event loop ordering - Generate random task sequences, verify ordering -Source map round-trip - Generate random mappings, verify reversibility -Type coercion - Generate random values, compare coercion with V8

### Integration Tests

Integration tests will verify end-to-end behavior: -Runtime - Run JavaScript test suites (Test262 subset) -Package manager - Install real packages, verify functionality -Bundler - Bundle real projects, verify output works -Test runner - Run test suites, verify results

### Test Configuration

```rust
/// Property-based test configuration /// Minimum 100 iterations per property test /// Tag format: Feature: dx-js-production-complete, Property {number}: {property_text}
```

## Implementation Phases

### Phase 1: Runtime Foundation (Weeks 1-4)

- Value representation with NaN-boxing
- Garbage collector (generational mark-sweep)
- Basic expression evaluation with correct precedence
- Control flow (if/else/for/while/switch)

### Phase 2: Runtime Completion (Weeks 5-8)

- Functions and closures
- Classes and inheritance
- Async/await and promises
- Event loop with timers

### Phase 3: Module System (Weeks 9-10)

- ES module loading and linking
- CommonJS compatibility
- Node.js API stubs

### Phase 4: Bundler Improvements (Weeks 11-12)

- Tree shaking implementation
- Source map generation
- Code splitting
- CSS bundling

### Phase 5: Package Manager Improvements (Weeks 13-14)

- Lifecycle script execution
- Private registry support
- Workspace support
- Peer dependency handling

### Phase 6: Test Runner Improvements (Weeks 15-16)

- Mocking and spying
- Code coverage
- Snapshot testing

### Phase 7: Polish and Documentation (Weeks 17-18)

- Remove all unsafe patterns
- Update documentation
- Performance benchmarks
- Cross-platform testing
