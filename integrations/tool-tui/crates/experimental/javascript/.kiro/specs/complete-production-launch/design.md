
# Design Document: DX JavaScript Tooling Complete Production Launch

## Overview

This design addresses all remaining incomplete implementations across the DX JavaScript tooling suite. The primary focus is replacing placeholder/stub code with real implementations to achieve production readiness. The implementation is organized into six major areas: -Runtime Codegen Completion - Replace all placeholder values with real object/array/function creation -Package Manager Pipeline - Complete the installation pipeline with real extraction -Bundler Completion - Source maps and remaining features -Test Runner Completion - Watch mode, coverage, snapshots -Compatibility Layer - Complete Node.js API implementations -Integration - Wire all components together

## Architecture

+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+
| ┌─────────────────────────────────────────────────────────────────┐                                                                                                                                       |
+===========================================================================================================================================================================================================+
| │CLI                                                                                                                                                                                                      |
+-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------+



## Components and Interfaces

### 1. Runtime Codegen - Object Creation

The current codegen returns placeholder values. We need to implement proper object allocation using the RuntimeHeap.
```rust
// runtime/src/compiler/codegen.rs - FIXED CreateFunction TypedInstruction::CreateFunction { dest, function_id, captured_vars, is_arrow } => { // Get runtime heap function reference let create_closure_ref = func_refs.get(&(u32::MAX - 20))
.ok_or_else(|| DxError::CompileError("Missing create_closure builtin".into()))?;
// Pass function ID let func_id_val = builder.ins().f64const(function_id.0 as f64);
// Pass captured variable count let captured_count = builder.ins().f64const(captured_vars.len() as f64);
// Call runtime to allocate closure let call = builder.ins().call(*create_closure_ref, &[func_id_val, captured_count]);
let closure_id = builder.inst_results(call)[0];
// Store captured variables in the closure for (i, &var_id) in captured_vars.iter().enumerate() { if let Some(&var_val) = locals.get(&var_id) { // Call set_captured(closure_id, index, value)
let set_captured_ref = func_refs.get(&(u32::MAX - 25))
.expect("Missing set_captured builtin");
let idx = builder.ins().f64const(i as f64);
builder.ins().call(*set_captured_ref, &[closure_id, idx, var_val]);
}
}
// Store is_arrow flag if *is_arrow { let set_arrow_ref = func_refs.get(&(u32::MAX - 26))
.expect("Missing set_arrow builtin");
builder.ins().call(*set_arrow_ref, &[closure_id]);
}
locals.insert(*dest, closure_id);
}
```
```rust
// runtime/src/compiler/codegen.rs - FIXED CreateArray TypedInstruction::CreateArray { dest, elements } => { // Create array with correct capacity let create_array_ref = func_refs.get(&(u32::MAX - 21))
.ok_or_else(|| DxError::CompileError("Missing create_array builtin".into()))?;
let count_val = builder.ins().f64const(elements.len() as f64);
let call = builder.ins().call(*create_array_ref, &[count_val]);
let array_id = builder.inst_results(call)[0];
// Set each element let array_set_ref = func_refs.get(&(u32::MAX - 27))
.expect("Missing array_set builtin");
for (i, elem_id) in elements.iter().enumerate() { if let Some(&elem_val) = locals.get(elem_id) { let idx = builder.ins().f64const(i as f64);
builder.ins().call(*array_set_ref, &[array_id, idx, elem_val]);
}
}
locals.insert(*dest, array_id);
}
```
```rust
// runtime/src/compiler/codegen.rs - FIXED CreateObject TypedInstruction::CreateObject { dest, properties } => { // Create empty object let create_object_ref = func_refs.get(&(u32::MAX - 24))
.ok_or_else(|| DxError::CompileError("Missing create_object builtin".into()))?;
let call = builder.ins().call(*create_object_ref, &[]);
let object_id = builder.inst_results(call)[0];
// Set each property let object_set_ref = func_refs.get(&(u32::MAX - 28))
.expect("Missing object_set builtin");
for (key, value_id) in properties { if let Some(&value_val) = locals.get(value_id) { // Store key as string ID (would need string interning)
let key_hash = builder.ins().f64const(hash_string(key) as f64);
builder.ins().call(*object_set_ref, &[object_id, key_hash, value_val]);
}
}
locals.insert(*dest, object_id);
}
```

### 2. Runtime Codegen - This Binding

```rust
// runtime/src/compiler/codegen.rs - FIXED GetThis TypedInstruction::GetThis { dest } => { // Get this binding from call frame // The this value is passed as a hidden first parameter for non-arrow functions let get_this_ref = func_refs.get(&(u32::MAX - 29))
.ok_or_else(|| DxError::CompileError("Missing get_this builtin".into()))?;
let call = builder.ins().call(*get_this_ref, &[]);
let this_val = builder.inst_results(call)[0];
locals.insert(*dest, this_val);
}
```

### 3. Runtime Codegen - TypeOf

```rust
// runtime/src/compiler/codegen.rs - FIXED TypeOf TypedInstruction::TypeOf { dest, operand } => { // Call runtime typeof function let typeof_ref = func_refs.get(&(u32::MAX - 30))
.ok_or_else(|| DxError::CompileError("Missing typeof builtin".into()))?;
let operand_val = locals[operand];
let call = builder.ins().call(*typeof_ref, &[operand_val]);
let type_id = builder.inst_results(call)[0];
locals.insert(*dest, type_id);
}
// New runtime function extern "C" fn builtin_typeof(value: f64) -> f64 { let heap = get_runtime_heap();
// Check if it's a heap object let id = value as u64;
if value.is_nan() { // undefined return TYPE_UNDEFINED;
}
if heap.closures.contains_key(&id) { return TYPE_FUNCTION;
}
if heap.arrays.contains_key(&id) { return TYPE_OBJECT; // arrays are objects in JS }
if heap.objects.contains_key(&id) { return TYPE_OBJECT;
}
// It's a number TYPE_NUMBER }
const TYPE_UNDEFINED: f64 = 0.0;
const TYPE_NUMBER: f64 = 1.0;
const TYPE_STRING: f64 = 2.0;
const TYPE_BOOLEAN: f64 = 3.0;
const TYPE_OBJECT: f64 = 4.0;
const TYPE_FUNCTION: f64 = 5.0;
const TYPE_SYMBOL: f64 = 6.0;
const TYPE_BIGINT: f64 = 7.0;
```

### 4. Console Timing Implementation

```rust
// runtime/src/compiler/builtins_registry.rs - FIXED console.time use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
lazy_static::lazy_static! { static ref CONSOLE_TIMERS: Mutex<HashMap<String, Instant>> = Mutex::new(HashMap::new());
}
fn builtin_console_time(args: &[Value]) -> Value { let label = match args.first() { Some(Value::String(s)) => s.clone(), Some(v) => v.to_string(), None => "default".to_string(), };
let mut timers = CONSOLE_TIMERS.lock().unwrap();
if timers.contains_key(&label) { eprintln!("Warning: Timer '{}' already exists", label);
}
timers.insert(label, Instant::now());
Value::Undefined }
fn builtin_console_time_end(args: &[Value]) -> Value { let label = match args.first() { Some(Value::String(s)) => s.clone(), Some(v) => v.to_string(), None => "default".to_string(), };
let mut timers = CONSOLE_TIMERS.lock().unwrap();
if let Some(start) = timers.remove(&label) { let elapsed = start.elapsed();
let ms = elapsed.as_secs_f64() * 1000.0;
println!("{}: {:.3}ms", label, ms);
} else { eprintln!("Warning: Timer '{}' does not exist", label);
}
Value::Undefined }
fn builtin_console_time_log(args: &[Value]) -> Value { let label = match args.first() { Some(Value::String(s)) => s.clone(), Some(v) => v.to_string(), None => "default".to_string(), };
let timers = CONSOLE_TIMERS.lock().unwrap();
if let Some(start) = timers.get(&label) { let elapsed = start.elapsed();
let ms = elapsed.as_secs_f64() * 1000.0;
// Print label and elapsed time, plus any additional arguments print!("{}: {:.3}ms", label, ms);
for arg in args.iter().skip(1) { print!(" {}", arg.to_string());
}
println!();
} else { eprintln!("Warning: Timer '{}' does not exist", label);
}
Value::Undefined }
```

### 5. Package Manager - Real Extraction

```rust
// package-manager/dx-pkg-extract/src/lib.rs - Complete implementation use anyhow::Result;
use flate2::read::GzDecoder;
use tar::Archive;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
pub struct PackageExtractor { use_hardlinks: bool, }
impl PackageExtractor { pub fn new() -> Self { Self { use_hardlinks: cfg!(unix) || cfg!(windows),
}
}
/// Extract npm tarball to destination pub fn extract_tarball(&self, tarball_path: &Path, dest: &Path) -> Result<()> { // Read tarball let file = File::open(tarball_path)?;
let decoder = GzDecoder::new(file);
let mut archive = Archive::new(decoder);
// Create destination fs::create_dir_all(dest)?;
// Extract each entry for entry in archive.entries()? { let mut entry = entry?;
let path = entry.path()?;
// npm tarballs have "package/" prefix let relative = path.strip_prefix("package/")
.unwrap_or(&path);
let dest_path = dest.join(relative);
// Create parent directories if let Some(parent) = dest_path.parent() { fs::create_dir_all(parent)?;
}
match entry.header().entry_type() { tar::EntryType::Regular => { // Extract file let mut file = File::create(&dest_path)?;
std::io::copy(&mut entry, &mut file)?;
// Preserve permissions on Unix


#[cfg(unix)]


{ use std::os::unix::fs::PermissionsExt;
if let Ok(mode) = entry.header().mode() { fs::set_permissions(&dest_path, fs::Permissions::from_mode(mode))?;
}
}
}
tar::EntryType::Symlink => { if let Some(link_name) = entry.link_name()? {


#[cfg(unix)]


std::os::unix::fs::symlink(&link_name, &dest_path)?;


#[cfg(windows)]


{ // Windows requires admin for symlinks, use junction or copy if link_name.is_dir() { junction::create(&link_name, &dest_path)?;
} else { fs::copy(&link_name, &dest_path)?;
}
}
}
}
tar::EntryType::Directory => { fs::create_dir_all(&dest_path)?;
}
_ => {}
}
}
Ok(())
}
/// Extract from cache using hardlinks (instant)
pub fn extract_with_hardlinks(&self, cache_path: &Path, dest: &Path) -> Result<()> { fs::create_dir_all(dest)?;
for entry in walkdir::WalkDir::new(cache_path) { let entry = entry?;
let relative = entry.path().strip_prefix(cache_path)?;
let dest_path = dest.join(relative);
if entry.file_type().is_dir() { fs::create_dir_all(&dest_path)?;
} else if entry.file_type().is_file() { if let Some(parent) = dest_path.parent() { fs::create_dir_all(parent)?;
}
// Try hardlink first if fs::hard_link(entry.path(), &dest_path).is_err() { // Fallback to copy fs::copy(entry.path(), &dest_path)?;
}
}
}
Ok(())
}
}
```

### 6. Package Manager - DXP Format Implementation

```rust
// package-manager/dx-pkg-format/src/lib.rs - Complete DxpBuilder use anyhow::Result;
use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::Path;
const DXP_MAGIC: &[u8; 4] = b"DXP1";
const DXP_VERSION: u32 = 1;
impl DxpBuilder { pub fn build<P: AsRef<Path>>(self, output: P) -> Result<()> { let file = File::create(output)?;
let mut writer = BufWriter::new(file);
// Write header writer.write_all(DXP_MAGIC)?;
writer.write_all(&DXP_VERSION.to_le_bytes())?;
// Write metadata let metadata = serde_json::to_vec(&self.metadata)?;
writer.write_all(&(metadata.len() as u32).to_le_bytes())?;
writer.write_all(&metadata)?;
// Write file table writer.write_all(&(self.files.len() as u32).to_le_bytes())?;
let mut data_offset = 0u64;
for (path, data) in &self.files { // Write path length and path let path_bytes = path.as_bytes();
writer.write_all(&(path_bytes.len() as u16).to_le_bytes())?;
writer.write_all(path_bytes)?;
// Write data offset and size writer.write_all(&data_offset.to_le_bytes())?;
writer.write_all(&(data.len() as u64).to_le_bytes())?;
data_offset += data.len() as u64;
}
// Write file data (compressed with lz4)
for (_, data) in &self.files { let compressed = lz4_flex::compress_prepend_size(data);
writer.write_all(&compressed)?;
}
writer.flush()?;
Ok(())
}
}
```

### 7. Module Compilation - Real Implementation

```rust
// runtime/src/compiler/modules.rs - Fixed compile_module pub fn compile_module(&mut self, path: &str) -> DxResult<ModuleId> { // Check if already compiled if let Some(&id) = self.module_ids.get(path) { return Ok(id);
}
// Read source file let source = std::fs::read_to_string(path)
.map_err(|e| DxError::ModuleError(format!("Cannot read {}: {}", path, e)))?;
// Parse with OXC let allocator = oxc_allocator::Allocator::default();
let source_type = if path.ends_with(".ts") || path.ends_with(".tsx") { oxc_parser::SourceType::from_path(path).unwrap_or_default()
} else { oxc_parser::SourceType::default().with_module(true)
};
let parsed = oxc_parser::Parser::new(&allocator, &source, source_type).parse();
if !parsed.errors.is_empty() { return Err(DxError::ParseError(format!( "Parse errors in {}: {:?}", path, parsed.errors )));
}
// Extract imports using AST let imports = self.extract_imports(&parsed.program);
// Resolve and compile dependencies first for import in &imports { let resolved_path = self.resolve_import(path, &import.specifier)?;
self.compile_module(&resolved_path)?;
}
// Lower AST to MIR let mir = self.lower_module(&parsed.program)?;
// Generate native code let compiled = self.codegen.generate(&mir)?;
// Register module let module_id = ModuleId(self.modules.len() as u32);
self.modules.push(Module { path: path.to_string(), imports, exports: self.extract_exports(&parsed.program), compiled: Some(compiled), });
self.module_ids.insert(path.to_string(), module_id);
Ok(module_id)
}
fn extract_imports(&self, program: &oxc_ast::ast::Program) -> Vec<ImportInfo> { use oxc_ast::ast::*;
let mut imports = Vec::new();
for stmt in &program.body { match stmt { Statement::ImportDeclaration(decl) => { imports.push(ImportInfo { specifier: decl.source.value.to_string(), kind: ImportKind::Static, });
}
Statement::ExportNamedDeclaration(decl) => { if let Some(source) = &decl.source { imports.push(ImportInfo { specifier: source.value.to_string(), kind: ImportKind::ReExport, });
}
}
Statement::ExportAllDeclaration(decl) => { imports.push(ImportInfo { specifier: decl.source.value.to_string(), kind: ImportKind::ReExport, });
}
_ => {}
}
}
imports }
```

## Correctness Properties

A property is a characteristic or behavior that should hold true across all valid executions of a system.

### Property 1: Object Creation Integrity

For any CreateObject instruction with N properties, the resulting object SHALL contain exactly N properties with correct keys and values. Validates: Requirements 1.3

### Property 2: Array Creation Integrity

For any CreateArray instruction with N elements, the resulting array SHALL have length N with all elements correctly initialized. Validates: Requirements 1.2

### Property 3: Closure Capture Correctness

For any closure that references K free variables, exactly K variables SHALL be captured and accessible within the closure. Validates: Requirements 1.1, 6.1, 6.2, 6.3

### Property 4: This Binding Correctness

For any method call on object O, the `this` value inside the method SHALL be O. Validates: Requirements 1.4

### Property 5: TypeOf Correctness

For any value V, typeof V SHALL return the correct type string according to ECMAScript specification. Validates: Requirements 1.5

### Property 6: Exception Propagation

For any thrown exception, it SHALL propagate to the nearest enclosing catch block or terminate the program. Validates: Requirements 3.1, 3.2, 3.3, 3.4

### Property 7: Console Timer Round-Trip

For any label L, calling console.time(L) followed by console.timeEnd(L) SHALL log a non-negative elapsed time. Validates: Requirements 4.1, 4.2

### Property 8: Package Extraction Integrity

For any npm tarball, extraction SHALL produce files with correct content, permissions, and symlinks preserved. Validates: Requirements 8.3, 8.4

### Property 9: Module Resolution Correctness

For any import specifier S from module M, resolution SHALL follow Node.js algorithm and return a valid path or error. Validates: Requirements 7.2, 7.3

### Property 10: No Placeholder Values

For any codegen instruction that produces a value, the result SHALL NOT be a placeholder (NaN for objects, 0.0/1.0 for counts). Validates: Requirements 18.3

## Error Handling

### Runtime Errors

- TypeError for invalid operations (calling non-function, property access on null)
- ReferenceError for undefined variables
- SyntaxError for parse failures with line/column information
- RangeError for invalid array lengths

### Package Manager Errors

- Network errors with retry information
- Extraction errors with file path
- Resolution errors with dependency chain
- Checksum mismatch with expected vs actual hash

### Bundler Errors

- Parse errors with source location
- Resolution errors with import chain
- Circular dependency warnings

## Testing Strategy

### Unit Tests

- Test each codegen instruction in isolation
- Test builtin functions with edge cases
- Test extraction with various tarball formats

### Property-Based Tests

- Generate random objects/arrays and verify creation
- Generate random closures and verify capture
- Generate random import graphs and verify resolution

### Integration Tests

- End-to-end script execution
- Full package installation flow
- Bundle and execute workflow

### Regression Tests

- Test262 conformance suite
- npm package compatibility tests
- Real-world project tests
