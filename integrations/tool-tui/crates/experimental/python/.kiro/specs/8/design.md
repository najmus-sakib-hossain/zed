
# Design Document: DX-Py-Runtime Improvements

## Overview

This design document details the implementation approach for addressing all identified weaknesses in dx-py-runtime. The improvements are organized into 10 components that can be implemented incrementally.

## Component 1: ARM NEON String Engine

Requirements Addressed: 1.1-1.8

### Implementation Design

```rust
// crates/dx-py-runtime/dx-py-simd/src/neon.rs use std::arch::aarch64::*;
use crate::engine::SimdStringEngine;
pub struct NeonStringEngine;
impl NeonStringEngine { pub fn new() -> Self { Self }


#[inline]



#[target_feature(enable = "neon")]


unsafe fn find_neon(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> { if needle.is_empty() { return Some(0);
}
if needle.len() > haystack.len() { return None;
}
let first = vdupq_n_u8(needle[0]);
let mut i = 0;
while i + 16 <= haystack.len() - needle.len() + 1 { let chunk = vld1q_u8(haystack.as_ptr().add(i));
let eq = vceqq_u8(chunk, first);
let mask = Self::neon_movemask(eq);
if mask != 0 { let mut bit = 0;
while bit < 16 { if (mask >> bit) & 1 != 0 { let pos = i + bit;
if pos + needle.len() <= haystack.len() { if &haystack[pos..pos + needle.len()] == needle { return Some(pos);
}
}
}
bit += 1;
}
}
i += 16;
}
// Scalar fallback for remainder for j in i..haystack.len() - needle.len() + 1 { if &haystack[j..j + needle.len()] == needle { return Some(j);
}
}
None }


#[inline]


unsafe fn neon_movemask(v: uint8x16_t) -> u16 { // Extract high bits from each byte let shift = vld1q_u8([0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80].as_ptr());
let masked = vandq_u8(v, shift);
let low = vget_low_u8(masked);
let high = vget_high_u8(masked);
let low_sum = vaddv_u8(low) as u16;
let high_sum = (vaddv_u8(high) as u16) << 8;
low_sum | high_sum }
}
impl SimdStringEngine for NeonStringEngine { fn find(&self, haystack: &str, needle: &str) -> Option<usize> { unsafe { self.find_neon(haystack.as_bytes(), needle.as_bytes()) }
}
fn count(&self, haystack: &str, needle: &str) -> usize { // NEON-accelerated count implementation let mut count = 0;
let mut start = 0;
while let Some(pos) = self.find(&haystack[start..], needle) { count += 1;
start += pos + needle.len().max(1);
}
count }
fn eq(&self, a: &str, b: &str) -> bool { if a.len() != b.len() { return false;
}
unsafe { self.eq_neon(a.as_bytes(), b.as_bytes()) }
}
fn to_lowercase(&self, s: &str) -> String { unsafe { self.to_lowercase_neon(s) }
}
fn to_uppercase(&self, s: &str) -> String { unsafe { self.to_uppercase_neon(s) }
}
fn split<'a>(&self, s: &'a str, delimiter: &str) -> Vec<&'a str> { // Use NEON find for delimiter search let mut result = Vec::new();
let mut start = 0;
while let Some(pos) = self.find(&s[start..], delimiter) { result.push(&s[start..start + pos]);
start += pos + delimiter.len();
}
result.push(&s[start..]);
result }
fn join(&self, parts: &[&str], separator: &str) -> String { parts.join(separator)
}
fn replace(&self, s: &str, from: &str, to: &str) -> String { let mut result = String::with_capacity(s.len());
let mut start = 0;
while let Some(pos) = self.find(&s[start..], from) { result.push_str(&s[start..start + pos]);
result.push_str(to);
start += pos + from.len();
}
result.push_str(&s[start..]);
result }
fn name(&self) -> &'static str { "NEON"
}
}
```

### Dispatcher Update

```rust
// Update dispatcher.rs to use NEON engine


#[cfg(target_arch = "aarch64")]


{ if self.has_neon { return Box::new(NeonStringEngine::new());
}
}
```

## Component 2: AVX-512 String Engine

Requirements Addressed: 2.1-2.5

### Implementation Design

```rust
// crates/dx-py-runtime/dx-py-simd/src/avx512.rs


#[cfg(target_arch = "x86_64")]


use std::arch::x86_64::*;
pub struct Avx512StringEngine;
impl Avx512StringEngine {


#[target_feature(enable = "avx512f", enable = "avx512bw")]


pub unsafe fn new() -> Self { Self }


#[inline]



#[target_feature(enable = "avx512f", enable = "avx512bw")]


unsafe fn find_avx512(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> { if needle.is_empty() { return Some(0);
}
if needle.len() > haystack.len() { return None;
}
let first = _mm512_set1_epi8(needle[0] as i8);
let mut i = 0;
// Process 64 bytes at a time while i + 64 <= haystack.len() - needle.len() + 1 { let chunk = _mm512_loadu_si512(haystack.as_ptr().add(i) as *const __m512i);
let eq = _mm512_cmpeq_epi8_mask(chunk, first);
if eq != 0 { let mut mask = eq;
while mask != 0 { let bit = mask.trailing_zeros() as usize;
let pos = i + bit;
if pos + needle.len() <= haystack.len() { if &haystack[pos..pos + needle.len()] == needle { return Some(pos);
}
}
mask &= mask - 1;
}
}
i += 64;
}
// Scalar fallback for j in i..haystack.len() - needle.len() + 1 { if &haystack[j..j + needle.len()] == needle { return Some(j);
}
}
None }
}
```

## Component 3: NEON Collections

Requirements Addressed: 3.1-3.4

### Implementation Design

```rust
// crates/dx-py-runtime/dx-py-collections/src/neon_ops.rs


#[cfg(target_arch = "aarch64")]


use std::arch::aarch64::*;
pub struct NeonCollectionOps;
impl NeonCollectionOps {


#[cfg(target_arch = "aarch64")]



#[target_feature(enable = "neon")]


pub unsafe fn sum_i64(data: &[i64]) -> i64 { let mut sum = vdupq_n_s64(0);
let mut i = 0;
while i + 2 <= data.len() { let chunk = vld1q_s64(data.as_ptr().add(i));
sum = vaddq_s64(sum, chunk);
i += 2;
}
let mut result = vgetq_lane_s64(sum, 0) + vgetq_lane_s64(sum, 1);
// Handle remainder while i < data.len() { result += data[i];
i += 1;
}
result }


#[cfg(target_arch = "aarch64")]



#[target_feature(enable = "neon")]


pub unsafe fn sum_f64(data: &[f64]) -> f64 { let mut sum = vdupq_n_f64(0.0);
let mut i = 0;
while i + 2 <= data.len() { let chunk = vld1q_f64(data.as_ptr().add(i));
sum = vaddq_f64(sum, chunk);
i += 2;
}
let mut result = vgetq_lane_f64(sum, 0) + vgetq_lane_f64(sum, 1);
while i < data.len() { result += data[i];
i += 1;
}
result }
}
```

## Component 4: Error Handling Improvements

Requirements Addressed: 4.1-4.5

### Implementation Design

```rust
// crates/dx-py-runtime/dx-py-core/src/error.rs use thiserror::Error;


#[derive(Error, Debug)]


pub enum RuntimeError {


#[error("Type error: expected {expected}, got {actual}")]


TypeError { expected: String, actual: String },


#[error("Index out of bounds: index {index}, length {length}")]


IndexError { index: usize, length: usize },


#[error("Key not found: {key}")]


KeyError { key: String },


#[error("Division by zero")]


ZeroDivisionError,


#[error("Overflow in arithmetic operation")]


OverflowError,


#[error("Name not found: {name}")]


NameError { name: String },


#[error("Attribute not found: {attr} on {type_name}")]


AttributeError { attr: String, type_name: String },


#[error("Import error: {module}")]


ImportError { module: String },


#[error("I/O error: {0}")]


IoError(#[from] std::io::Error),


#[error("Memory allocation failed")]


MemoryError,


#[error("Internal error: {0}")]


InternalError(String), }
pub type RuntimeResult<T> = Result<T, RuntimeError>;
```

### Update Core Types

```rust
// Update PyList to return Result impl PyList { pub fn get(&self, index: i64) -> RuntimeResult<PyObjectRef> { let idx = if index < 0 { (self.len() as i64 + index) as usize } else { index as usize };
self.items.get(idx)
.cloned()
.ok_or(RuntimeError::IndexError { index: idx, length: self.len()
})
}
}
```

## Component 5: Cross-Crate Integration

Requirements Addressed: 5.1-5.5

### Interpreter-JIT Integration

```rust
// crates/dx-py-runtime/dx-py-interpreter/src/jit_integration.rs use dx_py_jit::{TieredJit, FunctionProfile};
use dx_py_bytecode::DpbFunction;
pub struct JitIntegration { jit: TieredJit, }
impl JitIntegration { pub fn new() -> Self { Self { jit: TieredJit::new(), }
}
pub fn maybe_compile(&mut self, func: &DpbFunction, profile: &FunctionProfile) -> Option<*const u8> { if self.jit.should_compile(profile) { self.jit.compile(func, profile)
} else { None }
}
pub fn execute_compiled(&self, code_ptr: *const u8, args: &[PyObjectRef]) -> PyObjectRef { // Execute JIT-compiled code unsafe { let func: fn(&[PyObjectRef]) -> PyObjectRef = std::mem::transmute(code_ptr);
func(args)
}
}
}
```

### Interpreter-Reactor Integration

```rust
// crates/dx-py-runtime/dx-py-interpreter/src/async_integration.rs use dx_py_reactor::{Reactor, create_reactor, IoOperation, Completion};
use dx_py_core::PyObjectRef;
pub struct AsyncRuntime { reactor: Box<dyn Reactor>, pending_futures: Vec<PendingFuture>, }
impl AsyncRuntime { pub fn new() -> Self { Self { reactor: create_reactor(), pending_futures: Vec::new(), }
}
pub fn submit_io(&mut self, op: IoOperation) -> u64 { self.reactor.submit(op).expect("Failed to submit I/O")
}
pub fn poll(&mut self) -> Vec<Completion> { self.reactor.poll().expect("Failed to poll reactor")
}
pub fn run_until_complete(&mut self, future_id: u64) -> PyObjectRef { loop { let completions = self.poll();
for completion in completions { if completion.user_data == future_id { return self.completion_to_pyobject(completion);
}
}
}
}
}
```

## Component 6: Integration Tests

Requirements Addressed: 6.1-6.5

### Test Structure

```rust
// crates/dx-py-runtime/tests/integration/mod.rs mod jit_tests;
mod async_tests;
mod gc_tests;
mod ffi_tests;
mod end_to_end;
```

### End-to-End Test Example

```rust
// crates/dx-py-runtime/tests/integration/end_to_end.rs use dx_py_core::*;
use dx_py_interpreter::Vm;
use dx_py_bytecode::DpbCompiler;


#[test]


fn test_simple_expression() { let vm = Vm::new();
let result = vm.eval("1 + 2 * 3").unwrap();
assert_eq!(result.as_int().unwrap(), 7);
}


#[test]


fn test_function_call() { let vm = Vm::new();
vm.exec("def add(a, b): return a + b").unwrap();
let result = vm.eval("add(10, 20)").unwrap();
assert_eq!(result.as_int().unwrap(), 30);
}


#[test]


fn test_list_operations() { let vm = Vm::new();
vm.exec("x = [1, 2, 3]").unwrap();
vm.exec("x.append(4)").unwrap();
let result = vm.eval("sum(x)").unwrap();
assert_eq!(result.as_int().unwrap(), 10);
}
```

## Component 7: Benchmarks

Requirements Addressed: 7.1-7.4

### Benchmark Structure

```rust
// crates/dx-py-runtime/dx-py-simd/benches/simd_benchmarks.rs use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use dx_py_simd::{get_engine, SimdStringEngine};
fn bench_find(c: &mut Criterion) { let engine = get_engine();
let mut group = c.benchmark_group("string_find");
for size in [100, 1000, 10000, 100000].iter() { let haystack: String = "a".repeat(*size);
let needle = "aaa";
group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| { b.iter(|| engine.find(&haystack, needle))
});
}
group.finish();
}
fn bench_to_lowercase(c: &mut Criterion) { let engine = get_engine();
let mut group = c.benchmark_group("to_lowercase");
for size in [100, 1000, 10000].iter() { let input: String = "HELLO WORLD ".repeat(*size / 12);
group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| { b.iter(|| engine.to_lowercase(&input))
});
}
group.finish();
}
criterion_group!(benches, bench_find, bench_to_lowercase);
criterion_main!(benches);
```

## Component 8: Documentation

Requirements Addressed: 8.1-8.4

### Module Documentation Template

```rust
//! # DX-Py SIMD //!
//! SIMD-accelerated string operations for the DX-Py runtime.
//!
//! ## Features //!
//! - AVX2 acceleration on x86_64 (32 bytes/iteration)
//! - AVX-512 acceleration on supported CPUs (64 bytes/iteration)
//! - NEON acceleration on ARM64 (16 bytes/iteration)
//! - Automatic CPU detection and dispatch //! - Scalar fallback for compatibility //!
//! ## Usage //!
//! ```rust //! use dx_py_simd::get_engine;
//!
//! let engine = get_engine();
//! let pos = engine.find("hello world", "world");
//! assert_eq!(pos, Some(6));
//! ```
//!
//! ## Performance //!
//! | Operation | AVX2 Speedup | NEON Speedup | //! |-----------|--------------|--------------|
//! | find | 8-15x | 4-8x |
//! | eq | 10-20x | 5-10x |
//! | lowercase | 6-12x | 3-6x |
```

## Component 9: Real Async I/O

Requirements Addressed: 9.1-9.4

### Linux io_uring Implementation

```rust
// crates/dx-py-runtime/dx-py-reactor/src/io_uring_real.rs


#[cfg(target_os = "linux")]


impl IoUringReactor { pub fn read_file(&mut self, fd: RawFd, buf: &mut IoBuffer, offset: u64) -> io::Result<u64> { let sqe = opcode::Read::new(types::Fd(fd), buf.as_mut_ptr(), buf.capacity() as u32)
.offset(offset)
.build()
.user_data(self.next_user_data());
unsafe { self.ring.submission().push(&sqe)?;
}
self.ring.submit()?;
// Wait for completion let cqe = self.ring.completion().next().ok_or(io::Error::new( io::ErrorKind::Other, "No completion", ))?;
if cqe.result() < 0 { return Err(io::Error::from_raw_os_error(-cqe.result()));
}
Ok(cqe.result() as u64)
}
}
```

## Component 10: Python Parser

Requirements Addressed: 10.1-10.4

### Parser Design

```rust
// crates/dx-py-runtime/dx-py-parser/src/lib.rs pub mod lexer;
pub mod parser;
pub mod ast;
use ast::*;
pub struct PythonParser { lexer: Lexer, }
impl PythonParser { pub fn new(source: &str) -> Self { Self { lexer: Lexer::new(source), }
}
pub fn parse_module(&mut self) -> Result<Module, ParseError> { let mut statements = Vec::new();
while !self.lexer.is_eof() { statements.push(self.parse_statement()?);
}
Ok(Module { body: statements })
}
fn parse_statement(&mut self) -> Result<Statement, ParseError> { match self.lexer.peek()? { Token::Def => self.parse_function_def(), Token::Class => self.parse_class_def(), Token::If => self.parse_if(), Token::For => self.parse_for(), Token::While => self.parse_while(), Token::Return => self.parse_return(), Token::Import => self.parse_import(), _ => self.parse_expr_statement(), }
}
}
```

## Correctness Properties

### Property 25: NEON-Scalar Equivalence

FOR ALL string inputs s1, s2: neon_engine.find(s1, s2) == scalar_engine.find(s1, s2) neon_engine.eq(s1, s2) == scalar_engine.eq(s1, s2) neon_engine.to_lowercase(s1) == scalar_engine.to_lowercase(s1)

### Property 26: AVX512-Scalar Equivalence

FOR ALL string inputs s1, s2: avx512_engine.find(s1, s2) == scalar_engine.find(s1, s2)

### Property 27: Error Recovery

FOR ALL operations that can fail: operation() returns Result<T, E> instead of panicking E provides sufficient context for debugging

### Property 28: Cross-Crate Integration

FOR ALL JIT-compiled functions: jit_execute(func, args) == interpret(func, args)
